//! `hud-sender probe`: reflection + action list + first-frame capture. NEVER calls TriggerGameAction.
use crate::cli::ProbeArgs;
use crate::pb::{inspect, wire};
use crate::sc::client;
use prost::Message as _;
use prost_types::{FileDescriptorProto, FileDescriptorSet};
use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;
use tonic::transport::Channel;

macro_rules! reflection_impl {
    ($fname:ident, $ver:ident) => {
        /// Returns (service names, file descriptor protos). Every error is returned as Err(Status)
        /// for the list step; later per-file errors are noted into `notes` and skipped.
        async fn $fname(ch: Channel, notes: &mut Vec<String>) -> Result<Vec<Vec<u8>>, tonic::Status> {
            use tonic_reflection::pb::$ver::{
                server_reflection_client::ServerReflectionClient, server_reflection_request::MessageRequest,
                server_reflection_response::MessageResponse, ServerReflectionRequest,
            };
            async fn ask(
                rc: &mut ServerReflectionClient<Channel>,
                req: MessageRequest,
            ) -> Result<Vec<MessageResponse>, tonic::Status> {
                let r = ServerReflectionRequest { host: String::new(), message_request: Some(req) };
                let mut s = rc.server_reflection_info(tokio_stream::once(r)).await?.into_inner();
                let mut out = Vec::new();
                while let Some(m) = s.message().await? {
                    if let Some(x) = m.message_response {
                        out.push(x);
                    }
                }
                Ok(out)
            }
            let mut rc = ServerReflectionClient::new(ch);
            let mut services = Vec::new();
            for m in ask(&mut rc, MessageRequest::ListServices(String::new())).await? {
                if let MessageResponse::ListServicesResponse(l) = m {
                    services.extend(l.service.into_iter().map(|s| s.name));
                }
            }
            notes.push(format!("services: {}", services.join(", ")));
            let mut files: Vec<Vec<u8>> = Vec::new();
            for svc in services.iter().filter(|s| !s.starts_with("grpc.reflection.")) {
                match ask(&mut rc, MessageRequest::FileContainingSymbol(svc.clone())).await {
                    Ok(ms) => {
                        for m in ms {
                            if let MessageResponse::FileDescriptorResponse(f) = m {
                                files.extend(f.file_descriptor_proto);
                            }
                        }
                    }
                    Err(e) => notes.push(format!("FileContainingSymbol({svc}) failed: {e}")),
                }
            }
            // Fetch missing dependencies until closed.
            let mut seen: HashSet<String> = HashSet::new();
            let mut tried: HashSet<String> = HashSet::new();
            loop {
                let mut missing = Vec::new();
                seen.clear();
                for f in &files {
                    if let Ok(d) = FileDescriptorProto::decode(&f[..]) {
                        seen.insert(d.name().to_string());
                    }
                }
                for f in &files {
                    if let Ok(d) = FileDescriptorProto::decode(&f[..]) {
                        for dep in &d.dependency {
                            if !seen.contains(dep) && !tried.contains(dep) {
                                missing.push(dep.clone());
                            }
                        }
                    }
                }
                missing.sort();
                missing.dedup();
                if missing.is_empty() {
                    break;
                }
                for dep in missing {
                    tried.insert(dep.clone());
                    match ask(&mut rc, MessageRequest::FileByFilename(dep.clone())).await {
                        Ok(ms) => {
                            for m in ms {
                                if let MessageResponse::FileDescriptorResponse(f) = m {
                                    files.extend(f.file_descriptor_proto);
                                }
                            }
                        }
                        Err(e) => notes.push(format!("FileByFilename({dep}) failed: {e}")),
                    }
                }
            }
            Ok(files)
        }
    };
}

reflection_impl!(reflect_v1, v1);
reflection_impl!(reflect_v1alpha, v1alpha);

fn write(dir: &Path, name: &str, data: impl AsRef<[u8]>) {
    if let Err(e) = std::fs::write(dir.join(name), data) {
        eprintln!("cannot write {name}: {e}");
    }
}

pub async fn run(args: ProbeArgs) -> anyhow::Result<i32> {
    let dir = match &args.out {
        Some(p) => p.clone(),
        None => {
            let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs();
            Path::new("schema").join(format!("probe-{secs}"))
        }
    };
    std::fs::create_dir_all(&dir)?;
    let url = client::normalize_endpoint(&args.grpc);

    // 1. connect
    let ch = match client::connect(&url, Duration::from_secs(2)).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("cannot connect to {url}: {e}");
            return Ok(2);
        }
    };
    println!("connect: ok ({url})");

    // 2. reflection (v1, then v1alpha)
    let mut notes = Vec::new();
    let mut result = reflect_v1(ch.clone(), &mut notes).await;
    if let Err(e) = &result {
        if e.code() == tonic::Code::Unimplemented {
            notes.push("reflection v1 unimplemented, trying v1alpha".into());
            result = reflect_v1alpha(ch.clone(), &mut notes).await;
        }
    }
    match result {
        Ok(files) if !files.is_empty() => {
            let mut set = FileDescriptorSet::default();
            for f in &files {
                match FileDescriptorProto::decode(&f[..]) {
                    Ok(d) => set.file.push(d),
                    Err(e) => notes.push(format!("undecodable descriptor: {e}")),
                }
            }
            write(&dir, "descriptor_set.pb", set.encode_to_vec());
            write(&dir, "schema.txt", format!("{set:#?}"));
            write(&dir, "reflection.txt", notes.join("\n"));
            println!("reflection: ok ({} files)", set.file.len());
        }
        Ok(_) => {
            notes.push("reflection returned no files".into());
            write(&dir, "reflection.txt", notes.join("\n"));
            println!("reflection: no descriptors (see reflection.txt)");
        }
        Err(e) => {
            notes.push(format!("reflection failed: {e}"));
            write(&dir, "reflection.txt", notes.join("\n"));
            println!("reflection: unavailable ({})", e.code());
        }
    }

    // 3. ListTriggerableActions (read-only)
    match client::list_actions(ch.clone()).await {
        Ok(list) => {
            let txt: String = list
                .iter()
                .map(|a| format!("{}\t{}\tholdable={}\n", a.action_name, a.category, a.holdable))
                .collect();
            write(&dir, "actions.txt", txt);
            println!("actions: {} listed", list.len());
        }
        Err(e) => {
            write(&dir, "actions.txt", format!("failed: {e}\n"));
            println!("actions: failed ({})", e.code());
        }
    }

    // 4. first frame
    let got = match client::open_telemetry_stream(ch.clone()).await {
        Ok(mut s) => match tokio::time::timeout(Duration::from_secs(args.timeout_secs), s.message()).await {
            Ok(Ok(Some(b))) => Some(b),
            Ok(Ok(None)) => {
                println!("first frame: stream ended without a message");
                None
            }
            Ok(Err(e)) => {
                println!("first frame: stream error: {e}");
                None
            }
            Err(_) => {
                println!("first frame: none within {}s", args.timeout_secs);
                None
            }
        },
        Err(e) => {
            println!("first frame: cannot open stream: {e}");
            None
        }
    };
    // 5. never calls TriggerGameAction
    match got {
        Some(b) => {
            write(&dir, "first_frame.bin", &b[..]);
            let tree = inspect::tree(&b).unwrap_or_else(|e| format!("not a valid protobuf message: {e}\n"));
            write(&dir, "first_frame.txt", &tree);
            let top: Vec<String> = wire::parse_message(&b)
                .map(|f| f.iter().map(|x| x.number.to_string()).collect())
                .unwrap_or_default();
            println!("first frame: {} B, top-level fields: [{}]", b.len(), top.join(", "));
            println!("output: {}", dir.display());
            Ok(0)
        }
        None => {
            println!("output: {}", dir.display());
            Ok(3)
        }
    }
}

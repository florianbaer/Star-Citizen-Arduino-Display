//! Offline inspection subcommands (inspect, diff, decode).
use crate::cli::{DiffArgs, InspectArgs};
use crate::dumpfile::{self, Record};
use crate::pb::inspect;
use anyhow::{bail, Result};

fn frames_of(records: Vec<Record>) -> Vec<Vec<u8>> {
    records
        .into_iter()
        .filter_map(|r| match r {
            Record::Frame { data, .. } => Some(data),
            _ => None,
        })
        .collect()
}

pub fn inspect(args: InspectArgs) -> Result<i32> {
    let records = dumpfile::load_frames_or_raw(&args.file)?;
    match args.frame {
        None => print!("{}", inspect::summary(&records)),
        Some(n) => {
            let frames = frames_of(records);
            let Some(f) = frames.get(n) else {
                bail!("frame {n} out of range (file has {} frames)", frames.len());
            };
            if args.hex {
                println!("{}", f.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" "));
            }
            match inspect::tree(f) {
                Ok(t) => print!("{t}"),
                Err(e) => bail!("frame {n} is not a valid protobuf message: {e}"),
            }
        }
    }
    Ok(0)
}

pub fn diff(args: DiffArgs) -> Result<i32> {
    let frames = frames_of(dumpfile::load_frames_or_raw(&args.file)?);
    let get = |i: usize| frames.get(i).ok_or_else(|| anyhow::anyhow!("frame {i} out of range (file has {} frames)", frames.len()));
    let lines = inspect::diff(get(args.a)?, get(args.b)?, &args.ignore).map_err(|e| anyhow::anyhow!("cannot parse frame: {e}"))?;
    if lines.is_empty() {
        println!("(no differences)");
    }
    for l in lines {
        println!("{l}");
    }
    Ok(0)
}

/// `hud-sender decode FILE [--every N]`: run the Accumulator over a recording.
pub fn decode(args: crate::cli::DecodeArgs) -> Result<i32> {
    use crate::sc::{decode::Accumulator, schema::TelemetryStreamResponse};
    use prost::Message as _;
    use std::time::{Duration, Instant};

    let records = dumpfile::load_frames_or_raw(&args.file)?;
    let base = Instant::now();
    let mut acc = Accumulator::new(Some(Duration::from_millis(3000)));
    let every = args.every.max(1);
    let mut i = 0usize;
    for r in records {
        let Record::Frame { ts_us, data } = r else { continue };
        let now = base + Duration::from_micros(ts_us);
        match TelemetryStreamResponse::decode(&data[..]) {
            Ok(resp) => acc.ingest(resp, now),
            Err(e) => {
                println!("#{i} decode error: {e}");
                i += 1;
                continue;
            }
        }
        if i % every == 0 {
            let s = acc.snapshot(now);
            let f = |v: Option<f32>| v.map_or("-".to_string(), |x| format!("{x:.3}"));
            let (name, faces, lvl, hf, qf, ann, gear) = match &s.vehicle {
                Some(v) => (
                    v.name.clone(),
                    format!("{:?}", v.shield_faces),
                    f(v.shield_level),
                    f(v.hydrogen_fuel),
                    f(v.quantum_fuel),
                    v.annunciators.map_or("-".to_string(), |a| format!("{a:?}")),
                    v.gear.map_or("-".to_string(), |g| g.to_string()),
                ),
                None => ("-".into(), "[]".into(), "-".into(), "-".into(), "-".into(), "-".into(), "-".into()),
            };
            let (health, o2) = match &s.actor {
                Some(a) => (f(a.health), f(a.oxygen)),
                None => ("-".into(), "-".into()),
            };
            println!(
                "#{i} t={}ms link={:?} name={name} faces={faces} lvl={lvl} hf={hf} qf={qf} ann={ann} gear={gear} health={health} o2={o2}",
                ts_us / 1000,
                s.link
            );
        }
        i += 1;
    }
    Ok(0)
}

/// `hud-sender actions list|trigger`
pub async fn actions(cmd: crate::cli::ActionsCmd) -> Result<i32> {
    use crate::cli::ActionsCmd;
    use crate::sc::client;
    use std::time::Duration;
    match cmd {
        ActionsCmd::List { grpc } => {
            let ch = client::connect(&grpc, Duration::from_secs(2)).await?;
            for a in client::list_actions(ch).await.map_err(|e| anyhow::anyhow!("{e}"))? {
                println!("{}	{}	holdable={}", a.action_name, a.category, a.holdable);
            }
            Ok(0)
        }
        ActionsCmd::Trigger { name, hold_ms, grpc, yes } => {
            if !crate::actions::is_allowed(&name) {
                if crate::actions::is_blocked(&name) {
                    eprintln!("blocked");
                } else {
                    eprintln!("not in the built-in allowlist");
                }
                return Ok(1);
            }
            let hold = hold_ms.or_else(|| crate::actions::ACTIONS.iter().find(|a| a.name == name).and_then(|a| a.hold_ms));
            if !yes {
                println!("would call TriggerGameAction({name}, {hold:?})");
                return Ok(1);
            }
            let ch = client::connect(&grpc, Duration::from_secs(2)).await?;
            let r = client::trigger_action(ch, &name, hold).await.map_err(|e| anyhow::anyhow!("{e}"))?;
            println!("status={} message={:?}", r.status, r.message);
            Ok(0)
        }
    }
}

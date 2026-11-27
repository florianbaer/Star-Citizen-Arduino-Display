//! `hud-sender dump`: record the telemetry stream to an SCTD file.
use crate::cli::DumpArgs;
use crate::dumpfile::DumpWriter;
use crate::sc::client;
use std::io::BufRead;
use std::time::{Duration, Instant};

pub async fn run(args: DumpArgs) -> anyhow::Result<i32> {
    let url = client::normalize_endpoint(&args.grpc);
    let ch = match client::connect(&url, Duration::from_secs(2)).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("cannot connect to {url}: {e}");
            return Ok(2);
        }
    };
    let mut stream = match client::open_telemetry_stream(ch).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("cannot open telemetry stream: {e}");
            return Ok(2);
        }
    };
    let file = std::fs::File::create(&args.out)?;
    let mut w = DumpWriter::new(std::io::BufWriter::new(file))?;

    // Plain detached thread: a thread stuck in read_line must not block runtime shutdown.
    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(16);
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            match line {
                Ok(l) => {
                    if tx.blocking_send(l).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });
    let mut rx_open = true;

    let start = Instant::now();
    let deadline = async {
        match args.duration {
            Some(s) => tokio::time::sleep(Duration::from_secs(s)).await,
            None => std::future::pending::<()>().await,
        }
    };
    tokio::pin!(deadline);
    let mut tick = tokio::time::interval(Duration::from_secs(1));
    let (mut frames, mut markers, mut bytes) = (0u64, 0u64, 0u64);
    loop {
        tokio::select! {
            m = stream.message() => match m {
                Ok(Some(b)) => {
                    w.write_frame(start.elapsed().as_micros() as u64, &b)?;
                    frames += 1;
                    bytes += b.len() as u64;
                    if args.max_frames.is_some_and(|n| frames >= n) { break; }
                }
                Ok(None) => { eprintln!("stream ended"); break; }
                Err(e) => { eprintln!("stream error: {e}"); break; }
            },
            line = rx.recv(), if rx_open => match line {
                Some(text) => {
                    w.write_marker(start.elapsed().as_micros() as u64, &text)?;
                    markers += 1;
                }
                None => rx_open = false,
            },
            _ = tick.tick() => eprintln!("frames: {frames}"),
            _ = &mut deadline => break,
            _ = tokio::signal::ctrl_c() => break,
        }
    }
    w.flush()?;
    println!(
        "dump: {frames} frames, {markers} markers, {bytes} bytes, {:.1}s -> {}",
        start.elapsed().as_secs_f32(),
        args.out.display()
    );
    Ok(0)
}

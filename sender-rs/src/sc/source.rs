//! ScSource: connect to the game gRPC server, decode the stream, publish snapshots. Reconnects forever.
use crate::sc::{client, decode::Accumulator, schema::TelemetryStreamResponse};
use crate::snapshot::{Snapshot, SourceKind};
use prost::Message as _;
use std::time::{Duration, Instant};
use tokio::sync::watch;

pub struct ScConfig {
    pub endpoint: String,
    /// Default 2 s.
    pub connect_timeout: Duration,
    pub stale: Option<Duration>,
    /// Default None (off): we do not know whether the game streams continuously.
    pub idle_timeout: Option<Duration>,
}

impl ScConfig {
    pub fn new(endpoint: &str) -> Self {
        Self { endpoint: endpoint.to_string(), connect_timeout: Duration::from_secs(2), stale: Some(Duration::from_secs(3)), idle_timeout: None }
    }
}

fn publish_no_game(tx: &watch::Sender<Snapshot>) {
    tx.send_if_modified(|cur| {
        let ng = Snapshot::no_game(SourceKind::Sc);
        if *cur != ng {
            *cur = ng;
            true
        } else {
            false
        }
    });
}

fn publish(tx: &watch::Sender<Snapshot>, snap: Snapshot) {
    tx.send_if_modified(|cur| {
        if *cur != snap {
            *cur = snap;
            true
        } else {
            false
        }
    });
}

pub async fn run(cfg: ScConfig, tx: watch::Sender<Snapshot>) -> anyhow::Result<()> {
    let mut backoff = Duration::from_millis(500);
    let mut last_err = String::new();
    while !tx.is_closed() {
        publish_no_game(&tx);
        let stream = match client::connect(&cfg.endpoint, cfg.connect_timeout).await {
            Ok(ch) => client::open_telemetry_stream(ch).await.map_err(|e| anyhow::anyhow!("{e}")),
            Err(e) => Err(e),
        };
        let mut stream = match stream {
            Ok(s) => s,
            Err(e) => {
                let msg = format!("{e:#}");
                if msg != last_err {
                    eprintln!("[sc] cannot connect: {msg}");
                    last_err = msg;
                }
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(Duration::from_secs(5));
                continue;
            }
        };
        eprintln!("[sc] connected");
        last_err.clear();
        backoff = Duration::from_millis(500);
        let mut acc = Accumulator::new(cfg.stale);
        let mut decode_errors = 0u64;
        let mut last_msg = Instant::now();
        let mut tick = tokio::time::interval(Duration::from_millis(500));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                m = stream.message() => match m {
                    Ok(Some(b)) => {
                        last_msg = Instant::now();
                        match TelemetryStreamResponse::decode(&b[..]) {
                            Ok(resp) => {
                                let now = Instant::now();
                                acc.ingest(resp, now);
                                publish(&tx, acc.snapshot(now));
                            }
                            Err(e) => {
                                decode_errors += 1;
                                if decode_errors == 1 || decode_errors % 100 == 0 {
                                    eprintln!("[sc] decode error #{decode_errors}: {e}; frame {} B", b.len());
                                }
                            }
                        }
                    }
                    Ok(None) => { eprintln!("[sc] stream ended"); break; }
                    Err(e) => { eprintln!("[sc] stream ended: {e}"); break; }
                },
                _ = tick.tick() => {
                    if tx.is_closed() { return Ok(()); }
                    publish(&tx, acc.snapshot(Instant::now()));
                    if let Some(d) = cfg.idle_timeout {
                        if last_msg.elapsed() > d {
                            eprintln!("[sc] no message for {d:?}; reconnecting");
                            break;
                        }
                    }
                }
            }
        }
        // Avoid a hot reconnect loop when the server closes the stream immediately.
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Ok(())
}

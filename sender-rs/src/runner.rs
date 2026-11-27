//! `hud-sender run`: source -> mapping -> policy -> serial.
use crate::actions::{self, ActionGate, GateDecision};
use crate::cli::{RunArgs, SourceArg};
use crate::mapping::{to_wire, Scales};
use crate::policy::SendPolicy;
use crate::sc::schema::Scale;
use crate::sc::source::{self, ScConfig};
use crate::serial::{self, SerialLink};
use crate::snapshot::{LinkState, Snapshot, SourceKind};
use anyhow::Result;
use hud_proto::frame::{frame, frame_message, MAX_FRAME};
use hud_proto::messages::*;
use std::collections::HashSet;
use std::time::Duration;
use tokio::sync::watch;
use tokio::time::{Instant, MissedTickBehavior};

/// Warns once per field when a raw value exceeds 1.0 although the scale is `unit`.
fn range_warnings(s: &Snapshot, scales: Scales, warned: &mut HashSet<&'static str>) {
    let Some(v) = &s.vehicle else { return };
    let mut check = |field: &'static str, val: Option<f32>, scale: Scale, flag: &str| {
        if let Some(x) = val {
            if scale == Scale::Unit && x.is_finite() && x > 1.0 && warned.insert(field) {
                eprintln!("[map] {field}={x} > 1.0 with --{flag} unit; try --{flag} percent");
            }
        }
    };
    check("shield_level", v.shield_level, scales.shield, "shield-scale");
    check("shield_faces", v.shield_faces.iter().copied().fold(None, |m: Option<f32>, x| Some(m.map_or(x, |m| m.max(x)))), scales.shield, "shield-scale");
    check("hydrogen_fuel", v.hydrogen_fuel, scales.fuel, "fuel-scale");
    check("quantum_fuel", v.quantum_fuel, scales.fuel, "fuel-scale");
}

pub async fn run(args: RunArgs) -> Result<i32> {
    let kind = match args.source {
        SourceArg::Sc => SourceKind::Sc,
        SourceArg::Random => SourceKind::Random,
    };
    let (wtx, wrx) = watch::channel(Snapshot::no_game(kind));
    let source_task = match args.source {
        SourceArg::Sc => {
            let mut cfg = ScConfig::new(&args.grpc);
            cfg.stale = (args.frame_stale_ms > 0).then(|| Duration::from_millis(args.frame_stale_ms));
            cfg.idle_timeout = (args.idle_timeout_secs > 0).then(|| Duration::from_secs(args.idle_timeout_secs));
            tokio::spawn(source::run(cfg, wtx))
        }
        SourceArg::Random => tokio::spawn(crate::source::random::run(Duration::from_secs(args.interval.max(1)), wtx)),
    };

    let SerialLink { tx: link_tx, rx: mut link_rx } = if args.port == "-" {
        serial::stdout_link()
    } else {
        let l = serial::open(&args.port, args.baud)?;
        eprintln!("Connected to {} at {} baud", args.port, args.baud);
        l
    };

    let scales = Scales { shield: args.shield_scale, fuel: args.fuel_scale };
    let mut policy = SendPolicy::new(Duration::from_millis(args.keepalive_ms));
    let mut tick = tokio::time::interval(Duration::from_millis((1000 / args.max_hz.max(1) as u64).max(1)));
    tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let deadline = (args.duration > 0).then(|| Instant::now() + Duration::from_secs(args.duration));
    let mut warned = HashSet::new();
    let mut last_link = LinkState::NoGame;
    let mut hud_version: Option<u8> = None;
    let mut rx_open = true;
    let mut gate = ActionGate::new(args.allow_actions);

    let result: Result<()> = loop {
        tokio::select! {
            _ = tick.tick() => {
                let snap = wrx.borrow().clone();
                if snap.link != last_link {
                    eprintln!("[link] {:?} -> {:?}", last_link, snap.link);
                    last_link = snap.link;
                }
                range_warnings(&snap, scales, &mut warned);
                let frames = to_wire(&snap, scales).frames();
                let mut writer_dead = false;
                for (ty, payload) in policy.plan(std::time::Instant::now(), &frames) {
                    let mut buf = [0u8; MAX_FRAME];
                    let n = frame(ty, &payload, &mut buf);
                    if link_tx.send(buf[..n].to_vec()).is_err() {
                        writer_dead = true;
                        break;
                    }
                }
                if writer_dead {
                    break Err(anyhow::anyhow!("serial port closed"));
                }
            }
            m = link_rx.recv(), if rx_open => match m {
                Some((t, payload)) => {
                    match MsgType::from_u8(t) {
                        Some(MsgType::Hello) => {
                            if let Some(h) = HelloMsg::decode(&payload) {
                                if h.role == ROLE_HUD && hud_version != Some(h.version) {
                                    hud_version = Some(h.version);
                                    eprintln!("[hud] hello v{}", h.version);
                                    if h.version != PROTO_VERSION {
                                        eprintln!("[hud] warning: HUD speaks protocol v{}, sender v{PROTO_VERSION}", h.version);
                                    }
                                }
                            }
                        }
                        Some(MsgType::Action) => {
                            if let Some(a) = ActionMsg::decode(&payload) {
                                handle_action(&args, &mut gate, &link_tx, a);
                            }
                        }
                        _ => {}
                    }
                }
                None => rx_open = false,
            },
            _ = tokio::signal::ctrl_c() => break Ok(()),
            _ = async { match deadline { Some(d) => tokio::time::sleep_until(d).await, None => std::future::pending().await } } => break Ok(()),
        }
    };
    source_task.abort();
    result?;
    Ok(0)
}

fn send_ack(tx: &std::sync::mpsc::Sender<Vec<u8>>, m: &ActionMsg, result: u8, game_status: u8) {
    let ack = ActionAckMsg { action_id: m.action_id, seq: m.seq, result, game_status };
    let mut buf = [0u8; MAX_FRAME];
    let n = frame_message(&ack, &mut buf);
    let _ = tx.send(buf[..n].to_vec());
}

/// Inbound Action from the HUD. With `--source random` everything is ACK_NOT_CONNECTED.
fn handle_action(args: &RunArgs, gate: &mut ActionGate, tx: &std::sync::mpsc::Sender<Vec<u8>>, m: ActionMsg) {
    if args.source != SourceArg::Sc {
        send_ack(tx, &m, ACK_NOT_CONNECTED, 0);
        return;
    }
    match gate.decide(&m, std::time::Instant::now()) {
        GateDecision::Duplicate => {}
        GateDecision::Reject(code) => send_ack(tx, &m, code, 0),
        GateDecision::Execute(def) => {
            let endpoint = args.grpc.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                let (result, status) = actions::execute(&endpoint, def).await;
                send_ack(&tx, &m, result, status);
            });
        }
    }
}

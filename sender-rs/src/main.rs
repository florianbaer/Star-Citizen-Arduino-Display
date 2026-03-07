mod serial;

use anyhow::Result;
use clap::Parser;
use hud_proto::messages::TelemetryMsg;
use rand::Rng;
use std::time::{Duration, Instant};

use serial::HudSerial;

#[derive(Parser)]
#[command(name = "hud-sender", about = "Send random telemetry to the spaceship HUD")]
struct Args {
    /// Serial port (e.g. COM6, /dev/ttyUSB0)
    port: String,

    /// Target change interval in seconds
    #[arg(short, long, default_value_t = 5)]
    interval: u64,

    /// Baud rate
    #[arg(short, long, default_value_t = 115200)]
    baud: u32,
}

fn random_telemetry() -> TelemetryMsg {
    let mut rng = rand::thread_rng();
    TelemetryMsg {
        shield_front: rng.gen(),
        shield_back: rng.gen(),
        shield_left: rng.gen(),
        shield_right: rng.gen(),
        hydrogen_fuel: rng.gen(),
        quantum_fuel: rng.gen(),
    }
}

fn zero_telemetry() -> TelemetryMsg {
    TelemetryMsg {
        shield_front: 0,
        shield_back: 0,
        shield_left: 0,
        shield_right: 0,
        hydrogen_fuel: 0,
        quantum_fuel: 0,
    }
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    let v = a as f32 + (b as f32 - a as f32) * t;
    v.round() as u8
}

fn lerp_telemetry(a: &TelemetryMsg, b: &TelemetryMsg, t: f32) -> TelemetryMsg {
    let t = t.clamp(0.0, 1.0);
    TelemetryMsg {
        shield_front: lerp_u8(a.shield_front, b.shield_front, t),
        shield_back: lerp_u8(a.shield_back, b.shield_back, t),
        shield_left: lerp_u8(a.shield_left, b.shield_left, t),
        shield_right: lerp_u8(a.shield_right, b.shield_right, t),
        hydrogen_fuel: lerp_u8(a.hydrogen_fuel, b.hydrogen_fuel, t),
        quantum_fuel: lerp_u8(a.quantum_fuel, b.quantum_fuel, t),
    }
}

fn fmt_telemetry(t: &TelemetryMsg) -> String {
    format!(
        "SF:{} SB:{} SL:{} SR:{} HF:{} QF:{}",
        t.shield_front,
        t.shield_back,
        t.shield_left,
        t.shield_right,
        t.hydrogen_fuel,
        t.quantum_fuel
    )
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut serial = HudSerial::open(&args.port, args.baud)?;
    println!("Connected to {} at {} baud", args.port, args.baud);
    println!(
        "New target every {}s, interpolating at 60Hz",
        args.interval
    );

    let interval = Duration::from_secs(args.interval);
    let frame = Duration::from_micros(16_667); // ~60Hz

    let mut current = zero_telemetry();
    let mut target = random_telemetry();
    let mut target_time = Instant::now();

    println!("Target: {}", fmt_telemetry(&target));

    loop {
        let elapsed = target_time.elapsed();

        if elapsed >= interval {
            current = lerp_telemetry(&current, &target, 1.0);
            target = random_telemetry();
            target_time = Instant::now();
            println!("New target: {}", fmt_telemetry(&target));
        }

        let t = elapsed.as_secs_f32() / interval.as_secs_f32();
        let frame_val = lerp_telemetry(&current, &target, t);
        serial.send(&frame_val)?;

        std::thread::sleep(frame);
    }
}

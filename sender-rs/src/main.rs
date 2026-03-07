mod protocol;
mod serial;

use anyhow::Result;
use clap::Parser;
use std::time::{Duration, Instant};

use protocol::Telemetry;
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

    let mut current = Telemetry::zero();
    let mut target = Telemetry::random();
    let mut target_time = Instant::now();

    println!("Target: {}", target);

    loop {
        let elapsed = target_time.elapsed();

        if elapsed >= interval {
            current = Telemetry::lerp(&current, &target, 1.0);
            target = Telemetry::random();
            target_time = Instant::now();
            println!("New target: {}", target);
        }

        let t = elapsed.as_secs_f32() / interval.as_secs_f32();
        let frame_val = Telemetry::lerp(&current, &target, t);
        serial.send(&frame_val)?;

        std::thread::sleep(frame);
    }
}

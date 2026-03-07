use anyhow::{Context, Result};
use hud_proto::frame::{self, MAX_FRAME};
use hud_proto::messages::{MsgType, TelemetryMsg};
use serialport::SerialPort;
use std::io::Write;
use std::time::Duration;

pub struct HudSerial {
    port: Box<dyn SerialPort>,
}

impl HudSerial {
    pub fn open(port_name: &str, baud: u32) -> Result<Self> {
        let port = serialport::new(port_name, baud)
            .timeout(Duration::from_secs(1))
            .open()
            .with_context(|| format!("Failed to open serial port {}", port_name))?;
        Ok(Self { port })
    }

    pub fn send(&mut self, msg: &TelemetryMsg) -> Result<()> {
        let mut buf = [0u8; MAX_FRAME];
        let len = frame::frame(MsgType::Telemetry, msg.as_bytes(), &mut buf);
        self.port
            .write_all(&buf[..len])
            .context("Failed to write to serial port")?;
        self.port.flush().context("Failed to flush serial port")?;
        Ok(())
    }
}

use anyhow::{Context, Result};
use serialport::SerialPort;
use std::io::Write;
use std::time::Duration;

use crate::protocol::Telemetry;

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

    pub fn send(&mut self, telemetry: &Telemetry) -> Result<()> {
        let csv = telemetry.to_csv();
        self.port
            .write_all(csv.as_bytes())
            .context("Failed to write to serial port")?;
        self.port.flush().context("Failed to flush serial port")?;
        Ok(())
    }
}

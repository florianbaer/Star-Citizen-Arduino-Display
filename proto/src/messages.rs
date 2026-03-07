/// Message type identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MsgType {
    /// PC -> ESP32: shield and fuel telemetry
    Telemetry = 0x01,
}

impl MsgType {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0x01 => Some(Self::Telemetry),
            _ => None,
        }
    }
}

/// Telemetry message payload (PC -> ESP32).
/// All fields are 0-255.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed)]
pub struct TelemetryMsg {
    pub shield_front: u8,
    pub shield_back: u8,
    pub shield_left: u8,
    pub shield_right: u8,
    pub hydrogen_fuel: u8,
    pub quantum_fuel: u8,
}

impl TelemetryMsg {
    pub const SIZE: usize = 6;

    pub fn as_bytes(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self as *const Self as *const u8, Self::SIZE) }
    }

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < Self::SIZE {
            return None;
        }
        let mut msg = Self {
            shield_front: 0,
            shield_back: 0,
            shield_left: 0,
            shield_right: 0,
            hydrogen_fuel: 0,
            quantum_fuel: 0,
        };
        unsafe {
            core::ptr::copy_nonoverlapping(
                data.as_ptr(),
                &mut msg as *mut Self as *mut u8,
                Self::SIZE,
            );
        }
        Some(msg)
    }
}

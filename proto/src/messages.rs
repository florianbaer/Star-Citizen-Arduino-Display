/// Message type identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MsgType {
    /// PC -> ESP32: shield and fuel telemetry
    Telemetry = 0x01,
    /// PC -> ESP32: attitude / gyroscope data
    Attitude = 0x02,
    /// PC -> ESP32: engine gauges
    Engine = 0x03,
    /// PC -> ESP32: flight data (airspeed, altitude, etc.)
    FlightData = 0x04,
    /// PC -> ESP32: G-force data
    GForce = 0x05,
}

impl MsgType {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0x01 => Some(Self::Telemetry),
            0x02 => Some(Self::Attitude),
            0x03 => Some(Self::Engine),
            0x04 => Some(Self::FlightData),
            0x05 => Some(Self::GForce),
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

/// Attitude message payload (PC -> ESP32).
/// Values are in tenths of degrees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed)]
pub struct AttitudeMsg {
    /// Pitch in tenths of degrees (-1800 to +1800, nose up positive)
    pub pitch: i16,
    /// Roll in tenths of degrees (-1800 to +1800, right wing down positive)
    pub roll: i16,
    /// Heading in tenths of degrees (0 to 3599)
    pub heading: i16,
}

impl AttitudeMsg {
    pub const SIZE: usize = 6;

    pub fn as_bytes(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self as *const Self as *const u8, Self::SIZE) }
    }

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < Self::SIZE {
            return None;
        }
        let mut msg = Self {
            pitch: 0,
            roll: 0,
            heading: 0,
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

/// Engine gauges message payload (PC -> ESP32).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed)]
pub struct EngineMsg {
    pub rpm: u16,
    pub throttle: u8,
    pub fuel_flow: u8,
    pub oil_temp: u8,
    pub oil_press: u8,
}

impl EngineMsg {
    pub const SIZE: usize = 6;

    pub fn as_bytes(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self as *const Self as *const u8, Self::SIZE) }
    }

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < Self::SIZE {
            return None;
        }
        let mut msg: Self = unsafe { core::mem::zeroed() };
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

/// Flight data message payload (PC -> ESP32).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed)]
pub struct FlightDataMsg {
    /// Indicated airspeed in tenths of knots
    pub airspeed: u16,
    /// Altitude in feet (signed)
    pub altitude: i32,
    /// Vertical speed in fpm
    pub vspeed: i16,
    /// Ground speed in tenths of knots
    pub ground_speed: u16,
}

impl FlightDataMsg {
    pub const SIZE: usize = 10;

    pub fn as_bytes(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self as *const Self as *const u8, Self::SIZE) }
    }

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < Self::SIZE {
            return None;
        }
        let mut msg: Self = unsafe { core::mem::zeroed() };
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

/// G-force message payload (PC -> ESP32).
/// Values in hundredths of G.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed)]
pub struct GForceMsg {
    /// Longitudinal G (forward/back)
    pub gforce_x: i16,
    /// Vertical G (normal load, ~100 = 1G in level flight)
    pub gforce_y: i16,
    /// Lateral G (side force)
    pub gforce_z: i16,
}

impl GForceMsg {
    pub const SIZE: usize = 6;

    pub fn as_bytes(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self as *const Self as *const u8, Self::SIZE) }
    }

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < Self::SIZE {
            return None;
        }
        let mut msg: Self = unsafe { core::mem::zeroed() };
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

//! Protocol v2 messages. Explicit byte encoding, no unsafe.

pub const PROTO_VERSION: u8 = 2;

// Telemetry `valid` bits
pub const TV_SHIELD_FRONT: u8 = 0x01;
pub const TV_SHIELD_BACK: u8 = 0x02;
pub const TV_SHIELD_LEFT: u8 = 0x04;
pub const TV_SHIELD_RIGHT: u8 = 0x08;
pub const TV_HYDROGEN: u8 = 0x10;
pub const TV_QUANTUM: u8 = 0x20;
pub const TV_ALL: u8 = 0x3F;

// Annunciator indices
pub const ANN_PWR: usize = 0;
pub const ANN_WPN: usize = 1;
pub const ANN_THR: usize = 2;
pub const ANN_SHLD: usize = 3;
pub const ANN_COOL: usize = 4;
pub const ANN_QDRV: usize = 5;
pub const ANN_RADR: usize = 6;
pub const ANN_PROX: usize = 7;
pub const ANN_HIT: usize = 8;
pub const ANN_MISL: usize = 9;
pub const ANN_COUNT: usize = 10;
/// "no annunciator data"
pub const ANN_UNKNOWN: u8 = 15;

// AnnunciatorState (game enum)
pub const ANN_STATE_UNSPECIFIED: u8 = 0;
pub const ANN_STATE_NOMINAL: u8 = 1;
pub const ANN_STATE_OFF: u8 = 2;
pub const ANN_STATE_MINOR_FLASHING: u8 = 3;
pub const ANN_STATE_MINOR_ON: u8 = 4;
pub const ANN_STATE_MAJOR_FLASHING: u8 = 5;
pub const ANN_STATE_MAJOR_ON: u8 = 6;

// Status flags
pub const SF_LANDED: u8 = 0x01;
pub const SF_QT_SPOOLED: u8 = 0x02;
pub const SF_DECOUPLED: u8 = 0x04;
pub const SF_IN_VEHICLE: u8 = 0x08;
pub const SF_LIGHTS: u8 = 0x10;

pub const GEAR_UNKNOWN: u8 = 0xFF;
pub const MASTER_MODE_UNKNOWN: u8 = 0xFF;

// Vitals valid bits
pub const VV_HEALTH: u8 = 0x01;
pub const VV_OXYGEN: u8 = 0x02;

// Link state / source
pub const LINK_NO_GAME: u8 = 0;
pub const LINK_CONNECTED: u8 = 1;
pub const LINK_IN_VEHICLE: u8 = 2;
pub const SOURCE_SC: u8 = 0;
pub const SOURCE_RANDOM: u8 = 1;

// Hello role
pub const ROLE_SENDER: u8 = 0;
pub const ROLE_HUD: u8 = 1;

// ActionAck result
pub const ACK_OK: u8 = 0;
pub const ACK_UNKNOWN_ID: u8 = 1;
pub const ACK_DISABLED: u8 = 2;
pub const ACK_BLOCKED: u8 = 3;
pub const ACK_RATE_LIMITED: u8 = 4;
pub const ACK_GAME_REJECTED: u8 = 5;
pub const ACK_RPC_ERROR: u8 = 6;
pub const ACK_NOT_CONNECTED: u8 = 7;

/// Message type identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MsgType {
    Telemetry = 0x01,
    Status = 0x02,
    Vitals = 0x03,
    Link = 0x04,
    Hello = 0x05,
    VehicleName = 0x06,
    Action = 0x10,
    ActionAck = 0x11,
}

impl MsgType {
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0x01 => Self::Telemetry,
            0x02 => Self::Status,
            0x03 => Self::Vitals,
            0x04 => Self::Link,
            0x05 => Self::Hello,
            0x06 => Self::VehicleName,
            0x10 => Self::Action,
            0x11 => Self::ActionAck,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Telemetry => "Telemetry",
            Self::Status => "Status",
            Self::Vitals => "Vitals",
            Self::Link => "Link",
            Self::Hello => "Hello",
            Self::VehicleName => "VehicleName",
            Self::Action => "Action",
            Self::ActionAck => "ActionAck",
        }
    }
}

pub trait Message: Sized {
    const TYPE: MsgType;
    /// Bytes written by encode() (maximum, for VehicleName).
    const MAX_SIZE: usize;
    /// decode() returns None below this.
    const MIN_SIZE: usize;
    /// Writes the payload. Panics if out.len() < MAX_SIZE. Returns bytes written.
    fn encode(&self, out: &mut [u8]) -> usize;
    /// Ignores trailing extra bytes; fills documented defaults for missing optional trailing fields.
    fn decode(data: &[u8]) -> Option<Self>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TelemetryMsg {
    pub shield_front: u8,
    pub shield_back: u8,
    pub shield_left: u8,
    pub shield_right: u8,
    pub hydrogen_fuel: u8,
    pub quantum_fuel: u8,
    pub valid: u8,
}

impl Message for TelemetryMsg {
    const TYPE: MsgType = MsgType::Telemetry;
    const MAX_SIZE: usize = 7;
    const MIN_SIZE: usize = 6;
    fn encode(&self, out: &mut [u8]) -> usize {
        out[..7].copy_from_slice(&[
            self.shield_front,
            self.shield_back,
            self.shield_left,
            self.shield_right,
            self.hydrogen_fuel,
            self.quantum_fuel,
            self.valid,
        ]);
        7
    }
    fn decode(d: &[u8]) -> Option<Self> {
        if d.len() < 6 {
            return None;
        }
        Some(Self {
            shield_front: d[0],
            shield_back: d[1],
            shield_left: d[2],
            shield_right: d[3],
            hydrogen_fuel: d[4],
            quantum_fuel: d[5],
            valid: if d.len() >= 7 { d[6] } else { TV_ALL },
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusMsg {
    pub annunciators: [u8; 10],
    pub flags: u8,
    pub flags_valid: u8,
    pub gear: u8,
    pub master_mode: u8,
}

impl StatusMsg {
    /// All annunciators unknown, no flags valid, gear/master mode unknown.
    pub const fn unknown() -> Self {
        Self {
            annunciators: [ANN_UNKNOWN; 10],
            flags: 0,
            flags_valid: 0,
            gear: GEAR_UNKNOWN,
            master_mode: MASTER_MODE_UNKNOWN,
        }
    }
}

impl Message for StatusMsg {
    const TYPE: MsgType = MsgType::Status;
    const MAX_SIZE: usize = 9;
    const MIN_SIZE: usize = 9;
    fn encode(&self, out: &mut [u8]) -> usize {
        for k in 0..5 {
            let lo = self.annunciators[2 * k].min(15);
            let hi = self.annunciators[2 * k + 1].min(15);
            out[k] = lo | (hi << 4);
        }
        out[5] = self.flags;
        out[6] = self.flags_valid;
        out[7] = self.gear;
        out[8] = self.master_mode;
        9
    }
    fn decode(d: &[u8]) -> Option<Self> {
        if d.len() < 9 {
            return None;
        }
        let mut annunciators = [0u8; 10];
        for k in 0..5 {
            annunciators[2 * k] = d[k] & 0x0F;
            annunciators[2 * k + 1] = d[k] >> 4;
        }
        Some(Self { annunciators, flags: d[5], flags_valid: d[6], gear: d[7], master_mode: d[8] })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VitalsMsg {
    pub health: u8,
    pub oxygen: u8,
    pub valid: u8,
}

impl Message for VitalsMsg {
    const TYPE: MsgType = MsgType::Vitals;
    const MAX_SIZE: usize = 3;
    const MIN_SIZE: usize = 3;
    fn encode(&self, out: &mut [u8]) -> usize {
        out[..3].copy_from_slice(&[self.health, self.oxygen, self.valid]);
        3
    }
    fn decode(d: &[u8]) -> Option<Self> {
        if d.len() < 3 {
            return None;
        }
        Some(Self { health: d[0], oxygen: d[1], valid: d[2] })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LinkMsg {
    pub state: u8,
    pub source: u8,
}

impl Message for LinkMsg {
    const TYPE: MsgType = MsgType::Link;
    const MAX_SIZE: usize = 2;
    const MIN_SIZE: usize = 1;
    fn encode(&self, out: &mut [u8]) -> usize {
        out[..2].copy_from_slice(&[self.state, self.source]);
        2
    }
    fn decode(d: &[u8]) -> Option<Self> {
        if d.is_empty() {
            return None;
        }
        Some(Self { state: d[0], source: d.get(1).copied().unwrap_or(0) })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HelloMsg {
    pub version: u8,
    pub role: u8,
}

impl Message for HelloMsg {
    const TYPE: MsgType = MsgType::Hello;
    const MAX_SIZE: usize = 2;
    const MIN_SIZE: usize = 1;
    fn encode(&self, out: &mut [u8]) -> usize {
        out[..2].copy_from_slice(&[self.version, self.role]);
        2
    }
    fn decode(d: &[u8]) -> Option<Self> {
        if d.is_empty() {
            return None;
        }
        Some(Self { version: d[0], role: d.get(1).copied().unwrap_or(0) })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VehicleNameMsg {
    len: u8,
    bytes: [u8; 32],
}

impl VehicleNameMsg {
    pub const MAX_LEN: usize = 32;

    /// Each char in 0x20..=0x7E is kept, any other char becomes b'?'; truncated to 32 bytes.
    pub fn from_str_lossy(s: &str) -> Self {
        let mut bytes = [0u8; 32];
        let mut n = 0;
        for c in s.chars() {
            if n >= Self::MAX_LEN {
                break;
            }
            bytes[n] = if (' '..='~').contains(&c) { c as u8 } else { b'?' };
            n += 1;
        }
        Self { len: n as u8, bytes }
    }

    pub fn as_str(&self) -> &str {
        // Always valid ASCII by construction.
        core::str::from_utf8(&self.bytes[..self.len as usize]).unwrap_or("")
    }
}

impl Default for VehicleNameMsg {
    fn default() -> Self {
        Self { len: 0, bytes: [0; 32] }
    }
}

impl Message for VehicleNameMsg {
    const TYPE: MsgType = MsgType::VehicleName;
    const MAX_SIZE: usize = 32;
    const MIN_SIZE: usize = 0;
    /// Writes only `len` bytes.
    fn encode(&self, out: &mut [u8]) -> usize {
        let n = self.len as usize;
        out[..n].copy_from_slice(&self.bytes[..n]);
        n
    }
    fn decode(d: &[u8]) -> Option<Self> {
        let n = d.len().min(32);
        let mut bytes = [0u8; 32];
        for i in 0..n {
            bytes[i] = if (0x20..=0x7E).contains(&d[i]) { d[i] } else { b'?' };
        }
        Some(Self { len: n as u8, bytes })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionMsg {
    pub action_id: u8,
    pub seq: u8,
}

impl Message for ActionMsg {
    const TYPE: MsgType = MsgType::Action;
    const MAX_SIZE: usize = 2;
    const MIN_SIZE: usize = 2;
    fn encode(&self, out: &mut [u8]) -> usize {
        out[..2].copy_from_slice(&[self.action_id, self.seq]);
        2
    }
    fn decode(d: &[u8]) -> Option<Self> {
        if d.len() < 2 {
            return None;
        }
        Some(Self { action_id: d[0], seq: d[1] })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionAckMsg {
    pub action_id: u8,
    pub seq: u8,
    pub result: u8,
    pub game_status: u8,
}

impl Message for ActionAckMsg {
    const TYPE: MsgType = MsgType::ActionAck;
    const MAX_SIZE: usize = 4;
    const MIN_SIZE: usize = 3;
    fn encode(&self, out: &mut [u8]) -> usize {
        out[..4].copy_from_slice(&[self.action_id, self.seq, self.result, self.game_status]);
        4
    }
    fn decode(d: &[u8]) -> Option<Self> {
        if d.len() < 3 {
            return None;
        }
        Some(Self {
            action_id: d[0],
            seq: d[1],
            result: d[2],
            game_status: d.get(3).copied().unwrap_or(0),
        })
    }
}

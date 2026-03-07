use rand::Rng;
use std::fmt;

pub struct Telemetry {
    pub shield_front: u8,
    pub shield_back: u8,
    pub shield_left: u8,
    pub shield_right: u8,
    pub hydrogen_fuel: u8,
    pub quantum_fuel: u8,
}

impl Telemetry {
    pub fn zero() -> Self {
        Self {
            shield_front: 0,
            shield_back: 0,
            shield_left: 0,
            shield_right: 0,
            hydrogen_fuel: 0,
            quantum_fuel: 0,
        }
    }

    pub fn random() -> Self {
        let mut rng = rand::thread_rng();
        Self {
            shield_front: rng.gen(),
            shield_back: rng.gen(),
            shield_left: rng.gen(),
            shield_right: rng.gen(),
            hydrogen_fuel: rng.gen(),
            quantum_fuel: rng.gen(),
        }
    }

    pub fn lerp(a: &Telemetry, b: &Telemetry, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            shield_front: lerp_u8(a.shield_front, b.shield_front, t),
            shield_back: lerp_u8(a.shield_back, b.shield_back, t),
            shield_left: lerp_u8(a.shield_left, b.shield_left, t),
            shield_right: lerp_u8(a.shield_right, b.shield_right, t),
            hydrogen_fuel: lerp_u8(a.hydrogen_fuel, b.hydrogen_fuel, t),
            quantum_fuel: lerp_u8(a.quantum_fuel, b.quantum_fuel, t),
        }
    }

    pub fn to_csv(&self) -> String {
        format!(
            "SF:{},SB:{},SL:{},SR:{},HF:{},QF:{}\n",
            self.shield_front,
            self.shield_back,
            self.shield_left,
            self.shield_right,
            self.hydrogen_fuel,
            self.quantum_fuel
        )
    }
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    let v = a as f32 + (b as f32 - a as f32) * t;
    v.round() as u8
}

impl fmt::Display for Telemetry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SF:{} SB:{} SL:{} SR:{} HF:{} QF:{}",
            self.shield_front,
            self.shield_back,
            self.shield_left,
            self.shield_right,
            self.hydrogen_fuel,
            self.quantum_fuel
        )
    }
}

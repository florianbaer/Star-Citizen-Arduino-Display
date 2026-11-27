//! Snapshot -> protocol messages.
use crate::sc::schema::{Scale, FACE_ORDER_2, FACE_ORDER_4, FACE_ORDER_6};
use crate::snapshot::{LinkState, SourceKind, Snapshot};
use hud_proto::messages::*;

#[derive(Debug, Clone, Copy)]
pub struct Scales {
    pub shield: Scale,
    pub fuel: Scale,
}

pub struct WireSet {
    pub link: LinkMsg,
    pub telemetry: TelemetryMsg,
    pub status: StatusMsg,
    pub vitals: VitalsMsg,
    pub vehicle_name: VehicleNameMsg,
}

/// None if not finite; else `(v.clamp(0,1) * 255).round()` (f32 math).
pub fn to_u8(v: f32) -> Option<u8> {
    if !v.is_finite() {
        return None;
    }
    Some((v.clamp(0.0, 1.0) * 255.0).round() as u8)
}

pub fn apply_scale(v: f32, s: Scale) -> f32 {
    match s {
        Scale::Unit => v,
        Scale::Percent => v / 100.0,
    }
}

fn min_of(vals: &[f32]) -> f32 {
    vals.iter().copied().fold(f32::INFINITY, f32::min)
}

/// Input already scaled; output [front, back, left, right].
pub fn map_faces(faces: &[f32], level: Option<f32>) -> Option<[f32; 4]> {
    match faces.len() {
        0 => level.map(|l| [l; 4]),
        1 => Some([faces[0]; 4]),
        2 => {
            let front = faces[FACE_ORDER_2[0]];
            let back = faces[FACE_ORDER_2[1]];
            let m = front.min(back);
            Some([front, back, m, m])
        }
        4 => Some([faces[FACE_ORDER_4[0]], faces[FACE_ORDER_4[1]], faces[FACE_ORDER_4[2]], faces[FACE_ORDER_4[3]]]),
        6 => {
            let f = |i: usize| faces[FACE_ORDER_6[i]];
            let (front, back, left, right, top, bottom) = (f(0), f(1), f(2), f(3), f(4), f(5));
            Some([front, back, min_of(&[left, top, bottom]), min_of(&[right, top, bottom])])
        }
        _ => Some([min_of(faces); 4]),
    }
}

fn ann_value(raw: i32) -> u8 {
    if (0..=14).contains(&raw) {
        raw as u8
    } else {
        14
    }
}

fn opt_u8(v: Option<i32>) -> Option<u8> {
    v.filter(|g| (0..=254).contains(g)).map(|g| g as u8)
}

pub fn to_wire(s: &Snapshot, scales: Scales) -> WireSet {
    let link = LinkMsg {
        state: match s.link {
            LinkState::NoGame => LINK_NO_GAME,
            LinkState::Connected => LINK_CONNECTED,
            LinkState::InVehicle => LINK_IN_VEHICLE,
        },
        source: match s.source {
            SourceKind::Sc => SOURCE_SC,
            SourceKind::Random => SOURCE_RANDOM,
        },
    };

    let mut telemetry = TelemetryMsg::default();
    let mut status = StatusMsg::unknown();
    let mut vehicle_name = VehicleNameMsg::default();

    if let Some(v) = &s.vehicle {
        let faces: Vec<f32> = v.shield_faces.iter().map(|f| apply_scale(*f, scales.shield)).collect();
        let level = v.shield_level.map(|l| apply_scale(l, scales.shield));
        if let Some(m) = map_faces(&faces, level) {
            let slots = [
                (TV_SHIELD_FRONT, &mut telemetry.shield_front),
                (TV_SHIELD_BACK, &mut telemetry.shield_back),
                (TV_SHIELD_LEFT, &mut telemetry.shield_left),
                (TV_SHIELD_RIGHT, &mut telemetry.shield_right),
            ];
            let mut valid = 0u8;
            for ((bit, slot), val) in slots.into_iter().zip(m) {
                if let Some(b) = to_u8(val) {
                    *slot = b;
                    valid |= bit;
                }
            }
            telemetry.valid |= valid;
        }
        if let Some(b) = v.hydrogen_fuel.and_then(|f| to_u8(apply_scale(f, scales.fuel))) {
            telemetry.hydrogen_fuel = b;
            telemetry.valid |= TV_HYDROGEN;
        }
        if let Some(b) = v.quantum_fuel.and_then(|f| to_u8(apply_scale(f, scales.fuel))) {
            telemetry.quantum_fuel = b;
            telemetry.valid |= TV_QUANTUM;
        }

        if let Some(a) = v.annunciators {
            for i in 0..ANN_COUNT {
                status.annunciators[i] = ann_value(a[i]);
            }
        }
        let mut flag = |bit: u8, val: Option<bool>| {
            if let Some(b) = val {
                status.flags_valid |= bit;
                if b {
                    status.flags |= bit;
                }
            }
        };
        flag(SF_LANDED, v.is_landed);
        flag(SF_QT_SPOOLED, v.qt_spooled);
        flag(SF_DECOUPLED, v.decoupled);
        flag(SF_IN_VEHICLE, Some(true));
        flag(SF_LIGHTS, v.lights);
        status.gear = opt_u8(v.gear).unwrap_or(GEAR_UNKNOWN);
        status.master_mode = opt_u8(v.master_mode).unwrap_or(MASTER_MODE_UNKNOWN);
        vehicle_name = VehicleNameMsg::from_str_lossy(&v.name);
    } else if s.link != LinkState::NoGame {
        status.flags_valid |= SF_IN_VEHICLE; // valid, flag clear
    }

    let mut vitals = VitalsMsg::default();
    if let Some(a) = &s.actor {
        if let Some(b) = a.health.and_then(to_u8) {
            vitals.health = b;
            vitals.valid |= VV_HEALTH;
        }
        if let Some(b) = a.oxygen.and_then(to_u8) {
            vitals.oxygen = b;
            vitals.valid |= VV_OXYGEN;
        }
    }

    WireSet { link, telemetry, status, vitals, vehicle_name }
}

fn enc<M: Message>(m: &M) -> (MsgType, Vec<u8>) {
    let mut buf = [0u8; 64];
    let n = m.encode(&mut buf);
    (M::TYPE, buf[..n].to_vec())
}

impl WireSet {
    /// Order: Link, Telemetry, Status, Vitals, VehicleName.
    pub fn frames(&self) -> Vec<(MsgType, Vec<u8>)> {
        vec![enc(&self.link), enc(&self.telemetry), enc(&self.status), enc(&self.vitals), enc(&self.vehicle_name)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sc::{decode::Accumulator, schema::TelemetryStreamResponse};
    use crate::testvec::*;
    use prost::Message as _;
    use std::time::Instant;

    const UNIT: Scales = Scales { shield: Scale::Unit, fuel: Scale::Unit };

    #[test]
    fn to_u8_cases() {
        assert_eq!(to_u8(0.0), Some(0));
        assert_eq!(to_u8(1.0), Some(255));
        assert_eq!(to_u8(0.5), Some(128));
        assert_eq!(to_u8(0.25), Some(64));
        assert_eq!(to_u8(1.5), Some(255));
        assert_eq!(to_u8(-1.0), Some(0));
        assert_eq!(to_u8(f32::NAN), None);
        assert_eq!(to_u8(f32::INFINITY), None);
    }

    #[test]
    fn scale() {
        assert_eq!(apply_scale(57.0, Scale::Percent), 0.57);
        assert_eq!(apply_scale(0.3, Scale::Unit), 0.3);
    }

    #[test]
    fn faces_table() {
        assert_eq!(map_faces(&[], None), None);
        assert_eq!(map_faces(&[], Some(0.7)), Some([0.7; 4]));
        assert_eq!(map_faces(&[0.4], None), Some([0.4; 4]));
        assert_eq!(map_faces(&[0.9, 0.3], None), Some([0.9, 0.3, 0.3, 0.3]));
        assert_eq!(map_faces(&[0.1, 0.2, 0.3, 0.4], None), Some([0.1, 0.2, 0.3, 0.4]));
        // front, back, left, right, top, bottom
        assert_eq!(map_faces(&[0.9, 0.8, 0.7, 0.6, 0.5, 0.65], None), Some([0.9, 0.8, 0.5, 0.5]));
        assert_eq!(map_faces(&[0.9, 0.8, 0.2, 0.6, 0.5, 0.65], None), Some([0.9, 0.8, 0.2, 0.5]));
        assert_eq!(map_faces(&[0.9, 0.2, 0.7], None), Some([0.2; 4]));
    }

    #[test]
    fn annunciator_clamp_and_name() {
        assert_eq!(ann_value(20), 14);
        assert_eq!(ann_value(-1), 14);
        assert_eq!(ann_value(6), 6);
        assert_eq!(VehicleNameMsg::from_str_lossy("\u{dc}n\u{ef}code").as_str(), "?n?code");
        assert_eq!(VehicleNameMsg::from_str_lossy(&"x".repeat(40)).as_str().len(), 32);
    }

    fn snap_of(frames: &[&[u8]]) -> Snapshot {
        let t = Instant::now();
        let mut acc = Accumulator::new(None);
        for f in frames {
            acc.ingest(TelemetryStreamResponse::decode(*f).unwrap(), t);
        }
        acc.snapshot(t)
    }

    #[test]
    fn vehicle_frame_to_wire() {
        let w = to_wire(&snap_of(&[VEHICLE_FRAME]), UNIT);
        let t = w.telemetry;
        assert_eq!(
            [t.shield_front, t.shield_back, t.shield_left, t.shield_right, t.hydrogen_fuel, t.quantum_fuel],
            [255, 128, 64, 0, 153, 102]
        );
        assert_eq!(t.valid, 0x3F);
        let mut b = [0u8; 9];
        w.status.encode(&mut b);
        assert_eq!(&b[..5], &[0x01, 0x60, 0x00, 0x00, 0x05]);
        assert_eq!(w.status.flags, 0x0B);
        assert_eq!(w.status.flags_valid, 0x1B);
        assert_eq!(w.status.gear, 3);
        assert_eq!(w.status.master_mode, 2);
        assert_eq!(w.vehicle_name.as_str(), "Gladius");
        assert_eq!(w.vitals.valid, 0);
        assert_eq!(w.link, LinkMsg { state: LINK_IN_VEHICLE, source: SOURCE_SC });
    }

    #[test]
    fn health_only_to_wire() {
        let w = to_wire(&snap_of(&[HEALTH_FRAME]), UNIT);
        assert_eq!(w.link.state, LINK_CONNECTED);
        assert_eq!(w.telemetry, TelemetryMsg::default());
        assert_eq!(w.vitals.valid, VV_HEALTH);
        assert_eq!(w.vitals.health, 191); // 0.75 * 255 = 191.25
        assert_eq!(w.status.flags_valid, SF_IN_VEHICLE);
        assert_eq!(w.status.flags, 0);
        assert_eq!(w.status.annunciators, [ANN_UNKNOWN; 10]);
        assert_eq!(w.vehicle_name.as_str(), "");
    }

    #[test]
    fn no_game_to_wire() {
        let w = to_wire(&Snapshot::no_game(SourceKind::Sc), UNIT);
        assert_eq!(w.status, StatusMsg::unknown());
        assert_eq!(w.link.state, LINK_NO_GAME);
    }

    #[test]
    fn frames_order() {
        let w = to_wire(&Snapshot::no_game(SourceKind::Random), UNIT);
        let ty: Vec<MsgType> = w.frames().into_iter().map(|f| f.0).collect();
        assert_eq!(ty, vec![MsgType::Link, MsgType::Telemetry, MsgType::Status, MsgType::Vitals, MsgType::VehicleName]);
        assert_eq!(w.link.source, SOURCE_RANDOM);
    }
}

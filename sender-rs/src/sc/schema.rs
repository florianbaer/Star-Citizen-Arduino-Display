//! Star Citizen telemetry schema subset. SOURCE OF TRUTH for all field numbers.
//! Provenance: Appendix A of plan.md (sc_make_proto extraction; 3 identical public copies),
//! saved as schema/sc_telemetry_community.proto.
//! Health path 6.1.1.{1,2} CONFIRMED (two sources agree); everything else COMMUNITY unless noted.
//! Nothing here has been verified against a live game by this project.
//!
//! Provenance tags: CONFIRMED = reported by two sources (not verified live by this project);
//! COMMUNITY = from the extracted schema only; ASSUMPTION = our guess, must be verified live.

pub const SERVICE: &str = "sc.game.telemetry.v1.GameTelemetryService"; // CONFIRMED
pub const PATH_TELEMETRY_STREAM: &str = "/sc.game.telemetry.v1.GameTelemetryService/TelemetryStream"; // CONFIRMED
pub const PATH_TRIGGER_GAME_ACTION: &str = "/sc.game.telemetry.v1.GameTelemetryService/TriggerGameAction"; // COMMUNITY
pub const PATH_LIST_TRIGGERABLE_ACTIONS: &str = "/sc.game.telemetry.v1.GameTelemetryService/ListTriggerableActions"; // COMMUNITY
pub const DEFAULT_ENDPOINT: &str = "http://127.0.0.1:2942"; // CONFIRMED

#[derive(Clone, PartialEq, prost::Message)]
pub struct TelemetryStreamResponse {
    #[prost(message, optional, tag = "3")]
    pub vehicle: Option<VehicleFrame>,
    #[prost(message, optional, tag = "5")]
    pub events: Option<EventsFrame>,
    #[prost(message, optional, tag = "6")]
    pub actor: Option<ActorFrame>, // CONFIRMED (tag 6)
}
#[derive(Clone, PartialEq, prost::Message)]
pub struct VehicleFrame {
    #[prost(message, optional, tag = "2")]
    pub state: Option<VehicleState>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub struct VehicleState {
    #[prost(string, tag = "1")]
    pub name: String,
    #[prost(string, tag = "2")]
    pub class_name: String,
    #[prost(string, tag = "3")]
    pub manufacturer: String,
    #[prost(int32, tag = "4")]
    pub vehicle_type: i32,
    #[prost(bool, tag = "5")]
    pub lights_enabled: bool,
    #[prost(message, optional, tag = "7")]
    pub annunciators: Option<VehicleAnnunciator>,
    #[prost(message, optional, tag = "8")]
    pub flight: Option<VehicleFlight>,
    #[prost(message, optional, tag = "10")]
    pub shields: Option<VehicleShields>,
    #[prost(float, tag = "11")]
    pub health: f32,
    #[prost(message, optional, tag = "12")]
    pub master_mode: Option<VehicleMasterModeState>,
}
/// Each value: AnnunciatorState enum (ANN_STATE_*), encoded as an int32 varint.
#[derive(Clone, PartialEq, prost::Message)]
pub struct VehicleAnnunciator {
    #[prost(int32, tag = "1")]
    pub pwr: i32,
    #[prost(int32, tag = "2")]
    pub wpn: i32,
    #[prost(int32, tag = "3")]
    pub thr: i32,
    #[prost(int32, tag = "4")]
    pub shld: i32,
    #[prost(int32, tag = "5")]
    pub cool: i32,
    #[prost(int32, tag = "6")]
    pub qdrv: i32,
    #[prost(int32, tag = "7")]
    pub radr: i32,
    #[prost(int32, tag = "8")]
    pub prox: i32,
    #[prost(int32, tag = "9")]
    pub hit: i32,
    #[prost(int32, tag = "10")]
    pub misl: i32,
}
#[derive(Clone, PartialEq, prost::Message)]
pub struct VehicleFlight {
    #[prost(message, optional, tag = "1")]
    pub state: Option<VehicleFlightState>,
    #[prost(message, optional, tag = "2")]
    pub settings: Option<VehicleFlightSettings>,
    #[prost(message, optional, tag = "3")]
    pub instruments: Option<VehicleFlightInstruments>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub struct VehicleFlightState {
    #[prost(bool, tag = "1")]
    pub is_landed: bool,
    /// VehicleComponentDeploymentState
    #[prost(int32, optional, tag = "2")]
    pub gear: Option<i32>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub struct VehicleFlightSettings {
    #[prost(message, optional, tag = "1")]
    pub ifcs_coupled: Option<Availability>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub struct Availability {
    #[prost(bool, tag = "1")]
    pub enabled: bool,
    #[prost(bool, tag = "2")]
    pub active: bool,
}
#[derive(Clone, PartialEq, prost::Message)]
pub struct VehicleFlightInstruments {
    #[prost(float, tag = "1")]
    pub speed: f32,
    #[prost(float, tag = "2")]
    pub hydrogen_fuel: f32,
    #[prost(float, tag = "3")]
    pub quantum_fuel: f32,
    #[prost(message, optional, tag = "4")]
    pub g_meter: Option<Gauge>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub struct VehicleShields {
    #[prost(float, tag = "1")]
    pub level: f32,
    #[prost(float, repeated, tag = "2")]
    pub faces: Vec<f32>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub struct VehicleMasterModeState {
    #[prost(int32, tag = "1")]
    pub mode: i32,
    #[prost(bool, tag = "2")]
    pub spooled: bool,
}
#[derive(Clone, PartialEq, prost::Message)]
pub struct EventsFrame {
    #[prost(message, repeated, tag = "1")]
    pub events: Vec<GameEvent>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub struct GameEvent {
    #[prost(int32, tag = "1")]
    pub event_type: i32,
}
#[derive(Clone, PartialEq, prost::Message)]
pub struct ActorFrame {
    #[prost(message, optional, tag = "1")]
    pub vitals: Option<ActorVitals>, // CONFIRMED
    #[prost(message, optional, tag = "2")]
    pub state: Option<ActorState>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub struct ActorVitals {
    #[prost(message, optional, tag = "1")]
    pub health: Option<Gauge>, // CONFIRMED
    #[prost(message, optional, tag = "2")]
    pub oxygen: Option<Gauge>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub struct Gauge {
    #[prost(float, tag = "1")]
    pub current: f32,
    #[prost(float, tag = "2")]
    pub max: f32,
} // CONFIRMED
#[derive(Clone, PartialEq, prost::Message)]
pub struct ActorState {
    /// ActorLocomotionState
    #[prost(int32, tag = "1")]
    pub locomotion: i32,
    #[prost(int32, tag = "2")]
    pub crew_role: i32,
    #[prost(string, tag = "3")]
    pub seat_name: String,
}
#[derive(Clone, PartialEq, prost::Message)]
pub struct TriggerGameActionRequest {
    #[prost(string, tag = "1")]
    pub action_name: String,
    #[prost(uint32, optional, tag = "2")]
    pub hold_ms: Option<u32>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub struct TriggerGameActionResponse {
    #[prost(int32, tag = "1")]
    pub status: i32,
    #[prost(string, tag = "2")]
    pub message: String,
}
#[derive(Clone, PartialEq, prost::Message)]
pub struct TriggerableAction {
    #[prost(string, tag = "1")]
    pub action_name: String,
    #[prost(string, tag = "2")]
    pub category: String,
    #[prost(bool, tag = "3")]
    pub holdable: bool,
}
#[derive(Clone, PartialEq, prost::Message)]
pub struct ListTriggerableActionsResponse {
    #[prost(message, repeated, tag = "1")]
    pub actions: Vec<TriggerableAction>,
}

// Enum values (COMMUNITY)
pub const DEPLOY_UNSPECIFIED: i32 = 0;
pub const DEPLOY_RETRACTED: i32 = 1;
pub const DEPLOY_DEPLOYING: i32 = 2;
pub const DEPLOY_DEPLOYED: i32 = 3;
pub const DEPLOY_RETRACTING: i32 = 4;
pub const LOCOMOTION_UNSPECIFIED: i32 = 0;
pub const LOCOMOTION_ON_FOOT: i32 = 1;
pub const LOCOMOTION_EVA: i32 = 2;
pub const LOCOMOTION_SEATED: i32 = 3;
pub const EVENT_ACTOR_SPAWN: i32 = 1;
pub const EVENT_ACTOR_DEAD: i32 = 2;
pub const EVENT_ACTOR_ENTER_SEAT: i32 = 3;
pub const EVENT_ACTOR_LEAVE_SEAT: i32 = 4;
pub const EVENT_ACTOR_RESPAWN: i32 = 5;
pub const EVENT_ACTOR_DOWNED: i32 = 6;
pub const TRIGGER_STATUS_OK: i32 = 1;

// ---- ASSUMPTIONS (verify live; see schema/fields.md) ----

/// ASSUMPTION: units of shields.level / shields.faces[] and hydrogen/quantum fuel. Overridable via CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Scale {
    /// value already 0..1
    Unit,
    /// value 0..100
    Percent,
}
pub const DEFAULT_SHIELD_SCALE: Scale = Scale::Unit;
pub const DEFAULT_FUEL_SCALE: Scale = Scale::Unit;
/// ASSUMPTION: indices into faces[] for [front, back, left, right] (4-face ships).
pub const FACE_ORDER_4: [usize; 4] = [0, 1, 2, 3];
/// ASSUMPTION: indices into faces[] for [front, back, left, right, top, bottom] (6-face ships).
pub const FACE_ORDER_6: [usize; 6] = [0, 1, 2, 3, 4, 5];
/// ASSUMPTION: for 2-face ships faces = [front, back].
pub const FACE_ORDER_2: [usize; 2] = [0, 1];
/// ASSUMPTION: ifcs_coupled.active == true means "coupled", so decoupled = !active.
pub const DECOUPLED_WHEN_COUPLED_ACTIVE_IS: bool = false;

/// Path -> name table for `inspect` annotations (indices ignored).
pub const PATH_NAMES: &[(&[u32], &str)] = &[
    (&[3], "vehicle"),
    (&[3, 2], "vehicle.state"),
    (&[3, 2, 1], "vehicle.state.name"),
    (&[3, 2, 2], "vehicle.state.class_name"),
    (&[3, 2, 3], "vehicle.state.manufacturer"),
    (&[3, 2, 4], "vehicle.state.type"),
    (&[3, 2, 5], "vehicle.state.lights_enabled"),
    (&[3, 2, 7], "vehicle.state.annunciators"),
    (&[3, 2, 7, 1], "vehicle.state.annunciators.pwr"),
    (&[3, 2, 7, 2], "vehicle.state.annunciators.wpn"),
    (&[3, 2, 7, 3], "vehicle.state.annunciators.thr"),
    (&[3, 2, 7, 4], "vehicle.state.annunciators.shld"),
    (&[3, 2, 7, 5], "vehicle.state.annunciators.cool"),
    (&[3, 2, 7, 6], "vehicle.state.annunciators.qdrv"),
    (&[3, 2, 7, 7], "vehicle.state.annunciators.radr"),
    (&[3, 2, 7, 8], "vehicle.state.annunciators.prox"),
    (&[3, 2, 7, 9], "vehicle.state.annunciators.hit"),
    (&[3, 2, 7, 10], "vehicle.state.annunciators.misl"),
    (&[3, 2, 8, 1, 1], "vehicle.state.flight.state.is_landed"),
    (&[3, 2, 8, 1, 2], "vehicle.state.flight.state.gear"),
    (&[3, 2, 8, 2, 1], "vehicle.state.flight.settings.ifcs_coupled"),
    (&[3, 2, 8, 3, 1], "vehicle.state.flight.instruments.speed"),
    (&[3, 2, 8, 3, 2], "vehicle.state.flight.instruments.hydrogen_fuel"),
    (&[3, 2, 8, 3, 3], "vehicle.state.flight.instruments.quantum_fuel"),
    (&[3, 2, 8, 3, 4], "vehicle.state.flight.instruments.g_meter"),
    (&[3, 2, 10, 1], "vehicle.state.shields.level"),
    (&[3, 2, 10, 2], "vehicle.state.shields.faces"),
    (&[3, 2, 11], "vehicle.state.health"),
    (&[3, 2, 12, 1], "vehicle.state.master_mode.mode"),
    (&[3, 2, 12, 2], "vehicle.state.master_mode.spooled"),
    (&[4], "weapons"),
    (&[5], "events"),
    (&[5, 1, 1], "events.type"),
    (&[6], "actor"),
    (&[6, 1, 1, 1], "actor.vitals.health.current"),
    (&[6, 1, 1, 2], "actor.vitals.health.max"),
    (&[6, 1, 2, 1], "actor.vitals.oxygen.current"),
    (&[6, 1, 2, 2], "actor.vitals.oxygen.max"),
    (&[6, 2, 1], "actor.state.locomotion"),
    (&[7], "force_feedback"),
    (&[8], "turret"),
];

/// Looks up a name for a path given as field numbers (indices already dropped).
pub fn path_name(path: &[u32]) -> Option<&'static str> {
    PATH_NAMES.iter().find(|(p, _)| *p == path).map(|(_, n)| *n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use prost::Message;

    const HEALTH_FRAME: &[u8] = crate::testvec::HEALTH_FRAME;

    #[test]
    fn decodes_health_frame() {
        let r = TelemetryStreamResponse::decode(HEALTH_FRAME).unwrap();
        let g = r.actor.unwrap().vitals.unwrap().health.unwrap();
        assert_eq!((g.current, g.max), (75.0, 100.0));
        assert!(r.vehicle.is_none());
    }

    #[test]
    fn decodes_vehicle_frame() {
        let r = TelemetryStreamResponse::decode(crate::testvec::VEHICLE_FRAME).unwrap();
        let s = r.vehicle.unwrap().state.unwrap();
        assert_eq!(s.name, "Gladius");
        let a = s.annunciators.unwrap();
        assert_eq!((a.pwr, a.shld, a.hit), (1, 6, 5));
        let f = s.flight.unwrap();
        assert!(f.state.as_ref().unwrap().is_landed);
        assert_eq!(f.state.unwrap().gear, Some(3));
        let ins = f.instruments.unwrap();
        assert_eq!(ins.hydrogen_fuel, 0.6);
        assert_eq!(ins.quantum_fuel, 0.4);
        let sh = s.shields.unwrap();
        assert_eq!(sh.level, 0.8);
        assert_eq!(sh.faces, vec![1.0, 0.5, 0.25, 0.0]);
        let m = s.master_mode.unwrap();
        assert_eq!((m.mode, m.spooled), (2, true));
    }

    #[test]
    fn trigger_request_vectors() {
        let r = TriggerGameActionRequest { action_name: "v_invoke_ping".into(), hold_ms: None };
        assert_eq!(r.encode_to_vec(), crate::testvec::TRIGGER_REQ_NOHOLD);
        let r = TriggerGameActionRequest { action_name: "v_invoke_ping".into(), hold_ms: Some(500) };
        assert_eq!(r.encode_to_vec(), crate::testvec::TRIGGER_REQ_HOLD500);
        assert_eq!(TriggerGameActionResponse::decode(crate::testvec::TRIGGER_RESP_OK).unwrap().status, 1);
    }

    #[test]
    fn path_name_lookup() {
        assert_eq!(path_name(&[6, 1, 1, 1]), Some("actor.vitals.health.current"));
        assert_eq!(path_name(&[99]), None);
    }
}

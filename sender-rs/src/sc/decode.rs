//! Accumulator: TelemetryStreamResponse messages -> Snapshot.
//!
//! ASSUMPTIONS about how the game fills the stream (we do not know whether sub-frames are sent in every
//! message or only on change, or what is sent after leaving a ship): a message may carry only some
//! sub-frames, so we merge across messages per sub-frame (whole sub-frame replacement, not field-level
//! merge); an empty VehicleFrame clears the vehicle; seat-leave / death / respawn events clear it;
//! a stored sub-frame older than the stale timeout counts as absent.
//! Presence (proto3): a scalar whose parent message is present but absent on the wire is its default.
use crate::sc::schema::*;
use crate::snapshot::*;
use std::time::{Duration, Instant};

pub struct Accumulator {
    vehicle: Option<(Instant, VehicleState)>,
    actor: Option<(Instant, ActorFrame)>,
    stale: Option<Duration>,
}

/// Gauge to 0..1: `(current/max).clamp(0,1)` if max > 0 and finite, else None.
fn gauge_ratio(g: &Gauge) -> Option<f32> {
    if g.max > 0.0 && g.max.is_finite() && g.current.is_finite() {
        Some((g.current / g.max).clamp(0.0, 1.0))
    } else {
        None
    }
}

pub fn vehicle_snap(v: &VehicleState) -> VehicleSnap {
    let flight = v.flight.as_ref();
    let flight_state = flight.and_then(|f| f.state.as_ref());
    let instruments = flight.and_then(|f| f.instruments.as_ref());
    VehicleSnap {
        name: v.name.clone(),
        shield_level: v.shields.as_ref().map(|s| s.level),
        shield_faces: v.shields.as_ref().map(|s| s.faces.clone()).unwrap_or_default(),
        hydrogen_fuel: instruments.map(|i| i.hydrogen_fuel),
        quantum_fuel: instruments.map(|i| i.quantum_fuel),
        annunciators: v.annunciators.as_ref().map(|a| [a.pwr, a.wpn, a.thr, a.shld, a.cool, a.qdrv, a.radr, a.prox, a.hit, a.misl]),
        is_landed: flight_state.map(|s| s.is_landed),
        gear: flight_state.and_then(|s| s.gear),
        decoupled: flight
            .and_then(|f| f.settings.as_ref())
            .and_then(|s| s.ifcs_coupled.as_ref())
            .map(|a| a.active == DECOUPLED_WHEN_COUPLED_ACTIVE_IS),
        master_mode: v.master_mode.as_ref().map(|m| m.mode),
        qt_spooled: v.master_mode.as_ref().map(|m| m.spooled),
        lights: Some(v.lights_enabled),
    }
}

pub fn actor_snap(a: &ActorFrame) -> ActorSnap {
    ActorSnap {
        health: a.vitals.as_ref().and_then(|v| v.health.as_ref()).and_then(gauge_ratio),
        oxygen: a.vitals.as_ref().and_then(|v| v.oxygen.as_ref()).and_then(gauge_ratio),
        locomotion: a.state.as_ref().map(|s| s.locomotion).unwrap_or(0),
    }
}

impl Accumulator {
    pub fn new(stale: Option<Duration>) -> Self {
        Self { vehicle: None, actor: None, stale }
    }

    pub fn ingest(&mut self, resp: TelemetryStreamResponse, now: Instant) {
        match resp.vehicle {
            Some(VehicleFrame { state: Some(s) }) => self.vehicle = Some((now, s)),
            Some(VehicleFrame { state: None }) => self.vehicle = None,
            None => {}
        }
        if let Some(a) = resp.actor {
            self.actor = Some((now, a));
        }
        if let Some(ev) = resp.events {
            if ev
                .events
                .iter()
                .any(|e| matches!(e.event_type, EVENT_ACTOR_LEAVE_SEAT | EVENT_ACTOR_DEAD | EVENT_ACTOR_RESPAWN))
            {
                self.vehicle = None;
            }
        }
    }

    fn fresh(&self, t: Instant, now: Instant) -> bool {
        match self.stale {
            Some(s) => now.saturating_duration_since(t) <= s,
            None => true,
        }
    }

    /// Link is Connected or InVehicle; source = Sc.
    pub fn snapshot(&self, now: Instant) -> Snapshot {
        let actor = self.actor.as_ref().filter(|(t, _)| self.fresh(*t, now)).map(|(_, a)| actor_snap(a));
        let vehicle = self.vehicle.as_ref().filter(|(t, _)| self.fresh(*t, now)).map(|(_, v)| v);
        let on_foot = actor
            .as_ref()
            .is_some_and(|a| matches!(a.locomotion, LOCOMOTION_ON_FOOT | LOCOMOTION_EVA));
        let in_vehicle = vehicle.filter(|v| !v.name.is_empty() && !on_foot);
        Snapshot {
            link: if in_vehicle.is_some() { LinkState::InVehicle } else { LinkState::Connected },
            source: SourceKind::Sc,
            vehicle: in_vehicle.map(vehicle_snap),
            actor,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testvec::*;
    use prost::Message;

    fn ingest(acc: &mut Accumulator, b: &[u8], now: Instant) {
        acc.ingest(TelemetryStreamResponse::decode(b).unwrap(), now);
    }

    #[test]
    fn health_frame() {
        let t0 = Instant::now();
        let mut acc = Accumulator::new(None);
        ingest(&mut acc, HEALTH_FRAME, t0);
        let s = acc.snapshot(t0);
        assert_eq!(s.link, LinkState::Connected);
        assert_eq!(s.actor.unwrap().health, Some(0.75));
        assert!(s.vehicle.is_none());
        assert_eq!(s.source, SourceKind::Sc);
    }

    #[test]
    fn vehicle_frame() {
        let t0 = Instant::now();
        let mut acc = Accumulator::new(None);
        ingest(&mut acc, VEHICLE_FRAME, t0);
        let s = acc.snapshot(t0);
        assert_eq!(s.link, LinkState::InVehicle);
        let v = s.vehicle.unwrap();
        assert_eq!(v.name, "Gladius");
        assert_eq!(v.shield_level, Some(0.8));
        assert_eq!(v.shield_faces, vec![1.0, 0.5, 0.25, 0.0]);
        assert_eq!(v.hydrogen_fuel, Some(0.6));
        assert_eq!(v.quantum_fuel, Some(0.4));
        assert_eq!(v.is_landed, Some(true));
        assert_eq!(v.gear, Some(3));
        assert_eq!(v.decoupled, None);
        assert_eq!(v.master_mode, Some(2));
        assert_eq!(v.qt_spooled, Some(true));
        assert_eq!(v.annunciators, Some([1, 0, 0, 6, 0, 0, 0, 0, 5, 0]));
        assert_eq!(v.lights, Some(false));
    }

    #[test]
    fn on_foot_overrides_vehicle() {
        let t0 = Instant::now();
        let mut acc = Accumulator::new(None);
        ingest(&mut acc, VEHICLE_FRAME, t0);
        ingest(&mut acc, ACTOR_ON_FOOT, t0);
        let s = acc.snapshot(t0);
        assert_eq!(s.link, LinkState::Connected);
        assert!(s.vehicle.is_none());
    }

    #[test]
    fn staleness() {
        let t0 = Instant::now();
        let mut acc = Accumulator::new(Some(Duration::from_secs(3)));
        ingest(&mut acc, VEHICLE_FRAME, t0);
        assert_eq!(acc.snapshot(t0 + Duration::from_millis(3100)).link, LinkState::Connected);
        assert_eq!(acc.snapshot(t0 + Duration::from_millis(2900)).link, LinkState::InVehicle);
        let mut acc = Accumulator::new(None);
        ingest(&mut acc, VEHICLE_FRAME, t0);
        assert_eq!(acc.snapshot(t0 + Duration::from_millis(3100)).link, LinkState::InVehicle);
    }

    #[test]
    fn leave_seat_event_clears_vehicle() {
        let t0 = Instant::now();
        let mut acc = Accumulator::new(None);
        ingest(&mut acc, VEHICLE_FRAME, t0);
        ingest(&mut acc, LEAVE_SEAT_EVENT, t0);
        assert_eq!(acc.snapshot(t0).link, LinkState::Connected);
    }

    #[test]
    fn empty_vehicle_frame_clears() {
        let t0 = Instant::now();
        let mut acc = Accumulator::new(None);
        ingest(&mut acc, VEHICLE_FRAME, t0);
        ingest(&mut acc, EMPTY_VEHICLE_FRAME, t0);
        assert_eq!(acc.snapshot(t0).link, LinkState::Connected);
    }

    #[test]
    fn unknown_fields_ignored() {
        let mut b = HEALTH_FRAME.to_vec();
        crate::pb::encode::put_varint_field(&mut b, 99, 1);
        let mut nested = Vec::new();
        crate::pb::encode::put_varint_field(&mut nested, 1, 7);
        crate::pb::encode::put_bytes_field(&mut b, 98, &nested);
        let t0 = Instant::now();
        let mut acc = Accumulator::new(None);
        ingest(&mut acc, &b, t0);
        let s = acc.snapshot(t0);
        assert_eq!(s.actor.unwrap().health, Some(0.75));
        assert_eq!(s.link, LinkState::Connected);
    }

    #[test]
    fn truncated_frame_is_an_error() {
        assert!(TelemetryStreamResponse::decode(&HEALTH_FRAME[..HEALTH_FRAME.len() - 1]).is_err());
    }

    #[test]
    fn gauge_ratio_rules() {
        assert_eq!(gauge_ratio(&Gauge { current: 50.0, max: 100.0 }), Some(0.5));
        assert_eq!(gauge_ratio(&Gauge { current: 150.0, max: 100.0 }), Some(1.0));
        assert_eq!(gauge_ratio(&Gauge { current: 1.0, max: 0.0 }), None);
        assert_eq!(gauge_ratio(&Gauge { current: 1.0, max: f32::NAN }), None);
    }
}

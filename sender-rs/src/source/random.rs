//! Random demo source: a new random target every `interval`, linear interpolation published at 30 Hz.
use crate::sc::schema::LOCOMOTION_SEATED;
use crate::snapshot::*;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::time::{Duration, Instant};
use tokio::sync::watch;

/// [face0, face1, face2, face3, hydrogen, quantum], all 0..1.
type Values = [f32; 6];

fn random_values(rng: &mut StdRng) -> Values {
    let mut v = [0.0; 6];
    for x in v.iter_mut() {
        *x = rng.gen_range(0.0..=1.0);
    }
    v
}

fn lerp(a: &Values, b: &Values, t: f32) -> Values {
    let t = t.clamp(0.0, 1.0);
    let mut o = [0.0; 6];
    for i in 0..6 {
        o[i] = a[i] + (b[i] - a[i]) * t;
    }
    o
}

pub fn build_snapshot(v: &Values, health: f32) -> Snapshot {
    let faces = v[..4].to_vec();
    let mean = faces.iter().sum::<f32>() / 4.0;
    Snapshot {
        link: LinkState::InVehicle,
        source: SourceKind::Random,
        vehicle: Some(VehicleSnap {
            name: "RANDOM".into(),
            shield_level: Some(mean),
            shield_faces: faces,
            hydrogen_fuel: Some(v[4]),
            quantum_fuel: Some(v[5]),
            ..Default::default()
        }),
        actor: Some(ActorSnap { health: Some(health), oxygen: Some(1.0), locomotion: LOCOMOTION_SEATED }),
    }
}

pub async fn run(interval: Duration, tx: watch::Sender<Snapshot>) -> anyhow::Result<()> {
    let mut rng = StdRng::from_entropy();
    let mut current: Values = [0.0; 6];
    let mut target = random_values(&mut rng);
    let mut target_time = Instant::now();
    let mut health = 1.0f32;
    let mut tick = tokio::time::interval(Duration::from_micros(33_333));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let interval_s = interval.as_secs_f32().max(0.001);
    loop {
        tick.tick().await;
        let elapsed = target_time.elapsed();
        if elapsed >= interval {
            current = target;
            target = random_values(&mut rng);
            target_time = Instant::now();
        }
        let t = target_time.elapsed().as_secs_f32() / interval_s;
        health = (health + rng.gen_range(-0.01..=0.01)).clamp(0.3, 1.0);
        let snap = build_snapshot(&lerp(&current, &target, t), health);
        if tx.send(snap).is_err() {
            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_shape() {
        let s = build_snapshot(&[0.1, 0.2, 0.3, 0.4, 0.5, 0.6], 0.8);
        assert_eq!(s.link, LinkState::InVehicle);
        assert_eq!(s.source, SourceKind::Random);
        let v = s.vehicle.unwrap();
        assert_eq!(v.name, "RANDOM");
        assert_eq!(v.shield_faces.len(), 4);
        assert!(v.annunciators.is_none() && v.gear.is_none() && v.master_mode.is_none());
        assert_eq!(s.actor.unwrap().oxygen, Some(1.0));
    }

    #[test]
    fn lerp_endpoints() {
        let a = [0.0; 6];
        let b = [1.0; 6];
        assert_eq!(lerp(&a, &b, 0.0), a);
        assert_eq!(lerp(&a, &b, 1.0), b);
        assert_eq!(lerp(&a, &b, 2.0), b);
    }
}

use hud_proto_ctest::*;

const NO_SIGNAL: u8 = 0;
const NO_GAME: u8 = 1;
const ON_FOOT: u8 = 2;
const VEHICLE: u8 = 3;

#[test]
fn mode_table() {
    // no frame yet
    assert_eq!(c_compute_mode(5000, 0, false, false, 0), NO_SIGNAL);
    // timeout boundary: 2000 ms is still fine, 2001 is not
    assert_eq!(c_compute_mode(3000, 1000, true, false, 0), VEHICLE);
    assert_eq!(c_compute_mode(3001, 1000, true, false, 0), NO_SIGNAL);
    // v1 sender (no Link) -> vehicle
    assert_eq!(c_compute_mode(100, 100, true, false, 1), VEHICLE);
    // link states
    assert_eq!(c_compute_mode(100, 100, true, true, 0), NO_GAME);
    assert_eq!(c_compute_mode(100, 100, true, true, 1), ON_FOOT);
    assert_eq!(c_compute_mode(100, 100, true, true, 2), VEHICLE);
    assert_eq!(c_compute_mode(100, 100, true, true, 9), VEHICLE);
    // millis wraparound: now = 5, last = 0xFFFFFFF0 -> 21 ms apart
    assert_eq!(c_compute_mode(5, 0xFFFF_FFF0, true, true, 2), VEHICLE);
}

fn ann_with(pairs: &[(usize, u8)]) -> [u8; 10] {
    let mut a = [1u8; 10];
    for &(i, v) in pairs {
        a[i] = v;
    }
    a
}

#[test]
fn alert_cases() {
    let sh = [255u8; 4];
    assert_eq!(c_compute_alert(&ann_with(&[]), true, &sh, 0x0F), (0, false));
    assert_eq!(c_compute_alert(&ann_with(&[(3, 6)]), true, &sh, 0x0F), (2, false)); // shld major on
    assert_eq!(c_compute_alert(&ann_with(&[(8, 5)]), true, &sh, 0x0F), (2, true)); // hit major flashing
    assert_eq!(c_compute_alert(&ann_with(&[(4, 3)]), true, &sh, 0x0F), (1, true)); // cool minor flashing
    assert_eq!(c_compute_alert(&ann_with(&[(4, 4)]), true, &sh, 0x0F), (1, false));
    // critical steady + warning flashing -> critical, no blink
    assert_eq!(c_compute_alert(&ann_with(&[(3, 6), (4, 3)]), true, &sh, 0x0F), (2, false));
    // unknown annunciators -> legacy shield heuristic
    let unk = [15u8; 10];
    let low = [255u8, 84, 255, 255];
    assert_eq!(c_compute_alert(&unk, true, &low, 0x0F), (2, true));
    assert_eq!(c_compute_alert(&unk, true, &low, 0x0D), (0, false));
    // status not valid -> legacy even if values look known
    assert_eq!(c_compute_alert(&ann_with(&[(3, 6)]), false, &sh, 0x0F), (0, false));
}

#[test]
fn smoother_behaviour() {
    let mut s = CSmoother::new(150.0);
    s.set_target(0);
    s.set_target(255);
    assert!(s.tick(150));
    let v = s.value() as i32;
    assert!((v - 161).abs() <= 2, "value after one tau: {v}");
    s.tick(2000);
    assert_eq!(s.value(), 255);
    assert!(!s.tick(33), "stable value must not report a change");
    assert!(!s.tick(33));

    // first setTarget snaps
    let mut t = CSmoother::new(150.0);
    t.set_target(200);
    assert_eq!(t.value(), 200);
    t.snap(10);
    t.set_target(10);
    assert_eq!(t.value(), 10);
}

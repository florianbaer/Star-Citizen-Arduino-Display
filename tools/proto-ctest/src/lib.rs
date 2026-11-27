//! Safe Rust wrappers around the C++ firmware headers (see shim/shim.cpp).
use std::os::raw::c_char;

extern "C" {
    fn hudc_decode_stream(data: *const u8, len: usize, types_out: *mut u8, payloads_out: *mut u8, lens_out: *mut i32, max_frames: i32) -> i32;
    fn hudc_frame_encode(t: u8, p: *const u8, len: usize, out: *mut u8, cap: usize) -> usize;
    fn hudc_sizeof(t: u8) -> usize;
    fn hudc_decode_telemetry(p: *const u8, len: i32, out7: *mut u8) -> i32;
    fn hudc_decode_status(p: *const u8, len: i32, ann10: *mut u8, flags: *mut u8, fv: *mut u8, gear: *mut u8, mm: *mut u8) -> i32;
    fn hudc_decode_vitals(p: *const u8, len: i32, out3: *mut u8) -> i32;
    fn hudc_decode_link(p: *const u8, len: i32, out2: *mut u8) -> i32;
    fn hudc_decode_hello(p: *const u8, len: i32, out2: *mut u8) -> i32;
    fn hudc_decode_action_ack(p: *const u8, len: i32, out4: *mut u8) -> i32;
    fn hudc_decode_vehicle_name(p: *const u8, len: i32, out: *mut c_char, cap: i32) -> i32;
    fn hudc_compute_mode(now: u32, last_frame: u32, any_frame: i32, link_seen: i32, link_state: u8) -> u8;
    fn hudc_compute_alert(ann10: *const u8, status_valid: i32, shields4: *const u8, mask: u8, sev: *mut u8, blink: *mut u8);
    fn hudc_smoother_new(tau_ms: f32) -> *mut std::ffi::c_void;
    fn hudc_smoother_free(p: *mut std::ffi::c_void);
    fn hudc_smoother_set(p: *mut std::ffi::c_void, t: u8);
    fn hudc_smoother_snap(p: *mut std::ffi::c_void, v: u8);
    fn hudc_smoother_tick(p: *mut std::ffi::c_void, dt_ms: u32) -> i32;
    fn hudc_smoother_value(p: *mut std::ffi::c_void) -> u8;
}

/// HudMode as u8 (0 NO_SIGNAL, 1 NO_GAME, 2 ON_FOOT, 3 VEHICLE).
pub fn c_compute_mode(now: u32, last_frame: u32, any_frame: bool, link_seen: bool, link_state: u8) -> u8 {
    unsafe { hudc_compute_mode(now, last_frame, any_frame as i32, link_seen as i32, link_state) }
}

/// Returns (severity 0 none / 1 warning / 2 critical, blink).
pub fn c_compute_alert(ann: &[u8; 10], status_valid: bool, shields: &[u8; 4], mask: u8) -> (u8, bool) {
    let (mut sev, mut blink) = (0u8, 0u8);
    unsafe { hudc_compute_alert(ann.as_ptr(), status_valid as i32, shields.as_ptr(), mask, &mut sev, &mut blink) };
    (sev, blink != 0)
}

/// Wrapper around the C++ `Smoother`.
pub struct CSmoother(*mut std::ffi::c_void);

impl CSmoother {
    pub fn new(tau_ms: f32) -> Self {
        Self(unsafe { hudc_smoother_new(tau_ms) })
    }
    pub fn set_target(&mut self, t: u8) {
        unsafe { hudc_smoother_set(self.0, t) }
    }
    pub fn snap(&mut self, v: u8) {
        unsafe { hudc_smoother_snap(self.0, v) }
    }
    pub fn tick(&mut self, dt_ms: u32) -> bool {
        unsafe { hudc_smoother_tick(self.0, dt_ms) != 0 }
    }
    pub fn value(&self) -> u8 {
        unsafe { hudc_smoother_value(self.0) }
    }
}

impl Drop for CSmoother {
    fn drop(&mut self) {
        unsafe { hudc_smoother_free(self.0) }
    }
}

/// Feed `data` through a fresh C `FrameDecoder`; returns (type, payload) per decoded frame.
pub fn c_decode_stream(data: &[u8]) -> Vec<(u8, Vec<u8>)> {
    const MAX: usize = 64;
    let mut types = [0u8; MAX];
    let mut payloads = vec![0u8; MAX * 64];
    let mut lens = [0i32; MAX];
    let n = unsafe {
        hudc_decode_stream(data.as_ptr(), data.len(), types.as_mut_ptr(), payloads.as_mut_ptr(), lens.as_mut_ptr(), MAX as i32)
    };
    (0..n as usize)
        .map(|i| (types[i], payloads[i * 64..i * 64 + lens[i] as usize].to_vec()))
        .collect()
}

pub fn c_frame_encode(t: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = [0u8; 128];
    let n = unsafe { hudc_frame_encode(t, payload.as_ptr(), payload.len(), out.as_mut_ptr(), out.len()) };
    out[..n].to_vec()
}

pub fn c_sizeof(t: u8) -> usize {
    unsafe { hudc_sizeof(t) }
}

pub fn c_decode_telemetry(p: &[u8]) -> Option<[u8; 7]> {
    let mut o = [0u8; 7];
    let ok = unsafe { hudc_decode_telemetry(p.as_ptr(), p.len() as i32, o.as_mut_ptr()) };
    (ok != 0).then_some(o)
}

/// Returns (ann[10], flags, flags_valid, gear, master_mode)
pub fn c_decode_status(p: &[u8]) -> Option<([u8; 10], u8, u8, u8, u8)> {
    let mut ann = [0u8; 10];
    let (mut f, mut fv, mut g, mut m) = (0u8, 0u8, 0u8, 0u8);
    let ok = unsafe { hudc_decode_status(p.as_ptr(), p.len() as i32, ann.as_mut_ptr(), &mut f, &mut fv, &mut g, &mut m) };
    (ok != 0).then_some((ann, f, fv, g, m))
}

pub fn c_decode_vitals(p: &[u8]) -> Option<[u8; 3]> {
    let mut o = [0u8; 3];
    let ok = unsafe { hudc_decode_vitals(p.as_ptr(), p.len() as i32, o.as_mut_ptr()) };
    (ok != 0).then_some(o)
}

pub fn c_decode_link(p: &[u8]) -> Option<[u8; 2]> {
    let mut o = [0u8; 2];
    let ok = unsafe { hudc_decode_link(p.as_ptr(), p.len() as i32, o.as_mut_ptr()) };
    (ok != 0).then_some(o)
}

pub fn c_decode_hello(p: &[u8]) -> Option<[u8; 2]> {
    let mut o = [0u8; 2];
    let ok = unsafe { hudc_decode_hello(p.as_ptr(), p.len() as i32, o.as_mut_ptr()) };
    (ok != 0).then_some(o)
}

pub fn c_decode_action_ack(p: &[u8]) -> Option<[u8; 4]> {
    let mut o = [0u8; 4];
    let ok = unsafe { hudc_decode_action_ack(p.as_ptr(), p.len() as i32, o.as_mut_ptr()) };
    (ok != 0).then_some(o)
}

pub fn c_decode_vehicle_name(p: &[u8]) -> String {
    let mut out = [0 as c_char; 64];
    let n = unsafe { hudc_decode_vehicle_name(p.as_ptr(), p.len() as i32, out.as_mut_ptr(), out.len() as i32) };
    out[..n as usize].iter().map(|&c| c as u8 as char).collect()
}

//! Send policy: on-change, keepalive, periodic Hello.
use hud_proto::messages::{HelloMsg, Message, MsgType, PROTO_VERSION, ROLE_SENDER};
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct SendPolicy {
    last: HashMap<u8, Vec<u8>>,
    last_full: Option<Instant>,
    last_hello: Option<Instant>,
    keepalive: Duration,
    hello_every: Duration,
}

impl SendPolicy {
    pub fn new(keepalive: Duration) -> Self {
        Self { last: HashMap::new(), last_full: None, last_hello: None, keepalive, hello_every: Duration::from_secs(5) }
    }

    /// Frames to send now: Hello first if `hello_every` elapsed (or never sent); then everything whose
    /// payload differs from the last sent, or everything if the keepalive elapsed since the last full send.
    /// Hello is never sent in reply to a Hello.
    pub fn plan(&mut self, now: Instant, frames: &[(MsgType, Vec<u8>)]) -> Vec<(MsgType, Vec<u8>)> {
        let mut out = Vec::new();
        if self.last_hello.map_or(true, |t| now.duration_since(t) >= self.hello_every) {
            let mut b = [0u8; 2];
            HelloMsg { version: PROTO_VERSION, role: ROLE_SENDER }.encode(&mut b);
            out.push((MsgType::Hello, b.to_vec()));
            self.last_hello = Some(now);
        }
        let full = self.last_full.map_or(true, |t| now.duration_since(t) >= self.keepalive);
        for (ty, payload) in frames {
            let key = *ty as u8;
            if full || self.last.get(&key) != Some(payload) {
                self.last.insert(key, payload.clone());
                out.push((*ty, payload.clone()));
            }
        }
        if full {
            self.last_full = Some(now);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frames(tel: u8) -> Vec<(MsgType, Vec<u8>)> {
        vec![
            (MsgType::Link, vec![2, 0]),
            (MsgType::Telemetry, vec![tel, 0, 0, 0, 0, 0, 0x3F]),
            (MsgType::Status, vec![0; 9]),
            (MsgType::Vitals, vec![0; 3]),
            (MsgType::VehicleName, vec![]),
        ]
    }

    fn types(v: &[(MsgType, Vec<u8>)]) -> Vec<MsgType> {
        v.iter().map(|x| x.0).collect()
    }

    #[test]
    fn policy_sequence() {
        let t0 = Instant::now();
        let ms = |n| t0 + Duration::from_millis(n);
        let mut p = SendPolicy::new(Duration::from_millis(500));

        let first = p.plan(t0, &frames(1));
        assert_eq!(first.len(), 6);
        assert_eq!(first[0], (MsgType::Hello, vec![PROTO_VERSION, ROLE_SENDER]));

        assert!(p.plan(ms(33), &frames(1)).is_empty());

        let changed = p.plan(ms(66), &frames(2));
        assert_eq!(types(&changed), vec![MsgType::Telemetry]);

        let keepalive = p.plan(ms(500), &frames(2));
        assert_eq!(keepalive.len(), 5);
        assert!(!types(&keepalive).contains(&MsgType::Hello));

        let hello = p.plan(ms(5000), &frames(2));
        assert_eq!(hello[0].0, MsgType::Hello);
        assert_eq!(hello.len(), 6);
    }
}

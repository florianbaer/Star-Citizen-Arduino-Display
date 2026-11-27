//! Game action allowlist and blocklist.
//!
//! The five action names below are ASSUMPTIONS (taken from docs/PLAN.md; they look like key-binding
//! action names). Nobody has checked that the game TriggerGameAction accepts them. Compare them with
//! `actions.txt` from `hud-sender probe`. Do not add, rename or "correct" entries. A wrong name is
//! harmless: the game is expected to answer UNKNOWN_ACTION.
//!
//! Self-destruct / eject cannot be triggered, by three layers:
//! 1. allowlist: `client::trigger_action` (the only sender of the RPC) rejects every name for which
//!    `is_allowed` is false, before any I/O. The HUD can only send a numeric id, mapped by `lookup`.
//! 2. blocklist: `is_allowed` also requires `!is_blocked`; a unit test asserts no table entry is blocked.
//! 3. opt-in: the serial path executes nothing without `--allow-actions`; the CLI nothing without `--yes`.

use hud_proto::messages::*;
use std::time::{Duration, Instant};

pub struct ActionDef {
    pub id: u8,
    pub name: &'static str,
    pub hold_ms: Option<u32>,
}

pub const ACTIONS: &[ActionDef] = &[
    ActionDef { id: 1, name: "v_power_toggle_shields", hold_ms: None },
    ActionDef { id: 2, name: "v_toggle_landing_system", hold_ms: None },
    ActionDef { id: 3, name: "v_toggle_qdrive_engagement", hold_ms: None },
    ActionDef { id: 4, name: "v_ifcs_vector_decoupling_toggle", hold_ms: None },
    ActionDef { id: 5, name: "v_invoke_ping", hold_ms: None },
];

pub const BLOCKED_SUBSTRINGS: &[&str] = &["destruct", "eject", "suicide"];

/// Case-insensitive substring match against `BLOCKED_SUBSTRINGS`.
pub fn is_blocked(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    BLOCKED_SUBSTRINGS.iter().any(|b| lower.contains(b))
}

/// Exact match with an `ACTIONS` entry AND not blocked.
pub fn is_allowed(name: &str) -> bool {
    !is_blocked(name) && ACTIONS.iter().any(|a| a.name == name)
}

pub fn lookup(id: u8) -> Option<&'static ActionDef> {
    ACTIONS.iter().find(|a| a.id == id)
}

pub enum GateDecision {
    Execute(&'static ActionDef),
    /// Reject with this ACK_* code.
    Reject(u8),
    /// Same seq as the previous action within 2 s: ignored, no ack.
    Duplicate,
}

pub struct ActionGate {
    allow: bool,
    min_interval: Duration,
    last_exec: Option<Instant>,
    last_seq: Option<(u8, Instant)>,
}

impl ActionGate {
    pub fn new(allow: bool) -> Self {
        Self { allow, min_interval: Duration::from_millis(250), last_exec: None, last_seq: None }
    }

    pub fn decide(&mut self, m: &ActionMsg, now: Instant) -> GateDecision {
        if let Some((seq, t)) = self.last_seq {
            if seq == m.seq && now.duration_since(t) < Duration::from_secs(2) {
                return GateDecision::Duplicate;
            }
        }
        let Some(def) = lookup(m.action_id) else {
            return GateDecision::Reject(ACK_UNKNOWN_ID);
        };
        if is_blocked(def.name) {
            return GateDecision::Reject(ACK_BLOCKED);
        }
        if !self.allow {
            return GateDecision::Reject(ACK_DISABLED);
        }
        if let Some(t) = self.last_exec {
            if now.duration_since(t) < self.min_interval {
                return GateDecision::Reject(ACK_RATE_LIMITED);
            }
        }
        self.last_exec = Some(now);
        self.last_seq = Some((m.seq, now));
        GateDecision::Execute(def)
    }
}

/// Runs one table action against the game. Returns (ACK_* result, game_status).
pub async fn execute(endpoint: &str, def: &ActionDef) -> (u8, u8) {
    let ch = match crate::sc::client::connect(endpoint, Duration::from_secs(1)).await {
        Ok(c) => c,
        Err(_) => return (ACK_NOT_CONNECTED, 0),
    };
    match crate::sc::client::trigger_action(ch, def.name, def.hold_ms).await {
        Err(_) => (ACK_RPC_ERROR, 0),
        Ok(r) if r.status == crate::sc::schema::TRIGGER_STATUS_OK => (ACK_OK, 0),
        Ok(r) => (ACK_GAME_REJECTED, r.status.clamp(0, 255) as u8),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_table_entry_is_blocked() {
        for a in ACTIONS {
            assert!(!is_blocked(a.name), "{} is blocked", a.name);
            assert!(is_allowed(a.name));
        }
    }

    #[test]
    fn blocklist_cases() {
        assert!(is_blocked("V_SELF_DESTRUCT"));
        assert!(is_blocked("v_eject"));
        assert!(is_blocked("v_selfdestruct"));
        assert!(!is_allowed("v_self_destruct"));
        assert!(!is_allowed("anything_else"));
        assert!(is_allowed("v_invoke_ping"));
    }

    #[test]
    fn ids_unique_and_lookup() {
        let mut ids: Vec<u8> = ACTIONS.iter().map(|a| a.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), ACTIONS.len());
        assert_eq!(lookup(5).unwrap().name, "v_invoke_ping");
        assert!(lookup(0).is_none());
        assert!(lookup(99).is_none());
    }

    #[test]
    fn gate_branches() {
        let t0 = Instant::now();
        let ms = |n| t0 + Duration::from_millis(n);
        let m = |id, seq| ActionMsg { action_id: id, seq };

        // disabled
        let mut g = ActionGate::new(false);
        assert!(matches!(g.decide(&m(5, 1), t0), GateDecision::Reject(ACK_DISABLED)));
        // unknown id
        let mut g = ActionGate::new(true);
        assert!(matches!(g.decide(&m(99, 1), t0), GateDecision::Reject(ACK_UNKNOWN_ID)));
        // execute, duplicate, rate limit, execute again
        assert!(matches!(g.decide(&m(5, 2), t0), GateDecision::Execute(d) if d.name == "v_invoke_ping"));
        assert!(matches!(g.decide(&m(5, 2), ms(100)), GateDecision::Duplicate));
        assert!(matches!(g.decide(&m(5, 3), ms(100)), GateDecision::Reject(ACK_RATE_LIMITED)));
        assert!(matches!(g.decide(&m(5, 3), ms(300)), GateDecision::Execute(_)));
        // same seq after 2 s is no longer a duplicate
        assert!(matches!(g.decide(&m(5, 3), ms(2500)), GateDecision::Execute(_)));
    }
}

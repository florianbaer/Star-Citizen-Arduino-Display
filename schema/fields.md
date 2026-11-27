# Telemetry field table

Source: `schema/sc_telemetry_community.proto` (community extraction, unofficial) and `sender-rs/src/sc/schema.rs`, which is the only place in the code with field numbers.

Provenance: **CONFIRMED** = reported by two sources (the extracted schema and a reference example); **COMMUNITY** = from the extracted schema only; **ASSUMPTION** = our guess about meaning, units or names. None of the three means "this project has seen it work". The "verified live" column is empty in every row, health included.

| path | name | type | provenance | used by HUD | verified live (date, build) | observed range/units |
|------|------|------|------------|-------------|-----------------------------|----------------------|
| (rpc) | `/sc.game.telemetry.v1.GameTelemetryService/TelemetryStream` | server stream | CONFIRMED | yes | | |
| (rpc) | `.../TriggerGameAction` | unary | COMMUNITY | actions (not in HUD UI yet) | | |
| (rpc) | `.../ListTriggerableActions` | unary | COMMUNITY | probe, `actions list` | | |
| 3 | vehicle | message | COMMUNITY | yes | | |
| 3.2 | vehicle.state | message | COMMUNITY | yes | | |
| 3.2.1 | vehicle.state.name | string | COMMUNITY | VehicleName | | |
| 3.2.2 | vehicle.state.class_name | string | COMMUNITY | no | | |
| 3.2.3 | vehicle.state.manufacturer | string | COMMUNITY | no | | |
| 3.2.4 | vehicle.state.type | enum | COMMUNITY | no | | |
| 3.2.5 | vehicle.state.lights_enabled | bool | COMMUNITY | Status flag LIGHTS | | |
| 3.2.7.1-10 | vehicle.state.annunciators.{pwr,wpn,thr,shld,cool,qdrv,radr,prox,hit,misl} | enum AnnunciatorState | COMMUNITY | Status annunciators, alert | | |
| 3.2.8.1.1 | vehicle.state.flight.state.is_landed | bool | COMMUNITY | Status flag LANDED | | |
| 3.2.8.1.2 | vehicle.state.flight.state.gear | optional enum | COMMUNITY | Status gear, GEAR indicator | | |
| 3.2.8.2.1 | vehicle.state.flight.settings.ifcs_coupled | Availability {enabled, active} | COMMUNITY (meaning of `active`: ASSUMPTION) | Status flag DECOUPLED | | |
| 3.2.8.3.1 | vehicle.state.flight.instruments.speed | float | COMMUNITY | no | | |
| 3.2.8.3.2 | vehicle.state.flight.instruments.hydrogen_fuel | float | COMMUNITY (units: ASSUMPTION, default 0..1) | Telemetry hydrogen | | |
| 3.2.8.3.3 | vehicle.state.flight.instruments.quantum_fuel | float | COMMUNITY (units: ASSUMPTION, default 0..1) | Telemetry quantum | | |
| 3.2.8.3.4 | vehicle.state.flight.instruments.g_meter | Gauge | COMMUNITY | no | | |
| 3.2.10.1 | vehicle.state.shields.level | float | COMMUNITY (units: ASSUMPTION, default 0..1) | fallback when no faces | | |
| 3.2.10.2 | vehicle.state.shields.faces | repeated float | COMMUNITY (units and face order: ASSUMPTION) | Telemetry shields | | |
| 3.2.11 | vehicle.state.health | float | COMMUNITY | no | | |
| 3.2.12.1 | vehicle.state.master_mode.mode | enum | COMMUNITY | Status master_mode | | |
| 3.2.12.2 | vehicle.state.master_mode.spooled | bool | COMMUNITY | Status flag QT_SPOOLED | | |
| 4 | weapons | message | COMMUNITY | no | | |
| 5.1.1 | events.events[].type | enum GameEventType | COMMUNITY | clears the vehicle on LEAVE_SEAT / DEAD / RESPAWN (ASSUMPTION) | | |
| 6 | actor | message | CONFIRMED (tag 6) | yes | | |
| 6.1.1.1 | actor.vitals.health.current | float | CONFIRMED | Vitals health | | |
| 6.1.1.2 | actor.vitals.health.max | float | CONFIRMED | Vitals health | | |
| 6.1.2.1 | actor.vitals.oxygen.current | float | COMMUNITY | Vitals oxygen | | |
| 6.1.2.2 | actor.vitals.oxygen.max | float | COMMUNITY | Vitals oxygen | | |
| 6.2.1 | actor.state.locomotion | enum | COMMUNITY | in-vehicle decision (ASSUMPTION) | | |
| 7 | force_feedback | message | COMMUNITY | no | | |
| 8 | turret | message | COMMUNITY | no | | |

## Game actions (names are ASSUMPTIONS)

The five names come from `docs/PLAN.md`. They look like key-binding action names; nobody has checked that the game accepts them. Compare with `actions.txt` from `hud-sender probe`. A wrong name is harmless (the game is expected to answer UNKNOWN_ACTION). Self-destruct and eject can never be triggered: only these table entries are ever sent, and names containing `destruct`, `eject` or `suicide` are blocked.

| id | name | provenance | verified live |
|----|------|------------|---------------|
| 1 | `v_power_toggle_shields` | ASSUMPTION | |
| 2 | `v_toggle_landing_system` | ASSUMPTION | |
| 3 | `v_toggle_qdrive_engagement` | ASSUMPTION | |
| 4 | `v_ifcs_vector_decoupling_toggle` | ASSUMPTION | |
| 5 | `v_invoke_ping` | ASSUMPTION | |

## Other assumptions

| item | assumption | where |
|------|------------|-------|
| shield and fuel units | 0..1 (`--shield-scale` / `--fuel-scale percent` for 0..100) | `schema.rs` `DEFAULT_*_SCALE` |
| face order | faces[] = front, back, left, right (4 faces); front, back, left, right, top, bottom (6); front, back (2) | `schema.rs` `FACE_ORDER_*` |
| `ifcs_coupled.active` | true means coupled, so decoupled = !active | `schema.rs` `DECOUPLED_WHEN_COUPLED_ACTIVE_IS` |
| partial messages | a message may carry only some sub-frames; merged per sub-frame, stale after `--frame-stale-ms` (default 3000) | `sc/decode.rs` |
| continuous stream | unknown; idle reconnect is off by default (`--idle-timeout-secs 0`) | `sc/source.rs` |

## Live verification checklist (needs the game)

1. `hud-sender probe` and compare `schema.txt` with `schema/sc_telemetry_community.proto` (by eye, or with `protoc` later). Check `actions.txt` against the table above.
2. `hud-sender dump --out run.sctd`, and type a marker line (press Enter) at each step while you: sit in a ship, power on, drop each shield face, burn hydrogen fuel, spool quantum drive, raise and lower the gear, toggle decoupled mode, take damage, leave the seat and walk.
3. `hud-sender inspect run.sctd`, `inspect run.sctd --frame N`, `diff`, and `decode` to settle:
   - fuel and shield units (set the `--*-scale` defaults in `schema.rs`),
   - shield face order (`FACE_ORDER_*`),
   - the meaning of `ifcs_coupled.active`,
   - whether messages carry all sub-frames or only some (stale timeout),
   - whether the stream is continuous (idle timeout).
4. Fill the "verified live" column above with the date and game build for every row you confirmed.

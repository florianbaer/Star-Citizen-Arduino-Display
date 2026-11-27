# Wire protocol v2

Shared by `proto/` (Rust) and `lib/hud_proto/` (C). `PROTO_VERSION = 2`.

## Wire rules

- Frame: `0x00 | COBS( msg_type:u8 | payload | crc8 ) | 0x00`.
- CRC-8: polynomial 0x31, init 0x00, MSB-first, no reflection, computed over `msg_type | payload`. (The old name "CRC8/MAXIM" was inaccurate.)
- `MAX_PAYLOAD = 64`. Multi-byte integers are little-endian (none are used in v2).
- Forward compatibility: payloads only ever grow at the end. Decoders accept payloads longer than they know (extra bytes ignored) and give documented defaults to missing trailing optional fields. Decoders ignore unknown `msg_type` values without error.
- The stream decoder discards bytes until the first `0x00`, tolerates text (boot logs) between frames, and drops frames that overflow its 72-byte buffer.
- Neither side replies to a Hello. Each side announces itself on its own 5 s timer.

## Messages

| ID | Name | Dir | Size (min/max) | Layout (byte offsets) |
|----|------|-----|----------------|-----------------------|
| 0x01 | Telemetry | PC->HUD | 6 / 7 | 0 shield_front, 1 shield_back, 2 shield_left, 3 shield_right, 4 hydrogen_fuel, 5 quantum_fuel (all 0-255), 6 `valid` bitmask. If byte 6 is absent, `valid = 0x3F`. |
| 0x02 | Status | PC->HUD | 9 / 9 | 0-4 annunciator nibbles, 5 `flags`, 6 `flags_valid`, 7 `gear`, 8 `master_mode` |
| 0x03 | Vitals | PC->HUD | 3 / 3 | 0 health 0-255, 1 oxygen 0-255, 2 `valid` |
| 0x04 | Link | PC->HUD | 1 / 2 | 0 `state`, 1 `source` (absent = 0) |
| 0x05 | Hello | both | 1 / 2 | 0 `version`, 1 `role` (absent = 0) |
| 0x06 | VehicleName | PC->HUD | 0 / 32 | printable ASCII bytes, length = payload length |
| 0x10 | Action | HUD->PC | 2 / 2 | 0 `action_id`, 1 `seq` |
| 0x11 | ActionAck | PC->HUD | 3 / 4 | 0 `action_id`, 1 `seq`, 2 `result`, 3 `game_status` (absent = 0) |

Constants (same names in `proto/src/messages.rs` and `lib/hud_proto/messages.h`):

- Telemetry `valid` bits: `TV_SHIELD_FRONT=0x01, TV_SHIELD_BACK=0x02, TV_SHIELD_LEFT=0x04, TV_SHIELD_RIGHT=0x08, TV_HYDROGEN=0x10, TV_QUANTUM=0x20, TV_ALL=0x3F`.
- Status annunciator index order (nibble i): `ANN_PWR=0, ANN_WPN=1, ANN_THR=2, ANN_SHLD=3, ANN_COOL=4, ANN_QDRV=5, ANN_RADR=6, ANN_PROX=7, ANN_HIT=8, ANN_MISL=9, ANN_COUNT=10`. Byte `k` (0..4) = `ann[2k] & 0x0F | (ann[2k+1] & 0x0F) << 4`.
  Nibble value = the game's `AnnunciatorState` enum (0 UNSPECIFIED, 1 NOMINAL, 2 OFF, 3 MINOR_FLASHING, 4 MINOR_ON, 5 MAJOR_FLASHING, 6 MAJOR_ON). Raw values >= 15 are sent as 14. `ANN_UNKNOWN = 15` means "no annunciator data".
- Status `flags` / `flags_valid` bits: `SF_LANDED=0x01, SF_QT_SPOOLED=0x02, SF_DECOUPLED=0x04, SF_IN_VEHICLE=0x08, SF_LIGHTS=0x10`.
- Status `gear`: the game's `VehicleComponentDeploymentState` (0 UNSPECIFIED, 1 RETRACTED, 2 DEPLOYING, 3 DEPLOYED, 4 RETRACTING) or `GEAR_UNKNOWN = 0xFF`.
- Status `master_mode`: the game's `VehicleMasterMode` (0 UNSPECIFIED, 1 NAVIGATION, 2 SCM, 3 STEALTH) or `MASTER_MODE_UNKNOWN = 0xFF`.
- Vitals `valid`: `VV_HEALTH=0x01, VV_OXYGEN=0x02`.
- Link `state`: `LINK_NO_GAME=0, LINK_CONNECTED=1` (game reachable, not in a vehicle), `LINK_IN_VEHICLE=2`. Link `source`: `SOURCE_SC=0, SOURCE_RANDOM=1`.
- Hello `role`: `ROLE_SENDER=0, ROLE_HUD=1`.
- ActionAck `result`: `ACK_OK=0, ACK_UNKNOWN_ID=1, ACK_DISABLED=2, ACK_BLOCKED=3, ACK_RATE_LIMITED=4, ACK_GAME_REJECTED=5, ACK_RPC_ERROR=6, ACK_NOT_CONNECTED=7`. `game_status` is the game's `TriggerGameActionStatus` raw value when `result == ACK_GAME_REJECTED`, else 0.

## Golden vectors

Computed independently in Python; checked by `proto/tests/golden.rs` (and, for the C side, `tools/proto-ctest`).

| Name | Type | Payload | Full frame on the wire |
|------|------|---------|------------------------|
| TELEMETRY_V2 | 0x01 | `FF 80 40 20 C8 64 3F` | `00 0A 01 FF 80 40 20 C8 64 3F 3C 00` |
| TELEMETRY_V1 | 0x01 | `FF 80 40 20 C8 64` | `00 09 01 FF 80 40 20 C8 64 4B 00` |
| STATUS | 0x02 | `11 61 42 11 05 0A 0F 03 02` | `00 0C 02 11 61 42 11 05 0A 0F 03 02 5D 00` |
| VITALS | 0x03 | `C8 00 01` | `00 03 03 C8 03 01 D8 00` |
| LINK | 0x04 | `02 00` | `00 03 04 02 02 F0 00` |
| HELLO_SENDER | 0x05 | `02 00` | `00 03 05 02 02 B6 00` |
| HELLO_HUD | 0x05 | `02 01` | `00 05 05 02 01 87 00` |
| VEHICLE_NAME | 0x06 | `"AEGS_Gladius"` = `41 45 47 53 5F 47 6C 61 64 69 75 73` | `00 0F 06 41 45 47 53 5F 47 6C 61 64 69 75 73 58 00` |
| ACTION | 0x10 | `01 07` | `00 05 10 01 07 C7 00` |
| ACTION_ACK | 0x11 | `01 07 05 07` | `00 07 11 01 07 05 07 37 00` |

STATUS decoded: annunciators pwr=1, wpn=1, thr=1, shld=6, cool=2, qdrv=4, radr=1, prox=1, hit=5, misl=0; flags 0x0A (QT_SPOOLED, IN_VEHICLE); flags_valid 0x0F; gear 3 (DEPLOYED); master_mode 2 (SCM). ACTION_ACK: result 5 (GAME_REJECTED), game_status 7 (FILTERED).

## TODO (deferred): touch UI for game actions

The sender side is implemented (`Action` -> allowlist gate -> `ActionAck`), but nothing on the HUD sends `Action` frames yet. Still to do on the firmware: tap zones and a long-press UI, press feedback, and `ActionAck` display. The five action names in `sender-rs/src/actions.rs` are unverified placeholders; compare them with `actions.txt` from `hud-sender probe`.

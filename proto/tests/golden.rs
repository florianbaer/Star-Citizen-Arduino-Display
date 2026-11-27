use hud_proto::frame::*;
use hud_proto::messages::*;

const TELEMETRY_V2: &[u8] = &[0x00, 0x0A, 0x01, 0xFF, 0x80, 0x40, 0x20, 0xC8, 0x64, 0x3F, 0x3C, 0x00];
const TELEMETRY_V1: &[u8] = &[0x00, 0x09, 0x01, 0xFF, 0x80, 0x40, 0x20, 0xC8, 0x64, 0x4B, 0x00];
const STATUS: &[u8] = &[0x00, 0x0C, 0x02, 0x11, 0x61, 0x42, 0x11, 0x05, 0x0A, 0x0F, 0x03, 0x02, 0x5D, 0x00];
const VITALS: &[u8] = &[0x00, 0x03, 0x03, 0xC8, 0x03, 0x01, 0xD8, 0x00];
const LINK: &[u8] = &[0x00, 0x03, 0x04, 0x02, 0x02, 0xF0, 0x00];
const HELLO_SENDER: &[u8] = &[0x00, 0x03, 0x05, 0x02, 0x02, 0xB6, 0x00];
const HELLO_HUD: &[u8] = &[0x00, 0x05, 0x05, 0x02, 0x01, 0x87, 0x00];
const VEHICLE_NAME: &[u8] = &[
    0x00, 0x0F, 0x06, 0x41, 0x45, 0x47, 0x53, 0x5F, 0x47, 0x6C, 0x61, 0x64, 0x69, 0x75, 0x73, 0x58, 0x00,
];
const ACTION: &[u8] = &[0x00, 0x05, 0x10, 0x01, 0x07, 0xC7, 0x00];
const ACTION_ACK: &[u8] = &[0x00, 0x07, 0x11, 0x01, 0x07, 0x05, 0x07, 0x37, 0x00];

const ALL: [&[u8]; 10] = [
    TELEMETRY_V2, TELEMETRY_V1, STATUS, VITALS, LINK, HELLO_SENDER, HELLO_HUD, VEHICLE_NAME, ACTION, ACTION_ACK,
];

fn wire<M: Message>(m: &M) -> Vec<u8> {
    let mut out = [0u8; MAX_FRAME];
    let n = frame_message(m, &mut out);
    out[..n].to_vec()
}

fn payload_of(frame_bytes: &[u8]) -> (u8, Vec<u8>) {
    let mut buf = [0u8; MAX_PAYLOAD + 2];
    let d = deframe(&frame_bytes[1..frame_bytes.len() - 1], &mut buf).unwrap();
    (d.msg_type, d.payload.to_vec())
}

fn check<M: Message + PartialEq + core::fmt::Debug>(m: M, golden: &[u8]) {
    assert_eq!(wire(&m), golden);
    let (t, p) = payload_of(golden);
    assert_eq!(t, M::TYPE as u8);
    assert_eq!(M::decode(&p).unwrap(), m);
}

fn status() -> StatusMsg {
    StatusMsg {
        annunciators: [1, 1, 1, 6, 2, 4, 1, 1, 5, 0],
        flags: SF_QT_SPOOLED | SF_IN_VEHICLE,
        flags_valid: 0x0F,
        gear: 3,
        master_mode: 2,
    }
}

#[test]
fn golden_frames() {
    check(
        TelemetryMsg { shield_front: 0xFF, shield_back: 0x80, shield_left: 0x40, shield_right: 0x20, hydrogen_fuel: 0xC8, quantum_fuel: 0x64, valid: 0x3F },
        TELEMETRY_V2,
    );
    check(status(), STATUS);
    check(VitalsMsg { health: 0xC8, oxygen: 0, valid: 1 }, VITALS);
    check(LinkMsg { state: 2, source: 0 }, LINK);
    check(HelloMsg { version: 2, role: ROLE_SENDER }, HELLO_SENDER);
    check(HelloMsg { version: 2, role: ROLE_HUD }, HELLO_HUD);
    check(VehicleNameMsg::from_str_lossy("AEGS_Gladius"), VEHICLE_NAME);
    check(ActionMsg { action_id: 1, seq: 7 }, ACTION);
    check(ActionAckMsg { action_id: 1, seq: 7, result: ACK_GAME_REJECTED, game_status: 7 }, ACTION_ACK);
}

#[test]
fn telemetry_v1_defaults_valid() {
    let (t, p) = payload_of(TELEMETRY_V1);
    assert_eq!(t, 1);
    assert_eq!(p.len(), 6);
    assert_eq!(TelemetryMsg::decode(&p).unwrap().valid, TV_ALL);
}

fn extra<M: Message + PartialEq + core::fmt::Debug>(golden: &[u8]) {
    let (_, mut p) = payload_of(golden);
    let base = M::decode(&p).unwrap();
    p.extend_from_slice(&[0xAA, 0xBB]);
    if M::TYPE != MsgType::VehicleName {
        assert_eq!(M::decode(&p).unwrap(), base);
    }
}

#[test]
fn trailing_bytes_ignored() {
    extra::<TelemetryMsg>(TELEMETRY_V2);
    extra::<StatusMsg>(STATUS);
    extra::<VitalsMsg>(VITALS);
    extra::<LinkMsg>(LINK);
    extra::<HelloMsg>(HELLO_HUD);
    extra::<ActionMsg>(ACTION);
    extra::<ActionAckMsg>(ACTION_ACK);
}

#[test]
fn short_payloads_rejected() {
    fn short<M: Message>() {
        if M::MIN_SIZE > 0 {
            let z = [0u8; 64];
            assert!(M::decode(&z[..M::MIN_SIZE - 1]).is_none());
        }
        assert!(M::decode(&[0u8; 64][..M::MIN_SIZE]).is_some());
    }
    short::<TelemetryMsg>();
    short::<StatusMsg>();
    short::<VitalsMsg>();
    short::<LinkMsg>();
    short::<HelloMsg>();
    short::<VehicleNameMsg>();
    short::<ActionMsg>();
    short::<ActionAckMsg>();
}

#[test]
fn unknown_type_passes_deframe() {
    let mut out = [0u8; MAX_FRAME];
    // build a raw frame with unknown type 0x7E through a known-type frame call is impossible; encode by hand
    let raw = [0x7Eu8, 0x01, 0x02, hud_proto::crc8::crc8(&[0x7E, 0x01, 0x02])];
    let n = hud_proto::cobs::encode(&raw, &mut out);
    let mut buf = [0u8; MAX_PAYLOAD + 2];
    let d = deframe(&out[..n], &mut buf).unwrap();
    assert_eq!(d.msg_type, 0x7E);
    assert_eq!(d.kind(), None);
    assert_eq!(d.payload, &[1, 2]);
}

fn drain(dec: &mut FrameDecoder, bytes: &[u8]) -> Vec<(u8, Vec<u8>)> {
    let mut v = Vec::new();
    for &b in bytes {
        if let Some(m) = dec.feed(b) {
            v.push((m.msg_type, m.payload.to_vec()));
        }
    }
    v
}

#[test]
fn stream_decoder_with_garbage() {
    let mut stream = Vec::new();
    stream.extend_from_slice(b"Starting HUD...\r\n");
    for (i, f) in ALL.iter().enumerate() {
        stream.extend_from_slice(f);
        if i == 4 {
            stream.extend_from_slice(b"Starting HUD...\r\n");
        }
    }
    let mut dec = FrameDecoder::new();
    let got = drain(&mut dec, &stream);
    assert_eq!(got.len(), 10);
    for (g, f) in got.iter().zip(ALL.iter()) {
        assert_eq!(*g, payload_of(f));
    }
}

#[test]
fn stream_decoder_overflow() {
    let mut dec = FrameDecoder::new();
    let mut stream = vec![0x00];
    stream.extend(core::iter::repeat(0x55).take(100));
    stream.extend_from_slice(VITALS);
    let got = drain(&mut dec, &stream);
    assert_eq!(got, vec![payload_of(VITALS)]);
}

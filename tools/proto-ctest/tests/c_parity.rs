use hud_proto::frame::*;
use hud_proto::messages::*;
use hud_proto_ctest::*;

fn wire<M: Message>(m: &M) -> Vec<u8> {
    let mut out = [0u8; MAX_FRAME];
    let n = frame_message(m, &mut out);
    out[..n].to_vec()
}

fn payload<M: Message>(m: &M) -> Vec<u8> {
    let mut b = [0u8; 64];
    let n = m.encode(&mut b);
    b[..n].to_vec()
}

fn status() -> StatusMsg {
    StatusMsg { annunciators: [1, 1, 1, 6, 2, 4, 1, 1, 5, 0], flags: 0x0A, flags_valid: 0x0F, gear: 3, master_mode: 2 }
}
fn telemetry() -> TelemetryMsg {
    TelemetryMsg { shield_front: 0xFF, shield_back: 0x80, shield_left: 0x40, shield_right: 0x20, hydrogen_fuel: 0xC8, quantum_fuel: 0x64, valid: 0x3F }
}
fn ack() -> ActionAckMsg {
    ActionAckMsg { action_id: 1, seq: 7, result: ACK_GAME_REJECTED, game_status: 7 }
}

#[test]
fn struct_sizes_match() {
    assert_eq!(c_sizeof(TelemetryMsg::TYPE as u8), TelemetryMsg::MAX_SIZE);
    assert_eq!(c_sizeof(StatusMsg::TYPE as u8), StatusMsg::MAX_SIZE);
    assert_eq!(c_sizeof(VitalsMsg::TYPE as u8), VitalsMsg::MAX_SIZE);
    assert_eq!(c_sizeof(LinkMsg::TYPE as u8), LinkMsg::MAX_SIZE);
    assert_eq!(c_sizeof(HelloMsg::TYPE as u8), HelloMsg::MAX_SIZE);
    assert_eq!(c_sizeof(VehicleNameMsg::TYPE as u8), VehicleNameMsg::MAX_SIZE);
    assert_eq!(c_sizeof(ActionMsg::TYPE as u8), ActionMsg::MAX_SIZE);
    assert_eq!(c_sizeof(ActionAckMsg::TYPE as u8), ActionAckMsg::MAX_SIZE);
    assert_eq!(c_sizeof(0x7E), 0);
}

#[test]
fn rust_frames_decode_in_c() {
    let t = telemetry();
    let s = status();
    let v = VitalsMsg { health: 200, oxygen: 0, valid: 1 };
    let l = LinkMsg { state: 2, source: 0 };
    let h = HelloMsg { version: 2, role: ROLE_HUD };
    let n = VehicleNameMsg::from_str_lossy("AEGS_Gladius");
    let a = ActionMsg { action_id: 1, seq: 7 };
    let k = ack();
    let mut stream = Vec::new();
    for w in [wire(&t), wire(&s), wire(&v), wire(&l), wire(&h), wire(&n), wire(&a), wire(&k)] {
        stream.extend(w);
    }
    let got = c_decode_stream(&stream);
    let want = vec![
        (0x01, payload(&t)),
        (0x02, payload(&s)),
        (0x03, payload(&v)),
        (0x04, payload(&l)),
        (0x05, payload(&h)),
        (0x06, payload(&n)),
        (0x10, payload(&a)),
        (0x11, payload(&k)),
    ];
    assert_eq!(got, want);

    // field-level parity
    assert_eq!(c_decode_telemetry(&payload(&t)).unwrap().to_vec(), payload(&t));
    let (ann, f, fv, g, m) = c_decode_status(&payload(&s)).unwrap();
    assert_eq!(ann, s.annunciators);
    assert_eq!((f, fv, g, m), (s.flags, s.flags_valid, s.gear, s.master_mode));
    assert_eq!(c_decode_vitals(&payload(&v)).unwrap(), [200, 0, 1]);
    assert_eq!(c_decode_link(&payload(&l)).unwrap(), [2, 0]);
    assert_eq!(c_decode_hello(&payload(&h)).unwrap(), [2, 1]);
    assert_eq!(c_decode_action_ack(&payload(&k)).unwrap(), [1, 7, 5, 7]);
    assert_eq!(c_decode_vehicle_name(&payload(&n)), n.as_str());
    assert_eq!(c_decode_vehicle_name(b"A\x01B"), "A?B");
}

#[test]
fn c_encoder_matches_golden() {
    let hello_hud = [0x00, 0x05, 0x05, 0x02, 0x01, 0x87, 0x00];
    let action = [0x00, 0x05, 0x10, 0x01, 0x07, 0xC7, 0x00];
    let ack_g = [0x00, 0x07, 0x11, 0x01, 0x07, 0x05, 0x07, 0x37, 0x00];
    assert_eq!(c_frame_encode(0x05, &[2, 1]), hello_hud);
    assert_eq!(c_frame_encode(0x10, &[1, 7]), action);
    assert_eq!(c_frame_encode(0x11, &[1, 7, 5, 7]), ack_g);
    // Rust decoder accepts them
    let mut dec = FrameDecoder::new();
    for g in [&hello_hud[..], &action[..], &ack_g[..]] {
        let mut got = None;
        for &b in g {
            if let Some(m) = dec.feed(b) {
                got = Some((m.msg_type, m.payload.to_vec()));
            }
        }
        assert!(got.is_some());
    }
    // oversize payload refused
    assert!(c_frame_encode(0x06, &[0x41; 65]).is_empty());
    // all-zero max payload encodes identically in both implementations
    let p = [0u8; 64];
    let c = c_frame_encode(0x06, &p);
    let mut rust = [0u8; MAX_FRAME];
    let n = frame(MsgType::VehicleName, &p, &mut rust);
    assert_eq!(c, rust[..n].to_vec());
}

#[test]
fn defaults_and_nibbles() {
    assert_eq!(c_decode_telemetry(&[1, 2, 3, 4, 5, 6]).unwrap()[6], 0x3F);
    assert!(c_decode_telemetry(&[1, 2, 3, 4, 5]).is_none());
    let s = status();
    let (ann, ..) = c_decode_status(&payload(&s)).unwrap();
    let r = StatusMsg::decode(&payload(&s)).unwrap();
    assert_eq!(ann, r.annunciators);
    assert_eq!(c_decode_link(&[1]).unwrap(), [1, 0]);
    assert_eq!(c_decode_action_ack(&[1, 2, 3]).unwrap(), [1, 2, 3, 0]);
}

#[test]
fn stream_garbage_and_overflow() {
    let v = wire(&VitalsMsg { health: 1, oxygen: 2, valid: 3 });
    let l = wire(&LinkMsg { state: 1, source: 0 });
    let mut s = b"Starting HUD...\r\n".to_vec();
    s.extend(&v);
    s.extend(b"junk\r\n");
    s.extend(&l);
    let got = c_decode_stream(&s);
    assert_eq!(got.len(), 2);
    assert_eq!(got[0].0, 0x03);
    assert_eq!(got[1].0, 0x04);

    let mut o = vec![0x00];
    o.extend(std::iter::repeat(0x55).take(100));
    o.extend(&v);
    let got = c_decode_stream(&o);
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].0, 0x03);
}

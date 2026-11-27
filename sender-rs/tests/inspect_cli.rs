use hud_sender::dumpfile::DumpWriter;
use hud_sender::testvec::*;
use std::process::Command;

fn run(args: &[&str]) -> (i32, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_hud-sender")).args(args).output().unwrap();
    (out.status.code().unwrap_or(-1), String::from_utf8_lossy(&out.stdout).into_owned())
}

#[test]
fn inspect_diff_decode_cli() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.sctd");
    {
        let f = std::fs::File::create(&path).unwrap();
        let mut w = DumpWriter::new(f).unwrap();
        w.write_frame(0, HEALTH_FRAME).unwrap();
        w.write_marker(500_000, "took damage").unwrap();
        w.write_frame(1_000_000, HEALTH_FRAME_50).unwrap();
        w.write_frame(2_000_000, VEHICLE_FRAME).unwrap();
        w.flush().unwrap();
    }
    let p = path.to_str().unwrap();

    let (code, out) = run(&["inspect", p]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("frames: 3"), "{out}");
    assert!(out.contains("near frame #1: took damage"), "{out}");

    let (code, out) = run(&["inspect", p, "--frame", "0", "--hex"]);
    assert_eq!(code, 0);
    assert!(out.contains("6.1.1.1 fixed32 f32=75"), "{out}");
    assert!(out.contains("32 0E 0A"), "{out}");

    let (code, out) = run(&["diff", p, "0", "1"]);
    assert_eq!(code, 0);
    assert!(out.contains("~ 6.1.1.1:") && out.contains("-25"), "{out}");

    let (code, out) = run(&["decode", p]);
    assert_eq!(code, 0);
    assert!(out.contains("link=Connected") && out.contains("health=0.750"), "{out}");
    assert!(out.contains("link=InVehicle") && out.contains("name=Gladius"), "{out}");

    let (code, _) = run(&["inspect", p, "--frame", "99"]);
    assert_ne!(code, 0);
}

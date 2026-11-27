//! Enforces that ship_hud/*.h are byte-identical copies of lib/**/*.h
//! (same rules as scripts/sync_headers.py --check).
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

fn headers(dir: PathBuf) -> BTreeMap<String, Vec<u8>> {
    let mut m = BTreeMap::new();
    for e in fs::read_dir(&dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
        let p = e.unwrap().path();
        if p.extension().map(|x| x == "h").unwrap_or(false) {
            m.insert(p.file_name().unwrap().to_string_lossy().into_owned(), fs::read(&p).unwrap());
        }
    }
    m
}

#[test]
fn ship_hud_headers_match_lib() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut src = BTreeMap::new();
    for d in ["lib/hud_proto", "lib/hud_widgets"] {
        for (k, v) in headers(root.join(d)) {
            assert!(src.insert(k.clone(), v).is_none(), "duplicate header basename {k}");
        }
    }
    let dst = headers(root.join("ship_hud"));
    let mut problems = Vec::new();
    for (k, v) in &src {
        match dst.get(k) {
            None => problems.push(format!("missing ship_hud/{k}")),
            Some(d) if d != v => problems.push(format!("differs ship_hud/{k}")),
            _ => {}
        }
    }
    for k in dst.keys() {
        if !src.contains_key(k) {
            problems.push(format!("stale ship_hud/{k}"));
        }
    }
    assert!(problems.is_empty(), "{problems:?}\nrun: python scripts/sync_headers.py");
}

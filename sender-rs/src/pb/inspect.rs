//! Schemaless inspection of protobuf frames: tree dump, flatten, diff, summary.
use crate::dumpfile::Record;
use crate::pb::wire::{parse_message, Field, WireError, WireValue};
use crate::sc::schema;
use std::collections::{BTreeMap, HashSet};
use std::fmt::Write as _;

const MAX_DEPTH: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PathSeg {
    pub num: u32,
    /// Occurrence index; only set when the field number occurs more than once in the same parent.
    pub idx: Option<u32>,
}

pub type Path = Vec<PathSeg>;

pub fn path_display(p: &[PathSeg]) -> String {
    let mut s = String::new();
    for (i, seg) in p.iter().enumerate() {
        if i > 0 {
            s.push('.');
        }
        let _ = write!(s, "{}", seg.num);
        if let Some(ix) = seg.idx {
            let _ = write!(s, "[{ix}]");
        }
    }
    s
}

fn path_name(p: &[PathSeg]) -> Option<&'static str> {
    let nums: Vec<u32> = p.iter().map(|s| s.num).collect();
    schema::path_name(&nums)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Leaf {
    pub display: String,
    pub num: Option<f64>,
}

enum Interp<'a> {
    Msg(Vec<Field<'a>>),
    Str(String),
    Bytes(&'a [u8]),
}

/// Interprets a length-delimited value. This is a heuristic: packed repeated values and some
/// byte strings may be shown as a message, and short printable messages as strings.
/// Order: printable ASCII string, then a fully-consumed non-empty message, then UTF-8, then bytes.
fn interpret(b: &[u8], depth: usize) -> Interp<'_> {
    if !b.is_empty() && b.iter().all(|&c| (0x20..=0x7E).contains(&c)) {
        return Interp::Str(String::from_utf8_lossy(b).into_owned());
    }
    if depth < MAX_DEPTH {
        if let Ok(f) = parse_message(b) {
            if !f.is_empty() {
                return Interp::Msg(f);
            }
        }
    }
    if let Ok(s) = std::str::from_utf8(b) {
        if !b.is_empty() {
            return Interp::Str(s.to_string());
        }
    }
    Interp::Bytes(b)
}

fn zigzag(u: u64) -> i64 {
    ((u >> 1) as i64) ^ -((u & 1) as i64)
}

fn hex_upper(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02X}")).collect::<Vec<_>>().join(" ")
}

fn bytes_desc(b: &[u8]) -> String {
    let mut s = format!("({} B) {}", b.len(), hex_upper(&b[..b.len().min(32)]));
    if b.len() > 32 {
        s.push_str(" \u{2026}");
    }
    if !b.is_empty() && b.len() % 4 == 0 {
        let vals: Vec<String> = b
            .chunks_exact(4)
            .take(8)
            .map(|c| format!("{}", f32::from_le_bytes(c.try_into().unwrap())))
            .collect();
        let _ = write!(s, " f32[]=[{}]", vals.join(", "));
    }
    s
}

fn with_indices(fields: &[Field<'_>], parent: &[PathSeg]) -> Vec<(Path, usize)> {
    let mut totals: BTreeMap<u32, u32> = BTreeMap::new();
    for f in fields {
        *totals.entry(f.number).or_default() += 1;
    }
    let mut seen: BTreeMap<u32, u32> = BTreeMap::new();
    fields
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let n = seen.entry(f.number).or_default();
            let idx = if totals[&f.number] > 1 { Some(*n) } else { None };
            *n += 1;
            let mut p = parent.to_vec();
            p.push(PathSeg { num: f.number, idx });
            (p, i)
        })
        .collect()
}

/// Human readable tree of one frame.
pub fn tree(frame: &[u8]) -> Result<String, WireError> {
    let fields = parse_message(frame)?;
    let mut out = String::new();
    tree_fields(&fields, &[], 0, &mut out);
    Ok(out)
}

fn tree_fields(fields: &[Field<'_>], parent: &[PathSeg], depth: usize, out: &mut String) {
    for (path, i) in with_indices(fields, parent) {
        let f = &fields[i];
        let p = path_display(&path);
        let indent = "  ".repeat(depth);
        let name = path_name(&path).map(|n| format!("  # {n}")).unwrap_or_default();
        match f.value {
            WireValue::Varint(u) => {
                let _ = writeln!(out, "{indent}{p} varint {u} (zz {}){name}", zigzag(u));
            }
            WireValue::Fixed32(u) => {
                let _ = writeln!(out, "{indent}{p} fixed32 f32={} u32={u}{name}", f32::from_bits(u));
            }
            WireValue::Fixed64(u) => {
                let _ = writeln!(out, "{indent}{p} fixed64 f64={} u64={u}{name}", f64::from_bits(u));
            }
            WireValue::Bytes(b) => match interpret(b, depth) {
                Interp::Msg(sub) => {
                    let _ = writeln!(out, "{indent}{p} msg ({} B){name}", b.len());
                    tree_fields(&sub, &path, depth + 1, out);
                }
                Interp::Str(s) => {
                    let _ = writeln!(out, "{indent}{p} str \"{}\"{name}", s.escape_debug());
                }
                Interp::Bytes(b) => {
                    let _ = writeln!(out, "{indent}{p} bytes {}{name}", bytes_desc(b));
                }
            },
        }
    }
}

/// Flattens a frame into its leaf values keyed by path. Messages are not leaves.
pub fn flatten(frame: &[u8]) -> Result<BTreeMap<Path, Leaf>, WireError> {
    let fields = parse_message(frame)?;
    let mut m = BTreeMap::new();
    flatten_fields(&fields, &[], 0, &mut m);
    Ok(m)
}

fn flatten_fields(fields: &[Field<'_>], parent: &[PathSeg], depth: usize, out: &mut BTreeMap<Path, Leaf>) {
    for (path, i) in with_indices(fields, parent) {
        match fields[i].value {
            WireValue::Varint(u) => {
                out.insert(path, Leaf { display: u.to_string(), num: Some(u as f64) });
            }
            WireValue::Fixed32(u) => {
                let f = f32::from_bits(u);
                out.insert(path, Leaf { display: f.to_string(), num: Some(f as f64) });
            }
            WireValue::Fixed64(u) => {
                let f = f64::from_bits(u);
                out.insert(path, Leaf { display: f.to_string(), num: Some(f) });
            }
            WireValue::Bytes(b) => match interpret(b, depth) {
                Interp::Msg(sub) => flatten_fields(&sub, &path, depth + 1, out),
                Interp::Str(s) => {
                    out.insert(path, Leaf { display: format!("\"{}\"", s.escape_debug()), num: None });
                }
                Interp::Bytes(b) => {
                    out.insert(path, Leaf { display: hex_upper(&b[..b.len().min(32)]), num: None });
                }
            },
        }
    }
}

/// Diff of two frames. Lines: `~ path: old -> new` (numeric: ` (Δ d)`), `+ path: new`, `- path: old`.
pub fn diff(a: &[u8], b: &[u8], ignore: &[String]) -> Result<Vec<String>, WireError> {
    let fa = flatten(a)?;
    let fb = flatten(b)?;
    let mut keys: Vec<&Path> = fa.keys().chain(fb.keys()).collect();
    keys.sort();
    keys.dedup();
    let mut out = Vec::new();
    for k in keys {
        let name = path_display(k);
        if ignore.iter().any(|pre| name.starts_with(pre.as_str())) {
            continue;
        }
        match (fa.get(k), fb.get(k)) {
            (Some(x), Some(y)) if x.display != y.display => {
                let mut line = format!("~ {name}: {} -> {}", x.display, y.display);
                if let (Some(n0), Some(n1)) = (x.num, y.num) {
                    let _ = write!(line, " (\u{394} {})", n1 - n0);
                }
                out.push(line);
            }
            (Some(_), Some(_)) => {}
            (None, Some(y)) => out.push(format!("+ {name}: {}", y.display)),
            (Some(x), None) => out.push(format!("- {name}: {}", x.display)),
            (None, None) => {}
        }
    }
    Ok(out)
}

struct Stat {
    count: usize,
    min: Option<f64>,
    max: Option<f64>,
    distinct: HashSet<String>,
    first: String,
    last: String,
}

/// Summary of a recording: frame count, duration, markers, then one line per leaf path.
pub fn summary(records: &[Record]) -> String {
    let frames: Vec<(u64, &Vec<u8>)> = records
        .iter()
        .filter_map(|r| match r {
            Record::Frame { ts_us, data } => Some((*ts_us, data)),
            _ => None,
        })
        .collect();
    let duration_us = records.iter().map(|r| r.ts_us()).max().unwrap_or(0);
    let mut out = String::new();
    let _ = writeln!(out, "frames: {}", frames.len());
    let _ = writeln!(out, "duration: {:.3}s", duration_us as f64 / 1e6);
    let markers: Vec<_> = records
        .iter()
        .filter_map(|r| match r {
            Record::Marker { ts_us, text } => Some((*ts_us, text)),
            _ => None,
        })
        .collect();
    if !markers.is_empty() {
        let _ = writeln!(out, "markers:");
        for (ts, text) in markers {
            let near = frames
                .iter()
                .position(|(t, _)| *t >= ts)
                .unwrap_or_else(|| frames.len().saturating_sub(1));
            let _ = writeln!(out, "  [t={:.3}s] near frame #{near}: {text}", ts as f64 / 1e6);
        }
    }
    let mut stats: BTreeMap<Path, Stat> = BTreeMap::new();
    let mut errors = 0usize;
    for (_, data) in &frames {
        match flatten(data) {
            Ok(m) => {
                for (p, leaf) in m {
                    let s = stats.entry(p).or_insert_with(|| Stat {
                        count: 0,
                        min: None,
                        max: None,
                        distinct: HashSet::new(),
                        first: leaf.display.clone(),
                        last: String::new(),
                    });
                    s.count += 1;
                    if let Some(n) = leaf.num {
                        s.min = Some(s.min.map_or(n, |m| m.min(n)));
                        s.max = Some(s.max.map_or(n, |m| m.max(n)));
                    }
                    if s.distinct.len() < 1000 {
                        s.distinct.insert(leaf.display.clone());
                    }
                    s.last = leaf.display;
                }
            }
            Err(_) => errors += 1,
        }
    }
    if errors > 0 {
        let _ = writeln!(out, "undecodable frames: {errors}");
    }
    for (p, s) in &stats {
        let name = path_name(p).map(|n| format!("  # {n}")).unwrap_or_default();
        let fmt = |v: Option<f64>| v.map_or("-".to_string(), |x| x.to_string());
        let _ = writeln!(
            out,
            "{}  n={}  min={}  max={}  distinct={}  first={}  last={}{name}",
            path_display(p),
            s.count,
            fmt(s.min),
            fmt(s.max),
            s.distinct.len(),
            s.first,
            s.last
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dumpfile::DumpWriter;
    use crate::testvec::*;

    #[test]
    fn tree_health_frame() {
        let t = tree(HEALTH_FRAME).unwrap();
        assert!(t.contains("6 msg (14 B)"), "{t}");
        assert!(t.contains("6.1.1.1 fixed32 f32=75"), "{t}");
        assert!(t.contains("6.1.1.2 fixed32 f32=100"), "{t}");
        assert!(t.contains("actor.vitals.health.current"), "{t}");
    }

    #[test]
    fn tree_vehicle_frame() {
        let t = tree(VEHICLE_FRAME).unwrap();
        assert!(t.contains("3.2.1 str \"Gladius\""), "{t}");
        assert!(t.contains("3.2.10.2 bytes (16 B)"), "{t}");
        assert!(t.contains("f32[]=[1, 0.5, 0.25, 0]"), "{t}");
    }

    #[test]
    fn diff_health() {
        let d = diff(HEALTH_FRAME, HEALTH_FRAME_50, &[]).unwrap();
        assert_eq!(d.len(), 1, "{d:?}");
        assert!(d[0].starts_with("~ 6.1.1.1:"), "{d:?}");
        assert!(d[0].contains("(\u{394} -25)"), "{d:?}");
        assert!(diff(HEALTH_FRAME, HEALTH_FRAME_50, &["6.1".to_string()]).unwrap().is_empty());
    }

    #[test]
    fn diff_added_removed() {
        let d = diff(HEALTH_FRAME, ACTOR_ON_FOOT, &[]).unwrap();
        assert!(d.iter().any(|l| l.starts_with("+ 6.2.1:")), "{d:?}");
        assert!(d.iter().any(|l| l.starts_with("- 6.1.1.1:")), "{d:?}");
    }

    #[test]
    fn repeated_fields_get_indices() {
        let mut b = Vec::new();
        crate::pb::encode::put_varint_field(&mut b, 5, 1);
        crate::pb::encode::put_varint_field(&mut b, 5, 2);
        let t = tree(&b).unwrap();
        assert!(t.contains("5[0] varint 1"), "{t}");
        assert!(t.contains("5[1] varint 2"), "{t}");
    }

    #[test]
    fn summary_marker_near_frame() {
        let mut buf = Vec::new();
        let mut w = DumpWriter::new(&mut buf).unwrap();
        w.write_frame(1_000_000, HEALTH_FRAME).unwrap();
        w.write_frame(2_000_000, HEALTH_FRAME_50).unwrap();
        w.write_marker(2_500_000, "gear up").unwrap();
        w.write_frame(3_000_000, HEALTH_FRAME).unwrap();
        let recs = crate::dumpfile::read_all(&buf[..]).unwrap();
        let s = summary(&recs);
        assert!(s.contains("frames: 3"), "{s}");
        assert!(s.contains("[t=2.500s] near frame #2: gear up"), "{s}");
        assert!(s.contains("6.1.1.1  n=3  min=50  max=75  distinct=2"), "{s}");
    }
}

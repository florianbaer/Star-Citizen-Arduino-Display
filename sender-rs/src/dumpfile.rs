//! "SCTD v1" recording format.
//!
//! Header (8 bytes): b"SCTD", version u8 = 1, 3 zero bytes.
//! Records: kind u8 (0 = frame, 1 = marker), ts_us u64 LE, len u32 LE, then `len` bytes
//! (a raw `TelemetryStreamResponse` for frames, UTF-8 text for markers).
use anyhow::{anyhow, bail, Context, Result};
use std::io::{self, Read, Write};
use std::path::Path;

pub const MAGIC: &[u8; 4] = b"SCTD";
pub const VERSION: u8 = 1;
pub const MAX_RECORD_LEN: u32 = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Record {
    Frame { ts_us: u64, data: Vec<u8> },
    Marker { ts_us: u64, text: String },
}

impl Record {
    pub fn ts_us(&self) -> u64 {
        match self {
            Record::Frame { ts_us, .. } | Record::Marker { ts_us, .. } => *ts_us,
        }
    }
}

pub struct DumpWriter<W: Write> {
    w: W,
}

impl<W: Write> DumpWriter<W> {
    pub fn new(mut w: W) -> io::Result<Self> {
        w.write_all(MAGIC)?;
        w.write_all(&[VERSION, 0, 0, 0])?;
        Ok(Self { w })
    }

    fn write_record(&mut self, kind: u8, ts_us: u64, data: &[u8]) -> io::Result<()> {
        let len = u32::try_from(data.len())
            .ok()
            .filter(|&l| l <= MAX_RECORD_LEN)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "record too large"))?;
        self.w.write_all(&[kind])?;
        self.w.write_all(&ts_us.to_le_bytes())?;
        self.w.write_all(&len.to_le_bytes())?;
        self.w.write_all(data)
    }

    pub fn write_frame(&mut self, ts_us: u64, data: &[u8]) -> io::Result<()> {
        self.write_record(0, ts_us, data)
    }

    pub fn write_marker(&mut self, ts_us: u64, text: &str) -> io::Result<()> {
        self.write_record(1, ts_us, text.as_bytes())
    }

    pub fn flush(&mut self) -> io::Result<()> {
        self.w.flush()
    }
}

/// Reads the whole stream. Errors on wrong magic, version != 1, unknown kind,
/// truncated records (message includes the byte offset) and len > 16 MiB.
pub fn read_all<R: Read>(mut r: R) -> Result<Vec<Record>> {
    let mut all = Vec::new();
    r.read_to_end(&mut all)?;
    parse(&all)
}

fn parse(all: &[u8]) -> Result<Vec<Record>> {
    if all.len() < 8 || &all[..4] != MAGIC {
        bail!("not an SCTD file (bad magic)");
    }
    if all[4] != VERSION {
        bail!("unsupported SCTD version {}", all[4]);
    }
    let mut pos = 8usize;
    let mut out = Vec::new();
    while pos < all.len() {
        let start = pos;
        if all.len() - pos < 13 {
            bail!("truncated record header at byte offset {start}");
        }
        let kind = all[pos];
        let ts_us = u64::from_le_bytes(all[pos + 1..pos + 9].try_into().unwrap());
        let len = u32::from_le_bytes(all[pos + 9..pos + 13].try_into().unwrap());
        if len > MAX_RECORD_LEN {
            bail!("record length {len} exceeds 16 MiB at byte offset {start}");
        }
        pos += 13;
        let end = pos + len as usize;
        if end > all.len() {
            bail!("truncated record body at byte offset {start} (need {len} bytes, have {})", all.len() - pos);
        }
        let body = &all[pos..end];
        pos = end;
        out.push(match kind {
            0 => Record::Frame { ts_us, data: body.to_vec() },
            1 => Record::Marker {
                ts_us,
                text: String::from_utf8(body.to_vec()).map_err(|_| anyhow!("marker is not UTF-8 at byte offset {start}"))?,
            },
            k => bail!("unknown record kind {k} at byte offset {start}"),
        });
    }
    Ok(out)
}

/// Loads an SCTD file; a file without the SCTD magic is treated as one raw frame (ts 0).
pub fn load_frames_or_raw(path: &Path) -> Result<Vec<Record>> {
    let data = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    if data.len() >= 4 && &data[..4] == MAGIC {
        parse(&data)
    } else {
        Ok(vec![Record::Frame { ts_us: 0, data }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        let mut buf = Vec::new();
        let mut w = DumpWriter::new(&mut buf).unwrap();
        w.write_frame(10, &[1, 2, 3]).unwrap();
        w.write_marker(20, "gear up").unwrap();
        w.write_frame(30, &[]).unwrap();
        w.flush().unwrap();
        buf
    }

    #[test]
    fn roundtrip_via_tempfile() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.sctd");
        std::fs::write(&p, sample()).unwrap();
        let recs = load_frames_or_raw(&p).unwrap();
        assert_eq!(
            recs,
            vec![
                Record::Frame { ts_us: 10, data: vec![1, 2, 3] },
                Record::Marker { ts_us: 20, text: "gear up".into() },
                Record::Frame { ts_us: 30, data: vec![] },
            ]
        );
        assert_eq!(read_all(std::fs::File::open(&p).unwrap()).unwrap().len(), 3);
    }

    #[test]
    fn truncated_is_error_with_offset() {
        let mut b = sample();
        b.truncate(b.len() - 20);
        let e = read_all(&b[..]).unwrap_err().to_string();
        assert!(e.contains("offset"), "{e}");
    }

    #[test]
    fn bad_magic_version_kind() {
        assert!(read_all(&b"XXXX\x01\0\0\0"[..]).is_err());
        assert!(read_all(&b"SCTD\x02\0\0\0"[..]).is_err());
        let mut b = b"SCTD\x01\0\0\0".to_vec();
        b.extend_from_slice(&[9; 13]);
        assert!(read_all(&b[..]).is_err());
    }

    #[test]
    fn oversize_len_rejected() {
        let mut b = b"SCTD\x01\0\0\0".to_vec();
        b.push(0);
        b.extend_from_slice(&0u64.to_le_bytes());
        b.extend_from_slice(&(MAX_RECORD_LEN + 1).to_le_bytes());
        assert!(read_all(&b[..]).unwrap_err().to_string().contains("16 MiB"));
    }

    #[test]
    fn raw_fallback() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("raw.bin");
        std::fs::write(&p, [0x32, 0x00]).unwrap();
        assert_eq!(load_frames_or_raw(&p).unwrap(), vec![Record::Frame { ts_us: 0, data: vec![0x32, 0] }]);
    }
}

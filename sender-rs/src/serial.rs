//! Serial transport: a writer thread and a reader thread, plus a stdout "link" for dry runs.
use anyhow::{Context, Result};
use hud_proto::frame::{deframe, FrameDecoder, MAX_PAYLOAD};
use hud_proto::messages::MsgType;
use std::io::{Read, Write};
use std::time::Duration;

pub struct SerialLink {
    pub tx: std::sync::mpsc::Sender<Vec<u8>>,
    pub rx: tokio::sync::mpsc::Receiver<(u8, Vec<u8>)>,
}

/// Opens the port, clones it for the reader and spawns the two threads.
pub fn open(port: &str, baud: u32) -> Result<SerialLink> {
    let p = serialport::new(port, baud)
        .timeout(Duration::from_millis(200))
        .open()
        .with_context(|| format!("Failed to open serial port {port}"))?;
    let reader = p.try_clone().context("Failed to clone serial port for reading")?;
    let (wtx, wrx) = std::sync::mpsc::channel::<Vec<u8>>();
    let (rtx, rrx) = tokio::sync::mpsc::channel(64);
    std::thread::spawn(move || {
        if let Err(e) = writer_loop(p, wrx) {
            eprintln!("[serial] write error: {e}");
        }
    });
    std::thread::spawn(move || reader_loop(reader, rtx));
    Ok(SerialLink { tx: wtx, rx: rrx })
}

/// write_all + flush per frame. Returns Ok when the channel closes.
pub fn writer_loop<W: Write>(mut w: W, rx: std::sync::mpsc::Receiver<Vec<u8>>) -> std::io::Result<()> {
    for frame in rx {
        w.write_all(&frame)?;
        w.flush()?;
    }
    Ok(())
}

/// Feeds a `FrameDecoder`; ignores timeouts; exits on EOF, other errors or a closed channel.
pub fn reader_loop<R: Read>(mut r: R, tx: tokio::sync::mpsc::Sender<(u8, Vec<u8>)>) {
    let mut dec = FrameDecoder::new();
    let mut buf = [0u8; 64];
    loop {
        match r.read(&mut buf) {
            Ok(0) => return,
            Ok(n) => {
                for &b in &buf[..n] {
                    if let Some(m) = dec.feed(b) {
                        if tx.blocking_send((m.msg_type, m.payload.to_vec())).is_err() {
                            return;
                        }
                    }
                }
            }
            Err(e) if matches!(e.kind(), std::io::ErrorKind::TimedOut | std::io::ErrorKind::Interrupted) => continue,
            Err(_) => return,
        }
    }
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02X}")).collect::<Vec<_>>().join(" ")
}

/// "TX {type name} [{payload hex}] {frame hex}"
pub fn format_tx(frame: &[u8]) -> String {
    if frame.len() >= 2 {
        let mut scratch = [0u8; MAX_PAYLOAD + 2];
        if let Some(d) = deframe(&frame[1..frame.len() - 1], &mut scratch) {
            let name = d.kind().map(|k: MsgType| k.name().to_string()).unwrap_or_else(|| format!("0x{:02X}", d.msg_type));
            return format!("TX {name} [{}] {}", hex(d.payload), hex(frame));
        }
    }
    format!("TX ? [] {}", hex(frame))
}

/// PORT "-": the writer prints each frame; rx never yields.
pub fn stdout_link() -> SerialLink {
    let (wtx, wrx) = std::sync::mpsc::channel::<Vec<u8>>();
    let (rtx, rrx) = tokio::sync::mpsc::channel(1);
    std::thread::spawn(move || {
        // Keep the tokio Sender alive so rx.recv() stays pending instead of returning None at once.
        let _keep = rtx;
        for frame in wrx {
            println!("{}", format_tx(&frame));
        }
    });
    SerialLink { tx: wtx, rx: rrx }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn writer_writes_each_frame() {
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(vec![1, 2]).unwrap();
        tx.send(vec![3]).unwrap();
        drop(tx);
        let mut out = Vec::new();
        writer_loop(&mut out, rx).unwrap();
        assert_eq!(out, vec![1, 2, 3]);
    }

    #[test]
    fn reader_decodes_frames_amid_garbage() {
        let hello = [0x00u8, 0x05, 0x05, 0x02, 0x01, 0x87, 0x00];
        let ack = [0x00u8, 0x07, 0x11, 0x01, 0x07, 0x05, 0x07, 0x37, 0x00];
        let mut data = b"Starting HUD...\r\n".to_vec();
        data.extend_from_slice(&hello);
        data.extend_from_slice(b"junk");
        data.extend_from_slice(&ack);
        let (tx, mut rx) = tokio::sync::mpsc::channel(8);
        reader_loop(Cursor::new(data), tx);
        assert_eq!(rx.try_recv().unwrap(), (0x05, vec![2, 1]));
        assert_eq!(rx.try_recv().unwrap(), (0x11, vec![1, 7, 5, 7]));
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn format_tx_line() {
        let l = format_tx(&[0x00, 0x03, 0x04, 0x02, 0x02, 0xF0, 0x00]);
        assert!(l.starts_with("TX Link [02 00] "), "{l}");
    }
}

use bytes::Bytes;
use hud_sender::sc::{mock, source};
use hud_sender::snapshot::{LinkState, Snapshot, SourceKind};
use hud_sender::testvec::{HEALTH_FRAME, HEALTH_FRAME_50};
use std::time::{Duration, Instant};
use tokio::sync::watch;

async fn start_mock(cfg: mock::MockConfig) -> (u16, tokio::task::JoinHandle<anyhow::Result<()>>) {
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = l.local_addr().unwrap().port();
    (port, tokio::spawn(mock::serve(l, cfg)))
}

fn start_source(port: u16) -> (watch::Receiver<Snapshot>, tokio::task::JoinHandle<anyhow::Result<()>>) {
    let (tx, rx) = watch::channel(Snapshot::no_game(SourceKind::Sc));
    let cfg = source::ScConfig::new(&format!("127.0.0.1:{port}"));
    (rx, tokio::spawn(source::run(cfg, tx)))
}

/// Polls every 5 ms until `pred` holds or `deadline` passes.
async fn wait_for(rx: &watch::Receiver<Snapshot>, deadline: Duration, pred: impl Fn(&Snapshot) -> bool) -> bool {
    let end = Instant::now() + deadline;
    while Instant::now() < end {
        if pred(&rx.borrow().clone()) {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    false
}

fn health(s: &Snapshot) -> Option<f32> {
    s.actor.as_ref().and_then(|a| a.health)
}

#[tokio::test(flavor = "multi_thread")]
async fn health_frame_reaches_snapshot() {
    let cfg = mock::MockConfig::new(vec![Bytes::from_static(HEALTH_FRAME)], Duration::from_millis(20), true);
    let (port, server) = start_mock(cfg).await;
    let (rx, task) = start_source(port);
    assert!(
        wait_for(&rx, Duration::from_secs(5), |s| health(s) == Some(0.75) && s.link == LinkState::Connected).await
    );
    task.abort();
    server.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn decode_error_does_not_kill_stream_and_server_loss_gives_no_game() {
    let truncated = Bytes::from_static(&HEALTH_FRAME[..HEALTH_FRAME.len() - 1]);
    let cfg = mock::MockConfig::new(
        vec![Bytes::from_static(HEALTH_FRAME), truncated, Bytes::from_static(HEALTH_FRAME_50)],
        Duration::from_millis(50),
        false,
    );
    let (port, server) = start_mock(cfg).await;
    let (rx, task) = start_source(port);
    // 0.5 only exists after the bad frame.
    assert!(wait_for(&rx, Duration::from_secs(5), |s| health(s) == Some(0.5)).await);

    server.abort();
    let _ = server.await;
    assert!(wait_for(&rx, Duration::from_secs(5), |s| s.link == LinkState::NoGame).await);
    // and stays NoGame
    let end = Instant::now() + Duration::from_secs(1);
    while Instant::now() < end {
        assert_eq!(rx.borrow().link, LinkState::NoGame);
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    task.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn nothing_listening_stays_no_game() {
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    drop(l);
    let (rx, task) = start_source(port);
    let end = Instant::now() + Duration::from_millis(1500);
    while Instant::now() < end {
        assert_eq!(rx.borrow().link, LinkState::NoGame);
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert!(!task.is_finished());
    task.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn synthetic_frames_go_from_vehicle_to_on_foot() {
    let cfg = mock::MockConfig::new(mock::synthetic_frames(), Duration::from_millis(10), true);
    let (port, server) = start_mock(cfg).await;
    let (rx, task) = start_source(port);
    assert!(wait_for(&rx, Duration::from_secs(5), |s| s.link == LinkState::InVehicle).await);
    assert!(wait_for(&rx, Duration::from_secs(5), |s| s.link == LinkState::Connected).await);
    task.abort();
    server.abort();
}

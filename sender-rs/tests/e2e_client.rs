use bytes::Bytes;
use hud_sender::sc::{client, mock};
use hud_sender::testvec::HEALTH_FRAME;
use std::time::Duration;

#[tokio::test(flavor = "multi_thread")]
async fn stream_and_list_against_mock() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let cfg = mock::MockConfig::new(vec![Bytes::from_static(HEALTH_FRAME)], Duration::from_millis(20), true);
    let server = tokio::spawn(mock::serve(listener, cfg));

    let ep = format!("127.0.0.1:{}", addr.port());
    let ch = client::connect(&ep, Duration::from_secs(2)).await.unwrap();
    let mut stream = client::open_telemetry_stream(ch.clone()).await.unwrap();
    let first = tokio::time::timeout(Duration::from_secs(5), stream.message()).await.unwrap().unwrap().unwrap();
    assert_eq!(&first[..], HEALTH_FRAME);

    let actions = client::list_actions(ch).await.unwrap();
    assert_eq!(actions.len(), 5);
    server.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn connect_to_closed_port_fails_quickly() {
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    drop(l);
    let t = std::time::Instant::now();
    let r = client::connect(&format!("127.0.0.1:{port}"), Duration::from_secs(2)).await;
    assert!(r.is_err());
    assert!(t.elapsed() < Duration::from_secs(4));
}

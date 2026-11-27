use bytes::Bytes;
use hud_proto::messages::*;
use hud_sender::actions::{execute, lookup};
use hud_sender::sc::{client, mock};
use std::time::Duration;

async fn start(trigger_status: i32) -> (String, std::sync::Arc<std::sync::Mutex<Vec<(String, Option<u32>)>>>, tokio::task::JoinHandle<anyhow::Result<()>>) {
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = l.local_addr().unwrap().port();
    let mut cfg = mock::MockConfig::new(vec![Bytes::from_static(b"")], Duration::from_millis(50), true);
    cfg.trigger_status = trigger_status;
    let log = cfg.action_log.clone();
    (format!("127.0.0.1:{port}"), log, tokio::spawn(mock::serve(l, cfg)))
}

#[tokio::test(flavor = "multi_thread")]
async fn execute_ok_and_gate_layers() {
    let (ep, log, server) = start(1).await;
    let ping = lookup(5).unwrap();
    assert_eq!(execute(&ep, ping).await, (ACK_OK, 0));
    assert_eq!(log.lock().unwrap().clone(), vec![("v_invoke_ping".to_string(), None)]);

    let ch = client::connect(&ep, Duration::from_secs(2)).await.unwrap();
    for name in ["v_self_destruct", "v_not_in_table", "v_eject"] {
        let e = client::trigger_action(ch.clone(), name, None).await.unwrap_err();
        assert_eq!(e.code(), tonic::Code::PermissionDenied, "{name}");
    }
    // nothing left the process
    assert_eq!(log.lock().unwrap().len(), 1);
    assert_eq!(client::list_actions(ch).await.unwrap().len(), 5);
    server.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn game_rejection_is_reported() {
    let (ep, _log, server) = start(7).await;
    assert_eq!(execute(&ep, lookup(5).unwrap()).await, (ACK_GAME_REJECTED, 7));
    server.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn execute_without_server_is_not_connected() {
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    drop(l);
    assert_eq!(execute(&format!("127.0.0.1:{port}"), lookup(5).unwrap()).await, (ACK_NOT_CONNECTED, 0));
}

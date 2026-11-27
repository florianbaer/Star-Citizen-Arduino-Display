//! gRPC client calls. `trigger_action` is the ONLY function in the code base that sends TriggerGameAction.
use crate::actions;
use crate::sc::codec::RawCodec;
use crate::sc::schema::{self, ListTriggerableActionsResponse, TriggerGameActionRequest, TriggerGameActionResponse, TriggerableAction};
use bytes::Bytes;
use prost::Message as _;
use std::time::Duration;
use tonic::codegen::http::uri::PathAndQuery;
use tonic::transport::{Channel, Endpoint};
use tonic::{Request, Status};

/// Prepends `http://` when the value has no scheme.
pub fn normalize_endpoint(s: &str) -> String {
    let s = s.trim();
    if s.contains("://") {
        s.to_string()
    } else {
        format!("http://{s}")
    }
}

pub async fn connect(endpoint: &str, timeout: Duration) -> anyhow::Result<Channel> {
    let ep = Endpoint::from_shared(normalize_endpoint(endpoint))?
        .connect_timeout(timeout)
        .http2_keep_alive_interval(Duration::from_secs(10))
        .keep_alive_timeout(Duration::from_secs(5))
        .keep_alive_while_idle(true);
    match tokio::time::timeout(timeout + Duration::from_secs(1), ep.connect()).await {
        Ok(r) => Ok(r?),
        Err(_) => anyhow::bail!("connect timed out after {timeout:?}"),
    }
}

async fn ready(ch: Channel) -> Result<tonic::client::Grpc<Channel>, Status> {
    let mut grpc = tonic::client::Grpc::new(ch);
    grpc.ready().await.map_err(|e| Status::unavailable(e.to_string()))?;
    Ok(grpc)
}

pub async fn open_telemetry_stream(ch: Channel) -> Result<tonic::Streaming<Bytes>, Status> {
    let mut grpc = ready(ch).await?;
    let resp = grpc
        .server_streaming(
            Request::new(Bytes::new()),
            PathAndQuery::from_static(schema::PATH_TELEMETRY_STREAM),
            RawCodec,
        )
        .await?;
    Ok(resp.into_inner())
}

/// Sends TriggerGameAction. The allowlist check is the first statement and runs before any network I/O.
pub async fn trigger_action(ch: Channel, name: &str, hold_ms: Option<u32>) -> Result<TriggerGameActionResponse, Status> {
    if !actions::is_allowed(name) {
        return Err(Status::permission_denied("action not in allowlist"));
    }
    let mut grpc = ready(ch).await?;
    let req = TriggerGameActionRequest { action_name: name.to_string(), hold_ms };
    let resp: Bytes = grpc
        .unary(
            Request::new(Bytes::from(req.encode_to_vec())),
            PathAndQuery::from_static(schema::PATH_TRIGGER_GAME_ACTION),
            RawCodec,
        )
        .await?
        .into_inner();
    TriggerGameActionResponse::decode(resp).map_err(|e| Status::internal(e.to_string()))
}

pub async fn list_actions(ch: Channel) -> Result<Vec<TriggerableAction>, Status> {
    let mut grpc = ready(ch).await?;
    let resp: Bytes = grpc
        .unary(
            Request::new(Bytes::new()),
            PathAndQuery::from_static(schema::PATH_LIST_TRIGGERABLE_ACTIONS),
            RawCodec,
        )
        .await?
        .into_inner();
    ListTriggerableActionsResponse::decode(resp)
        .map(|r| r.actions)
        .map_err(|e| Status::internal(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_normalisation() {
        assert_eq!(normalize_endpoint("127.0.0.1:2942"), "http://127.0.0.1:2942");
        assert_eq!(normalize_endpoint("http://x:1"), "http://x:1");
    }
}

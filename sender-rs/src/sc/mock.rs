//! In-process fake GameTelemetryService (tests and the `mock-server` subcommand).
use crate::actions;
use crate::cli::MockArgs;
use crate::dumpfile::{self, Record};
use crate::sc::codec::RawCodec;
use crate::sc::schema::*;
use bytes::Bytes;
use prost::Message as _;
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;
use tonic::codegen::http;
use tonic::{Request, Response, Status};

#[derive(Clone)]
pub struct MockConfig {
    /// Each = one TelemetryStreamResponse.
    pub frames: Vec<Bytes>,
    pub interval: Duration,
    /// Loop forever, or end the stream after the last frame.
    pub repeat: bool,
    pub action_log: Arc<Mutex<Vec<(String, Option<u32>)>>>,
    /// Status returned by every TriggerGameAction (1 = OK).
    pub trigger_status: i32,
}

impl MockConfig {
    pub fn new(frames: Vec<Bytes>, interval: Duration, repeat: bool) -> Self {
        Self { frames, interval, repeat, action_log: Arc::new(Mutex::new(Vec::new())), trigger_status: TRIGGER_STATUS_OK }
    }
}

#[derive(Clone)]
struct MockService(Arc<MockConfig>);

impl tonic::server::NamedService for MockService {
    const NAME: &'static str = SERVICE;
}

type BoxStream = Pin<Box<dyn tokio_stream::Stream<Item = Result<Bytes, Status>> + Send>>;
type BoxFut<T> = Pin<Box<dyn Future<Output = Result<Response<T>, Status>> + Send>>;

struct StreamSvc(Arc<MockConfig>);
impl tonic::server::ServerStreamingService<Bytes> for StreamSvc {
    type Response = Bytes;
    type ResponseStream = BoxStream;
    type Future = BoxFut<BoxStream>;
    fn call(&mut self, _r: Request<Bytes>) -> Self::Future {
        let cfg = self.0.clone();
        Box::pin(async move {
            let (tx, rx) = tokio::sync::mpsc::channel(16);
            tokio::spawn(async move {
                if cfg.frames.is_empty() {
                    return;
                }
                loop {
                    for frame in &cfg.frames {
                        if tx.send(Ok(frame.clone())).await.is_err() {
                            return; // client went away
                        }
                        tokio::time::sleep(cfg.interval).await;
                    }
                    if !cfg.repeat {
                        return;
                    }
                }
            });
            Ok(Response::new(Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx)) as BoxStream))
        })
    }
}

struct TriggerSvc(Arc<MockConfig>);
impl tonic::server::UnaryService<Bytes> for TriggerSvc {
    type Response = Bytes;
    type Future = BoxFut<Bytes>;
    fn call(&mut self, r: Request<Bytes>) -> Self::Future {
        let cfg = self.0.clone();
        Box::pin(async move {
            let req = TriggerGameActionRequest::decode(r.into_inner()).map_err(|e| Status::invalid_argument(e.to_string()))?;
            cfg.action_log.lock().unwrap().push((req.action_name, req.hold_ms));
            let resp = TriggerGameActionResponse { status: cfg.trigger_status, message: String::new() };
            Ok(Response::new(Bytes::from(resp.encode_to_vec())))
        })
    }
}

struct ListSvc;
impl tonic::server::UnaryService<Bytes> for ListSvc {
    type Response = Bytes;
    type Future = BoxFut<Bytes>;
    fn call(&mut self, _r: Request<Bytes>) -> Self::Future {
        Box::pin(async move {
            let resp = ListTriggerableActionsResponse {
                actions: actions::ACTIONS
                    .iter()
                    .map(|a| TriggerableAction { action_name: a.name.to_string(), category: "mock".into(), holdable: false })
                    .collect(),
            };
            Ok(Response::new(Bytes::from(resp.encode_to_vec())))
        })
    }
}

impl tonic::codegen::Service<http::Request<tonic::body::Body>> for MockService {
    type Response = http::Response<tonic::body::Body>;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Infallible>> + Send>>;
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        Poll::Ready(Ok(()))
    }
    fn call(&mut self, req: http::Request<tonic::body::Body>) -> Self::Future {
        let path = req.uri().path().to_string();
        let cfg = self.0.clone();
        Box::pin(async move {
            match path.as_str() {
                PATH_TELEMETRY_STREAM => {
                    let mut g = tonic::server::Grpc::new(RawCodec);
                    Ok(g.server_streaming(StreamSvc(cfg), req).await)
                }
                PATH_TRIGGER_GAME_ACTION => {
                    let mut g = tonic::server::Grpc::new(RawCodec);
                    Ok(g.unary(TriggerSvc(cfg), req).await)
                }
                PATH_LIST_TRIGGERABLE_ACTIONS => {
                    let mut g = tonic::server::Grpc::new(RawCodec);
                    Ok(g.unary(ListSvc, req).await)
                }
                // Reflection and anything else: unimplemented (the code path `probe` must handle).
                _ => Ok(Status::unimplemented("").into_http()),
            }
        })
    }
}

pub async fn serve(listener: tokio::net::TcpListener, cfg: MockConfig) -> anyhow::Result<()> {
    tonic::transport::Server::builder()
        .add_service(MockService(Arc::new(cfg)))
        .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener))
        .await?;
    Ok(())
}

/// 300 frames at the intended 10 Hz (30 s cycle): 0..199 in a vehicle, 200..299 on foot.
pub fn synthetic_frames() -> Vec<Bytes> {
    use std::f32::consts::TAU;
    (0..300usize)
        .map(|i| {
            let t = i as f32;
            let resp = if i < 200 {
                let faces: Vec<f32> = (0..4).map(|k| 0.5 + 0.5 * (TAU * (t / 60.0 + k as f32 / 4.0)).sin()).collect();
                let level = faces.iter().sum::<f32>() / 4.0;
                let shld = if (120..140).contains(&i) { 5 } else { 1 };
                TelemetryStreamResponse {
                    vehicle: Some(VehicleFrame {
                        state: Some(VehicleState {
                            name: "MOCK Gladius".into(),
                            lights_enabled: true,
                            annunciators: Some(VehicleAnnunciator {
                                pwr: 1, wpn: 1, thr: 1, shld, cool: 1, qdrv: 1, radr: 1, prox: 1, hit: 1, misl: 1,
                            }),
                            flight: Some(VehicleFlight {
                                state: Some(VehicleFlightState { is_landed: i < 50, gear: Some(DEPLOY_DEPLOYED) }),
                                settings: None,
                                instruments: Some(VehicleFlightInstruments {
                                    speed: 0.0,
                                    hydrogen_fuel: 1.0 - 0.8 * t / 200.0,
                                    quantum_fuel: 0.9 - 0.5 * t / 200.0,
                                    g_meter: None,
                                }),
                            }),
                            shields: Some(VehicleShields { level, faces }),
                            master_mode: Some(VehicleMasterModeState { mode: 2, spooled: (150..200).contains(&i) }),
                            ..Default::default()
                        }),
                    }),
                    events: None,
                    actor: Some(ActorFrame {
                        vitals: Some(ActorVitals { health: Some(Gauge { current: 100.0, max: 100.0 }), oxygen: None }),
                        state: Some(ActorState { locomotion: LOCOMOTION_SEATED, ..Default::default() }),
                    }),
                }
            } else {
                let health = 100.0 - 40.0 * (i - 200) as f32 / 99.0;
                TelemetryStreamResponse {
                    vehicle: None,
                    events: None,
                    actor: Some(ActorFrame {
                        vitals: Some(ActorVitals {
                            health: Some(Gauge { current: health, max: 100.0 }),
                            oxygen: Some(Gauge { current: 80.0, max: 100.0 }),
                        }),
                        state: Some(ActorState { locomotion: LOCOMOTION_ON_FOOT, ..Default::default() }),
                    }),
                }
            };
            Bytes::from(resp.encode_to_vec())
        })
        .collect()
}

/// `hud-sender mock-server`
pub async fn run_cli(args: MockArgs) -> anyhow::Result<i32> {
    let frames = match &args.replay {
        None => synthetic_frames(),
        Some(p) => dumpfile::load_frames_or_raw(p)?
            .into_iter()
            .filter_map(|r| match r {
                Record::Frame { data, .. } => Some(Bytes::from(data)),
                _ => None,
            })
            .collect(),
    };
    let hz = args.hz.max(1);
    let cfg = MockConfig::new(frames, Duration::from_secs_f64(1.0 / hz as f64), !args.once);
    let log = cfg.action_log.clone();
    let listener = tokio::net::TcpListener::bind(&args.listen).await?;
    eprintln!("[mock] listening on {} ({} frames, {hz} Hz)", listener.local_addr()?, cfg.frames.len());
    let printer = tokio::spawn(async move {
        let mut printed = 0usize;
        loop {
            tokio::time::sleep(Duration::from_millis(200)).await;
            let log = log.lock().unwrap();
            for (name, hold) in &log[printed..] {
                eprintln!("[mock] TriggerGameAction({name}, {hold:?})");
            }
            printed = log.len();
        }
    });
    let deadline = async {
        match args.duration {
            Some(s) => tokio::time::sleep(Duration::from_secs(s)).await,
            None => std::future::pending::<()>().await,
        }
    };
    tokio::select! {
        r = serve(listener, cfg) => { r?; }
        _ = deadline => {}
        _ = tokio::signal::ctrl_c() => {}
    }
    printer.abort();
    Ok(0)
}

//! Command line interface. Subcommands are implemented in their own modules.
use clap::{Args, Parser, Subcommand, ValueEnum};
use crate::sc::schema::Scale;
use std::path::PathBuf;

pub const DEFAULT_GRPC: &str = "http://127.0.0.1:2942";

#[derive(Parser, Debug)]
#[command(name = "hud-sender", version, about = "Star Citizen telemetry to ESP32 HUD sender and verification tools")]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Command,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SourceArg {
    Sc,
    Random,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Send telemetry to the HUD (PORT "-" prints frames to stdout)
    Run(RunArgs),
    /// Connect to the game's gRPC server, dump what it offers, capture the first frame
    Probe(ProbeArgs),
    /// Record the telemetry stream to an SCTD file (stdin lines become markers)
    Dump(DumpArgs),
    /// Summarise a recording, or print the tree of one frame
    Inspect(InspectArgs),
    /// Diff two frames of a recording
    Diff(DiffArgs),
    /// Run the snapshot decoder over a recording
    Decode(DecodeArgs),
    /// Run an in-process fake GameTelemetryService
    MockServer(MockArgs),
    /// List or trigger (allowlisted) game actions
    Actions {
        #[command(subcommand)]
        cmd: ActionsCmd,
    },
}

#[derive(Args, Debug)]
pub struct RunArgs {
    /// Serial port (e.g. COM6, /dev/ttyUSB0) or "-" for stdout
    pub port: String,
    #[arg(long, value_enum, default_value_t = SourceArg::Sc)]
    pub source: SourceArg,
    #[arg(long, default_value = DEFAULT_GRPC)]
    pub grpc: String,
    #[arg(long, default_value_t = 115200)]
    pub baud: u32,
    /// Random source: target change interval in seconds
    #[arg(long, default_value_t = 5)]
    pub interval: u64,
    #[arg(long, default_value_t = 30)]
    pub max_hz: u32,
    #[arg(long, default_value_t = 500)]
    pub keepalive_ms: u64,
    /// Treat a sub-frame older than this as absent (0 disables)
    #[arg(long, default_value_t = 3000)]
    pub frame_stale_ms: u64,
    /// Reconnect if no message arrives for this long (0 disables)
    #[arg(long, default_value_t = 0)]
    pub idle_timeout_secs: u64,
    /// Units of shield values (ASSUMPTION: see schema/fields.md)
    #[arg(long, value_enum, default_value_t = Scale::Unit)]
    pub shield_scale: Scale,
    /// Units of fuel values (ASSUMPTION: see schema/fields.md)
    #[arg(long, value_enum, default_value_t = Scale::Unit)]
    pub fuel_scale: Scale,
    /// Allow the HUD to trigger allowlisted game actions
    #[arg(long)]
    pub allow_actions: bool,
    /// Stop after this many seconds (0 = forever)
    #[arg(long, default_value_t = 0)]
    pub duration: u64,
    #[arg(long)]
    pub verbose: bool,
}

#[derive(Args, Debug)]
pub struct ProbeArgs {
    #[arg(long, default_value = DEFAULT_GRPC)]
    pub grpc: String,
    /// Output directory (default schema/probe-<unix_secs>/)
    #[arg(long)]
    pub out: Option<PathBuf>,
    #[arg(long, default_value_t = 5)]
    pub timeout_secs: u64,
}

#[derive(Args, Debug)]
pub struct DumpArgs {
    #[arg(long)]
    pub out: PathBuf,
    #[arg(long, default_value = DEFAULT_GRPC)]
    pub grpc: String,
    #[arg(long)]
    pub duration: Option<u64>,
    #[arg(long)]
    pub max_frames: Option<u64>,
}

#[derive(Args, Debug)]
pub struct InspectArgs {
    pub file: PathBuf,
    /// Print the tree of frame N (0-based over frame records)
    #[arg(long)]
    pub frame: Option<usize>,
    /// With --frame: also print the raw hex
    #[arg(long)]
    pub hex: bool,
}

#[derive(Args, Debug)]
pub struct DiffArgs {
    pub file: PathBuf,
    pub a: usize,
    pub b: usize,
    /// Skip paths whose display starts with this prefix (repeatable)
    #[arg(long = "ignore")]
    pub ignore: Vec<String>,
}

#[derive(Args, Debug)]
pub struct DecodeArgs {
    pub file: PathBuf,
    #[arg(long, default_value_t = 1)]
    pub every: usize,
}

#[derive(Args, Debug)]
pub struct MockArgs {
    #[arg(long, default_value = "127.0.0.1:50051")]
    pub listen: String,
    #[arg(long)]
    pub replay: Option<PathBuf>,
    #[arg(long, default_value_t = 10)]
    pub hz: u32,
    /// End the stream after one pass instead of looping
    #[arg(long)]
    pub once: bool,
    #[arg(long)]
    pub duration: Option<u64>,
}

#[derive(Subcommand, Debug)]
pub enum ActionsCmd {
    /// Ask the game which actions it offers (read-only)
    List {
        #[arg(long, default_value = DEFAULT_GRPC)]
        grpc: String,
    },
    /// Trigger one of the built-in allowlisted actions
    Trigger {
        name: String,
        #[arg(long)]
        hold_ms: Option<u32>,
        #[arg(long, default_value = DEFAULT_GRPC)]
        grpc: String,
        /// Actually send it (otherwise only prints what would happen)
        #[arg(long)]
        yes: bool,
    },
}

/// Runs the chosen subcommand and returns the process exit code.
pub async fn dispatch(cli: Cli) -> anyhow::Result<i32> {
    match cli.cmd {
        Command::Run(a) => crate::runner::run(a).await,
        Command::Probe(a) => crate::sc::probe::run(a).await,
        Command::Dump(a) => crate::sc::dump::run(a).await,
        Command::Inspect(a) => crate::commands::inspect(a),
        Command::Diff(a) => crate::commands::diff(a),
        Command::Decode(a) => crate::commands::decode(a),
        Command::MockServer(a) => crate::sc::mock::run_cli(a).await,
        Command::Actions { cmd } => crate::commands::actions(cmd).await,
    }
}

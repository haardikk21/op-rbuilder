//! Additional Node command arguments.
//!
//! Copied from OptimismNode to allow easy extension.

//! clap [Args](clap::Args) for optimism rollup configuration

use crate::tx_signer::Signer;
use anyhow::{anyhow, Result};
use clap::Parser;
use reth_optimism_cli::commands::Commands;
use reth_optimism_node::args::RollupArgs;
use std::path::PathBuf;

/// Parameters for rollup configuration
#[derive(Debug, Clone, PartialEq, Eq, clap::Args)]
#[command(next_help_heading = "Rollup")]
pub struct OpRbuilderArgs {
    /// Rollup configuration
    #[command(flatten)]
    pub rollup_args: RollupArgs,
    /// Builder secret key for signing last transaction in block
    #[arg(long = "rollup.builder-secret-key", env = "BUILDER_SECRET_KEY")]
    pub builder_signer: Option<Signer>,

    /// chain block time in milliseconds
    #[arg(
        long = "rollup.chain-block-time",
        default_value = "1000",
        env = "CHAIN_BLOCK_TIME"
    )]
    pub chain_block_time: u64,

    /// Signals whether to log pool transaction events
    #[arg(long = "builder.log-pool-transactions", default_value = "false")]
    pub log_pool_transactions: bool,

    /// How much time extra to wait for the block building job to complete and not get garbage collected
    #[arg(long = "builder.extra-block-deadline-secs", default_value = "20")]
    pub extra_block_deadline_secs: u64,
    /// Whether to enable revert protection by default
    #[arg(long = "builder.enable-revert-protection", default_value = "false")]
    pub enable_revert_protection: bool,

    /// Path to builder playgorund to automatically start up the node connected to it
    #[arg(
        long = "builder.playground",
        num_args = 0..=1,
        default_missing_value = "$HOME/.playground/devnet/",
        value_parser = expand_path,
        env = "PLAYGROUND_DIR",
    )]
    pub playground: Option<PathBuf>,
    #[command(flatten)]
    pub flashblocks: FlashblocksArgs,
    #[command(flatten)]
    pub telemetry: TelemetryArgs,
}

impl Default for OpRbuilderArgs {
    fn default() -> Self {
        let args = crate::args::Cli::parse_from(["dummy", "node"]);
        let Commands::Node(node_command) = args.command else {
            unreachable!()
        };
        node_command.ext
    }
}

fn expand_path(s: &str) -> Result<PathBuf> {
    shellexpand::full(s)
        .map_err(|e| anyhow!("expansion error for `{s}`: {e}"))?
        .into_owned()
        .parse()
        .map_err(|e| anyhow!("invalid path after expansion: {e}"))
}

/// Parameters for Flashblocks configuration
/// The names in the struct are prefixed with `flashblocks` to avoid conflicts
/// with the standard block building configuration since these args are flattened
/// into the main `OpRbuilderArgs` struct with the other rollup/node args.
#[derive(Debug, Clone, PartialEq, Eq, clap::Args)]
pub struct FlashblocksArgs {
    /// When set to true, the builder will build flashblocks
    /// and will build standard blocks at the chain block time.
    ///
    /// The default value will change in the future once the flashblocks
    /// feature is stable.
    #[arg(
        long = "flashblocks.enabled",
        default_value = "false",
        env = "ENABLE_FLASHBLOCKS"
    )]
    pub enabled: bool,

    /// The port that we bind to for the websocket server that provides flashblocks
    #[arg(
        long = "flashblocks.port",
        env = "FLASHBLOCKS_WS_PORT",
        default_value = "1111"
    )]
    pub flashblocks_port: u16,

    /// The address that we bind to for the websocket server that provides flashblocks
    #[arg(
        long = "flashblocks.addr",
        env = "FLASHBLOCKS_WS_ADDR",
        default_value = "127.0.0.1"
    )]
    pub flashblocks_addr: String,

    /// Number of Flashblocks per block
    #[arg(
        long = "flashblocks.per-block",
        default_value = "10",
        env = "FLASHBLOCKS_PER_BLOCK"
    )]
    pub flashblocks_per_block: u64,

    /// Overhead for reset of block production (e.g. new payload request to sequencer EL)
    #[arg(
        long = "flashblocks.block-overhead",
        default_value = "100",
        env = "FLASHBLOCK_BLOCK_OVERHEAD"
    )]
    pub flashblocks_block_overhead: u64,
}

impl Default for FlashblocksArgs {
    fn default() -> Self {
        let args = crate::args::Cli::parse_from(["dummy", "node"]);
        let Commands::Node(node_command) = args.command else {
            unreachable!()
        };
        node_command.ext.flashblocks
    }
}

/// Parameters for telemetry configuration
#[derive(Debug, Clone, Default, PartialEq, Eq, clap::Args)]
pub struct TelemetryArgs {
    /// OpenTelemetry endpoint for traces
    #[arg(long = "telemetry.otlp-endpoint", env = "OTEL_EXPORTER_OTLP_ENDPOINT")]
    pub otlp_endpoint: Option<String>,

    /// OpenTelemetry headers for authentication
    #[arg(long = "telemetry.otlp-headers", env = "OTEL_EXPORTER_OTLP_HEADERS")]
    pub otlp_headers: Option<String>,
}

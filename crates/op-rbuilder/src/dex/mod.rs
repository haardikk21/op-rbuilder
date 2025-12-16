pub mod handler;
/// Enshrined DEX integration for op-rbuilder
///
/// This module provides the integration between the enshrined-dex library
/// and the op-rbuilder Flashblocks builder. It intercepts transactions to
/// the predeploy address and executes them using the in-memory DEX.
pub mod predeploy;
pub mod types;

pub use handler::DexHandler;
pub use predeploy::DEX_PREDEPLOY_ADDRESS;
pub use types::*;

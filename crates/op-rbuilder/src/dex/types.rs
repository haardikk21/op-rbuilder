/// Type conversions between op-rbuilder types and enshrined-dex types
use alloy_primitives::{Address, B256, U256};

/// Convert between alloy U256 and enshrined-dex Amount
pub fn to_dex_amount(amount: U256) -> U256 {
    amount
}

/// Convert between alloy Address and enshrined-dex TokenId
pub fn to_token_id(addr: Address) -> Address {
    addr
}

/// Result of a DEX operation
#[derive(Debug, Clone)]
pub enum DexResult {
    /// Pair created successfully
    PairCreated {
        token0: Address,
        token1: Address,
        pair_id: B256,
    },
    /// Limit order placed successfully
    OrderPlaced {
        order_id: B256,
        trader: Address,
        token_in: Address,
        token_out: Address,
        amount: U256,
    },
    /// Order cancelled successfully
    OrderCancelled { order_id: B256 },
    /// Swap executed successfully
    SwapExecuted {
        trader: Address,
        token_in: Address,
        token_out: Address,
        amount_in: U256,
        amount_out: U256,
    },
    /// Quote result
    Quote { amount_out: U256, route: Vec<B256> },
}

impl DexResult {
    /// Encode the result as return data for EVM
    pub fn encode(&self) -> Vec<u8> {
        use alloy_sol_types::SolValue;

        match self {
            DexResult::PairCreated { pair_id, .. } => {
                // Return pair_id as bytes32
                pair_id.abi_encode()
            }
            DexResult::OrderPlaced { order_id, .. } => {
                // Return order_id as bytes32
                order_id.abi_encode()
            }
            DexResult::OrderCancelled { .. } => {
                // Return success (empty return data)
                vec![]
            }
            DexResult::SwapExecuted { amount_out, .. } => {
                // Return amountOut as uint256
                amount_out.abi_encode()
            }
            DexResult::Quote { amount_out, route } => {
                // Return (uint256 amountOut, bytes32[] route)
                (*amount_out, route.as_slice()).abi_encode()
            }
        }
    }
}

/// Errors that can occur during DEX operations
#[derive(Debug, thiserror::Error)]
pub enum DexError {
    #[error("Invalid calldata: {0}")]
    InvalidCalldata(String),

    #[error("Pair already exists")]
    PairAlreadyExists,

    #[error("Pair does not exist")]
    PairDoesNotExist,

    #[error("Invalid token address")]
    InvalidTokenAddress,

    #[error("Invalid amount")]
    InvalidAmount,

    #[error("Invalid price")]
    InvalidPrice,

    #[error("Order not found")]
    OrderNotFound,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Slippage exceeded")]
    SlippageExceeded,

    #[error("Insufficient balance")]
    InsufficientBalance,

    #[error("No route found")]
    NoRouteFound,

    #[error("DEX error: {0}")]
    DexLibraryError(String),
}

impl From<dex::PoolError> for DexError {
    fn from(err: dex::PoolError) -> Self {
        match err {
            dex::PoolError::PairAlreadyExists => DexError::PairAlreadyExists,
            dex::PoolError::PairNotFound => DexError::PairDoesNotExist,
            dex::PoolError::NoRouteFound => DexError::NoRouteFound,
            dex::PoolError::SlippageExceeded => DexError::SlippageExceeded,
            dex::PoolError::InsufficientLiquidity => DexError::InsufficientBalance,
            dex::PoolError::InvalidAmount => DexError::InvalidAmount,
            dex::PoolError::InvalidPair => DexError::InvalidTokenAddress,
            dex::PoolError::OrderError(order_err) => match order_err {
                dex::OrderError::OrderNotFound => DexError::OrderNotFound,
                dex::OrderError::InsufficientLiquidity => DexError::InsufficientBalance,
                dex::OrderError::InvalidPrice => DexError::InvalidPrice,
                dex::OrderError::BelowMinimumSize => DexError::InvalidAmount,
            },
        }
    }
}

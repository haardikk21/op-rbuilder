/// Predeploy address for the enshrined DEX
///
/// Following Optimism's convention of using 0x42XX...XX for system contracts
use alloy_primitives::address;

/// The predeploy address for the enshrined DEX
/// 0x4200000000000000000000000000000000000042
pub const DEX_PREDEPLOY_ADDRESS: alloy_primitives::Address =
    address!("4200000000000000000000000000000000000042");

/// Function selectors for DEX operations (keccak256 of function signature)
pub mod selectors {
    use alloy_primitives::FixedBytes;

    /// createPair(address,address)
    pub const CREATE_PAIR: FixedBytes<4> = FixedBytes([0x6f, 0x36, 0xc6, 0x28]);

    /// placeLimitOrder(address,address,bool,uint256,uint256,uint256)
    pub const PLACE_LIMIT_ORDER: FixedBytes<4> = FixedBytes([0x9e, 0x8c, 0xc0, 0x4b]);

    /// cancelOrder(bytes32)
    pub const CANCEL_ORDER: FixedBytes<4> = FixedBytes([0xa0, 0x71, 0x96, 0xf1]);

    /// swap(address,address,uint256,uint256)
    pub const SWAP: FixedBytes<4> = FixedBytes([0x12, 0x8a, 0xcb, 0x08]);

    /// getQuote(address,address,uint256)
    pub const GET_QUOTE: FixedBytes<4> = FixedBytes([0x99, 0x8f, 0x94, 0xf8]);

    /// getOrderbookDepth(address,address,uint256)
    pub const GET_ORDERBOOK_DEPTH: FixedBytes<4> = FixedBytes([0x4d, 0x4f, 0xb0, 0xb0]);

    /// getPairStats(address,address)
    pub const GET_PAIR_STATS: FixedBytes<4> = FixedBytes([0xf7, 0x88, 0x8a, 0xec]);

    /// getUserOrders(address)
    pub const GET_USER_ORDERS: FixedBytes<4> = FixedBytes([0x0b, 0x86, 0x5c, 0x52]);

    /// getOrder(bytes32)
    pub const GET_ORDER: FixedBytes<4> = FixedBytes([0xd0, 0x9e, 0xf2, 0x41]);
}

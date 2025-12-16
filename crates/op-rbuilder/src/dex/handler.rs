/// DEX transaction handler
///
/// This module handles transactions sent to the DEX predeploy address,
/// decoding calldata and executing operations on the enshrined DEX.
use super::{predeploy::selectors, types::*};
use alloy_primitives::{Address, Bytes, B256, U256};
use alloy_sol_types::SolValue;
use dex::{OrderSide, PoolManager, Price};
use eyre::Result;
use parking_lot::RwLock;
use std::sync::Arc;

/// Handler for enshrined DEX operations
pub struct DexHandler {
    /// The underlying pool manager from enshrined-dex
    pool_manager: Arc<RwLock<PoolManager>>,
}

impl std::fmt::Debug for DexHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DexHandler")
            .finish_non_exhaustive()
    }
}

impl DexHandler {
    /// Create a new DexHandler with a fresh PoolManager
    pub fn new() -> Self {
        Self {
            pool_manager: Arc::new(RwLock::new(PoolManager::new())),
        }
    }

    /// Create a DexHandler from an existing PoolManager
    pub fn from_pool_manager(pool_manager: PoolManager) -> Self {
        Self {
            pool_manager: Arc::new(RwLock::new(pool_manager)),
        }
    }

    /// Get a reference to the pool manager (for inspection/queries)
    pub fn pool_manager(&self) -> Arc<RwLock<PoolManager>> {
        Arc::clone(&self.pool_manager)
    }

    /// Handle a transaction to the DEX predeploy
    ///
    /// # Arguments
    /// * `caller` - The address calling the DEX
    /// * `calldata` - The transaction calldata
    /// * `value` - ETH value sent with the transaction
    ///
    /// # Returns
    /// * `Ok(DexResult)` - The result of the operation
    /// * `Err(DexError)` - If the operation failed
    pub fn handle_transaction(
        &self,
        caller: Address,
        calldata: &Bytes,
        value: U256,
    ) -> Result<DexResult, DexError> {
        if calldata.len() < 4 {
            return Err(DexError::InvalidCalldata(
                "calldata too short for function selector".to_string(),
            ));
        }

        let selector = &calldata[0..4];

        match selector {
            s if s == selectors::CREATE_PAIR.as_slice() => {
                self.handle_create_pair(caller, &calldata[4..])
            }
            s if s == selectors::PLACE_LIMIT_ORDER.as_slice() => {
                self.handle_place_limit_order(caller, &calldata[4..], value)
            }
            s if s == selectors::CANCEL_ORDER.as_slice() => {
                self.handle_cancel_order(caller, &calldata[4..])
            }
            s if s == selectors::SWAP.as_slice() => self.handle_swap(caller, &calldata[4..], value),
            s if s == selectors::GET_QUOTE.as_slice() => self.handle_get_quote(&calldata[4..]),
            _ => Err(DexError::InvalidCalldata(format!(
                "unknown function selector: 0x{}",
                hex::encode(selector)
            ))),
        }
    }

    /// Handle createPair(address,address)
    fn handle_create_pair(&self, _caller: Address, data: &[u8]) -> Result<DexResult, DexError> {
        let (token0, token1): (Address, Address) = <(Address, Address)>::abi_decode(data)
            .map_err(|e| {
                DexError::InvalidCalldata(format!("failed to decode createPair: {}", e))
            })?;

        let mut pm = self.pool_manager.write();
        let pair = pm.create_pair(token0, token1).map_err(DexError::from)?;

        // Convert PairId to B256 for the event
        let pair_id = pair.id();
        let pair_id_bytes = B256::from_slice(&pair_id.0);

        Ok(DexResult::PairCreated {
            token0,
            token1,
            pair_id: pair_id_bytes,
        })
    }

    /// Handle placeLimitOrder(address,address,bool,uint256,uint256,uint256)
    fn handle_place_limit_order(
        &self,
        caller: Address,
        data: &[u8],
        _value: U256,
    ) -> Result<DexResult, DexError> {
        let (token_in, token_out, is_buy, amount, price_num, price_denom): (
            Address,
            Address,
            bool,
            U256,
            U256,
            U256,
        ) = <(Address, Address, bool, U256, U256, U256)>::abi_decode(data).map_err(|e| {
            DexError::InvalidCalldata(format!("failed to decode placeLimitOrder: {}", e))
        })?;

        if amount == U256::ZERO {
            return Err(DexError::InvalidAmount);
        }

        if price_num == U256::ZERO || price_denom == U256::ZERO {
            return Err(DexError::InvalidPrice);
        }

        // Convert to u128 (enshrined-dex uses u128 for prices)
        let price_num_u128: u128 = price_num.try_into().map_err(|_| DexError::InvalidPrice)?;
        let price_denom_u128: u128 = price_denom.try_into().map_err(|_| DexError::InvalidPrice)?;

        let price = Price::from_u128(price_num_u128, price_denom_u128);
        let side = if is_buy {
            OrderSide::Buy
        } else {
            OrderSide::Sell
        };

        let mut pm = self.pool_manager.write();
        let (order_id, _trade_result) = pm
            .place_limit_order(token_in, token_out, caller, side, price, amount)
            .map_err(DexError::from)?;

        // Convert OrderId (u64) to B256
        let mut bytes = [0u8; 32];
        bytes[24..32].copy_from_slice(&order_id.0.to_be_bytes());
        let order_id_bytes = B256::from(bytes);

        Ok(DexResult::OrderPlaced {
            order_id: order_id_bytes,
            trader: caller,
            token_in,
            token_out,
            amount,
        })
    }

    /// Handle cancelOrder(bytes32)
    fn handle_cancel_order(&self, _caller: Address, data: &[u8]) -> Result<DexResult, DexError> {
        let order_id: B256 = <B256>::abi_decode(data).map_err(|e| {
            DexError::InvalidCalldata(format!("failed to decode cancelOrder: {}", e))
        })?;

        // For now, we'll just return success
        // TODO: Implement proper order cancellation with order tracking
        // The enshrined-dex library needs to be updated to support order cancellation by ID

        Ok(DexResult::OrderCancelled { order_id })
    }

    /// Handle swap(address,address,uint256,uint256)
    fn handle_swap(
        &self,
        caller: Address,
        data: &[u8],
        _value: U256,
    ) -> Result<DexResult, DexError> {
        let (token_in, token_out, amount_in, min_amount_out): (Address, Address, U256, U256) =
            <(Address, Address, U256, U256)>::abi_decode(data)
                .map_err(|e| DexError::InvalidCalldata(format!("failed to decode swap: {}", e)))?;

        if amount_in == U256::ZERO {
            return Err(DexError::InvalidAmount);
        }

        let mut pm = self.pool_manager.write();
        let result = pm
            .execute_swap(caller, token_in, token_out, amount_in, min_amount_out)
            .map_err(DexError::from)?;

        Ok(DexResult::SwapExecuted {
            trader: caller,
            token_in,
            token_out,
            amount_in,
            amount_out: result.amount_out,
        })
    }

    /// Handle getQuote(address,address,uint256)
    fn handle_get_quote(&self, data: &[u8]) -> Result<DexResult, DexError> {
        let (token_in, token_out, amount_in): (Address, Address, U256) =
            <(Address, Address, U256)>::abi_decode(data).map_err(|e| {
                DexError::InvalidCalldata(format!("failed to decode getQuote: {}", e))
            })?;

        let pm = self.pool_manager.read();
        let result = pm
            .get_quote(token_in, token_out, amount_in)
            .map_err(DexError::from)?;

        // Convert route to B256 array (extract pair IDs from hops)
        let route: Vec<B256> = result
            .route
            .hops
            .iter()
            .map(|hop| {
                let pair_id = hop.pair.id();
                B256::from_slice(&pair_id.0)
            })
            .collect();

        Ok(DexResult::Quote {
            amount_out: result.amount_out,
            route,
        })
    }
}

impl Default for DexHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for DexHandler {
    fn clone(&self) -> Self {
        Self {
            pool_manager: Arc::clone(&self.pool_manager),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::address;

    #[test]
    fn test_create_pair() {
        let handler = DexHandler::new();

        let token0 = address!("0000000000000000000000000000000000000000");
        let token1 = address!("0000000000000000000000000000000000000001");

        let calldata = [
            selectors::CREATE_PAIR.as_slice(),
            &(token0, token1).abi_encode(),
        ]
        .concat();

        let caller = address!("0000000000000000000000000000000000000042");
        let calldata_bytes: Bytes = calldata.into();
        let result = handler
            .handle_transaction(caller, &calldata_bytes, U256::ZERO)
            .expect("should create pair");

        match result {
            DexResult::PairCreated { token0: t0, token1: t1, .. } => {
                assert_eq!(t0, token0);
                assert_eq!(t1, token1);
            }
            _ => panic!("unexpected result"),
        }
    }

    #[test]
    fn test_complete_dex_flow() {
        // This test demonstrates a complete DEX flow:
        // 1. Create pair
        // 2. Place limit order
        // 3. Get quote
        // 4. Execute swap

        let caller = address!("0000000000000000000000000000000000000042");
        let trader = address!("0000000000000000000000000000000000000099");
        let handler = DexHandler::new();

        let eth = address!("0000000000000000000000000000000000000000");
        let usdc = address!("0000000000000000000000000000000000000001");

        // 1. Create pair
        let create_pair_calldata = [
            selectors::CREATE_PAIR.as_slice(),
            &(eth, usdc).abi_encode(),
        ]
        .concat();

        let result = handler
            .handle_transaction(caller, &create_pair_calldata.into(), U256::ZERO)
            .expect("createPair should succeed");

        assert!(matches!(result, DexResult::PairCreated { .. }));

        // 2. Place limit order: sell 1 ETH at 2000 USDC
        let place_order_calldata = [
            selectors::PLACE_LIMIT_ORDER.as_slice(),
            &(
                eth,
                usdc,
                false, // isBuy = false (sell)
                U256::from(10u64.pow(18)), // 1 ETH
                U256::from(2000),          // price numerator
                U256::from(1),             // price denominator
            )
                .abi_encode(),
        ]
        .concat();

        let result = handler
            .handle_transaction(trader, &place_order_calldata.into(), U256::ZERO)
            .expect("placeLimitOrder should succeed");

        match result {
            DexResult::OrderPlaced { order_id, amount, .. } => {
                assert_eq!(amount, U256::from(10u64.pow(18)));
                assert_ne!(order_id, B256::ZERO);
            }
            _ => panic!("expected OrderPlaced"),
        }

        // 3. Get quote: how much ETH for 100 USDC?
        let quote_calldata = [
            selectors::GET_QUOTE.as_slice(),
            &(usdc, eth, U256::from(100 * 10u64.pow(6))).abi_encode(),
        ]
        .concat();

        let result = handler
            .handle_transaction(trader, &quote_calldata.into(), U256::ZERO)
            .expect("getQuote should succeed");

        match result {
            DexResult::Quote { amount_out, route } => {
                assert!(amount_out > U256::ZERO, "should get ETH for USDC");
                assert!(!route.is_empty(), "route should exist");
            }
            _ => panic!("expected Quote"),
        }

        // 4. Execute swap: trade 100 USDC for ETH
        let swap_calldata = [
            selectors::SWAP.as_slice(),
            &(
                usdc,
                eth,
                U256::from(100 * 10u64.pow(6)), // 100 USDC
                U256::from(0),                   // no slippage protection for test
            )
                .abi_encode(),
        ]
        .concat();

        let result = handler
            .handle_transaction(trader, &swap_calldata.into(), U256::ZERO)
            .expect("swap should succeed");

        match result {
            DexResult::SwapExecuted { amount_in, amount_out, .. } => {
                assert_eq!(amount_in, U256::from(100 * 10u64.pow(6)));
                assert!(amount_out > U256::ZERO, "should receive ETH");
            }
            _ => panic!("expected SwapExecuted"),
        }
    }

}

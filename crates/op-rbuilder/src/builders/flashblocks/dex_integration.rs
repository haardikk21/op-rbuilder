/// DEX integration for Flashblocks builder
///
/// This module provides the integration between the Flashblocks builder and the enshrined DEX.
/// It intercepts transactions sent to the DEX predeploy address and executes them using the
/// in-memory DEX library.
use crate::{
    dex::{DexHandler, DexResult, DEX_PREDEPLOY_ADDRESS},
    primitives::reth::ExecutionInfo,
};
use alloy_consensus::Transaction as _;
use alloy_primitives::{Address, Bytes, Log, U256};
use eyre::Result;
use op_alloy_consensus::OpTxEnvelope;
use reth_optimism_primitives::OpReceipt;
use tracing::{debug, warn};

/// Check if a transaction is targeting the DEX predeploy
pub(super) fn is_dex_transaction(tx: &OpTxEnvelope) -> bool {
    match tx.to() {
        Some(addr) => addr == DEX_PREDEPLOY_ADDRESS,
        None => false,
    }
}

/// Execute a DEX transaction
///
/// # Arguments
/// * `dex_handler` - The DEX handler instance
/// * `tx` - The transaction to execute
/// * `sender` - The recovered sender address
/// * `info` - Execution info to update with results
///
/// # Returns
/// * `Ok(())` if the transaction was executed successfully
/// * `Err(e)` if the transaction failed
pub(crate) fn execute_dex_transaction<Extra: std::fmt::Debug + Default>(
    dex_handler: &DexHandler,
    tx: &OpTxEnvelope,
    sender: Address,
    info: &mut ExecutionInfo<Extra>,
) -> Result<()> {
    debug!(
        target: "dex",
        tx_hash = ?tx.tx_hash(),
        sender = ?sender,
        "Executing DEX transaction"
    );

    let calldata: Bytes = tx.input().clone();
    let value: U256 = tx.value();

    // Execute the DEX operation
    let result = match dex_handler.handle_transaction(sender, &calldata, value) {
        Ok(result) => result,
        Err(e) => {
            warn!(
                target: "dex",
                error = ?e,
                tx_hash = ?tx.tx_hash(),
                "DEX transaction failed"
            );
            // Create a failed receipt
            let receipt = create_failed_dex_receipt(info.cumulative_gas_used, 21000);
            info.receipts.push(receipt);
            info.cumulative_gas_used += 21000; // Charge base gas for failed tx
            info.executed_senders.push(sender);
            info.executed_transactions.push(tx.clone());
            return Ok(()); // Don't propagate error - just mark as reverted
        }
    };

    // Calculate gas used for DEX operation
    // DEX operations use a fixed amount of gas based on the operation type
    let gas_used = estimate_dex_gas(&result);
    info.cumulative_gas_used += gas_used;

    // Create logs for the DEX operation
    let logs = create_dex_logs(&result, tx.tx_hash());

    // Create receipt
    let receipt = create_dex_receipt(info.cumulative_gas_used, gas_used, logs, &result);
    info.receipts.push(receipt);

    // Track transaction execution
    info.executed_senders.push(sender);
    info.executed_transactions.push(tx.clone());

    debug!(
        target: "dex",
        tx_hash = ?tx.tx_hash(),
        gas_used,
        "DEX transaction executed successfully"
    );

    Ok(())
}

/// Estimate gas used for a DEX operation
fn estimate_dex_gas(result: &DexResult) -> u64 {
    match result {
        DexResult::PairCreated { .. } => 100_000,
        DexResult::OrderPlaced { .. } => 150_000,
        DexResult::OrderCancelled { .. } => 50_000,
        DexResult::SwapExecuted { .. } => 200_000,
        DexResult::Quote { .. } => 0, // View function, no gas
    }
}

/// Create logs for a DEX operation
fn create_dex_logs(result: &DexResult, _tx_hash: alloy_primitives::B256) -> Vec<Log> {
    // For now, we'll create simple logs
    // In production, you'd want to emit proper events with indexed topics
    let mut logs = Vec::new();

    match result {
        DexResult::PairCreated {
            token0,
            token1,
            pair_id,
        } => {
            // PairCreated(address indexed token0, address indexed token1, bytes32 indexed pairId)
            logs.push(Log {
                address: DEX_PREDEPLOY_ADDRESS,
                data: alloy_primitives::LogData::new_unchecked(
                    vec![
                        alloy_primitives::keccak256("PairCreated(address,address,bytes32)"),
                        alloy_primitives::B256::left_padding_from(token0.as_slice()),
                        alloy_primitives::B256::left_padding_from(token1.as_slice()),
                        *pair_id,
                    ],
                    Bytes::new(),
                ),
            });
        }
        DexResult::OrderPlaced {
            order_id,
            trader,
            token_in,
            token_out: _,
            amount,
        } => {
            // OrderPlaced event
            logs.push(Log {
                address: DEX_PREDEPLOY_ADDRESS,
                data: alloy_primitives::LogData::new_unchecked(
                    vec![
                        alloy_primitives::keccak256("LimitOrderPlaced(bytes32,address,address,address,bool,uint256,uint256,uint256)"),
                        *order_id,
                        alloy_primitives::B256::left_padding_from(trader.as_slice()),
                        alloy_primitives::B256::left_padding_from(token_in.as_slice()),
                    ],
                    amount.abi_encode().into(),
                ),
            });
        }
        DexResult::OrderCancelled { order_id } => {
            logs.push(Log {
                address: DEX_PREDEPLOY_ADDRESS,
                data: alloy_primitives::LogData::new_unchecked(
                    vec![
                        alloy_primitives::keccak256("OrderCancelled(bytes32,address)"),
                        *order_id,
                    ],
                    Bytes::new(),
                ),
            });
        }
        DexResult::SwapExecuted {
            trader,
            token_in,
            token_out,
            amount_in,
            amount_out,
        } => {
            // Swap event
            let data = (*amount_in, *amount_out).abi_encode();
            logs.push(Log {
                address: DEX_PREDEPLOY_ADDRESS,
                data: alloy_primitives::LogData::new_unchecked(
                    vec![
                        alloy_primitives::keccak256(
                            "Swap(address,address,address,uint256,uint256,bytes32[])",
                        ),
                        alloy_primitives::B256::left_padding_from(trader.as_slice()),
                        alloy_primitives::B256::left_padding_from(token_in.as_slice()),
                        alloy_primitives::B256::left_padding_from(token_out.as_slice()),
                    ],
                    data.into(),
                ),
            });
        }
        DexResult::Quote { .. } => {
            // View function, no logs
        }
    }

    logs
}

/// Create a receipt for a successful DEX operation
fn create_dex_receipt(
    cumulative_gas_used: u64,
    _gas_used: u64,
    logs: Vec<Log>,
    _result: &DexResult,
) -> OpReceipt {
    use alloy_consensus::{Eip658Value, Receipt};

    OpReceipt::Eip1559(Receipt {
        status: Eip658Value::Eip658(true),
        cumulative_gas_used,
        logs,
    })
}

/// Create a receipt for a failed DEX operation
fn create_failed_dex_receipt(cumulative_gas_used: u64, gas_used: u64) -> OpReceipt {
    use alloy_consensus::{Eip658Value, Receipt};

    OpReceipt::Eip1559(Receipt {
        status: Eip658Value::Eip658(false),
        cumulative_gas_used: cumulative_gas_used + gas_used,
        logs: vec![],
    })
}

use alloy_sol_types::SolValue;

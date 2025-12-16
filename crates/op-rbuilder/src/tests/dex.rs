use crate::{
    dex::{predeploy::selectors, DEX_PREDEPLOY_ADDRESS},
    tests::LocalInstance,
};
use alloy_network::ReceiptResponse;
use alloy_primitives::{address, Address, Bytes, U256};
use alloy_provider::Provider;
use alloy_sol_types::SolValue;
use macros::rb_test;
use tracing::info;

/// Integration test: Create a pair, add liquidity via limit orders, and execute a swap
#[rb_test(flashblocks)]
async fn dex_create_pair_add_liquidity_and_swap(rbuilder: LocalInstance) -> eyre::Result<()> {
    let driver = rbuilder.driver().await?;

    // Define tokens
    let eth = address!("0000000000000000000000000000000000000000");
    let usdc = address!("0000000000000000000000000000000000000001");

    info!("=== DEX Integration Test ===");
    info!("ETH: {:?}", eth);
    info!("USDC: {:?}", usdc);

    // ============================================================================
    // Step 1: Create a trading pair (ETH/USDC)
    // ============================================================================
    info!("\n[1/4] Creating ETH/USDC trading pair...");

    let create_pair_calldata = encode_create_pair(eth, usdc);

    let create_pair_tx = driver
        .create_transaction()
        .with_to(DEX_PREDEPLOY_ADDRESS)
        .with_input(create_pair_calldata)
        .send()
        .await?;

    info!("CreatePair tx sent: {:?}", create_pair_tx.tx_hash());

    // Build a block to include the transaction
    let block = driver.build_new_block_with_current_timestamp(None).await?;
    info!("Block built with {} transactions", block.transactions.len());

    // Verify the transaction was included and succeeded
    let provider = driver.provider();
    let create_pair_receipt = provider
        .get_transaction_receipt(*create_pair_tx.tx_hash())
        .await?
        .expect("CreatePair receipt should exist");

    assert!(
        create_pair_receipt.status(),
        "CreatePair transaction should succeed"
    );
    info!("✓ Pair created successfully!");

    // ============================================================================
    // Step 2: Add liquidity by placing limit orders
    // ============================================================================
    info!("\n[2/4] Adding liquidity via limit orders...");

    // Place a SELL order: Sell 1 ETH for 2000 USDC (price = 2000 USDC per ETH)
    let sell_order_calldata = encode_place_limit_order(
        eth,
        usdc,
        false,                         // isBuy = false (sell order)
        U256::from(10u64.pow(18)),     // 1 ETH
        U256::from(2000),              // price numerator
        U256::from(1),                 // price denominator
    );

    let sell_order_tx = driver
        .create_transaction()
        .with_to(DEX_PREDEPLOY_ADDRESS)
        .with_input(sell_order_calldata)
        .send()
        .await?;

    info!("Sell limit order tx sent: {:?}", sell_order_tx.tx_hash());

    // Build block
    let block = driver.build_new_block_with_current_timestamp(None).await?;
    info!("Block built with {} transactions", block.transactions.len());

    let sell_order_receipt = provider
        .get_transaction_receipt(*sell_order_tx.tx_hash())
        .await?
        .expect("Sell order receipt should exist");

    assert!(
        sell_order_receipt.status(),
        "Sell limit order should succeed"
    );
    info!("✓ Sell limit order placed (1 ETH @ 2000 USDC)");

    // Place another SELL order for more liquidity
    let sell_order_2_calldata = encode_place_limit_order(
        eth,
        usdc,
        false,
        U256::from(5 * 10u64.pow(17)),  // 0.5 ETH
        U256::from(2100),               // slightly higher price
        U256::from(1),
    );

    let sell_order_2_tx = driver
        .create_transaction()
        .with_to(DEX_PREDEPLOY_ADDRESS)
        .with_input(sell_order_2_calldata)
        .send()
        .await?;

    let block = driver.build_new_block_with_current_timestamp(None).await?;
    info!("Block built with {} transactions", block.transactions.len());

    let sell_order_2_receipt = provider
        .get_transaction_receipt(*sell_order_2_tx.tx_hash())
        .await?
        .expect("Sell order 2 receipt should exist");

    assert!(
        sell_order_2_receipt.status(),
        "Second sell limit order should succeed"
    );
    info!("✓ Second sell limit order placed (0.5 ETH @ 2100 USDC)");

    // ============================================================================
    // Step 3: Get a quote before swapping
    // ============================================================================
    info!("\n[3/4] Getting quote for swap...");

    let quote_calldata = encode_get_quote(
        usdc,
        eth,
        U256::from(100 * 10u64.pow(6)), // 100 USDC
    );

    let quote_tx = driver
        .create_transaction()
        .with_to(DEX_PREDEPLOY_ADDRESS)
        .with_input(quote_calldata)
        .send()
        .await?;

    let block = driver.build_new_block_with_current_timestamp(None).await?;
    info!("Block built with {} transactions", block.transactions.len());

    let quote_receipt = provider
        .get_transaction_receipt(*quote_tx.tx_hash())
        .await?
        .expect("Quote receipt should exist");

    assert!(quote_receipt.status(), "GetQuote should succeed");
    info!("✓ Quote obtained successfully");

    // ============================================================================
    // Step 4: Execute a swap (buy ETH with USDC)
    // ============================================================================
    info!("\n[4/4] Executing swap: 100 USDC → ETH...");

    let swap_calldata = encode_swap(
        usdc,
        eth,
        U256::from(100 * 10u64.pow(6)), // 100 USDC (assuming 6 decimals)
        U256::from(0),                   // minAmountOut = 0 (no slippage protection for test)
    );

    let swap_tx = driver
        .create_transaction()
        .with_to(DEX_PREDEPLOY_ADDRESS)
        .with_input(swap_calldata)
        .send()
        .await?;

    info!("Swap tx sent: {:?}", swap_tx.tx_hash());

    // Build block
    let block = driver.build_new_block_with_current_timestamp(None).await?;
    info!("Block built with {} transactions", block.transactions.len());

    let swap_receipt = provider
        .get_transaction_receipt(*swap_tx.tx_hash())
        .await?
        .expect("Swap receipt should exist");

    assert!(swap_receipt.status(), "Swap should succeed");
    info!("✓ Swap executed successfully!");

    // Verify events were emitted
    assert!(!swap_receipt.inner.logs().is_empty(), "Swap should emit events");
    info!("Events emitted: {}", swap_receipt.inner.logs().len());

    info!("\n=== Test Completed Successfully ===");
    info!("✓ Created trading pair");
    info!("✓ Placed 2 limit orders for liquidity");
    info!("✓ Got quote");
    info!("✓ Executed swap");

    Ok(())
}

// ============================================================================
// Helper functions to encode calldata
// ============================================================================

fn encode_create_pair(token0: Address, token1: Address) -> Bytes {
    let params = (token0, token1).abi_encode();
    [selectors::CREATE_PAIR.as_slice(), &params]
        .concat()
        .into()
}

fn encode_place_limit_order(
    token_in: Address,
    token_out: Address,
    is_buy: bool,
    amount: U256,
    price_num: U256,
    price_denom: U256,
) -> Bytes {
    let params = (token_in, token_out, is_buy, amount, price_num, price_denom).abi_encode();
    [selectors::PLACE_LIMIT_ORDER.as_slice(), &params]
        .concat()
        .into()
}

fn encode_swap(
    token_in: Address,
    token_out: Address,
    amount_in: U256,
    min_amount_out: U256,
) -> Bytes {
    let params = (token_in, token_out, amount_in, min_amount_out).abi_encode();
    [selectors::SWAP.as_slice(), &params].concat().into()
}

fn encode_get_quote(token_in: Address, token_out: Address, amount_in: U256) -> Bytes {
    let params = (token_in, token_out, amount_in).abi_encode();
    [selectors::GET_QUOTE.as_slice(), &params].concat().into()
}

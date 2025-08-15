use super::*;
use ethers::prelude::*;
use std::sync::Arc;

#[test]
fn test_config_from_env() {
    std::env::set_var("PRIVATE_KEY", "0x1234567890123456789012345678901234567890123456789012345678901234");
    std::env::set_var("MIN_PROFIT_USD", "10.0");
    std::env::set_var("SIMULATION_MODE", "true");

    let config = Config::from_env();
    assert!(config.is_ok());

    let config = config.unwrap();
    assert_eq!(config.min_profit_usd, 10.0);
    assert!(config.simulation_mode);
}

#[test]
fn test_opportunity_validation() {
    let opp = ArbOpportunity {
        timestamp_ns: 1000000000,
        pair: "WMATIC-USDC".to_string(),
        token_a: "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270".parse().unwrap(),
        token_b: "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".parse().unwrap(),
        dex_buy_router: "0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff".parse().unwrap(),
        dex_sell_router: "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506".parse().unwrap(),
        price_a: 0.8,
        price_b: 0.82,
        liquidity_a: 100000.0,
        liquidity_b: 80000.0,
        estimated_profit_usd: 15.0,
        gas_estimate: 300000,
    };

    // Verify opportunity fields
    assert_eq!(opp.pair, "WMATIC-USDC");
    assert_eq!(opp.estimated_profit_usd, 15.0);
    assert!(opp.price_b > opp.price_a);
}

#[tokio::test]
async fn test_dex_manager() {
    let provider = Provider::<Http>::try_from("https://polygon-mainnet.public.blastapi.io").unwrap();
    let wallet = "0x0000000000000000000000000000000000000000000000000000000000000001"
        .parse::<LocalWallet>()
        .unwrap()
        .with_chain_id(137u64);
    let client = Arc::new(SignerMiddleware::new(provider, wallet));

    let dex_manager = DexManager::new(client);

    // Test router lookups
    let quickswap: Address = "0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff".parse().unwrap();
    let router = dex_manager.get_router(&quickswap);
    assert!(router.is_some());
    assert_eq!(router.unwrap().name, "QuickSwap");

    // Test token lookups
    let wmatic: Address = "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270".parse().unwrap();
    let token = dex_manager.get_token(&wmatic);
    assert!(token.is_some());
    assert_eq!(token.unwrap().symbol, "WMATIC");
    assert_eq!(token.unwrap().decimals, 18);
}

#[test]
fn test_profit_calculation() {
    // Test case: Buy at 0.8, Sell at 0.82
    let buy_price = 0.8;
    let sell_price = 0.82;
    let trade_amount_usd = 1000.0;
    
    let tokens_bought = trade_amount_usd / buy_price;  // 1250 tokens
    let revenue = tokens_bought * sell_price;  // 1025 USD
    let gross_profit = revenue - trade_amount_usd;  // 25 USD
    
    assert_eq!(tokens_bought, 1250.0);
    assert_eq!(revenue, 1025.0);
    assert_eq!(gross_profit, 25.0);
    
    // With 0.3% fees on both sides
    let fee_rate = 0.003;
    let buy_fee = trade_amount_usd * fee_rate;  // 3 USD
    let sell_fee = revenue * fee_rate;  // 3.075 USD
    let net_profit = gross_profit - buy_fee - sell_fee;  // 18.925 USD
    
    assert!((net_profit - 18.925_f64).abs() < 0.001);
}

#[test]
fn test_slippage_calculation() {
    let config = Config {
        rpc_url: "".to_string(),
        private_key: "".to_string(),
        chain_id: 137,
        min_profit_usd: 5.0,
        max_gas_price_gwei: 100.0,
        max_opportunity_age_ms: 5000,
        simulation_mode: true,
        max_trade_percentage: 0.5,
        slippage_tolerance: 0.005,  // 0.5%
    };

    let expected_amount = U256::from(1000u64);
    let slippage_factor = 1.0 - config.slippage_tolerance;
    let min_amount = expected_amount * U256::from((slippage_factor * 1000.0) as u64) / U256::from(1000);
    
    // Should be 995 (99.5% of 1000)
    assert_eq!(min_amount, U256::from(995u64));
}

#[test]
fn test_opportunity_age_check() {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

    let fresh_opp = ArbOpportunity {
        timestamp_ns: now - 1_000_000_000,  // 1 second old
        pair: "TEST".to_string(),
        token_a: Address::zero(),
        token_b: Address::zero(),
        dex_buy_router: Address::zero(),
        dex_sell_router: Address::zero(),
        price_a: 1.0,
        price_b: 1.0,
        liquidity_a: 0.0,
        liquidity_b: 0.0,
        estimated_profit_usd: 10.0,
        gas_estimate: 0,
    };

    let stale_opp = ArbOpportunity {
        timestamp_ns: now - 10_000_000_000,  // 10 seconds old
        ..fresh_opp.clone()
    };

    let age_fresh = (now - fresh_opp.timestamp_ns) / 1_000_000;  // Convert to ms
    let age_stale = (now - stale_opp.timestamp_ns) / 1_000_000;

    assert!(age_fresh < 5000);  // Fresh opportunity
    assert!(age_stale > 5000);  // Stale opportunity
}
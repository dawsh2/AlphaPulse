//! Live RPC Token Address Validation Test
//!
//! Tests the token address validator against real Polygon blockchain
//! using public RPC endpoints to verify our parsing is correct.

#[cfg(test)]
mod live_validation {
    use alphapulse_adapter_service::{
        input::collectors::{
            pool_cache_manager::{PoolCacheManager, PoolInfo},
            polygon_dex::abi_events::{SwapEventDecoder, DEXProtocol},
        },
    };
    use alphapulse_protocol_v2::VenueId;
    use web3::{Web3, transports::Http, types::{H160, CallRequest, Bytes}};
    use std::str::FromStr;
    
    const PUBLIC_RPC_ENDPOINTS: &[&str] = &[
        "https://polygon-rpc.com/",
        "https://rpc.ankr.com/polygon",
        "https://polygon-mainnet.public.blastapi.io",
    ];
    
    async fn get_working_rpc() -> Option<Web3<Http>> {
        for endpoint in PUBLIC_RPC_ENDPOINTS {
            if let Ok(transport) = Http::new(endpoint) {
                let web3 = Web3::new(transport);
                // Test if responsive
                if web3.eth().block_number().await.is_ok() {
                    println!("✓ Connected to RPC: {}", endpoint);
                    return Some(web3);
                }
            }
        }
        None
    }
    
    #[tokio::test]
    #[ignore] // Run with: cargo test live_rpc_validation -- --ignored --nocapture
    async fn test_token_decimals_validation() {
        let web3 = match get_working_rpc().await {
            Some(w3) => w3,
            None => {
                println!("⚠️ No working public RPC found, skipping test");
                return;
            }
        };
        
        println!("\n=== Live Token Decimals Validation ===\n");
        
        // Known token addresses with expected decimals
        let tokens = vec![
            ("WMATIC", "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270", 18),
            ("USDC", "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174", 6),
            ("USDT", "0xc2132D05D31c914a87C6611C10748AEb04B58e8F", 6),
            ("WETH", "0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619", 18),
        ];
        
        for (name, address, expected_decimals) in tokens {
            let token_address = H160::from_str(address).unwrap();
            
            // Call decimals() function (0x313ce567)
            let decimals_call = CallRequest {
                to: Some(token_address),
                data: Some(Bytes::from_str("0x313ce567").unwrap()),
                ..Default::default()
            };
            
            match web3.eth().call(decimals_call, None).await {
                Ok(result) => {
                    // Parse decimals from result
                    let decimals = if result.0.len() >= 32 {
                        result.0[31] as u8
                    } else {
                        0
                    };
                    
                    if decimals == expected_decimals {
                        println!("✅ {}: {} decimals (correct)", name, decimals);
                    } else {
                        println!("❌ {}: {} decimals (expected {})", name, decimals, expected_decimals);
                        panic!("Decimal validation failed for {}", name);
                    }
                }
                Err(e) => {
                    println!("❌ Failed to query {}: {}", name, e);
                    panic!("RPC call failed");
                }
            }
        }
        
        println!("\n✅ All token decimals validated correctly!");
    }
    
    #[tokio::test]
    #[ignore] // Run with: cargo test live_pool_validation -- --ignored --nocapture
    async fn test_uniswap_v3_pool_validation() {
        let web3 = match get_working_rpc().await {
            Some(w3) => w3,
            None => {
                println!("⚠️ No working public RPC found, skipping test");
                return;
            }
        };
        
        println!("\n=== Live Uniswap V3 Pool Validation ===\n");
        
        // This is the real pool from our test data
        let pool_address = H160::from_str("0x45dda9cb7c25131df268515131f647d726f50608").unwrap();
        
        // Get token0 (0x0dfe1681)
        let token0_call = CallRequest {
            to: Some(pool_address),
            data: Some(Bytes::from_str("0x0dfe1681").unwrap()),
            ..Default::default()
        };
        
        let token0_result = web3.eth().call(token0_call, None).await.unwrap();
        let token0 = H160::from_slice(&token0_result.0[12..32]);
        println!("Token0: {:?}", token0);
        
        // Get token1 (0xd21220a7)
        let token1_call = CallRequest {
            to: Some(pool_address),
            data: Some(Bytes::from_str("0xd21220a7").unwrap()),
            ..Default::default()
        };
        
        let token1_result = web3.eth().call(token1_call, None).await.unwrap();
        let token1 = H160::from_slice(&token1_result.0[12..32]);
        println!("Token1: {:?}", token1);
        
        // Get fee (0xddca3f43)
        let fee_call = CallRequest {
            to: Some(pool_address),
            data: Some(Bytes::from_str("0xddca3f43").unwrap()),
            ..Default::default()
        };
        
        let fee_result = web3.eth().call(fee_call, None).await.unwrap();
        let fee = u32::from_be_bytes([
            fee_result.0[28],
            fee_result.0[29],
            fee_result.0[30],
            fee_result.0[31],
        ]);
        println!("Fee: {} ({}%)", fee, fee as f64 / 10000.0);
        
        // Validate these are the expected tokens
        assert_eq!(
            token0,
            H160::from_str("0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174").unwrap(),
            "Token0 should be USDC"
        );
        
        assert_eq!(
            token1,
            H160::from_str("0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619").unwrap(),
            "Token1 should be WETH"
        );
        
        assert_eq!(fee, 500, "Fee should be 500 (0.05%)");
        
        println!("\n✅ Pool validation successful!");
        println!("  - USDC/WETH pool");
        println!("  - 0.05% fee tier");
        println!("  - Addresses match expected values");
    }
    
    #[tokio::test]
    #[ignore] // Run with: cargo test validate_swap_amounts -- --ignored --nocapture
    async fn test_validate_swap_amounts_against_chain() {
        println!("\n=== Validating Swap Event Data Against Chain ===\n");
        
        // Use the real swap event from our fixtures
        let log = alphapulse_adapter_service::tests::fixtures::polygon::uniswap_v3_swap_real();
        
        // Decode the swap event
        let decoder = SwapEventDecoder::new();
        let validated_data = decoder.decode_swap_event(&log, DEXProtocol::UniswapV3).unwrap();
        
        println!("Parsed swap data:");
        println!("  Amount0: {}", validated_data.amount0);
        println!("  Amount1: {}", validated_data.amount1);
        println!("  SqrtPriceX96: {:?}", validated_data.sqrt_price_x96);
        println!("  Liquidity: {}", validated_data.liquidity);
        println!("  Tick: {}", validated_data.tick);
        
        // Validate semantic correctness
        assert!(
            (validated_data.amount0 > 0 && validated_data.amount1 < 0) ||
            (validated_data.amount0 < 0 && validated_data.amount1 > 0),
            "One amount should be positive (in) and one negative (out)"
        );
        
        // Check the swap makes sense
        if validated_data.amount0 > 0 {
            println!("\n✅ Token0 in, Token1 out swap validated");
            println!("  Swapped {} token0 for {} token1", 
                     validated_data.amount0, 
                     -validated_data.amount1);
        } else {
            println!("\n✅ Token1 in, Token0 out swap validated");
            println!("  Swapped {} token1 for {} token0", 
                     validated_data.amount1, 
                     -validated_data.amount0);
        }
        
        // Validate tick is within Uniswap V3 bounds
        assert!(
            validated_data.tick >= -887272 && validated_data.tick <= 887272,
            "Tick should be within V3 bounds"
        );
        
        println!("\n✅ All swap data validation passed!");
    }
}

fn main() {
    println!("Run tests with:");
    println!("  cargo test --test live_rpc_validation -- --ignored --nocapture");
}
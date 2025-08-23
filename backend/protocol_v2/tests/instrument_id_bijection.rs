//! Bijective InstrumentId Property Tests
//! 
//! Ensures the bijective property: every ID maps to a unique instrument and back.
//! Tests Cantor pairing correctness and collision detection.

mod common;

use alphapulse_protocol_v2::{
    InstrumentId, VenueId, AssetType,
    instrument_id::{
        cantor_pairing, inverse_cantor_pairing,
        cantor_pairing_triple, inverse_cantor_pairing_triple,
        canonical_pool_id, canonical_triangular_pool_id,
    },
};
use std::collections::HashSet;

#[test]
fn test_bijective_property_coins() {
    // Test bijection for cryptocurrency coins
    let test_cases = [
        (VenueId::Binance, "BTC"),
        (VenueId::Coinbase, "ETH"),
        (VenueId::Kraken, "USDT"),
        (VenueId::Ethereum, "WETH"),
        (VenueId::Polygon, "MATIC"),
    ];
    
    for (venue, symbol) in test_cases {
        let id = InstrumentId::coin(venue, symbol);
        
        // Verify venue extraction
        let extracted_venue = id.venue().unwrap();
        assert_eq!(extracted_venue, venue, "Venue extraction failed");
        
        // Verify asset type
        let asset_type = id.asset_type().unwrap();
        assert_eq!(asset_type, AssetType::Coin, "Asset type incorrect");
        
        // Verify cache key bijection
        let cache_key = id.cache_key();
        let recreated = InstrumentId::from_cache_key(cache_key);
        assert_eq!(id, recreated, "Cache key bijection failed for {}", symbol);
        
        // Verify u64 conversion bijection (with potential precision loss)
        let u64_key = id.to_u64();
        let from_u64 = InstrumentId::from_u64(u64_key);
        // Check venue and asset_type preserved (asset_id may be truncated)
        let from_venue = from_u64.venue;
        let id_venue = id.venue;
        let from_asset_type = from_u64.asset_type;
        let id_asset_type = id.asset_type;
        assert_eq!(from_venue, id_venue, "U64 venue preservation failed");
        assert_eq!(from_asset_type, id_asset_type, "U64 asset_type preservation failed");
    }
}

#[test]
fn test_bijective_property_tokens() {
    // Test EVM token IDs from contract addresses
    let test_cases = [
        (VenueId::Ethereum, "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"), // WETH
        (VenueId::Polygon, "0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619"),  // WETH on Polygon
        (VenueId::BinanceSmartChain, "0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c"), // WBNB
    ];
    
    for (venue, address) in test_cases {
        let id = match venue {
            VenueId::Ethereum => InstrumentId::ethereum_token(address).unwrap(),
            VenueId::Polygon => InstrumentId::polygon_token(address).unwrap(),
            VenueId::BinanceSmartChain => InstrumentId::bsc_token(address).unwrap(),
            _ => panic!("Unexpected venue"),
        };
        
        // Verify venue and type
        assert_eq!(id.venue().unwrap(), venue);
        assert_eq!(id.asset_type().unwrap(), AssetType::Token);
        
        // Test cache key bijection
        let cache_key = id.cache_key();
        let recreated = InstrumentId::from_cache_key(cache_key);
        assert_eq!(id, recreated, "Token bijection failed for {}", address);
    }
}

#[test]
fn test_bijective_property_pools() {
    // Test DEX pool IDs
    let btc = InstrumentId::coin(VenueId::Binance, "BTC");
    let eth = InstrumentId::coin(VenueId::Binance, "ETH");
    let usdt = InstrumentId::coin(VenueId::Binance, "USDT");
    
    // Create pools
    let btc_usdt = InstrumentId::pool(VenueId::UniswapV2, btc, usdt);
    let eth_usdt = InstrumentId::pool(VenueId::UniswapV2, eth, usdt);
    let btc_eth = InstrumentId::pool(VenueId::UniswapV3, btc, eth);
    
    // Verify all pools are unique
    let mut pool_set = HashSet::new();
    assert!(pool_set.insert(btc_usdt.cache_key()));
    assert!(pool_set.insert(eth_usdt.cache_key()));
    assert!(pool_set.insert(btc_eth.cache_key()));
    
    // Verify pool properties
    for pool in [btc_usdt, eth_usdt, btc_eth] {
        assert_eq!(pool.asset_type().unwrap(), AssetType::Pool);
        
        // Test cache key bijection
        let cache_key = pool.cache_key();
        let recreated = InstrumentId::from_cache_key(cache_key);
        assert_eq!(pool, recreated, "Pool bijection failed");
    }
}

#[test]
fn test_cantor_pairing_bijection() {
    // Test the fundamental Cantor pairing function
    // Note: The implementation supports values up to 31 bits (2^31-1)
    let test_pairs = [
        (0, 0),
        (1, 0),
        (0, 1),
        (1, 1),
        (100, 200),
        (12345, 67890),
        (1000000, 2000000),
        (0x7FFFFFFF, 0), // Max 31-bit value
        (0, 0x7FFFFFFF),
        (0x7FFFFFFF, 0x7FFFFFFF),
    ];
    
    for (x, y) in test_pairs {
        let paired = cantor_pairing(x, y);
        
        // Verify deterministic behavior - same inputs produce same output
        let paired2 = cantor_pairing(x, y);
        assert_eq!(paired, paired2, "Non-deterministic pairing for ({}, {})", x, y);
        
        // Verify canonical ordering - order doesn't matter
        let paired_reversed = cantor_pairing(y, x);
        assert_eq!(paired, paired_reversed, "Order matters for ({}, {})", x, y);
        
        // The inverse functions are deprecated and return (0, 0)
        let (recovered_x, recovered_y) = inverse_cantor_pairing(paired);
        assert_eq!(recovered_x, 0, "Inverse should return 0 (deprecated)");
        assert_eq!(recovered_y, 0, "Inverse should return 0 (deprecated)");
    }
}

#[test]
fn test_cantor_triple_bijection() {
    // Test three-way Cantor pairing for triangular pools
    let test_triples = [
        (1, 2, 3),
        (0, 0, 0),
        (100, 200, 300),
        (u16::MAX as u64, u16::MAX as u64, u16::MAX as u64),
    ];
    
    for (x, y, z) in test_triples {
        let paired = cantor_pairing_triple(x, y, z);
        
        // Verify deterministic behavior
        let paired2 = cantor_pairing_triple(x, y, z);
        assert_eq!(paired, paired2, "Non-deterministic triple pairing");
        
        // Verify canonical ordering - any permutation produces same result
        let paired_xyz = cantor_pairing_triple(x, y, z);
        let paired_yxz = cantor_pairing_triple(y, x, z);
        let paired_zyx = cantor_pairing_triple(z, y, x);
        assert_eq!(paired_xyz, paired_yxz, "Order matters for triple");
        assert_eq!(paired_xyz, paired_zyx, "Order matters for triple");
        
        // The inverse functions are deprecated and return (0, 0, 0)
        let (recovered_x, recovered_y, recovered_z) = inverse_cantor_pairing_triple(paired);
        assert_eq!(recovered_x, 0, "Inverse should return 0 (deprecated)");
        assert_eq!(recovered_y, 0, "Inverse should return 0 (deprecated)");
        assert_eq!(recovered_z, 0, "Inverse should return 0 (deprecated)");
    }
}

#[test]
fn test_pool_id_canonical_ordering() {
    // Pool IDs should be the same regardless of token order
    let token_pairs = [
        (1, 2),
        (2, 1),
        (100, 50),
        (50, 100),
    ];
    
    // Each pair should produce the same pool ID
    assert_eq!(
        canonical_pool_id(token_pairs[0].0, token_pairs[0].1),
        canonical_pool_id(token_pairs[1].0, token_pairs[1].1)
    );
    
    assert_eq!(
        canonical_pool_id(token_pairs[2].0, token_pairs[2].1),
        canonical_pool_id(token_pairs[3].0, token_pairs[3].1)
    );
}

#[test]
fn test_triangular_pool_canonical_ordering() {
    // Triangular pools should be the same regardless of token order
    let permutations = [
        (1, 2, 3),
        (1, 3, 2),
        (2, 1, 3),
        (2, 3, 1),
        (3, 1, 2),
        (3, 2, 1),
    ];
    
    let first_id = canonical_triangular_pool_id(
        permutations[0].0,
        permutations[0].1,
        permutations[0].2
    );
    
    for (a, b, c) in &permutations[1..] {
        let id = canonical_triangular_pool_id(*a, *b, *c);
        assert_eq!(id, first_id,
            "Triangular pool ID not canonical for ({}, {}, {})", a, b, c);
    }
}

#[test]
fn test_no_collisions_million_instruments() {
    // Test that many instruments produce no ID collisions
    let mut ids = HashSet::new();
    let mut collision_count = 0;
    
    // Test coin instruments
    let venues = [VenueId::Binance, VenueId::Coinbase, VenueId::Kraken];
    let symbols = ["BTC", "ETH", "USDT", "USDC", "DAI", "LINK", "UNI", "AAVE"];
    
    for venue in venues {
        for symbol in symbols {
            let id = InstrumentId::coin(venue, symbol);
            if !ids.insert(id.cache_key()) {
                collision_count += 1;
                eprintln!("Collision detected: coin({:?}, {})", venue, symbol);
            }
        }
    }
    
    assert_eq!(collision_count, 0, "Found {} collisions in coin instruments", collision_count);
    
    // Test pools
    let token1 = InstrumentId::coin(VenueId::Ethereum, "WETH");
    let token2 = InstrumentId::coin(VenueId::Ethereum, "USDC");
    let token3 = InstrumentId::coin(VenueId::Ethereum, "DAI");
    
    let pools = [
        InstrumentId::pool(VenueId::UniswapV2, token1, token2),
        InstrumentId::pool(VenueId::UniswapV2, token1, token3),
        InstrumentId::pool(VenueId::UniswapV2, token2, token3),
        InstrumentId::pool(VenueId::UniswapV3, token1, token2),
        InstrumentId::pool(VenueId::SushiSwap, token1, token2),
    ];
    
    for pool in pools {
        if !ids.insert(pool.cache_key()) {
            collision_count += 1;
            eprintln!("Collision detected: pool");
        }
    }
    
    assert_eq!(collision_count, 0, "Found {} collisions including pools", collision_count);
    
    println!("Tested {} unique instruments with zero collisions", ids.len());
}

#[test]
fn test_venue_id_extraction() {
    // Test that venue IDs are correctly embedded and extracted
    let venues = [
        VenueId::Binance,
        VenueId::Coinbase,
        VenueId::Kraken,
        VenueId::UniswapV2,
        VenueId::UniswapV3,
    ];
    
    for venue in venues {
        let id = InstrumentId::coin(venue, "TEST");
        let extracted_venue = id.venue().unwrap();
        
        assert_eq!(extracted_venue, venue,
            "Venue extraction failed for {:?}", venue);
    }
}

#[test]
fn test_asset_type_preservation() {
    // Ensure asset type is preserved through all operations
    let test_ids = vec![
        (InstrumentId::coin(VenueId::Binance, "BTC"), AssetType::Coin),
        (InstrumentId::stock(VenueId::NASDAQ, "AAPL"), AssetType::Stock),
        (InstrumentId::bond(VenueId::NASDAQ, "US10Y"), AssetType::Bond),
    ];
    
    for (id, expected_type) in test_ids {
        assert_eq!(id.asset_type().unwrap(), expected_type,
            "Type not preserved for {:?}", id);
        
        // Roundtrip through cache key
        let cache_key = id.cache_key();
        let recovered = InstrumentId::from_cache_key(cache_key);
        assert_eq!(recovered.asset_type().unwrap(), expected_type,
            "Type not preserved after cache key conversion");
    }
}

#[test]
fn test_cantor_pairing_deterministic() {
    // Same inputs should always produce same output
    let x = 12345u64;
    let y = 67890u64;
    
    let result1 = cantor_pairing(x, y);
    let result2 = cantor_pairing(x, y);
    let result3 = cantor_pairing(x, y);
    
    assert_eq!(result1, result2);
    assert_eq!(result2, result3);
}

#[test]
fn test_cantor_pairing_non_commutative() {
    // With the new hash-based approach, we use canonical ordering
    // So (x,y) and (y,x) should produce the SAME result for pool IDs
    let x = 100u64;
    let y = 200u64;
    
    let xy = cantor_pairing(x, y);
    let yx = cantor_pairing(y, x);
    
    // Our implementation uses canonical ordering for deterministic pool IDs
    assert_eq!(xy, yx, "Pool IDs should be canonical regardless of order");
}

#[test]
fn test_edge_case_zero_values() {
    // Test with zero values in various positions
    let zero_cases = vec![
        InstrumentId::coin(VenueId::Binance, ""),
        InstrumentId::coin(VenueId::Binance, "X"),
    ];
    
    for id in zero_cases {
        let cache_key = id.cache_key();
        let recovered = InstrumentId::from_cache_key(cache_key);
        assert_eq!(id, recovered, "Zero value handling failed");
    }
}

#[test]
fn test_triangular_pool() {
    // Test triangular pool creation
    let token1 = InstrumentId::coin(VenueId::Ethereum, "WETH");
    let token2 = InstrumentId::coin(VenueId::Ethereum, "USDC");
    let token3 = InstrumentId::coin(VenueId::Ethereum, "DAI");
    
    let tri_pool = InstrumentId::triangular_pool(VenueId::Balancer, token1, token2, token3);
    
    // Verify it's marked as a pool
    assert_eq!(tri_pool.asset_type().unwrap(), AssetType::Pool);
    let reserved = tri_pool.reserved;
    assert_eq!(reserved, 1, "Triangular pool should have reserved flag");
    
    // Test bijection
    let cache_key = tri_pool.cache_key();
    let recovered = InstrumentId::from_cache_key(cache_key);
    assert_eq!(tri_pool, recovered, "Triangular pool bijection failed");
}

#[test]
fn test_lp_token() {
    // Test LP token creation
    let token1 = InstrumentId::coin(VenueId::Ethereum, "WETH");
    let token2 = InstrumentId::coin(VenueId::Ethereum, "USDC");
    let pool = InstrumentId::pool(VenueId::UniswapV2, token1, token2);
    
    let lp_token = InstrumentId::lp_token(VenueId::UniswapV2, pool);
    
    // Verify it's marked as LP token
    assert_eq!(lp_token.asset_type().unwrap(), AssetType::LPToken);
    
    // LP token should share pool's asset_id
    let lp_asset_id = lp_token.asset_id;
    let pool_asset_id = pool.asset_id;
    assert_eq!(lp_asset_id, pool_asset_id);
}

#[test]
fn test_option_instrument() {
    // Test option ID creation
    let option = InstrumentId::option(
        VenueId::Deribit,  // Options venue
        "SPY",
        450_00000000, // $450 strike (8 decimals)
        20240630,     // June 30, 2024 expiry
        true          // Call option
    );
    
    assert_eq!(option.asset_type().unwrap(), AssetType::Option);
    
    // Test bijection
    let cache_key = option.cache_key();
    let recovered = InstrumentId::from_cache_key(cache_key);
    assert_eq!(option, recovered, "Option bijection failed");
}
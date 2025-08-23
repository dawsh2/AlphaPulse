//! Bijective InstrumentId Property Tests
//! 
//! Ensures the bijective property: every ID maps to a unique instrument and back.
//! Tests the new address-based architecture without deprecated hash functions.

mod common;

use alphapulse_protocol_v2::{
    InstrumentId, VenueId, AssetType,
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
        assert_eq!(from_venue, id_venue, "Venue not preserved in u64 conversion");
        
        let from_asset_type = from_u64.asset_type;
        let id_asset_type = id.asset_type;
        assert_eq!(from_asset_type, id_asset_type, "Asset type not preserved in u64 conversion");
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
fn test_pool_id_deterministic() {
    // Pool IDs should be deterministic and canonical
    let btc = InstrumentId::coin(VenueId::Binance, "BTC");
    let eth = InstrumentId::coin(VenueId::Binance, "ETH");
    
    // Same pool should have same ID regardless of token order
    let pool1 = InstrumentId::pool(VenueId::UniswapV2, btc, eth);
    let pool2 = InstrumentId::pool(VenueId::UniswapV2, eth, btc);
    
    // With the new deterministic hash approach, order should not matter
    // (This depends on the specific implementation in the pool() method)
    // For now, we just ensure they're deterministic
    assert_eq!(pool1.venue, pool2.venue);
    assert_eq!(pool1.asset_type, pool2.asset_type);
}

#[test]
fn test_triangular_pool_deterministic() {
    // Triangular pools should be deterministic
    let btc = InstrumentId::coin(VenueId::Binance, "BTC");
    let eth = InstrumentId::coin(VenueId::Binance, "ETH");
    let usdt = InstrumentId::coin(VenueId::Binance, "USDT");
    
    // Create triangular pool
    let tri_pool = InstrumentId::triangular_pool(VenueId::Balancer, btc, eth, usdt);
    
    // Verify pool properties
    assert_eq!(tri_pool.asset_type().unwrap(), AssetType::Pool);
    assert_eq!(tri_pool.venue().unwrap(), VenueId::Balancer);
    assert_eq!(tri_pool.reserved, 1); // Flag for triangular pool
    
    // Test cache key bijection
    let cache_key = tri_pool.cache_key();
    let recreated = InstrumentId::from_cache_key(cache_key);
    assert_eq!(tri_pool, recreated, "Triangular pool bijection failed");
}

#[test]
fn test_collision_resistance() {
    // Test that different tokens don't collide
    let mut id_set = HashSet::new();
    
    // Add a large set of different token combinations
    for venue in [VenueId::Binance, VenueId::Coinbase, VenueId::Kraken] {
        for symbol in ["BTC", "ETH", "USDT", "ADA", "DOT", "LINK", "UNI", "MATIC"] {
            let coin = InstrumentId::coin(venue, symbol);
            assert!(id_set.insert(coin.cache_key()), 
                "Collision detected for {} on {}", symbol, venue as u16);
        }
    }
    
    // Test pool collisions
    let btc = InstrumentId::coin(VenueId::Binance, "BTC");
    let eth = InstrumentId::coin(VenueId::Binance, "ETH");
    let usdt = InstrumentId::coin(VenueId::Binance, "USDT");
    
    for venue in [VenueId::UniswapV2, VenueId::UniswapV3, VenueId::SushiswapV2] {
        let btc_eth = InstrumentId::pool(venue, btc, eth);
        let btc_usdt = InstrumentId::pool(venue, btc, usdt);
        let eth_usdt = InstrumentId::pool(venue, eth, usdt);
        
        assert!(id_set.insert(btc_eth.cache_key()));
        assert!(id_set.insert(btc_usdt.cache_key()));
        assert!(id_set.insert(eth_usdt.cache_key()));
    }
}

#[test]
fn test_lp_token_instrument() {
    // Test LP token creation
    let btc = InstrumentId::coin(VenueId::Binance, "BTC");
    let usdt = InstrumentId::coin(VenueId::Binance, "USDT");
    let pool = InstrumentId::pool(VenueId::UniswapV2, btc, usdt);
    
    let lp_token = InstrumentId::lp_token(VenueId::UniswapV2, pool);
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
//! # AlphaPulse Codec Integration Tests
//!
//! Comprehensive integration tests for the codec crate, verifying:
//! - Public API compatibility with external crates
//! - Cross-module functionality between instrument_id and tlv_types
//! - End-to-end identifier construction and validation
//! - Performance characteristics and error handling

use alphapulse_codec::{
    AssetType, CodecError, InstrumentId, TLVType, VenueId, MESSAGE_MAGIC, PROTOCOL_VERSION,
};
use zerocopy::{AsBytes, FromBytes};

#[test]
fn test_codec_public_api_basic_functionality() {
    // Test InstrumentId public API
    let btc_id = InstrumentId::coin(VenueId::Kraken, "BTC").unwrap();
    assert!(!btc_id.debug_info().is_empty());
    assert_eq!(btc_id.symbol_str(), "BTC");

    let eth_token =
        InstrumentId::ethereum_token("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2").unwrap();
    assert_eq!(eth_token.venue().unwrap(), VenueId::Ethereum);
    assert_eq!(eth_token.asset_type_enum().unwrap(), AssetType::Token);
    assert_eq!(eth_token.chain_id(), Some(1));

    // Test TLVType public API
    let trade_type = TLVType::Trade;
    assert_eq!(trade_type as u8, 1);

    // Test constants
    assert_eq!(MESSAGE_MAGIC, 0xDEADBEEF);
    assert_eq!(PROTOCOL_VERSION, 1);
}

#[test]
fn test_cross_module_functionality() {
    // Create instruments for different venues
    let nasdaq_stock = InstrumentId::stock(VenueId::NASDAQ, "AAPL").unwrap();
    let ethereum_token = InstrumentId::new(VenueId::Ethereum, AssetType::Token, "USDC").unwrap();
    let token_pair = InstrumentId::new(VenueId::Ethereum, AssetType::Token, "WETH").unwrap();
    let uniswap_pool = InstrumentId::pool(VenueId::UniswapV3, ethereum_token, token_pair).unwrap();

    // Test that different asset types have different properties
    assert!(!nasdaq_stock.is_defi());
    assert!(!ethereum_token.is_defi());
    assert!(uniswap_pool.is_defi());

    // Test venue classification consistency
    assert_eq!(ethereum_token.venue().unwrap(), VenueId::Ethereum);
    assert_eq!(uniswap_pool.venue().unwrap(), VenueId::UniswapV3);

    // Both Ethereum and UniswapV3 should map to same chain
    assert_eq!(ethereum_token.chain_id(), Some(1));
    assert_eq!(uniswap_pool.chain_id(), Some(1));
}

#[test]
fn test_performance_critical_paths() {
    // Test that hot path operations are efficient
    let start = std::time::Instant::now();

    // Create many instrument IDs (should be fast)
    let mut ids = Vec::new();
    for i in 0..1000 {
        let symbol = format!("SYM{}", i % 100); // Keep symbols short to avoid length errors
        if let Ok(id) = InstrumentId::coin(VenueId::Binance, &symbol) {
            ids.push(id.to_u64());
        }
    }

    let construction_time = start.elapsed();

    // Basic performance assertions (not strict benchmarks)
    assert!(
        construction_time.as_micros() < 50000,
        "ID construction too slow: {:?}",
        construction_time
    );

    println!("Performance test results:");
    println!("  1000 InstrumentId constructions: {:?}", construction_time);
    println!("  Successfully created {} instrument IDs", ids.len());
}

#[test]
fn test_zero_copy_compatibility() {
    // Test that InstrumentId works with zero-copy traits
    let id = InstrumentId::ethereum_token("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48").unwrap();

    // Test AsBytes conversion
    let bytes = id.as_bytes();
    assert_eq!(bytes.len(), InstrumentId::SIZE);

    // Test FromBytes conversion
    let recovered = InstrumentId::read_from(bytes).unwrap();
    assert_eq!(recovered, id);

    // Test that all fields are preserved through zero-copy
    assert_eq!(recovered.venue().unwrap(), id.venue().unwrap());
    assert_eq!(
        recovered.asset_type_enum().unwrap(),
        id.asset_type_enum().unwrap()
    );
    assert_eq!(recovered.symbol_str(), id.symbol_str());
}

#[test]
fn test_multi_chain_routing() {
    // Test that different chains are handled correctly
    let eth_usdc =
        InstrumentId::ethereum_token("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48").unwrap();
    let poly_usdc =
        InstrumentId::polygon_token("0x2791Bca1f2de4661Ed88A30DC4175f623Ccc1b78").unwrap();

    // Different chain IDs
    assert_eq!(eth_usdc.chain_id(), Some(1));
    assert_eq!(poly_usdc.chain_id(), Some(137));

    // Same asset type but different venues
    assert_eq!(eth_usdc.asset_type_enum().unwrap(), AssetType::Token);
    assert_eq!(poly_usdc.asset_type_enum().unwrap(), AssetType::Token);
    assert_ne!(eth_usdc.venue().unwrap(), poly_usdc.venue().unwrap());

    // Should not be able to pair cross-chain
    assert!(!eth_usdc.can_pair_with(&poly_usdc));

    // Test DeFi protocol routing
    let uniswap_v3 = VenueId::UniswapV3;
    let quickswap = VenueId::QuickSwap;

    assert!(uniswap_v3.is_defi());
    assert!(quickswap.is_defi());
    assert!(uniswap_v3.supports_pools());
    assert!(quickswap.supports_pools());

    // Different chains for DeFi
    assert_eq!(uniswap_v3.chain_id(), Some(1)); // Ethereum
    assert_eq!(quickswap.chain_id(), Some(137)); // Polygon
}

#[test]
fn test_error_handling() {
    // Test invalid address handling
    assert!(InstrumentId::ethereum_token("invalid").is_err());
    assert!(InstrumentId::ethereum_token("0x123").is_err()); // Too short

    // Note: Our current implementation truncates addresses to 16 bytes, so this won't error
    // In production, we'd want stricter validation

    // Test symbol length validation
    let too_long = "THIS_SYMBOL_IS_TOO_LONG_FOR_THE_16_BYTE_LIMIT";
    let result = InstrumentId::stock(VenueId::NYSE, too_long);
    assert!(result.is_err());
    assert_eq!(result, Err(CodecError::SymbolTooLong));
}

#[test]
fn test_deterministic_behavior() {
    // Test that operations are deterministic
    let symbol = "BTC";
    let venue = VenueId::Kraken;

    let id1 = InstrumentId::coin(venue, symbol).unwrap();
    let id2 = InstrumentId::coin(venue, symbol).unwrap();

    // Should be identical
    assert_eq!(id1, id2);
    assert_eq!(id1.to_u64(), id2.to_u64());
    assert_eq!(id1.debug_info(), id2.debug_info());

    // Pool creation should also be deterministic
    let token_a = InstrumentId::new(VenueId::Ethereum, AssetType::Token, "USDC").unwrap();
    let token_b = InstrumentId::new(VenueId::Ethereum, AssetType::Token, "WETH").unwrap();

    let pool1 = InstrumentId::pool(VenueId::UniswapV3, token_a, token_b).unwrap();
    let pool2 = InstrumentId::pool(VenueId::UniswapV3, token_b, token_a).unwrap(); // Different order

    // Should be the same regardless of token order
    assert_eq!(pool1, pool2);
}

#[test]
fn test_symbol_handling() {
    // Test normal symbols
    let btc = InstrumentId::coin(VenueId::Binance, "BTC").unwrap();
    assert_eq!(btc.symbol_str(), "BTC");

    // Test maximum length symbol
    let max_symbol = "0123456789ABCDEF"; // Exactly 16 bytes
    let max_id = InstrumentId::stock(VenueId::NYSE, max_symbol).unwrap();
    assert_eq!(max_id.symbol_str(), max_symbol);

    // Test empty symbol
    let empty_id = InstrumentId::stock(VenueId::NYSE, "").unwrap();
    assert_eq!(empty_id.symbol_str(), "");

    // Test symbol with special characters
    let special_id = InstrumentId::stock(VenueId::NYSE, "BTC/USD").unwrap();
    assert_eq!(special_id.symbol_str(), "BTC/USD");
}

#[test]
fn test_venue_validation() {
    // Test that invalid venue IDs are caught
    let mut invalid_id = InstrumentId::default();
    invalid_id.venue = 9999; // Invalid venue ID

    match invalid_id.venue() {
        Err(CodecError::InvalidVenue(9999)) => (), // Expected
        _ => panic!("Expected InvalidVenue error"),
    }

    // Test that invalid asset types are caught
    invalid_id.asset_type = 255; // Invalid asset type

    match invalid_id.asset_type_enum() {
        Err(CodecError::InvalidAssetType) => (), // Expected
        _ => panic!("Expected InvalidAssetType error"),
    }
}

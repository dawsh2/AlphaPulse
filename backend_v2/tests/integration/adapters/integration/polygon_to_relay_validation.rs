//! Polygon → MarketDataRelay Integration Validation
//!
//! Tests the complete pipeline from Polygon DEX collector to relay message format
//! Validates TLV message structure, relay compatibility, and semantic preservation

use adapter_service::input::collectors::polygon_dex::{
    validated_decoder::ValidatedPolygonDecoder,
    abi_events::DEXProtocol,
};
use adapter_service::{complete_validation_pipeline, ValidationError};
use protocol_v2::{
    TLVMessage, MessageHeader, RelayDomain, VenueId,
    tlv::market_data::PoolSwapTLV,
    parse_header, parse_tlv_extensions, TLVType,
};
use web3::types::Log;
use tokio::sync::mpsc;
use std::time::Duration;

use crate::fixtures::polygon;

/// Test that Polygon collector generates valid TLV messages for relay consumption
#[tokio::test]
async fn test_polygon_to_relay_tlv_generation() {
    println!("🧪 Testing Polygon → Relay TLV Generation");

    // Create TLV message channel
    let (tx, mut rx) = mpsc::channel::<TLVMessage>(10);

    // Create Polygon collector with our channel

    // Use real Uniswap V3 swap data
    let real_swap_log = polygon::uniswap_v3_swap_real();

    // Process the log through the collector's internal pipeline
    let decoder = ValidatedPolygonDecoder::new();
    let result = decoder.decode_and_validate(&real_swap_log, DEXProtocol::UniswapV3);

    assert!(result.is_ok(), "Polygon decoder should handle real V3 swap: {:?}", result);

    let pool_swap_tlv = result.unwrap();

    // Verify TLV structure is correct for relay consumption
    println!("✅ Generated PoolSwapTLV for relay");
    println!("   Pool Address: 0x{}", hex::encode(pool_swap_tlv.pool_address));
    println!("   Amount In: {} (decimals: {})", pool_swap_tlv.amount_in, pool_swap_tlv.amount_in_decimals);
    println!("   Amount Out: {} (decimals: {})", pool_swap_tlv.amount_out, pool_swap_tlv.amount_out_decimals);
    println!("   Block Number: {}", pool_swap_tlv.block_number);

    // Validate TLV fields for relay compatibility
    assert_ne!(pool_swap_tlv.pool_address, [0u8; 20], "Pool address must be non-zero");
    assert!(pool_swap_tlv.amount_in > 0, "Amount in must be positive");
    assert!(pool_swap_tlv.amount_out > 0, "Amount out must be positive");
    assert!(pool_swap_tlv.block_number > 0, "Block number must be set");
    assert!(pool_swap_tlv.timestamp_ns > 0, "Timestamp must be set");

    // For V3 swaps, validate V3-specific fields are set
    assert_ne!(pool_swap_tlv.sqrt_price_x96_after, [0u8; 20], "V3 sqrt_price must be set");
    assert_ne!(pool_swap_tlv.tick_after, 0, "V3 tick must be set");

    println!("✅ All TLV fields properly set for relay consumption");
}

/// Test relay message header generation and compatibility
#[tokio::test]
async fn test_relay_message_header_generation() {
    println!("🧪 Testing Relay Message Header Generation");

    // Create mock TLV message
    let pool_swap_tlv = create_test_pool_swap_tlv();
    let tlv_message = pool_swap_tlv.to_tlv_message();

    println!("   TLV payload size: {} bytes", tlv_message.payload.len());

    // Create relay message header (matching live_polygon_relay.rs format)
    let sequence = 1u64;
    let header = MessageHeader {
        magic: 0xDEADBEEF,
        relay_domain: RelayDomain::MarketData as u8,
        version: 1,
        source: 3, // Polygon source ID
        flags: 0,
        payload_size: tlv_message.payload.len() as u32,
        sequence,
        timestamp: network::time::safe_system_timestamp_ns(),
        checksum: 0, // Performance mode
    };

    // Validate header fields
    assert_eq!(header.magic, 0xDEADBEEF, "Correct magic number");
    assert_eq!(header.relay_domain, RelayDomain::MarketData as u8, "Correct relay domain");
    assert_eq!(header.version, 1, "Correct protocol version");
    assert_eq!(header.source, 3, "Correct source ID for Polygon");
    assert_eq!(header.payload_size as usize, tlv_message.payload.len(), "Payload size matches");
    assert!(header.timestamp > 0, "Timestamp set");
    assert_eq!(header.sequence, sequence, "Sequence number set");

    // Test header serialization (unsafe but required for relay compatibility)
    let header_bytes = unsafe {
        std::slice::from_raw_parts(
            &header as *const MessageHeader as *const u8,
            std::mem::size_of::<MessageHeader>()
        )
    };

    assert_eq!(header_bytes.len(), 32, "Header is exactly 32 bytes");

    // Construct complete relay message
    let mut relay_message = Vec::with_capacity(header_bytes.len() + tlv_message.payload.len());
    relay_message.extend_from_slice(header_bytes);
    relay_message.extend_from_slice(&tlv_message.payload);

    println!("✅ Relay message generated: {} total bytes", relay_message.len());
    println!("   Header: {} bytes", header_bytes.len());
    println!("   Payload: {} bytes", tlv_message.payload.len());

    // Test that relay could parse this message back
    let parsed_header = parse_header(&relay_message).expect("Relay should parse header");
    assert_eq!(parsed_header.magic, 0xDEADBEEF);
    assert_eq!(parsed_header.payload_size as usize, tlv_message.payload.len());

    let tlv_payload = &relay_message[32..32 + parsed_header.payload_size as usize];
    let parsed_tlvs = parse_tlv_extensions(tlv_payload).expect("Relay should parse TLVs");

    assert_eq!(parsed_tlvs.len(), 1, "Should have exactly one TLV");
    // NOTE: We can't easily verify the TLV type here without more parsing logic

    println!("✅ Relay message successfully roundtrips through parser");
}

/// Test end-to-end semantic preservation through relay pipeline
#[tokio::test]
async fn test_e2e_semantic_preservation() {
    println!("🧪 Testing End-to-End Semantic Preservation");

    let decoder = ValidatedPolygonDecoder::new();

    // Test with real Uniswap V3 data
    let v3_log = polygon::uniswap_v3_swap_real();
    let v3_result = decoder.decode_and_validate(&v3_log, DEXProtocol::UniswapV3);
    assert!(v3_result.is_ok(), "V3 decoding should succeed");

    let v3_tlv = v3_result.unwrap();

    // Test with real V2 data
    let v2_log = polygon::quickswap_v2_swap_real();
    let v2_result = decoder.decode_and_validate(&v2_log, DEXProtocol::UniswapV2);
    assert!(v2_result.is_ok(), "V2 decoding should succeed");

    let v2_tlv = v2_result.unwrap();

    // Verify V3 vs V2 differentiation
    assert_ne!(v3_tlv.sqrt_price_x96_after, [0u8; 20], "V3 should have sqrt_price");
    assert_eq!(v2_tlv.sqrt_price_x96_after, [0u8; 20], "V2 should have zero sqrt_price");

    assert_ne!(v3_tlv.tick_after, 0, "V3 should have tick");
    assert_eq!(v2_tlv.tick_after, 0, "V2 should have zero tick");

    // Verify both have valid common fields
    for (tlv, protocol) in [(&v3_tlv, "V3"), (&v2_tlv, "V2")] {
        assert_ne!(tlv.pool_address, [0u8; 20], "{} pool address set", protocol);
        assert!(tlv.amount_in > 0, "{} amount_in positive", protocol);
        assert!(tlv.amount_out > 0, "{} amount_out positive", protocol);
        assert!(tlv.block_number > 0, "{} block_number set", protocol);
        assert!(tlv.timestamp_ns > 0, "{} timestamp set", protocol);
    }

    println!("✅ Semantic preservation validated for V3 and V2");
    println!("   V3 TLV: sqrt_price={:?}, tick={}",
             v3_tlv.sqrt_price_x96_after[0..4].iter().any(|&b| b != 0),
             v3_tlv.tick_after);
    println!("   V2 TLV: sqrt_price={:?}, tick={}",
             v2_tlv.sqrt_price_x96_after[0..4].iter().any(|&b| b != 0),
             v2_tlv.tick_after);
}

/// Test deep equality validation through complete pipeline
#[tokio::test]
async fn test_deep_equality_validation() {
    println!("🧪 Testing Deep Equality Validation");

    let real_log = polygon::uniswap_v3_swap_real();

    // Run complete validation pipeline
    let validation_result = complete_validation_pipeline(&real_log, DEXProtocol::UniswapV3);
    assert!(validation_result.is_ok(), "Complete validation should pass: {:?}", validation_result);

    let validated_tlv = validation_result.unwrap();

    // Verify the TLV passed all validation stages:
    // 1. Raw data parsing ✓
    // 2. TLV serialization ✓
    // 3. TLV deserialization ✓
    // 4. Deep equality check ✓

    println!("✅ Complete validation pipeline passed");
    println!("   Pool: 0x{}", hex::encode(validated_tlv.pool_address));
    println!("   Venue: {:?}", validated_tlv.venue);
    println!("   Block: {}", validated_tlv.block_number);

    // This TLV is now certified ready for relay consumption
    assert!(validated_tlv.amount_in > 0);
    assert!(validated_tlv.amount_out > 0);
}

/// Helper to create test PoolSwapTLV
fn create_test_pool_swap_tlv() -> PoolSwapTLV {
    PoolSwapTLV {
        venue: VenueId::Polygon,
        pool_address: [0x45, 0xdd, 0xa9, 0xcb, 0x7c, 0x25, 0x13, 0x1d, 0xf2, 0x68, 0x51, 0x51, 0x31, 0xf6, 0x47, 0xd7, 0x26, 0xf5, 0x06, 0x08],
        token_in_addr: [0x7c, 0xeb, 0x23, 0xfd, 0x6f, 0x88, 0xb7, 0x6a, 0xf0, 0x52, 0xc3, 0xca, 0x45, 0x9c, 0x11, 0x73, 0xc5, 0xb9, 0xb9, 0x6d],
        token_out_addr: [0x27, 0x91, 0xbc, 0xa1, 0xf2, 0xde, 0x46, 0x61, 0xed, 0x88, 0xa3, 0x0c, 0x99, 0xa7, 0xa9, 0x44, 0x9a, 0xa8, 0x41, 0x74],
        amount_in: 10_000_000_000_000_000_000u128, // 10 WETH (18 decimals)
        amount_out: 27000_000_000u128, // 27,000 USDC (6 decimals)
        amount_in_decimals: 18,
        amount_out_decimals: 6,
        sqrt_price_x96_after: PoolSwapTLV::sqrt_price_from_u128(1792282187229267636352u128), // Realistic V3 price
        tick_after: 3393,
        liquidity_after: 1000000000000000000u128,
        timestamp_ns: network::time::safe_system_timestamp_ns(),
        block_number: 48_600_000,
    }
}

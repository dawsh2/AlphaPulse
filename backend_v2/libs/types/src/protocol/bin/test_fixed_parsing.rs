//! Test Fixed TLV Parsing Performance
//!
//! Validates the relay parser fix that resolves the 0 msg/s bottleneck

use alphapulse_types::{
    tlv::{parse_tlv_extensions_for_relay, TLVMessageBuilder},
    MessageHeader, RelayDomain, SourceType, TLVType,
};
use std::time::Instant;
use tracing::{info, Level};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("🔧 Testing Fixed TLV Parsing Performance");

    // Test 1: Verify the fix works with the problematic payload size
    test_33_byte_payload_parsing()?;

    // Test 2: Performance test with optimized parser
    test_optimized_relay_processing().await?;

    // Test 3: Compare old vs new parser performance
    test_parser_performance_comparison().await?;

    info!("✅ All fixed parsing tests completed successfully!");

    Ok(())
}

/// Test that 33-byte payloads now work (the ones that failed before)
fn test_33_byte_payload_parsing() -> Result<(), Box<dyn std::error::Error>> {
    info!("🧪 Test 1: 33-byte Payload Parsing (Previously Failed)");

    // Create the exact payload that was failing: 33 bytes
    let trade_payload = vec![
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, // instrument_id (8 bytes)
        0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, // price (8 bytes)
        0x00, 0x00, 0x00, 0x00, 0x0F, 0x00, 0x00, 0x00, // volume (8 bytes)
        0x01, // side (1 byte)
        0x30, 0x39, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // trade_id (8 bytes)
    ]; // Total: 33 bytes

    info!("   Created payload: {} bytes", trade_payload.len());

    let message = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
        .add_tlv_bytes(TLVType::Trade, trade_payload)
        .build();

    let tlv_payload = &message[MessageHeader::SIZE..];

    // Try with the optimized relay parser
    match parse_tlv_extensions_for_relay(tlv_payload) {
        Ok(tlvs) => {
            info!(
                "   ✅ SUCCESS: Parsed {} TLVs with optimized parser",
                tlvs.len()
            );
            info!(
                "   TLV Type: {}, Length: {}",
                tlvs[0].tlv_type, tlvs[0].tlv_length
            );

            // Verify it's valid for market data domain
            if tlvs[0].is_valid_for_domain(RelayDomain::MarketData) {
                info!("   ✅ Domain validation: Valid for MarketData");
            } else {
                return Err("Domain validation failed".into());
            }
        }
        Err(e) => {
            return Err(format!("Optimized parser still fails: {:?}", e).into());
        }
    }

    Ok(())
}

/// Test performance with the optimized relay processor
async fn test_optimized_relay_processing() -> Result<(), Box<dyn std::error::Error>> {
    info!("⚡ Test 2: Optimized Relay Processing Performance");

    const BATCH_SIZE: usize = 10_000;
    let mut processed_count = 0;

    let start = Instant::now();

    for i in 0..BATCH_SIZE {
        // Create the exact same payload that was failing before
        let trade_payload = vec![
            0x01,
            0x02,
            0x03,
            0x04,
            0x05,
            0x06,
            0x07,
            0x08, // instrument_id
            (i & 0xFF) as u8,
            0x00,
            0x10,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00, // price (varying)
            0x00,
            0x00,
            0x00,
            0x00,
            0x0F,
            0x00,
            0x00,
            0x00, // volume
            0x01, // side
            ((i + 10000) & 0xFF) as u8,
            0x39,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00, // trade_id
        ];

        let message = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
            .add_tlv_bytes(TLVType::Trade, trade_payload)
            .build();

        // Extract TLV payload section
        let tlv_payload = &message[MessageHeader::SIZE..];

        // Use the optimized relay parser
        match parse_tlv_extensions_for_relay(tlv_payload) {
            Ok(tlvs) => {
                // Check domain validation
                for tlv in tlvs {
                    if tlv.is_valid_for_domain(RelayDomain::MarketData) {
                        processed_count += 1;
                        break; // Only count once per message
                    }
                }
            }
            Err(_) => {
                // Should not happen with the fixed parser
            }
        }
    }

    let processing_time = start.elapsed();
    let throughput = processed_count as f64 / processing_time.as_secs_f64();

    info!("   Optimized Processing Results:");
    info!(
        "      Messages processed: {}/{} ({:.1}%)",
        processed_count,
        BATCH_SIZE,
        (processed_count as f64 / BATCH_SIZE as f64) * 100.0
    );
    info!("      Processing time: {:?}", processing_time);
    info!("      Throughput: {:.0} msg/s", throughput);

    if processed_count == BATCH_SIZE {
        info!("   ✅ ALL MESSAGES PROCESSED SUCCESSFULLY!");
        if throughput > 1_000_000.0 {
            info!(
                "   🚀 EXCEEDS 1M MSG/S TARGET: {:.1}x faster than target",
                throughput / 1_000_000.0
            );
        } else {
            info!(
                "   📊 Throughput: {:.1}% of 1M msg/s target",
                (throughput / 1_000_000.0) * 100.0
            );
        }
    } else {
        return Err(format!("Only processed {}/{} messages", processed_count, BATCH_SIZE).into());
    }

    Ok(())
}

/// Compare old parser vs new parser performance
async fn test_parser_performance_comparison() -> Result<(), Box<dyn std::error::Error>> {
    info!("📊 Test 3: Parser Performance Comparison");

    const COMPARISON_SIZE: usize = 1_000;

    // Create test payload (24 bytes - should work with both parsers)
    let good_payload = vec![0u8; 24];

    let message = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
        .add_tlv_bytes(TLVType::Trade, good_payload)
        .build();

    let tlv_payload = &message[MessageHeader::SIZE..];

    // Test optimized parser performance
    let start = Instant::now();
    let mut relay_success = 0;
    for _ in 0..COMPARISON_SIZE {
        if let Ok(tlvs) = parse_tlv_extensions_for_relay(tlv_payload) {
            if !tlvs.is_empty() {
                relay_success += 1;
            }
        }
    }
    let relay_time = start.elapsed();
    let relay_throughput = relay_success as f64 / relay_time.as_secs_f64();

    // Test original parser performance (should also work with 24-byte payload)
    let start = Instant::now();
    let mut original_success = 0;
    for _ in 0..COMPARISON_SIZE {
        if let Ok(tlvs) = protocol_v2::parse_tlv_extensions(tlv_payload) {
            if !tlvs.is_empty() {
                original_success += 1;
            }
        }
    }
    let original_time = start.elapsed();
    let original_throughput = original_success as f64 / original_time.as_secs_f64();

    info!("   Performance Comparison Results:");
    info!("      Original Parser: {:.0} msg/s", original_throughput);
    info!("      Optimized Parser: {:.0} msg/s", relay_throughput);

    if relay_throughput > original_throughput {
        let speedup = relay_throughput / original_throughput;
        info!("      🚀 Optimized is {:.1}x FASTER", speedup);
    } else {
        info!("      📊 Similar performance (both working)");
    }

    // Test with problematic payload (33 bytes)
    info!("   Testing with 33-byte payload (original fails, optimized works):");
    let bad_payload = vec![0u8; 33];
    let bad_message = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
        .add_tlv_bytes(TLVType::Trade, bad_payload)
        .build();

    let bad_tlv_payload = &bad_message[MessageHeader::SIZE..];

    // Original parser should fail
    match protocol_v2::parse_tlv_extensions(bad_tlv_payload) {
        Ok(_) => info!("      ⚠️  Original parser unexpectedly succeeded"),
        Err(_) => info!("      ❌ Original parser fails (as expected)"),
    }

    // Optimized parser should succeed
    match parse_tlv_extensions_for_relay(bad_tlv_payload) {
        Ok(tlvs) => info!(
            "      ✅ Optimized parser succeeds with {} TLVs",
            tlvs.len()
        ),
        Err(e) => return Err(format!("Optimized parser failed: {:?}", e).into()),
    }

    Ok(())
}

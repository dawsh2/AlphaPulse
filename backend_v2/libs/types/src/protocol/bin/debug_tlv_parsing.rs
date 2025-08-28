//! Debug TLV Parsing Issues
//!
//! Investigates the critical TLV parsing bottleneck found in performance tests.
//! This binary will isolate and fix the specific issue causing 0 msg/s throughput.

use torq_types::{
    parse_tlv_extensions, tlv::TLVMessageBuilder, MessageHeader, RelayDomain, SourceType, TLVType,
};
use tracing::{error, info, warn, Level};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    info!("🔍 Debugging TLV Parsing Bottleneck");

    // Test 1: Create a simple message and try to parse its TLVs
    test_basic_tlv_parsing()?;

    // Test 2: Test with various TLV sizes
    test_different_tlv_sizes()?;

    // Test 3: Test the exact scenario from performance test
    test_performance_test_scenario()?;

    // Test 4: Test TLV validation logic
    test_tlv_validation_overhead()?;

    info!("✅ TLV parsing debug complete");

    Ok(())
}

/// Test basic TLV parsing with a simple message
fn test_basic_tlv_parsing() -> Result<(), Box<dyn std::error::Error>> {
    info!("🧪 Test 1: Basic TLV Parsing");

    // Create a simple trade message
    let trade_payload = vec![
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, // instrument_id (8 bytes)
        0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, // price (8 bytes)
        0x00, 0x00, 0x00, 0x00, 0x0F, 0x00, 0x00, 0x00, // volume (8 bytes)
    ];

    info!(
        "   Creating TLV message with {} byte payload...",
        trade_payload.len()
    );

    let message = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
        .add_tlv_bytes(TLVType::Trade, trade_payload)
        .build();

    info!("   Message created: {} bytes total", message.len());

    // Extract TLV payload section
    let tlv_payload = &message[MessageHeader::SIZE..];
    info!("   TLV payload section: {} bytes", tlv_payload.len());
    info!(
        "   TLV bytes: {:?}",
        &tlv_payload[..std::cmp::min(20, tlv_payload.len())]
    );

    // Try to parse TLVs
    match parse_tlv_extensions(tlv_payload) {
        Ok(tlvs) => {
            info!("   ✅ Successfully parsed {} TLVs", tlvs.len());
            for (i, tlv) in tlvs.iter().enumerate() {
                match tlv {
                    protocol_v2::TLVExtensionEnum::Standard(std_tlv) => {
                        let tlv_type = std_tlv.header.tlv_type;
                        let tlv_length = std_tlv.header.tlv_length;
                        info!("     TLV {}: Type={}, Length={}", i, tlv_type, tlv_length);
                    }
                    protocol_v2::TLVExtensionEnum::Extended(ext_tlv) => {
                        let tlv_type = ext_tlv.header.tlv_type;
                        let tlv_length = ext_tlv.header.tlv_length;
                        info!(
                            "     TLV {}: Extended Type={}, Length={}",
                            i, tlv_type, tlv_length
                        );
                    }
                }
            }
        }
        Err(e) => {
            error!("   ❌ TLV parsing failed: {:?}", e);
            return Err(Box::new(e));
        }
    }

    Ok(())
}

/// Test TLV parsing with different payload sizes
fn test_different_tlv_sizes() -> Result<(), Box<dyn std::error::Error>> {
    info!("🧪 Test 2: Different TLV Sizes");

    let test_cases = vec![
        ("Small (4 bytes)", vec![0x01, 0x02, 0x03, 0x04]),
        ("Medium (24 bytes)", vec![0; 24]), // Trade size
        ("Large (100 bytes)", vec![0; 100]),
    ];

    for (name, payload) in test_cases {
        info!("   Testing {}: {} bytes", name, payload.len());

        let message = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
            .add_tlv_bytes(TLVType::Trade, payload)
            .build();

        let tlv_payload = &message[MessageHeader::SIZE..];

        match parse_tlv_extensions(tlv_payload) {
            Ok(tlvs) => {
                info!("     ✅ Parsed {} TLVs", tlvs.len());
            }
            Err(e) => {
                error!("     ❌ Failed: {:?}", e);
            }
        }
    }

    Ok(())
}

/// Test the exact scenario from the performance test that failed
fn test_performance_test_scenario() -> Result<(), Box<dyn std::error::Error>> {
    info!("🧪 Test 3: Performance Test Scenario Reproduction");

    // This mirrors the exact code from the performance test that caused 0 throughput
    const BATCH_SIZE: usize = 1000;
    let mut processed_count = 0;

    info!("   Processing {} messages...", BATCH_SIZE);

    for i in 0..BATCH_SIZE {
        // Create the exact same payload as performance test
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

        // Try the parsing that failed in performance test
        match parse_tlv_extensions(tlv_payload) {
            Ok(tlvs) => {
                // Check if any TLVs were found
                for tlv in tlvs {
                    let tlv_type = match tlv {
                        protocol_v2::TLVExtensionEnum::Standard(ref std_tlv) => {
                            std_tlv.header.tlv_type
                        }
                        protocol_v2::TLVExtensionEnum::Extended(ref ext_tlv) => {
                            ext_tlv.header.tlv_type
                        }
                    };

                    if (1..=19).contains(&tlv_type) {
                        processed_count += 1;
                        break; // Only count once per message
                    }
                }
            }
            Err(e) => {
                error!("   Message {}: TLV parsing failed: {:?}", i, e);
                // Don't break - let's see how many fail
            }
        }

        // Log progress periodically
        if i % 100 == 0 {
            info!(
                "   Processed {}/{}, successful: {}",
                i, BATCH_SIZE, processed_count
            );
        }
    }

    info!(
        "   Final result: {}/{} messages processed successfully",
        processed_count, BATCH_SIZE
    );

    if processed_count == 0 {
        error!("   ❌ REPRODUCED THE BUG: 0 messages processed!");
    } else if processed_count == BATCH_SIZE {
        info!("   ✅ All messages processed successfully");
    } else {
        warn!(
            "   ⚠️  Partial success: {:.1}% messages processed",
            (processed_count as f64 / BATCH_SIZE as f64) * 100.0
        );
    }

    Ok(())
}

/// Test TLV validation overhead specifically
fn test_tlv_validation_overhead() -> Result<(), Box<dyn std::error::Error>> {
    info!("🧪 Test 4: TLV Validation Overhead");

    // Test the size validation that might be causing issues
    let test_cases = vec![
        ("Valid Trade (24 bytes)", TLVType::Trade, vec![0; 24], true),
        (
            "Invalid Trade (20 bytes)",
            TLVType::Trade,
            vec![0; 20],
            false,
        ),
        (
            "Invalid Trade (30 bytes)",
            TLVType::Trade,
            vec![0; 30],
            false,
        ),
        ("Valid Quote (32 bytes)", TLVType::Quote, vec![0; 32], true),
    ];

    for (name, tlv_type, payload, should_succeed) in test_cases {
        info!("   Testing {}", name);

        let message = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
            .add_tlv_bytes(tlv_type, payload)
            .build();

        let tlv_payload = &message[MessageHeader::SIZE..];

        match parse_tlv_extensions(tlv_payload) {
            Ok(tlvs) => {
                if should_succeed {
                    info!("     ✅ Expected success: parsed {} TLVs", tlvs.len());
                } else {
                    warn!("     ⚠️  Unexpected success: parsed {} TLVs", tlvs.len());
                }
            }
            Err(e) => {
                if should_succeed {
                    error!("     ❌ Unexpected failure: {:?}", e);
                } else {
                    info!("     ✅ Expected failure: {:?}", e);
                }
            }
        }
    }

    Ok(())
}

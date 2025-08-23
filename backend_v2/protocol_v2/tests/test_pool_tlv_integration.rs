//! Integration test for pool TLV messages through the protocol stack
//!
//! Tests parsing, building, and relay of all pool-related TLVs

use alphapulse_protocol::{
    tlv::{
        parse_tlv_extensions, PoolBurnTLV, PoolMintTLV, PoolSwapTLV, PoolSyncTLV,
        TLVMessageBuilder, TLVType,
    },
    PoolInstrumentId, PoolProtocol, RelayDomain, SourceType, VenueId,
};

#[test]
fn test_pool_swap_through_builder_and_parser() {
    // Create a V3 swap with all fields populated
    let pool_id = PoolInstrumentId::from_v3_pair(VenueId::Polygon, 1234, 5678);

    let swap = PoolSwapTLV {
        venue: VenueId::Polygon,
        pool_id: pool_id.clone(),
        token_in: 1234,
        token_out: 5678,
        amount_in: 1000_00000000,
        amount_out: 2000_00000000,
        fee_paid: 3_00000000,
        sqrt_price_x96_after: 79228162514264337593543950336,
        tick_after: 100,
        liquidity_after: 1000000_00000000,
        timestamp_ns: 1234567890,
        block_number: 1000,
    };

    // Build TLV message
    let message = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::Collector)
        .add_tlv_bytes(TLVType::PoolSwap, swap.to_bytes())
        .build()
        .expect("Failed to build message");

    // Parse it back
    let extensions = parse_tlv_extensions(&message.payload).expect("Failed to parse");
    assert_eq!(extensions.len(), 1);

    // Verify the data
    if let Some(ext) = extensions.first() {
        match ext {
            TLVExtensionEnum::Standard(tlv) => {
                assert_eq!(tlv.header.tlv_type, TLVType::PoolSwap as u8);
                let decoded = PoolSwapTLV::from_bytes(&tlv.payload).expect("Failed to decode");
                assert_eq!(decoded.sqrt_price_x96_after, swap.sqrt_price_x96_after);
                assert_eq!(decoded.tick_after, swap.tick_after);
                assert_eq!(decoded.liquidity_after, swap.liquidity_after);
            }
            _ => panic!("Expected standard TLV"),
        }
    }
}

#[test]
fn test_pool_sync_through_builder_and_parser() {
    let pool_id = PoolInstrumentId::from_v2_pair(VenueId::Polygon, 1234, 5678);

    let sync = PoolSyncTLV {
        venue: VenueId::Polygon,
        pool_id: pool_id.clone(),
        reserve0: 1000000_00000000,
        reserve1: 2000000_00000000,
        timestamp_ns: 1234567890,
        block_number: 1000,
    };

    // Build TLV message
    let message = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::Collector)
        .add_tlv_bytes(TLVType::PoolSync, sync.to_bytes())
        .build()
        .expect("Failed to build message");

    // Parse it back
    let extensions = parse_tlv_extensions(&message.payload).expect("Failed to parse");
    assert_eq!(extensions.len(), 1);

    // Verify the data
    if let Some(ext) = extensions.first() {
        match ext {
            TLVExtensionEnum::Standard(tlv) => {
                assert_eq!(tlv.header.tlv_type, TLVType::PoolSync as u8);
                let decoded = PoolSyncTLV::from_bytes(&tlv.payload).expect("Failed to decode");
                assert_eq!(decoded.reserve0, sync.reserve0);
                assert_eq!(decoded.reserve1, sync.reserve1);
            }
            _ => panic!("Expected standard TLV"),
        }
    }
}

#[test]
fn test_v2_vs_v3_pool_differentiation() {
    // V2 pool
    let v2_pool = PoolInstrumentId::from_v2_pair(VenueId::Polygon, 1234, 5678);
    assert!(v2_pool.is_v2());
    assert!(!v2_pool.is_v3());

    // V3 pool
    let v3_pool = PoolInstrumentId::from_v3_pair(VenueId::Polygon, 1234, 5678);
    assert!(!v3_pool.is_v2());
    assert!(v3_pool.is_v3());

    // Different hashes for same tokens but different protocols
    assert_ne!(v2_pool.fast_hash, v3_pool.fast_hash);

    // V2 swap should have zero V3 fields
    let v2_swap = PoolSwapTLV {
        venue: VenueId::Polygon,
        pool_id: v2_pool,
        token_in: 1234,
        token_out: 5678,
        amount_in: 1000_00000000,
        amount_out: 2000_00000000,
        fee_paid: 3_00000000,
        sqrt_price_x96_after: 0, // V2 doesn't have this
        tick_after: 0,
        liquidity_after: 0,
        timestamp_ns: 1234567890,
        block_number: 1000,
    };

    let bytes = v2_swap.to_bytes();
    let decoded = PoolSwapTLV::from_bytes(&bytes).expect("Failed to decode");
    assert_eq!(decoded.sqrt_price_x96_after, 0);
    assert_eq!(decoded.tick_after, 0);
    assert_eq!(decoded.liquidity_after, 0);
}

#[test]
fn test_multiple_pool_tlvs_in_single_message() {
    let pool_id = PoolInstrumentId::from_v2_pair(VenueId::Polygon, 1234, 5678);

    // Create different pool events
    let swap = PoolSwapTLV {
        venue: VenueId::Polygon,
        pool_id: pool_id.clone(),
        token_in: 1234,
        token_out: 5678,
        amount_in: 100_00000000,
        amount_out: 200_00000000,
        fee_paid: 3_000000,
        sqrt_price_x96_after: 0,
        tick_after: 0,
        liquidity_after: 0,
        timestamp_ns: 1000000000,
        block_number: 100,
    };

    let sync = PoolSyncTLV {
        venue: VenueId::Polygon,
        pool_id: pool_id.clone(),
        reserve0: 1000000_00000000,
        reserve1: 2000000_00000000,
        timestamp_ns: 1000000001,
        block_number: 100,
    };

    let mint = PoolMintTLV {
        venue: VenueId::Polygon,
        pool_id: pool_id.clone(),
        provider: 0xDEADBEEF,
        tick_lower: i32::MIN, // V2 uses full range
        tick_upper: i32::MAX,
        liquidity_delta: 1000_00000000,
        amount0: 500_00000000,
        amount1: 500_00000000,
        timestamp_ns: 1000000002,
        block_number: 101,
    };

    // Build message with multiple TLVs
    let message = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::Collector)
        .add_tlv_bytes(TLVType::PoolSwap, swap.to_bytes())
        .add_tlv_bytes(TLVType::PoolSync, sync.to_bytes())
        .add_tlv_bytes(TLVType::PoolMint, mint.to_bytes())
        .build()
        .expect("Failed to build message");

    // Parse all TLVs
    let extensions = parse_tlv_extensions(&message.payload).expect("Failed to parse");
    assert_eq!(extensions.len(), 3);

    // Verify each TLV
    let mut found_swap = false;
    let mut found_sync = false;
    let mut found_mint = false;

    for ext in extensions {
        match ext {
            TLVExtensionEnum::Standard(tlv) => match TLVType::try_from(tlv.header.tlv_type) {
                Ok(TLVType::PoolSwap) => {
                    found_swap = true;
                    let decoded = PoolSwapTLV::from_bytes(&tlv.payload).unwrap();
                    assert_eq!(decoded.amount_in, swap.amount_in);
                }
                Ok(TLVType::PoolSync) => {
                    found_sync = true;
                    let decoded = PoolSyncTLV::from_bytes(&tlv.payload).unwrap();
                    assert_eq!(decoded.reserve0, sync.reserve0);
                }
                Ok(TLVType::PoolMint) => {
                    found_mint = true;
                    let decoded = PoolMintTLV::from_bytes(&tlv.payload).unwrap();
                    assert_eq!(decoded.liquidity_delta, mint.liquidity_delta);
                }
                _ => {}
            },
            _ => {}
        }
    }

    assert!(found_swap, "PoolSwap TLV not found");
    assert!(found_sync, "PoolSync TLV not found");
    assert!(found_mint, "PoolMint TLV not found");
}

#[test]
fn test_pool_state_flow() {
    // Simulate the flow of pool state updates
    let pool_id = PoolInstrumentId::from_v3_pair(VenueId::Polygon, 1234, 5678);

    // 1. First swap discovers the pool (V3 includes state)
    let initial_swap = PoolSwapTLV {
        venue: VenueId::Polygon,
        pool_id: pool_id.clone(),
        token_in: 1234,
        token_out: 5678,
        amount_in: 100_00000000,
        amount_out: 99_00000000,
        fee_paid: 30_000000,
        sqrt_price_x96_after: 79228162514264337593543950336,
        tick_after: 0,
        liquidity_after: 1000000_00000000,
        timestamp_ns: 1000000000,
        block_number: 100,
    };

    // 2. Mint adds liquidity
    let mint = PoolMintTLV {
        venue: VenueId::Polygon,
        pool_id: pool_id.clone(),
        provider: 0xDEADBEEF,
        tick_lower: -887272,
        tick_upper: 887272,
        liquidity_delta: 500_00000000,
        amount0: 250_00000000,
        amount1: 250_00000000,
        timestamp_ns: 1000000001,
        block_number: 101,
    };

    // 3. Another swap with updated state
    let second_swap = PoolSwapTLV {
        venue: VenueId::Polygon,
        pool_id: pool_id.clone(),
        token_in: 5678,
        token_out: 1234,
        amount_in: 50_00000000,
        amount_out: 49_50000000,
        fee_paid: 15_000000,
        sqrt_price_x96_after: 79628162514264337593543950336, // Price moved
        tick_after: 10,                                      // Tick changed
        liquidity_after: 1000500_00000000,                   // Liquidity increased from mint
        timestamp_ns: 1000000002,
        block_number: 102,
    };

    // Build message simulating event stream
    let message = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::Collector)
        .add_tlv_bytes(TLVType::PoolSwap, initial_swap.to_bytes())
        .add_tlv_bytes(TLVType::PoolMint, mint.to_bytes())
        .add_tlv_bytes(TLVType::PoolSwap, second_swap.to_bytes())
        .build()
        .expect("Failed to build message");

    // Parse and verify state progression
    let extensions = parse_tlv_extensions(&message.payload).expect("Failed to parse");
    assert_eq!(extensions.len(), 3);

    // Pool state should progress: initial swap -> mint increases liquidity -> second swap shows new state
    let mut swap_count = 0;
    for ext in extensions {
        if let TLVExtensionEnum::Standard(tlv) = ext {
            if tlv.header.tlv_type == TLVType::PoolSwap as u8 {
                let decoded = PoolSwapTLV::from_bytes(&tlv.payload).unwrap();
                swap_count += 1;

                if swap_count == 1 {
                    // Initial state
                    assert_eq!(decoded.liquidity_after, 1000000_00000000);
                    assert_eq!(decoded.tick_after, 0);
                } else if swap_count == 2 {
                    // After mint
                    assert_eq!(decoded.liquidity_after, 1000500_00000000);
                    assert_eq!(decoded.tick_after, 10);
                    assert!(decoded.sqrt_price_x96_after > initial_swap.sqrt_price_x96_after);
                }
            }
        }
    }

    assert_eq!(swap_count, 2, "Expected 2 swap events");
}

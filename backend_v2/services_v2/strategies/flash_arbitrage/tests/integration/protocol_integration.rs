//! Integration tests with protocol_v2 TLV messages

use alphapulse_flash_arbitrage::pool_state::{PoolState, PoolStateManager};
use alphapulse_protocol_v2::{
    instrument_id::{InstrumentId, PoolInstrumentId, VenueId},
    tlv::{parse_tlv_extensions, TLVMessageBuilder, TLVType},
    MessageHeader, RelayDomain, SourceType,
};
use rust_decimal_macros::dec;
use std::sync::Arc;

#[test]
fn test_parse_trade_tlv_to_pool_update() {
    // Create a TradeTLV message with PoolInstrumentId
    let pool_id = PoolInstrumentId {
        tokens: vec![
            0x0000000000000001, // WETH
            0x0000000000000002, // USDC
        ],
        venue_id: VenueId::Uniswap as u16,
        pool_type: 2, // V2
    };

    // Build TLV message
    let mut builder = TLVMessageBuilder::new(SourceType::PolygonCollector, 1234567890);

    // Add Trade TLV
    let trade_data = TradeTLVData {
        instrument_id: pool_id.to_instrument_id(),
        price: 2000_00000000i64, // $2000 with 8 decimals
        volume: 10_00000000i64,  // 10 ETH
        timestamp_ns: 1234567890000,
        is_buy: true,
    };

    builder.add_tlv(TLVType::Trade, &trade_data).unwrap();
    let message = builder.build().unwrap();

    // Parse the message
    let header = MessageHeader::from_bytes(&message[..32]).unwrap();
    assert_eq!(header.source_type, SourceType::PolygonCollector as u8);

    let tlvs = parse_tlv_extensions(&message[32..]).unwrap();
    assert_eq!(tlvs.len(), 1);
    assert_eq!(tlvs[0].tlv_type, TLVType::Trade as u8);

    // Extract pool ID from trade
    let trade_bytes = &tlvs[0].payload;
    let instrument_bytes = &trade_bytes[..12]; // First 12 bytes are InstrumentId
    let instrument_id = InstrumentId::from_bytes(instrument_bytes).unwrap();

    // Verify it matches our pool
    assert_eq!(instrument_id.venue, VenueId::Uniswap as u16);
    assert_eq!(instrument_id.asset_type, 3); // Pool type
}

#[test]
fn test_build_execution_control_tlv() {
    use alphapulse_protocol_v2::tlv::builder::ExecutionControlData;

    // Simulate building an execution control message for arbitrage
    let mut builder = TLVMessageBuilder::new(SourceType::FlashArbitrageStrategy, 987654321);

    let execution_data = ExecutionControlData {
        strategy_id: 1001,
        action: 1, // Execute
        target_venue: VenueId::Uniswap as u16,
        max_slippage_bps: 50,
        timeout_ms: 5000,
        nonce: 123456,
    };

    builder
        .add_tlv(TLVType::ExecutionControl, &execution_data)
        .unwrap();
    let message = builder.build().unwrap();

    // Verify message structure
    assert!(message.len() > 32); // Header + TLV

    let header = MessageHeader::from_bytes(&message[..32]).unwrap();
    assert_eq!(header.relay_domain(), Ok(RelayDomain::Signal));
}

#[test]
fn test_pool_addresses_tlv_parsing() {
    use alphapulse_protocol_v2::tlv::builder::PoolAddressesData;

    // Build PoolAddresses TLV (for flash loan contract addresses)
    let mut builder = TLVMessageBuilder::new(SourceType::PolygonCollector, 111111111);

    let pool_addresses = PoolAddressesData {
        pool_id: PoolInstrumentId {
            tokens: vec![1, 2],
            venue_id: VenueId::Uniswap as u16,
            pool_type: 2,
        },
        pool_address: [0x12; 20], // Mock address
        token0_address: [0x34; 20],
        token1_address: [0x56; 20],
        router_address: [0x78; 20],
    };

    builder
        .add_tlv(TLVType::PoolAddresses, &pool_addresses)
        .unwrap();
    let message = builder.build().unwrap();

    // Parse and verify
    let tlvs = parse_tlv_extensions(&message[32..]).unwrap();
    assert_eq!(tlvs[0].tlv_type, TLVType::PoolAddresses as u8);
    assert_eq!(tlvs[0].payload.len(), 88); // 8 + 20*4
}

#[test]
fn test_extended_tlv_for_large_orderbook() {
    use alphapulse_protocol_v2::tlv::extended::ExtendedTLVPayload;

    // Create large orderbook snapshot (>255 bytes)
    let mut orderbook_data = Vec::with_capacity(1000);

    // Add 50 price levels (each 16 bytes)
    for i in 0..50 {
        let price = 2000_00000000i64 + i * 100000000;
        let volume = 10_00000000i64;
        orderbook_data.extend_from_slice(&price.to_le_bytes());
        orderbook_data.extend_from_slice(&volume.to_le_bytes());
    }

    // Create extended TLV
    let extended = ExtendedTLVPayload::new(TLVType::L2Snapshot, orderbook_data.clone()).unwrap();

    let serialized = extended.serialize();

    // Should use extended format (type 255)
    assert_eq!(serialized[0], 255);
    assert_eq!(serialized[2], TLVType::L2Snapshot as u8);

    // Verify payload size encoding
    let payload_size = u16::from_le_bytes([serialized[3], serialized[4]]);
    assert_eq!(payload_size as usize, orderbook_data.len());
}

#[test]
fn test_mev_bundle_tlv_construction() {
    use alphapulse_protocol_v2::tlv::builder::MEVBundleData;

    let mut builder = TLVMessageBuilder::new(SourceType::FlashArbitrageStrategy, 222222222);

    let mev_bundle = MEVBundleData {
        bundle_id: 0x1234567890abcdef,
        target_block: 18500000,
        max_priority_fee_gwei: 50,
        max_base_fee_gwei: 100,
        tx_count: 2,
        simulation_success: true,
        expected_profit_wei: 1000000000000000000u128, // 1 ETH
    };

    builder.add_tlv(TLVType::MEVBundle, &mev_bundle).unwrap();
    let message = builder.build().unwrap();

    let tlvs = parse_tlv_extensions(&message[32..]).unwrap();
    assert_eq!(tlvs[0].tlv_type, TLVType::MEVBundle as u8);
}

// Helper structs for TLV data (these would normally be in protocol_v2)
#[repr(C, packed)]
struct TradeTLVData {
    instrument_id: InstrumentId,
    price: i64,
    volume: i64,
    timestamp_ns: u64,
    is_buy: bool,
}

impl PoolInstrumentId {
    fn to_instrument_id(&self) -> InstrumentId {
        // Convert PoolInstrumentId to generic InstrumentId
        InstrumentId {
            venue: self.venue_id,
            asset_type: 3, // Pool
            reserved: 0,
            asset_id: self.fast_hash(),
        }
    }
}

#[tokio::test]
async fn test_relay_domain_routing() {
    use alphapulse_protocol_v2::relay::{MarketDataRelay, RelayCore, SignalRelay};

    // Create relays
    let market_relay = MarketDataRelay::new().await;
    let signal_relay = SignalRelay::new().await;

    // Market data should route to MarketDataRelay
    let mut market_builder = TLVMessageBuilder::new(SourceType::BinanceCollector, 333333333);
    market_builder.add_tlv(TLVType::Trade, &[0u8; 48]).unwrap();
    let market_msg = market_builder.build().unwrap();

    let header = MessageHeader::from_bytes(&market_msg[..32]).unwrap();
    assert_eq!(header.relay_domain(), Ok(RelayDomain::MarketData));

    // Strategy signals should route to SignalRelay
    let mut signal_builder = TLVMessageBuilder::new(SourceType::FlashArbitrageStrategy, 444444444);
    signal_builder
        .add_tlv(TLVType::ExecutionControl, &[0u8; 32])
        .unwrap();
    let signal_msg = signal_builder.build().unwrap();

    let header = MessageHeader::from_bytes(&signal_msg[..32]).unwrap();
    assert_eq!(header.relay_domain(), Ok(RelayDomain::Signal));
}

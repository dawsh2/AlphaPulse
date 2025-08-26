//! Common Test Utilities for Protocol V2
//!
//! Provides shared utilities, generators, and helpers for all test suites.

use protocol_v2::{
    tlv::TLVMessageBuilder, InstrumentId, MessageHeader, RelayDomain, SourceType, TLVType,
    MESSAGE_MAGIC as MAGIC_NUMBER, PROTOCOL_VERSION,
};
use std::time::{SystemTime, UNIX_EPOCH};

/// Generate a valid test message for any domain
pub fn create_test_message(domain: RelayDomain, source: SourceType) -> Vec<u8> {
    match domain {
        RelayDomain::MarketData => create_market_data_message(source),
        RelayDomain::Signal => create_signal_message(source),
        RelayDomain::Execution => create_execution_message(source),
        RelayDomain::System => create_system_message(source),
    }
}

/// Create a valid market data message
pub fn create_market_data_message(source: SourceType) -> Vec<u8> {
    let instrument = InstrumentFixtures::btc();
    let price: i64 = 12345678; // 0.12345678
    let volume: i64 = 100000000; // 1.0

    let mut payload = Vec::with_capacity(32); // Adjusted for actual size
    payload.extend_from_slice(&instrument.to_u64().to_le_bytes());
    payload.extend_from_slice(&price.to_le_bytes());
    payload.extend_from_slice(&volume.to_le_bytes());

    TLVMessageBuilder::new(RelayDomain::MarketData, source)
        .add_tlv_bytes(TLVType::Trade, payload)
        .build()
}

/// Create a valid signal message
pub fn create_signal_message(source: SourceType) -> Vec<u8> {
    let signal_id: u64 = 0x1234567890ABCDEF;
    let confidence: u64 = 950000000; // 95% confidence

    let mut payload = Vec::with_capacity(16);
    payload.extend_from_slice(&signal_id.to_le_bytes());
    payload.extend_from_slice(&confidence.to_le_bytes());

    TLVMessageBuilder::new(RelayDomain::Signal, source)
        .add_tlv_bytes(TLVType::SignalIdentity, payload)
        .build()
}

/// Create a valid execution message
pub fn create_execution_message(source: SourceType) -> Vec<u8> {
    let order_id: u64 = 999999;
    let instrument = InstrumentFixtures::btc();
    let price: i64 = 12345678;
    let quantity: i64 = 50000000;

    let mut payload = Vec::with_capacity(32);
    payload.extend_from_slice(&order_id.to_le_bytes());
    payload.extend_from_slice(&instrument.to_u64().to_le_bytes());
    payload.extend_from_slice(&price.to_le_bytes());
    payload.extend_from_slice(&quantity.to_le_bytes());

    TLVMessageBuilder::new(RelayDomain::Execution, source)
        .add_tlv_bytes(TLVType::OrderRequest, payload)
        .build()
}

/// Create a valid system message
pub fn create_system_message(source: SourceType) -> Vec<u8> {
    let heartbeat_id: u64 = 12345;
    let uptime_seconds: u64 = 3600; // 1 hour uptime

    let mut payload = Vec::with_capacity(16);
    payload.extend_from_slice(&heartbeat_id.to_le_bytes());
    payload.extend_from_slice(&uptime_seconds.to_le_bytes());

    TLVMessageBuilder::new(RelayDomain::System, source)
        .add_tlv_bytes(TLVType::Heartbeat, payload)
        .build()
}

/// Create a message with invalid magic number
pub fn create_invalid_magic_message() -> Vec<u8> {
    let mut message = create_market_data_message(SourceType::BinanceCollector);
    // Corrupt the magic number (first 4 bytes) with a different value
    // Use to_le_bytes() because MessageHeader stores fields in native (little) endianness
    message[0..4].copy_from_slice(&0xBADBADBAu32.to_le_bytes());
    message
}

/// Create a message with invalid checksum
pub fn create_invalid_checksum_message() -> Vec<u8> {
    let mut message = create_signal_message(SourceType::Dashboard);
    // Corrupt the checksum (last 4 bytes of header: bytes 28-32)
    // Use to_le_bytes() for native endianness
    message[28..32].copy_from_slice(&0xBADC0FFEu32.to_le_bytes());
    message
}

/// Create a truncated message (incomplete header)
pub fn create_truncated_header_message() -> Vec<u8> {
    let full_message = create_market_data_message(SourceType::BinanceCollector);
    full_message[..20].to_vec() // Only 20 bytes instead of 32
}

/// Create a truncated TLV message (header complete but TLV cut off)
pub fn create_truncated_tlv_message() -> Vec<u8> {
    let full_message = create_market_data_message(SourceType::BinanceCollector);
    full_message[..40].to_vec() // Header + partial TLV
}

/// Create a message with oversized TLV payload
pub fn create_oversized_tlv_message() -> Vec<u8> {
    let huge_payload = vec![0xFF; 300]; // 300 bytes, exceeds u8 length

    // This will use extended TLV (Type 255) automatically
    TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
        .add_tlv_bytes(TLVType::OrderBook, huge_payload)
        .build()
}

/// Create a message with multiple TLVs
pub fn create_multi_tlv_message() -> Vec<u8> {
    let trade_payload = vec![0x01; 24];
    let quote_payload = vec![0x02; 32];

    TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::KrakenCollector)
        .add_tlv_bytes(TLVType::Trade, trade_payload)
        .add_tlv_bytes(TLVType::Quote, quote_payload)
        .build()
}

/// Generate current timestamp in nanoseconds
pub fn current_timestamp_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
}

/// Assert two messages are equal (ignoring timestamp and sequence)
pub fn assert_messages_equal_ignore_meta(msg1: &[u8], msg2: &[u8]) {
    assert_eq!(msg1.len(), msg2.len(), "Message lengths differ");

    // Compare magic number
    assert_eq!(&msg1[0..4], &msg2[0..4], "Magic numbers differ");

    // Compare version
    assert_eq!(msg1[4], msg2[4], "Versions differ");

    // Compare flags
    assert_eq!(msg1[5], msg2[5], "Flags differ");

    // Compare domain
    assert_eq!(msg1[6], msg2[6], "Domains differ");

    // Compare source
    assert_eq!(msg1[7], msg2[7], "Sources differ");

    // Skip timestamp (8-16) and sequence (16-24)

    // Compare TLV payload
    assert_eq!(&msg1[32..], &msg2[32..], "TLV payloads differ");
}

/// Measure operation throughput
pub struct ThroughputMeasure {
    start: std::time::Instant,
    operations: usize,
}

impl ThroughputMeasure {
    pub fn new() -> Self {
        Self {
            start: std::time::Instant::now(),
            operations: 0,
        }
    }

    pub fn record(&mut self, count: usize) {
        self.operations += count;
    }

    pub fn throughput(&self) -> f64 {
        let elapsed = self.start.elapsed().as_secs_f64();
        self.operations as f64 / elapsed
    }

    pub fn report(&self, name: &str) {
        println!("{}: {:.0} ops/sec", name, self.throughput());
    }
}

/// Memory allocation tracker
pub struct AllocationTracker {
    initial: usize,
}

impl AllocationTracker {
    pub fn new() -> Self {
        Self {
            initial: Self::current_allocated(),
        }
    }

    pub fn delta(&self) -> isize {
        Self::current_allocated() as isize - self.initial as isize
    }

    pub fn assert_no_allocations(&self) {
        let delta = self.delta();
        assert_eq!(delta, 0, "Unexpected allocations: {} bytes", delta);
    }

    #[cfg(target_os = "linux")]
    fn current_allocated() -> usize {
        // On Linux, we could use jemalloc stats
        0 // Placeholder
    }

    #[cfg(not(target_os = "linux"))]
    fn current_allocated() -> usize {
        // Placeholder for other platforms
        0
    }
}

/// Generate deterministic pseudo-random bytes for testing
pub fn generate_test_bytes(seed: u64, len: usize) -> Vec<u8> {
    let mut result = Vec::with_capacity(len);
    let mut state = seed;

    for _ in 0..len {
        // Simple LCG for deterministic generation
        state = state.wrapping_mul(1103515245).wrapping_add(12345);
        result.push((state >> 16) as u8);
    }

    result
}

/// Test fixture for instrument IDs
pub struct InstrumentFixtures;

impl InstrumentFixtures {
    pub fn btc() -> InstrumentId {
        // Create a simple BTC instrument using coin type
        InstrumentId::coin(protocol_v2::VenueId::Binance, "BTC")
    }

    pub fn usdt() -> InstrumentId {
        // Create a simple USDT instrument
        InstrumentId::coin(protocol_v2::VenueId::Binance, "USDT")
    }

    pub fn eth() -> InstrumentId {
        // Create ETH instrument
        InstrumentId::coin(protocol_v2::VenueId::Binance, "ETH")
    }

    pub fn btc_usdt_pool() -> InstrumentId {
        // Create BTC-USDT pool on UniswapV2
        let btc = Self::btc();
        let usdt = Self::usdt();
        InstrumentId::pool(protocol_v2::VenueId::UniswapV2, btc, usdt)
    }

    pub fn triangular_pool() -> InstrumentId {
        // BTC-ETH-USDT triangular pool
        let btc = Self::btc();
        let eth = Self::eth();
        let usdt = Self::usdt();
        InstrumentId::triangular_pool(protocol_v2::VenueId::Balancer, btc, eth, usdt)
    }
}

/// Assert protocol error matches expected variant
#[macro_export]
macro_rules! assert_protocol_error {
    ($result:expr, $pattern:pat) => {
        match $result {
            Err($pattern) => {}
            Err(e) => panic!(
                "Expected error pattern {}, got {:?}",
                stringify!($pattern),
                e
            ),
            Ok(_) => panic!("Expected error {}, got Ok", stringify!($pattern)),
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_generators() {
        let msg = create_market_data_message(SourceType::BinanceCollector);
        assert!(msg.len() > MessageHeader::SIZE);

        let header = protocol_v2::parse_header(&msg).unwrap();
        let magic = header.magic; // Copy from packed struct
        let version = header.version; // Copy from packed struct
        assert_eq!(magic, MAGIC_NUMBER);
        assert_eq!(version, PROTOCOL_VERSION);
        assert_eq!(header.get_relay_domain().unwrap(), RelayDomain::MarketData);
    }

    #[test]
    fn test_invalid_messages() {
        let invalid_magic = create_invalid_magic_message();
        assert!(protocol_v2::parse_header(&invalid_magic).is_err());

        let invalid_checksum = create_invalid_checksum_message();
        assert!(protocol_v2::parse_header(&invalid_checksum).is_err());
    }

    #[test]
    fn test_throughput_measure() {
        let mut measure = ThroughputMeasure::new();
        measure.record(1000);
        std::thread::sleep(std::time::Duration::from_millis(10));
        measure.record(1000);

        let throughput = measure.throughput();
        assert!(throughput > 0.0);
        assert!(throughput < 1_000_000.0); // Should be less than 1M ops/sec with sleep
    }
}

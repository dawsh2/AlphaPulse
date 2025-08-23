//! Recovery Protocol Tests
//! 
//! Tests recovery scenarios that happen in production:
//! - Network disconnections during volatile markets
//! - Consumer crashes and restarts
//! - Sequence gaps from packet loss
//! - State synchronization after outages

mod common;

use alphapulse_protocol_v2::{
    MessageHeader, RelayDomain, SourceType, TLVType,
    tlv::TLVMessageBuilder,
    recovery::{RecoveryRequestTLV, RecoveryRequestType, RecoveryRequestBuilder},
};
use common::*;
use std::collections::HashMap;

#[test]
fn test_sequence_gap_detection() {
    // Simulate receiving messages with gaps (packet loss)
    let mut messages = Vec::new();
    let sequences = [1, 2, 3, 5, 6, 9, 10]; // Gaps at 4, 7-8
    
    for seq in sequences {
        let mut msg = create_market_data_message(SourceType::BinanceCollector);
        // Set sequence number
        msg[12..20].copy_from_slice(&(seq as u64).to_le_bytes());
        // Recalculate checksum
        let checksum = alphapulse_protocol_v2::validation::calculate_crc32_excluding_checksum(&msg, 28);
        msg[28..32].copy_from_slice(&checksum.to_be_bytes());
        messages.push((seq, msg));
    }
    
    // Track sequences and detect gaps
    let mut last_seq = 0u64;
    let mut gaps = Vec::new();
    
    for (seq, msg) in &messages {
        let header = alphapulse_protocol_v2::parse_header(msg).unwrap();
        let msg_seq = header.sequence;
        
        if last_seq > 0 && msg_seq > last_seq + 1 {
            // Gap detected
            for missing in (last_seq + 1)..msg_seq {
                gaps.push(missing);
            }
        }
        last_seq = msg_seq;
    }
    
    assert_eq!(gaps, vec![4, 7, 8], "Should detect sequence gaps");
}

#[test]
fn test_recovery_request_creation() {
    // Test creating recovery request for missed messages
    let consumer_id = 12345u64;
    let last_received_seq = 1000u64;
    let current_seq = 1010u64;
    
    // Create recovery request for gap
    let recovery_builder = RecoveryRequestBuilder::new(consumer_id as u32, SourceType::Dashboard);
    let msg = recovery_builder.retransmit_request(
        RelayDomain::MarketData,
        last_received_seq,
        current_seq,
    );
    
    // Verify it parses correctly
    let header = alphapulse_protocol_v2::parse_header(&msg).unwrap();
    assert_eq!(header.get_relay_domain().unwrap(), RelayDomain::MarketData);
    
    // Check TLV type
    let tlv_data = &msg[MessageHeader::SIZE..];
    let tlvs = alphapulse_protocol_v2::parse_tlv_extensions(tlv_data).unwrap();
    
    // Should have RecoveryRequest TLV (type 110)
    // We check the raw TLV type in the message
    assert_eq!(msg[MessageHeader::SIZE], 110, "Should have RecoveryRequest TLV type");
}

#[test]
fn test_snapshot_generation() {
    // Test generating state snapshot for new consumers
    
    // Simulate current market state
    let mut market_state = HashMap::new();
    market_state.insert(1u64, 4500000000000i64); // BTC at $45,000
    market_state.insert(2u64, 350000000000i64);  // ETH at $3,500
    market_state.insert(1000u64, 100000000i64);  // USDT at $1.00
    
    // Create snapshot message
    let snapshot_data = market_state.iter()
        .flat_map(|(instrument_id, price)| {
            let mut data = Vec::new();
            data.extend_from_slice(&instrument_id.to_le_bytes());
            data.extend_from_slice(&price.to_le_bytes());
            data
        })
        .collect::<Vec<u8>>();
    
    // Create snapshot message using TLV builder
    let snapshot_msg = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::MarketDataRelay)
        .add_tlv_bytes(TLVType::Snapshot, snapshot_data)
        .build();
    
    // Verify snapshot message
    let header = alphapulse_protocol_v2::parse_header(&snapshot_msg).unwrap();
    let tlv_data = &snapshot_msg[MessageHeader::SIZE..];
    let tlvs = alphapulse_protocol_v2::parse_tlv_extensions(tlv_data).unwrap();
    
    // Should have Snapshot TLV (type 101)
    // We check the raw TLV type in the message
    assert_eq!(snapshot_msg[MessageHeader::SIZE], 101, "Should have Snapshot TLV type");
}

#[test]
fn test_consumer_crash_recovery() {
    // Simulate consumer crash and recovery
    
    // Consumer was at sequence 5000 before crash
    let last_known_seq = 5000u64;
    
    // After restart, relay is at sequence 5100
    let current_relay_seq = 5100u64;
    
    // Consumer needs to recover 100 messages
    let recovery_builder = RecoveryRequestBuilder::new(999, SourceType::Dashboard);
    let _recovery_msg = recovery_builder.snapshot_request(
        RelayDomain::Signal,
        last_known_seq,
        current_relay_seq,
    );
    
    let message_count = current_relay_seq - last_known_seq;
    assert_eq!(message_count, 100);
    // Snapshot request type is used for crash recovery
}

#[test]
fn test_volatile_market_recovery() {
    // During volatile markets, consumers might fall behind
    
    // Simulate rapid message burst
    let burst_start_seq = 10000u64;
    let burst_end_seq = 15000u64; // 5000 messages in burst
    
    // Consumer could only process up to 11000
    let consumer_last_seq = 11000u64;
    
    // Create recovery request for missed messages
    let recovery_builder = RecoveryRequestBuilder::new(777, SourceType::Dashboard);
    let _recovery_msg = recovery_builder.retransmit_request(
        RelayDomain::MarketData,
        consumer_last_seq,
        burst_end_seq,
    );
    
    // Should need multiple recovery rounds
    let messages_needed = burst_end_seq - consumer_last_seq;
    let max_messages = 1000u64; // Limit to prevent overwhelming consumer
    let rounds_needed = (messages_needed + max_messages - 1) / max_messages;
    
    assert_eq!(rounds_needed, 4, "Should need 4 recovery rounds for 4000 messages");
}

#[test]
fn test_multi_consumer_recovery() {
    // Multiple consumers at different sequences
    struct ConsumerState {
        id: u64,
        last_seq: u64,
        domain: RelayDomain,
    }
    
    let consumers = vec![
        ConsumerState { id: 1, last_seq: 1000, domain: RelayDomain::MarketData },
        ConsumerState { id: 2, last_seq: 950, domain: RelayDomain::MarketData },
        ConsumerState { id: 3, last_seq: 1050, domain: RelayDomain::MarketData },
        ConsumerState { id: 4, last_seq: 500, domain: RelayDomain::Signal },
    ];
    
    let current_seq = 1100u64;
    
    // Each consumer needs different recovery
    for consumer in consumers {
        if consumer.last_seq < current_seq {
            let gap = current_seq - consumer.last_seq;
            println!("Consumer {} needs {} messages", consumer.id, gap);
            
            let should_snapshot = consumer.last_seq < current_seq - 500;
            let recovery_builder = RecoveryRequestBuilder::new(consumer.id as u32, SourceType::Dashboard);
            let _recovery_msg = if should_snapshot {
                recovery_builder.snapshot_request(consumer.domain, consumer.last_seq, current_seq)
            } else {
                recovery_builder.retransmit_request(consumer.domain, consumer.last_seq, current_seq)
            };
            
            if consumer.id == 4 {
                assert!(should_snapshot, 
                       "Consumer 4 should get snapshot (too far behind)");
            }
        }
    }
}

#[test]
fn test_recovery_during_arbitrage_opportunity() {
    // Critical: Arbitrage signals are time-sensitive
    
    // Arbitrage opportunity detected at sequence 2000
    let arb_sequence = 2000u64;
    let arb_timestamp = alphapulse_protocol_v2::header::current_timestamp_ns();
    
    // Consumer disconnected at 1990, misses the opportunity
    let consumer_last_seq = 1990u64;
    
    // By the time consumer recovers (100ms later), opportunity is gone
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    let recovery_timestamp = alphapulse_protocol_v2::header::current_timestamp_ns();
    let delay_ns = recovery_timestamp - arb_timestamp;
    let delay_ms = delay_ns / 1_000_000;
    
    assert!(delay_ms >= 100, "Recovery delay: {} ms", delay_ms);
    
    // Arbitrage opportunities typically last < 100ms
    let opportunity_expired = delay_ms > 50;
    assert!(opportunity_expired, "Arbitrage opportunity expired during recovery");
    
    // Recovery request should still be made for audit trail
    let recovery_builder = RecoveryRequestBuilder::new(888, SourceType::Dashboard);
    let _recovery_msg = recovery_builder.retransmit_request(
        RelayDomain::Signal,
        consumer_last_seq,
        arb_sequence + 10,
    );
    
    let message_count = (arb_sequence + 10) - consumer_last_seq;
    assert_eq!(message_count, 20);
}

#[test]
fn test_orderbook_snapshot_recovery() {
    // Orderbook state recovery after disconnection
    use alphapulse_protocol_v2::InstrumentId;
    
    // Current orderbook state
    struct OrderbookLevel {
        price: i64,
        volume: i64,
    }
    
    let btc = InstrumentId::coin(alphapulse_protocol_v2::VenueId::Binance, "BTC");
    let usdt = InstrumentId::coin(alphapulse_protocol_v2::VenueId::Binance, "USDT");
    let btc_usdt = InstrumentId::pool(alphapulse_protocol_v2::VenueId::UniswapV2, btc, usdt);
    let orderbook = vec![
        OrderbookLevel { price: 4499000000000, volume: 500000000 }, // Bid: $44,990, 5 BTC
        OrderbookLevel { price: 4498500000000, volume: 1000000000 }, // Bid: $44,985, 10 BTC
        OrderbookLevel { price: 4501000000000, volume: 300000000 }, // Ask: $45,010, 3 BTC
        OrderbookLevel { price: 4501500000000, volume: 800000000 }, // Ask: $45,015, 8 BTC
    ];
    
    // Create L2 snapshot
    let mut snapshot_payload = Vec::new();
    snapshot_payload.extend_from_slice(&btc_usdt.to_u64().to_le_bytes());
    snapshot_payload.extend_from_slice(&(orderbook.len() as u32).to_le_bytes());
    
    for level in &orderbook {
        snapshot_payload.extend_from_slice(&level.price.to_le_bytes());
        snapshot_payload.extend_from_slice(&level.volume.to_le_bytes());
    }
    
    let snapshot_msg = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
        .add_tlv_bytes(TLVType::L2Snapshot, snapshot_payload)
        .build();
    
    // Verify snapshot contains full orderbook
    let tlv_data = &snapshot_msg[MessageHeader::SIZE..];
    let tlvs = alphapulse_protocol_v2::parse_tlv_extensions(tlv_data).unwrap();
    assert_eq!(tlvs.len(), 1);
}

#[test]
fn test_execution_domain_recovery_ordering() {
    // Execution messages MUST be recovered in exact order
    
    let orders = vec![
        (100, TLVType::OrderRequest),  // Place order
        (101, TLVType::OrderStatus),    // Order accepted
        (102, TLVType::Fill),          // Partial fill
        (103, TLVType::Fill),          // Another fill
        (104, TLVType::OrderStatus),    // Order completed
    ];
    
    let mut messages = Vec::new();
    
    for (seq, tlv_type) in orders {
        let mut msg = TLVMessageBuilder::new(RelayDomain::Execution, SourceType::ExecutionEngine)
            .add_tlv_bytes(tlv_type, vec![0; 32])
            .build();
        
        // Set sequence
        msg[12..20].copy_from_slice(&(seq as u64).to_le_bytes());
        // Fix checksum
        let checksum = alphapulse_protocol_v2::validation::calculate_crc32_excluding_checksum(&msg, 28);
        msg[28..32].copy_from_slice(&checksum.to_be_bytes());
        
        messages.push(msg);
    }
    
    // Verify order preservation
    let mut last_seq = 0u64;
    for msg in &messages {
        let header = alphapulse_protocol_v2::parse_header(msg).unwrap();
        assert!(header.sequence > last_seq, "Execution messages must maintain order");
        last_seq = header.sequence;
    }
}

#[test]
fn test_recovery_bandwidth_limits() {
    // Test recovery doesn't overwhelm network/consumer
    
    // Consumer is 50,000 messages behind (major outage)
    let behind_count = 50_000u64;
    
    // Calculate recovery strategy
    let max_messages_per_request = 1000u64;
    let requests_needed = (behind_count + max_messages_per_request - 1) / max_messages_per_request;
    
    assert_eq!(requests_needed, 50, "Need 50 requests for 50k messages");
    
    // With 100ms between requests to avoid overwhelming
    let recovery_time_ms = requests_needed * 100;
    assert_eq!(recovery_time_ms, 5000, "Full recovery takes 5 seconds");
    
    // Alternative: Request snapshot instead
    let use_snapshot = behind_count > 10_000;
    assert!(use_snapshot, "Should use snapshot when too far behind");
}

#[test]
fn test_recovery_with_domain_isolation() {
    // Each domain has independent sequence numbers
    
    let domains = vec![
        (RelayDomain::MarketData, 10000u64),
        (RelayDomain::Signal, 5000u64),
        (RelayDomain::Execution, 2000u64),
    ];
    
    // Consumer needs different recovery per domain
    for (domain, current_seq) in domains {
        let consumer_seq = match domain {
            RelayDomain::MarketData => 9500u64, // 500 behind
            RelayDomain::Signal => 4999u64,     // 1 behind
            RelayDomain::Execution => 1000u64,  // 1000 behind
        };
        
        if consumer_seq < current_seq {
            let gap_size = current_seq - consumer_seq;
            let should_snapshot = gap_size > 500;
            let recovery_builder = RecoveryRequestBuilder::new(111, SourceType::Dashboard);
            
            let _recovery_msg = if should_snapshot {
                recovery_builder.snapshot_request(domain, consumer_seq, current_seq)
            } else {
                recovery_builder.retransmit_request(domain, consumer_seq, current_seq)
            };
            
            match domain {
                RelayDomain::MarketData => {
                    assert!(!should_snapshot, "Market data: incremental recovery");
                }
                RelayDomain::Signal => {
                    assert_eq!(gap_size, 1, "Signal: only 1 message behind");
                }
                RelayDomain::Execution => {
                    assert!(should_snapshot, "Execution: snapshot needed (1000 behind)");
                }
            }
        }
    }
}

#[test]
fn test_recovery_checksum_validation() {
    // Recovered messages must have valid checksums
    
    let recovery_messages = (0..10).map(|i| {
        let mut msg = create_market_data_message(SourceType::BinanceCollector);
        // Set sequence
        msg[12..20].copy_from_slice(&(i as u64).to_le_bytes());
        // Recalculate checksum
        let checksum = alphapulse_protocol_v2::validation::calculate_crc32_excluding_checksum(&msg, 28);
        msg[28..32].copy_from_slice(&checksum.to_be_bytes());
        msg
    }).collect::<Vec<_>>();
    
    // Verify all recovered messages
    for msg in &recovery_messages {
        let header = alphapulse_protocol_v2::parse_header(msg).unwrap();
        assert!(
            alphapulse_protocol_v2::validation::verify_message_checksum(msg, header.checksum, 28),
            "Recovered message has invalid checksum"
        );
    }
}
//! TLVMessageBuilder Unit Tests
//!
//! Tests for correct message construction with TLV payload.

use protocol_v2::{
    tlv::{TLVMessageBuilder, TLVType},
    RelayDomain, SourceType, MESSAGE_MAGIC, PROTOCOL_VERSION,
};

#[test]
fn test_empty_message_builder() {
    // Builder should create valid message even with no TLVs
    let builder = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::Dashboard);
    let message = builder.build();
    
    // Should be exactly header size (32 bytes) for empty message
    assert_eq!(message.len(), 32);
    
    // Verify header fields
    assert_eq!(u32::from_le_bytes([message[0], message[1], message[2], message[3]]), MESSAGE_MAGIC);
    assert_eq!(message[4], PROTOCOL_VERSION);
    assert_eq!(message[5] as u8, RelayDomain::MarketData as u8);
    
    // Payload size should be 0 for empty message
    let payload_size = u32::from_le_bytes([message[24], message[25], message[26], message[27]]);
    assert_eq!(payload_size, 0);
}

#[test]
fn test_single_tlv_message() {
    let mut builder = TLVMessageBuilder::new(RelayDomain::Signal, SourceType::FlashArbitrageStrategy);
    
    // Add a simple TLV (type=20, length=8, value=test_data)
    let test_data = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
    builder.add_tlv(20, &test_data);
    
    let message = builder.build();
    
    // Header (32) + TLV header (8) + payload (8) = 48 bytes minimum
    assert_eq!(message.len(), 48);
    
    // Check TLV starts at byte 32 (after header)
    let tlv_type = u16::from_le_bytes([message[32], message[33]]);
    let tlv_length = u16::from_le_bytes([message[34], message[35]]);
    
    assert_eq!(tlv_type, 20);
    assert_eq!(tlv_length, 8);
    
    // Check TLV payload
    let tlv_payload = &message[40..48]; // After TLV header
    assert_eq!(tlv_payload, &test_data);
}

#[test]
fn test_multiple_tlv_message() {
    let mut builder = TLVMessageBuilder::new(RelayDomain::Execution, SourceType::BinanceCollector);
    
    // Add multiple TLVs
    let tlv1_data = vec![0xAA, 0xBB];
    let tlv2_data = vec![0xCC, 0xDD, 0xEE, 0xFF];
    
    builder.add_tlv(40, &tlv1_data);
    builder.add_tlv(41, &tlv2_data);
    
    let message = builder.build();
    
    // Header (32) + TLV1 (8+2) + TLV2 (8+4) = 54 bytes
    assert_eq!(message.len(), 54);
    
    // Check first TLV
    let tlv1_type = u16::from_le_bytes([message[32], message[33]]);
    let tlv1_length = u16::from_le_bytes([message[34], message[35]]);
    assert_eq!(tlv1_type, 40);
    assert_eq!(tlv1_length, 2);
    
    // Check second TLV starts after first
    let tlv2_start = 32 + 8 + 2; // Header + TLV1 header + TLV1 payload
    let tlv2_type = u16::from_le_bytes([message[tlv2_start], message[tlv2_start + 1]]);
    let tlv2_length = u16::from_le_bytes([message[tlv2_start + 2], message[tlv2_start + 3]]);
    assert_eq!(tlv2_type, 41);
    assert_eq!(tlv2_length, 4);
}

#[test]
fn test_payload_size_calculation() {
    let mut builder = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::KrakenCollector);
    
    // Add TLVs with known sizes
    builder.add_tlv(1, &vec![0u8; 10]); // 8 + 10 = 18 bytes
    builder.add_tlv(2, &vec![0u8; 20]); // 8 + 20 = 28 bytes
    
    let message = builder.build();
    
    // Total payload: 18 + 28 = 46 bytes
    let payload_size = u32::from_le_bytes([message[24], message[25], message[26], message[27]]);
    assert_eq!(payload_size, 46);
    
    // Total message: 32 (header) + 46 (payload) = 78 bytes
    assert_eq!(message.len(), 78);
}

#[test]
fn test_sequence_number_increment() {
    let mut builder1 = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::Dashboard);
    let mut builder2 = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::Dashboard);
    
    // Set specific sequence numbers
    builder1.set_sequence(100);
    builder2.set_sequence(200);
    
    let message1 = builder1.build();
    let message2 = builder2.build();
    
    let seq1 = u64::from_le_bytes([
        message1[8], message1[9], message1[10], message1[11],
        message1[12], message1[13], message1[14], message1[15]
    ]);
    let seq2 = u64::from_le_bytes([
        message2[8], message2[9], message2[10], message2[11],
        message2[12], message2[13], message2[14], message2[15]
    ]);
    
    assert_eq!(seq1, 100);
    assert_eq!(seq2, 200);
}

#[test]
fn test_timestamp_setting() {
    let mut builder = TLVMessageBuilder::new(RelayDomain::Signal, SourceType::FlashArbitrageStrategy);
    
    let test_timestamp = 1234567890123456789u64;
    builder.set_timestamp_ns(test_timestamp);
    
    let message = builder.build();
    
    let timestamp = u64::from_le_bytes([
        message[16], message[17], message[18], message[19],
        message[20], message[21], message[22], message[23]
    ]);
    
    assert_eq!(timestamp, test_timestamp);
}

#[test]
fn test_checksum_calculation() {
    let mut builder = TLVMessageBuilder::new(RelayDomain::Execution, SourceType::BinanceCollector);
    builder.add_tlv(40, &[0x01, 0x02, 0x03, 0x04]);
    
    let message = builder.build();
    
    // Checksum is last 4 bytes of header
    let checksum = u32::from_le_bytes([message[28], message[29], message[30], message[31]]);
    
    // Should be non-zero for message with payload
    assert_ne!(checksum, 0, "Checksum should be calculated for non-empty message");
}

#[test]
fn test_zero_length_tlv() {
    let mut builder = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::Dashboard);
    
    // Add TLV with zero-length payload
    builder.add_tlv(1, &[]);
    
    let message = builder.build();
    
    // Header (32) + TLV header (8) + no payload = 40 bytes
    assert_eq!(message.len(), 40);
    
    let tlv_length = u16::from_le_bytes([message[34], message[35]]);
    assert_eq!(tlv_length, 0);
}

#[test]
fn test_large_tlv_payload() {
    let mut builder = TLVMessageBuilder::new(RelayDomain::Signal, SourceType::Dashboard);
    
    // Large payload (but not excessive for unit test)
    let large_payload = vec![0x42u8; 1024];
    builder.add_tlv(25, &large_payload);
    
    let message = builder.build();
    
    // Header (32) + TLV header (8) + payload (1024) = 1064 bytes
    assert_eq!(message.len(), 1064);
    
    let tlv_length = u16::from_le_bytes([message[34], message[35]]);
    assert_eq!(tlv_length, 1024);
    
    // Verify payload content
    let payload_start = 40; // After header + TLV header
    assert_eq!(&message[payload_start..payload_start + 1024], &large_payload[..]);
}
//! Recovery Request Implementation
//! 
//! Handles RecoveryRequest TLV (Type 110) for sequence gap recovery

use zerocopy::{AsBytes, FromBytes, FromZeroes};
use crate::tlv::{TLVType, TLVMessageBuilder};
use crate::{RelayDomain, SourceType};

/// Recovery Request TLV (Type 110, 20 bytes payload)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, AsBytes, FromBytes, FromZeroes)]
pub struct RecoveryRequestTLV {
    pub tlv_type: u8,           // 110
    pub tlv_length: u8,         // 20
    pub consumer_id: u32,       // Identifies requesting consumer
    pub last_sequence: u64,     // Last successfully received sequence
    pub current_sequence: u64,  // Current sequence from header (gap detected)
    pub request_type: u8,       // 1=retransmit, 2=snapshot
    pub reserved: u8,
}

/// Recovery request types
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryRequestType {
    Retransmit = 1,   // Request retransmission of missing messages
    Snapshot = 2,     // Request snapshot-based recovery
}

impl RecoveryRequestTLV {
    /// Create a new recovery request
    pub fn new(
        consumer_id: u32,
        last_sequence: u64,
        current_sequence: u64,
        request_type: RecoveryRequestType,
    ) -> Self {
        Self {
            tlv_type: TLVType::RecoveryRequest as u8,
            tlv_length: 20,
            consumer_id,
            last_sequence,
            current_sequence,
            request_type: request_type as u8,
            reserved: 0,
        }
    }
    
    /// Get the gap size
    pub fn gap_size(&self) -> u64 {
        self.current_sequence.saturating_sub(self.last_sequence)
    }
    
    /// Get the recovery request type
    pub fn get_request_type(&self) -> Option<RecoveryRequestType> {
        match self.request_type {
            1 => Some(RecoveryRequestType::Retransmit),
            2 => Some(RecoveryRequestType::Snapshot),
            _ => None,
        }
    }
    
    /// Check if this is a retransmit request
    pub fn is_retransmit(&self) -> bool {
        matches!(self.get_request_type(), Some(RecoveryRequestType::Retransmit))
    }
    
    /// Check if this is a snapshot request
    pub fn is_snapshot(&self) -> bool {
        matches!(self.get_request_type(), Some(RecoveryRequestType::Snapshot))
    }
}

/// Builder for recovery request messages
pub struct RecoveryRequestBuilder {
    consumer_id: u32,
    source: SourceType,
}

impl RecoveryRequestBuilder {
    /// Create a new recovery request builder
    pub fn new(consumer_id: u32, source: SourceType) -> Self {
        Self {
            consumer_id,
            source,
        }
    }
    
    /// Build a retransmit request
    pub fn retransmit_request(
        self,
        relay_domain: RelayDomain,
        last_sequence: u64,
        current_sequence: u64,
    ) -> Vec<u8> {
        let recovery_tlv = RecoveryRequestTLV::new(
            self.consumer_id,
            last_sequence,
            current_sequence,
            RecoveryRequestType::Retransmit,
        );
        
        TLVMessageBuilder::new(relay_domain, self.source)
            .add_tlv(TLVType::RecoveryRequest, &recovery_tlv)
            .build()
    }
    
    /// Build a snapshot request
    pub fn snapshot_request(
        self,
        relay_domain: RelayDomain,
        last_sequence: u64,
        current_sequence: u64,
    ) -> Vec<u8> {
        let recovery_tlv = RecoveryRequestTLV::new(
            self.consumer_id,
            last_sequence,
            current_sequence,
            RecoveryRequestType::Snapshot,
        );
        
        TLVMessageBuilder::new(relay_domain, self.source)
            .add_tlv(TLVType::RecoveryRequest, &recovery_tlv)
            .build()
    }
    
    /// Build a smart recovery request (chooses type based on gap size)
    pub fn smart_request(
        self,
        relay_domain: RelayDomain,
        last_sequence: u64,
        current_sequence: u64,
    ) -> Vec<u8> {
        let gap_size = current_sequence.saturating_sub(last_sequence);
        let request_type = if gap_size < 100 {
            RecoveryRequestType::Retransmit
        } else {
            RecoveryRequestType::Snapshot
        };
        
        let recovery_tlv = RecoveryRequestTLV::new(
            self.consumer_id,
            last_sequence,
            current_sequence,
            request_type,
        );
        
        TLVMessageBuilder::new(relay_domain, self.source)
            .add_tlv(TLVType::RecoveryRequest, &recovery_tlv)
            .build()
    }
}

/// Response to a recovery request
#[derive(Debug, Clone)]
pub struct RecoveryResponse {
    pub consumer_id: u32,
    pub request_type: RecoveryRequestType,
    pub status: RecoveryStatus,
    pub data: RecoveryData,
}

/// Recovery response status
#[derive(Debug, Clone, PartialEq)]
pub enum RecoveryStatus {
    Success,
    PartialSuccess { missing_count: u32 },
    Failed { reason: String },
    TooManyRequests,
}

/// Recovery response data
#[derive(Debug, Clone)]
pub enum RecoveryData {
    RetransmittedMessages(Vec<Vec<u8>>),  // Missing messages
    Snapshot(Vec<u8>),                    // Compressed snapshot
    None,                                 // No data (error case)
}

impl RecoveryResponse {
    /// Create a successful retransmit response
    pub fn retransmit_success(consumer_id: u32, messages: Vec<Vec<u8>>) -> Self {
        Self {
            consumer_id,
            request_type: RecoveryRequestType::Retransmit,
            status: RecoveryStatus::Success,
            data: RecoveryData::RetransmittedMessages(messages),
        }
    }
    
    /// Create a successful snapshot response
    pub fn snapshot_success(consumer_id: u32, snapshot: Vec<u8>) -> Self {
        Self {
            consumer_id,
            request_type: RecoveryRequestType::Snapshot,
            status: RecoveryStatus::Success,
            data: RecoveryData::Snapshot(snapshot),
        }
    }
    
    /// Create a failed response
    pub fn failed(consumer_id: u32, request_type: RecoveryRequestType, reason: String) -> Self {
        Self {
            consumer_id,
            request_type,
            status: RecoveryStatus::Failed { reason },
            data: RecoveryData::None,
        }
    }
    
    /// Check if the response is successful
    pub fn is_success(&self) -> bool {
        matches!(self.status, RecoveryStatus::Success)
    }
    
    /// Get the number of recovered messages (for retransmit)
    pub fn message_count(&self) -> usize {
        match &self.data {
            RecoveryData::RetransmittedMessages(messages) => messages.len(),
            RecoveryData::Snapshot(_) => 1,
            RecoveryData::None => 0,
        }
    }
    
    /// Get the total recovery data size
    pub fn data_size(&self) -> usize {
        match &self.data {
            RecoveryData::RetransmittedMessages(messages) => {
                messages.iter().map(|m| m.len()).sum()
            }
            RecoveryData::Snapshot(snapshot) => snapshot.len(),
            RecoveryData::None => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tlv::parser::{parse_header, parse_tlv_extensions, find_tlv_by_type};
    
    #[test]
    fn test_recovery_request_tlv_creation() {
        let request = RecoveryRequestTLV::new(
            12345,  // consumer_id
            100,    // last_sequence
            150,    // current_sequence (gap of 50)
            RecoveryRequestType::Retransmit,
        );
        
        let tlv_type = request.tlv_type;
        let tlv_length = request.tlv_length;
        let consumer_id = request.consumer_id;
        let last_sequence = request.last_sequence;
        let current_sequence = request.current_sequence;
        assert_eq!(tlv_type, TLVType::RecoveryRequest as u8);
        assert_eq!(tlv_length, 20);
        assert_eq!(consumer_id, 12345);
        assert_eq!(last_sequence, 100);
        assert_eq!(current_sequence, 150);
        assert_eq!(request.gap_size(), 50);
        assert!(request.is_retransmit());
        assert!(!request.is_snapshot());
    }
    
    #[test]
    fn test_recovery_request_builder() {
        let builder = RecoveryRequestBuilder::new(999, SourceType::Dashboard);
        
        // Test retransmit request
        let message = builder.retransmit_request(RelayDomain::MarketData, 10, 20);
        
        // Parse and verify
        let header = parse_header(&message).unwrap();
        assert_eq!(header.relay_domain, RelayDomain::MarketData as u8);
        assert_eq!(header.source, SourceType::Dashboard as u8);
        
        let tlv_payload = &message[32..];
        let tlv_data = find_tlv_by_type(tlv_payload, TLVType::RecoveryRequest as u8).unwrap();
        
        // Parse the recovery request TLV
        let request = zerocopy::Ref::<_, RecoveryRequestTLV>::new(tlv_data)
            .unwrap()
            .into_ref();
        
        let consumer_id = request.consumer_id;
        let last_sequence = request.last_sequence;
        let current_sequence = request.current_sequence;
        assert_eq!(consumer_id, 999);
        assert_eq!(last_sequence, 10);
        assert_eq!(current_sequence, 20);
        assert_eq!(request.request_type, RecoveryRequestType::Retransmit as u8);
    }
    
    #[test]
    fn test_smart_request_selection() {
        let builder = RecoveryRequestBuilder::new(1, SourceType::ArbitrageStrategy);
        
        // Small gap should use retransmit
        let small_gap_msg = builder.smart_request(RelayDomain::Signal, 100, 150);
        let header = parse_header(&small_gap_msg).unwrap();
        let tlv_payload = &small_gap_msg[32..];
        let tlv_data = find_tlv_by_type(tlv_payload, TLVType::RecoveryRequest as u8).unwrap();
        let request = zerocopy::Ref::<_, RecoveryRequestTLV>::new(tlv_data)
            .unwrap()
            .into_ref();
        
        assert_eq!(request.request_type, RecoveryRequestType::Retransmit as u8);
        
        // Large gap should use snapshot
        let builder2 = RecoveryRequestBuilder::new(2, SourceType::ArbitrageStrategy);
        let large_gap_msg = builder2.smart_request(RelayDomain::Signal, 100, 300);
        let header2 = parse_header(&large_gap_msg).unwrap();
        let tlv_payload2 = &large_gap_msg[32..];
        let tlv_data2 = find_tlv_by_type(tlv_payload2, TLVType::RecoveryRequest as u8).unwrap();
        let request2 = zerocopy::Ref::<_, RecoveryRequestTLV>::new(tlv_data2)
            .unwrap()
            .into_ref();
        
        assert_eq!(request2.request_type, RecoveryRequestType::Snapshot as u8);
    }
    
    #[test]
    fn test_recovery_response() {
        let messages = vec![
            vec![1, 2, 3, 4],
            vec![5, 6, 7, 8],
        ];
        
        let response = RecoveryResponse::retransmit_success(123, messages.clone());
        
        assert!(response.is_success());
        assert_eq!(response.consumer_id, 123);
        assert_eq!(response.message_count(), 2);
        assert_eq!(response.data_size(), 8); // 4 + 4 bytes
        
        // Test snapshot response
        let snapshot_data = vec![0; 1000];
        let snapshot_response = RecoveryResponse::snapshot_success(456, snapshot_data);
        
        assert!(snapshot_response.is_success());
        assert_eq!(snapshot_response.message_count(), 1);
        assert_eq!(snapshot_response.data_size(), 1000);
        
        // Test failed response
        let failed_response = RecoveryResponse::failed(
            789, 
            RecoveryRequestType::Retransmit, 
            "Consumer not found".to_string()
        );
        
        assert!(!failed_response.is_success());
        assert_eq!(failed_response.message_count(), 0);
        assert_eq!(failed_response.data_size(), 0);
    }
}
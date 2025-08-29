//! Network Layer Message Interface
//!
//! Provides a simple, transport-focused Message trait that ONLY handles:
//! - Moving bytes from point A to point B
//! - Basic framing/envelope structure  
//! - Connection management
//! - Transport protocols
//!
//! Does NOT handle:
//! - Financial calculations
//! - Domain-specific message types
//! - Business validation rules
//! - Token decimals or precision
//!
//! This maintains proper architectural boundaries between the network layer
//! and business logic layers.

use crate::error::{NetworkError, Result};
use std::fmt;

/// Network-layer message trait - ONLY handles bytes and transport
pub trait NetworkMessage: Send + Sync + fmt::Debug {
    /// Get the raw byte payload for transport
    fn as_bytes(&self) -> &[u8];
    
    /// Get the message size in bytes (for buffering/allocation)
    fn byte_size(&self) -> usize {
        self.as_bytes().len()
    }
    
    /// Get message type hint for routing (optional)
    fn message_type_hint(&self) -> Option<u16> {
        None
    }
    
    /// Check if this message can be sent over the given transport
    fn is_compatible_with_transport(&self, transport_type: &str) -> bool {
        // By default, all network messages work with all transports
        // Individual implementations can override this
        match transport_type {
            "unix" | "tcp" | "udp" => true,
            _ => false,
        }
    }
}

/// Raw byte message - the simplest network message
#[derive(Debug, Clone)]
pub struct ByteMessage {
    pub data: Vec<u8>,
    pub type_hint: Option<u16>,
}

impl ByteMessage {
    /// Create a new byte message
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            type_hint: None,
        }
    }
    
    /// Create a byte message with a type hint for routing
    pub fn with_type_hint(data: Vec<u8>, type_hint: u16) -> Self {
        Self {
            data,
            type_hint: Some(type_hint),
        }
    }
    
    /// Create from a slice (will allocate)
    pub fn from_slice(data: &[u8]) -> Self {
        Self::new(data.to_vec())
    }
}

impl NetworkMessage for ByteMessage {
    fn as_bytes(&self) -> &[u8] {
        &self.data
    }
    
    fn message_type_hint(&self) -> Option<u16> {
        self.type_hint
    }
}

/// Envelope for network messages with basic routing information
#[derive(Debug, Clone)]
pub struct NetworkEnvelope {
    /// Source identifier (for debugging/tracing only)
    pub source: String,
    /// Destination identifier (for routing)
    pub destination: String,
    /// Raw message payload
    pub payload: Vec<u8>,
    /// Optional priority (network layer only - not business priority)
    pub network_priority: NetworkPriority,
}

/// Network-level priority (affects transport, not business logic)
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NetworkPriority {
    /// Background traffic (bulk data transfers)
    Background,
    /// Normal priority (default)
    Normal,
    /// High priority (latency-sensitive but not critical)
    High,
    /// Critical system messages (heartbeats, connection management)
    Critical,
}

/// Priority alias for routing compatibility
pub type Priority = NetworkPriority;

impl Default for NetworkPriority {
    fn default() -> Self {
        Self::Normal
    }
}

impl NetworkEnvelope {
    /// Create a new network envelope
    pub fn new(source: String, destination: String, payload: Vec<u8>) -> Self {
        Self {
            source,
            destination,
            payload,
            network_priority: NetworkPriority::Normal,
        }
    }
    
    /// Set network priority
    pub fn with_priority(mut self, priority: NetworkPriority) -> Self {
        self.network_priority = priority;
        self
    }
    
    /// Serialize envelope for transport (simple format)
    pub fn to_wire_format(&self) -> Result<Vec<u8>> {
        // Simple wire format: [source_len:2][dest_len:2][source][dest][payload]
        let source_bytes = self.source.as_bytes();
        let dest_bytes = self.destination.as_bytes();
        
        if source_bytes.len() > u16::MAX as usize || dest_bytes.len() > u16::MAX as usize {
            return Err(NetworkError::transport(
                "Source or destination name too long".to_string(),
                Some("envelope_serialization")
            ));
        }
        
        let mut buffer = Vec::with_capacity(
            4 + source_bytes.len() + dest_bytes.len() + self.payload.len()
        );
        
        buffer.extend_from_slice(&(source_bytes.len() as u16).to_le_bytes());
        buffer.extend_from_slice(&(dest_bytes.len() as u16).to_le_bytes());
        buffer.extend_from_slice(source_bytes);
        buffer.extend_from_slice(dest_bytes);
        buffer.extend_from_slice(&self.payload);
        
        Ok(buffer)
    }
    
    /// Parse envelope from wire format
    pub fn from_wire_format(data: &[u8]) -> Result<Self> {
        if data.len() < 4 {
            return Err(NetworkError::parsing("Envelope too short".to_string()));
        }
        
        let source_len = u16::from_le_bytes([data[0], data[1]]) as usize;
        let dest_len = u16::from_le_bytes([data[2], data[3]]) as usize;
        
        if data.len() < 4 + source_len + dest_len {
            return Err(NetworkError::parsing("Truncated envelope".to_string()));
        }
        
        let source = String::from_utf8_lossy(&data[4..4 + source_len]).to_string();
        let destination = String::from_utf8_lossy(&data[4 + source_len..4 + source_len + dest_len]).to_string();
        let payload = data[4 + source_len + dest_len..].to_vec();
        
        Ok(Self::new(source, destination, payload))
    }
}

impl NetworkMessage for NetworkEnvelope {
    fn as_bytes(&self) -> &[u8] {
        &self.payload
    }
    
    fn byte_size(&self) -> usize {
        4 + self.source.len() + self.destination.len() + self.payload.len()
    }
}

/// Network message statistics (transport layer only)
#[derive(Debug, Clone)]
pub struct NetworkMessageStats {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub transport_errors: u64,
}

impl Default for NetworkMessageStats {
    fn default() -> Self {
        Self {
            bytes_sent: 0,
            bytes_received: 0,
            messages_sent: 0,
            messages_received: 0,
            transport_errors: 0,
        }
    }
}

impl NetworkMessageStats {
    /// Record a sent message
    pub fn record_sent(&mut self, byte_size: usize) {
        self.messages_sent += 1;
        self.bytes_sent += byte_size as u64;
    }
    
    /// Record a received message
    pub fn record_received(&mut self, byte_size: usize) {
        self.messages_received += 1;
        self.bytes_received += byte_size as u64;
    }
    
    /// Record a transport error
    pub fn record_error(&mut self) {
        self.transport_errors += 1;
    }
    
    /// Get average message size
    pub fn average_message_size(&self) -> Option<f64> {
        let total_messages = self.messages_sent + self.messages_received;
        if total_messages > 0 {
            let total_bytes = self.bytes_sent + self.bytes_received;
            Some(total_bytes as f64 / total_messages as f64)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_message() {
        let data = b"hello world".to_vec();
        let msg = ByteMessage::new(data.clone());
        
        assert_eq!(msg.as_bytes(), &data);
        assert_eq!(msg.byte_size(), data.len());
        assert_eq!(msg.message_type_hint(), None);
    }
    
    #[test]
    fn test_byte_message_with_hint() {
        let data = b"hello".to_vec();
        let msg = ByteMessage::with_type_hint(data.clone(), 42);
        
        assert_eq!(msg.as_bytes(), &data);
        assert_eq!(msg.message_type_hint(), Some(42));
    }
    
    #[test]
    fn test_network_envelope_wire_format() {
        let envelope = NetworkEnvelope::new(
            "source".to_string(),
            "destination".to_string(),
            b"payload data".to_vec(),
        );
        
        // Test serialization
        let wire_data = envelope.to_wire_format().unwrap();
        assert!(wire_data.len() > envelope.payload.len());
        
        // Test deserialization
        let parsed = NetworkEnvelope::from_wire_format(&wire_data).unwrap();
        assert_eq!(parsed.source, "source");
        assert_eq!(parsed.destination, "destination");
        assert_eq!(parsed.payload, b"payload data");
    }
    
    #[test]
    fn test_network_priority() {
        let envelope = NetworkEnvelope::new(
            "src".to_string(),
            "dst".to_string(),
            Vec::new(),
        ).with_priority(NetworkPriority::High);
        
        assert_eq!(envelope.network_priority, NetworkPriority::High);
    }
    
    #[test]
    fn test_message_stats() {
        let mut stats = NetworkMessageStats::default();
        
        stats.record_sent(100);
        stats.record_sent(200);
        stats.record_received(150);
        
        assert_eq!(stats.messages_sent, 2);
        assert_eq!(stats.bytes_sent, 300);
        assert_eq!(stats.messages_received, 1);
        assert_eq!(stats.bytes_received, 150);
        
        let avg = stats.average_message_size().unwrap();
        assert!((avg - 150.0).abs() < 0.1); // (300 + 150) / 3 = 150
    }
    
    #[test]
    fn test_transport_compatibility() {
        let msg = ByteMessage::new(b"test".to_vec());
        
        assert!(msg.is_compatible_with_transport("unix"));
        assert!(msg.is_compatible_with_transport("tcp"));
        assert!(msg.is_compatible_with_transport("udp"));
        assert!(!msg.is_compatible_with_transport("invalid"));
    }
}
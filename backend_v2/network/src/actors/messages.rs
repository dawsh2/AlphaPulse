//! Actor System Message Transport (MYCEL-002)
//!
//! Transport-layer message handling for the actor system. This module ONLY handles:
//! - Message routing between actors
//! - Serialization for inter-process communication  
//! - Actor-to-actor message delivery
//!
//! Domain-specific message types have been moved to torq-types/src/messages.rs
//! to maintain proper architectural boundaries.

use crate::error::{NetworkError, Result};
use crate::message::NetworkMessage;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::warn;

/// Transport-layer message trait for actor system
pub trait ActorMessage: Send + Sync + 'static {
    /// Serialize for inter-process transport
    fn serialize(&self) -> Result<Vec<u8>>;
    
    /// Deserialize from transport bytes
    fn deserialize(bytes: &[u8]) -> Result<Self>
    where 
        Self: Sized;
    
    /// Type ID for runtime checking and downcasting
    fn message_type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }
    
    /// Convert to Any for local passing
    fn as_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> where Self: Sized {
        self as Arc<dyn Any + Send + Sync>
    }
    
    /// Estimated message size for transport buffering
    fn estimated_size(&self) -> usize where Self: Sized {
        std::mem::size_of::<Self>()
    }
}

/// Actor message envelope for routing
#[derive(Debug, Clone)]
pub struct ActorEnvelope {
    /// Source actor ID
    pub from_actor: String,
    /// Destination actor ID
    pub to_actor: String,
    /// Message type identifier
    pub message_type: String,
    /// Serialized message payload
    pub payload: Vec<u8>,
    /// Delivery timestamp (nanoseconds)
    pub timestamp_ns: u64,
}

impl ActorEnvelope {
    /// Create a new actor envelope
    pub fn new<M: ActorMessage>(
        from_actor: String,
        to_actor: String,
        message: &M,
    ) -> Result<Self> {
        let payload = message.serialize()?;
        let timestamp_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| NetworkError::system(format!("System time error: {}", e)))?
            .as_nanos() as u64;
        
        Ok(Self {
            from_actor,
            to_actor,
            message_type: std::any::type_name::<M>().to_string(),
            payload,
            timestamp_ns,
        })
    }
    
    /// Deserialize the payload as a specific message type
    pub fn deserialize_payload<M: ActorMessage>(&self) -> Result<M> {
        M::deserialize(&self.payload)
    }
}

impl NetworkMessage for ActorEnvelope {
    fn as_bytes(&self) -> &[u8] {
        &self.payload
    }
    
    fn byte_size(&self) -> usize {
        self.from_actor.len() + 
        self.to_actor.len() + 
        self.message_type.len() + 
        self.payload.len() + 
        std::mem::size_of::<u64>()  // timestamp
    }
}

/// Type-safe receiver for actor message channels
pub struct TypedReceiver<M: ActorMessage> {
    rx: mpsc::Receiver<Arc<dyn Any + Send + Sync>>,
    _phantom: PhantomData<M>,
}

impl<M: ActorMessage> TypedReceiver<M> {
    /// Create new typed receiver from channel
    pub fn new(rx: mpsc::Receiver<Arc<dyn Any + Send + Sync>>) -> Self {
        Self {
            rx,
            _phantom: PhantomData,
        }
    }
    
    /// Receive next message of expected type
    pub async fn recv(&mut self) -> Option<Arc<M>> {
        while let Some(any_msg) = self.rx.recv().await {
            // Try to downcast to expected type
            if let Ok(typed) = any_msg.downcast::<M>() {
                return Some(typed);
            } else {
                // Log unexpected message type and continue waiting
                warn!(
                    expected_type = std::any::type_name::<M>(),
                    "Received unexpected message type in TypedReceiver"
                );
            }
        }
        None
    }
    
    /// Try to receive without blocking
    pub fn try_recv(&mut self) -> Result<Arc<M>, tokio::sync::mpsc::error::TryRecvError> {
        match self.rx.try_recv() {
            Ok(any_msg) => {
                if let Ok(typed) = any_msg.downcast::<M>() {
                    Ok(typed)
                } else {
                    warn!(
                        expected_type = std::any::type_name::<M>(),
                        "Received unexpected message type in TypedReceiver"
                    );
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty)
                }
            },
            Err(e) => Err(e),
        }
    }
}

/// Message handler trait for actor dispatch
pub trait MessageHandler: Send + Sync {
    type Message: ActorMessage;
    
    /// Handle incoming message
    async fn handle(&mut self, msg: Self::Message) -> Result<()>;
}

/// Simple byte message for actor system
#[derive(Debug, Clone, PartialEq)]
pub struct ByteActorMessage {
    pub data: Vec<u8>,
    pub msg_type: String,
}

impl ByteActorMessage {
    pub fn new(data: Vec<u8>, msg_type: impl Into<String>) -> Self {
        Self {
            data,
            msg_type: msg_type.into(),
        }
    }
}

impl ActorMessage for ByteActorMessage {
    fn serialize(&self) -> Result<Vec<u8>> {
        // Simple serialization: [type_len:4][type][data]
        let type_bytes = self.msg_type.as_bytes();
        let mut serialized = Vec::with_capacity(4 + type_bytes.len() + self.data.len());
        
        serialized.extend_from_slice(&(type_bytes.len() as u32).to_le_bytes());
        serialized.extend_from_slice(type_bytes);
        serialized.extend_from_slice(&self.data);
        
        Ok(serialized)
    }
    
    fn deserialize(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 4 {
            return Err(NetworkError::parsing("Message too short".to_string()));
        }
        
        let type_len = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        
        if bytes.len() < 4 + type_len {
            return Err(NetworkError::parsing("Truncated message".to_string()));
        }
        
        let msg_type = String::from_utf8_lossy(&bytes[4..4 + type_len]).to_string();
        let data = bytes[4 + type_len..].to_vec();
        
        Ok(Self::new(data, msg_type))
    }
    
    fn estimated_size(&self) -> usize {
        4 + self.msg_type.len() + self.data.len()
    }
}

/// Message statistics for actor system
#[derive(Debug, Default)]
pub struct ActorMessageRegistry {
    types: HashMap<TypeId, &'static str>,
    counts: HashMap<TypeId, AtomicU64>,
    bytes_processed: HashMap<TypeId, AtomicU64>,
}

impl ActorMessageRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Register an actor message type
    pub fn register<M: ActorMessage>(&mut self, name: &'static str) {
        let type_id = TypeId::of::<M>();
        self.types.insert(type_id, name);
        self.counts.insert(type_id, AtomicU64::new(0));
        self.bytes_processed.insert(type_id, AtomicU64::new(0));
    }
    
    /// Record message processed
    pub fn record_message<M: ActorMessage>(&self, byte_size: usize) {
        let type_id = TypeId::of::<M>();
        
        if let Some(counter) = self.counts.get(&type_id) {
            counter.fetch_add(1, Ordering::Relaxed);
        }
        
        if let Some(bytes_counter) = self.bytes_processed.get(&type_id) {
            bytes_counter.fetch_add(byte_size as u64, Ordering::Relaxed);
        }
    }
    
    /// Get message statistics
    pub fn get_stats(&self) -> ActorMessageStats {
        let mut message_counts = HashMap::new();
        let mut bytes_processed = HashMap::new();
        
        for (type_id, counter) in &self.counts {
            if let Some(&name) = self.types.get(type_id) {
                let count = counter.load(Ordering::Relaxed);
                message_counts.insert(name.to_string(), count);
                
                if let Some(bytes_counter) = self.bytes_processed.get(type_id) {
                    let bytes = bytes_counter.load(Ordering::Relaxed);
                    bytes_processed.insert(name.to_string(), bytes);
                }
            }
        }
        
        ActorMessageStats {
            message_counts,
            bytes_processed,
        }
    }
}

/// Actor message statistics
#[derive(Debug, Clone)]
pub struct ActorMessageStats {
    pub message_counts: HashMap<String, u64>,
    pub bytes_processed: HashMap<String, u64>,
}

impl ActorMessageStats {
    /// Get total messages processed
    pub fn total_messages(&self) -> u64 {
        self.message_counts.values().sum()
    }
    
    /// Get total bytes processed  
    pub fn total_bytes(&self) -> u64 {
        self.bytes_processed.values().sum()
    }
    
    /// Get average message size
    pub fn average_message_size(&self) -> Option<f64> {
        let total_messages = self.total_messages();
        if total_messages > 0 {
            Some(self.total_bytes() as f64 / total_messages as f64)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_actor_message_serialization() {
        let msg = ByteActorMessage::new(b"hello world".to_vec(), "test");
        
        let serialized = msg.serialize().unwrap();
        let deserialized = ByteActorMessage::deserialize(&serialized).unwrap();
        
        assert_eq!(msg, deserialized);
        assert_eq!(deserialized.msg_type, "test");
        assert_eq!(deserialized.data, b"hello world");
    }

    #[test]
    fn test_actor_envelope() {
        let msg = ByteActorMessage::new(b"test".to_vec(), "TestMsg");
        
        let envelope = ActorEnvelope::new(
            "sender".to_string(),
            "receiver".to_string(), 
            &msg,
        ).unwrap();
        
        assert_eq!(envelope.from_actor, "sender");
        assert_eq!(envelope.to_actor, "receiver");
        assert!(envelope.timestamp_ns > 0);
        
        // Test deserialization
        let deserialized: ByteActorMessage = envelope.deserialize_payload().unwrap();
        assert_eq!(deserialized, msg);
    }

    #[test]
    fn test_message_registry() {
        let mut registry = ActorMessageRegistry::new();
        registry.register::<ByteActorMessage>("ByteMsg");
        
        registry.record_message::<ByteActorMessage>(100);
        registry.record_message::<ByteActorMessage>(200);
        
        let stats = registry.get_stats();
        assert_eq!(stats.message_counts["ByteMsg"], 2);
        assert_eq!(stats.bytes_processed["ByteMsg"], 300);
        assert_eq!(stats.total_messages(), 2);
        assert_eq!(stats.total_bytes(), 300);
        assert_eq!(stats.average_message_size().unwrap(), 150.0);
    }

    #[tokio::test]
    async fn test_typed_receiver() {
        let (tx, rx) = mpsc::channel(10);
        let mut typed_rx = TypedReceiver::<ByteActorMessage>::new(rx);
        
        let msg = Arc::new(ByteActorMessage::new(b"test".to_vec(), "TestMsg"));
        tx.send(msg.clone() as Arc<dyn Any + Send + Sync>).await.unwrap();
        
        let received = typed_rx.recv().await.unwrap();
        assert_eq!(*received, *msg);
    }
}
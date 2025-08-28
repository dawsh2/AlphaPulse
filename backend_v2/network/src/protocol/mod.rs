//! Network Protocol Infrastructure
//!
//! This module consolidates all wire protocol, serialization, compression,
//! and security functionality for the network layer.

pub mod compression;
pub mod envelope;
pub mod security;

// Re-export commonly used types
pub use compression::{CompressionEngine, CompressionInfo, CompressionType};
pub use envelope::{MessageFlags, NetworkEnvelope, WireFormat};
pub use security::{SecurityInfo, SecurityLayer};

use crate::Result;

/// Protocol configuration
#[derive(Debug, Clone)]
pub struct ProtocolConfig {
    /// Enable compression for messages larger than this threshold (bytes)
    pub compression_threshold: usize,
    /// Compression algorithm to use
    pub compression_type: CompressionType,
    /// Enable encryption
    pub enable_encryption: bool,
    /// Security layer configuration
    pub security_config: Option<SecurityConfig>,
    /// Maximum message size in bytes
    pub max_message_size: usize,
}

impl Default for ProtocolConfig {
    fn default() -> Self {
        Self {
            compression_threshold: 1024,  // 1KB
            compression_type: CompressionType::Lz4,
            enable_encryption: false,
            security_config: None,
            max_message_size: 16 * 1024 * 1024,  // 16MB
        }
    }
}

/// Security configuration
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    pub key_file: String,
    pub cert_file: String,
    pub ca_file: Option<String>,
    pub verify_peer: bool,
}

/// Protocol processor that handles message transformation
pub struct ProtocolProcessor {
    config: ProtocolConfig,
    compression_engine: Option<CompressionEngine>,
    security_layer: Option<SecurityLayer>,
}

impl ProtocolProcessor {
    /// Create new protocol processor
    pub fn new(config: ProtocolConfig) -> Result<Self> {
        let compression_engine = if config.compression_threshold > 0 {
            Some(CompressionEngine::new(config.compression_type)?)
        } else {
            None
        };
        
        let security_layer = if config.enable_encryption {
            if let Some(ref security_config) = config.security_config {
                Some(SecurityLayer::new(security_config.clone())?)
            } else {
                return Err(crate::TransportError::configuration(
                    "Encryption enabled but no security config provided",
                    Some("security_config")
                ));
            }
        } else {
            None
        };
        
        Ok(Self {
            config,
            compression_engine,
            security_layer,
        })
    }
    
    /// Process outbound message (compress, encrypt, envelope)
    pub fn process_outbound(&self, message: &[u8]) -> Result<Vec<u8>> {
        if message.len() > self.config.max_message_size {
            return Err(crate::TransportError::protocol(format!(
                "Message size {} exceeds maximum {}",
                message.len(), self.config.max_message_size
            )));
        }
        
        let mut data = message.to_vec();
        
        // Apply compression if configured and message is large enough
        if let Some(ref engine) = self.compression_engine {
            if data.len() >= self.config.compression_threshold {
                data = engine.compress(&data)?;
            }
        }
        
        // Apply encryption if configured
        if let Some(ref layer) = self.security_layer {
            data = layer.encrypt(&data)?;
        }
        
        // Wrap in envelope
        let envelope = NetworkEnvelope::new(data, MessageFlags::default())?;
        envelope.serialize()
    }
    
    /// Process inbound message (de-envelope, decrypt, decompress)
    pub fn process_inbound(&self, message: &[u8]) -> Result<Vec<u8>> {
        // Parse envelope
        let envelope = NetworkEnvelope::deserialize(message)?;
        let mut data = envelope.payload;
        
        // Apply decryption if configured
        if let Some(ref layer) = self.security_layer {
            data = layer.decrypt(&data)?;
        }
        
        // Apply decompression if needed
        if let Some(ref engine) = self.compression_engine {
            if envelope.flags.compressed {
                data = engine.decompress(&data)?;
            }
        }
        
        Ok(data)
    }
    
    /// Get protocol statistics
    pub fn stats(&self) -> ProtocolStats {
        ProtocolStats {
            compression_enabled: self.compression_engine.is_some(),
            encryption_enabled: self.security_layer.is_some(),
            max_message_size: self.config.max_message_size,
            compression_threshold: self.config.compression_threshold,
        }
    }
}

/// Protocol statistics
#[derive(Debug, Clone)]
pub struct ProtocolStats {
    pub compression_enabled: bool,
    pub encryption_enabled: bool,
    pub max_message_size: usize,
    pub compression_threshold: usize,
}
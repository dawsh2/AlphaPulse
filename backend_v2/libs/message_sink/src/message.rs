use crate::SinkError;
use alphapulse_network::current_timestamp_ns;

/// Maximum message size in bytes (16MB default)
pub const DEFAULT_MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024;

/// Protocol-agnostic message wrapper
#[derive(Debug, Clone)]
pub struct Message {
    /// Raw message bytes (could be TLV, JSON, etc.)
    pub payload: Vec<u8>,

    /// Optional routing metadata
    pub metadata: MessageMetadata,
}

impl Default for Message {
    fn default() -> Self {
        Self {
            payload: Vec::new(),
            metadata: MessageMetadata::default(),
        }
    }
}

impl Message {
    /// Create a new message with payload, validating size
    pub fn new(payload: Vec<u8>) -> Result<Self, SinkError> {
        Self::new_with_limit(payload, DEFAULT_MAX_MESSAGE_SIZE)
    }

    /// Create a new message with payload and custom size limit
    pub fn new_with_limit(payload: Vec<u8>, max_size: usize) -> Result<Self, SinkError> {
        if payload.len() > max_size {
            return Err(SinkError::message_too_large(payload.len(), max_size));
        }

        Ok(Self {
            payload,
            metadata: MessageMetadata::new(),
        })
    }

    /// Create a new message with payload and metadata, validating size
    pub fn with_metadata(payload: Vec<u8>, metadata: MessageMetadata) -> Result<Self, SinkError> {
        Self::with_metadata_and_limit(payload, metadata, DEFAULT_MAX_MESSAGE_SIZE)
    }

    /// Create a new message with payload, metadata, and custom size limit
    pub fn with_metadata_and_limit(
        payload: Vec<u8>,
        metadata: MessageMetadata,
        max_size: usize,
    ) -> Result<Self, SinkError> {
        if payload.len() > max_size {
            return Err(SinkError::message_too_large(payload.len(), max_size));
        }

        Ok(Self { payload, metadata })
    }

    /// Create a new message without size validation (for internal use)
    pub fn new_unchecked(payload: Vec<u8>) -> Self {
        Self {
            payload,
            metadata: MessageMetadata::new(),
        }
    }

    /// Get message size in bytes
    pub fn size(&self) -> usize {
        self.payload.len()
    }

    /// Check if message exceeds size limit
    pub fn exceeds_limit(&self, limit: usize) -> bool {
        self.payload.len() > limit
    }

    /// Validate precision for financial data (Protocol V2 requirement)
    /// DEX tokens: preserve native precision (18 decimals WETH, 6 USDC)
    /// Traditional exchanges: 8-decimal fixed-point for USD prices
    pub fn validate_precision(&self, is_dex: bool) -> Result<(), SinkError> {
        // This would typically parse the payload based on message type
        // For now, we ensure the payload is properly aligned for numeric data

        if self.payload.len() < 8 {
            // Too small to contain financial data
            return Ok(());
        }

        // Check for common precision issues
        if is_dex {
            // DEX validation: ensure no truncation of wei values
            // Wei values should be i64 or u64, check alignment
            if self.payload.len() % 8 != 0 {
                return Err(SinkError::invalid_config(
                    "DEX message payload not aligned for native precision values".to_string(),
                ));
            }
        } else {
            // Traditional exchange: ensure 8-decimal fixed-point representation
            // This is a placeholder - actual implementation would parse the specific format
            // and validate precision preservation
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct MessageMetadata {
    /// Target service hint (optional)
    pub target: Option<String>,

    /// Message priority for queueing
    pub priority: MessagePriority,

    /// Timestamp when created
    pub timestamp_ns: u64,

    /// Correlation ID for tracing
    pub correlation_id: Option<String>,
}

impl MessageMetadata {
    /// Create metadata with high-performance timestamp
    pub fn new() -> Self {
        Self {
            target: None,
            priority: MessagePriority::Normal,
            timestamp_ns: current_timestamp_ns(),
            correlation_id: None,
        }
    }

    /// Set target service
    pub fn with_target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    /// Set priority
    pub fn with_priority(mut self, priority: MessagePriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set correlation ID
    pub fn with_correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MessagePriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl Default for MessagePriority {
    fn default() -> Self {
        MessagePriority::Normal
    }
}

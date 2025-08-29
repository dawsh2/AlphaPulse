//! # Unified TLV Message Validation System
//!
//! ## Purpose
//!
//! Consolidated validation framework for TLV messages with configurable policies
//! optimized for different relay domains while maintaining high throughput.
//!
//! ## Architecture
//!
//! ```text
//! Message Input → TLVValidator → Domain Validator → Validated Message
//!       ↓              ↓             ↓                    ↓
//!   Raw Bytes    Policy Check   TLV Range Check    ValidatedMessage
//!   Header       Size Limits    Type Validation    Ready for Processing
//! ```
//!
//! ## Validation Levels
//!
//! - **Performance** (Market Data): Minimal validation, >1M msg/s throughput
//! - **Standard** (Signals): CRC32 validation, >100K msg/s throughput  
//! - **Audit** (Execution): Full validation + logging, >50K msg/s throughput

// Submodules
pub mod config;
pub mod validator;
pub mod builder;
pub mod domain;

// Re-export main types for convenience
pub use config::{
    ValidationConfig, 
    DomainMessageLimits, 
    TimestampConfig, 
    SequenceConfig, 
    PoolDiscoveryConfig
};

pub use validator::{
    TLVValidator,
    ValidationError,
    ValidationPolicy,
    ValidatedMessage,
    ValidationLevel,
    DomainValidationRules,
    SequenceTracker,
    PoolDiscoveryQueue,
    TLVExtensionZeroCopy,
};

pub use builder::{
    ValidatingTLVMessageBuilder,
    BuilderFactory,
    patterns,
};

pub use domain::{
    DomainValidator,
    MarketDataValidator,
    SignalValidator,
    ExecutionValidator,
    create_domain_validator,
};
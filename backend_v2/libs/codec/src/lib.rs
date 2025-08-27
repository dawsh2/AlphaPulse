//! # AlphaPulse Protocol Codec - Consolidated Validation System
//!
//! ## Purpose
//!
//! This crate contains the "Rules" layer of the AlphaPulse system with consolidated
//! validation logic from the relay infrastructure:
//! - Protocol encoding/decoding logic
//! - **Unified message validation system**
//! - **Domain-specific validation rules**
//! - Message construction and validation
//! - Bijective identifier systems
//! - TLV type registry and constants
//!
//! ## MYC-004 Codec Consolidation
//!
//! **Consolidated Components**:
//! - All TLV parsing/validation logic from relays moved to codec
//! - Domain-specific validators (MarketData, Signal, Execution)
//! - Enhanced message builder with validation
//! - Performance-tuned validation policies
//! - Migration compatibility layer
//!
//! ## Integration Points
//!
//! - **Message Construction**: Enhanced TLVMessageBuilder with validation
//! - **Parsing Validation**: Consolidated TLVValidator with domain rules
//! - **Relay Routing**: Automatic domain-based routing to appropriate relay services
//! - **Cache Systems**: Bijective InstrumentId system enables ultra-fast lookups
//! - **Cross-Service Communication**: Self-describing identifiers eliminate registry dependencies
//! - **Migration Support**: Compatibility layer for gradual service migration
//!
//! ## Architecture Role
//!
//! ```text
//! libs/types → [consolidated codec] → network/
//!     ↑              ↓                    ↓
//! Pure Data    Protocol Rules         Transport
//! Structures   Validation/Encoding    Connections
//! TradeTLV     TLVValidator           Sockets
//! ```
//!
//! ## What This Crate Contains
//! - **TLVValidator**: Consolidated validation with domain-specific rules
//! - **ValidatingTLVMessageBuilder**: Enhanced builder with validation
//! - **Domain Validators**: MarketData, Signal, Execution validators
//! - **Migration Layer**: Compatibility functions for gradual migration
//! - TLVMessageBuilder for constructing valid messages
//! - InstrumentId bijective identifier system
//! - Protocol parsing functions
//! - TLVType registry and validation
//! - Protocol constants and error types
//!
//! ## What This Crate Does NOT Contain
//! - Network transport logic (belongs in network/)
//! - Raw data structure definitions (belongs in libs/types)
//! - Socket management or connection handling
//!
//! ## Performance Profile
//!
//! **Maintained Performance**:
//! - **Identifier Construction**: >19M identifiers/second (measured: 19,796,915 ops/s)
//! - **Message Parsing**: >1.6M msg/s parsing performance (preserved)
//! - **Message Construction**: >1M msg/s construction performance (preserved)
//! - **Validation Overhead**: <2μs per message with domain-specific policies
//! - **Cache Efficiency**: u64/u128 keys maximize CPU cache utilization
//! - **Zero-Copy Operations**: zerocopy traits for minimal allocation overhead
//!
//! **Validation Performance by Domain**:
//! - **MarketData**: Performance mode, >1M msg/s (minimal validation)
//! - **Signal**: Standard mode, >100K msg/s (checksum validation)
//! - **Execution**: Audit mode, >50K msg/s (full validation + logging)

// Core modules
pub mod constants;
pub mod error;
pub mod instrument_id;
pub mod message_builder;
pub mod parser;
pub mod tlv_types;

// MYC-004 Consolidated validation modules - IMPLEMENTED
pub mod validation;
pub mod validation_config; 
pub mod validation_enhanced;
pub mod enhanced_builder;
pub mod migration;

// Re-export key types for convenience
pub use constants::*;
pub use error::{ParseError, ParseResult, ProtocolError, ProtocolResult};
pub use instrument_id::{AssetType, InstrumentId, VenueId};
pub use message_builder::{TLVMessageBuilder, VendorTLVBuilder};
pub use parser::{
    extract_tlv_payload, find_tlv_by_type, parse_header, parse_header_without_checksum,
    parse_tlv_extensions, validate_tlv_size, ExtendedTLVExtension, ExtendedTLVHeader,
    SimpleTLVExtension, SimpleTLVHeader, TLVExtensionEnum,
};
pub use tlv_types::{TLVSizeConstraint, TLVType, TlvTypeRegistry};

// Re-export consolidated validation system
pub use validation::{
    TLVValidator, ValidationError, ValidationPolicy, ValidatedMessage,
    DomainValidator, MarketDataValidator, SignalValidator, ExecutionValidator,
    create_domain_validator, DomainValidationRules, ValidationLevel,
};
pub use validation_config::{
    ValidationConfig, DomainMessageLimits, TimestampConfig, SequenceConfig, PoolDiscoveryConfig,
};
pub use validation_enhanced::{
    EnhancedTLVValidator, EnhancedValidationError, SequenceTracker, PoolDiscoveryQueue,
    TLVExtensionZeroCopy,
};
pub use enhanced_builder::{
    ValidatingTLVMessageBuilder, BuilderFactory, patterns,
};

// Migration support (with deprecation warnings)
pub use migration::{
    compat, migration_utils, test_utils, MigrationConfig,
};

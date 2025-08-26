//! # AlphaPulse Protocol Codec
//!
//! ## Purpose
//!
//! This crate contains the "Rules" layer of the AlphaPulse system:
//! - Protocol encoding/decoding logic
//! - Message construction and validation
//! - Bijective identifier systems
//! - TLV type registry and constants
//!
//! ## Integration Points
//!
//! - **Message Construction**: TLVMessageBuilder uses type metadata for format selection
//! - **Parsing Validation**: Parser validates payload sizes against type constraints
//! - **Relay Routing**: Automatic domain-based routing to appropriate relay services
//! - **Cache Systems**: Bijective InstrumentId system enables ultra-fast lookups
//! - **Cross-Service Communication**: Self-describing identifiers eliminate registry dependencies
//!
//! ## Architecture Role
//!
//! ```text
//! libs/types → [alphapulse_codec] → network/
//!     ↑              ↓                ↓
//! Pure Data    Protocol Rules    Transport
//! Structures   Encoding/Decoding  Connections
//! TradeTLV     TLVMessageBuilder  Sockets
//! ```
//!
//! ## What This Crate Contains
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
//! - **Identifier Construction**: >19M identifiers/second (measured: 19,796,915 ops/s)
//! - **Message Parsing**: >1.6M msg/s parsing performance
//! - **Message Construction**: >1M msg/s construction performance
//! - **Cache Efficiency**: u64/u128 keys maximize CPU cache utilization
//! - **Zero-Copy Operations**: zerocopy traits for minimal allocation overhead

pub mod constants;
pub mod instrument_id;
pub mod parser;
pub mod tlv_types;

// Re-export key types for convenience
pub use constants::*;
pub use instrument_id::{AssetType, CodecError, InstrumentId, VenueId};
pub use parser::{
    parse_header, parse_tlv_extensions, validate_tlv_size, ParseResult, TlvExtension,
};
pub use tlv_types::{TLVSizeConstraint, TLVType, TlvTypeRegistry};

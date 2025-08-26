//! # Protocol Constants - Protocol V2 Core Constants
//!
//! ## Purpose
//!
//! Central registry of protocol-level constants used throughout the AlphaPulse system.
//! These values define the core protocol behavior and must remain stable for backward
//! compatibility across all services and message formats.
//!
//! ## Integration Points
//!
//! - **Message Headers**: MESSAGE_MAGIC used for protocol identification
//! - **Version Negotiation**: PROTOCOL_VERSION for compatibility checking
//! - **Service Discovery**: Socket paths for relay communication
//! - **Validation**: Magic number verification in message parsing
//!
//! ## Architecture Role
//!
//! ```text
//! Services → [Protocol Constants] → Message Construction
//!     ↑              ↓                     ↓
//! Config Lookup  Standard Values    Header Fields
//! Path Discovery Version Checks     Magic Numbers
//! ```
//!
//! The constants module provides the foundational values that ensure protocol
//! consistency across all AlphaPulse components.

/// Protocol magic number for message headers
///
/// This magic number (0xDEADBEEF) is used to identify AlphaPulse Protocol V2 messages
/// and must be the first 4 bytes of every message header for protocol validation.
pub const MESSAGE_MAGIC: u32 = 0xDEADBEEF;

/// Current protocol version
///
/// Version 1 is the stable Protocol V2 implementation supporting:
/// - 32-byte MessageHeader with TLV payload
/// - Bijective InstrumentId system
/// - Domain-based relay routing (MarketData, Signals, Execution)
/// - Zero-copy message parsing with zerocopy traits
pub const PROTOCOL_VERSION: u8 = 1;

/// Unix domain socket path for market data relay
///
/// High-frequency market data messages (TLV types 1-19) are routed through
/// this relay for performance isolation from other message types.
pub const MARKET_DATA_RELAY_PATH: &str = "/tmp/alphapulse/market_data.sock";

/// Unix domain socket path for signal relay
///
/// Strategy signals and analytics messages (TLV types 20-39, 60-79) are routed
/// through this relay for coordination between trading strategies and risk management.
pub const SIGNAL_RELAY_PATH: &str = "/tmp/alphapulse/signals.sock";

/// Unix domain socket path for execution relay
///
/// Order execution and trade confirmation messages (TLV types 40-59) are routed
/// through this relay with strict validation for financial safety.
pub const EXECUTION_RELAY_PATH: &str = "/tmp/alphapulse/execution.sock";

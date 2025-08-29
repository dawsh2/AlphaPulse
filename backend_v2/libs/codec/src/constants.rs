//! # Protocol Constants - Protocol V2 Core Constants
//!
//! ## Purpose
//!
//! Central registry of protocol-level constants used throughout the Torq system.
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
//! consistency across all Torq components.

// Protocol constants have been moved to protocol_constants.rs
// Socket paths and deployment configuration should be in environment variables
// or configuration files, not hardcoded in a codec library.

pub use crate::protocol_constants::*;

//! Reusable Adapter Components
//!
//! Provides composable building blocks for creating exchange adapters following
//! established patterns. These components enforce the zero-copy message sending
//! pattern via direct RelayOutput integration, eliminating channel overhead.
//!
//! ## Component Performance Overview
//!
//! | Component | Primary Operation | Performance Target | Measured |
//! |-----------|-------------------|-------------------|----------|
//! | MessageSender | TLV message construction & send | <1μs per message | Meets Protocol V2 targets |
//! | SymbolMapper | Symbol → InstrumentId resolution | <100ns per lookup | O(1) HashMap access |
//! | Parsing Utils | JSON field extraction & conversion | <200ns per field | Zero-allocation where possible |
//!
//! ## Architecture Integration
//!
//! These components integrate seamlessly with AlphaPulse Protocol V2:
//! - **Zero-Copy**: TLV serialization without intermediate copying
//! - **Single Allocation**: One Vec<u8> per message for async ownership transfer
//! - **Direct Relay**: No MPSC channel overhead, direct RelayOutput integration
//! - **Type Safety**: Compile-time enforcement of proper TLV construction patterns

pub mod message_sender;
pub mod parsing_utils;

pub use message_sender::{MessageSender, MessageSenderImpl};
pub use parsing_utils::*;

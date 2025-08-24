//! Shared DEX functionality library
//!
//! This library provides common DEX-related functionality that is shared across
//! multiple services including collectors, strategies, and validators.
//!
//! # Architecture
//!
//! ```text
//! libs/dex/
//! ├── abi/        # ABI definitions and event decoders
//! │   ├── events.rs      # Event structures and decoders
//! │   ├── uniswap_v2.rs  # V2 specific ABIs
//! │   └── uniswap_v3.rs  # V3 specific ABIs
//! └── math/       # AMM mathematics (future)
//! ```
//!
//! # Design Principles
//! - Single canonical source for DEX ABIs
//! - Protocol-agnostic interfaces
//! - Zero-copy where possible
//! - Semantic validation built-in

pub mod abi;

// Re-export commonly used types
pub use abi::{
    detect_dex_protocol,
    events::{DecodingError, ValidatedBurn, ValidatedMint, ValidatedSwap},
    get_all_event_signatures, get_swap_signatures, BurnEventDecoder, MintEventDecoder,
    SwapEventDecoder,
};

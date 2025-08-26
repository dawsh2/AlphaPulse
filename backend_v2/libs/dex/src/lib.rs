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
pub mod event_signatures;

// Re-export commonly used types
pub use abi::{
    detect_dex_protocol,
    events::{DecodingError, ValidatedBurn, ValidatedMint, ValidatedSwap},
    get_all_event_signatures, get_swap_signatures, BurnEventDecoder, MintEventDecoder,
    SwapEventDecoder,
};

// Re-export centralized event signature constants
pub use event_signatures::{
    // Utility functions
    get_all_dex_signatures,
    get_liquidity_signatures,
    get_swap_signatures as get_swap_signature_constants,
    to_hex_string,
    ERC20_APPROVAL,
    // ERC-20 signatures
    ERC20_TRANSFER,
    UNISWAP_V2_BURN,
    UNISWAP_V2_MINT,
    // Uniswap V2 signatures
    UNISWAP_V2_SWAP,
    UNISWAP_V2_SYNC,
    UNISWAP_V3_BURN,
    UNISWAP_V3_MINT,
    // Uniswap V3 signatures
    UNISWAP_V3_SWAP,
};

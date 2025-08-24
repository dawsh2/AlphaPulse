//! ABI definitions and event decoding for DEX protocols
//!
//! This module provides:
//! - Canonical ABI definitions for various DEX protocols
//! - Type-safe event decoders with semantic validation
//! - Protocol detection utilities
//!
//! # Supported Protocols
//! - Uniswap V2 and forks (Sushiswap, Quickswap V2)
//! - Uniswap V3 and forks (Quickswap V3)

pub mod events;
pub mod uniswap_v2;
pub mod uniswap_v3;

// Temporary DEXProtocol enum for testing until protocol_v2 is fixed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DEXProtocol {
    UniswapV2,
    UniswapV3,
    SushiswapV2,
    QuickswapV2,
    QuickswapV3,
}

// Re-export main components
pub use events::{
    detect_dex_protocol, BurnEventDecoder, DecodingError, MintEventDecoder, SwapEventDecoder,
    ValidatedBurn, ValidatedMint, ValidatedSwap,
};

/// Get event signatures for WebSocket subscription
pub fn get_all_event_signatures() -> Vec<String> {
    vec![
        format!("0x{:x}", uniswap_v2::swap_event().signature()),
        format!("0x{:x}", uniswap_v3::swap_event().signature()),
        format!("0x{:x}", uniswap_v2::mint_event().signature()),
        format!("0x{:x}", uniswap_v3::mint_event().signature()),
        format!("0x{:x}", uniswap_v2::burn_event().signature()),
        format!("0x{:x}", uniswap_v3::burn_event().signature()),
        format!("0x{:x}", uniswap_v2::sync_event().signature()),
    ]
}

/// Get swap event signatures specifically
pub fn get_swap_signatures() -> (String, String) {
    (
        format!("0x{:x}", uniswap_v2::swap_event().signature()), // V2
        format!("0x{:x}", uniswap_v3::swap_event().signature()), // V3
    )
}

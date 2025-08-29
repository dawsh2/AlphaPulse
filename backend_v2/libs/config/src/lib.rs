//! # Torq Centralized Configuration
//!
//! This crate provides centralized configuration management and constants
//! for all Torq services, eliminating duplication across the codebase.
//!
//! ## Features
//!
//! - **Blockchain Constants**: Event signatures, token addresses, DEX routers
//! - **Protocol V2 Constants**: Magic numbers, TLV domain ranges, message sizes
//! - **Service Configuration**: Default values, timeouts, performance targets
//!
//! ## Usage
//!
//! ```rust
//! use torq_config::{blockchain, protocol};
//!
//! // Use blockchain constants
//! let swap_signature = blockchain::events::UNISWAP_V3_SWAP;
//! let usdc_address = blockchain::tokens::USDC;
//!
//! // Use protocol constants
//! let magic = protocol::MAGIC_NUMBER;
//! let market_data_range = protocol::tlv::MARKET_DATA_RANGE;
//! ```

pub mod protocol;
pub mod service;

// Re-export commonly used types
pub use protocol::*;
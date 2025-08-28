//! # Torq Centralized Configuration
//!
//! This crate provides centralized configuration management and constants
//! for all Torq services, eliminating duplication across the codebase.
//!
//! ## Features
//!
//! - **Blockchain Constants**: Event signatures, token addresses, DEX routers
//! - **Protocol V2 Constants**: Magic numbers, TLV domain ranges, message sizes
//! - **Financial Constants**: Precision multipliers, decimal places
//! - **Service Configuration**: Default values, timeouts, performance targets
//!
//! ## Usage
//!
//! ```rust
//! use torq_config::{blockchain, protocol, financial};
//!
//! // Use blockchain constants
//! let swap_signature = blockchain::events::UNISWAP_V3_SWAP;
//! let usdc_address = blockchain::tokens::USDC;
//!
//! // Use protocol constants
//! let magic = protocol::MAGIC_NUMBER;
//! let market_data_range = protocol::tlv::MARKET_DATA_RANGE;
//!
//! // Use financial constants
//! let usd_multiplier = financial::USD_FIXED_POINT_MULTIPLIER;
//! ```

pub mod blockchain;
pub mod financial;
pub mod protocol;
pub mod service;

// Re-export commonly used types
pub use blockchain::*;
pub use financial::*;
pub use protocol::*;
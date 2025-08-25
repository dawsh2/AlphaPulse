//! # AlphaPulse Types Library
//!
//! Shared type definitions for the AlphaPulse trading system, providing
//! type-safe fixed-point arithmetic for financial calculations.
//!
//! ## Design Philosophy
//!
//! - **No Precision Loss**: All financial values stored as scaled integers
//! - **Type Safety**: Distinct types for different scales prevent mixing
//! - **Overflow Protection**: Comprehensive checked arithmetic
//! - **Clear Boundaries**: Explicit conversion points between floating and fixed point
//! - **Performance**: Direct integer operations after validation
//!
//! ## Usage Examples
//!
//! ```rust
//! use alphapulse_types::{UsdFixedPoint8, PercentageFixedPoint4};
//!
//! // Parse from decimal strings (primary method)
//! let price = UsdFixedPoint8::from_decimal_str("42.12345678").unwrap();
//! let spread = PercentageFixedPoint4::from_decimal_str("0.25").unwrap();
//!
//! // Safe f64 conversion for AMM boundaries
//! let amm_result = UsdFixedPoint8::try_from_f64(123.456).unwrap();
//!
//! // Checked arithmetic for critical calculations
//! let fee = UsdFixedPoint8::ONE_CENT;
//! if let Some(total) = price.checked_add(fee) {
//!     // Process successful calculation
//!     println!("Total: {}", total);
//! } else {
//!     // Handle overflow
//!     println!("Overflow occurred");
//! }
//!
//! // Saturating arithmetic for analytics/display
//! let large_amount = UsdFixedPoint8::from_raw(i64::MAX / 2);
//! let capped_value = price.saturating_add(large_amount);
//! ```
//!
//! ## Integration Points
//!
//! This library is used throughout the AlphaPulse system:
//! - **Strategy Services**: Arbitrage profit calculations
//! - **Portfolio Management**: Position value tracking
//! - **Risk Management**: Exposure calculations
//! - **Dashboard Services**: Display formatting
//! - **Protocol V2 TLV**: Fixed-point value serialization

pub mod errors;
pub mod fixed_point;

// Re-export main types for convenience
pub use errors::FixedPointError;
pub use fixed_point::{PercentageFixedPoint4, UsdFixedPoint8};

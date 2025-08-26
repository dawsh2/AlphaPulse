//! Financial Precision Unit Tests
//!
//! Critical tests for precision handling in financial calculations:
//! - Native token precision preservation (18 decimals WETH, 6 USDC)
//! - 8-decimal fixed-point for USD prices
//! - No precision loss in conversions
//! - Decimal boundary validation

pub mod token_precision_tests;
pub mod price_precision_tests;
pub mod conversion_tests;
pub mod boundary_tests;
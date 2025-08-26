//! Property-Based Tests - Specialized Financial Testing
//!
//! Property-based tests validate financial calculations across wide ranges
//! of inputs to catch edge cases, precision errors, and mathematical invariants.
//!
//! ## Test Categories
//! - `arbitrage/` - Arbitrage calculation properties
//! - `precision/` - Financial precision invariants
//! - `amm_math/` - AMM mathematical properties
//! - `risk/` - Risk calculation validation

pub mod arbitrage;
pub mod precision;
pub mod amm_math;
pub mod risk;
//! Unit Tests - Layer 1 of Testing Pyramid
//!
//! Fast, isolated tests that validate individual functions and components.
//! These tests run in milliseconds and test specific behaviors without
//! external dependencies.
//!
//! ## Structure
//! - `core/` - Core protocol functionality (headers, parsing, validation)
//! - `tlv/` - TLV message construction and parsing
//! - `identifiers/` - Bijective ID system tests
//! - `precision/` - Financial precision validation
//! - `performance/` - Micro-benchmarks for critical paths

pub mod core;
pub mod tlv;
pub mod identifiers;
pub mod precision;
pub mod performance;
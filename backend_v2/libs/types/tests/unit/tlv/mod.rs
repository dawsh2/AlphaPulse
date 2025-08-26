//! TLV (Type-Length-Value) Unit Tests
//!
//! Tests for TLV message construction, parsing, and validation:
//! - TLVMessageBuilder correctness
//! - TLV type registration and bounds
//! - Payload size calculations
//! - Zero-copy parsing validation

pub mod builder_tests;
pub mod parser_tests;
pub mod types_tests;
pub mod zerocopy_tests;
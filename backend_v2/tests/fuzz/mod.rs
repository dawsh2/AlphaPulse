//! Fuzz Tests - Security and Robustness Testing
//!
//! Fuzz testing finds security vulnerabilities and crashes by feeding
//! malformed or unexpected data to the system. Critical for financial
//! systems where malicious actors may try to exploit parsing vulnerabilities.
//!
//! ## Test Categories
//! - `tlv_parser/` - TLV message parsing robustness
//! - `message_validation/` - Protocol validation edge cases
//! - `precision_handling/` - Financial calculation overflow/underflow
//! - `network_messages/` - Network message corruption handling

pub mod tlv_parser;
pub mod message_validation;
pub mod precision_handling;
pub mod network_messages;
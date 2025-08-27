//! Architecture validation tests
//! 
//! This module contains comprehensive tests to validate the architecture
//! and design patterns of the AlphaPulse system according to CLAUDE.md specifications.
//!
//! Validates:
//! 1. Protocol V2 compliance (TLV structure, magic numbers, etc.)
//! 2. Precision preservation (no floating point for prices)
//! 3. No mock data or services
//! 4. Proper file organization and project structure
//! 5. No duplicate implementations
//! 6. Performance requirements
//! 7. Breaking changes handling
//! 8. Documentation standards

pub mod protocol_v2_compliance;
pub mod precision_validation;
pub mod mock_detection;
pub mod file_organization;
pub mod duplicate_detection;
pub mod performance_validation;
pub mod breaking_changes;
pub mod documentation_standards;
pub mod common;

// Re-export common utilities
pub use common::*;
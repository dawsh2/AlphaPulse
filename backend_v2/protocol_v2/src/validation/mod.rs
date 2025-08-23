//! Message Validation
//!
//! Provides validation functions for message integrity and safety

pub mod bounds;
pub mod checksum;

pub use bounds::*;
pub use checksum::*;

// Re-export TLVSizeConstraint as SizeConstraint for backwards compatibility
pub use crate::tlv::types::TLVSizeConstraint as SizeConstraint;

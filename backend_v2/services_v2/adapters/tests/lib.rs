//! Adapter Test Suite
//!
//! Validation framework-based tests using real exchange data.
//! All tests demonstrate proper usage of the four-step validation pipeline.

pub mod fixtures;
pub mod integration;
pub mod validation;

// Re-export validation framework for tests
pub use alphapulse_adapter_service::validation::{
    complete_validation_pipeline, validate_equality, RawDataValidator, SemanticValidator,
    ValidationError, ValidationResult,
};

// Re-export Protocol V2 types for tests
pub use alphapulse_protocol_v2::{
    tlv::market_data::{PoolBurnTLV, PoolLiquidityTLV, PoolMintTLV, PoolSwapTLV, PoolTickTLV},
    InstrumentId, VenueId,
};

/// Test configuration constants
pub mod config {
    /// Maximum validation time per event (production requirement)
    pub const MAX_VALIDATION_TIME_MS: u64 = 10;

    /// Test fixture directory
    pub const FIXTURE_PATH: &str = "tests/fixtures";
}

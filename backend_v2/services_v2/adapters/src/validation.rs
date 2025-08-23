//! # Adapter Validation Framework
//!
//! Provides both runtime and optional compile-time validation for adapters.
//! This framework ensures zero data loss through the complete serialization pipeline.
//!
//! ## Quick Start
//!
//! ### Runtime Validation (Default - Development Only)
//! ```rust
//! use alphapulse_adapters::validation::{complete_validation_pipeline, ValidationConfig};
//!
//! // During development - test your adapter with real data
//! let result = complete_validation_pipeline(raw_data, parsed_data)?;
//! println!("✅ Validation passed - zero data loss confirmed");
//! ```
//!
//! ### Compile-Time Validation (Optional - Production Safety)
//! Enable with `--features strict-validation` to enforce validation at compile time:
//!
//! ```toml
//! # In your Cargo.toml
//! [features]
//! strict-validation = ["alphapulse_adapters/strict-validation"]
//! ```
//!
//! ```rust
//! # #[cfg(feature = "strict-validation")]
//! use alphapulse_adapters::validation::ValidatedAdapter;
//!
//! # #[cfg(feature = "strict-validation")]
//! impl ValidatedAdapter for MyAdapter {
//!     const VALIDATION_IMPLEMENTED: bool = true;
//!     
//!     fn validate_implementation() -> Result<(), ValidationError> {
//!         // Must implement the four-step validation pipeline
//!         // This ensures production adapters have proper validation tests
//!         println!("✅ MyAdapter validation implemented");
//!         Ok(())
//!     }
//! }
//! ```
//!
//! ## Four-Step Validation Process
//!
//! Every adapter MUST complete this pipeline during development:
//!
//! | Step | Purpose | Common Failures Caught |
//! |------|---------|-------------------------|
//! | 1. Raw Parsing | External → Struct | Missing fields, wrong types, precision loss |
//! | 2. Serialization | Struct → Binary | Overflow, encoding errors |
//! | 3. Deserialization | Binary → Struct | Corruption, alignment issues |
//! | 4. Deep Equality | Round-trip check | Any data loss |
//!
//! ### Complete Example
//! ```rust
//! # use alphapulse_adapters::validation::*;
//! # use protocol_v2::tlv::market_data::PoolSwapTLV;
//! # use protocol_v2::VenueId;
//!
//! #[test]
//! fn test_my_adapter_validation() -> ValidationResult<()> {
//!     // Real fixture data (never use synthetic!)
//!     let raw_data = load_real_fixture("my_exchange_data.json");
//!     let parsed = MyExchangeEvent::from_json(&raw_data)?;
//!     
//!     // Complete four-step pipeline
//!     let validated = complete_validation_pipeline(
//!         raw_data.as_bytes(),
//!         parsed
//!     )?;
//!     
//!     println!("✅ Perfect round-trip: zero data loss confirmed");
//!     Ok(())
//! }
//! ```
//!
//! ## Common Validation Failures & Solutions
//!
//! ### "Price precision lost"
//! **Cause**: Using `f64` instead of `Decimal`/fixed-point
//! ```rust
//! // ❌ WRONG - precision loss
//! let price: f64 = 123.456789;
//!
//! // ✅ CORRECT - exact precision
//! use rust_decimal::{Decimal, prelude::FromStr};
//! let price = Decimal::from_str("123.456789")?;
//! let fixed_point = (price * Decimal::from(100_000_000)).to_i64()?;
//! ```
//!
//! ### "Segmentation fault in tests"
//! **Cause**: Direct access to packed struct fields
//! ```rust
//! // ❌ WRONG - unaligned access on ARM/M1
//! assert_eq!(tlv.price, expected);
//!
//! // ✅ CORRECT - copy packed fields first
//! let price = tlv.price;  // Copy to stack
//! assert_eq!(price, expected);
//! ```
//!
//! ### "InstrumentId mismatch"
//! **Cause**: Inconsistent symbol normalization
//! ```rust
//! // Problem: BTC-USD vs BTC/USD vs BTCUSD
//! // Solution: Always normalize consistently
//! let normalized = symbol.replace('-', "/").replace('_', "/");
//! let id = InstrumentId::coin(venue, &normalized);
//! ```
//!
//! ## Performance Requirements
//!
//! Validation should complete quickly during development:
//! - Step 1 (Parsing): <1ms per message
//! - Step 2 (Serialization): <100μs per message
//! - Step 3 (Deserialization): <50μs per message
//! - Step 4 (Equality): <10μs per comparison
//! - **Total: <2ms per complete validation**
//!
//! ## Validation vs Production
//!
//! **CRITICAL**: Validation is development-time only!
//!
//! ```rust
//! // ✅ In tests and development
//! #[test]
//! fn validate_my_adapter() {
//!     complete_validation_pipeline(data, parsed).unwrap();
//! }
//!
//! // ❌ NOT in production hot path
//! async fn process_message(&self, msg: &str) {
//!     let parsed = parse_message(msg)?;
//!     // complete_validation_pipeline(parsed)?; // ❌ Too slow!
//!     self.send_tlv(parsed.into()).await?;  // ✅ Direct processing
//! }
//! ```

use crate::error::AdapterError;
use protocol_v2::tlv::market_data::*;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub type ValidationResult<T> = Result<T, ValidationError>;

/// Compile-time validation enforcement trait
///
/// Enable with `--features strict-validation` to require validation implementation.
/// This ensures production adapters have proper validation tests.
///
/// # Example
/// ```rust
/// # use alphapulse_adapters::validation::{ValidatedAdapter, ValidationResult};
/// # struct MyAdapter;
/// # #[cfg(feature = "strict-validation")]
/// impl ValidatedAdapter for MyAdapter {
///     const VALIDATION_IMPLEMENTED: bool = true;
///     
///     fn validate_implementation() -> ValidationResult<()> {
///         // Your complete four-step validation pipeline here
///         println!("✅ MyAdapter validation implemented");
///         Ok(())
///     }
/// }
/// ```
///
/// # When to Use
///
/// - **Development**: Optional, helps catch missing validation
/// - **CI/CD**: Enable in build pipeline to ensure all adapters have tests
/// - **Production**: Required for adapters handling real money
#[cfg(feature = "strict-validation")]
pub trait ValidatedAdapter {
    /// Must be set to true to indicate validation is implemented
    const VALIDATION_IMPLEMENTED: bool;

    /// Validate that the adapter implementation is correct
    ///
    /// This should run the complete four-step validation pipeline
    /// with real fixture data to ensure zero data loss.
    ///
    /// # Requirements
    ///
    /// 1. Load real fixture data (minimum 10 samples)
    /// 2. Run [`complete_validation_pipeline`] on all samples
    /// 3. Verify performance targets (<2ms per validation)
    /// 4. Test edge cases (min/max values, empty fields)
    fn validate_implementation() -> ValidationResult<()>;
}

/// Configuration for validation behavior
///
/// Controls how strict the validation framework should be during development.
///
/// # Example
/// ```rust
/// use alphapulse_adapters::validation::ValidationConfig;
///
/// let config = ValidationConfig {
///     max_validation_time_ms: 5,  // Allow up to 5ms per validation
///     min_fixture_count: 20,      // Require 20+ test samples
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Maximum acceptable validation time per message
    pub max_validation_time_ms: u64,
    /// Minimum number of fixtures required for testing
    pub min_fixture_count: usize,
    /// Require fixture files for testing (recommended: true)
    pub require_fixtures: bool,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            max_validation_time_ms: 2,
            min_fixture_count: 10,
            require_fixtures: true,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Raw parsing validation failed: {0}")]
    RawParsing(String),

    #[error("TLV serialization validation failed: {0}")]
    TlvSerialization(String),

    #[error("TLV deserialization validation failed: {0}")]
    TlvDeserialization(String),

    #[error("Semantic validation failed: {0}")]
    Semantic(String),

    #[error("Deep equality validation failed: {0}")]
    DeepEquality(String),

    #[error("Precision loss detected: {0}")]
    PrecisionLoss(String),

    #[error("Cross-source validation failed: {0}")]
    CrossSource(String),
}

/// Trait for validating raw parsed data before TLV conversion
pub trait RawDataValidator {
    /// Validate that all required fields are present and reasonable
    fn validate_required_fields(&self) -> ValidationResult<()>;

    /// Validate that data types match provider specification
    fn validate_types_against_spec(&self) -> ValidationResult<()>;

    /// Validate that field values are within reasonable ranges
    fn validate_field_ranges(&self) -> ValidationResult<()>;

    /// Validate that no precision was lost during parsing
    fn validate_precision_preserved(&self) -> ValidationResult<()>;
}

/// Trait for semantic validation of TLV structures
pub trait SemanticValidator {
    /// Validate that fields have correct semantic meaning
    fn validate_semantics(&self) -> ValidationResult<()>;

    /// Validate that field values are within expected ranges
    fn validate_ranges(&self) -> ValidationResult<()>;
}

/// Step 1: Validate Raw Data Parsing
pub fn validate_raw_parsing<T: RawDataValidator>(
    _raw_data: &[u8],
    parsed: &T,
) -> ValidationResult<()> {
    // 1. All required fields extracted
    parsed
        .validate_required_fields()
        .map_err(|e| ValidationError::RawParsing(format!("Missing required fields: {}", e)))?;

    // 2. Data types match provider specification
    parsed
        .validate_types_against_spec()
        .map_err(|e| ValidationError::RawParsing(format!("Type mismatch: {}", e)))?;

    // 3. Field values are reasonable
    parsed
        .validate_field_ranges()
        .map_err(|e| ValidationError::RawParsing(format!("Invalid field ranges: {}", e)))?;

    // 4. No truncation or corruption during parsing
    parsed
        .validate_precision_preserved()
        .map_err(|e| ValidationError::RawParsing(format!("Precision lost: {}", e)))?;

    Ok(())
}

/// Step 2: Validate TLV Serialization
pub fn validate_tlv_serialization<T: SemanticValidator + Clone>(
    tlv: &T,
) -> ValidationResult<Vec<u8>>
where
    T: TlvSerializable,
{
    // 1. Semantic validation before serialization
    tlv.validate_semantics().map_err(|e| {
        ValidationError::TlvSerialization(format!("Semantic validation failed: {}", e))
    })?;

    // 2. Serialize to bytes
    let bytes = tlv.to_bytes();

    // 3. Validate serialized format
    if bytes.is_empty() {
        return Err(ValidationError::TlvSerialization(
            "Serialization produced empty bytes".to_string(),
        ));
    }

    if bytes.len() > 255 {
        return Err(ValidationError::TlvSerialization(format!(
            "TLV payload too large: {} bytes",
            bytes.len()
        )));
    }

    // 4. Check expected byte structure
    validate_tlv_byte_structure(&bytes)?;

    Ok(bytes)
}

/// Step 3: Validate TLV Deserialization
pub fn validate_tlv_deserialization<T: SemanticValidator>(bytes: &[u8]) -> ValidationResult<T>
where
    T: TlvDeserializable,
{
    // 1. Deserialize from bytes
    let recovered = T::from_bytes(bytes).map_err(|e| {
        ValidationError::TlvDeserialization(format!("Deserialization failed: {}", e))
    })?;

    // 2. Semantic validation on deserialized data
    recovered.validate_semantics().map_err(|e| {
        ValidationError::TlvDeserialization(format!(
            "Post-deserialization semantic validation failed: {}",
            e
        ))
    })?;

    // 3. Range validation
    recovered.validate_ranges().map_err(|e| {
        ValidationError::TlvDeserialization(format!(
            "Post-deserialization range validation failed: {}",
            e
        ))
    })?;

    Ok(recovered)
}

/// Step 4: Validate Semantic & Deep Equality
pub fn validate_equality<T: PartialEq + Hash + Clone>(
    original: &T,
    recovered: &T,
) -> ValidationResult<()>
where
    T: TlvSerializable,
{
    // 1. Deep equality - byte-for-byte identical
    if original != recovered {
        return Err(ValidationError::DeepEquality(
            "Deep equality failed".to_string(),
        ));
    }

    // 2. Hash comparison - extra verification
    let original_hash = hash_tlv(original);
    let recovered_hash = hash_tlv(recovered);

    if original_hash != recovered_hash {
        return Err(ValidationError::DeepEquality(
            "Hash equality failed".to_string(),
        ));
    }

    // 3. Re-serialization produces identical bytes
    let original_bytes = original.to_bytes();
    let recovered_bytes = recovered.to_bytes();

    if original_bytes != recovered_bytes {
        return Err(ValidationError::DeepEquality(
            "Re-serialization differs".to_string(),
        ));
    }

    Ok(())
}

/// Complete Four-Step Validation Pipeline
pub fn complete_validation_pipeline<Raw, Tlv>(raw_data: &[u8], parsed: Raw) -> ValidationResult<Tlv>
where
    Raw: RawDataValidator,
    Tlv: SemanticValidator + PartialEq + Hash + Clone + TlvSerializable + TlvDeserializable,
    Tlv: From<Raw>,
{
    // STEP 1: Validate raw data parsing
    validate_raw_parsing(raw_data, &parsed)?;

    // Transform to TLV
    let original_tlv = Tlv::from(parsed);

    // STEP 2: Validate TLV serialization
    let bytes = validate_tlv_serialization(&original_tlv)?;

    // STEP 3: Validate TLV deserialization
    let recovered_tlv: Tlv = validate_tlv_deserialization(&bytes)?;

    // STEP 4: Validate semantic & deep equality
    validate_equality(&original_tlv, &recovered_tlv)?;

    Ok(recovered_tlv)
}

/// Validate TLV byte structure
fn validate_tlv_byte_structure(bytes: &[u8]) -> ValidationResult<()> {
    if bytes.len() < 2 {
        return Err(ValidationError::TlvSerialization(
            "TLV too short, missing header".to_string(),
        ));
    }

    // Check that declared length matches actual payload
    let declared_length = bytes[1] as usize;
    if bytes.len() != 2 + declared_length {
        return Err(ValidationError::TlvSerialization(format!(
            "TLV length mismatch: declared {} but got {} bytes",
            declared_length,
            bytes.len() - 2
        )));
    }

    Ok(())
}

/// Hash a TLV structure for equality verification
fn hash_tlv<T: Hash>(tlv: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    tlv.hash(&mut hasher);
    hasher.finish()
}

// Trait abstractions for the validation framework
pub trait TlvSerializable {
    fn to_bytes(&self) -> Vec<u8>;
}

pub trait TlvDeserializable: Sized {
    fn from_bytes(bytes: &[u8]) -> Result<Self, String>;
}

// Implement validation for Protocol V2 TLV types

impl SemanticValidator for PoolSwapTLV {
    fn validate_semantics(&self) -> ValidationResult<()> {
        // Validate amounts are positive
        if self.amount_in == 0 {
            return Err(ValidationError::Semantic(
                "amount_in must be positive".to_string(),
            ));
        }

        if self.amount_out == 0 {
            return Err(ValidationError::Semantic(
                "amount_out must be positive".to_string(),
            ));
        }

        // Validate pool address is not zero
        if self.pool_address == [0u8; 20] {
            return Err(ValidationError::Semantic(
                "Pool address cannot be zero".to_string(),
            ));
        }

        // Validate decimals are reasonable (0-30)
        if self.amount_in_decimals > 30 || self.amount_out_decimals > 30 {
            return Err(ValidationError::Semantic(
                "Decimal places too high (max 30)".to_string(),
            ));
        }

        // Validate tick_after is within Uniswap V3 bounds
        if self.tick_after < -887272 || self.tick_after > 887272 {
            return Err(ValidationError::Semantic("Tick out of bounds".to_string()));
        }

        Ok(())
    }

    fn validate_ranges(&self) -> ValidationResult<()> {
        // Only validate structural correctness - no business logic limits

        // Note: sqrt_price_x96_after can be zero for V2 pools (they don't use this field)
        // Only V3 pools require non-zero sqrt_price values, but we can't distinguish
        // V2 vs V3 at this level since the TLV doesn't include protocol information.
        // The validation that sqrt_price cannot be zero was incorrectly applied to all swaps.
        // V2 swaps legitimately have [0u8; 20] for sqrt_price_x96_after.

        Ok(())
    }
}

impl SemanticValidator for PoolMintTLV {
    fn validate_semantics(&self) -> ValidationResult<()> {
        // Validate pool address
        if self.pool_address == [0u8; 20] {
            return Err(ValidationError::Semantic(
                "Pool address cannot be zero".to_string(),
            ));
        }

        // Validate tick range
        if self.tick_lower >= self.tick_upper {
            return Err(ValidationError::Semantic(
                "tick_lower must be less than tick_upper".to_string(),
            ));
        }

        if self.tick_lower < -887272 || self.tick_upper > 887272 {
            return Err(ValidationError::Semantic("Ticks out of bounds".to_string()));
        }

        // Validate liquidity delta is not zero (would be pointless mint)
        if self.liquidity_delta == 0 {
            return Err(ValidationError::Semantic(
                "liquidity_delta cannot be zero".to_string(),
            ));
        }

        Ok(())
    }

    fn validate_ranges(&self) -> ValidationResult<()> {
        // Only structural validation - no business logic limits
        Ok(())
    }
}

impl SemanticValidator for PoolBurnTLV {
    fn validate_semantics(&self) -> ValidationResult<()> {
        // Same validations as mint
        if self.pool_address == [0u8; 20] {
            return Err(ValidationError::Semantic(
                "Pool address cannot be zero".to_string(),
            ));
        }

        if self.tick_lower >= self.tick_upper {
            return Err(ValidationError::Semantic(
                "tick_lower must be less than tick_upper".to_string(),
            ));
        }

        if self.liquidity_delta == 0 {
            return Err(ValidationError::Semantic(
                "liquidity_delta cannot be zero".to_string(),
            ));
        }

        Ok(())
    }

    fn validate_ranges(&self) -> ValidationResult<()> {
        // Only structural validation - no business logic limits
        Ok(())
    }
}

impl SemanticValidator for PoolTickTLV {
    fn validate_semantics(&self) -> ValidationResult<()> {
        // Validate tick bounds
        if self.tick < -887272 || self.tick > 887272 {
            return Err(ValidationError::Semantic("Tick out of bounds".to_string()));
        }

        // Validate price_sqrt is not zero
        if self.price_sqrt == 0 {
            return Err(ValidationError::Semantic(
                "price_sqrt cannot be zero".to_string(),
            ));
        }

        Ok(())
    }

    fn validate_ranges(&self) -> ValidationResult<()> {
        // Only structural validation - no business logic limits
        // liquidity_net can be any valid i64 value (positive or negative)
        Ok(())
    }
}

impl SemanticValidator for PoolLiquidityTLV {
    fn validate_semantics(&self) -> ValidationResult<()> {
        // Validate pool address
        if self.pool_address == [0u8; 20] {
            return Err(ValidationError::Semantic(
                "Pool address cannot be zero".to_string(),
            ));
        }

        // Validate reserves array is not empty and reasonable size
        if self.reserves.is_empty() {
            return Err(ValidationError::Semantic(
                "Reserves array cannot be empty".to_string(),
            ));
        }

        if self.reserves.len() > 10 {
            return Err(ValidationError::Semantic(
                "Too many reserves (max 10)".to_string(),
            ));
        }

        Ok(())
    }

    fn validate_ranges(&self) -> ValidationResult<()> {
        // Only structural validation - no business logic limits
        // Reserves can be any valid u128 value from provider
        Ok(())
    }
}

// Implement TLV serialization traits for Protocol V2 types
macro_rules! impl_tlv_traits {
    ($tlv_type:ty, $tlv_type_value:expr) => {
        impl TlvSerializable for $tlv_type {
            fn to_bytes(&self) -> Vec<u8> {
                // Create proper TLV format: [type][length][payload]
                // Call the struct's own to_bytes() method to get the payload
                let payload = <$tlv_type>::to_bytes(self);
                let mut tlv_bytes = Vec::new();
                tlv_bytes.push($tlv_type_value); // TLV type byte
                tlv_bytes.push(payload.len() as u8); // Length byte
                tlv_bytes.extend(payload); // Payload
                tlv_bytes
            }
        }

        impl TlvDeserializable for $tlv_type {
            fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
                // Parse TLV format: [type][length][payload]
                if bytes.len() < 2 {
                    return Err("TLV too short".to_string());
                }

                let tlv_type = bytes[0];
                let payload_len = bytes[1] as usize;

                if tlv_type != $tlv_type_value {
                    return Err(format!(
                        "Wrong TLV type: expected {}, got {}",
                        $tlv_type_value, tlv_type
                    ));
                }

                if bytes.len() != 2 + payload_len {
                    return Err(format!(
                        "TLV length mismatch: declared {} but total is {} bytes",
                        payload_len,
                        bytes.len()
                    ));
                }

                let payload = &bytes[2..2 + payload_len];
                <$tlv_type>::from_bytes(payload).map_err(|e| e.to_string())
            }
        }
    };
}

impl_tlv_traits!(PoolSwapTLV, 11); // TLVType::PoolSwap = 11
impl_tlv_traits!(PoolMintTLV, 12); // TLVType::PoolMint = 12
impl_tlv_traits!(PoolBurnTLV, 13); // TLVType::PoolBurn = 13
impl_tlv_traits!(PoolTickTLV, 14); // TLVType::PoolTick = 14
impl_tlv_traits!(PoolLiquidityTLV, 10); // TLVType::PoolLiquidity = 10

#[cfg(test)]
mod tests {
    use super::*;
    use protocol_v2::VenueId;

    #[test]
    fn test_pool_swap_semantic_validation_success() {
        let swap = PoolSwapTLV {
            venue: VenueId::Polygon,
            pool_address: [1u8; 20], // Non-zero address
            token_in_addr: [2u8; 20],
            token_out_addr: [3u8; 20],
            amount_in: 1000000000000000000, // 1 token (18 decimals)
            amount_out: 2000000000,         // 2000 tokens (6 decimals)
            amount_in_decimals: 18,
            amount_out_decimals: 6,
            tick_after: 123456, // Valid tick
            sqrt_price_x96_after: PoolSwapTLV::sqrt_price_from_u128(1000000000000000000000000), // Non-zero sqrt price
            liquidity_after: 5000000000000000000, // Some liquidity
            timestamp_ns: 1000000000000000000,
            block_number: 45000000,
        };

        assert!(swap.validate_semantics().is_ok());
        assert!(swap.validate_ranges().is_ok());
    }

    #[test]
    fn test_pool_swap_semantic_validation_failures() {
        // Test zero amount_in
        let mut swap = PoolSwapTLV {
            venue: VenueId::Polygon,
            pool_address: [1u8; 20],
            token_in_addr: [2u8; 20],
            token_out_addr: [3u8; 20],
            amount_in: 0, // Invalid: zero
            amount_out: 2000000000,
            amount_in_decimals: 18,
            amount_out_decimals: 6,
            tick_after: 123456,
            sqrt_price_x96_after: PoolSwapTLV::sqrt_price_from_u128(1000000000000000000000000),
            liquidity_after: 5000000000000000000,
            timestamp_ns: 1000000000000000000,
            block_number: 45000000,
        };

        assert!(swap.validate_semantics().is_err());

        // Test zero pool address
        swap.amount_in = 1000000000000000000;
        swap.pool_address = [0u8; 20]; // Invalid: zero address
        assert!(swap.validate_semantics().is_err());

        // Test tick out of bounds
        swap.pool_address = [1u8; 20];
        swap.tick_after = -1000000; // Invalid: out of bounds
        assert!(swap.validate_semantics().is_err());

        // Test excessive decimals
        swap.tick_after = 123456;
        swap.amount_in_decimals = 50; // Invalid: too many decimals
        assert!(swap.validate_semantics().is_err());
    }

    #[test]
    fn test_complete_validation_pipeline() {
        // This would require implementing RawDataValidator for a test type
        // Keeping simple for now - real implementation will be in adapter-specific code
    }

    #[test]
    fn test_deep_equality_validation() {
        let swap1 = PoolSwapTLV {
            venue: VenueId::Polygon,
            pool_address: [1u8; 20],
            token_in_addr: [2u8; 20],
            token_out_addr: [3u8; 20],
            amount_in: 1000000000000000000,
            amount_out: 2000000000,
            amount_in_decimals: 18,
            amount_out_decimals: 6,
            tick_after: 123456,
            sqrt_price_x96_after: PoolSwapTLV::sqrt_price_from_u128(1000000000000000000000000),
            liquidity_after: 5000000000000000000,
            timestamp_ns: 1000000000000000000,
            block_number: 45000000,
        };

        let swap2 = swap1.clone();

        // Should pass equality validation
        assert!(validate_equality(&swap1, &swap2).is_ok());

        // Should fail if different
        let mut swap3 = swap1.clone();
        swap3.amount_in = 2000000000000000000; // Different amount
        assert!(validate_equality(&swap1, &swap3).is_err());
    }

    #[test]
    fn test_tlv_serialization_fix() {
        println!("🔧 Testing TLV serialization fix...");

        let swap = PoolSwapTLV {
            venue: VenueId::Polygon,
            pool_address: [1u8; 20],
            token_in_addr: [2u8; 20],
            token_out_addr: [3u8; 20],
            amount_in: 1000000000000000000,
            amount_out: 2000000000,
            amount_in_decimals: 18,
            amount_out_decimals: 6,
            tick_after: 123456,
            sqrt_price_x96_after: PoolSwapTLV::sqrt_price_from_u128(1000000000000000000000000),
            liquidity_after: 5000000000000000000,
            timestamp_ns: 1000000000000000000,
            block_number: 45000000,
        };

        // Test native serialization (should be ~152 bytes based on struct layout with uint160)
        let native_bytes = PoolSwapTLV::to_bytes(&swap);
        println!("Native PoolSwapTLV bytes: {}", native_bytes.len());
        assert_eq!(native_bytes.len(), 152); // Actual size: 148 + 4 bytes for uint160 precision increase

        // Test TLV serialization through validation framework
        let tlv_bytes = TlvSerializable::to_bytes(&swap);
        println!(
            "TLV format bytes: {} (should be 154: 1 type + 1 length + 152 payload)",
            tlv_bytes.len()
        );
        assert_eq!(tlv_bytes.len(), 154); // 1 (type) + 1 (length) + 152 (payload)
        assert_eq!(tlv_bytes[0], 11); // TLVType::PoolSwap
        assert_eq!(tlv_bytes[1], 152); // Payload length

        // Test validation - should not have length mismatch anymore
        let result = validate_tlv_byte_structure(&tlv_bytes);
        if let Err(e) = &result {
            println!("❌ TLV validation error: {}", e);
        } else {
            println!("✅ TLV validation successful");
        }
        assert!(result.is_ok(), "TLV validation should succeed");

        // Test round-trip deserialization
        let recovered = TlvDeserializable::from_bytes(&tlv_bytes).unwrap();
        assert_eq!(swap, recovered, "Round-trip should preserve data");

        println!("✅ TLV serialization fix verified!");
    }
}

//! Error types for fixed-point arithmetic and identifier validation
//!
//! Provides comprehensive error handling for overflow, underflow, and conversion
//! failures in financial calculations, as well as validation failures for typed IDs.

use thiserror::Error;

/// Errors that can occur during typed ID validation
#[derive(Debug, Error, Clone, PartialEq)]
pub enum ValidationError {
    /// ID value is null/zero when non-null required
    #[error("ID cannot be null/zero")]
    NullId,

    /// ID value exceeds maximum allowed value
    #[error("ID value {value} exceeds maximum allowed value {max}")]
    ValueTooLarge { value: u64, max: u64 },

    /// ID value is below minimum allowed value
    #[error("ID value {value} is below minimum allowed value {min}")]
    ValueTooSmall { value: u64, min: u64 },

    /// ID value is not within allowed range
    #[error("ID value {value} is not in allowed range [{min}, {max}]")]
    ValueOutOfRange { value: u64, min: u64, max: u64 },

    /// Reserved ID value that should not be used
    #[error("ID value {value} is reserved and cannot be used")]
    ReservedValue { value: u64 },

    /// Custom validation failure with message
    #[error("Validation failed: {message}")]
    Custom { message: String },
}

/// Errors that can occur during fixed-point arithmetic operations
#[derive(Debug, Error, Clone, PartialEq)]
pub enum FixedPointError {
    /// Value exceeds the maximum representable value for the type
    #[error("Overflow: value {value} exceeds maximum representable value")]
    Overflow { value: f64 },

    /// Value is below the minimum representable value for the type
    #[error("Underflow: value {value} is below minimum representable value")]
    Underflow { value: f64 },

    /// Invalid decimal string format
    #[error("Invalid decimal string: '{input}' - expected numeric format")]
    InvalidDecimal { input: String },

    /// Division by zero in fixed-point arithmetic
    #[error("Division by zero in fixed-point arithmetic")]
    DivisionByZero,

    /// Precision loss during conversion
    #[error("Precision loss: value {original} cannot be represented exactly")]
    PrecisionLoss { original: f64 },

    /// Value is not finite (NaN or infinity)
    #[error("Value is not finite: {value}")]
    NotFinite { value: f64 },

    /// Invalid format for identifier creation
    #[error("Invalid format: {0}")]
    InvalidFormat(String),
}

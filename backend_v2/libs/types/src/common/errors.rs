//! Error types for fixed-point arithmetic operations
//!
//! Provides comprehensive error handling for overflow, underflow, and conversion
//! failures in financial calculations to ensure system safety.

use thiserror::Error;

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

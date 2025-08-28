//! End-to-End Test Framework for Torq
//!
//! Comprehensive testing suite that validates the entire system from
//! exchange data ingestion through strategy execution to dashboard display.

pub mod fixtures;
pub mod framework;
pub mod scenarios;
pub mod validation;
// pub mod orchestration; // Commented out - file not found

pub use fixtures::*;
pub use framework::{TestFramework, TestResult, TestScenario};
pub use scenarios::*;
pub use validation::*;

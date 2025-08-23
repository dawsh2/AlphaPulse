//! End-to-End Test Framework for AlphaPulse
//!
//! Comprehensive testing suite that validates the entire system from
//! exchange data ingestion through strategy execution to dashboard display.

pub mod framework;
pub mod scenarios;
pub mod fixtures;
pub mod validation;
pub mod orchestration;

pub use framework::{TestFramework, TestResult, TestScenario};
pub use scenarios::*;
pub use fixtures::*;
pub use validation::*;
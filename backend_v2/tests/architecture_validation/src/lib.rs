//! Architecture validation tests
//! 
//! This module contains comprehensive tests to validate the architecture
//! and design patterns of the Torq system according to AUDIT-007 specifications.
//!
//! Validates:
//! 1. Services use codec library consistently (no protocol duplication)
//! 2. Plugin compliance for adapter trait implementations
//! 3. Typed ID usage instead of raw primitives
//! 4. Code quality patterns (no hardcoded values, proper error handling)
//! 5. Proper configuration usage
//! 6. No mock data in production code

pub mod common;
pub mod dependency_validation;
pub mod plugin_compliance;
pub mod typed_id_usage;
pub mod code_quality;
pub mod advanced_validations;

// Re-export common utilities
pub use common::*;

use anyhow::Result;
use colored::Colorize;

/// Run all architecture validation tests
pub fn run_all_validations() -> Result<()> {
    println!("{}", "Running Architecture Validation Tests".bold().cyan());
    println!("{}", "=".repeat(50).cyan());

    let metadata = get_workspace_metadata()?;

    let mut all_passed = true;

    let tests = vec![
        // Dependency and import validations
        dependency_validation::validate_codec_usage(&metadata),
        dependency_validation::validate_no_protocol_duplication(&metadata),
        dependency_validation::validate_no_direct_protocol_imports(&metadata),
        
        // Plugin architecture validations
        plugin_compliance::validate_adapter_trait_implementation(&metadata),
        plugin_compliance::validate_plugin_structure(&metadata),
        plugin_compliance::validate_common_module_usage(&metadata),
        plugin_compliance::validate_adapter_method_signatures(&metadata),
        
        // Typed ID and bijective architecture validations
        typed_id_usage::validate_typed_id_usage(&metadata),
        typed_id_usage::validate_typed_id_imports(&metadata),
        typed_id_usage::validate_bijective_id_usage(&metadata),
        
        // Code quality validations
        code_quality::validate_no_hardcoded_values(&metadata),
        code_quality::validate_configuration_usage(&metadata),
        code_quality::validate_no_mock_usage(&metadata),
        code_quality::validate_error_handling(&metadata),
        code_quality::validate_no_float_for_finance(&metadata),
        
        // Advanced architectural validations
        advanced_validations::validate_tlv_construction_patterns(&metadata),
        advanced_validations::validate_precision_handling(&metadata),
        advanced_validations::validate_service_boundaries(&metadata),
        advanced_validations::validate_async_patterns(&metadata),
        advanced_validations::validate_zerocopy_usage(&metadata),
    ];

    for result in tests {
        result.report();
        if !result.passed {
            all_passed = false;
        }
    }

    println!("{}", "=".repeat(50).cyan());

    if all_passed {
        println!(
            "{}",
            "All architecture validation tests passed!"
                .green()
                .bold()
        );
        Ok(())
    } else {
        anyhow::bail!("Some architecture validation tests failed");
    }
}
//! Precision validation tests
//!
//! Validates that the codebase preserves financial precision:
//! - No floating point (f32/f64) usage for financial calculations
//! - Proper use of native token precision (18 decimals WETH, 6 USDC)
//! - 8-decimal fixed-point for USD prices
//! - No precision loss during calculations

use std::fs;
use walkdir::WalkDir;
use regex::Regex;
use syn::{visit::Visit, Type, TypePath};
use crate::common::*;

#[test]
fn test_no_floating_point_in_financial_context() {
    let project_root = get_project_root().expect("Failed to find project root");
    let mut violations = Vec::new();

    // Patterns for floating point types
    let float_patterns = [
        r"\bf32\b",
        r"\bf64\b",
        r"\bdouble\b",
        r"\bfloat\b",
    ];

    let float_regexes: Vec<_> = float_patterns.iter()
        .map(|pattern| Regex::new(pattern).unwrap())
        .collect();

    for entry in WalkDir::new(&project_root)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
    {
        // Skip test files and certain whitelisted directories
        let path_str = entry.path().to_string_lossy();
        if matches_patterns(entry.path(), &[
            r".*/tests/.*",
            r".*/benches/.*",
            r".*/examples/.*",
            r".*/graphics/.*",
            r".*/ui/.*",
            r".*/display/.*",
            r".*/render/.*",
        ]) {
            continue;
        }

        if let Ok(content) = fs::read_to_string(entry.path()) {
            for (line_num, line) in content.lines().enumerate() {
                // Check if line contains financial context
                let has_financial_context = FINANCIAL_KEYWORDS.iter()
                    .any(|keyword| line.to_lowercase().contains(keyword));

                if has_financial_context {
                    for regex in &float_regexes {
                        if regex.is_match(line) {
                            violations.push(Violation::new(
                                entry.path().to_path_buf(),
                                ViolationType::PrecisionViolation,
                                format!("Floating point type used in financial context: {}", 
                                       line.trim()),
                                Severity::Critical,
                            ).with_line(line_num + 1));
                        }
                    }
                }
            }
        }
    }

    if !violations.is_empty() {
        panic!("Precision violations found:\n{}", format_violations(&violations));
    }
}

#[test]
fn test_proper_decimal_usage() {
    let project_root = get_project_root().expect("Failed to find project root");
    let mut has_decimal_usage = false;
    let mut violations = Vec::new();

    // Look for rust_decimal usage patterns
    let decimal_patterns = [
        r"use.*rust_decimal",
        r"Decimal::",
        r"from_str.*Decimal",
    ];

    let decimal_regexes: Vec<_> = decimal_patterns.iter()
        .map(|pattern| Regex::new(pattern).unwrap())
        .collect();

    for entry in WalkDir::new(&project_root)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
    {
        if let Ok(content) = fs::read_to_string(entry.path()) {
            // Check for proper decimal usage
            for regex in &decimal_regexes {
                if regex.is_match(&content) {
                    has_decimal_usage = true;
                    break;
                }
            }

            // Check for improper precision scaling
            let improper_scaling_patterns = [
                r"as f64.*\* 1e8", // Wrong: using f64 for fixed-point
                r"/ 1000000\.0",   // Wrong: float division
                r"\* 100000000\.0", // Wrong: float multiplication
            ];

            for pattern in &improper_scaling_patterns {
                if let Ok(regex) = Regex::new(pattern) {
                    for (line_num, line) in content.lines().enumerate() {
                        if regex.is_match(line) {
                            violations.push(Violation::new(
                                entry.path().to_path_buf(),
                                ViolationType::PrecisionViolation,
                                format!("Improper precision scaling using floating point: {}", 
                                       line.trim()),
                                Severity::Critical,
                            ).with_line(line_num + 1));
                        }
                    }
                }
            }
        }
    }

    // Warn if no decimal usage is found - this might indicate precision issues
    if !has_decimal_usage {
        violations.push(Violation::new(
            project_root.clone(),
            ViolationType::PrecisionViolation,
            "No rust_decimal usage found - ensure proper precision handling".to_string(),
            Severity::Warning,
        ));
    }

    if !violations.is_empty() {
        panic!("Decimal usage violations found:\n{}", format_violations(&violations));
    }
}

#[test]
fn test_native_token_precision_preservation() {
    let project_root = get_project_root().expect("Failed to find project root");
    let mut violations = Vec::new();

    // Patterns that indicate precision scaling/normalization (which is wrong)
    let normalization_patterns = [
        r"normalize_precision",
        r"scale_to_18_decimals",
        r"convert_to_standard_precision",
        r"PRECISION_MULTIPLIER",
    ];

    for entry in WalkDir::new(&project_root)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
    {
        if let Ok(content) = fs::read_to_string(entry.path()) {
            for pattern in &normalization_patterns {
                if let Ok(regex) = Regex::new(pattern) {
                    for (line_num, line) in content.lines().enumerate() {
                        if regex.is_match(line) {
                            violations.push(Violation::new(
                                entry.path().to_path_buf(),
                                ViolationType::PrecisionViolation,
                                format!("Native token precision should be preserved, not normalized: {}", 
                                       line.trim()),
                                Severity::Critical,
                            ).with_line(line_num + 1));
                        }
                    }
                }
            }
        }
    }

    if !violations.is_empty() {
        panic!("Native token precision violations found:\n{}", 
               format_violations(&violations));
    }
}

#[test]
fn test_fixed_point_arithmetic_usage() {
    let project_root = get_project_root().expect("Failed to find project root");
    let mut violations = Vec::new();

    // Look for proper fixed-point patterns
    let good_patterns = [
        r"100_000_000", // 8-decimal fixed-point multiplier
        r"1_000_000_000_000_000_000", // 18-decimal (wei)
        r"1_000_000", // 6-decimal (USDC)
        r"to_i64\(\)",
        r"from_i64",
    ];

    // Look for problematic patterns
    let bad_patterns = [
        r"\* 1e8", // Should use integer multiplier
        r"/ 1e8",  // Should use integer divisor
        r"pow\(10,", // Should use compile-time constants
    ];

    for entry in WalkDir::new(&project_root)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
    {
        if let Ok(content) = fs::read_to_string(entry.path()) {
            // Check for financial context
            let has_financial_context = FINANCIAL_KEYWORDS.iter()
                .any(|keyword| content.to_lowercase().contains(keyword));

            if has_financial_context {
                for pattern in &bad_patterns {
                    if let Ok(regex) = Regex::new(pattern) {
                        for (line_num, line) in content.lines().enumerate() {
                            if regex.is_match(line) {
                                violations.push(Violation::new(
                                    entry.path().to_path_buf(),
                                    ViolationType::PrecisionViolation,
                                    format!("Use compile-time integer constants for fixed-point arithmetic: {}", 
                                           line.trim()),
                                    Severity::Warning,
                                ).with_line(line_num + 1));
                            }
                        }
                    }
                }
            }
        }
    }

    if !violations.is_empty() {
        panic!("Fixed-point arithmetic violations found:\n{}", 
               format_violations(&violations));
    }
}

/// Visitor to check for floating-point types in financial contexts
struct FloatingPointVisitor {
    violations: Vec<Violation>,
    file_path: std::path::PathBuf,
}

impl FloatingPointVisitor {
    fn new(file_path: std::path::PathBuf) -> Self {
        Self {
            violations: Vec::new(),
            file_path,
        }
    }
}

impl<'ast> Visit<'ast> for FloatingPointVisitor {
    fn visit_type(&mut self, node: &'ast Type) {
        if let Type::Path(TypePath { path, .. }) = node {
            if let Some(segment) = path.segments.last() {
                let type_name = segment.ident.to_string();
                if matches!(type_name.as_str(), "f32" | "f64" | "float" | "double") {
                    self.violations.push(Violation::new(
                        self.file_path.clone(),
                        ViolationType::PrecisionViolation,
                        format!("Floating point type '{}' should not be used for financial calculations", type_name),
                        Severity::Critical,
                    ));
                }
            }
        }

        syn::visit::visit_type(self, node);
    }
}

#[test]
fn test_ast_floating_point_detection() {
    let project_root = get_project_root().expect("Failed to find project root");
    let mut all_violations = Vec::new();

    for entry in WalkDir::new(&project_root)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
    {
        // Skip test and whitelist files
        if matches_patterns(entry.path(), &[
            r".*/tests/.*",
            r".*/benches/.*",
            r".*/examples/.*",
        ]) {
            continue;
        }

        if let Ok(ast) = parse_rust_file(entry.path()) {
            if let Ok(content) = fs::read_to_string(entry.path()) {
                // Only check files with financial context
                let has_financial_context = FINANCIAL_KEYWORDS.iter()
                    .any(|keyword| content.to_lowercase().contains(keyword));

                if has_financial_context {
                    let mut visitor = FloatingPointVisitor::new(entry.path().to_path_buf());
                    visitor.visit_file(&ast);
                    all_violations.extend(visitor.violations);
                }
            }
        }
    }

    if !all_violations.is_empty() {
        panic!("AST floating point detection violations found:\n{}", 
               format_violations(&all_violations));
    }
}

fn format_violations(violations: &[Violation]) -> String {
    violations.iter()
        .map(|v| {
            format!("  {:?}: {} at {}{}",
                   v.severity,
                   v.message,
                   v.file_path.display(),
                   if let Some(line) = v.line {
                       format!(":{}", line)
                   } else {
                       String::new()
                   })
        })
        .collect::<Vec<_>>()
        .join("\n")
}
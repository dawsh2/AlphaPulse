//! Common utilities for architecture validation tests

use anyhow::{Context, Result};
use cargo_metadata::{Metadata, MetadataCommand, Package};
use colored::Colorize;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use syn::{File as SynFile, Item};
use walkdir::WalkDir;

/// Test result with detailed failure information
pub struct ValidationResult {
    pub test_name: String,
    pub passed: bool,
    pub violations: Vec<Violation>,
}

/// Represents an architectural violation
#[derive(Debug, Clone)]
pub struct Violation {
    pub file: PathBuf,
    pub line: Option<usize>,
    pub rule: String,
    pub message: String,
    pub suggestion: Option<String>,
}

impl Violation {
    pub fn new(
        file: PathBuf,
        line: Option<usize>,
        rule: String,
        message: String,
        suggestion: Option<String>,
    ) -> Self {
        Self {
            file,
            line,
            rule,
            message,
            suggestion,
        }
    }
}

impl ValidationResult {
    pub fn new(test_name: impl Into<String>) -> Self {
        Self {
            test_name: test_name.into(),
            passed: true,
            violations: Vec::new(),
        }
    }

    pub fn add_violation(&mut self, violation: Violation) {
        self.passed = false;
        self.violations.push(violation);
    }

    pub fn report(&self) {
        if self.passed {
            println!("{} {}", "✓".green(), self.test_name.green());
        } else {
            println!("{} {}", "✗".red(), self.test_name.red());
            for violation in &self.violations {
                println!("  {} {}", "→".yellow(), violation.message);
                println!("    File: {}", violation.file.display().to_string().cyan());
                if let Some(line) = violation.line {
                    println!("    Line: {}", line.to_string().cyan());
                }
                println!("    Rule: {}", violation.rule.yellow());
                if let Some(suggestion) = &violation.suggestion {
                    println!("    Suggestion: {}", suggestion.green());
                }
            }
        }
    }

    pub fn assert_passed(&self) {
        if !self.passed {
            self.report();
            panic!(
                "Architecture validation failed: {} ({} violations)",
                self.test_name,
                self.violations.len()
            );
        }
    }
}

/// Get workspace metadata using cargo_metadata
pub fn get_workspace_metadata() -> Result<Metadata> {
    MetadataCommand::new()
        .exec()
        .context("Failed to get workspace metadata")
}

/// Find all Rust source files in a directory
pub fn find_rust_files(dir: &Path) -> Vec<PathBuf> {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            let path = entry.path();
            path.extension().map_or(false, |ext| ext == "rs")
                && !path.to_string_lossy().contains("/target/")
                && !path.to_string_lossy().contains("/.git/")
        })
        .map(|entry| entry.path().to_path_buf())
        .collect()
}

/// Parse a Rust file and return the syntax tree
pub fn parse_rust_file(path: &Path) -> Result<SynFile> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))?;

    syn::parse_file(&content)
        .with_context(|| format!("Failed to parse Rust file: {}", path.display()))
}

/// Extract all use statements from a parsed Rust file
pub fn extract_imports(file: &SynFile) -> Vec<String> {
    let mut imports = Vec::new();

    for item in &file.items {
        if let Item::Use(use_item) = item {
            imports.push(format!("{}", quote::quote!(#use_item)));
        }
    }

    imports
}

/// Check if a file contains a specific pattern
pub fn file_contains_pattern(path: &Path, pattern: &Regex) -> Result<Vec<(usize, String)>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))?;

    let mut matches = Vec::new();
    for (line_num, line) in content.lines().enumerate() {
        if pattern.is_match(line) {
            matches.push((line_num + 1, line.to_string()));
        }
    }

    Ok(matches)
}

/// Get all service packages from workspace
pub fn get_service_packages(metadata: &Metadata) -> Vec<&Package> {
    metadata
        .packages
        .iter()
        .filter(|pkg| {
            let path = pkg.manifest_path.as_str();
            path.contains("/services_v2/") || path.contains("/relays/")
        })
        .collect()
}

/// Get all adapter packages from workspace
pub fn get_adapter_packages(metadata: &Metadata) -> Vec<&Package> {
    metadata
        .packages
        .iter()
        .filter(|pkg| {
            let path = pkg.manifest_path.as_str();
            path.contains("/services_v2/adapters/")
        })
        .collect()
}

/// Check if a package depends on another package
pub fn package_depends_on(package: &Package, dependency_name: &str) -> bool {
    package
        .dependencies
        .iter()
        .any(|dep| dep.name == dependency_name)
}

/// Find duplicate implementations of a pattern across files
pub fn find_duplicate_patterns(
    files: &[PathBuf],
    pattern: &Regex,
) -> HashMap<String, Vec<PathBuf>> {
    let mut pattern_locations: HashMap<String, Vec<PathBuf>> = HashMap::new();

    for file in files {
        if let Ok(matches) = file_contains_pattern(file, pattern) {
            for (_, matched_line) in matches {
                let normalized = normalize_code_pattern(&matched_line);
                pattern_locations
                    .entry(normalized)
                    .or_insert_with(Vec::new)
                    .push(file.clone());
            }
        }
    }

    pattern_locations
        .into_iter()
        .filter(|(_, locations)| locations.len() > 1)
        .collect()
}

/// Normalize a code pattern for comparison (remove whitespace variations)
fn normalize_code_pattern(code: &str) -> String {
    // Use regex to normalize patterns more systematically
    use regex::Regex;
    
    let normalized = code.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    
    // Fix function parentheses pattern: "fn name (" -> "fn name("
    let re = Regex::new(r"(\w+) \(").unwrap();
    let normalized = re.replace_all(&normalized, "$1(").to_string();
    
    // Fix empty parentheses: "( )" -> "()"
    let normalized = normalized.replace("( )", "()");
    
    // Fix empty braces: "{ }" -> "{}"  
    let normalized = normalized.replace("{ }", "{}");
    
    normalized
}

/// Validate that a service follows the expected structure
pub fn validate_service_structure(service_path: &Path) -> ValidationResult {
    let mut result = ValidationResult::new(format!(
        "Service structure validation: {}",
        service_path.display()
    ));

    let expected_files = ["Cargo.toml", "src/lib.rs"];

    for file in expected_files {
        let file_path = service_path.join(file);
        if !file_path.exists() {
            result.add_violation(Violation {
                file: service_path.to_path_buf(),
                line: None,
                rule: "service-structure".to_string(),
                message: format!("Missing expected file: {}", file),
                suggestion: Some(format!("Create {} in the service directory", file)),
            });
        }
    }

    result
}

/// Check if code uses typed IDs instead of raw primitives
pub fn check_typed_id_usage(_file: &SynFile, path: &Path) -> Vec<Violation> {
    let mut violations = Vec::new();
    let raw_id_pattern =
        Regex::new(r"\b(id|instrument_id|pool_id|token_id):\s*(u64|u32|i64|i32)\b").unwrap();

    let content = fs::read_to_string(path).unwrap_or_default();
    for (line_num, line) in content.lines().enumerate() {
        if raw_id_pattern.is_match(line)
            && !line.contains("//")
            && !line.contains("InstrumentId")
        {
            violations.push(Violation {
                file: path.to_path_buf(),
                line: Some(line_num + 1),
                rule: "typed-ids".to_string(),
                message: "Using raw numeric type for ID instead of typed ID".to_string(),
                suggestion: Some(
                    "Use InstrumentId, PoolId, or other typed IDs from torq_types"
                        .to_string(),
                ),
            });
        }
    }

    violations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_code_pattern() {
        let code1 = "pub fn   test_function  ( )  {  }";
        let code2 = "pub fn test_function() {}";

        assert_eq!(
            normalize_code_pattern(code1),
            normalize_code_pattern(code2)
        );
    }

    #[test]
    fn test_validation_result() {
        let mut result = ValidationResult::new("test");
        assert!(result.passed);

        result.add_violation(Violation {
            file: PathBuf::from("test.rs"),
            line: Some(10),
            rule: "test-rule".to_string(),
            message: "Test violation".to_string(),
            suggestion: None,
        });

        assert!(!result.passed);
        assert_eq!(result.violations.len(), 1);
    }
}
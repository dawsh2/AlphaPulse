//! Typed ID usage validation tests

use crate::common::{get_service_packages, ValidationResult, Violation};
use cargo_metadata::Metadata;

/// Validate that typed IDs are used consistently throughout the codebase
pub fn validate_typed_id_usage(metadata: &Metadata) -> ValidationResult {
    use crate::common::{check_typed_id_usage, find_rust_files, parse_rust_file};
    
    let mut result = ValidationResult::new("Typed IDs used instead of raw primitives");
    let service_packages = get_service_packages(metadata);
    
    for package in service_packages {
        let package_dir = package.manifest_path.parent().unwrap();
        let rust_files = find_rust_files(package_dir.as_std_path());
        
        for file in rust_files {
            // Skip test files - they might use raw types for simplicity
            if file.to_string_lossy().contains("/tests/") {
                continue;
            }
            
            if let Ok(parsed_file) = parse_rust_file(&file) {
                let violations = check_typed_id_usage(&parsed_file, &file);
                for violation in violations {
                    result.add_violation(violation);
                }
            }
        }
    }
    
    result
}

/// Validate that services import typed IDs from the correct location
pub fn validate_typed_id_imports(metadata: &Metadata) -> ValidationResult {
    use crate::common::{find_rust_files, file_contains_pattern};
    use regex::Regex;
    
    let mut result = ValidationResult::new("Typed IDs imported from codec library");
    let service_packages = get_service_packages(metadata);
    
    let correct_import_pattern = Regex::new(r"use\s+.*codec::.*InstrumentId").unwrap();
    let incorrect_import_patterns = vec![
        Regex::new(r"use\s+.*types::.*InstrumentId").unwrap(),
        Regex::new(r"use\s+.*protocol::.*InstrumentId").unwrap(),
    ];
    
    for package in service_packages {
        // Skip codec itself
        if package.name.contains("codec") {
            continue;
        }
        
        let package_dir = package.manifest_path.parent().unwrap();
        let rust_files = find_rust_files(package_dir.as_std_path());
        
        for file in rust_files {
            // Skip test files
            if file.to_string_lossy().contains("/tests/") {
                continue;
            }
            
            // Check for incorrect imports
            for pattern in &incorrect_import_patterns {
                if let Ok(matches) = file_contains_pattern(&file, pattern) {
                    for (line_num, matched_line) in matches {
                        result.add_violation(Violation {
                            file: file.clone(),
                            line: Some(line_num),
                            rule: "correct-typed-id-imports".to_string(),
                            message: format!(
                                "InstrumentId should be imported from codec library: '{}'", 
                                matched_line.trim()
                            ),
                            suggestion: Some(
                                "Import InstrumentId from codec instead: use torq_codec::InstrumentId;"
                                    .to_string(),
                            ),
                        });
                    }
                }
            }
            
            // Check if file uses InstrumentId but doesn't import from codec
            let uses_instrument_id = Regex::new(r"\bInstrumentId\b").unwrap();
            if let Ok(usage_matches) = file_contains_pattern(&file, &uses_instrument_id) {
                if !usage_matches.is_empty() {
                    // Check if it has correct import
                    if let Ok(correct_imports) = file_contains_pattern(&file, &correct_import_pattern) {
                        if correct_imports.is_empty() {
                            result.add_violation(Violation {
                                file: file.clone(),
                                line: None,
                                rule: "missing-codec-import".to_string(),
                                message: format!(
                                    "File uses InstrumentId but doesn't import from codec library"
                                ),
                                suggestion: Some(
                                    "Add: use torq_codec::InstrumentId; at the top of the file"
                                        .to_string(),
                                ),
                            });
                        }
                    }
                }
            }
        }
    }
    
    result
}

/// Validate that bijective IDs are used instead of instrument registries/lookups
pub fn validate_bijective_id_usage(metadata: &Metadata) -> ValidationResult {
    use crate::common::find_rust_files;
    use std::fs;
    use regex::Regex;

    let mut result = ValidationResult::new("No instrument registries - use bijective IDs");
    let service_packages = get_service_packages(metadata);
    
    // Patterns that violate bijective ID design
    let violation_patterns = [
        // HashMap lookups with InstrumentId as key - violates bijective design  
        (r"HashMap\s*<\s*InstrumentId\s*,", "HashMap<InstrumentId, _> registry pattern"),
        (r"BTreeMap\s*<\s*InstrumentId\s*,", "BTreeMap<InstrumentId, _> registry pattern"),
        (r"DashMap\s*<\s*InstrumentId\s*,", "DashMap<InstrumentId, _> registry pattern"),
        
        // Registry/lookup data structures
        (r"InstrumentRegistry|instrument_registry", "Instrument registry data structure"),
        (r"symbol_to_id\s*:", "Symbol-to-ID lookup table"),
        (r"id_to_symbol\s*:", "ID-to-symbol lookup table"),
        (r"instrument_map\s*:", "Instrument mapping structure"),
        
        // Lookup method patterns
        (r"\.get_instrument\(", "Instrument lookup method"),
        (r"\.lookup_instrument\(", "Instrument lookup method"),
        (r"\.resolve_instrument\(", "Instrument resolution method"),
        (r"\.find_by_symbol\(", "Symbol-based lookup method"),
    ];
    
    let patterns: Vec<Regex> = violation_patterns
        .iter()
        .map(|(pattern, _)| Regex::new(pattern).expect("Invalid regex pattern"))
        .collect();

    for package in &service_packages {
        let package_dir = package.manifest_path.parent().unwrap();
        let rust_files = find_rust_files(package_dir.as_std_path());
        
        for file_path in rust_files {
            if let Ok(content) = fs::read_to_string(&file_path) {
                for (pattern_regex, (_pattern, description)) in patterns.iter().zip(violation_patterns.iter()) {
                    if let Some(_matched_line) = content.lines().find(|line| {
                        // Skip comments - they may mention patterns as documentation
                        let trimmed = line.trim();
                        if trimmed.starts_with("//") || trimmed.starts_with("///") || trimmed.starts_with("*") {
                            return false;
                        }
                        pattern_regex.is_match(line)
                    }) {
                        let relative_path = file_path.strip_prefix(package_dir.as_std_path())
                            .unwrap_or(&file_path)
                            .display();
                        
                        result.add_violation(Violation {
                            file: file_path.clone(),
                            line: None,
                            rule: "bijective-ids".to_string(),
                            message: format!(
                                "{} in {} ({})", 
                                description, 
                                package.name, 
                                relative_path
                            ),
                            suggestion: Some(format!(
                                "Use bijective InstrumentId methods instead: \
                                 InstrumentId encodes venue/asset/symbol data directly. \
                                 Use .venue(), .asset_type(), .to_symbol() extraction methods \
                                 instead of external lookups."
                            )),
                        });
                    }
                }
            }
        }
    }
    
    result
}
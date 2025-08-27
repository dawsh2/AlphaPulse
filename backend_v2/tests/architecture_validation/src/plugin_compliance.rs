//! Plugin compliance validation tests

use crate::common::{get_adapter_packages, ValidationResult, Violation};
use cargo_metadata::Metadata;

/// Validate that all adapters implement the required Adapter trait
pub fn validate_adapter_trait_implementation(metadata: &Metadata) -> ValidationResult {
    use crate::common::{find_rust_files, file_contains_pattern};
    use regex::Regex;
    
    let mut result = ValidationResult::new("All adapters implement Adapter trait");
    let adapter_packages = get_adapter_packages(metadata);
    
    let impl_pattern = Regex::new(r"impl\s+\w*Adapter\s+for\s+\w+").unwrap();
    let trait_import_pattern = Regex::new(r"use\s+.*Adapter").unwrap();
    
    for package in adapter_packages {
        let package_dir = package.manifest_path.parent().unwrap();
        let rust_files = find_rust_files(package_dir.as_std_path());
        
        let mut has_adapter_impl = false;
        let mut has_adapter_import = false;
        
        for file in rust_files {
            // Skip test files
            if file.to_string_lossy().contains("/tests/") {
                continue;
            }
            
            if let Ok(impl_matches) = file_contains_pattern(&file, &impl_pattern) {
                if !impl_matches.is_empty() {
                    has_adapter_impl = true;
                }
            }
            
            if let Ok(import_matches) = file_contains_pattern(&file, &trait_import_pattern) {
                if !import_matches.is_empty() {
                    has_adapter_import = true;
                }
            }
        }
        
        if !has_adapter_impl && !has_adapter_import {
            result.add_violation(Violation {
                file: package.manifest_path.clone().into(),
                line: None,
                rule: "adapter-trait-implementation".to_string(),
                message: format!(
                    "Adapter package '{}' should implement or use an Adapter trait", 
                    package.name
                ),
                suggestion: Some(
                    "Implement a trait like ExchangeAdapter or DataCollector for consistent interface"
                        .to_string(),
                ),
            });
        }
    }
    
    result
}

/// Validate that adapter plugins follow the expected directory structure
pub fn validate_plugin_structure(metadata: &Metadata) -> ValidationResult {
    use crate::common::validate_service_structure;
    
    let mut result = ValidationResult::new("Adapter plugins follow expected structure");
    let adapter_packages = get_adapter_packages(metadata);
    
    for package in adapter_packages {
        let package_dir = package.manifest_path.parent().unwrap();
        let structure_result = validate_service_structure(package_dir.as_std_path());
        
        // Transfer violations from structure validation
        for violation in structure_result.violations {
            result.add_violation(violation);
        }
        
        // Additional adapter-specific structure checks
        let expected_adapter_files = ["src/lib.rs"];
        let recommended_files = ["src/adapter.rs", "src/config.rs"];
        
        for file in expected_adapter_files {
            let file_path = package_dir.join(file);
            if !file_path.exists() {
                result.add_violation(Violation {
                    file: package_dir.as_std_path().to_path_buf(),
                    line: None,
                    rule: "adapter-structure".to_string(),
                    message: format!(
                        "Adapter '{}' missing required file: {}", 
                        package.name, file
                    ),
                    suggestion: Some(format!("Create {} for adapter implementation", file)),
                });
            }
        }
        
        // Check for recommended files (warnings, not failures)
        let mut missing_recommended = Vec::new();
        for file in recommended_files {
            let file_path = package_dir.join(file);
            if !file_path.exists() {
                missing_recommended.push(file);
            }
        }
        
        if !missing_recommended.is_empty() {
            result.add_violation(Violation {
                file: package_dir.as_std_path().to_path_buf(),
                line: None,
                rule: "adapter-recommended-structure".to_string(),
                message: format!(
                    "Adapter '{}' missing recommended files: {}", 
                    package.name,
                    missing_recommended.join(", ")
                ),
                suggestion: Some(
                    "Consider creating separate modules for adapter logic and configuration"
                        .to_string(),
                ),
            });
        }
    }
    
    result
}

/// Validate that adapters use common modules appropriately
pub fn validate_common_module_usage(metadata: &Metadata) -> ValidationResult {
    use crate::common::{find_rust_files, file_contains_pattern};
    use regex::Regex;
    
    let mut result = ValidationResult::new("Adapters use common modules appropriately");
    let adapter_packages = get_adapter_packages(metadata);
    
    // Patterns for common module usage
    let common_module_patterns = vec![
        Regex::new(r"use\s+.*common::").unwrap(),
        Regex::new(r"use\s+.*libs::(codec|types|amm|dex)::").unwrap(),
        Regex::new(r"use\s+.*message_sink::").unwrap(),
    ];
    
    // Anti-patterns that indicate direct implementation instead of using common modules
    let duplicate_patterns = vec![
        Regex::new(r"fn\s+parse_tlv").unwrap(),
        Regex::new(r"fn\s+serialize").unwrap(),
        Regex::new(r"struct.*Header.*\{").unwrap(),
        Regex::new(r"const\s+MAGIC.*0x").unwrap(),
    ];
    
    for package in adapter_packages {
        let package_dir = package.manifest_path.parent().unwrap();
        let rust_files = find_rust_files(package_dir.as_std_path());
        
        let mut uses_common_modules = false;
        let mut has_duplicated_logic = false;
        
        for file in rust_files {
            // Skip test files
            if file.to_string_lossy().contains("/tests/") {
                continue;
            }
            
            // Check for common module usage
            for pattern in &common_module_patterns {
                if let Ok(matches) = file_contains_pattern(&file, pattern) {
                    if !matches.is_empty() {
                        uses_common_modules = true;
                        break;
                    }
                }
            }
            
            // Check for duplicate implementations
            for pattern in &duplicate_patterns {
                if let Ok(matches) = file_contains_pattern(&file, pattern) {
                    for (line_num, matched_line) in matches {
                        if !matched_line.trim_start().starts_with("//") {
                            has_duplicated_logic = true;
                            result.add_violation(Violation {
                                file: file.clone(),
                                line: Some(line_num),
                                rule: "duplicate-implementation".to_string(),
                                message: format!(
                                    "Adapter '{}' implements logic that exists in common modules: '{}'",
                                    package.name, matched_line.trim()
                                ),
                                suggestion: Some(
                                    "Use shared libraries (libs/codec, libs/types) instead of reimplementing common functionality"
                                        .to_string(),
                                ),
                            });
                        }
                    }
                }
            }
        }
        
        if !uses_common_modules && !has_duplicated_logic {
            // This might be okay for simple adapters, so just note it
            result.add_violation(Violation {
                file: package.manifest_path.clone().into(),
                line: None,
                rule: "no-common-modules".to_string(),
                message: format!(
                    "Adapter '{}' doesn't seem to use common modules. Consider if shared functionality could be used.",
                    package.name
                ),
                suggestion: Some(
                    "Review if this adapter could benefit from using libs/codec for message handling or libs/types for data structures"
                        .to_string(),
                ),
            });
        }
    }
    
    result
}

/// Check that adapter trait methods have consistent signatures
pub fn validate_adapter_method_signatures(metadata: &Metadata) -> ValidationResult {
    use crate::common::{find_rust_files, file_contains_pattern};
    use regex::Regex;
    
    let mut result = ValidationResult::new("Adapter trait methods have consistent signatures");
    let adapter_packages = get_adapter_packages(metadata);
    
    // Common adapter method patterns that should be consistent
    let method_patterns = vec![
        (Regex::new(r"async\s+fn\s+start\(").unwrap(), "start method should be async"),
        (Regex::new(r"async\s+fn\s+stop\(").unwrap(), "stop method should be async"),
        (Regex::new(r"fn\s+collect.*\(\s*&.*self").unwrap(), "collect methods should take &self"),
        (Regex::new(r"fn\s+process.*Result<").unwrap(), "process methods should return Result"),
    ];
    
    for package in adapter_packages {
        let package_dir = package.manifest_path.parent().unwrap();
        let rust_files = find_rust_files(package_dir.as_std_path());
        
        for file in rust_files {
            // Skip test files
            if file.to_string_lossy().contains("/tests/") {
                continue;
            }
            
            for (pattern, expected) in &method_patterns {
                if let Ok(matches) = file_contains_pattern(&file, pattern) {
                    for (line_num, matched_line) in matches {
                        // Check for common signature issues
                        if matched_line.contains("fn start") && !matched_line.contains("async") {
                            result.add_violation(Violation {
                                file: file.clone(),
                                line: Some(line_num),
                                rule: "inconsistent-signature".to_string(),
                                message: format!(
                                    "Method signature in '{}' should be async: '{}'",
                                    package.name, matched_line.trim()
                                ),
                                suggestion: Some(expected.to_string()),
                            });
                        }
                        
                        if matched_line.contains("fn collect") && matched_line.contains("&mut self") {
                            result.add_violation(Violation {
                                file: file.clone(),
                                line: Some(line_num),
                                rule: "inconsistent-signature".to_string(),
                                message: format!(
                                    "Collection methods should typically use &self for concurrent access: '{}'",
                                    matched_line.trim()
                                ),
                                suggestion: Some(
                                    "Consider if this method needs &mut self or can use &self with interior mutability"
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
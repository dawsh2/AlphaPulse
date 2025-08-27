//! Dependency validation tests

use crate::common::{get_service_packages, package_depends_on, ValidationResult, Violation};
use cargo_metadata::Metadata;

/// Validate that all services use the codec library
pub fn validate_codec_usage(metadata: &Metadata) -> ValidationResult {
    let mut result = ValidationResult::new("Services use codec library");

    let service_packages = get_service_packages(metadata);
    let codec_dependency_names = ["codec"];

    for package in service_packages {
        if package.name.contains("test") || package.name == "architecture_validation" {
            continue;
        }

        let uses_codec = codec_dependency_names
            .iter()
            .any(|name| package_depends_on(package, name));

        if !uses_codec {
            result.add_violation(Violation {
                file: package.manifest_path.clone().into(),
                line: None,
                rule: "codec-usage".to_string(),
                message: format!(
                    "Service '{}' should depend on codec library for protocol functionality",
                    package.name
                ),
                suggestion: Some(
                    "Add 'codec = { path = \"../libs/codec\" }' to dependencies".to_string()
                ),
            });
        }
    }

    result
}

/// Validate that there's no duplicated protocol parsing logic
pub fn validate_no_protocol_duplication(metadata: &Metadata) -> ValidationResult {
    use crate::common::{find_duplicate_patterns, find_rust_files};
    use regex::Regex;
    
    let mut result = ValidationResult::new("No duplicated protocol parsing logic");
    
    // Find all Rust files in service directories
    let service_packages = get_service_packages(metadata);
    let mut all_files = Vec::new();
    
    for package in service_packages {
        let package_dir = package.manifest_path.parent().unwrap();
        let rust_files = find_rust_files(package_dir.as_std_path());
        all_files.extend(rust_files);
    }
    
    // Look for TLV parsing patterns that should be in codec
    let tlv_patterns = vec![
        Regex::new(r"parse_tlv_header\s*\(").unwrap(),
        Regex::new(r"TLVType::\w+\s*=>\s*\{").unwrap(),
        Regex::new(r"MessageHeader\s*\{").unwrap(),
        Regex::new(r"let\s+\w+\s*=\s*&payload\[").unwrap(),
    ];
    
    for pattern in tlv_patterns {
        let duplicates = find_duplicate_patterns(&all_files, &pattern);
        
        for (matched_code, locations) in duplicates {
            if locations.len() > 1 {
                // Skip if one of the locations is in the codec itself
                let codec_locations: Vec<_> = locations
                    .iter()
                    .filter(|path| path.to_string_lossy().contains("/libs/codec/"))
                    .collect();
                
                let non_codec_locations: Vec<_> = locations
                    .iter()
                    .filter(|path| !path.to_string_lossy().contains("/libs/codec/"))
                    .collect();
                
                if !non_codec_locations.is_empty() && !codec_locations.is_empty() {
                    for location in non_codec_locations {
                        result.add_violation(Violation {
                            file: location.clone(),
                            line: None,
                            rule: "no-protocol-duplication".to_string(),
                            message: format!(
                                "Duplicated protocol parsing logic found: '{}'", 
                                matched_code.chars().take(60).collect::<String>()
                            ),
                            suggestion: Some(
                                "Use codec library functions instead of implementing protocol parsing directly"
                                    .to_string(),
                            ),
                        });
                    }
                }
            }
        }
    }
    
    result
}

/// Validate no direct protocol imports
pub fn validate_no_direct_protocol_imports(metadata: &Metadata) -> ValidationResult {
    use crate::common::{find_rust_files, file_contains_pattern};
    use regex::Regex;
    
    let mut result = ValidationResult::new("No direct protocol imports bypassing codec");
    
    let service_packages = get_service_packages(metadata);
    
    // Patterns for direct protocol imports that should go through codec
    let forbidden_imports = vec![
        Regex::new(r"use\s+.*protocol_v2::").unwrap(),
        Regex::new(r"use\s+.*types::protocol::").unwrap(),
        Regex::new(r"extern\s+crate\s+protocol_v2").unwrap(),
    ];
    
    for package in service_packages {
        if package.name.contains("codec") {
            continue; // Codec itself can import protocol
        }
        
        let package_dir = package.manifest_path.parent().unwrap();
        let rust_files = find_rust_files(package_dir.as_std_path());
        
        for file in rust_files {
            // Skip test files and examples
            if file.to_string_lossy().contains("/tests/") 
                || file.to_string_lossy().contains("/examples/") {
                continue;
            }
            
            for pattern in &forbidden_imports {
                if let Ok(matches) = file_contains_pattern(&file, pattern) {
                    for (line_num, matched_line) in matches {
                        result.add_violation(Violation {
                            file: file.clone(),
                            line: Some(line_num),
                            rule: "no-direct-protocol-imports".to_string(),
                            message: format!(
                                "Direct protocol import found in '{}': '{}'", 
                                package.name,
                                matched_line.trim()
                            ),
                            suggestion: Some(
                                "Import protocol types through codec library instead of directly"
                                    .to_string(),
                            ),
                        });
                    }
                }
            }
        }
    }
    
    result
}
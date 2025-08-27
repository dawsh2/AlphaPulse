//! Code quality validation tests

use crate::common::{get_service_packages, ValidationResult, Violation};
use cargo_metadata::Metadata;

/// Validate that services don't contain hardcoded values
pub fn validate_no_hardcoded_values(metadata: &Metadata) -> ValidationResult {
    use crate::common::{find_rust_files, file_contains_pattern};
    use regex::Regex;
    
    let mut result = ValidationResult::new("No hardcoded values in financial logic");
    let service_packages = get_service_packages(metadata);
    
    // Patterns for hardcoded financial values
    let violation_patterns = vec![
        // Hardcoded thresholds and percentages
        Regex::new(r">\s*0\.\d+\s*[;,)].*(?:spread|profit|threshold|percentage)").unwrap(),
        Regex::new(r"<\s*\d+\.\d+.*(?:spread|profit|threshold|percentage)").unwrap(),
        
        // Hardcoded token amounts or prices
        Regex::new(r"1000000000000000000.*(?:wei|eth|token)").unwrap(), // 1 ETH in wei
        Regex::new(r"1000000.*(?:usdc|usdt|dai)").unwrap(), // 1 USDC in 6 decimals
        
        // Hardcoded gas limits
        Regex::new(r"gas.*=\s*\d{4,}").unwrap(),
        
        // Hardcoded addresses (should use configuration) - only in variable assignments
        Regex::new(r"(let|const|static)\s+\w+.*=.*0x[a-fA-F0-9]{40}").unwrap(),
    ];
    
    for package in service_packages {
        // Skip test packages
        if package.name.contains("test") {
            continue;
        }
        
        let package_dir = package.manifest_path.parent().unwrap();
        let rust_files = find_rust_files(package_dir.as_std_path());
        
        for file in rust_files {
            // Skip test files, examples, and fixture files
            let file_str = file.to_string_lossy();
            if file_str.contains("/tests/") 
                || file_str.contains("/examples/") 
                || file_str.contains("/fixtures/")
                || file_str.contains("test_") {
                continue;
            }
            
            for pattern in &violation_patterns {
                if let Ok(matches) = file_contains_pattern(&file, pattern) {
                    for (line_num, matched_line) in matches {
                        // Skip commented lines and tests
                        let trimmed = matched_line.trim_start();
                        if trimmed.starts_with("//") || trimmed.starts_with("#[test]") 
                           || matched_line.to_lowercase().contains("test") {
                            continue;
                        }
                        
                        result.add_violation(Violation {
                            file: file.clone(),
                            line: Some(line_num),
                            rule: "no-hardcoded-values".to_string(),
                            message: format!(
                                "Hardcoded financial value detected: '{}'", 
                                matched_line.trim()
                            ),
                            suggestion: Some(
                                "Move hardcoded values to configuration files. Use dynamic values \
                                 that can be adjusted per environment and trading conditions."
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

/// Validate that services use proper configuration patterns
pub fn validate_configuration_usage(metadata: &Metadata) -> ValidationResult {
    use crate::common::{find_rust_files, file_contains_pattern};
    use regex::Regex;
    
    let mut result = ValidationResult::new("Services use proper configuration patterns");
    let service_packages = get_service_packages(metadata);
    
    // Patterns indicating good configuration practices
    let config_patterns = vec![
        Regex::new(r"#\[derive.*Config.*\]").unwrap(),
        Regex::new(r"struct.*Config\s*\{").unwrap(),
        Regex::new(r"serde::Deserialize.*Config").unwrap(),
        Regex::new(r"toml::from_str|config::Config").unwrap(),
    ];
    
    // Anti-patterns indicating hardcoded values
    let hardcoded_patterns = vec![
        Regex::new(r#"const\s+\w+.*URL.*=.*"http"#).unwrap(),
        Regex::new(r"const\s+\w+.*PORT.*=.*\d{4,}").unwrap(),
        Regex::new(r"const\s+\w+.*TIMEOUT.*=.*\d+").unwrap(),
    ];
    
    for package in service_packages {
        let package_dir = package.manifest_path.parent().unwrap();
        let rust_files = find_rust_files(package_dir.as_std_path());
        
        let mut has_config_pattern = false;
        let mut has_hardcoded_values = false;
        
        for file in rust_files {
            // Skip test files
            if file.to_string_lossy().contains("/tests/") {
                continue;
            }
            
            // Check for configuration patterns
            for pattern in &config_patterns {
                if let Ok(matches) = file_contains_pattern(&file, pattern) {
                    if !matches.is_empty() {
                        has_config_pattern = true;
                        break;
                    }
                }
            }
            
            // Check for hardcoded configuration values
            for pattern in &hardcoded_patterns {
                if let Ok(matches) = file_contains_pattern(&file, pattern) {
                    for (line_num, matched_line) in matches {
                        if !matched_line.trim_start().starts_with("//") {
                            has_hardcoded_values = true;
                            result.add_violation(Violation {
                                file: file.clone(),
                                line: Some(line_num),
                                rule: "hardcoded-config".to_string(),
                                message: format!(
                                    "Hardcoded configuration value in '{}': '{}'",
                                    package.name, matched_line.trim()
                                ),
                                suggestion: Some(
                                    "Move configuration values to config files (TOML/JSON) and use serde for deserialization"
                                        .to_string(),
                                ),
                            });
                        }
                    }
                }
            }
        }
        
        // Services should have some form of configuration
        if !has_config_pattern && !package.name.contains("test") {
            result.add_violation(Violation {
                file: package.manifest_path.clone().into(),
                line: None,
                rule: "missing-configuration".to_string(),
                message: format!(
                    "Service '{}' doesn't appear to use configuration patterns",
                    package.name
                ),
                suggestion: Some(
                    "Consider adding a Config struct with serde::Deserialize for configurable values"
                        .to_string(),
                ),
            });
        }
    }
    
    result
}

/// Validate that mock data or services are not used in production code
pub fn validate_no_mock_usage(metadata: &Metadata) -> ValidationResult {
    use crate::common::{find_rust_files, file_contains_pattern};
    use regex::Regex;
    
    let mut result = ValidationResult::new("No mock data or services in production code");
    let service_packages = get_service_packages(metadata);
    
    let mock_patterns = vec![
        Regex::new(r"\bmock\w*\s*::\s*\w+").unwrap(),
        Regex::new(r"MockData\w*").unwrap(),
        Regex::new(r"mock_\w+").unwrap(),
        Regex::new(r"\.mock\(\)").unwrap(),
        Regex::new(r"fake_\w+").unwrap(),
        Regex::new(r"dummy_\w+").unwrap(),
        Regex::new(r"stub_\w+").unwrap(),
    ];
    
    for package in service_packages {
        // Skip test packages
        if package.name.contains("test") {
            continue;
        }
        
        let package_dir = package.manifest_path.parent().unwrap();
        let rust_files = find_rust_files(package_dir.as_std_path());
        
        for file in rust_files {
            // Skip test files and mock files
            let file_str = file.to_string_lossy();
            if file_str.contains("/tests/") 
                || file_str.contains("/test_")
                || file_str.contains("mock")
                || file_str.contains("/examples/") {
                continue;
            }
            
            for pattern in &mock_patterns {
                if let Ok(matches) = file_contains_pattern(&file, pattern) {
                    for (line_num, matched_line) in matches {
                        // Skip commented lines
                        if matched_line.trim_start().starts_with("//") {
                            continue;
                        }
                        
                        result.add_violation(Violation {
                            file: file.clone(),
                            line: Some(line_num),
                            rule: "no-mock-usage".to_string(),
                            message: format!(
                                "Mock usage found in production code: '{}'", 
                                matched_line.trim()
                            ),
                            suggestion: Some(
                                "Replace mock/fake/dummy data with real connections and data"
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

/// Validate proper error handling patterns
pub fn validate_error_handling(metadata: &Metadata) -> ValidationResult {
    use crate::common::{find_rust_files, file_contains_pattern};
    use regex::Regex;
    
    let mut result = ValidationResult::new("Proper error handling patterns used");
    let service_packages = get_service_packages(metadata);
    
    // Anti-patterns for error handling
    let bad_patterns = vec![
        (Regex::new(r"\.unwrap\(\)").unwrap(), "unwrap() can panic - use proper error handling"),
        (Regex::new(r#"\.expect\("#).unwrap(), "expect() should have descriptive messages"),
        (Regex::new(r"panic!\(").unwrap(), "avoid panic! in production code"),
        (Regex::new(r"todo!\(").unwrap(), "todo! should not remain in production code"),
        (Regex::new(r"unreachable!\(\)").unwrap(), "unreachable! should be justified with comments"),
    ];
    
    // Good patterns for error handling  
    let good_patterns = vec![
        Regex::new(r"thiserror::Error").unwrap(),
        Regex::new(r"anyhow::Result").unwrap(),
        Regex::new(r"\.map_err\(").unwrap(),
        Regex::new(r"\.context\(").unwrap(),
    ];
    
    for package in service_packages {
        let package_dir = package.manifest_path.parent().unwrap();
        let rust_files = find_rust_files(package_dir.as_std_path());
        
        let mut has_good_error_handling = false;
        
        for file in rust_files {
            // Skip test files - they can use unwrap/expect
            if file.to_string_lossy().contains("/tests/") {
                continue;
            }
            
            // Check for good error handling patterns
            for pattern in &good_patterns {
                if let Ok(matches) = file_contains_pattern(&file, pattern) {
                    if !matches.is_empty() {
                        has_good_error_handling = true;
                        break;
                    }
                }
            }
            
            // Check for bad error handling patterns
            for (pattern, message) in &bad_patterns {
                if let Ok(matches) = file_contains_pattern(&file, pattern) {
                    for (line_num, matched_line) in matches {
                        let trimmed = matched_line.trim_start();
                        
                        // Skip comments and test-related code
                        if trimmed.starts_with("//") || matched_line.contains("#[test]") {
                            continue;
                        }
                        
                        // Allow some unwraps in specific contexts
                        if matched_line.contains(".unwrap()") {
                            if matched_line.contains("env!") || matched_line.contains("include_str!") 
                                || matched_line.contains("std::env::") {
                                continue; // These are generally safe
                            }
                        }
                        
                        result.add_violation(Violation {
                            file: file.clone(),
                            line: Some(line_num),
                            rule: "error-handling".to_string(),
                            message: format!(
                                "Poor error handling in '{}': '{}' - {}",
                                package.name, matched_line.trim(), message
                            ),
                            suggestion: Some(
                                "Use Result<T, E> types, thiserror for custom errors, and anyhow for error context"
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

/// Validate that floating point types are not used for financial calculations
pub fn validate_no_float_for_finance(metadata: &Metadata) -> ValidationResult {
    use crate::common::{find_rust_files, file_contains_pattern};
    use regex::Regex;
    
    let mut result = ValidationResult::new("No floating point for financial calculations");
    let service_packages = get_service_packages(metadata);
    
    let float_patterns = vec![
        Regex::new(r"\bf32\b").unwrap(),
        Regex::new(r"\bf64\b").unwrap(),
        Regex::new(r"\bfloat\b").unwrap(),
        Regex::new(r"\bdouble\b").unwrap(),
    ];
    
    // Financial keywords that indicate problematic float usage
    let financial_keywords = [
        "price", "amount", "value", "cost", "profit", "loss", "fee", "balance",
        "quantity", "volume", "trade", "order", "swap", "liquidity", "reserve",
        "usd", "eth", "btc", "token", "currency", "money", "wei"
    ];
    
    for package in service_packages {
        let package_dir = package.manifest_path.parent().unwrap();
        let rust_files = find_rust_files(package_dir.as_std_path());
        
        for file in rust_files {
            // Skip test files - they're more lenient
            if file.to_string_lossy().contains("/tests/") {
                continue;
            }
            
            for pattern in &float_patterns {
                if let Ok(matches) = file_contains_pattern(&file, pattern) {
                    for (line_num, matched_line) in matches {
                        // Skip commented lines
                        if matched_line.trim_start().starts_with("//") {
                            continue;
                        }
                        
                        // Check if the line contains financial context
                        let line_lower = matched_line.to_lowercase();
                        let has_financial_context = financial_keywords
                            .iter()
                            .any(|&keyword| line_lower.contains(keyword));
                        
                        if has_financial_context {
                            result.add_violation(Violation {
                                file: file.clone(),
                                line: Some(line_num),
                                rule: "no-float-for-finance".to_string(),
                                message: format!(
                                    "Float type used in financial context: '{}'", 
                                    matched_line.trim()
                                ),
                                suggestion: Some(
                                    "Use fixed-point arithmetic with i64/u64. For DEX: preserve native precision (18 decimals WETH, 6 USDC). For USD: use 8-decimal fixed-point (* 100_000_000)"
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
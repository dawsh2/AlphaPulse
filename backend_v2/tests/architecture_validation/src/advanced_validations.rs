//! Advanced architectural validation rules for AlphaPulse system

use crate::common::{get_service_packages, find_rust_files, ValidationResult, Violation};
use cargo_metadata::Metadata;
use std::fs;
use regex::Regex;

/// Validate TLV message construction patterns
pub fn validate_tlv_construction_patterns(metadata: &Metadata) -> ValidationResult {
    let mut result = ValidationResult::new("Proper TLV message construction patterns");
    let service_packages = get_service_packages(metadata);
    
    // Anti-patterns in TLV construction
    let violation_patterns = [
        (r"magic\s*=\s*0x[^D]", "TLV messages should use standard magic number 0xDEADBEEF"),
        (r"payload_size\s*=\s*\d+", "TLV payload size should be calculated, not hardcoded"),
        (r"Vec::new\(\).*TLV", "Use TLVMessageBuilder instead of manual Vec construction"),
        (r"\.extend_from_slice.*u32::to_le_bytes", "Use TLVMessageBuilder helper methods"),
    ];
    
    let patterns: Vec<Regex> = violation_patterns
        .iter()
        .map(|(pattern, _)| Regex::new(pattern).expect("Invalid regex"))
        .collect();

    for package in &service_packages {
        let package_dir = package.manifest_path.parent().unwrap();
        let rust_files = find_rust_files(package_dir.as_std_path());
        
        for file_path in rust_files {
            if let Ok(content) = fs::read_to_string(&file_path) {
                for (pattern_regex, (_pattern, description)) in patterns.iter().zip(violation_patterns.iter()) {
                    if let Some(matched_line) = content.lines().find(|line| {
                        let trimmed = line.trim();
                        if trimmed.starts_with("//") || trimmed.starts_with("///") {
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
                            rule: "tlv-construction".to_string(),
                            message: format!("{} in {}", description, relative_path),
                            suggestion: Some(format!(
                                "Use TLVMessageBuilder from codec library for consistent message construction"
                            )),
                        });
                    }
                }
            }
        }
    }
    
    result
}

/// Validate precision handling patterns
pub fn validate_precision_handling(metadata: &Metadata) -> ValidationResult {
    let mut result = ValidationResult::new("Proper precision handling for different asset types");
    let service_packages = get_service_packages(metadata);
    
    // Patterns that might indicate precision issues
    let violation_patterns = [
        (r"/ 1000000\b", "Division by 1M might lose precision for non-USDC tokens"),
        (r"/ 100000000\b", "Verify this is correct precision for the asset type"),
        (r"\* 1000000\b", "Multiplication should preserve native token precision"),
        (r"\.truncate\(\)", "Truncation causes precision loss"),
        (r"\.round\(\)", "Rounding can cause precision loss in financial calculations"),
        (r"as u64.*price|as u32.*price", "Casting prices might truncate precision"),
    ];
    
    let patterns: Vec<Regex> = violation_patterns
        .iter()
        .map(|(pattern, _)| Regex::new(pattern).expect("Invalid regex"))
        .collect();

    for package in &service_packages {
        let package_dir = package.manifest_path.parent().unwrap();
        let rust_files = find_rust_files(package_dir.as_std_path());
        
        for file_path in rust_files {
            if let Ok(content) = fs::read_to_string(&file_path) {
                for (pattern_regex, (_pattern, description)) in patterns.iter().zip(violation_patterns.iter()) {
                    if let Some(matched_line) = content.lines().find(|line| {
                        let trimmed = line.trim();
                        if trimmed.starts_with("//") || trimmed.starts_with("///") {
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
                            rule: "precision-handling".to_string(),
                            message: format!("{} in {}: {}", description, relative_path, matched_line.trim()),
                            suggestion: Some(format!(
                                "Use appropriate precision per asset: 18 decimals for WETH, 6 for USDC, \
                                 8-decimal fixed-point for USD prices (* 100_000_000)"
                            )),
                        });
                    }
                }
            }
        }
    }
    
    result
}

/// Validate service boundaries and dependencies
pub fn validate_service_boundaries(metadata: &Metadata) -> ValidationResult {
    let mut result = ValidationResult::new("Respect service boundaries and dependencies");
    let service_packages = get_service_packages(metadata);
    
    // Define forbidden cross-service dependencies
    let forbidden_patterns = [
        (r"use.*strategies.*adapters", "Strategies should not directly use adapter internals"),
        (r"use.*adapters.*strategies", "Adapters should not depend on strategies"),
        (r"use.*dashboard.*strategies", "Dashboard should not depend on strategies directly"),
        (r"use.*execution.*market_data", "Execution should not directly use market data internals"),
        (r"\.\..*\.\..*use", "Avoid deep relative imports - use explicit paths"),
    ];
    
    let patterns: Vec<Regex> = forbidden_patterns
        .iter()
        .map(|(pattern, _)| Regex::new(pattern).expect("Invalid regex"))
        .collect();

    for package in &service_packages {
        let package_dir = package.manifest_path.parent().unwrap();
        let rust_files = find_rust_files(package_dir.as_std_path());
        
        for file_path in rust_files {
            if let Ok(content) = fs::read_to_string(&file_path) {
                for (pattern_regex, (_pattern, description)) in patterns.iter().zip(forbidden_patterns.iter()) {
                    if let Some(matched_line) = content.lines().find(|line| {
                        let trimmed = line.trim();
                        if trimmed.starts_with("//") || trimmed.starts_with("///") {
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
                            rule: "service-boundaries".to_string(),
                            message: format!("{} in {}: {}", description, relative_path, matched_line.trim()),
                            suggestion: Some(format!(
                                "Use shared libraries (libs/) for common functionality. \
                                 Services should communicate via TLV messages or well-defined APIs."
                            )),
                        });
                    }
                }
            }
        }
    }
    
    result
}

/// Validate async/await patterns for high-frequency trading
pub fn validate_async_patterns(metadata: &Metadata) -> ValidationResult {
    let mut result = ValidationResult::new("Efficient async patterns for HFT performance");
    let service_packages = get_service_packages(metadata);
    
    // Patterns that may indicate performance issues
    let violation_patterns = [
        (r"\.await.*\.await.*\.await", "Sequential awaits - consider join!/select! for parallelism"),
        (r"tokio::time::sleep\(Duration::from_millis\([1-9]\d{2,}\)", "Long sleeps might indicate polling - use events"),
        (r"Arc<Mutex<", "Arc<Mutex> has contention - consider Arc<RwLock> or message passing"),
        (r"spawn\(\s*async\s*move\s*\{[\s\S]*?thread::sleep", "Don't use thread::sleep in async tasks"),
        (r"blocking_task|spawn_blocking.*simple", "Avoid spawn_blocking for simple operations"),
    ];
    
    let patterns: Vec<Regex> = violation_patterns
        .iter()
        .map(|(pattern, _)| Regex::new(pattern).expect("Invalid regex"))
        .collect();

    for package in &service_packages {
        // Only check performance-critical services
        if !package.name.contains("adapter") && !package.name.contains("relay") 
           && !package.name.contains("strategy") && !package.name.contains("execution") {
            continue;
        }
        
        let package_dir = package.manifest_path.parent().unwrap();
        let rust_files = find_rust_files(package_dir.as_std_path());
        
        for file_path in rust_files {
            if let Ok(content) = fs::read_to_string(&file_path) {
                for (pattern_regex, (_pattern, description)) in patterns.iter().zip(violation_patterns.iter()) {
                    if let Some(matched_line) = content.lines().find(|line| {
                        let trimmed = line.trim();
                        if trimmed.starts_with("//") || trimmed.starts_with("///") {
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
                            rule: "async-patterns".to_string(),
                            message: format!("{} in {}: {}", description, relative_path, matched_line.trim()),
                            suggestion: Some(format!(
                                "Optimize async patterns for low-latency trading: use join!/select!, \
                                 prefer RwLock over Mutex, avoid blocking operations in async context."
                            )),
                        });
                    }
                }
            }
        }
    }
    
    result
}

/// Validate that zerocopy traits are used appropriately
pub fn validate_zerocopy_usage(metadata: &Metadata) -> ValidationResult {
    let mut result = ValidationResult::new("Proper zerocopy trait usage for performance");
    let service_packages = get_service_packages(metadata);
    
    // Patterns indicating missing zerocopy opportunities
    let violation_patterns = [
        (r"serde::Serialize.*TLV", "TLV structs should use zerocopy, not serde"),
        (r"bincode::serialize.*TLV", "TLV structs should use AsBytes, not bincode"),
        (r"struct.*TLV.*\{", "Check that TLV structs derive AsBytes"),
        (r"Vec<u8>.*message.*extend", "Consider using zerocopy for message construction"),
    ];
    
    let patterns: Vec<Regex> = violation_patterns
        .iter()
        .map(|(pattern, _)| Regex::new(pattern).expect("Invalid regex"))
        .collect();

    for package in &service_packages {
        let package_dir = package.manifest_path.parent().unwrap();
        let rust_files = find_rust_files(package_dir.as_std_path());
        
        for file_path in rust_files {
            if let Ok(content) = fs::read_to_string(&file_path) {
                for (pattern_regex, (_pattern, description)) in patterns.iter().zip(violation_patterns.iter()) {
                    if let Some(matched_line) = content.lines().find(|line| {
                        let trimmed = line.trim();
                        if trimmed.starts_with("//") || trimmed.starts_with("///") {
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
                            rule: "zerocopy-usage".to_string(),
                            message: format!("{} in {}: {}", description, relative_path, matched_line.trim()),
                            suggestion: Some(format!(
                                "Use zerocopy traits (AsBytes, FromBytes) for >1M msg/s performance. \
                                 Derive them on TLV structs and use direct memory operations."
                            )),
                        });
                    }
                }
            }
        }
    }
    
    result
}
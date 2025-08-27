//! Protocol V2 compliance validation tests
//!
//! Validates that the codebase adheres to Protocol V2 specifications:
//! - TLV message format with 32-byte MessageHeader + variable TLV payload
//! - Correct magic numbers (0xDEADBEEF)
//! - Domain separation (Market Data 1-19, Signals 20-39, Execution 40-79)
//! - Nanosecond timestamps (not milliseconds)
//! - Sequence integrity
//! - TLV type registry consistency

use std::fs;
use std::path::Path;
use walkdir::WalkDir;
use regex::Regex;
use anyhow::Result;
use crate::common::*;

#[test]
fn test_tlv_message_header_format() {
    let project_root = get_project_root().expect("Failed to find project root");
    let mut violations = Vec::new();

    // Look for MessageHeader definitions
    for entry in WalkDir::new(&project_root)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
    {
        if let Ok(content) = fs::read_to_string(entry.path()) {
            // Check for MessageHeader struct definitions
            if content.contains("struct MessageHeader") {
                // Ensure it has the correct magic number
                if !content.contains("0xDEADBEEF") {
                    violations.push(Violation::new(
                        entry.into_path(),
                        ViolationType::ProtocolV2Compliance,
                        "MessageHeader must use magic number 0xDEADBEEF".to_string(),
                        Severity::Critical,
                    ));
                }

                // Check for 32-byte size requirement
                if content.contains("SIZE") && !content.contains("32") {
                    violations.push(Violation::new(
                        entry.into_path(),
                        ViolationType::ProtocolV2Compliance,
                        "MessageHeader must be exactly 32 bytes".to_string(),
                        Severity::Critical,
                    ));
                }
            }
        }
    }

    if !violations.is_empty() {
        panic!("Protocol V2 MessageHeader violations found:\n{}", 
               format_violations(&violations));
    }
}

#[test]
fn test_tlv_type_domain_separation() {
    let project_root = get_project_root().expect("Failed to find project root");
    let mut violations = Vec::new();

    // Pattern to match TLV type assignments
    let tlv_type_pattern = Regex::new(r"(\w+)\s*=\s*(\d+)").unwrap();

    for entry in WalkDir::new(&project_root)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
    {
        if let Ok(content) = fs::read_to_string(entry.path()) {
            // Look for TLV type definitions
            if content.contains("enum TLVType") || content.contains("TLV_TYPE_") {
                for line in content.lines().enumerate() {
                    let (line_num, line_content) = line;
                    
                    if let Some(captures) = tlv_type_pattern.captures(line_content) {
                        if let Ok(type_num) = captures[2].parse::<u16>() {
                            let type_name = &captures[1];
                            
                            // Check domain ranges
                            if type_name.contains("Market") || type_name.contains("Trade") || 
                               type_name.contains("Quote") || type_name.contains("OrderBook") {
                                if !MARKET_DATA_TLV_RANGE.contains(&type_num) {
                                    violations.push(Violation::new(
                                        entry.path().to_path_buf(),
                                        ViolationType::ProtocolV2Compliance,
                                        format!("Market data TLV type {} = {} must be in range 1-19", 
                                               type_name, type_num),
                                        Severity::Critical,
                                    ).with_line(line_num + 1));
                                }
                            } else if type_name.contains("Signal") || type_name.contains("Identity") {
                                if !SIGNAL_TLV_RANGE.contains(&type_num) {
                                    violations.push(Violation::new(
                                        entry.path().to_path_buf(),
                                        ViolationType::ProtocolV2Compliance,
                                        format!("Signal TLV type {} = {} must be in range 20-39", 
                                               type_name, type_num),
                                        Severity::Critical,
                                    ).with_line(line_num + 1));
                                }
                            } else if type_name.contains("Execution") || type_name.contains("Order") {
                                if !EXECUTION_TLV_RANGE.contains(&type_num) {
                                    violations.push(Violation::new(
                                        entry.path().to_path_buf(),
                                        ViolationType::ProtocolV2Compliance,
                                        format!("Execution TLV type {} = {} must be in range 40-79", 
                                               type_name, type_num),
                                        Severity::Critical,
                                    ).with_line(line_num + 1));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if !violations.is_empty() {
        panic!("TLV domain separation violations found:\n{}", 
               format_violations(&violations));
    }
}

#[test]
fn test_nanosecond_timestamp_usage() {
    let project_root = get_project_root().expect("Failed to find project root");
    let mut violations = Vec::new();

    // Patterns that indicate millisecond timestamp truncation
    let ms_patterns = [
        r"timestamp_ms",
        r"/ 1_000_000", // ns to ms conversion
        r"timestamp\s*/\s*1000000",
        r"\.as_millis\(\)",
        r"duration_since.*as_millis",
    ];

    let ms_regexes: Vec<_> = ms_patterns.iter()
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
            for (line_num, line) in content.lines().enumerate() {
                // Skip test files - they may have different requirements
                if entry.path().to_string_lossy().contains("test") {
                    continue;
                }

                for regex in &ms_regexes {
                    if regex.is_match(line) {
                        violations.push(Violation::new(
                            entry.path().to_path_buf(),
                            ViolationType::ProtocolV2Compliance,
                            "Timestamps must preserve nanosecond precision, not truncate to milliseconds".to_string(),
                            Severity::Critical,
                        ).with_line(line_num + 1));
                    }
                }
            }
        }
    }

    if !violations.is_empty() {
        panic!("Timestamp precision violations found:\n{}", 
               format_violations(&violations));
    }
}

#[test]
fn test_tlv_type_registry_uniqueness() {
    let project_root = get_project_root().expect("Failed to find project root");
    let mut type_assignments = std::collections::HashMap::new();
    let mut violations = Vec::new();

    let tlv_type_pattern = Regex::new(r"(\w+)\s*=\s*(\d+)").unwrap();

    for entry in WalkDir::new(&project_root)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
    {
        if let Ok(content) = fs::read_to_string(entry.path()) {
            if content.contains("enum TLVType") || content.contains("TLV_TYPE_") {
                for (line_num, line) in content.lines().enumerate() {
                    if let Some(captures) = tlv_type_pattern.captures(line) {
                        if let Ok(type_num) = captures[2].parse::<u16>() {
                            let type_name = captures[1].to_string();
                            
                            if let Some((existing_name, existing_path, existing_line)) = 
                                type_assignments.get(&type_num) {
                                violations.push(Violation::new(
                                    entry.path().to_path_buf(),
                                    ViolationType::ProtocolV2Compliance,
                                    format!("TLV type number {} reused: {} conflicts with {} at {}:{}",
                                           type_num, type_name, existing_name, 
                                           existing_path.display(), existing_line),
                                    Severity::Critical,
                                ).with_line(line_num + 1));
                            } else {
                                type_assignments.insert(type_num, (
                                    type_name,
                                    entry.path().to_path_buf(),
                                    line_num + 1
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    if !violations.is_empty() {
        panic!("TLV type registry uniqueness violations found:\n{}", 
               format_violations(&violations));
    }
}

#[test]
fn test_sequence_integrity() {
    let project_root = get_project_root().expect("Failed to find project root");
    let mut violations = Vec::new();

    let sequence_patterns = [
        r"sequence.*\+\+", // Non-atomic increment
        r"sequence\s*=\s*\d+", // Hard-coded sequence reset
        r"sequence.*reset", // Explicit reset without proper handling
    ];

    let sequence_regexes: Vec<_> = sequence_patterns.iter()
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
            for (line_num, line) in content.lines().enumerate() {
                for regex in &sequence_regexes {
                    if regex.is_match(line) {
                        violations.push(Violation::new(
                            entry.path().to_path_buf(),
                            ViolationType::ProtocolV2Compliance,
                            "Sequence numbers must be monotonic and properly managed".to_string(),
                            Severity::Warning,
                        ).with_line(line_num + 1));
                    }
                }
            }
        }
    }

    if !violations.is_empty() {
        panic!("Sequence integrity violations found:\n{}", 
               format_violations(&violations));
    }
}

#[test]
fn test_expected_payload_size_consistency() {
    let project_root = get_project_root().expect("Failed to find project root");
    let mut violations = Vec::new();

    // Check that TLV structs with zerocopy have corresponding expected_payload_size
    for entry in WalkDir::new(&project_root)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
    {
        if let Ok(content) = fs::read_to_string(entry.path()) {
            // Look for TLV struct definitions with zerocopy
            let tlv_struct_pattern = Regex::new(
                r"#\[derive\(.*zerocopy.*\)\]\s*struct\s+(\w+TLV)\s*\{"
            ).unwrap();

            for captures in tlv_struct_pattern.captures_iter(&content) {
                let struct_name = &captures[1];
                
                // Check if there's a corresponding expected_payload_size function
                if !content.contains(&format!("expected_payload_size() -> usize")) ||
                   !content.contains(&format!("size_of::<{}>", struct_name)) {
                    violations.push(Violation::new(
                        entry.path().to_path_buf(),
                        ViolationType::ProtocolV2Compliance,
                        format!("TLV struct {} must have expected_payload_size() implementation", 
                               struct_name),
                        Severity::Critical,
                    ));
                }
            }
        }
    }

    if !violations.is_empty() {
        panic!("Expected payload size consistency violations found:\n{}", 
               format_violations(&violations));
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
//! Mock detection validation tests
//!
//! Validates that the codebase adheres to the "NO MOCKS EVER" principle:
//! - No mock data, services, or mocked testing
//! - No simulation modes that fake exchange responses  
//! - No stubbed WebSocket connections or API responses
//! - All testing uses real data and live connections

use std::fs;
use walkdir::WalkDir;
use regex::Regex;
use crate::common::*;

#[test]
fn test_no_mock_services() {
    let project_root = get_project_root().expect("Failed to find project root");
    let mut violations = Vec::new();

    // Patterns that indicate mock services
    let mock_service_patterns = [
        r"MockService",
        r"FakeService", 
        r"DummyService",
        r"StubService",
        r"TestService", // Often indicates mocking
        r"SimulatedService",
        r"MockClient",
        r"FakeClient",
        r"DummyClient",
    ];

    let mock_regexes: Vec<_> = mock_service_patterns.iter()
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
                for regex in &mock_regexes {
                    if regex.is_match(line) {
                        violations.push(Violation::new(
                            entry.path().to_path_buf(),
                            ViolationType::MockUsage,
                            format!("Mock service usage detected: {}", line.trim()),
                            Severity::Critical,
                        ).with_line(line_num + 1));
                    }
                }
            }
        }
    }

    if !violations.is_empty() {
        panic!("Mock service violations found:\n{}", format_violations(&violations));
    }
}

#[test]
fn test_no_mock_data() {
    let project_root = get_project_root().expect("Failed to find project root");
    let mut violations = Vec::new();

    // Patterns that indicate mock data
    let mock_data_patterns = [
        r"mock_data",
        r"fake_data",
        r"dummy_data",
        r"test_data.*=.*vec!\[", // Hardcoded test data vectors
        r"fake_price",
        r"dummy_price",
        r"mock_trade",
        r"fake_trade",
        r"simulate_trade",
        r"mock_response",
        r"fake_response",
        r"dummy_response",
    ];

    let mock_regexes: Vec<_> = mock_data_patterns.iter()
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
                for regex in &mock_regexes {
                    if regex.is_match(line) {
                        violations.push(Violation::new(
                            entry.path().to_path_buf(),
                            ViolationType::MockUsage,
                            format!("Mock data usage detected: {}", line.trim()),
                            Severity::Critical,
                        ).with_line(line_num + 1));
                    }
                }
            }
        }
    }

    if !violations.is_empty() {
        panic!("Mock data violations found:\n{}", format_violations(&violations));
    }
}

#[test]
fn test_no_simulation_modes() {
    let project_root = get_project_root().expect("Failed to find project root");
    let mut violations = Vec::new();

    // Patterns that indicate simulation modes
    let simulation_patterns = [
        r"simulation_mode",
        r"mock_mode",
        r"test_mode.*=.*true",
        r"simulate_exchange",
        r"fake_exchange",
        r"is_simulation",
        r"enable_simulation",
        r"simulation_enabled",
        r"SimulationConfig",
        r"TestingMode",
    ];

    let simulation_regexes: Vec<_> = simulation_patterns.iter()
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
                for regex in &simulation_regexes {
                    if regex.is_match(line) {
                        violations.push(Violation::new(
                            entry.path().to_path_buf(),
                            ViolationType::MockUsage,
                            format!("Simulation mode detected: {}", line.trim()),
                            Severity::Critical,
                        ).with_line(line_num + 1));
                    }
                }
            }
        }
    }

    if !violations.is_empty() {
        panic!("Simulation mode violations found:\n{}", format_violations(&violations));
    }
}

#[test]
fn test_no_stubbed_connections() {
    let project_root = get_project_root().expect("Failed to find project root");
    let mut violations = Vec::new();

    // Patterns that indicate stubbed connections
    let stub_patterns = [
        r"stub_websocket",
        r"fake_websocket",
        r"mock_websocket",
        r"dummy_connection",
        r"fake_connection",
        r"stub_connection",
        r"MockWebSocket",
        r"FakeWebSocket",
        r"StubConnection",
        r"TestConnection.*fake",
        r"simulate_connection",
    ];

    let stub_regexes: Vec<_> = stub_patterns.iter()
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
                for regex in &stub_regexes {
                    if regex.is_match(line) {
                        violations.push(Violation::new(
                            entry.path().to_path_buf(),
                            ViolationType::MockUsage,
                            format!("Stubbed connection detected: {}", line.trim()),
                            Severity::Critical,
                        ).with_line(line_num + 1));
                    }
                }
            }
        }
    }

    if !violations.is_empty() {
        panic!("Stubbed connection violations found:\n{}", format_violations(&violations));
    }
}

#[test]
fn test_real_exchange_usage() {
    let project_root = get_project_root().expect("Failed to find project root");
    let mut has_real_exchange_usage = false;
    let mut violations = Vec::new();

    // Patterns that indicate real exchange usage (good)
    let real_exchange_patterns = [
        r"wss://",                    // Real WebSocket URLs
        r"https://api\.",            // Real API endpoints
        r"stream\.binance\.",        // Binance streaming
        r"ws-feed\.exchange\.",      // Exchange WebSocket feeds
        r"api\.kraken\.com",         // Real Kraken API
        r"api\.coinbase\.com",       // Real Coinbase API
        r"polygon\.io",              // Real Polygon.io
    ];

    let real_regexes: Vec<_> = real_exchange_patterns.iter()
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
            for regex in &real_regexes {
                if regex.is_match(&content) {
                    has_real_exchange_usage = true;
                    break;
                }
            }
        }
    }

    // Also check config files for real endpoints
    for entry in WalkDir::new(&project_root)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            if let Some(ext) = e.path().extension() {
                matches!(ext.to_str(), Some("toml") | Some("yaml") | Some("json"))
            } else {
                false
            }
        })
    {
        if let Ok(content) = fs::read_to_string(entry.path()) {
            for regex in &real_regexes {
                if regex.is_match(&content) {
                    has_real_exchange_usage = true;
                    break;
                }
            }
        }
    }

    if !has_real_exchange_usage {
        violations.push(Violation::new(
            project_root.clone(),
            ViolationType::MockUsage,
            "No real exchange usage found - ensure system connects to live exchanges".to_string(),
            Severity::Warning,
        ));
    }

    if !violations.is_empty() {
        panic!("Real exchange usage violations found:\n{}", format_violations(&violations));
    }
}

#[test]
fn test_no_mockito_or_wiremock() {
    let project_root = get_project_root().expect("Failed to find project root");
    let mut violations = Vec::new();

    // Check Cargo.toml files for mocking libraries
    for entry in WalkDir::new(&project_root)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().file_name().map_or(false, |name| name == "Cargo.toml"))
    {
        if let Ok(content) = fs::read_to_string(entry.path()) {
            let mock_dependencies = [
                "mockito",
                "wiremock",
                "mockall",
                "mock_derive",
                "proptest", // Sometimes used for property testing with mocks
            ];

            for dep in &mock_dependencies {
                if content.contains(dep) {
                    violations.push(Violation::new(
                        entry.path().to_path_buf(),
                        ViolationType::MockUsage,
                        format!("Mock dependency '{}' found in Cargo.toml", dep),
                        Severity::Critical,
                    ));
                }
            }
        }
    }

    if !violations.is_empty() {
        panic!("Mock library dependencies found:\n{}", format_violations(&violations));
    }
}

#[test]
fn test_live_data_requirements() {
    let project_root = get_project_root().expect("Failed to find project root");
    let mut violations = Vec::new();

    // Look for patterns that suggest live data usage requirements
    let live_data_indicators = [
        r"live_data",
        r"real_time",
        r"stream.*live",
        r"production.*data",
        r"exchange.*feed",
    ];

    let mut found_live_indicators = false;

    for entry in WalkDir::new(&project_root)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
    {
        if let Ok(content) = fs::read_to_string(entry.path()) {
            for pattern in &live_data_indicators {
                if let Ok(regex) = Regex::new(pattern) {
                    if regex.is_match(&content) {
                        found_live_indicators = true;
                        break;
                    }
                }
            }

            // Look for test patterns that should be using live data
            if content.contains("#[test]") && content.contains("exchange") {
                let lines: Vec<&str> = content.lines().collect();
                for (i, line) in lines.iter().enumerate() {
                    if line.contains("#[test]") && i + 10 < lines.len() {
                        // Check next 10 lines for mock patterns in test
                        let test_body = lines[i..i+10].join(" ");
                        if contains_mock_keywords(&test_body) {
                            violations.push(Violation::new(
                                entry.path().to_path_buf(),
                                ViolationType::MockUsage,
                                "Exchange test using mock data instead of live data".to_string(),
                                Severity::Critical,
                            ).with_line(i + 1));
                        }
                    }
                }
            }
        }
    }

    if !violations.is_empty() {
        panic!("Live data requirement violations found:\n{}", format_violations(&violations));
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
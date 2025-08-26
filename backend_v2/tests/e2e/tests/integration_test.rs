//! Integration test for E2E framework

use alphapulse_e2e_tests::{
    framework::{TestConfig, TestFramework, ValidationLevel},
    scenarios::KrakenToDashboardTest,
};
use tokio_test;

#[tokio::test]
async fn test_framework_creation() {
    let config = TestConfig {
        timeout_secs: 60,
        cleanup: true,
        verbose: false,
        validation_level: ValidationLevel::Basic,
        data_dir: std::path::PathBuf::from("/tmp/test_framework"),
    };

    let framework = TestFramework::new(config).expect("Failed to create test framework");

    // Test that framework initializes correctly
    assert_eq!(framework.config().timeout_secs, 60);
    assert!(framework
        .relay_paths()
        .market_data
        .contains("market_data.sock"));
    assert!(framework.relay_paths().signals.contains("signals.sock"));
    assert!(framework.relay_paths().execution.contains("execution.sock"));
}

#[tokio::test]
async fn test_scenario_creation() {
    let scenario = KrakenToDashboardTest {
        use_live_data: false,
        expected_messages: 10,
        max_latency_ms: 100,
    };

    assert_eq!(scenario.name(), "kraken_to_dashboard");
    assert!(!scenario.description().is_empty());
    assert!(scenario.timeout().as_secs() > 0);
}

#[tokio::test]
async fn test_system_health_validation() {
    let config = TestConfig::default();
    let framework = TestFramework::new(config).expect("Failed to create test framework");

    // Should be able to validate system health even with no services
    let health_results = framework
        .validate_system_health()
        .await
        .expect("Failed to validate system health");

    // Should have at least checks for relay sockets
    assert!(health_results.len() >= 3); // One for each relay socket
}

// TODO: Add more comprehensive integration tests once the dependencies are resolved

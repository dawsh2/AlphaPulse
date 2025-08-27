//! Common test helper functions

use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::{timeout, Duration};

/// Initialize test logging (call once per test)
pub fn init_test_logging() {
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .with_env_filter("debug")
        .try_init();
}

/// Create a test timeout wrapper for async operations
pub async fn with_timeout<F, T>(duration_secs: u64, future: F) -> Result<T, &'static str>
where
    F: std::future::Future<Output = T>,
{
    timeout(Duration::from_secs(duration_secs), future)
        .await
        .map_err(|_| "Test timed out")
}

/// Sleep for testing with millisecond precision
pub async fn test_sleep_ms(millis: u64) {
    tokio::time::sleep(Duration::from_millis(millis)).await;
}

/// Generate current system timestamp for testing
pub fn current_timestamp_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_nanos() as u64
}

/// Assert that two floating point numbers are approximately equal
pub fn assert_approx_eq(a: f64, b: f64, epsilon: f64) {
    let diff = (a - b).abs();
    assert!(
        diff < epsilon,
        "Values not approximately equal: {} vs {} (diff: {}, epsilon: {})",
        a, b, diff, epsilon
    );
}

/// Assert that a value is within a percentage range of expected
pub fn assert_within_percent(actual: f64, expected: f64, percent: f64) {
    let tolerance = expected * (percent / 100.0);
    let diff = (actual - expected).abs();
    assert!(
        diff <= tolerance,
        "Value {} not within {}% of expected {} (tolerance: {}, actual diff: {})",
        actual, percent, expected, tolerance, diff
    );
}

/// Create a test pool address (deterministic for testing)
pub fn test_pool_address(index: u8) -> String {
    format!("0x{:040x}", index)
}

/// Generate test instrument ID
pub fn test_instrument_id(symbol: &str) -> String {
    format!("TEST:{}", symbol)
}

/// Wait for condition with timeout
pub async fn wait_for_condition<F>(
    mut condition: F,
    timeout_secs: u64,
    check_interval_ms: u64,
) -> Result<(), &'static str>
where
    F: FnMut() -> bool,
{
    let start = std::time::Instant::now();
    let timeout_duration = Duration::from_secs(timeout_secs);
    let check_interval = Duration::from_millis(check_interval_ms);

    while start.elapsed() < timeout_duration {
        if condition() {
            return Ok(());
        }
        tokio::time::sleep(check_interval).await;
    }

    Err("Condition not met within timeout")
}

/// Create test data directory if it doesn't exist
pub fn ensure_test_data_dir() -> std::path::PathBuf {
    let mut path = std::env::current_dir().expect("Failed to get current directory");
    path.push("test_data");
    
    if !path.exists() {
        std::fs::create_dir_all(&path).expect("Failed to create test data directory");
    }
    
    path
}
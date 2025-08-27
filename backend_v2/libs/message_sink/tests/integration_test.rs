//! Integration tests for SINK-003 SinkFactory implementation
//!
//! These tests validate the complete factory pattern implementation including:
//! - Configuration loading and parsing
//! - Service registry functionality
//! - Factory sink creation for all sink types
//! - Lazy connection wrapping
//! - Error handling and validation

use alphapulse_message_sink::*;
use std::io::Write;
use tempfile::TempDir;
use tokio::test;

/// Create a test TOML configuration with all sink types
fn create_test_config() -> &'static str {
    r#"
# Test configuration for SinkFactory integration tests

[services.test_relay]
type = "relay"
endpoint = "/tmp/test_relay.sock"
buffer_size = 5000

[services.test_direct_tcp]
type = "direct"  
endpoint = "tcp://localhost:8080"

[services.test_direct_ws]
type = "direct"
endpoint = "ws://localhost:9001"

[services.test_direct_unix]
type = "direct"
endpoint = "unix:///tmp/test_direct.sock"

[services.test_composite_fanout]
type = "composite"
pattern = "fanout"
targets = ["test_relay", "test_direct_tcp"]

[services.test_composite_round_robin]
type = "composite"
pattern = "round_robin"
targets = ["test_direct_tcp", "test_direct_ws"]

[services.test_composite_failover]
type = "composite"
pattern = "failover"
targets = ["test_direct_tcp", "test_direct_unix", "test_relay"]

[services.test_lazy_relay]
type = "relay"
endpoint = "/tmp/test_lazy_relay.sock"
buffer_size = 8000

[services.test_lazy_relay.lazy]
max_retries = 5
retry_delay_ms = 500
backoff_multiplier = 1.5
max_retry_delay_secs = 60
auto_reconnect = true
connect_timeout_secs = 10
wait_timeout_secs = 20

[services.test_lazy_direct]
type = "direct"
endpoint = "tcp://localhost:8081"

[services.test_lazy_direct.lazy]
max_retries = 3
retry_delay_ms = 250
backoff_multiplier = 2.0
max_retry_delay_secs = 30
auto_reconnect = false
connect_timeout_secs = 5
wait_timeout_secs = 15

[services.test_lazy_composite]
type = "composite"
pattern = "fanout"
targets = ["test_lazy_relay", "test_lazy_direct"]

[services.test_lazy_composite.lazy]
max_retries = 2
retry_delay_ms = 100
backoff_multiplier = 3.0
max_retry_delay_secs = 15
auto_reconnect = true
connect_timeout_secs = 3
wait_timeout_secs = 10
    "#
}

async fn create_test_factory() -> (SinkFactory, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("services.toml");
    
    let mut file = std::fs::File::create(&config_path).unwrap();
    file.write_all(create_test_config().as_bytes()).unwrap();
    
    let registry = ServiceRegistry::from_file(&config_path).unwrap();
    let factory = SinkFactory::new(registry);
    
    (factory, temp_dir)
}

#[test]
async fn test_factory_creation() {
    let (factory, _temp_dir) = create_test_factory().await;
    
    let stats = factory.stats().await;
    assert_eq!(stats.total_services, 12); // All services from config
    assert_eq!(stats.cached_sinks, 0);
    assert_eq!(stats.cache_hit_rate, 0.0);
    assert!(stats.name.contains("sink-factory"));
}

#[test]
async fn test_relay_sink_creation() {
    let (factory, _temp_dir) = create_test_factory().await;
    
    // Create basic relay sink
    let sink = factory.create_sink("test_relay").await.unwrap();
    let metadata = sink.metadata();
    
    // Should be wrapped in LazyMessageSink since no explicit lazy config = still wrapped
    assert!(metadata.name.contains("test_relay") || metadata.sink_type == "lazy");
    
    // Check that it's cached
    let stats = factory.stats().await;
    assert_eq!(stats.cached_sinks, 1);
    
    // Creating same sink should return cached version
    let sink2 = factory.create_sink("test_relay").await.unwrap();
    let stats2 = factory.stats().await;
    assert_eq!(stats2.cached_sinks, 1); // Still 1, not 2
}

#[test]
async fn test_direct_sink_creation_all_types() {
    let (factory, _temp_dir) = create_test_factory().await;
    
    // Test TCP direct sink
    let tcp_sink = factory.create_sink("test_direct_tcp").await.unwrap();
    let tcp_metadata = tcp_sink.metadata();
    assert!(tcp_metadata.name.contains("tcp") || tcp_metadata.sink_type == "lazy");
    
    // Test WebSocket direct sink
    let ws_sink = factory.create_sink("test_direct_ws").await.unwrap();
    let ws_metadata = ws_sink.metadata();
    assert!(ws_metadata.name.contains("ws") || ws_metadata.sink_type == "lazy");
    
    // Test Unix socket direct sink
    let unix_sink = factory.create_sink("test_direct_unix").await.unwrap();
    let unix_metadata = unix_sink.metadata();
    assert!(unix_metadata.name.contains("unix") || unix_metadata.sink_type == "lazy");
    
    // All should be cached
    let stats = factory.stats().await;
    assert_eq!(stats.cached_sinks, 3);
}

#[test]
async fn test_composite_sink_creation_all_patterns() {
    let (factory, _temp_dir) = create_test_factory().await;
    
    // Test fanout pattern
    let fanout_sink = factory.create_sink("test_composite_fanout").await.unwrap();
    let fanout_metadata = fanout_sink.metadata();
    assert!(fanout_metadata.name.contains("fanout") || fanout_metadata.sink_type == "lazy");
    
    // Test round-robin pattern  
    let rr_sink = factory.create_sink("test_composite_round_robin").await.unwrap();
    let rr_metadata = rr_sink.metadata();
    assert!(rr_metadata.name.contains("round-robin") || rr_metadata.sink_type == "lazy");
    
    // Test failover pattern
    let failover_sink = factory.create_sink("test_composite_failover").await.unwrap();
    let failover_metadata = failover_sink.metadata();
    assert!(failover_metadata.name.contains("failover") || failover_metadata.sink_type == "lazy");
    
    // Should have created composite sinks + their dependencies
    let stats = factory.stats().await;
    assert!(stats.cached_sinks >= 3); // At least the 3 composite sinks
}

#[test]
async fn test_lazy_configuration() {
    let (factory, _temp_dir) = create_test_factory().await;
    
    // Create lazy relay sink
    let lazy_relay = factory.create_sink("test_lazy_relay").await.unwrap();
    let relay_metadata = lazy_relay.metadata();
    assert_eq!(relay_metadata.sink_type, "lazy");
    
    // Create lazy direct sink
    let lazy_direct = factory.create_sink("test_lazy_direct").await.unwrap();
    let direct_metadata = lazy_direct.metadata();
    assert_eq!(direct_metadata.sink_type, "lazy");
    
    // Create lazy composite sink
    let lazy_composite = factory.create_sink("test_lazy_composite").await.unwrap();
    let composite_metadata = lazy_composite.metadata();
    assert_eq!(composite_metadata.sink_type, "lazy");
}

#[test]
async fn test_error_handling() {
    let (factory, _temp_dir) = create_test_factory().await;
    
    // Test nonexistent service
    let result = factory.create_sink("nonexistent_service").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
    
    // Test circular dependency (if we had one in config)
    // This would be caught by the factory's circular dependency detection
}

#[test]
async fn test_config_validation() {
    let (factory, _temp_dir) = create_test_factory().await;
    
    // Valid service should validate
    assert!(factory.validate_config("test_relay").is_ok());
    assert!(factory.validate_config("test_composite_fanout").is_ok());
    
    // Invalid service should fail validation
    assert!(factory.validate_config("nonexistent").is_err());
}

#[test] 
async fn test_cache_management() {
    let (factory, _temp_dir) = create_test_factory().await;
    
    // Create some sinks
    factory.create_sink("test_relay").await.unwrap();
    factory.create_sink("test_direct_tcp").await.unwrap();
    
    let stats = factory.stats().await;
    assert_eq!(stats.cached_sinks, 2);
    
    // Get cached sinks list
    let cached = factory.cached_sinks().await;
    assert_eq!(cached.len(), 2);
    
    // Remove one from cache
    let removed = factory.remove_from_cache("test_relay").await;
    assert!(removed);
    
    let stats_after_removal = factory.stats().await;
    assert_eq!(stats_after_removal.cached_sinks, 1);
    
    // Clear entire cache
    factory.clear_cache().await;
    
    let stats_after_clear = factory.stats().await;
    assert_eq!(stats_after_clear.cached_sinks, 0);
}

#[test]
async fn test_factory_with_custom_name() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("services.toml");
    
    let mut file = std::fs::File::create(&config_path).unwrap();
    file.write_all(create_test_config().as_bytes()).unwrap();
    
    let registry = ServiceRegistry::from_file(&config_path).unwrap();
    let factory = SinkFactory::with_name(registry, "custom-factory");
    
    let stats = factory.stats().await;
    assert_eq!(stats.name, "custom-factory");
}

#[test]
async fn test_lazy_config_conversion() {
    // Test LazyConfigToml to LazyConfig conversion
    let lazy_config_toml = LazyConfigToml {
        max_retries: Some(5),
        retry_delay_ms: Some(1000),
        backoff_multiplier: Some(2.0),
        max_retry_delay_secs: Some(60),
        auto_reconnect: Some(false),
        connect_timeout_secs: Some(15),
        wait_timeout_secs: Some(30),
    };
    
    let lazy_config = lazy_config_toml.to_lazy_config();
    
    assert_eq!(lazy_config.max_retries, 5);
    assert_eq!(lazy_config.retry_delay.as_millis(), 1000);
    assert_eq!(lazy_config.backoff_multiplier, 2.0);
    assert_eq!(lazy_config.max_retry_delay.as_secs(), 60);
    assert!(!lazy_config.auto_reconnect);
    assert_eq!(lazy_config.connect_timeout.as_secs(), 15);
    assert_eq!(lazy_config.wait_timeout.as_secs(), 30);
}

#[test]
async fn test_invalid_configurations() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("invalid.toml");
    
    // Test config with missing endpoint
    let invalid_config = r#"
[services.missing_endpoint]
type = "relay"
# endpoint is missing
buffer_size = 1000
    "#;
    
    std::fs::write(&config_path, invalid_config).unwrap();
    
    let registry = ServiceRegistry::from_file(&config_path).unwrap();
    let factory = SinkFactory::new(registry);
    
    let result = factory.create_sink("missing_endpoint").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("missing endpoint"));
}

#[test]  
async fn test_circular_dependency_prevention() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("circular.toml");
    
    // Create config with circular dependency
    let circular_config = r#"
[services.circular_a]
type = "composite"
pattern = "fanout"
targets = ["circular_b"]

[services.circular_b]
type = "composite"
pattern = "fanout"
targets = ["circular_a"]
    "#;
    
    std::fs::write(&config_path, circular_config).unwrap();
    
    let registry = ServiceRegistry::from_file(&config_path).unwrap();
    let factory = SinkFactory::new(registry);
    
    // This should be detected and prevented
    let result = factory.create_sink("circular_a").await;
    assert!(result.is_err());
}

#[test]
async fn test_sink_metadata_consistency() {
    let (factory, _temp_dir) = create_test_factory().await;
    
    // Create sinks of each type and verify metadata consistency
    let relay_sink = factory.create_sink("test_relay").await.unwrap();
    let direct_sink = factory.create_sink("test_direct_tcp").await.unwrap();
    let composite_sink = factory.create_sink("test_composite_fanout").await.unwrap();
    
    // All should have consistent metadata structure
    let relay_meta = relay_sink.metadata();
    let direct_meta = direct_sink.metadata();
    let composite_meta = composite_sink.metadata();
    
    assert!(!relay_meta.name.is_empty());
    assert!(!direct_meta.name.is_empty());
    assert!(!composite_meta.name.is_empty());
    
    assert!(!relay_meta.sink_type.is_empty());
    assert!(!direct_meta.sink_type.is_empty());
    assert!(!composite_meta.sink_type.is_empty());
}

#[test]
async fn test_factory_stats_accuracy() {
    let (factory, _temp_dir) = create_test_factory().await;
    
    let initial_stats = factory.stats().await;
    assert_eq!(initial_stats.cached_sinks, 0);
    
    // Create several sinks
    factory.create_sink("test_relay").await.unwrap();
    factory.create_sink("test_direct_tcp").await.unwrap();
    
    let after_creation_stats = factory.stats().await;
    assert_eq!(after_creation_stats.cached_sinks, 2);
    
    // Cache hit rate should be calculated correctly
    let hit_rate = after_creation_stats.cached_sinks as f64 / after_creation_stats.total_services as f64;
    assert_eq!(after_creation_stats.cache_hit_rate, hit_rate);
}
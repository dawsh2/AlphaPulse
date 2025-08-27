//! Core E2E testing framework

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Main test framework coordinator
pub struct TestFramework {
    config: TestConfig,
    services: Arc<Mutex<HashMap<String, ServiceHandle>>>,
    relay_paths: RelayPaths,
    test_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    /// Test timeout in seconds
    pub timeout_secs: u64,

    /// Cleanup after test
    pub cleanup: bool,

    /// Enable detailed logging
    pub verbose: bool,

    /// Data validation level
    pub validation_level: ValidationLevel,

    /// Test data directory
    pub data_dir: PathBuf,

    /// Signal relay socket path for tests
    pub signal_relay_path: String,

    /// Dashboard WebSocket port for tests
    pub dashboard_port: u16,

    /// Test timeout in seconds (alternative field)
    pub test_timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationLevel {
    /// Basic connectivity tests
    Basic,
    /// Data integrity validation
    DataIntegrity,
    /// Full precision and latency validation
    Comprehensive,
}

#[derive(Debug, Clone)]
pub struct RelayPaths {
    pub market_data: String,
    pub signals: String,
    pub execution: String,
}

pub struct ServiceHandle {
    pub name: String,
    pub handle: tokio::task::JoinHandle<Result<()>>,
    pub health_endpoint: Option<String>,
}

/// Test scenario trait
#[async_trait::async_trait]
pub trait TestScenario {
    async fn setup(&self, framework: &TestFramework) -> Result<()>;
    async fn execute(&self, framework: &TestFramework) -> Result<TestResult>;
    async fn cleanup(&self, framework: &TestFramework) -> Result<()>;

    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn timeout(&self) -> Duration;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub scenario_name: String,
    pub success: bool,
    pub duration: Duration,
    pub error_message: Option<String>,
    pub metrics: TestMetrics,
    pub validation_results: Vec<ValidationResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestMetrics {
    pub messages_processed: u64,
    pub avg_latency_ns: u64,
    pub max_latency_ns: u64,
    pub throughput_msg_per_sec: f64,
    pub memory_usage_mb: f64,
    pub precision_errors: u32,
    pub signals_generated: u32,
    pub dashboard_connections: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub validator: String,
    pub passed: bool,
    pub message: String,
    pub severity: ValidationSeverity,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 300, // 5 minutes
            cleanup: true,
            verbose: false,
            validation_level: ValidationLevel::Comprehensive,
            data_dir: PathBuf::from("/tmp/alphapulse_e2e_tests"),
            signal_relay_path: "/tmp/alphapulse_test/signals.sock".to_string(),
            dashboard_port: 8080,
            test_timeout_secs: 30,
        }
    }
}

impl TestFramework {
    pub fn new(config: TestConfig) -> Result<Self> {
        let test_id = Uuid::new_v4();
        let relay_paths = RelayPaths {
            market_data: format!("/tmp/alphapulse/e2e_{}/market_data.sock", test_id),
            signals: format!("/tmp/alphapulse/e2e_{}/signals.sock", test_id),
            execution: format!("/tmp/alphapulse/e2e_{}/execution.sock", test_id),
        };

        // Create relay socket directory
        std::fs::create_dir_all(format!("/tmp/alphapulse/e2e_{}", test_id))?;

        Ok(Self {
            config,
            services: Arc::new(Mutex::new(HashMap::new())),
            relay_paths,
            test_id,
        })
    }

    /// Run a complete test scenario
    pub async fn run_scenario<S: TestScenario>(&self, scenario: S) -> Result<TestResult> {
        info!("Starting test scenario: {}", scenario.name());
        info!("Description: {}", scenario.description());
        info!("Test ID: {}", self.test_id);

        let start_time = Instant::now();

        // Setup phase
        info!("Setting up test scenario...");
        if let Err(e) = scenario.setup(self).await {
            error!("Setup failed: {}", e);
            return Ok(TestResult {
                scenario_name: scenario.name().to_string(),
                success: false,
                duration: start_time.elapsed(),
                error_message: Some(format!("Setup failed: {}", e)),
                metrics: TestMetrics::default(),
                validation_results: vec![],
            });
        }

        // Execute with timeout
        let execution_result =
            tokio::time::timeout(scenario.timeout(), scenario.execute(self)).await;

        let mut test_result = match execution_result {
            Ok(Ok(result)) => result,
            Ok(Err(e)) => {
                error!("Test execution failed: {}", e);
                TestResult {
                    scenario_name: scenario.name().to_string(),
                    success: false,
                    duration: start_time.elapsed(),
                    error_message: Some(format!("Execution failed: {}", e)),
                    metrics: TestMetrics::default(),
                    validation_results: vec![],
                }
            }
            Err(_) => {
                error!("Test execution timed out");
                TestResult {
                    scenario_name: scenario.name().to_string(),
                    success: false,
                    duration: start_time.elapsed(),
                    error_message: Some("Test execution timed out".to_string()),
                    metrics: TestMetrics::default(),
                    validation_results: vec![],
                }
            }
        };

        test_result.duration = start_time.elapsed();

        // Cleanup phase
        if self.config.cleanup {
            info!("Cleaning up test scenario...");
            if let Err(e) = scenario.cleanup(self).await {
                warn!("Cleanup failed: {}", e);
            }
        }

        info!(
            "Test scenario completed: {} (success: {})",
            scenario.name(),
            test_result.success
        );

        Ok(test_result)
    }

    /// Start a service for testing
    pub async fn start_service<F, Fut>(&self, name: String, service_fn: F) -> Result<()>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        info!("Starting service: {}", name);

        let handle = tokio::spawn(async move { service_fn().await });

        let service_handle = ServiceHandle {
            name: name.clone(),
            handle,
            health_endpoint: None,
        };

        self.services.lock().await.insert(name, service_handle);

        // Give service time to start
        tokio::time::sleep(Duration::from_millis(500)).await;

        Ok(())
    }

    /// Stop a service
    pub async fn stop_service(&self, name: &str) -> Result<()> {
        info!("Stopping service: {}", name);

        let mut services = self.services.lock().await;
        if let Some(service) = services.remove(name) {
            service.handle.abort();

            // Wait a bit for graceful shutdown
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Ok(())
    }

    /// Stop all services
    pub async fn stop_all_services(&self) -> Result<()> {
        info!("Stopping all services");

        let mut services = self.services.lock().await;
        for (name, service) in services.drain() {
            debug!("Stopping service: {}", name);
            service.handle.abort();
        }

        // Wait for graceful shutdown
        tokio::time::sleep(Duration::from_millis(500)).await;

        Ok(())
    }

    /// Get relay paths for test
    pub fn relay_paths(&self) -> &RelayPaths {
        &self.relay_paths
    }

    /// Get test configuration
    pub fn config(&self) -> &TestConfig {
        &self.config
    }

    /// Get test ID
    pub fn test_id(&self) -> Uuid {
        self.test_id
    }

    /// Validate system health
    pub async fn validate_system_health(&self) -> Result<Vec<ValidationResult>> {
        let mut results = Vec::new();

        // Check relay socket existence
        for (name, path) in [
            ("market_data", &self.relay_paths.market_data),
            ("signals", &self.relay_paths.signals),
            ("execution", &self.relay_paths.execution),
        ] {
            if std::path::Path::new(path).exists() {
                results.push(ValidationResult {
                    validator: format!("relay_socket_{}", name),
                    passed: true,
                    message: format!("Relay socket {} exists", name),
                    severity: ValidationSeverity::Info,
                    details: None,
                });
            } else {
                results.push(ValidationResult {
                    validator: format!("relay_socket_{}", name),
                    passed: false,
                    message: format!("Relay socket {} missing", name),
                    severity: ValidationSeverity::Error,
                    details: None,
                });
            }
        }

        // Check service health
        let services = self.services.lock().await;
        for (name, service) in services.iter() {
            if service.handle.is_finished() {
                results.push(ValidationResult {
                    validator: "service_health".to_string(),
                    passed: false,
                    message: format!("Service {} has stopped unexpectedly", name),
                    severity: ValidationSeverity::Critical,
                    details: None,
                });
            } else {
                results.push(ValidationResult {
                    validator: "service_health".to_string(),
                    passed: true,
                    message: format!("Service {} is running", name),
                    severity: ValidationSeverity::Info,
                    details: None,
                });
            }
        }

        Ok(results)
    }
}

impl Default for TestMetrics {
    fn default() -> Self {
        Self {
            messages_processed: 0,
            avg_latency_ns: 0,
            max_latency_ns: 0,
            throughput_msg_per_sec: 0.0,
            memory_usage_mb: 0.0,
            precision_errors: 0,
            signals_generated: 0,
            dashboard_connections: 0,
        }
    }
}

/// Simplified test runner for basic E2E tests
pub struct TestRunner {
    pub config: TestConfig,
    services: Vec<tokio::task::JoinHandle<()>>,
}

impl TestRunner {
    pub fn new(config: TestConfig) -> Self {
        // Create test directories
        std::fs::create_dir_all("/tmp/alphapulse_test").ok();

        Self {
            config,
            services: Vec::new(),
        }
    }

    pub async fn start_signal_relay(&mut self) -> Result<()> {
        info!(
            "Starting signal relay for test: {}",
            self.config.signal_relay_path
        );

        let relay_path = self.config.signal_relay_path.clone();
        let handle = tokio::spawn(async move {
            // Mock signal relay that just accepts connections
            let _ = std::fs::remove_file(&relay_path);
            let listener = tokio::net::UnixListener::bind(&relay_path).unwrap();

            loop {
                if let Ok((mut stream, _)) = listener.accept().await {
                    tokio::spawn(async move {
                        let mut buf = vec![0u8; 1024];
                        while stream.readable().await.is_ok() {
                            if stream.try_read(&mut buf).is_err() {
                                break;
                            }
                        }
                    });
                }
            }
        });

        self.services.push(handle);
        tokio::time::sleep(Duration::from_millis(500)).await; // Let it start
        Ok(())
    }

    pub async fn start_dashboard(&mut self) -> Result<()> {
        info!(
            "Starting dashboard WebSocket server on port {}",
            self.config.dashboard_port
        );

        let port = self.config.dashboard_port;
        let signal_relay_path = self.config.signal_relay_path.clone();

        let handle = tokio::spawn(async move {
            use alphapulse_dashboard_websocket::{DashboardConfig, DashboardServer};

            let config = DashboardConfig {
                bind_address: "127.0.0.1".to_string(),
                port,
                market_data_relay_path: "/tmp/unused_market.sock".to_string(),
                signal_relay_path,
                execution_relay_path: "/tmp/unused_execution.sock".to_string(),
                max_connections: 100,
                client_buffer_size: 1000,
                enable_cors: true,
                heartbeat_interval_secs: 30,
            };

            let server = DashboardServer::new(config);
            let _ = server.start().await;
        });

        self.services.push(handle);
        tokio::time::sleep(Duration::from_secs(2)).await; // Let it start
        Ok(())
    }

    pub async fn wait_for_dashboard_message(
        &self,
        mut ws_stream: tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        expected_msg_type: &str,
        timeout: Duration,
    ) -> Result<(
        serde_json::Value,
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    )> {
        use futures_util::StreamExt;
        use tokio_tungstenite::tungstenite::Message;

        let deadline = tokio::time::Instant::now() + timeout;

        while tokio::time::Instant::now() < deadline {
            tokio::select! {
                msg = ws_stream.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                                if json.get("msg_type").and_then(|v| v.as_str()) == Some(expected_msg_type) {
                                    return Ok((json, ws_stream));
                                }
                            }
                        }
                        Some(Err(e)) => {
                            return Err(anyhow::anyhow!("WebSocket error: {}", e));
                        }
                        None => {
                            return Err(anyhow::anyhow!("WebSocket connection closed"));
                        }
                        _ => {} // Ignore other message types
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(100)) => {
                    // Continue waiting
                }
            }
        }

        Err(anyhow::anyhow!(
            "Timeout waiting for message type: {}",
            expected_msg_type
        ))
    }

    pub async fn shutdown(&mut self) {
        info!("Shutting down test services");

        for handle in self.services.drain(..) {
            handle.abort();
        }

        // Cleanup test files
        let _ = std::fs::remove_file(&self.config.signal_relay_path);
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

impl Drop for TestFramework {
    fn drop(&mut self) {
        // Cleanup relay directory
        if self.config.cleanup {
            let _ = std::fs::remove_dir_all(format!("/tmp/alphapulse/e2e_{}", self.test_id));
        }
    }
}

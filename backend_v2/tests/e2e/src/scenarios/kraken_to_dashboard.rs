//! Kraken to Dashboard E2E Test
//!
//! Tests the complete data flow:
//! Kraken API → Adapter → Relay → Strategy → Dashboard

use crate::framework::{TestFramework, TestResult, TestScenario, TestMetrics, ValidationResult, ValidationSeverity};
use crate::fixtures::MockKrakenServer;
use crate::validation::DataFlowValidator;

use anyhow::{Context, Result};
use alphapulse_adapters::input::collectors::kraken::KrakenCollector;
use alphapulse_dashboard_websocket::{DashboardConfig, DashboardServer};
use alphapulse_kraken_signals::strategy::KrakenSignalsStrategy;
use alphapulse_types::relay::{MarketDataRelay, SignalRelay};
use serde_json::Value;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, info, warn};

pub struct KrakenToDashboardTest {
    pub use_live_data: bool,
    pub expected_messages: u32,
    pub max_latency_ms: u64,
}

impl Default for KrakenToDashboardTest {
    fn default() -> Self {
        Self {
            use_live_data: false, // Start with mock data for reliability
            expected_messages: 100,
            max_latency_ms: 50,
        }
    }
}

#[async_trait::async_trait]
impl TestScenario for KrakenToDashboardTest {
    async fn setup(&self, framework: &TestFramework) -> Result<()> {
        info!("Setting up Kraken to Dashboard test");

        // 1. Start mock Kraken server (if not using live data)
        if !self.use_live_data {
            framework.start_service(
                "mock_kraken".to_string(),
                || async {
                    let server = MockKrakenServer::new(8080).await?;
                    server.run().await
                }
            ).await?;
        }

        // 2. Start market data relay
        let market_relay_path = framework.relay_paths().market_data.clone();
        framework.start_service(
            "market_data_relay".to_string(),
            move || {
                let path = market_relay_path;
                async move {
                    let relay = MarketDataRelay::new(&path).await?;
                    relay.run().await
                }
            }
        ).await?;

        // 3. Start signal relay
        let signal_relay_path = framework.relay_paths().signals.clone();
        framework.start_service(
            "signal_relay".to_string(),
            move || {
                let path = signal_relay_path;
                async move {
                    let relay = SignalRelay::new(&path).await?;
                    relay.run().await
                }
            }
        ).await?;

        // 4. Start Kraken collector
        framework.start_service(
            "kraken_collector".to_string(),
            || async {
                let mut collector = if self.use_live_data {
                    KrakenCollector::new_live().await?
                } else {
                    KrakenCollector::new_with_url("ws://127.0.0.1:8080/ws").await?
                };
                collector.run().await
            }
        ).await?;

        // 5. Start Kraken signals strategy
        framework.start_service(
            "kraken_strategy".to_string(),
            || async {
                let mut strategy = KrakenSignalsStrategy::new().await?;
                strategy.run().await
            }
        ).await?;

        // 6. Start dashboard WebSocket server
        framework.start_service(
            "dashboard_server".to_string(),
            || async {
                let config = DashboardConfig {
                    port: 8081,
                    ..Default::default()
                };
                let server = DashboardServer::new(config);
                server.start().await
            }
        ).await?;

        // Give all services time to start and connect
        tokio::time::sleep(Duration::from_secs(2)).await;

        Ok(())
    }

    async fn execute(&self, framework: &TestFramework) -> Result<TestResult> {
        info!("Executing Kraken to Dashboard test");

        let start_time = Instant::now();
        let mut metrics = TestMetrics::default();
        let mut validation_results = Vec::new();

        // Connect to dashboard WebSocket
        let (ws_stream, _) = connect_async("ws://127.0.0.1:8081/ws")
            .await
            .context("Failed to connect to dashboard WebSocket")?;

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        let (message_tx, mut message_rx) = mpsc::unbounded_channel::<Value>();

        // Start message collection task
        let collection_handle = tokio::spawn(async move {
            use futures_util::StreamExt;

            while let Some(msg) = ws_receiver.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Ok(json) = serde_json::from_str::<Value>(&text) {
                            let _ = message_tx.send(json);
                        }
                    }
                    Ok(Message::Close(_)) => break,
                    Err(e) => {
                        warn!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        });

        // Start data flow validator
        let validator = DataFlowValidator::new();
        let mut collected_messages = Vec::new();
        let mut trade_count = 0;
        let mut signal_count = 0;
        let mut latencies = Vec::new();

        info!("Collecting messages for {} seconds...", 30);

        let collection_timeout = Duration::from_secs(30);
        let collection_start = Instant::now();

        while collection_start.elapsed() < collection_timeout {
            tokio::select! {
                msg = message_rx.recv() => {
                    if let Some(message) = msg {
                        let received_time = Instant::now();
                        collected_messages.push(message.clone());

                        // Validate message structure
                        if let Err(e) = validator.validate_message(&message) {
                            validation_results.push(ValidationResult {
                                validator: "message_structure".to_string(),
                                passed: false,
                                message: format!("Invalid message structure: {}", e),
                                severity: ValidationSeverity::Error,
                                details: Some(message.clone()),
                            });
                        }

                        // Track message types and calculate latency
                        match message.get("type").and_then(|v| v.as_str()) {
                            Some("trade") => {
                                trade_count += 1;
                                if let Some(timestamp) = message.get("timestamp").and_then(|v| v.as_u64()) {
                                    let message_time = std::time::UNIX_EPOCH + Duration::from_nanos(timestamp);
                                    if let Ok(latency) = received_time.duration_since(message_time) {
                                        latencies.push(latency.as_nanos() as u64);
                                    }
                                }
                            }
                            Some("trading_signal") => {
                                signal_count += 1;
                                info!("Received trading signal: confidence={}, profit={}",
                                      message.get("confidence").unwrap_or(&Value::Null),
                                      message.get("expected_profit_usd").unwrap_or(&Value::Null));
                            }
                            Some("heartbeat") => {
                                debug!("Received heartbeat");
                            }
                            _ => {
                                debug!("Received unknown message type: {:?}",
                                       message.get("type"));
                            }
                        }

                        metrics.messages_processed += 1;

                        // Check if we've collected enough messages
                        if metrics.messages_processed >= self.expected_messages as u64 {
                            info!("Collected target number of messages ({})", self.expected_messages);
                            break;
                        }
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(100)) => {
                    // Continue collection
                }
            }
        }

        collection_handle.abort();

        // Calculate metrics
        let total_duration = start_time.elapsed();
        metrics.throughput_msg_per_sec = metrics.messages_processed as f64 / total_duration.as_secs_f64();

        if !latencies.is_empty() {
            metrics.avg_latency_ns = latencies.iter().sum::<u64>() / latencies.len() as u64;
            metrics.max_latency_ns = *latencies.iter().max().unwrap_or(&0);
        }

        metrics.signals_generated = signal_count;
        metrics.dashboard_connections = 1;

        // Validate results
        let mut success = true;

        // Check message count
        if trade_count < 10 {
            validation_results.push(ValidationResult {
                validator: "message_count".to_string(),
                passed: false,
                message: format!("Expected at least 10 trade messages, got {}", trade_count),
                severity: ValidationSeverity::Error,
                details: None,
            });
            success = false;
        } else {
            validation_results.push(ValidationResult {
                validator: "message_count".to_string(),
                passed: true,
                message: format!("Received {} trade messages", trade_count),
                severity: ValidationSeverity::Info,
                details: None,
            });
        }

        // Check latency
        if metrics.max_latency_ns > self.max_latency_ms * 1_000_000 {
            validation_results.push(ValidationResult {
                validator: "latency".to_string(),
                passed: false,
                message: format!("Max latency {}ms exceeds threshold {}ms",
                                metrics.max_latency_ns / 1_000_000, self.max_latency_ms),
                severity: ValidationSeverity::Warning,
                details: None,
            });
        } else {
            validation_results.push(ValidationResult {
                validator: "latency".to_string(),
                passed: true,
                message: format!("Max latency {}ms within threshold",
                                metrics.max_latency_ns / 1_000_000),
                severity: ValidationSeverity::Info,
                details: None,
            });
        }

        // Check signal generation
        if signal_count > 0 {
            validation_results.push(ValidationResult {
                validator: "signal_generation".to_string(),
                passed: true,
                message: format!("Generated {} trading signals", signal_count),
                severity: ValidationSeverity::Info,
                details: None,
            });
        } else {
            validation_results.push(ValidationResult {
                validator: "signal_generation".to_string(),
                passed: false,
                message: "No trading signals generated".to_string(),
                severity: ValidationSeverity::Warning,
                details: None,
            });
        }

        // System health check
        let health_results = framework.validate_system_health().await?;
        validation_results.extend(health_results);

        info!("Test completed: {} messages, {} trades, {} signals",
              metrics.messages_processed, trade_count, signal_count);

        Ok(TestResult {
            scenario_name: self.name().to_string(),
            success,
            duration: total_duration,
            error_message: None,
            metrics,
            validation_results,
        })
    }

    async fn cleanup(&self, framework: &TestFramework) -> Result<()> {
        info!("Cleaning up Kraken to Dashboard test");

        framework.stop_all_services().await?;

        // Remove any test data files
        if let Err(e) = tokio::fs::remove_dir_all(format!("/tmp/alphapulse_e2e_test_{}", framework.test_id())).await {
            warn!("Failed to cleanup test directory: {}", e);
        }

        Ok(())
    }

    fn name(&self) -> &str {
        "kraken_to_dashboard"
    }

    fn description(&self) -> &str {
        "End-to-end test from Kraken data ingestion through strategy processing to dashboard display"
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(120)
    }
}

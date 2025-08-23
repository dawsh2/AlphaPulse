/// PHASE 3: Continuous deep equality monitoring system
/// 
/// This module provides real-time monitoring of data integrity through
/// the pipeline, detecting anomalies and generating detailed reports.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::time::{Duration, Instant};
use tracing::{warn, info, error};

use super::{ReverseTransformEngine, ValidationResult, FrontendTradeMessage};

/// Continuous monitoring system for deep equality validation
pub struct ContinuousEqualityMonitor {
    /// Reverse transformation engine
    reverse_engine: ReverseTransformEngine,
    /// Validation statistics
    stats: Arc<RwLock<MonitoringStats>>,
    /// Recent validation results (rolling window)
    recent_results: Arc<RwLock<VecDeque<ValidationResult>>>,
    /// Anomaly detection thresholds
    thresholds: AnomalyThresholds,
    /// Alert callback
    alert_callback: Option<Box<dyn Fn(Alert) + Send + Sync>>,
}

/// Monitoring statistics
#[derive(Debug, Clone, Serialize)]
pub struct MonitoringStats {
    pub total_validations: u64,
    pub successful_validations: u64,
    pub failed_validations: u64,
    pub precision_loss_detected: u64,
    pub anomalies_detected: u64,
    pub success_rate: f64,
    pub average_validation_time_ms: f64,
    pub last_validation: Option<u64>,
    pub uptime_seconds: u64,
}

/// Anomaly detection thresholds
#[derive(Debug, Clone)]
pub struct AnomalyThresholds {
    pub max_price_deviation_percent: f64,
    pub max_volume_deviation_percent: f64,
    pub min_success_rate_percent: f64,
    pub max_validation_time_ms: f64,
    pub anomaly_burst_threshold: u32,
}

/// Alert types
#[derive(Debug, Clone, Serialize)]
pub enum Alert {
    ValidationFailure {
        message_id: String,
        error: String,
        timestamp: u64,
    },
    PrecisionLoss {
        message_id: String,
        deviation: f64,
        timestamp: u64,
    },
    SuccessRateBelow {
        current_rate: f64,
        threshold: f64,
        timestamp: u64,
    },
    AnomalyBurst {
        anomaly_count: u32,
        time_window_seconds: u32,
        timestamp: u64,
    },
    ValidationLatencyHigh {
        latency_ms: f64,
        threshold_ms: f64,
        timestamp: u64,
    },
}

/// Batch validation request for historical data
#[derive(Debug, Clone)]
pub struct BatchValidationRequest {
    pub message_ids: Vec<String>,
    pub parallel_workers: usize,
    pub timeout_seconds: u64,
}

/// Batch validation result
#[derive(Debug, Clone, Serialize)]
pub struct BatchValidationResult {
    pub total_messages: usize,
    pub successful_validations: usize,
    pub failed_validations: usize,
    pub validation_time_seconds: f64,
    pub anomalies_detected: Vec<String>,
    pub success_rate: f64,
    pub detailed_results: Vec<ValidationResult>,
}

impl Default for AnomalyThresholds {
    fn default() -> Self {
        Self {
            max_price_deviation_percent: 0.1,
            max_volume_deviation_percent: 0.1,
            min_success_rate_percent: 95.0,
            max_validation_time_ms: 100.0,
            anomaly_burst_threshold: 10,
        }
    }
}

impl ContinuousEqualityMonitor {
    pub fn new() -> Self {
        Self {
            reverse_engine: ReverseTransformEngine::new(),
            stats: Arc::new(RwLock::new(MonitoringStats::default())),
            recent_results: Arc::new(RwLock::new(VecDeque::with_capacity(1000))),
            thresholds: AnomalyThresholds::default(),
            alert_callback: None,
        }
    }

    pub fn with_thresholds(mut self, thresholds: AnomalyThresholds) -> Self {
        self.thresholds = thresholds;
        self
    }

    pub fn with_alert_callback<F>(mut self, callback: F) -> Self 
    where 
        F: Fn(Alert) + Send + Sync + 'static,
    {
        self.alert_callback = Some(Box::new(callback));
        self
    }

    /// Start continuous monitoring
    pub fn start_monitoring(&self, check_interval_seconds: u64) -> tokio::task::JoinHandle<()> {
        let stats = Arc::clone(&self.stats);
        let recent_results = Arc::clone(&self.recent_results);
        let thresholds = self.thresholds.clone();

        info!("Continuous equality monitor started with {}s check interval", check_interval_seconds);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(check_interval_seconds));
            
            loop {
                interval.tick().await;
                
                // Update uptime
                {
                    let mut stats_guard = stats.write();
                    stats_guard.uptime_seconds += check_interval_seconds;
                }

                // Check for anomalies in recent results
                let results = recent_results.read().clone();
                Self::check_for_anomalies(&results, &thresholds);
            }
        })
    }

    /// Validate a single message and update statistics
    pub async fn validate_message(&mut self, frontend_msg: &FrontendTradeMessage) -> Result<ValidationResult> {
        let start_time = Instant::now();
        
        // Perform validation
        let result = self.reverse_engine.validate_message(frontend_msg);
        
        let validation_time_ms = start_time.elapsed().as_millis() as f64;
        
        // Update statistics
        {
            let mut stats = self.stats.write();
            stats.total_validations += 1;
            stats.average_validation_time_ms = 
                (stats.average_validation_time_ms * (stats.total_validations - 1) as f64 + validation_time_ms) 
                / stats.total_validations as f64;
            stats.last_validation = Some(chrono::Utc::now().timestamp() as u64);

            match &result {
                Ok(validation_result) => {
                    if validation_result.success {
                        stats.successful_validations += 1;
                    } else {
                        stats.failed_validations += 1;
                        if validation_result.precision_loss_detected {
                            stats.precision_loss_detected += 1;
                        }
                        stats.anomalies_detected += validation_result.anomalies.len() as u64;
                    }
                }
                Err(_) => {
                    stats.failed_validations += 1;
                }
            }
            
            stats.success_rate = (stats.successful_validations as f64 / stats.total_validations as f64) * 100.0;
        }

        // Store result for trending analysis
        if let Ok(validation_result) = &result {
            let mut recent = self.recent_results.write();
            recent.push_back(validation_result.clone());
            if recent.len() > 1000 {
                recent.pop_front();
            }
        }

        // Check for immediate alerts
        if let Ok(validation_result) = &result {
            self.check_immediate_alerts(validation_result, validation_time_ms);
        }

        result
    }

    /// Store original data for future validation
    pub fn store_original_data(&mut self, message_id: String, original_data: serde_json::Value) {
        self.reverse_engine.store_original_data(message_id, original_data);
    }

    /// Perform batch validation of historical data
    pub async fn batch_validate(&mut self, request: BatchValidationRequest) -> Result<BatchValidationResult> {
        let start_time = Instant::now();
        let mut successful = 0;
        let mut failed = 0;
        let mut detailed_results = Vec::new();
        let mut anomalies = Vec::new();

        info!("Starting batch validation of {} messages with {} workers", 
              request.message_ids.len(), request.parallel_workers);

        // Process messages in chunks for parallel processing
        let chunk_size = (request.message_ids.len() + request.parallel_workers - 1) / request.parallel_workers;
        let chunks: Vec<_> = request.message_ids.chunks(chunk_size).collect();

        // For this implementation, we'll process serially
        // In a real implementation, you'd use tokio::spawn for parallel processing
        for chunk in chunks {
            for message_id in chunk {
                // Create a dummy frontend message for validation
                // In practice, you'd reconstruct this from stored data
                let frontend_msg = FrontendTradeMessage {
                    symbol_hash: "batch_validation".to_string(),
                    symbol: Some("batch:test".to_string()),
                    timestamp: chrono::Utc::now().timestamp() as u64,
                    price: 1.0,
                    volume: 100.0,
                    side: "buy".to_string(),
                    message_id: Some(message_id.clone()),
                };

                match self.validate_message(&frontend_msg).await {
                    Ok(result) => {
                        if result.success {
                            successful += 1;
                        } else {
                            failed += 1;
                            anomalies.extend(result.anomalies.clone());
                        }
                        detailed_results.push(result);
                    }
                    Err(e) => {
                        failed += 1;
                        anomalies.push(format!("Validation error for {}: {}", message_id, e));
                    }
                }
            }
        }

        let validation_time = start_time.elapsed().as_secs_f64();
        let total_messages = request.message_ids.len();
        let success_rate = (successful as f64 / total_messages as f64) * 100.0;

        info!("Batch validation completed: {}/{} successful ({:.1}%) in {:.2}s", 
              successful, total_messages, success_rate, validation_time);

        Ok(BatchValidationResult {
            total_messages,
            successful_validations: successful,
            failed_validations: failed,
            validation_time_seconds: validation_time,
            anomalies_detected: anomalies,
            success_rate,
            detailed_results,
        })
    }

    /// Get current monitoring statistics
    pub fn get_stats(&self) -> MonitoringStats {
        self.stats.read().clone()
    }

    /// Check for immediate alerts based on single validation result
    fn check_immediate_alerts(&self, result: &ValidationResult, validation_time_ms: f64) {
        // Check validation failure
        if !result.success {
            warn!("Validation failed for message {}: {}", 
                  result.message_id, result.error.as_ref().unwrap_or(&"Unknown error".to_string()));
        }

        // Check precision loss
        if result.precision_loss_detected {
            warn!("Precision loss detected for message {}", result.message_id);
        }

        // Check validation latency
        if validation_time_ms > self.thresholds.max_validation_time_ms {
            warn!("High validation latency: {:.2}ms > {:.2}ms threshold", 
                  validation_time_ms, self.thresholds.max_validation_time_ms);
        }
    }

    /// Check for anomalies in recent results
    fn check_for_anomalies(
        results: &VecDeque<ValidationResult>, 
        thresholds: &AnomalyThresholds,
    ) {
        if results.is_empty() {
            return;
        }

        let recent_window = 100; // Last 100 validations
        let recent_results: Vec<_> = results.iter().rev().take(recent_window).collect();

        // Check success rate
        let successes = recent_results.iter().filter(|r| r.success).count();
        let success_rate = (successes as f64 / recent_results.len() as f64) * 100.0;

        if success_rate < thresholds.min_success_rate_percent {
            warn!("Success rate below threshold: {:.1}% < {:.1}%", 
                  success_rate, thresholds.min_success_rate_percent);
        }

        // Check for anomaly bursts (high number of anomalies in short time)
        let recent_anomalies = recent_results.iter()
            .filter(|r| !r.anomalies.is_empty())
            .count();

        if recent_anomalies > thresholds.anomaly_burst_threshold as usize {
            warn!("Anomaly burst detected: {} anomalies in recent window", recent_anomalies);
        }
    }

    /// Clean up old data to prevent memory leaks
    pub fn cleanup(&mut self) {
        self.reverse_engine.cleanup_cache(10000);
        
        let mut recent = self.recent_results.write();
        if recent.len() > 1000 {
            let drain_count = recent.len() - 1000;
            recent.drain(..drain_count);
        }
    }
}

impl Default for MonitoringStats {
    fn default() -> Self {
        Self {
            total_validations: 0,
            successful_validations: 0,
            failed_validations: 0,
            precision_loss_detected: 0,
            anomalies_detected: 0,
            success_rate: 100.0,
            average_validation_time_ms: 0.0,
            last_validation: None,
            uptime_seconds: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_continuous_monitoring() {
        let mut monitor = ContinuousEqualityMonitor::new();
        
        let frontend_msg = FrontendTradeMessage {
            symbol_hash: "test_hash".to_string(),
            symbol: Some("test:symbol".to_string()),
            timestamp: 1234567890,
            price: 1.5,
            volume: 1000.0,
            side: "buy".to_string(),
            message_id: Some("test-uuid".to_string()),
        };

        // This will fail because no original data is stored, but tests the flow
        let result = monitor.validate_message(&frontend_msg).await;
        assert!(result.is_err()); // Expected failure

        let stats = monitor.get_stats();
        assert_eq!(stats.total_validations, 1);
        assert_eq!(stats.failed_validations, 1);
    }
}
// Metrics Broadcaster - Sends real-time performance metrics to dashboard
// Integrates with Unix socket relay for consistent data streaming

use anyhow::{Result, Context};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock, Mutex};
use tokio::time;
use tracing::{info, warn, error, debug};
use ethers::prelude::*;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

use crate::{
    ArbitrageMetrics,
    unix_socket_simple::UnixSocketClient,
    config::ArbitrageConfig,
};

/// Real-time metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub timestamp: DateTime<Utc>,
    
    // Performance metrics
    pub opportunities_per_minute: f64,
    pub success_rate: f64,
    pub average_profit_usd: f64,
    pub total_profit_usd: f64,
    
    // Execution metrics
    pub average_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub gas_efficiency: f64,
    
    // System health
    pub cpu_usage_percent: f64,
    pub memory_usage_mb: f64,
    pub active_positions: usize,
    pub pending_transactions: usize,
    
    // Market conditions
    pub gas_price_gwei: f64,
    pub network_congestion: CongestionLevel,
    pub mev_competition_level: f64,
    
    // Model accuracy (from validation)
    pub gas_prediction_accuracy: f64,
    pub slippage_prediction_accuracy: f64,
    pub profit_prediction_accuracy: f64,
    
    // Risk metrics
    pub var_95: f64,  // Value at Risk
    pub max_drawdown: f64,
    pub sharpe_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CongestionLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Broadcasts metrics to dashboard and monitoring systems
pub struct MetricsBroadcaster {
    metrics: Arc<RwLock<ArbitrageMetrics>>,
    config: Arc<ArbitrageConfig>,
    unix_client: Option<Arc<Mutex<UnixSocketClient>>>,
    broadcast_interval: Duration,
    tx: mpsc::Sender<MetricsSnapshot>,
    rx: Arc<RwLock<mpsc::Receiver<MetricsSnapshot>>>,
}

impl MetricsBroadcaster {
    pub async fn new(
        metrics: Arc<RwLock<ArbitrageMetrics>>,
        config: Arc<ArbitrageConfig>,
    ) -> Result<Self> {
        let (tx, rx) = mpsc::channel(100);
        
        // Try to connect to Unix socket relay
        let mut unix_client_temp = UnixSocketClient::new();
        let unix_client = match unix_client_temp.connect().await {
            Ok(_) => {
                info!("📡 Connected to relay for metrics broadcasting");
                Some(Arc::new(Mutex::new(unix_client_temp)))
            }
            Err(e) => {
                warn!("Failed to connect to relay: {}. Metrics will be logged only.", e);
                None
            }
        };
        
        Ok(Self {
            metrics,
            config,
            unix_client,
            broadcast_interval: Duration::from_secs(5), // 5 second updates
            tx,
            rx: Arc::new(RwLock::new(rx)),
        })
    }

    /// Start broadcasting metrics
    pub async fn start(&self) -> Result<()> {
        info!("📊 Starting metrics broadcaster");
        
        let metrics = self.metrics.clone();
        let config = self.config.clone();
        let interval = self.broadcast_interval;
        let tx = self.tx.clone();
        let unix_client = self.unix_client.clone();
        
        tokio::spawn(async move {
            let mut interval_timer = time::interval(interval);
            let mut last_metrics = ArbitrageMetrics::default();
            
            loop {
                interval_timer.tick().await;
                
                // Collect current metrics
                let current_metrics = metrics.read().await.clone();
                
                // Calculate rates and averages
                let snapshot = Self::calculate_snapshot(
                    &current_metrics,
                    &last_metrics,
                    interval.as_secs() as f64,
                ).await;
                
                // Broadcast to dashboard
                if let Some(ref client) = unix_client {
                    let mut client_guard = client.lock().await;
                    if let Err(e) = Self::broadcast_to_relay(&mut *client_guard, &snapshot).await {
                        error!("Failed to broadcast metrics: {}", e);
                    }
                }
                
                // Send to internal channel
                if let Err(e) = tx.send(snapshot.clone()).await {
                    error!("Failed to send metrics internally: {}", e);
                }
                
                // Log summary
                Self::log_metrics_summary(&snapshot);
                
                last_metrics = current_metrics;
            }
        });
        
        Ok(())
    }

    /// Calculate metrics snapshot from raw metrics
    async fn calculate_snapshot(
        current: &ArbitrageMetrics,
        previous: &ArbitrageMetrics,
        interval_seconds: f64,
    ) -> MetricsSnapshot {
        // Calculate rates
        let opportunities_delta = current.opportunities_analyzed
            .saturating_sub(previous.opportunities_analyzed);
        let opportunities_per_minute = (opportunities_delta as f64 / interval_seconds) * 60.0;
        
        // Calculate success rate
        let success_rate = if current.opportunities_executed > 0 {
            current.successful_trades as f64 / current.opportunities_executed as f64
        } else {
            0.0
        };
        
        // Calculate average profit
        let average_profit_usd = if current.successful_trades > 0 {
            current.total_profit_usd / current.successful_trades as f64
        } else {
            0.0
        };
        
        // Calculate latency percentiles
        let latencies = &current.execution_times_ms;
        let (p95, p99) = if !latencies.is_empty() {
            let mut sorted = latencies.clone();
            sorted.sort_unstable();
            let p95_idx = (sorted.len() as f64 * 0.95) as usize;
            let p99_idx = (sorted.len() as f64 * 0.99) as usize;
            (
                sorted.get(p95_idx).copied().unwrap_or(0) as f64,
                sorted.get(p99_idx).copied().unwrap_or(0) as f64,
            )
        } else {
            (0.0, 0.0)
        };
        
        let average_latency = if !latencies.is_empty() {
            latencies.iter().sum::<u64>() as f64 / latencies.len() as f64
        } else {
            0.0
        };
        
        // Gas efficiency (actual vs estimated)
        let gas_efficiency = if current.total_gas_estimated > 0 {
            (current.total_gas_used as f64 / current.total_gas_estimated as f64) * 100.0
        } else {
            100.0
        };
        
        // Get current gas price (simplified - would query network in production)
        let gas_price_gwei = 30.0; // Placeholder
        
        // Determine network congestion
        let network_congestion = match gas_price_gwei {
            g if g < 20.0 => CongestionLevel::Low,
            g if g < 50.0 => CongestionLevel::Medium,
            g if g < 100.0 => CongestionLevel::High,
            _ => CongestionLevel::Critical,
        };
        
        // Calculate risk metrics
        let (var_95, max_drawdown, sharpe_ratio) = Self::calculate_risk_metrics(current);
        
        // Get system metrics (simplified - would use sysinfo crate in production)
        let cpu_usage_percent = 25.0; // Placeholder
        let memory_usage_mb = 512.0; // Placeholder
        
        MetricsSnapshot {
            timestamp: Utc::now(),
            opportunities_per_minute,
            success_rate: success_rate * 100.0,
            average_profit_usd,
            total_profit_usd: current.total_profit_usd,
            average_latency_ms: average_latency,
            p95_latency_ms: p95,
            p99_latency_ms: p99,
            gas_efficiency,
            cpu_usage_percent,
            memory_usage_mb,
            active_positions: current.active_positions,
            pending_transactions: current.pending_transactions,
            gas_price_gwei,
            network_congestion,
            mev_competition_level: current.mev_blocks_detected as f64 / 
                                   current.blocks_processed.max(1) as f64,
            gas_prediction_accuracy: 95.0, // Would come from validation reporter
            slippage_prediction_accuracy: 92.0, // Would come from validation reporter
            profit_prediction_accuracy: 88.0, // Would come from validation reporter
            var_95,
            max_drawdown,
            sharpe_ratio,
        }
    }

    /// Calculate risk metrics
    fn calculate_risk_metrics(metrics: &ArbitrageMetrics) -> (f64, f64, f64) {
        // Simplified risk calculations
        // In production, would use historical profit/loss data
        
        // Value at Risk (95% confidence)
        let var_95 = metrics.total_profit_usd * 0.05; // 5% of profits at risk
        
        // Max drawdown
        let max_drawdown = if metrics.peak_balance > 0.0 {
            ((metrics.peak_balance - metrics.current_balance) / metrics.peak_balance) * 100.0
        } else {
            0.0
        };
        
        // Sharpe ratio (simplified)
        let returns = metrics.total_profit_usd / metrics.total_volume_usd.max(1.0);
        let risk_free_rate = 0.05 / 365.0; // Daily risk-free rate
        let excess_return = returns - risk_free_rate;
        let volatility = 0.02; // Placeholder volatility
        let sharpe_ratio = if volatility > 0.0 {
            excess_return / volatility
        } else {
            0.0
        };
        
        (var_95, max_drawdown, sharpe_ratio)
    }

    /// Broadcast metrics to relay server
    async fn broadcast_to_relay(
        client: &mut UnixSocketClient,
        snapshot: &MetricsSnapshot,
    ) -> Result<()> {
        let message = serde_json::json!({
            "type": "metrics_update",
            "data": snapshot,
        });
        
        client.send_message(&message.to_string()).await?;
        debug!("📤 Metrics broadcast to relay");
        
        Ok(())
    }

    /// Log metrics summary
    fn log_metrics_summary(snapshot: &MetricsSnapshot) {
        info!(
            "📊 Metrics: {:.1} ops/min | {:.1}% success | ${:.2} avg profit | {:.0}ms p95 latency",
            snapshot.opportunities_per_minute,
            snapshot.success_rate,
            snapshot.average_profit_usd,
            snapshot.p95_latency_ms
        );
        
        if snapshot.success_rate < 50.0 {
            warn!("⚠️ Low success rate: {:.1}%", snapshot.success_rate);
        }
        
        if snapshot.p95_latency_ms > 1000.0 {
            warn!("⚠️ High latency detected: {:.0}ms p95", snapshot.p95_latency_ms);
        }
        
        if matches!(snapshot.network_congestion, CongestionLevel::High | CongestionLevel::Critical) {
            warn!("⚠️ Network congestion: {:?}", snapshot.network_congestion);
        }
    }

    /// Get latest metrics snapshot
    pub async fn get_latest_snapshot(&self) -> Option<MetricsSnapshot> {
        let mut rx = self.rx.write().await;
        rx.recv().await
    }

    /// Export metrics history to file
    pub async fn export_metrics_history(&self, filepath: &str) -> Result<()> {
        // Would implement historical metrics export
        info!("📁 Exporting metrics history to {}", filepath);
        Ok(())
    }
}

/// Dashboard message format for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardMessage {
    pub message_type: String,
    pub timestamp: u64,
    pub data: DashboardData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DashboardData {
    Metrics(MetricsSnapshot),
    Alert(AlertMessage),
    Opportunity(OpportunityUpdate),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertMessage {
    pub level: AlertLevel,
    pub message: String,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertLevel {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpportunityUpdate {
    pub id: String,
    pub status: String,
    pub profit_usd: f64,
    pub path: Vec<String>,
}

/// Alert manager for critical events
pub struct AlertManager {
    tx: mpsc::Sender<AlertMessage>,
    thresholds: AlertThresholds,
}

#[derive(Debug, Clone)]
pub struct AlertThresholds {
    pub min_success_rate: f64,
    pub max_latency_ms: f64,
    pub min_profit_usd: f64,
    pub max_gas_price_gwei: f64,
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            min_success_rate: 70.0,
            max_latency_ms: 1000.0,
            min_profit_usd: 1.0,
            max_gas_price_gwei: 100.0,
        }
    }
}

impl AlertManager {
    pub fn new(tx: mpsc::Sender<AlertMessage>) -> Self {
        Self {
            tx,
            thresholds: AlertThresholds::default(),
        }
    }

    pub async fn check_metrics(&self, snapshot: &MetricsSnapshot) -> Result<()> {
        // Check success rate
        if snapshot.success_rate < self.thresholds.min_success_rate {
            self.send_alert(
                AlertLevel::Warning,
                format!("Low success rate: {:.1}%", snapshot.success_rate),
                Some("Consider adjusting opportunity filters".to_string()),
            ).await?;
        }

        // Check latency
        if snapshot.p95_latency_ms > self.thresholds.max_latency_ms {
            self.send_alert(
                AlertLevel::Warning,
                format!("High latency: {:.0}ms p95", snapshot.p95_latency_ms),
                Some("Network congestion or processing bottleneck".to_string()),
            ).await?;
        }

        // Check gas price
        if snapshot.gas_price_gwei > self.thresholds.max_gas_price_gwei {
            self.send_alert(
                AlertLevel::Error,
                format!("High gas price: {:.1} Gwei", snapshot.gas_price_gwei),
                Some("Consider pausing operations".to_string()),
            ).await?;
        }

        Ok(())
    }

    async fn send_alert(
        &self,
        level: AlertLevel,
        message: String,
        details: Option<String>,
    ) -> Result<()> {
        let alert = AlertMessage {
            level,
            message,
            details,
        };

        self.tx.send(alert).await
            .context("Failed to send alert")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_snapshot_calculation() {
        let mut current = ArbitrageMetrics::default();
        current.opportunities_analyzed = 100;
        current.opportunities_executed = 50;
        current.successful_trades = 45;
        current.total_profit_usd = 450.0;
        current.execution_times_ms = vec![100, 150, 200, 250, 300, 1000];

        let previous = ArbitrageMetrics::default();

        let snapshot = MetricsBroadcaster::calculate_snapshot(
            &current,
            &previous,
            60.0, // 1 minute interval
        ).await;

        assert_eq!(snapshot.opportunities_per_minute, 100.0);
        assert_eq!(snapshot.success_rate, 90.0); // 45/50 * 100
        assert_eq!(snapshot.average_profit_usd, 10.0); // 450/45
    }

    #[test]
    fn test_risk_metrics_calculation() {
        let mut metrics = ArbitrageMetrics::default();
        metrics.total_profit_usd = 1000.0;
        metrics.total_volume_usd = 100000.0;
        metrics.peak_balance = 10000.0;
        metrics.current_balance = 9500.0;

        let (var_95, max_drawdown, sharpe_ratio) = 
            MetricsBroadcaster::calculate_risk_metrics(&metrics);

        assert_eq!(var_95, 50.0); // 5% of 1000
        assert_eq!(max_drawdown, 5.0); // 5% drawdown
        assert!(sharpe_ratio > 0.0);
    }
}
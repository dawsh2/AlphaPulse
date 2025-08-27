//! Health Monitoring for TraceCollector
//!
//! Monitors and reports the health of the TraceCollector service.

use crate::{Result, TraceCollectorStats};
use alphapulse_types::{SourceType, SystemHealthTLV};
use parking_lot::RwLock;
use std::sync::Arc;
use sysinfo::{Pid, System};
use tracing::{error, info, warn};

/// Health reporter for TraceCollector service
#[derive(Clone)]
pub struct HealthReporter {
    /// Reference to collector statistics
    stats: Arc<RwLock<TraceCollectorStats>>,

    /// System information gatherer
    system: Arc<RwLock<System>>,

    /// Process ID for monitoring
    pid: u32,
}

/// Overall health status of TraceCollector
#[derive(Debug, Clone, serde::Serialize)]
pub struct CollectorHealth {
    /// Overall health status
    pub status: HealthStatus,

    /// CPU usage percentage
    pub cpu_usage: f32,

    /// Memory usage in MB
    pub memory_usage_mb: u64,

    /// Number of active Unix socket connections
    pub connection_count: usize,

    /// Events processed per second
    pub events_per_second: f64,

    /// Number of active traces
    pub active_traces: usize,

    /// Number of completed traces
    pub completed_traces: usize,

    /// Uptime in seconds
    pub uptime_seconds: u64,

    /// Any health warnings or errors
    pub warnings: Vec<String>,

    /// Last health check timestamp
    pub last_check_ns: u64,
}

/// Health status levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

impl From<HealthStatus> for u8 {
    fn from(status: HealthStatus) -> u8 {
        match status {
            HealthStatus::Healthy => SystemHealthTLV::HEALTH_OK,
            HealthStatus::Degraded => SystemHealthTLV::HEALTH_DEGRADED,
            HealthStatus::Unhealthy => SystemHealthTLV::HEALTH_UNHEALTHY,
            HealthStatus::Unknown => SystemHealthTLV::HEALTH_UNKNOWN,
        }
    }
}

impl HealthReporter {
    /// Create new health reporter
    pub fn new(stats: Arc<RwLock<TraceCollectorStats>>) -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        let pid = std::process::id();

        Self {
            stats,
            system: Arc::new(RwLock::new(system)),
            pid,
        }
    }

    /// Perform health check and return status
    pub async fn check_health(&self) -> Result<CollectorHealth> {
        // Refresh system information
        {
            let mut system = self.system.write();
            system.refresh_all();
        }

        let stats = self.stats.read().clone();
        let (cpu_usage, memory_usage_mb) = self.get_process_metrics();

        let mut warnings = Vec::new();

        // Determine overall health status
        let status = self.assess_health_status(&stats, cpu_usage, memory_usage_mb, &mut warnings);

        Ok(CollectorHealth {
            status,
            cpu_usage,
            memory_usage_mb,
            connection_count: 0, // TODO: Track connection count
            events_per_second: stats.events_per_second,
            active_traces: stats.active_traces,
            completed_traces: stats.completed_traces,
            uptime_seconds: stats.uptime_seconds,
            warnings,
            last_check_ns: current_timestamp_ns(),
        })
    }

    /// Report health status to system health monitoring
    pub async fn report_health(&self) -> Result<()> {
        let health = self.check_health().await?;

        // Create SystemHealthTLV message
        let _health_tlv = SystemHealthTLV::new(
            SourceType::MetricsCollector, // TraceCollector reports as MetricsCollector
            health.status.into(),
            health.cpu_usage as u8,
            (health.memory_usage_mb * 100 / 1024) as u8, // Approximate memory percentage
            health.connection_count as u32,
            health.events_per_second as u32,
        );

        // TODO: Send to SystemRelay via Unix socket
        // For now, just log the health status

        match health.status {
            HealthStatus::Healthy => {
                info!(
                    "TraceCollector health: OK (CPU: {:.1}%, Memory: {}MB, Events/sec: {:.1}, Active traces: {})",
                    health.cpu_usage,
                    health.memory_usage_mb,
                    health.events_per_second,
                    health.active_traces
                );
            }
            HealthStatus::Degraded => {
                warn!(
                    "TraceCollector health: DEGRADED (CPU: {:.1}%, Memory: {}MB, Warnings: {:?})",
                    health.cpu_usage, health.memory_usage_mb, health.warnings
                );
            }
            HealthStatus::Unhealthy => {
                error!(
                    "TraceCollector health: UNHEALTHY (CPU: {:.1}%, Memory: {}MB, Warnings: {:?})",
                    health.cpu_usage, health.memory_usage_mb, health.warnings
                );
            }
            HealthStatus::Unknown => {
                warn!("TraceCollector health: UNKNOWN");
            }
        }

        Ok(())
    }

    /// Get CPU and memory metrics for our process
    fn get_process_metrics(&self) -> (f32, u64) {
        let system = self.system.read();

        if let Some(process) = system.process(Pid::from(self.pid as usize)) {
            let cpu_usage = process.cpu_usage();
            let memory_kb = process.memory();
            let memory_mb = memory_kb / 1024;

            (cpu_usage, memory_mb)
        } else {
            (0.0, 0)
        }
    }

    /// Assess overall health status based on metrics
    fn assess_health_status(
        &self,
        stats: &TraceCollectorStats,
        cpu_usage: f32,
        memory_usage_mb: u64,
        warnings: &mut Vec<String>,
    ) -> HealthStatus {
        let mut is_healthy = true;
        let mut is_degraded = false;

        // Check CPU usage
        if cpu_usage > 80.0 {
            warnings.push(format!("High CPU usage: {:.1}%", cpu_usage));
            is_healthy = false;
        } else if cpu_usage > 60.0 {
            warnings.push(format!("Elevated CPU usage: {:.1}%", cpu_usage));
            is_degraded = true;
        }

        // Check memory usage
        if memory_usage_mb > 1000 {
            warnings.push(format!("High memory usage: {}MB", memory_usage_mb));
            is_healthy = false;
        } else if memory_usage_mb > 500 {
            warnings.push(format!("Elevated memory usage: {}MB", memory_usage_mb));
            is_degraded = true;
        }

        // Check active traces count
        if stats.active_traces > 5000 {
            warnings.push(format!("Too many active traces: {}", stats.active_traces));
            is_healthy = false;
        } else if stats.active_traces > 1000 {
            warnings.push(format!(
                "High number of active traces: {}",
                stats.active_traces
            ));
            is_degraded = true;
        }

        // Check event processing rate
        if stats.events_per_second < 1.0 && stats.uptime_seconds > 60 {
            warnings.push("Low event processing rate".to_string());
            is_degraded = true;
        }

        // Check for high timeout rate
        if stats.timed_out_traces > 0 {
            let timeout_rate = stats.timed_out_traces as f64
                / (stats.active_traces + stats.completed_traces) as f64;
            if timeout_rate > 0.1 {
                warnings.push(format!("High timeout rate: {:.1}%", timeout_rate * 100.0));
                is_degraded = true;
            }
        }

        if !is_healthy {
            HealthStatus::Unhealthy
        } else if is_degraded {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        }
    }

    /// Get detailed system information
    pub fn get_system_info(&self) -> serde_json::Value {
        let system = self.system.read();

        serde_json::json!({
            "total_memory": system.total_memory(),
            "used_memory": system.used_memory(),
            "total_swap": system.total_swap(),
            "used_swap": system.used_swap(),
            "system_name": System::name().unwrap_or_default(),
            "kernel_version": System::kernel_version().unwrap_or_default(),
            "os_version": System::os_version().unwrap_or_default(),
            "host_name": System::host_name().unwrap_or_default(),
        })
    }

    /// Check if service is responding normally
    pub async fn is_responsive(&self) -> bool {
        // Simple responsiveness check
        let start = std::time::Instant::now();
        let _stats = self.stats.read();
        let duration = start.elapsed();

        // If getting stats takes more than 100ms, consider unresponsive
        duration.as_millis() < 100
    }

    /// Get performance recommendations based on current metrics
    pub async fn get_recommendations(&self) -> Vec<String> {
        let health = match self.check_health().await {
            Ok(health) => health,
            Err(_) => return vec!["Unable to assess health".to_string()],
        };

        let mut recommendations = Vec::new();

        if health.cpu_usage > 70.0 {
            recommendations.push(
                "Consider increasing CPU resources or optimizing event processing".to_string(),
            );
        }

        if health.memory_usage_mb > 800 {
            recommendations
                .push("Consider increasing memory or reducing trace buffer sizes".to_string());
        }

        if health.active_traces > 2000 {
            recommendations.push(
                "Consider reducing trace timeout duration or increasing cleanup frequency"
                    .to_string(),
            );
        }

        if health.events_per_second < 10.0 && health.uptime_seconds > 300 {
            recommendations
                .push("Low event rate - check if services are properly connected".to_string());
        }

        if recommendations.is_empty() {
            recommendations.push("System is performing well".to_string());
        }

        recommendations
    }
}

/// Get current timestamp in nanoseconds
fn current_timestamp_ns() -> u64 {
    alphapulse_network::time::safe_system_timestamp_ns()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check() {
        let stats = Arc::new(RwLock::new(TraceCollectorStats {
            events_processed: 1000,
            active_traces: 50,
            completed_traces: 100,
            avg_events_per_trace: 5.0,
            avg_trace_duration_ms: 25.0,
            timed_out_traces: 2,
            events_per_second: 10.0,
            memory_usage_bytes: 50_000_000,
            uptime_seconds: 300,
        }));

        let health_reporter = HealthReporter::new(stats);
        let health = health_reporter.check_health().await.unwrap();

        assert_eq!(health.status, HealthStatus::Healthy);
        assert_eq!(health.active_traces, 50);
        assert_eq!(health.completed_traces, 100);
        assert_eq!(health.uptime_seconds, 300);
    }

    #[tokio::test]
    async fn test_responsiveness_check() {
        let stats = Arc::new(RwLock::new(TraceCollectorStats {
            events_processed: 0,
            active_traces: 0,
            completed_traces: 0,
            avg_events_per_trace: 0.0,
            avg_trace_duration_ms: 0.0,
            timed_out_traces: 0,
            events_per_second: 0.0,
            memory_usage_bytes: 0,
            uptime_seconds: 0,
        }));

        let health_reporter = HealthReporter::new(stats);
        let is_responsive = health_reporter.is_responsive().await;

        assert!(is_responsive);
    }
}

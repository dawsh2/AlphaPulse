//! Composite sink patterns for multi-target message distribution
//!
//! CompositeSink provides three essential patterns for message distribution:
//! - **Fanout**: Send to all targets simultaneously
//! - **RoundRobin**: Distribute messages across targets evenly
//! - **Failover**: Primary target with fallback chain

use crate::{
    BatchResult, ConnectionHealth, ConnectionState, ExtendedSinkMetadata, Message, MessageSink,
    SinkError, SinkMetadata,
};
use async_trait::async_trait;
use futures_util::future;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

/// Patterns for composite message distribution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositePattern {
    /// Send to all targets simultaneously
    Fanout,
    /// Rotate between targets for load balancing
    RoundRobin,
    /// Primary with fallback targets
    Failover,
}

/// Composite sink for multi-target message distribution
#[derive(Debug)]
pub struct CompositeSink {
    /// Target sinks
    targets: Vec<Arc<dyn MessageSink>>,

    /// Distribution pattern
    pattern: CompositePattern,

    /// Round-robin index (for RoundRobin pattern)
    round_robin_index: AtomicUsize,

    /// Metrics
    messages_sent: AtomicU64,
    messages_failed: AtomicU64,
    fanout_partial_failures: AtomicU64,
    failover_switches: AtomicU64,
    last_successful_send: Arc<RwLock<Option<SystemTime>>>,

    /// Sink name
    name: String,

    /// Per-target health tracking
    target_health: Arc<RwLock<Vec<ConnectionHealth>>>,
}

impl CompositeSink {
    /// Create a new composite sink
    pub fn new(targets: Vec<Arc<dyn MessageSink>>, pattern: CompositePattern) -> Self {
        if targets.is_empty() {
            panic!("Composite sink requires at least one target");
        }

        let target_count = targets.len();
        let name = format!("{}-composite-{}", pattern.name(), target_count);

        Self {
            targets,
            pattern,
            round_robin_index: AtomicUsize::new(0),
            messages_sent: AtomicU64::new(0),
            messages_failed: AtomicU64::new(0),
            fanout_partial_failures: AtomicU64::new(0),
            failover_switches: AtomicU64::new(0),
            last_successful_send: Arc::new(RwLock::new(None)),
            name,
            target_health: Arc::new(RwLock::new(vec![ConnectionHealth::Unknown; target_count])),
        }
    }

    /// Create fanout composite sink (send to all targets)
    pub fn fanout(targets: Vec<Arc<dyn MessageSink>>) -> Self {
        Self::new(targets, CompositePattern::Fanout)
    }

    /// Create round-robin composite sink (distribute across targets)
    pub fn round_robin(targets: Vec<Arc<dyn MessageSink>>) -> Self {
        Self::new(targets, CompositePattern::RoundRobin)
    }

    /// Create failover composite sink (primary with fallbacks)
    pub fn failover(targets: Vec<Arc<dyn MessageSink>>) -> Self {
        Self::new(targets, CompositePattern::Failover)
    }

    /// Get pattern
    pub fn pattern(&self) -> CompositePattern {
        self.pattern
    }

    /// Get target count
    pub fn target_count(&self) -> usize {
        self.targets.len()
    }

    /// Get current round-robin index
    pub fn current_round_robin_index(&self) -> usize {
        self.round_robin_index.load(Ordering::Relaxed)
    }

    /// Update target health status
    async fn update_target_health(&self) {
        let mut health = self.target_health.write().await;
        for (i, target) in self.targets.iter().enumerate() {
            health[i] = target.connection_health();
        }
    }

    /// Send message using fanout pattern
    async fn send_fanout(&self, message: Message) -> Result<(), SinkError> {
        let mut results = Vec::new();
        let mut successful_sends = 0;

        // Send to all targets concurrently
        let send_futures: Vec<_> = self
            .targets
            .iter()
            .map(|target| target.send(message.clone()))
            .collect();

        let send_results = future::join_all(send_futures).await;

        // Collect results
        for (i, result) in send_results.into_iter().enumerate() {
            let is_success = result.is_ok();
            results.push((i, result));
            if is_success {
                successful_sends += 1;
            }
        }

        if successful_sends == 0 {
            // All targets failed
            self.messages_failed.fetch_add(1, Ordering::Relaxed);
            let errors: Vec<String> = results
                .iter()
                .filter_map(|(i, r)| r.as_ref().err().map(|e| format!("target[{}]: {}", i, e)))
                .collect();
            Err(SinkError::send_failed(format!(
                "Fanout failed on all targets: {}",
                errors.join(", ")
            )))
        } else if successful_sends < self.targets.len() {
            // Partial success
            self.messages_sent.fetch_add(1, Ordering::Relaxed);
            self.fanout_partial_failures.fetch_add(1, Ordering::Relaxed);

            // Log partial failures
            let failed_targets: Vec<String> = results
                .iter()
                .filter_map(|(i, r)| r.as_ref().err().map(|e| format!("target[{}]: {}", i, e)))
                .collect();
            tracing::warn!(
                "Fanout partial failure on {}/{} targets: {}",
                self.targets.len() - successful_sends,
                self.targets.len(),
                failed_targets.join(", ")
            );

            // Update last successful send
            {
                let mut last_send = self.last_successful_send.write().await;
                *last_send = Some(SystemTime::now());
            }

            Ok(())
        } else {
            // Complete success
            self.messages_sent.fetch_add(1, Ordering::Relaxed);

            // Update last successful send
            {
                let mut last_send = self.last_successful_send.write().await;
                *last_send = Some(SystemTime::now());
            }

            Ok(())
        }
    }

    /// Send message using round-robin pattern
    async fn send_round_robin(&self, message: Message) -> Result<(), SinkError> {
        if self.targets.is_empty() {
            return Err(SinkError::send_failed("No targets available"));
        }

        // Get next target index
        let index = self.round_robin_index.fetch_add(1, Ordering::Relaxed) % self.targets.len();
        let target = &self.targets[index];

        match target.send(message).await {
            Ok(()) => {
                self.messages_sent.fetch_add(1, Ordering::Relaxed);

                // Update last successful send
                {
                    let mut last_send = self.last_successful_send.write().await;
                    *last_send = Some(SystemTime::now());
                }

                Ok(())
            }
            Err(e) => {
                self.messages_failed.fetch_add(1, Ordering::Relaxed);
                Err(SinkError::send_failed(format!(
                    "Round-robin target[{}] failed: {}",
                    index, e
                )))
            }
        }
    }

    /// Send message using failover pattern
    async fn send_failover(&self, message: Message) -> Result<(), SinkError> {
        let mut last_error = None;

        // Try targets in order until one succeeds
        for (i, target) in self.targets.iter().enumerate() {
            match target.send(message.clone()).await {
                Ok(()) => {
                    self.messages_sent.fetch_add(1, Ordering::Relaxed);

                    // Track failover switches (if not using primary target)
                    if i > 0 {
                        self.failover_switches.fetch_add(1, Ordering::Relaxed);
                        tracing::info!("Failover switched to target[{}]", i);
                    }

                    // Update last successful send
                    {
                        let mut last_send = self.last_successful_send.write().await;
                        *last_send = Some(SystemTime::now());
                    }

                    return Ok(());
                }
                Err(e) => {
                    tracing::warn!("Failover target[{}] failed: {}", i, e);
                    last_error = Some(e);
                }
            }
        }

        // All targets failed
        self.messages_failed.fetch_add(1, Ordering::Relaxed);
        Err(last_error.unwrap_or_else(|| SinkError::send_failed("All failover targets failed")))
    }

    /// Get composite-specific metrics
    pub fn composite_metrics(&self) -> CompositeMetrics {
        CompositeMetrics {
            pattern: self.pattern,
            target_count: self.targets.len(),
            messages_sent: self.messages_sent.load(Ordering::Relaxed),
            messages_failed: self.messages_failed.load(Ordering::Relaxed),
            fanout_partial_failures: self.fanout_partial_failures.load(Ordering::Relaxed),
            failover_switches: self.failover_switches.load(Ordering::Relaxed),
            current_round_robin_index: self.round_robin_index.load(Ordering::Relaxed)
                % self.targets.len(),
        }
    }
}

/// Metrics specific to composite sinks
#[derive(Debug, Clone)]
pub struct CompositeMetrics {
    pub pattern: CompositePattern,
    pub target_count: usize,
    pub messages_sent: u64,
    pub messages_failed: u64,
    pub fanout_partial_failures: u64,
    pub failover_switches: u64,
    pub current_round_robin_index: usize,
}

impl CompositePattern {
    /// Get pattern name
    pub fn name(self) -> &'static str {
        match self {
            CompositePattern::Fanout => "fanout",
            CompositePattern::RoundRobin => "round-robin",
            CompositePattern::Failover => "failover",
        }
    }
}

#[async_trait]
impl MessageSink for CompositeSink {
    async fn send(&self, message: Message) -> Result<(), SinkError> {
        // Update target health before sending
        self.update_target_health().await;

        match self.pattern {
            CompositePattern::Fanout => self.send_fanout(message).await,
            CompositePattern::RoundRobin => self.send_round_robin(message).await,
            CompositePattern::Failover => self.send_failover(message).await,
        }
    }

    async fn send_batch(&self, messages: Vec<Message>) -> Result<BatchResult, SinkError> {
        let mut result = BatchResult::new(messages.len());

        for (index, message) in messages.into_iter().enumerate() {
            match self.send(message).await {
                Ok(()) => result.record_success(),
                Err(e) => result.record_failure(index, e),
            }
        }

        Ok(result)
    }

    fn is_connected(&self) -> bool {
        match self.pattern {
            CompositePattern::Fanout => {
                // At least one target must be connected
                self.targets.iter().any(|target| target.is_connected())
            }
            CompositePattern::RoundRobin => {
                // At least one target must be connected
                self.targets.iter().any(|target| target.is_connected())
            }
            CompositePattern::Failover => {
                // Primary target should be connected (for best performance)
                // But any target being connected means we can send
                self.targets.iter().any(|target| target.is_connected())
            }
        }
    }

    async fn connect(&self) -> Result<(), SinkError> {
        let mut errors = Vec::new();
        let mut connected_count = 0;

        // Try to connect all targets
        for (i, target) in self.targets.iter().enumerate() {
            match target.connect().await {
                Ok(()) => connected_count += 1,
                Err(e) => {
                    errors.push(format!("target[{}]: {}", i, e));
                }
            }
        }

        match self.pattern {
            CompositePattern::Fanout => {
                // Fanout needs all targets connected for optimal operation
                if connected_count == self.targets.len() {
                    Ok(())
                } else if connected_count > 0 {
                    tracing::warn!(
                        "Fanout partially connected: {}/{} targets",
                        connected_count,
                        self.targets.len()
                    );
                    Ok(()) // Partial connection is acceptable
                } else {
                    Err(SinkError::connection_failed(format!(
                        "No targets connected: {}",
                        errors.join(", ")
                    )))
                }
            }
            CompositePattern::RoundRobin | CompositePattern::Failover => {
                // Round-robin and failover need at least one target
                if connected_count > 0 {
                    Ok(())
                } else {
                    Err(SinkError::connection_failed(format!(
                        "No targets connected: {}",
                        errors.join(", ")
                    )))
                }
            }
        }
    }

    async fn disconnect(&self) -> Result<(), SinkError> {
        let mut errors = Vec::new();

        // Disconnect all targets
        for (i, target) in self.targets.iter().enumerate() {
            if let Err(e) = target.disconnect().await {
                errors.push(format!("target[{}]: {}", i, e));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(SinkError::send_failed(format!(
                "Disconnect errors: {}",
                errors.join(", ")
            )))
        }
    }

    fn metadata(&self) -> SinkMetadata {
        let messages_sent = self.messages_sent.load(Ordering::Relaxed);
        let messages_failed = self.messages_failed.load(Ordering::Relaxed);

        // Aggregate connection state from targets
        let connected_targets = self.targets.iter().filter(|t| t.is_connected()).count();
        let state = if connected_targets == self.targets.len() {
            ConnectionState::Connected
        } else if connected_targets > 0 {
            ConnectionState::Connecting // Partially connected
        } else {
            ConnectionState::Disconnected
        };

        SinkMetadata {
            name: self.name.clone(),
            sink_type: format!("composite-{}", self.pattern.name()),
            endpoint: Some(format!("composite://{}-targets", self.targets.len())),
            state,
            messages_sent,
            messages_failed,
            last_error: None,
        }
    }

    fn extended_metadata(&self) -> ExtendedSinkMetadata {
        let metadata = self.metadata();
        let last_send = {
            let last_send = self.last_successful_send.try_read().ok();
            last_send.and_then(|ls| *ls)
        };

        let total_messages = metadata.messages_sent + metadata.messages_failed;
        let error_rate = if total_messages > 0 {
            Some(metadata.messages_failed as f64 / total_messages as f64)
        } else {
            Some(0.0)
        };

        // Estimate latency based on pattern
        let avg_latency = match self.pattern {
            CompositePattern::Fanout => Some(100000), // Higher due to parallel sends
            CompositePattern::RoundRobin => Some(50000), // Single target latency
            CompositePattern::Failover => Some(75000), // May need retries
        };

        ExtendedSinkMetadata {
            metadata,
            health: self.connection_health(),
            last_successful_send: last_send,
            avg_latency_ns: avg_latency,
            error_rate,
            active_connections: self.targets.iter().filter(|t| t.is_connected()).count(),
            preferred_connections: self.targets.len(),
            supports_multiplexing: true, // Composite sinks inherently multiplex
        }
    }

    fn connection_health(&self) -> ConnectionHealth {
        let connected_targets = self.targets.iter().filter(|t| t.is_connected()).count();
        let healthy_targets = self
            .targets
            .iter()
            .filter(|t| t.connection_health() == ConnectionHealth::Healthy)
            .count();

        match self.pattern {
            CompositePattern::Fanout => {
                if healthy_targets == self.targets.len() {
                    ConnectionHealth::Healthy
                } else if healthy_targets > 0 {
                    ConnectionHealth::Degraded
                } else {
                    ConnectionHealth::Unhealthy
                }
            }
            CompositePattern::RoundRobin => {
                let healthy_ratio = healthy_targets as f64 / self.targets.len() as f64;
                if healthy_ratio >= 0.8 {
                    ConnectionHealth::Healthy
                } else if healthy_ratio >= 0.5 {
                    ConnectionHealth::Degraded
                } else if connected_targets > 0 {
                    ConnectionHealth::Degraded
                } else {
                    ConnectionHealth::Unhealthy
                }
            }
            CompositePattern::Failover => {
                // Primary target health is most important
                if let Some(primary) = self.targets.first() {
                    let primary_health = primary.connection_health();
                    if primary_health == ConnectionHealth::Healthy {
                        ConnectionHealth::Healthy
                    } else if connected_targets > 0 {
                        ConnectionHealth::Degraded
                    } else {
                        ConnectionHealth::Unhealthy
                    }
                } else {
                    ConnectionHealth::Unhealthy
                }
            }
        }
    }

    fn last_successful_send(&self) -> Option<SystemTime> {
        let last_send = self.last_successful_send.try_read().ok()?;
        *last_send
    }

    fn preferred_connection_count(&self) -> usize {
        self.targets.len()
    }

    fn supports_multiplexing(&self) -> bool {
        // Composite sinks support multiplexing if all targets do
        self.targets
            .iter()
            .all(|target| target.supports_multiplexing())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::CollectorSink;

    #[tokio::test]
    async fn test_fanout_creation() {
        let targets: Vec<Arc<dyn MessageSink>> = vec![
            Arc::new(CollectorSink::new()),
            Arc::new(CollectorSink::new()),
        ];

        let fanout = CompositeSink::fanout(targets);
        assert_eq!(fanout.pattern(), CompositePattern::Fanout);
        assert_eq!(fanout.target_count(), 2);
    }

    #[tokio::test]
    async fn test_round_robin_distribution() {
        let targets: Vec<Arc<dyn MessageSink>> = vec![
            Arc::new(CollectorSink::new()),
            Arc::new(CollectorSink::new()),
        ];

        // Force connect all targets
        for target in &targets {
            target.connect().await.unwrap();
        }

        let round_robin = CompositeSink::round_robin(targets.clone());

        // Send multiple messages
        for i in 0..4 {
            let message = Message::new_unchecked(format!("msg{}", i).as_bytes().to_vec());
            round_robin.send(message).await.unwrap();
        }

        // Check distribution
        let metrics = round_robin.composite_metrics();
        assert_eq!(metrics.messages_sent, 4);
        assert_eq!(metrics.current_round_robin_index, 0); // Should wrap around
    }

    #[tokio::test]
    async fn test_failover_pattern() {
        let targets: Vec<Arc<dyn MessageSink>> = vec![
            Arc::new(CollectorSink::new()), // Will fail (not connected)
            Arc::new(CollectorSink::new()), // Will succeed (connected)
        ];

        // Connect only second target
        targets[1].connect().await.unwrap();

        let failover = CompositeSink::failover(targets);

        let message = Message::new_unchecked(b"test".to_vec());
        failover.send(message).await.unwrap();

        let metrics = failover.composite_metrics();
        assert_eq!(metrics.messages_sent, 1);
        assert_eq!(metrics.failover_switches, 1); // Switched to second target
    }

    #[tokio::test]
    async fn test_composite_metadata() {
        let targets: Vec<Arc<dyn MessageSink>> = vec![Arc::new(CollectorSink::new())];

        let composite = CompositeSink::fanout(targets);

        let metadata = composite.metadata();
        assert_eq!(metadata.sink_type, "composite-fanout");
        assert_eq!(metadata.endpoint, Some("composite://1-targets".to_string()));

        let ext_metadata = composite.extended_metadata();
        assert!(ext_metadata.avg_latency_ns.is_some());
    }

    #[test]
    fn test_pattern_names() {
        assert_eq!(CompositePattern::Fanout.name(), "fanout");
        assert_eq!(CompositePattern::RoundRobin.name(), "round-robin");
        assert_eq!(CompositePattern::Failover.name(), "failover");
    }
}

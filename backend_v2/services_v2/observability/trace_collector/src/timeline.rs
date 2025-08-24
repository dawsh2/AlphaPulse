//! Trace Timeline Construction
//!
//! Builds complete message flow timelines from individual trace events.

use crate::{trace_id_to_hex, Result, TraceError, TraceId};
use protocol_v2::{SourceType, TraceEvent, TraceEventType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::time::Instant;

/// Complete trace timeline showing message flow through the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceTimeline {
    /// Unique trace identifier
    pub trace_id: TraceId,

    /// When this trace started
    #[serde(with = "instant_serde")]
    pub start_time: Instant,

    /// When this trace was last updated
    #[serde(with = "instant_serde")]
    pub last_update: Instant,

    /// Individual spans (hops) in the message flow
    pub spans: Vec<TraceSpan>,

    /// Overall message flow summary
    pub message_flow: MessageFlow,

    /// Total duration of the trace (nanoseconds)
    pub total_duration_ns: u64,

    /// Whether this trace is complete or still active
    pub is_complete: bool,

    /// Any errors that occurred during processing
    pub errors: Vec<TraceError>,

    /// Metadata extracted from events
    pub metadata: HashMap<String, String>,
}

/// Individual span (hop) in a trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSpan {
    /// Service that processed this span
    pub service: SourceType,

    /// Type of event (collected, sent, received, processed, etc.)
    pub event_type: TraceEventType,

    /// When this span occurred
    pub timestamp_ns: u64,

    /// Duration of processing (if available)
    pub duration_ns: Option<u64>,

    /// Span metadata
    pub metadata: HashMap<String, String>,
}

/// High-level message flow summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageFlow {
    /// Origin service (e.g., PolygonCollector)
    pub origin: SourceType,

    /// Services that processed this message
    pub services_traversed: Vec<SourceType>,

    /// Final destination (if known)
    pub destination: Option<SourceType>,

    /// Total hops in the message flow
    pub hop_count: usize,

    /// End-to-end latency in nanoseconds
    pub end_to_end_latency_ns: u64,

    /// Processing latencies by service (nanoseconds)
    pub service_latencies: HashMap<SourceType, u64>,

    /// Whether execution was triggered
    pub execution_triggered: bool,

    /// Whether any errors occurred
    pub has_errors: bool,
}

impl TraceTimeline {
    /// Create new timeline from first trace event
    pub fn new(initial_event: TraceEvent) -> Self {
        let start_time = Instant::now();
        let trace_id = initial_event.trace_id;

        let initial_span = TraceSpan {
            service: initial_event.service,
            event_type: initial_event.event_type,
            timestamp_ns: initial_event.timestamp_ns,
            duration_ns: initial_event.duration_ns,
            metadata: initial_event.metadata.clone(),
        };

        let message_flow = MessageFlow {
            origin: initial_event.service,
            services_traversed: vec![initial_event.service],
            destination: None,
            hop_count: 1,
            end_to_end_latency_ns: 0,
            service_latencies: HashMap::new(),
            execution_triggered: initial_event.event_type == TraceEventType::ExecutionTriggered,
            has_errors: initial_event.event_type == TraceEventType::ErrorOccurred,
        };

        let mut metadata = HashMap::new();
        // Extract common metadata from first event
        for (key, value) in &initial_event.metadata {
            metadata.insert(key.clone(), value.clone());
        }

        Self {
            trace_id,
            start_time,
            last_update: start_time,
            spans: vec![initial_span],
            message_flow,
            total_duration_ns: 0,
            is_complete: false,
            errors: Vec::new(),
            metadata,
        }
    }

    /// Add new event to this timeline
    pub fn add_event(&mut self, event: TraceEvent) -> Result<()> {
        if event.trace_id != self.trace_id {
            return Err(TraceError::Timeline(format!(
                "Event trace ID {} doesn't match timeline trace ID {}",
                trace_id_to_hex(&event.trace_id),
                trace_id_to_hex(&self.trace_id)
            )));
        }

        self.last_update = Instant::now();

        // Add span
        let span = TraceSpan {
            service: event.service,
            event_type: event.event_type,
            timestamp_ns: event.timestamp_ns,
            duration_ns: event.duration_ns,
            metadata: event.metadata.clone(),
        };

        self.spans.push(span);

        // Update message flow
        self.update_message_flow(&event);

        // Update overall metadata
        for (key, value) in &event.metadata {
            self.metadata.insert(key.clone(), value.clone());
        }

        // Calculate total duration from first to last event
        if let Some(first_span) = self.spans.first() {
            if let Some(last_span) = self.spans.last() {
                self.total_duration_ns = last_span
                    .timestamp_ns
                    .saturating_sub(first_span.timestamp_ns);
            }
        }

        Ok(())
    }

    /// Update message flow summary
    fn update_message_flow(&mut self, event: &TraceEvent) {
        // Add to services traversed if not already present
        if !self
            .message_flow
            .services_traversed
            .contains(&event.service)
        {
            self.message_flow.services_traversed.push(event.service);
        }

        // Update hop count
        self.message_flow.hop_count = self.spans.len();

        // Check for execution
        if event.event_type == TraceEventType::ExecutionTriggered {
            self.message_flow.execution_triggered = true;
            self.message_flow.destination = Some(event.service);
        }

        // Check for errors
        if event.event_type == TraceEventType::ErrorOccurred {
            self.message_flow.has_errors = true;
        }

        // Calculate service latencies
        if let Some(duration_ns) = event.duration_ns {
            *self
                .message_flow
                .service_latencies
                .entry(event.service)
                .or_insert(0) += duration_ns;
        }

        // Update end-to-end latency
        if let (Some(first_span), Some(last_span)) = (self.spans.first(), self.spans.last()) {
            self.message_flow.end_to_end_latency_ns = last_span
                .timestamp_ns
                .saturating_sub(first_span.timestamp_ns);
        }
    }

    /// Mark timeline as complete
    pub fn mark_complete(&mut self) {
        self.is_complete = true;
        self.last_update = Instant::now();
    }

    /// Get trace ID
    pub fn trace_id(&self) -> TraceId {
        self.trace_id
    }

    /// Get start time
    pub fn start_time(&self) -> Instant {
        self.start_time
    }

    /// Get trace ID as hex string
    pub fn trace_id_hex(&self) -> String {
        trace_id_to_hex(&self.trace_id)
    }

    /// Get human-readable summary
    pub fn summary(&self) -> String {
        format!(
            "Trace {} ({}): {} -> {} ({} hops, {:.2}ms, {})",
            self.trace_id_hex()[..8].to_string(),
            if self.is_complete {
                "complete"
            } else {
                "active"
            },
            format!("{:?}", self.message_flow.origin),
            match self.message_flow.destination {
                Some(dest) => format!("{:?}", dest),
                None => "ongoing".to_string(),
            },
            self.message_flow.hop_count,
            self.total_duration_ns as f64 / 1_000_000.0, // Convert to milliseconds
            if self.message_flow.has_errors {
                "errors"
            } else {
                "ok"
            }
        )
    }

    /// Get processing latency breakdown by service
    pub fn get_latency_breakdown(&self) -> Vec<(SourceType, u64)> {
        let mut latencies: Vec<_> = self
            .message_flow
            .service_latencies
            .iter()
            .map(|(&service, &latency)| (service, latency))
            .collect();
        latencies.sort_by_key(|(_, latency)| *latency);
        latencies.reverse(); // Highest latency first
        latencies
    }

    /// Find bottleneck service (highest processing latency)
    pub fn find_bottleneck(&self) -> Option<(SourceType, u64)> {
        self.message_flow
            .service_latencies
            .iter()
            .max_by_key(|(_, &latency)| latency)
            .map(|(&service, &latency)| (service, latency))
    }

    /// Check if trace shows healthy message flow
    pub fn is_healthy(&self) -> bool {
        // Consider healthy if:
        // - No errors occurred
        // - Reasonable end-to-end latency (< 100ms)
        // - Expected number of hops (2-5 typical)

        !self.message_flow.has_errors &&
        self.message_flow.end_to_end_latency_ns < 100_000_000 && // 100ms
        self.message_flow.hop_count >= 2 && self.message_flow.hop_count <= 8
    }

    /// Get spans for a specific service
    pub fn spans_for_service(&self, service: SourceType) -> Vec<&TraceSpan> {
        self.spans
            .iter()
            .filter(|span| span.service == service)
            .collect()
    }

    /// Get timeline as JSON for web interface
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(|e| TraceError::Json(e.to_string()))
    }
}

// Custom serde module for Instant serialization
mod instant_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{SystemTime, UNIX_EPOCH};
    use tokio::time::Instant;

    pub fn serialize<S>(_instant: &Instant, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Convert to timestamp for serialization
        // Note: This is approximate since Instant doesn't have a fixed epoch
        let now = SystemTime::now();
        let duration_since_epoch = now.duration_since(UNIX_EPOCH).unwrap_or_default();
        duration_since_epoch.as_nanos().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Instant, D::Error>
    where
        D: Deserializer<'de>,
    {
        let _nanos = u128::deserialize(deserializer)?;
        // For deserialization, just use current time
        // (this is mainly for JSON export, not storage)
        Ok(Instant::now())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol_v2::{SourceType, TraceEvent, TraceEventType};
    use std::collections::HashMap;

    fn create_test_event(
        trace_id: TraceId,
        service: SourceType,
        event_type: TraceEventType,
        timestamp_ns: u64,
    ) -> TraceEvent {
        TraceEvent {
            trace_id,
            service,
            event_type,
            timestamp_ns,
            duration_ns: Some(1_000_000), // 1ms
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn test_timeline_creation() {
        let trace_id = [1u8; 16];
        let event = create_test_event(
            trace_id,
            SourceType::PolygonCollector,
            TraceEventType::DataCollected,
            1000000000,
        );

        let timeline = TraceTimeline::new(event);

        assert_eq!(timeline.trace_id, trace_id);
        assert_eq!(timeline.spans.len(), 1);
        assert_eq!(timeline.message_flow.origin, SourceType::PolygonCollector);
        assert_eq!(timeline.message_flow.hop_count, 1);
        assert!(!timeline.is_complete);
    }

    #[test]
    fn test_timeline_event_addition() {
        let trace_id = [2u8; 16];
        let first_event = create_test_event(
            trace_id,
            SourceType::PolygonCollector,
            TraceEventType::DataCollected,
            1000000000,
        );

        let mut timeline = TraceTimeline::new(first_event);

        let second_event = create_test_event(
            trace_id,
            SourceType::MarketDataRelay,
            TraceEventType::MessageReceived,
            1001000000, // 1ms later
        );

        timeline.add_event(second_event).unwrap();

        assert_eq!(timeline.spans.len(), 2);
        assert_eq!(timeline.message_flow.hop_count, 2);
        assert_eq!(timeline.message_flow.services_traversed.len(), 2);
        assert_eq!(timeline.total_duration_ns, 1000000); // 1ms
    }

    #[test]
    fn test_timeline_completion() {
        let trace_id = [3u8; 16];
        let event = create_test_event(
            trace_id,
            SourceType::ArbitrageStrategy,
            TraceEventType::ExecutionTriggered,
            1000000000,
        );

        let mut timeline = TraceTimeline::new(event);
        timeline.mark_complete();

        assert!(timeline.is_complete);
        assert!(timeline.message_flow.execution_triggered);
        assert_eq!(
            timeline.message_flow.destination,
            Some(SourceType::ArbitrageStrategy)
        );
    }

    #[test]
    fn test_latency_breakdown() {
        let trace_id = [4u8; 16];
        let mut timeline = TraceTimeline::new(create_test_event(
            trace_id,
            SourceType::PolygonCollector,
            TraceEventType::DataCollected,
            1000000000,
        ));

        // Add event with longer duration
        let mut slow_event = create_test_event(
            trace_id,
            SourceType::MarketDataRelay,
            TraceEventType::MessageProcessed,
            1002000000,
        );
        slow_event.duration_ns = Some(5_000_000); // 5ms
        timeline.add_event(slow_event).unwrap();

        let breakdown = timeline.get_latency_breakdown();
        assert_eq!(breakdown.len(), 2);

        // Should be sorted by latency (highest first)
        assert!(breakdown[0].1 >= breakdown[1].1);

        let bottleneck = timeline.find_bottleneck().unwrap();
        assert_eq!(bottleneck.0, SourceType::MarketDataRelay);
        assert_eq!(bottleneck.1, 5_000_000);
    }
}

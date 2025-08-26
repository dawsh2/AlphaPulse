//! Trace Event Processing
//!
//! Handles incoming trace events and updates trace timelines.

use crate::{trace_id_to_hex, Result, TraceCollectorStats, TraceError, TraceId, TraceTimeline};
use dashmap::DashMap;
use parking_lot::RwLock;
use alphapulse_types::TraceEvent;
use ringbuffer::{AllocRingBuffer, RingBuffer};
use std::sync::Arc;
use tokio::time::Instant;
use tracing::{debug, warn};

/// Processes trace events and maintains trace timelines
#[derive(Clone)]
pub struct TraceEventProcessor {
    /// Active traces being built
    active_traces: Arc<DashMap<TraceId, TraceTimeline>>,

    /// Completed traces ring buffer
    completed_traces: Arc<RwLock<AllocRingBuffer<TraceTimeline>>>,

    /// Performance statistics
    stats: Arc<RwLock<TraceCollectorStats>>,

    /// Event buffer for rate limiting (optional)
    event_buffer: Arc<RwLock<EventBuffer>>,
}

/// Buffer for managing event processing rate
pub struct EventBuffer {
    /// Events processed in the last minute (for rate calculation)
    recent_events: Vec<Instant>,

    /// Total events processed since startup
    total_events: u64,

    /// Events that caused errors
    error_events: u64,
}

impl EventBuffer {
    fn new() -> Self {
        Self {
            recent_events: Vec::new(),
            total_events: 0,
            error_events: 0,
        }
    }

    /// Record new event and update rate statistics
    fn record_event(&mut self, success: bool) {
        let now = Instant::now();
        self.recent_events.push(now);
        self.total_events += 1;

        if !success {
            self.error_events += 1;
        }

        // Keep only events from the last minute for rate calculation
        let one_minute_ago = now - tokio::time::Duration::from_secs(60);
        self.recent_events
            .retain(|&timestamp| timestamp > one_minute_ago);
    }

    /// Get events per second rate
    fn get_rate(&self) -> f64 {
        self.recent_events.len() as f64 / 60.0
    }

    /// Get error rate as percentage
    fn get_error_rate(&self) -> f64 {
        if self.total_events == 0 {
            0.0
        } else {
            (self.error_events as f64 / self.total_events as f64) * 100.0
        }
    }
}

impl TraceEventProcessor {
    /// Create new event processor
    pub fn new(
        active_traces: Arc<DashMap<TraceId, TraceTimeline>>,
        completed_traces: Arc<RwLock<AllocRingBuffer<TraceTimeline>>>,
        stats: Arc<RwLock<TraceCollectorStats>>,
    ) -> Self {
        Self {
            active_traces,
            completed_traces,
            stats,
            event_buffer: Arc::new(RwLock::new(EventBuffer::new())),
        }
    }

    /// Process a single trace event
    pub async fn process_event(&self, event: TraceEvent) -> Result<()> {
        let trace_id = event.trace_id;
        let event_type = event.event_type;
        let service = event.service;

        debug!(
            "Processing event: {} from {:?} (type: {:?})",
            trace_id_to_hex(&trace_id),
            service,
            event_type
        );

        let success = match self.process_event_internal(event).await {
            Ok(()) => {
                debug!(
                    "Successfully processed event for trace {}",
                    trace_id_to_hex(&trace_id)
                );
                true
            }
            Err(e) => {
                warn!(
                    "Failed to process event for trace {}: {}",
                    trace_id_to_hex(&trace_id),
                    e
                );
                false
            }
        };

        // Update event buffer statistics
        {
            let mut buffer = self.event_buffer.write();
            buffer.record_event(success);

            // Update global statistics
            let mut stats = self.stats.write();
            stats.events_processed = buffer.total_events;
            stats.events_per_second = buffer.get_rate();
            stats.active_traces = self.active_traces.len();
            stats.completed_traces = self.completed_traces.read().len();
        }

        if success {
            Ok(())
        } else {
            Err(TraceError::Timeline("Event processing failed".to_string()))
        }
    }

    /// Internal event processing logic
    async fn process_event_internal(&self, event: TraceEvent) -> Result<()> {
        let trace_id = event.trace_id;

        // Check if this is a new trace or continuation
        if let Some(mut timeline_entry) = self.active_traces.get_mut(&trace_id) {
            // Add event to existing timeline
            timeline_entry.add_event(event)?;

            // Check if trace should be considered complete
            if self.is_trace_complete(&timeline_entry) {
                // Move to completed traces
                let trace_id = timeline_entry.trace_id();
                drop(timeline_entry); // Release the lock

                if let Some((_, mut timeline)) = self.active_traces.remove(&trace_id) {
                    timeline.mark_complete();

                    debug!("Trace completed: {}", timeline.summary());

                    // Add to completed traces ring buffer
                    {
                        let mut completed = self.completed_traces.write();
                        completed.push(timeline);
                    }
                }
            }
        } else {
            // Create new timeline for this trace
            let timeline = TraceTimeline::new(event);
            debug!("New trace started: {}", timeline.summary());

            self.active_traces.insert(trace_id, timeline);
        }

        Ok(())
    }

    /// Determine if a trace should be considered complete
    fn is_trace_complete(&self, timeline: &TraceTimeline) -> bool {
        // Consider trace complete if:
        // 1. Execution was triggered (final step)
        // 2. An error occurred (trace ends)
        // 3. Too many hops (probably stuck in loop)

        timeline.message_flow.execution_triggered
            || timeline.message_flow.has_errors
            || timeline.message_flow.hop_count > 10 // Reasonable max hops
    }

    /// Get processing statistics
    pub fn get_stats(&self) -> (u64, f64, f64) {
        let buffer = self.event_buffer.read();
        (
            buffer.total_events,
            buffer.get_rate(),
            buffer.get_error_rate(),
        )
    }

    /// Process batch of events for efficiency
    pub async fn process_batch(&self, events: Vec<TraceEvent>) -> Result<Vec<Result<()>>> {
        let mut results = Vec::with_capacity(events.len());

        for event in events {
            let result = self.process_event(event).await;
            results.push(result);
        }

        Ok(results)
    }

    /// Get active trace count
    pub fn active_count(&self) -> usize {
        self.active_traces.len()
    }

    /// Get completed trace count
    pub fn completed_count(&self) -> usize {
        self.completed_traces.read().len()
    }

    /// Get trace by ID (active only)
    pub fn get_active_trace(&self, trace_id: &TraceId) -> Option<TraceTimeline> {
        self.active_traces.get(trace_id).map(|t| t.clone())
    }

    /// Get all active trace IDs
    pub fn get_active_trace_ids(&self) -> Vec<TraceId> {
        self.active_traces
            .iter()
            .map(|entry| *entry.key())
            .collect()
    }

    /// Force complete a trace (for debugging/testing)
    pub fn force_complete_trace(&self, trace_id: &TraceId) -> Result<()> {
        if let Some((_, mut timeline)) = self.active_traces.remove(trace_id) {
            timeline.mark_complete();

            let mut completed = self.completed_traces.write();
            completed.push(timeline);

            Ok(())
        } else {
            Err(TraceError::InvalidTraceId(format!(
                "Active trace not found: {}",
                trace_id_to_hex(trace_id)
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alphapulse_types::{SourceType, TraceEvent, TraceEventType};
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
            duration_ns: Some(1_000_000),
            metadata: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_event_processor_new_trace() {
        let active_traces = Arc::new(DashMap::new());
        let completed_traces = Arc::new(RwLock::new(AllocRingBuffer::new(100)));
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

        let processor = TraceEventProcessor::new(active_traces.clone(), completed_traces, stats);

        let trace_id = [1u8; 16];
        let event = create_test_event(
            trace_id,
            SourceType::PolygonCollector,
            TraceEventType::DataCollected,
            1000000000,
        );

        processor.process_event(event).await.unwrap();

        assert_eq!(active_traces.len(), 1);
        assert!(active_traces.contains_key(&trace_id));
    }

    #[tokio::test]
    async fn test_event_processor_trace_completion() {
        let active_traces = Arc::new(DashMap::new());
        let completed_traces = Arc::new(RwLock::new(AllocRingBuffer::new(100)));
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

        let processor =
            TraceEventProcessor::new(active_traces.clone(), completed_traces.clone(), stats);

        let trace_id = [2u8; 16];

        // Start trace
        let start_event = create_test_event(
            trace_id,
            SourceType::PolygonCollector,
            TraceEventType::DataCollected,
            1000000000,
        );
        processor.process_event(start_event).await.unwrap();

        // Complete trace with execution
        let complete_event = create_test_event(
            trace_id,
            SourceType::ArbitrageStrategy,
            TraceEventType::ExecutionTriggered,
            1001000000,
        );
        processor.process_event(complete_event).await.unwrap();

        // Should be moved to completed traces
        assert_eq!(active_traces.len(), 0);
        assert_eq!(completed_traces.read().len(), 1);
    }

    #[tokio::test]
    async fn test_event_buffer_rate_calculation() {
        let mut buffer = EventBuffer::new();

        // Record some events
        for _ in 0..10 {
            buffer.record_event(true);
        }

        assert_eq!(buffer.total_events, 10);
        assert!(buffer.get_rate() > 0.0);
        assert_eq!(buffer.get_error_rate(), 0.0);

        // Record some errors
        for _ in 0..2 {
            buffer.record_event(false);
        }

        assert_eq!(buffer.total_events, 12);
        assert_eq!(buffer.error_events, 2);
        assert!((buffer.get_error_rate() - 16.67).abs() < 0.1); // ~16.67%
    }
}

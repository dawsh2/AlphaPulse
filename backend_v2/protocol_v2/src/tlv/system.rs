//! System TLV Structures - System Domain (Types 100-119)
//!
//! Defines TLV structures for system operations including:
//! - TraceContext: Distributed tracing for message flow observability
//! - SystemHealth: Component health monitoring
//! - ResourceUsage: Performance metrics
//!
//! These messages route through SystemRelay for centralized monitoring.

use super::ParseError;
// Legacy TLV types removed - using Protocol V2 MessageHeader + TLV extensions
use crate::SourceType; // TLVType removed with legacy TLV system
use std::collections::HashMap;
use zerocopy::{AsBytes, FromBytes, FromZeroes};

/// Type alias for trace identifiers
pub type TraceId = [u8; 16];

/// TraceContext TLV - Distributed tracing for message flow observability
///
/// Routes through SystemRelay for centralized trace aggregation.
/// Enables end-to-end tracing from Polygon Collector → Dashboard.
///
/// Size: 32 bytes (fits in bounded constraint 32-256 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, AsBytes, FromBytes, FromZeroes)]
pub struct TraceContextTLV {
    /// Unique trace ID for this message flow (16 bytes for UUID-like uniqueness)
    pub trace_id: [u8; 16],

    /// Source service that initiated this trace (relay consumer, strategy, etc.)
    pub source_service: u8, // SourceType as u8

    /// Current span depth (how many hops from origin)
    pub span_depth: u8,

    /// Processing stage flags (bit field for different pipeline stages)
    pub stage_flags: u8, // 0x01=Collected, 0x02=Relayed, 0x04=Processed, 0x08=Executed

    /// Reserved for future use
    pub reserved: u8,

    /// Timestamp when trace was started (nanoseconds since epoch)
    pub start_timestamp_ns: u64,

    /// Current processing timestamp (nanoseconds since epoch)
    pub current_timestamp_ns: u64,
}

impl TraceContextTLV {
    /// Processing stage flags for trace context
    pub const STAGE_COLLECTED: u8 = 0x01; // Data collected from exchange
    pub const STAGE_RELAYED: u8 = 0x02; // Forwarded by relay
    pub const STAGE_PROCESSED: u8 = 0x04; // Processed by strategy
    pub const STAGE_EXECUTED: u8 = 0x08; // Execution completed

    /// Create new trace context with unique trace ID
    pub fn new(source: SourceType) -> Self {
        let current_time = current_timestamp_ns();

        Self {
            trace_id: generate_trace_id(),
            source_service: source as u8,
            span_depth: 0,
            stage_flags: 0,
            reserved: 0,
            start_timestamp_ns: current_time,
            current_timestamp_ns: current_time,
        }
    }

    /// Continue existing trace with incremented span depth
    pub fn continue_trace(&self, current_service: SourceType) -> Self {
        Self {
            trace_id: self.trace_id,
            source_service: current_service as u8,
            span_depth: self.span_depth.saturating_add(1),
            stage_flags: self.stage_flags,
            reserved: 0,
            start_timestamp_ns: self.start_timestamp_ns,
            current_timestamp_ns: current_timestamp_ns(),
        }
    }

    /// Mark processing stage as completed
    pub fn mark_stage(&mut self, stage: u8) {
        self.stage_flags |= stage;
        self.current_timestamp_ns = current_timestamp_ns();
    }

    /// Check if stage is completed
    pub fn has_stage(&self, stage: u8) -> bool {
        (self.stage_flags & stage) != 0
    }

    /// Get trace ID as hex string for logging
    pub fn trace_id_hex(&self) -> String {
        hex::encode(self.trace_id)
    }

    /// Get elapsed time since trace start (nanoseconds)
    pub fn elapsed_ns(&self) -> u64 {
        self.current_timestamp_ns
            .saturating_sub(self.start_timestamp_ns)
    }

    /// Get source service type
    pub fn source(&self) -> Result<SourceType, ParseError> {
        SourceType::try_from(self.source_service)
            .map_err(|_| ParseError::UnknownSource(self.source_service))
    }

    // Legacy to_tlv_message removed - use Protocol V2 TLVMessageBuilder instead

    /// Parse from bytes with validation
    pub fn from_bytes(data: &[u8]) -> Result<Self, ParseError> {
        if data.len() < std::mem::size_of::<Self>() {
            return Err(ParseError::MessageTooSmall {
                need: std::mem::size_of::<Self>(),
                got: data.len(),
            });
        }

        use zerocopy::Ref;
        let tlv_ref = Ref::<_, Self>::new(data).ok_or(ParseError::InvalidExtendedTLV)?;
        Ok(tlv_ref.into_ref().clone())
    }
}

/// SystemHealth TLV - Component health monitoring
///
/// Reports health status of individual services for real-time monitoring.
/// Size: 48 bytes (fixed size for predictable processing)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, AsBytes, FromBytes, FromZeroes)]
pub struct SystemHealthTLV {
    /// Service type reporting health
    pub service_type: u8, // SourceType as u8

    /// Health status: 0=Healthy, 1=Degraded, 2=Unhealthy, 3=Unknown
    pub health_status: u8,

    /// CPU usage percentage (0-100)
    pub cpu_usage_pct: u8,

    /// Memory usage percentage (0-100)  
    pub memory_usage_pct: u8,

    /// Active connections count
    pub connection_count: u32,

    /// Messages processed per second (average over last minute)
    pub message_rate_per_sec: u32,

    /// Error rate per thousand messages (0-1000)
    pub error_rate_per_thousand: u16,

    /// Latency percentile 95th in microseconds
    pub latency_p95_us: u16,

    /// Timestamp of health check
    pub timestamp_ns: u64,

    /// Last error code (0 = no error)
    pub last_error_code: u32,

    /// Reserved for future metrics
    pub reserved: [u8; 16],
}

impl SystemHealthTLV {
    /// Health status constants
    pub const HEALTH_OK: u8 = 0;
    pub const HEALTH_DEGRADED: u8 = 1;
    pub const HEALTH_UNHEALTHY: u8 = 2;
    pub const HEALTH_UNKNOWN: u8 = 3;

    /// Create new health report
    pub fn new(
        service: SourceType,
        status: u8,
        cpu_pct: u8,
        memory_pct: u8,
        connections: u32,
        msg_rate: u32,
    ) -> Self {
        Self {
            service_type: service as u8,
            health_status: status,
            cpu_usage_pct: cpu_pct,
            memory_usage_pct: memory_pct,
            connection_count: connections,
            message_rate_per_sec: msg_rate,
            error_rate_per_thousand: 0,
            latency_p95_us: 0,
            timestamp_ns: current_timestamp_ns(),
            last_error_code: 0,
            reserved: [0; 16],
        }
    }

    /// Check if service is healthy
    pub fn is_healthy(&self) -> bool {
        self.health_status == Self::HEALTH_OK
    }

    /// Get service type
    pub fn service(&self) -> Result<SourceType, ParseError> {
        SourceType::try_from(self.service_type)
            .map_err(|_| ParseError::UnknownSource(self.service_type))
    }

    // Legacy to_tlv_message removed - use Protocol V2 TLVMessageBuilder instead

    /// Parse from bytes with validation
    pub fn from_bytes(data: &[u8]) -> Result<Self, ParseError> {
        if data.len() < std::mem::size_of::<Self>() {
            return Err(ParseError::MessageTooSmall {
                need: std::mem::size_of::<Self>(),
                got: data.len(),
            });
        }

        use zerocopy::Ref;
        let tlv_ref = Ref::<_, Self>::new(data).ok_or(ParseError::InvalidExtendedTLV)?;
        Ok(tlv_ref.into_ref().clone())
    }
}

/// Generate unique trace ID (16 bytes)
///
/// Uses timestamp + random bytes for uniqueness across distributed services
fn generate_trace_id() -> [u8; 16] {
    let mut trace_id = [0u8; 16];

    // First 8 bytes: current timestamp in nanoseconds
    let timestamp = current_timestamp_ns();
    trace_id[0..8].copy_from_slice(&timestamp.to_le_bytes());

    // Last 8 bytes: random data for uniqueness
    // In a real implementation, use proper random number generator
    // For now, use a simple pseudo-random based on timestamp
    let random_seed = timestamp.wrapping_mul(0x5DEECE66D).wrapping_add(0xB);
    trace_id[8..16].copy_from_slice(&random_seed.to_le_bytes());

    trace_id
}

/// Get current timestamp in nanoseconds since Unix epoch  
fn current_timestamp_ns() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

/// Trace Event - individual step in message flow
///
/// Used by TraceCollector to track message progression through the system
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TraceEvent {
    pub trace_id: [u8; 16],
    pub service: SourceType,
    pub event_type: TraceEventType,
    pub timestamp_ns: u64,
    pub duration_ns: Option<u64>,
    pub metadata: HashMap<String, String>,
}

/// Types of trace events in the pipeline
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TraceEventType {
    /// Data collected from external exchange
    DataCollected,

    /// Message sent to relay
    MessageSent,

    /// Message received from relay
    MessageReceived,

    /// Message processed by strategy/consumer  
    MessageProcessed,

    /// Execution action taken
    ExecutionTriggered,

    /// Error occurred during processing
    ErrorOccurred,
}

impl TraceEvent {
    /// Create new trace event
    pub fn new(trace_id: [u8; 16], service: SourceType, event_type: TraceEventType) -> Self {
        Self {
            trace_id,
            service,
            event_type,
            timestamp_ns: current_timestamp_ns(),
            duration_ns: None,
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to trace event
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Set duration for timing analysis
    pub fn with_duration(mut self, duration_ns: u64) -> Self {
        self.duration_ns = Some(duration_ns);
        self
    }

    /// Get trace ID as hex string
    pub fn trace_id_hex(&self) -> String {
        hex::encode(self.trace_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_context_size() {
        assert_eq!(std::mem::size_of::<TraceContextTLV>(), 32);
    }

    #[test]
    fn test_system_health_size() {
        assert_eq!(std::mem::size_of::<SystemHealthTLV>(), 48);
    }

    #[test]
    fn test_trace_context_creation() {
        let trace = TraceContextTLV::new(SourceType::PolygonCollector);

        assert_eq!(trace.source_service, SourceType::PolygonCollector as u8);
        assert_eq!(trace.span_depth, 0);
        assert_eq!(trace.stage_flags, 0);
        assert!(trace.start_timestamp_ns > 0);
        assert_eq!(trace.start_timestamp_ns, trace.current_timestamp_ns);
    }

    #[test]
    fn test_trace_context_continue() {
        let original = TraceContextTLV::new(SourceType::PolygonCollector);
        let continued = original.continue_trace(SourceType::MarketDataRelay);

        // Same trace ID but different service and incremented depth
        assert_eq!(original.trace_id, continued.trace_id);
        assert_eq!(continued.source_service, SourceType::MarketDataRelay as u8);
        assert_eq!(continued.span_depth, 1);
        assert_eq!(continued.start_timestamp_ns, original.start_timestamp_ns);
        assert!(continued.current_timestamp_ns >= original.current_timestamp_ns);
    }

    #[test]
    fn test_trace_stage_marking() {
        let mut trace = TraceContextTLV::new(SourceType::PolygonCollector);

        assert!(!trace.has_stage(TraceContextTLV::STAGE_COLLECTED));

        trace.mark_stage(TraceContextTLV::STAGE_COLLECTED);
        assert!(trace.has_stage(TraceContextTLV::STAGE_COLLECTED));

        trace.mark_stage(TraceContextTLV::STAGE_RELAYED);
        assert!(trace.has_stage(TraceContextTLV::STAGE_COLLECTED));
        assert!(trace.has_stage(TraceContextTLV::STAGE_RELAYED));
    }

    #[test]
    fn test_trace_serialization() {
        let original = TraceContextTLV::new(SourceType::ArbitrageStrategy);
        // Legacy TLV message test removed - use Protocol V2 TLVMessageBuilder for testing
        let bytes = original.as_bytes().to_vec();

        assert_eq!(bytes.len(), 32);

        let recovered = TraceContextTLV::from_bytes(&bytes).unwrap();
        assert_eq!(original, recovered);
    }

    #[test]
    fn test_health_creation() {
        let health = SystemHealthTLV::new(
            SourceType::PolygonCollector,
            SystemHealthTLV::HEALTH_OK,
            25,   // 25% CPU
            40,   // 40% memory
            10,   // 10 connections
            1500, // 1500 msg/sec
        );

        assert!(health.is_healthy());
        assert_eq!(health.cpu_usage_pct, 25);
        assert_eq!(health.memory_usage_pct, 40);
        assert_eq!(health.connection_count, 10);
        assert_eq!(health.message_rate_per_sec, 1500);
    }

    #[test]
    fn test_trace_event_creation() {
        let trace_id = generate_trace_id();
        let event = TraceEvent::new(
            trace_id,
            SourceType::PolygonCollector,
            TraceEventType::DataCollected,
        )
        .with_metadata("exchange", "polygon")
        .with_metadata("symbol", "USDC/WETH")
        .with_duration(1_500_000); // 1.5ms

        assert_eq!(event.trace_id, trace_id);
        assert_eq!(event.service, SourceType::PolygonCollector);
        assert_eq!(event.event_type, TraceEventType::DataCollected);
        assert_eq!(event.duration_ns, Some(1_500_000));
        assert_eq!(event.metadata.get("exchange"), Some(&"polygon".to_string()));
    }
}

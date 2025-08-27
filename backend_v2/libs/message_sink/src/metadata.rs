/// Information about a sink for monitoring/debugging
#[derive(Debug, Clone, Default)]
pub struct SinkMetadata {
    /// Human-readable sink name
    pub name: String,

    /// Sink type (relay, direct, composite, etc.)
    pub sink_type: String,

    /// Connection endpoint if applicable
    pub endpoint: Option<String>,

    /// Current connection state
    pub state: ConnectionState,

    /// Messages sent successfully
    pub messages_sent: u64,

    /// Messages failed to send
    pub messages_failed: u64,

    /// Last error if any
    pub last_error: Option<String>,
}

impl SinkMetadata {
    /// Create new metadata with name and type
    pub fn new(name: impl Into<String>, sink_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            sink_type: sink_type.into(),
            endpoint: None,
            state: ConnectionState::Disconnected,
            messages_sent: 0,
            messages_failed: 0,
            last_error: None,
        }
    }

    /// Set endpoint
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    /// Set connection state
    pub fn with_state(mut self, state: ConnectionState) -> Self {
        self.state = state;
        self
    }

    /// Record successful message send
    pub fn record_success(&mut self) {
        self.messages_sent += 1;
    }

    /// Record failed message send
    pub fn record_failure(&mut self, error: Option<String>) {
        self.messages_failed += 1;
        self.last_error = error;
    }

    /// Update connection state
    pub fn update_state(&mut self, state: ConnectionState) {
        self.state = state;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Failed,
}

impl Default for ConnectionState {
    fn default() -> Self {
        ConnectionState::Disconnected
    }
}

impl ConnectionState {
    /// Check if connection is active
    pub fn is_active(&self) -> bool {
        matches!(self, ConnectionState::Connected)
    }

    /// Check if connection can be established
    pub fn can_connect(&self) -> bool {
        matches!(
            self,
            ConnectionState::Disconnected | ConnectionState::Failed
        )
    }

    /// Check if connection is in progress
    pub fn is_connecting(&self) -> bool {
        matches!(self, ConnectionState::Connecting)
    }
}

/// Connection health information
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionHealth {
    /// Connection is healthy
    Healthy,
    /// Connection is degraded (slow responses, intermittent failures)
    Degraded,
    /// Connection is unhealthy (frequent failures, timeouts)
    Unhealthy,
    /// Connection health unknown (insufficient data)
    Unknown,
}

impl Default for ConnectionHealth {
    fn default() -> Self {
        ConnectionHealth::Unknown
    }
}

impl ConnectionHealth {
    /// Check if connection is usable
    pub fn is_usable(&self) -> bool {
        matches!(self, ConnectionHealth::Healthy | ConnectionHealth::Degraded)
    }

    /// Check if connection needs attention
    pub fn needs_attention(&self) -> bool {
        matches!(
            self,
            ConnectionHealth::Degraded | ConnectionHealth::Unhealthy
        )
    }
}

/// Extended sink metadata with health monitoring
#[derive(Debug, Clone)]
pub struct ExtendedSinkMetadata {
    /// Basic sink metadata
    pub metadata: SinkMetadata,

    /// Connection health status
    pub health: ConnectionHealth,

    /// Last successful send timestamp
    pub last_successful_send: Option<std::time::SystemTime>,

    /// Average latency in nanoseconds (rolling window)
    pub avg_latency_ns: Option<u64>,

    /// Error rate over last N operations (0.0 to 1.0)
    pub error_rate: Option<f64>,

    /// Current active connections
    pub active_connections: usize,

    /// Preferred connection count
    pub preferred_connections: usize,

    /// Whether sink supports multiplexing
    pub supports_multiplexing: bool,
}

impl Default for ExtendedSinkMetadata {
    fn default() -> Self {
        Self {
            metadata: SinkMetadata::default(),
            health: ConnectionHealth::Unknown,
            last_successful_send: None,
            avg_latency_ns: None,
            error_rate: None,
            active_connections: 0,
            preferred_connections: 1,
            supports_multiplexing: false,
        }
    }
}

impl ExtendedSinkMetadata {
    /// Create extended metadata from basic metadata
    pub fn from_metadata(metadata: SinkMetadata) -> Self {
        Self {
            metadata,
            ..Default::default()
        }
    }

    /// Update health based on recent metrics
    pub fn update_health(&mut self, recent_success_rate: f64, recent_avg_latency_ns: u64) {
        self.error_rate = Some(1.0 - recent_success_rate);
        self.avg_latency_ns = Some(recent_avg_latency_ns);

        self.health = if recent_success_rate >= 0.99 && recent_avg_latency_ns < 10_000_000 {
            // >99% success rate and <10ms latency = healthy
            ConnectionHealth::Healthy
        } else if recent_success_rate >= 0.95 {
            // >95% success rate = degraded but usable
            ConnectionHealth::Degraded
        } else if recent_success_rate > 0.0 {
            // Some successes = unhealthy but not dead
            ConnectionHealth::Unhealthy
        } else {
            // No recent successes = unknown/dead
            ConnectionHealth::Unknown
        };
    }

    /// Record successful operation
    pub fn record_successful_operation(&mut self) {
        self.last_successful_send = Some(std::time::SystemTime::now());
        self.metadata.record_success();
    }

    /// Record failed operation
    pub fn record_failed_operation(&mut self, error: String) {
        self.metadata.record_failure(Some(error));
    }
}

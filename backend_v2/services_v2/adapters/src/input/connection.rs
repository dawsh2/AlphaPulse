//! WebSocket connection management with automatic reconnection

use alphapulse_types::VenueId;
use futures_util::{SinkExt, StreamExt};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

use crate::{AdapterError, Result};
use crate::{AdapterMetrics, CircuitBreaker, CircuitBreakerConfig, ErrorType};

type WsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

/// Connection states for WebSocket lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Not connected
    Disconnected,
    /// Attempting to connect
    Connecting,
    /// Successfully connected and receiving data
    Connected,
    /// Attempting to reconnect after failure
    Reconnecting,
    /// Permanent failure, manual intervention required
    Failed,
}

/// Reason for disconnection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisconnectReason {
    /// Network error or timeout
    NetworkError,
    /// Authentication failed
    AuthenticationFailed,
    /// Rate limited by venue
    RateLimited,
    /// Internal error in processing
    InternalError,
    /// User-requested disconnection
    GracefulShutdown,
}

/// Configuration for connection management
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    /// WebSocket URL
    pub url: String,
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Message timeout (no messages received)
    pub message_timeout: Duration,
    /// Base backoff time for reconnection
    pub base_backoff_ms: u64,
    /// Maximum backoff time
    pub max_backoff_ms: u64,
    /// Maximum reconnection attempts
    pub max_reconnect_attempts: u32,
    /// Health check interval
    pub health_check_interval: Duration,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            connect_timeout: Duration::from_secs(10),
            message_timeout: Duration::from_secs(30),
            base_backoff_ms: 1000,
            max_backoff_ms: 30000,
            max_reconnect_attempts: 10,
            health_check_interval: Duration::from_secs(5),
        }
    }
}

/// WebSocket connection manager with automatic reconnection
pub struct ConnectionManager {
    venue: VenueId,
    config: ConnectionConfig,
    state: Arc<RwLock<ConnectionState>>,
    websocket: Arc<RwLock<Option<WsStream>>>,
    circuit_breaker: CircuitBreaker,
    metrics: Arc<AdapterMetrics>,

    // Tracking
    last_message_time: Arc<RwLock<u64>>, // Nanoseconds since epoch
    reconnect_count: Arc<RwLock<u32>>,
    backoff_multiplier: Arc<RwLock<u32>>,
    tracked_instruments: Arc<RwLock<HashSet<alphapulse_types::protocol::InstrumentId>>>,
}

impl ConnectionManager {
    /// Create a new connection manager
    pub fn new(venue: VenueId, config: ConnectionConfig, metrics: Arc<AdapterMetrics>) -> Self {
        let circuit_config = CircuitBreakerConfig {
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(60),
            success_threshold: 3,
            half_open_max_failures: 1,
        };

        Self {
            venue,
            config,
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            websocket: Arc::new(RwLock::new(None)),
            circuit_breaker: CircuitBreaker::new(circuit_config),
            metrics,
            last_message_time: Arc::new(RwLock::new(current_nanos())),
            reconnect_count: Arc::new(RwLock::new(0)),
            backoff_multiplier: Arc::new(RwLock::new(1)),
            tracked_instruments: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Connect to the WebSocket endpoint
    pub async fn connect(&self) -> Result<()> {
        // Check circuit breaker state first
        if !self.circuit_breaker.should_attempt().await {
            return Err(AdapterError::CircuitBreakerOpen { venue: self.venue });
        }

        // Attempt connection
        match self.attempt_connection().await {
            Ok(()) => {
                self.circuit_breaker.on_success().await;
                Ok(())
            }
            Err(e) => {
                self.circuit_breaker.on_failure().await;
                Err(e)
            }
        }
    }

    /// Attempt to establish connection
    async fn attempt_connection(&self) -> Result<()> {
        *self.state.write().await = ConnectionState::Connecting;

        tracing::info!("Connecting to {} at {}", self.venue, self.config.url);

        // Connect with timeout
        let connect_future = connect_async(&self.config.url);

        match timeout(self.config.connect_timeout, connect_future).await {
            Ok(Ok((ws_stream, response))) => {
                tracing::info!(
                    "Connected to {} with response: {:?}",
                    self.venue,
                    response.status()
                );

                *self.websocket.write().await = Some(ws_stream);
                *self.state.write().await = ConnectionState::Connected;
                *self.last_message_time.write().await = current_nanos();
                *self.backoff_multiplier.write().await = 1; // Reset backoff

                self.metrics.record_connection(self.venue);

                Ok(())
            }
            Ok(Err(e)) => {
                tracing::error!("WebSocket connection error for {}: {}", self.venue, e);
                *self.state.write().await = ConnectionState::Disconnected;
                self.metrics.record_connection_failure(self.venue);

                Err(AdapterError::ConnectionFailed {
                    venue: self.venue,
                    reason: e.to_string(),
                })
            }
            Err(_) => {
                tracing::error!(
                    "Connection timeout for {} after {:?}",
                    self.venue,
                    self.config.connect_timeout
                );
                *self.state.write().await = ConnectionState::Disconnected;
                self.metrics.record_connection_failure(self.venue);

                Err(AdapterError::ConnectionTimeout {
                    venue: self.venue,
                    timeout_ms: self.config.connect_timeout.as_millis() as u64,
                })
            }
        }
    }

    /// Handle disconnection and trigger reconnection
    pub async fn handle_disconnection(&self, reason: DisconnectReason) -> Result<()> {
        tracing::warn!("Disconnected from {} due to {:?}", self.venue, reason);

        *self.state.write().await = ConnectionState::Reconnecting;
        *self.websocket.write().await = None;
        self.metrics.record_disconnection(self.venue);

        // Don't reconnect on graceful shutdown
        if reason == DisconnectReason::GracefulShutdown {
            *self.state.write().await = ConnectionState::Disconnected;
            return Ok(());
        }

        // Check if we should give up
        let reconnect_count = {
            let mut count = self.reconnect_count.write().await;
            *count += 1;
            *count
        };

        if reconnect_count >= self.config.max_reconnect_attempts {
            tracing::error!(
                "Max reconnection attempts ({}) exceeded for {}",
                self.config.max_reconnect_attempts,
                self.venue
            );
            *self.state.write().await = ConnectionState::Failed;

            return Err(AdapterError::MaxReconnectAttemptsExceeded {
                venue: self.venue,
                max_attempts: self.config.max_reconnect_attempts,
            });
        }

        // Calculate backoff
        let backoff = self.calculate_backoff().await;
        tracing::info!(
            "Will reconnect to {} in {}ms (attempt {})",
            self.venue,
            backoff.as_millis(),
            reconnect_count
        );

        // Wait before reconnecting
        tokio::time::sleep(backoff).await;

        // Attempt reconnection
        self.connect().await
    }

    /// Calculate exponential backoff duration
    async fn calculate_backoff(&self) -> Duration {
        let multiplier = *self.backoff_multiplier.read().await;
        let backoff_ms = self.config.base_backoff_ms * 2_u64.pow(multiplier);
        let capped_backoff = backoff_ms.min(self.config.max_backoff_ms);

        // Increment multiplier for next time (cap at 6 for 2^6 = 64x)
        *self.backoff_multiplier.write().await = (multiplier + 1).min(6);

        Duration::from_millis(capped_backoff)
    }

    /// Send a message through the WebSocket
    pub async fn send(&self, message: Message) -> Result<()> {
        let mut ws_guard = self.websocket.write().await;

        if let Some(ws) = ws_guard.as_mut() {
            ws.send(message).await.map_err(AdapterError::WebSocket)?;
            Ok(())
        } else {
            Err(AdapterError::ConnectionFailed {
                venue: self.venue,
                reason: "Not connected".to_string(),
            })
        }
    }

    /// Receive next message from WebSocket
    pub async fn receive(&self) -> Result<Option<Message>> {
        let mut ws_guard = self.websocket.write().await;

        if let Some(ws) = ws_guard.as_mut() {
            match ws.next().await {
                Some(Ok(msg)) => {
                    *self.last_message_time.write().await = current_nanos();
                    Ok(Some(msg))
                }
                Some(Err(e)) => {
                    tracing::error!("WebSocket error for {}: {}", self.venue, e);
                    Err(AdapterError::WebSocket(e))
                }
                None => Ok(None),
            }
        } else {
            Err(AdapterError::ConnectionFailed {
                venue: self.venue,
                reason: "Not connected".to_string(),
            })
        }
    }

    /// Check connection health
    pub async fn health_check(&self) -> Result<()> {
        let last_message = *self.last_message_time.read().await;
        let age = Duration::from_nanos(current_nanos() - last_message);

        if age > self.config.message_timeout {
            tracing::warn!(
                "Message timeout for {} ({}ms since last message)",
                self.venue,
                age.as_millis()
            );

            self.metrics.record_processing_error(ErrorType::Timeout);

            return Err(AdapterError::ConnectionTimeout {
                venue: self.venue,
                timeout_ms: age.as_millis() as u64,
            });
        }

        Ok(())
    }

    /// Get current connection state
    pub async fn state(&self) -> ConnectionState {
        *self.state.read().await
    }

    /// Check if connected
    pub async fn is_connected(&self) -> bool {
        *self.state.read().await == ConnectionState::Connected
    }

    /// Add an instrument to track
    pub async fn track_instrument(&self, instrument: alphapulse_types::protocol::InstrumentId) {
        self.tracked_instruments.write().await.insert(instrument);
        self.metrics
            .update_instrument_count(self.venue, self.tracked_instruments.read().await.len());
    }

    /// Get tracked instruments
    pub async fn tracked_instruments(&self) -> Vec<alphapulse_types::protocol::InstrumentId> {
        self.tracked_instruments
            .read()
            .await
            .iter()
            .copied()
            .collect()
    }

    /// Clear all tracked instruments (for state invalidation)
    pub async fn clear_instruments(&self) {
        let count = self.tracked_instruments.read().await.len();
        self.tracked_instruments.write().await.clear();
        self.metrics.record_state_invalidation(self.venue, count);
    }

    /// Close the connection gracefully
    pub async fn close(&self) -> Result<()> {
        let was_connected = matches!(*self.state.read().await, ConnectionState::Connected);
        *self.state.write().await = ConnectionState::Disconnected;

        if let Some(mut ws) = self.websocket.write().await.take() {
            ws.close(None).await.ok();
        }

        // Only record disconnection if we were actually connected
        if was_connected {
            self.metrics.record_disconnection(self.venue);
        }
        Ok(())
    }
}

/// Get current time in nanoseconds since epoch
fn current_nanos() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
}

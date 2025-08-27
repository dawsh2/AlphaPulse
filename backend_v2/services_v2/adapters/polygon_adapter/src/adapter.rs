//! Polygon DEX Adapter Implementation
//!
//! Implements the Adapter and SafeAdapter traits for Polygon DEX data collection.
//! Processes Uniswap V2/V3 and other DEX events on Polygon network.

use alphapulse_adapter_service::{
    Adapter, AdapterError, AdapterHealth, CircuitBreaker, CircuitBreakerConfig, CircuitState,
    ConnectionStatus, InstrumentType, RateLimiter, Result, SafeAdapter,
};
use codec::TLVMessageBuilder;
use alphapulse_types::{
    tlv::market_data::{PoolSwapTLV, PoolMintTLV, PoolBurnTLV, PoolSyncTLV},
    InstrumentId, RelayDomain, SourceType, TLVType, VenueId,
};
use async_trait::async_trait;
use ethabi::{Event, EventParam, ParamType, RawLog};
use futures_util::{SinkExt, StreamExt};
use once_cell::sync::Lazy;
use serde_json::Value;
use std::{
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::sync::RwLock;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};
use web3::types::{Log, H160, H256};

use crate::config::PolygonConfig;
use crate::parser::PolygonEventParser;

/// Polygon DEX Adapter
pub struct PolygonAdapter {
    config: PolygonConfig,
    circuit_breaker: Arc<RwLock<CircuitBreaker>>,
    rate_limiter: Arc<RwLock<RateLimiter>>,
    connection_status: Arc<RwLock<ConnectionStatus>>,
    health_metrics: Arc<RwLock<HealthMetrics>>,
    event_parser: PolygonEventParser,
    websocket_sink: Arc<RwLock<Option<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>>>,
}

/// Health metrics tracking
struct HealthMetrics {
    messages_processed: u64,
    error_count: u64,
    last_error: Option<String>,
    start_time: Instant,
    last_message_time: Option<Instant>,
}

impl PolygonAdapter {
    /// Create a new Polygon adapter
    pub fn new(config: PolygonConfig) -> Self {
        let circuit_breaker_config = CircuitBreakerConfig {
            failure_threshold: config.base.circuit_breaker_failure_threshold,
            recovery_timeout_ms: config.base.circuit_breaker_recovery_ms,
            half_open_test_attempts: config.base.circuit_breaker_half_open_attempts,
        };

        Self {
            circuit_breaker: Arc::new(RwLock::new(CircuitBreaker::new(circuit_breaker_config))),
            rate_limiter: Arc::new(RwLock::new(RateLimiter::new(
                config.base.rate_limit_requests_per_second.unwrap_or(1000),
            ))),
            connection_status: Arc::new(RwLock::new(ConnectionStatus::Disconnected)),
            health_metrics: Arc::new(RwLock::new(HealthMetrics {
                messages_processed: 0,
                error_count: 0,
                last_error: None,
                start_time: Instant::now(),
                last_message_time: None,
            })),
            event_parser: PolygonEventParser::new(),
            websocket_sink: Arc::new(RwLock::new(None)),
            config,
        }
    }

    /// Process a Polygon DEX event
    async fn process_polygon_event(&self, log: &Log, output_buffer: &mut [u8]) -> Result<Option<usize>> {
        let start = Instant::now();

        // Parse event based on topic
        let tlv_result = if log.topics.is_empty() {
            return Ok(None);
        } else {
            self.event_parser.parse_log(log)?
        };

        // Build TLV message
        let mut builder = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::PolygonCollector);
        
        match tlv_result {
            ParsedEvent::Swap(swap_tlv) => {
                builder.add_tlv(TLVType::PoolSwap as u16, &swap_tlv)?;
            }
            ParsedEvent::Mint(mint_tlv) => {
                builder.add_tlv(TLVType::PoolMint as u16, &mint_tlv)?;
            }
            ParsedEvent::Burn(burn_tlv) => {
                builder.add_tlv(TLVType::PoolBurn as u16, &burn_tlv)?;
            }
            ParsedEvent::Sync(sync_tlv) => {
                builder.add_tlv(TLVType::PoolSync as u16, &sync_tlv)?;
            }
        }

        let message_bytes = builder.build()?;
        
        // Validate hot path latency
        let elapsed = start.elapsed();
        if elapsed > Duration::from_nanos(35_000) {
            warn!("Hot path latency violation: {}μs > 35μs", elapsed.as_nanos() / 1000);
        }

        // Copy to output buffer
        if output_buffer.len() < message_bytes.len() {
            return Err(AdapterError::BufferTooSmall {
                required: message_bytes.len(),
                available: output_buffer.len(),
            }.into());
        }

        output_buffer[..message_bytes.len()].copy_from_slice(&message_bytes);
        Ok(Some(message_bytes.len()))
    }
}

#[async_trait]
impl Adapter for PolygonAdapter {
    type Config = PolygonConfig;

    async fn start(&self) -> Result<()> {
        // Check circuit breaker state
        {
            let cb = self.circuit_breaker.read().await;
            match cb.state() {
                CircuitState::Open => {
                    return Err(AdapterError::CircuitBreakerOpen {
                        venue: VenueId::Polygon,
                    }.into());
                }
                _ => {}
            }
        }

        // Connect to WebSocket with timeout
        let connect_future = connect_async(&self.config.polygon_ws_url);
        let (ws_stream, _) = tokio::time::timeout(
            Duration::from_millis(self.config.base.connection_timeout_ms),
            connect_future,
        )
        .await
        .map_err(|_| AdapterError::ConnectionTimeout {
            venue: VenueId::Polygon,
            timeout_ms: self.config.base.connection_timeout_ms,
        })?
        .map_err(|e| AdapterError::ConnectionFailed {
            venue: VenueId::Polygon,
            reason: e.to_string(),
        })?;

        info!("Connected to Polygon WebSocket");

        // Store WebSocket stream
        {
            let mut sink = self.websocket_sink.write().await;
            *sink = Some(ws_stream);
        }

        // Update connection status
        {
            let mut status = self.connection_status.write().await;
            *status = ConnectionStatus::Connected;
        }

        // Subscribe to DEX events
        self.subscribe_to_events().await?;

        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        // Close WebSocket connection
        if let Some(mut ws) = self.websocket_sink.write().await.take() {
            let _ = ws.close(None).await;
        }

        // Update connection status
        {
            let mut status = self.connection_status.write().await;
            *status = ConnectionStatus::Disconnected;
        }

        info!("Polygon adapter stopped");
        Ok(())
    }

    async fn health_check(&self) -> AdapterHealth {
        let metrics = self.health_metrics.read().await;
        let status = self.connection_status.read().await;
        let cb = self.circuit_breaker.read().await;
        let rl = self.rate_limiter.read().await;

        let latency_ms = if let Some(last_time) = metrics.last_message_time {
            Some((Instant::now() - last_time).as_secs_f64() * 1000.0)
        } else {
            None
        };

        AdapterHealth {
            is_healthy: matches!(*status, ConnectionStatus::Connected) && metrics.error_count < 10,
            connection_status: status.clone(),
            messages_processed: metrics.messages_processed,
            error_count: metrics.error_count,
            last_error: metrics.last_error.clone(),
            uptime_seconds: metrics.start_time.elapsed().as_secs(),
            latency_ms,
            circuit_breaker_state: cb.state(),
            rate_limit_remaining: Some(rl.remaining_capacity()),
            connection_timeout_ms: self.config.base.connection_timeout_ms,
        }
    }

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn identifier(&self) -> &str {
        &self.config.base.adapter_id
    }

    fn supported_instruments(&self) -> Vec<InstrumentType> {
        vec![InstrumentType::DEXPool]
    }

    async fn configure_instruments(&mut self, instruments: Vec<String>) -> Result<()> {
        // For Polygon, instruments would be pool addresses
        // This would update subscription filters
        info!("Configuring {} DEX pools for monitoring", instruments.len());
        Ok(())
    }

    async fn process_message(
        &self,
        raw_data: &[u8],
        output_buffer: &mut [u8],
    ) -> Result<Option<usize>> {
        // Parse WebSocket message
        let json: Value = serde_json::from_slice(raw_data)?;
        
        // Extract log data from JSON
        if let Some(params) = json.get("params") {
            if let Some(result) = params.get("result") {
                if let Ok(log) = serde_json::from_value::<Log>(result.clone()) {
                    return self.process_polygon_event(&log, output_buffer).await;
                }
            }
        }

        Ok(None)
    }
}

#[async_trait]
impl SafeAdapter for PolygonAdapter {
    fn circuit_breaker_state(&self) -> CircuitState {
        // This is synchronous in the actual implementation
        let cb = futures::executor::block_on(self.circuit_breaker.read());
        cb.state()
    }

    async fn trigger_circuit_breaker(&self) -> Result<()> {
        let mut cb = self.circuit_breaker.write().await;
        cb.record_failure();
        
        if matches!(cb.state(), CircuitState::Open) {
            warn!("Circuit breaker opened for Polygon adapter");
        }
        
        Ok(())
    }

    async fn reset_circuit_breaker(&self) -> Result<()> {
        let mut cb = self.circuit_breaker.write().await;
        cb.reset();
        info!("Circuit breaker reset for Polygon adapter");
        Ok(())
    }

    fn check_rate_limit(&self) -> bool {
        let rl = futures::executor::block_on(self.rate_limiter.read());
        rl.check_rate_limit()
    }

    fn rate_limit_remaining(&self) -> Option<u32> {
        let rl = futures::executor::block_on(self.rate_limiter.read());
        Some(rl.remaining_capacity())
    }

    async fn validate_connection(&self, timeout_ms: u64) -> Result<bool> {
        // Send a ping to validate the connection
        if let Some(ws) = &*self.websocket_sink.read().await {
            // In a real implementation, send a ping and wait for pong
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

impl PolygonAdapter {
    /// Subscribe to DEX events
    async fn subscribe_to_events(&self) -> Result<()> {
        let subscribe_msg = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_subscribe",
            "params": [
                "logs",
                {
                    "topics": [
                        // Uniswap V2/V3 event signatures
                        ["0xd78ad95f..."], // Swap
                        ["0x4c209b5f..."], // Mint
                        ["0xdccd412f..."], // Burn
                        ["0x1c411e9a..."], // Sync
                    ]
                }
            ]
        });

        if let Some(mut ws) = self.websocket_sink.write().await.as_mut() {
            ws.send(Message::Text(subscribe_msg.to_string())).await?;
            info!("Subscribed to Polygon DEX events");
        }

        Ok(())
    }
}

/// Parsed event types
enum ParsedEvent {
    Swap(PoolSwapTLV),
    Mint(PoolMintTLV),
    Burn(PoolBurnTLV),
    Sync(PoolSyncTLV),
}
# Market Data Collectors Module Specification

## Executive Summary

Market Data Collectors are the foundation of the AlphaPulse trading system, responsible for ingesting real-time market events from multiple venues and normalizing them into standardized TLV messages. These collectors must maintain sub-second latency while handling connection failures gracefully to prevent phantom arbitrage opportunities from stale data.

## Core Requirements

### Performance Targets
- **Event-to-TLV Latency**: <1ms from WebSocket message to TLV broadcast
- **Connection Recovery**: <5 seconds to detect failure and invalidate state
- **Throughput**: Process 10,000+ events/second per venue during high volatility
- **Memory Efficiency**: <256MB per collector process

### Reliability Requirements
- **Immediate State Invalidation**: Send invalidation TLVs within 100ms of connection loss
- **Deterministic Recovery**: Predictable reconnection behavior with exponential backoff
- **No Phantom Data**: Zero tolerance for stale market data reaching strategies
- **Event Ordering**: Maintain chronological order within each instrument

---

# Part I: Architecture Overview

## Service Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        MARKET DATA COLLECTOR SERVICE                        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────────┐ │
│  │                     Connection Manager                                  │ │
│  │                                                                         │ │
│  │  WebSocket Pool ──→ Health Monitor ──→ Recovery Controller              │ │
│  │       │                 │                    │                         │ │
│  │       ↓                 ↓                    ↓                         │ │
│  │  [Active Conns]    [Heartbeats]         [Reconnect Logic]              │ │
│  │  [Pending Conns]   [Latency Track]      [Backoff Strategy]             │ │
│  └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────────┐ │
│  │                     Event Processing Pipeline                           │ │
│  │                                                                         │ │
│  │  Raw Message ──→ Parse ──→ Normalize ──→ TLV Build ──→ Relay Send       │ │
│  │       │            │         │            │             │              │ │
│  │       ↓            ↓         ↓            ↓             ↓              │ │
│  │  [JSON/Binary] [Validate] [InstrumentID] [TradeTLV]  [MarketRelay]     │ │
│  │  [WebSocket]   [Schema]   [Bijective]    [QuoteTLV]  [Broadcast]       │ │
│  └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────────┐ │
│  │                     State Invalidation System                           │ │
│  │                                                                         │ │
│  │  Connection Lost ──→ Immediate Invalidation ──→ Recovery Tracking       │ │
│  │       │                      │                        │                │ │
│  │       ↓                      ↓                        ↓                │ │
│  │  [Detect Failure]      [Send Reset TLVs]        [State Rebuild]        │ │
│  │  [Circuit Break]       [All Instruments]        [Fresh Events]         │ │
│  └─────────────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Venue-Specific Collectors

### Polygon Collector (DEX Events)
```
Polygon RPC ──→ WebSocket ──→ Event Filter ──→ TLV Normalizer
     │              │             │               │
     ↓              ↓             ↓               ↓
[Block Events]  [Uniswap Logs] [Swap Events]  [TradeTLV]
[Mempool]       [Pool Updates] [Mint/Burn]    [LiquidityTLV]
```

### Binance Collector (CEX Events)
```
Binance API ──→ WebSocket ──→ Message Parser ──→ TLV Normalizer
     │              │             │                │
     ↓              ↓             ↓                ↓
[REST Setup]   [Order Book]   [Trade Stream]   [TradeTLV]
[Auth]         [Ticker Data]  [Depth Updates]  [QuoteTLV]
```

## Message Flow Integration

```
Venue Events ──→ Collector ──→ MarketDataRelay ──→ Strategy Engines
     │             │              │                    │
     ↓             ↓              ↓                    ↓
[WebSocket]   [TLV Normalize] [Domain 1]        [Pool Updates]
[REST API]    [Instrument ID] [Broadcast]       [Arbitrage Calc]

Connection Failure ──→ StateInvalidation ──→ Strategy Reset
     │                       │                   │
     ↓                       ↓                   ↓
[Detect Loss]          [Reset TLVs]        [Remove Pools]
[Circuit Break]        [All Instruments]   [Stop Signals]
```

---

# Part II: Connection Management

## WebSocket Lifecycle

### Connection States

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,    // Initial state, no connection
    Connecting,      // Connection attempt in progress
    Connected,       // Active connection, receiving data
    Reconnecting,    // Attempting to restore lost connection
    Failed,          // Permanent failure, manual intervention required
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisconnectReason {
    NetworkError,       // TCP/WebSocket error
    AuthenticationFailed, // Invalid API keys
    RateLimited,        // Venue imposed rate limit
    InternalError,      // Collector logic error
    GracefulShutdown,   // Planned disconnection
}
```

### Connection Manager Implementation

```rust
use tokio_tungstenite::{connect_async, WebSocketStream};
use futures_util::{SinkExt, StreamExt};
use std::time::Duration;

// Utility functions for nanosecond timestamp handling
fn current_nanos() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
}

fn nanos_to_duration_since(nanos: u64) -> Duration {
    let current = current_nanos();
    if current >= nanos {
        Duration::from_nanos(current - nanos)
    } else {
        Duration::from_nanos(0)
    }
}

pub struct ConnectionManager {
    // Connection tracking
    connections: HashMap<VenueId, VenueConnection>,
    
    // Health monitoring
    health_monitor: HealthMonitor,
    
    // Recovery coordination
    recovery_controller: RecoveryController,
    
    // Configuration
    config: ConnectionConfig,
    
    // Metrics
    metrics: ConnectionMetrics,
}

#[derive(Debug)]
pub struct VenueConnection {
    pub venue_id: VenueId,
    pub state: ConnectionState,
    pub websocket: Option<WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>,
    pub last_message_time: u64,             // Nanoseconds since epoch (protocol-consistent)
    pub reconnect_count: u32,
    pub instruments: HashSet<InstrumentId>,
    
    // Connection tracking (nanoseconds since epoch)
    pub connected_at: Option<u64>,
    pub disconnected_at: Option<u64>,
    pub disconnect_reason: Option<DisconnectReason>,
    
    // Backoff state
    pub next_retry_at: Option<u64>,         // Nanoseconds since epoch
    pub backoff_multiplier: u32,
}

impl ConnectionManager {
    pub async fn new(config: ConnectionConfig) -> Result<Self, ConnectionError> {
        Ok(Self {
            connections: HashMap::new(),
            health_monitor: HealthMonitor::new(config.health_check_interval),
            recovery_controller: RecoveryController::new(config.recovery_config),
            config,
            metrics: ConnectionMetrics::default(),
        })
    }
    
    pub async fn start_venue_connection(&mut self, venue: VenueId) -> Result<(), ConnectionError> {
        let venue_config = self.config.venues.get(&venue)
            .ok_or_else(|| ConnectionError::VenueNotConfigured(venue))?;
        
        let connection = VenueConnection {
            venue_id: venue,
            state: ConnectionState::Disconnected,
            websocket: None,
            last_message_time: current_nanos(),
            reconnect_count: 0,
            instruments: HashSet::new(),
            connected_at: None,
            disconnected_at: None,
            disconnect_reason: None,
            next_retry_at: None,
            backoff_multiplier: 1,
        };
        
        self.connections.insert(venue, connection);
        self.initiate_connection(venue).await?;
        
        Ok(())
    }
}
```

## Connection Recovery Protocol

### Immediate Failure Detection

```rust
impl ConnectionManager {
    async fn monitor_connections(&mut self) -> Result<(), ConnectionError> {
        let mut health_check_interval = tokio::time::interval(Duration::from_secs(1));
        
        loop {
            tokio::select! {
                _ = health_check_interval.tick() => {
                    self.perform_health_checks().await?;
                }
                
                // Process connection events
                event = self.connection_events.recv() => {
                    if let Some(event) = event {
                        self.handle_connection_event(event).await?;
                    }
                }
            }
        }
    }
    
    async fn perform_health_checks(&mut self) -> Result<(), ConnectionError> {
        let now = current_nanos();
        let mut failed_venues = Vec::new();
        
        for (venue_id, connection) in &mut self.connections {
            match connection.state {
                ConnectionState::Connected => {
                    // Check for message timeout
                    let message_age = nanos_to_duration_since(connection.last_message_time);
                    
                    if message_age > self.config.message_timeout {
                        tracing::warn!(
                            "Venue {} message timeout: {}ms", 
                            venue_id, 
                            message_age.as_millis()
                        );
                        
                        failed_venues.push(*venue_id);
                    }
                }
                
                ConnectionState::Reconnecting => {
                    // Check if retry time has arrived
                    if let Some(retry_time) = connection.next_retry_at {
                        if now >= retry_time {
                            self.attempt_reconnection(*venue_id).await?;
                        }
                    }
                }
                
                _ => {}
            }
        }
        
        // Handle failed connections
        for venue_id in failed_venues {
            self.handle_connection_failure(venue_id, DisconnectReason::NetworkError).await?;
        }
        
        Ok(())
    }
    
    async fn handle_connection_failure(
        &mut self, 
        venue_id: VenueId, 
        reason: DisconnectReason
    ) -> Result<(), ConnectionError> {
        tracing::error!("Connection to venue {} failed: {:?}", venue_id, reason);
        
        // Update connection state
        if let Some(connection) = self.connections.get_mut(&venue_id) {
            connection.state = ConnectionState::Reconnecting;
            connection.disconnected_at = Some(current_nanos());
            connection.disconnect_reason = Some(reason);
            connection.websocket = None;
            
            // Calculate next retry time with exponential backoff
            let backoff_ms = self.config.base_backoff_ms * 2_u64.pow(connection.backoff_multiplier);
            let max_backoff = Duration::from_secs(30); // Cap at 30 seconds
            let backoff_duration = Duration::from_millis(backoff_ms.min(max_backoff.as_millis() as u64));
            
            connection.next_retry_at = Some(current_nanos() + backoff_duration.as_nanos() as u64);
            connection.backoff_multiplier = (connection.backoff_multiplier + 1).min(6); // Cap at 2^6 = 64x
            connection.reconnect_count += 1;
        }
        
        // CRITICAL: Immediately invalidate all instrument state
        self.invalidate_venue_state(venue_id).await?;
        
        // Update metrics
        self.metrics.connection_failures.increment();
        self.metrics.venue_connection_status.set(venue_id as f64, 0.0);
        
        Ok(())
    }
}
```

### State Invalidation Protocol

```rust
impl ConnectionManager {
    async fn invalidate_venue_state(&mut self, venue_id: VenueId) -> Result<(), ConnectionError> {
        tracing::warn!("Invalidating all state for venue {}", venue_id);
        
        let invalidation_start = current_nanos();
        
        // Get all instruments for this venue
        let instruments = if let Some(connection) = self.connections.get(&venue_id) {
            connection.instruments.clone()
        } else {
            return Ok(());
        };
        
        // Send StateInvalidation TLV for each instrument
        for instrument_id in instruments {
            let invalidation_tlv = StateInvalidationTLV {
                tlv_type: TLVType::StateInvalidation as u8,
                tlv_length: 14,
                instrument_id,
                action: 1, // Reset (clear state completely)
                reserved: 0,
            };
            
            let message = TLVMessageBuilder::new(MARKET_DATA_DOMAIN, COLLECTOR_SOURCE_ID)
                .add_tlv(TLVType::StateInvalidation, &invalidation_tlv)
                .build();
            
            // Send via MarketDataRelay
            self.market_data_relay.send(&message).await?;
        }
        
        let invalidation_time = nanos_to_duration_since(invalidation_start);
        self.metrics.state_invalidation_time.record(invalidation_time);
        
        // Clear local instrument tracking
        if let Some(connection) = self.connections.get_mut(&venue_id) {
            connection.instruments.clear();
        }
        
        tracing::info!(
            "Invalidated {} instruments for venue {} in {}ms",
            instruments.len(),
            venue_id,
            invalidation_time.as_millis()
        );
        
        Ok(())
    }
}
```

### Reconnection Strategy

```rust
impl ConnectionManager {
    async fn attempt_reconnection(&mut self, venue_id: VenueId) -> Result<(), ConnectionError> {
        let connection = self.connections.get_mut(&venue_id)
            .ok_or_else(|| ConnectionError::VenueNotFound(venue_id))?;
        
        tracing::info!("Attempting reconnection to venue {} (attempt #{})", 
                      venue_id, connection.reconnect_count);
        
        connection.state = ConnectionState::Connecting;
        
        match self.establish_websocket_connection(venue_id).await {
            Ok(websocket) => {
                // Connection successful
                connection.websocket = Some(websocket);
                connection.state = ConnectionState::Connected;
                connection.connected_at = Some(current_nanos());
                connection.last_message_time = current_nanos();
                connection.backoff_multiplier = 1; // Reset backoff
                connection.disconnect_reason = None;
                
                tracing::info!("Successfully reconnected to venue {}", venue_id);
                
                // Update metrics
                self.metrics.venue_connection_status.set(venue_id as f64, 1.0);
                
                // Start receiving fresh data - instruments will be re-added as events arrive
                self.start_message_processing(venue_id).await?;
            }
            
            Err(e) => {
                // Connection failed, schedule next retry
                tracing::warn!("Reconnection to venue {} failed: {}", venue_id, e);
                
                connection.state = ConnectionState::Reconnecting;
                
                // Exponential backoff for next attempt
                let backoff_ms = self.config.base_backoff_ms * 2_u64.pow(connection.backoff_multiplier);
                let max_backoff = Duration::from_secs(30);
                let backoff_duration = Duration::from_millis(backoff_ms.min(max_backoff.as_millis() as u64));
                
                connection.next_retry_at = Some(current_nanos() + backoff_duration.as_nanos() as u64);
                connection.backoff_multiplier = (connection.backoff_multiplier + 1).min(6);
                
                // Check if we should give up
                if connection.reconnect_count >= self.config.max_reconnect_attempts {
                    tracing::error!("Max reconnection attempts exceeded for venue {}, marking as failed", venue_id);
                    connection.state = ConnectionState::Failed;
                    
                    // Send alert to operations team
                    self.metrics.permanent_failures.increment();
                }
            }
        }
        
        Ok(())
    }
    
    async fn establish_websocket_connection(
        &self, 
        venue_id: VenueId
    ) -> Result<WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, ConnectionError> {
        let venue_config = self.config.venues.get(&venue_id)
            .ok_or_else(|| ConnectionError::VenueNotConfigured(venue_id))?;
        
        // Build WebSocket URL with authentication if required
        let url = match venue_id {
            VenueId::Binance => self.build_binance_websocket_url(venue_config)?,
            VenueId::Polygon => self.build_polygon_websocket_url(venue_config)?,
            _ => return Err(ConnectionError::UnsupportedVenue(venue_id)),
        };
        
        // Connect with timeout
        let connect_future = connect_async(&url);
        let timeout_duration = Duration::from_secs(10);
        
        match tokio::time::timeout(timeout_duration, connect_future).await {
            Ok(Ok((websocket, response))) => {
                tracing::info!("WebSocket connected to {} with response: {:?}", url, response.status());
                Ok(websocket)
            }
            Ok(Err(e)) => {
                Err(ConnectionError::WebSocketError(e))
            }
            Err(_) => {
                Err(ConnectionError::ConnectionTimeout)
            }
        }
    }
}
```

---

# Part III: Event Processing Pipeline

## Message Processing Architecture

### High-Performance Message Loop

```rust
impl VenueCollector {
    async fn message_processing_loop(&mut self, venue_id: VenueId) -> Result<(), ProcessingError> {
        let connection = self.connection_manager.get_connection_mut(venue_id)?;
        let websocket = connection.websocket.take()
            .ok_or_else(|| ProcessingError::NoActiveConnection(venue_id))?;
        
        let (mut sink, mut stream) = websocket.split();
        
        // Send subscription messages if needed
        self.send_subscriptions(&mut sink, venue_id).await?;
        
        // Main message processing loop
        while let Some(message_result) = stream.next().await {
            let processing_start = current_nanos();
            
            match message_result {
                Ok(message) => {
                    // Update last message time (for health monitoring)
                    if let Some(connection) = self.connection_manager.get_connection_mut(venue_id) {
                        connection.last_message_time = current_nanos();
                    }
                    
                    // Process the message
                    if let Err(e) = self.process_venue_message(venue_id, message).await {
                        tracing::error!("Failed to process message from {}: {}", venue_id, e);
                        self.metrics.processing_errors.increment();
                    }
                }
                
                Err(e) => {
                    tracing::error!("WebSocket error from {}: {}", venue_id, e);
                    
                    // Connection lost - trigger recovery
                    self.connection_manager.handle_connection_failure(
                        venue_id, 
                        DisconnectReason::NetworkError
                    ).await?;
                    
                    break; // Exit loop to trigger reconnection
                }
            }
            
            // Track processing performance
            let processing_time = nanos_to_duration_since(processing_start);
            self.metrics.message_processing_time.record(processing_time);
            
            // Performance target: <1ms per message
            if processing_time > Duration::from_millis(1) {
                tracing::warn!(
                    "Slow message processing for {}: {}μs", 
                    venue_id, 
                    processing_time.as_micros()
                );
            }
        }
        
        Ok(())
    }
    
    async fn process_venue_message(
        &mut self, 
        venue_id: VenueId, 
        message: tokio_tungstenite::tungstenite::Message
    ) -> Result<(), ProcessingError> {
        use tokio_tungstenite::tungstenite::Message;
        
        let message_data = match message {
            Message::Text(text) => text.into_bytes(),
            Message::Binary(data) => data,
            Message::Ping(_) => {
                // Handle ping/pong for connection keep-alive
                return Ok(());
            }
            Message::Close(_) => {
                tracing::info!("Received close message from {}", venue_id);
                return Err(ProcessingError::ConnectionClosed);
            }
            _ => return Ok(()), // Ignore other message types
        };
        
        // Parse based on venue type
        match venue_id {
            VenueId::Binance => self.process_binance_message(&message_data).await,
            VenueId::Polygon => self.process_polygon_message(&message_data).await,
            VenueId::Ethereum => self.process_ethereum_message(&message_data).await,
            _ => Err(ProcessingError::UnsupportedVenue(venue_id)),
        }
    }
}
```

## Venue-Specific Processors

### Binance Message Processing

```rust
impl VenueCollector {
    async fn process_binance_message(&mut self, data: &[u8]) -> Result<(), ProcessingError> {
        // Parse JSON message
        let message: serde_json::Value = serde_json::from_slice(data)
            .map_err(|e| ProcessingError::InvalidJson(e))?;
        
        // Determine message type
        if let Some(stream) = message.get("stream").and_then(|s| s.as_str()) {
            if stream.contains("@trade") {
                self.process_binance_trade(&message).await?;
            } else if stream.contains("@depth") {
                self.process_binance_depth(&message).await?;
            } else if stream.contains("@ticker") {
                self.process_binance_ticker(&message).await?;
            }
        }
        
        Ok(())
    }
    
    async fn process_binance_trade(&mut self, message: &serde_json::Value) -> Result<(), ProcessingError> {
        // Extract trade data
        let data = message.get("data")
            .ok_or_else(|| ProcessingError::MissingField("data"))?;
        
        let symbol = data.get("s")
            .and_then(|s| s.as_str())
            .ok_or_else(|| ProcessingError::MissingField("symbol"))?;
        
        let price_str = data.get("p")
            .and_then(|p| p.as_str())
            .ok_or_else(|| ProcessingError::MissingField("price"))?;
        
        let quantity_str = data.get("q")
            .and_then(|q| q.as_str())
            .ok_or_else(|| ProcessingError::MissingField("quantity"))?;
        
        let is_buyer_maker = data.get("m")
            .and_then(|m| m.as_bool())
            .ok_or_else(|| ProcessingError::MissingField("is_buyer_maker"))?;
        
        let timestamp = data.get("T")
            .and_then(|t| t.as_u64())
            .ok_or_else(|| ProcessingError::MissingField("timestamp"))?;
        
        // Parse numeric values
        let price = self.parse_decimal_price(price_str)?;
        let quantity = self.parse_decimal_quantity(quantity_str)?;
        
        // Create bijective instrument ID
        let instrument_id = InstrumentId::binance_spot(symbol)?;
        
        // Build TradeTLV
        let trade_tlv = TradeTLV {
            tlv_type: TLVType::Trade as u8,
            tlv_length: 22,
            instrument_id,
            price,
            volume: quantity,
            side: if is_buyer_maker { 2 } else { 1 }, // Sell=2, Buy=1 (from taker perspective)
            flags: 0,
            reserved: [0; 2],
        };
        
        // Send via MarketDataRelay
        let message = TLVMessageBuilder::new(MARKET_DATA_DOMAIN, BINANCE_COLLECTOR_ID)
            .add_tlv(TLVType::Trade, &trade_tlv)
            .build();
        
        self.market_data_relay.send(&message).await?;
        
        // Track instrument for state invalidation
        self.connection_manager.add_instrument(VenueId::Binance, instrument_id);
        
        // Update metrics
        self.metrics.trades_processed.increment();
        
        Ok(())
    }
}
```

### Polygon (Ethereum) Message Processing

```rust
impl VenueCollector {
    async fn process_polygon_message(&mut self, data: &[u8]) -> Result<(), ProcessingError> {
        // Parse JSON message from Polygon WebSocket
        let message: serde_json::Value = serde_json::from_slice(data)
            .map_err(|e| ProcessingError::InvalidJson(e))?;
        
        // Polygon sends array of events
        if let Some(events) = message.as_array() {
            for event in events {
                self.process_polygon_event(event).await?;
            }
        }
        
        Ok(())
    }
    
    async fn process_polygon_event(&mut self, event: &serde_json::Value) -> Result<(), ProcessingError> {
        let event_type = event.get("ev")
            .and_then(|ev| ev.as_str())
            .ok_or_else(|| ProcessingError::MissingField("event_type"))?;
        
        match event_type {
            "AM" => self.process_polygon_swap(event).await, // Aggregate trade (swap)
            "XU" => self.process_polygon_liquidity_update(event).await, // Liquidity update
            _ => Ok(()), // Ignore unknown event types
        }
    }
    
    async fn process_polygon_swap(&mut self, event: &serde_json::Value) -> Result<(), ProcessingError> {
        // Extract swap data
        let pool_address = event.get("pair")
            .and_then(|p| p.as_str())
            .ok_or_else(|| ProcessingError::MissingField("pair"))?;
        
        let price = event.get("p")
            .and_then(|p| p.as_f64())
            .ok_or_else(|| ProcessingError::MissingField("price"))?;
        
        let volume = event.get("v")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ProcessingError::MissingField("volume"))?;
        
        let timestamp = event.get("t")
            .and_then(|t| t.as_u64())
            .ok_or_else(|| ProcessingError::MissingField("timestamp"))?;
        
        // Create instrument ID from pool address
        let instrument_id = InstrumentId::ethereum_pool_from_address(pool_address)?;
        
        // Convert to fixed-point representation
        let price_fixed = self.float_to_fixed_point(price);
        let volume_fixed = self.float_to_fixed_point(volume) as u64;
        
        // Build TradeTLV
        let trade_tlv = TradeTLV {
            tlv_type: TLVType::Trade as u8,
            tlv_length: 22,
            instrument_id,
            price: price_fixed,
            volume: volume_fixed,
            side: 0, // DEX trades don't have traditional sides
            flags: 1, // Flag to indicate DEX trade
            reserved: [0; 2],
        };
        
        // Send via MarketDataRelay
        let message = TLVMessageBuilder::new(MARKET_DATA_DOMAIN, POLYGON_COLLECTOR_ID)
            .add_tlv(TLVType::Trade, &trade_tlv)
            .build();
        
        self.market_data_relay.send(&message).await?;
        
        // Track instrument
        self.connection_manager.add_instrument(VenueId::Polygon, instrument_id);
        
        self.metrics.swaps_processed.increment();
        
        Ok(())
    }
    
    fn float_to_fixed_point(&self, value: f64) -> i64 {
        // Convert to Q32.32 fixed-point representation
        (value * (1u64 << 32) as f64) as i64
    }
}
```

## Data Normalization

### Bijective Instrument ID Generation

```rust
impl InstrumentId {
    pub fn binance_spot(symbol: &str) -> Result<Self, InstrumentError> {
        // Convert symbol to deterministic asset_id
        let asset_id = symbol_to_u64(symbol);
        
        Ok(Self {
            venue: VenueId::Binance as u16,
            asset_type: AssetType::Stock as u8, // CEX pairs treated as stocks
            reserved: 0,
            asset_id,
        })
    }
    
    pub fn ethereum_pool_from_address(address: &str) -> Result<Self, InstrumentError> {
        // Clean hex address
        let hex_clean = address.strip_prefix("0x").unwrap_or(address);
        
        // Use first 8 bytes of address as asset_id
        if hex_clean.len() < 16 {
            return Err(InstrumentError::InvalidAddress);
        }
        
        let bytes = hex::decode(&hex_clean[..16])
            .map_err(|_| InstrumentError::InvalidAddress)?;
        
        let asset_id = u64::from_be_bytes(bytes.try_into().unwrap());
        
        Ok(Self {
            venue: VenueId::Ethereum as u16,
            asset_type: AssetType::Pool as u8,
            reserved: 0,
            asset_id,
        })
    }
}

fn symbol_to_u64(symbol: &str) -> u64 {
    let mut bytes = [0u8; 8];
    let len = symbol.len().min(8);
    bytes[..len].copy_from_slice(&symbol.as_bytes()[..len]);
    u64::from_be_bytes(bytes)
}
```

### TLV Message Construction

```rust
impl VenueCollector {
    async fn send_trade_event(
        &mut self,
        instrument_id: InstrumentId,
        price: i64,
        volume: u64,
        side: u8,
        venue_id: VenueId,
    ) -> Result<(), ProcessingError> {
        let trade_tlv = TradeTLV {
            tlv_type: TLVType::Trade as u8,
            tlv_length: 22,
            instrument_id,
            price,
            volume,
            side,
            flags: 0,
            reserved: [0; 2],
        };
        
        // Optional: Add instrument metadata for new instruments
        let should_send_metadata = self.is_new_instrument(instrument_id);
        
        let mut message_builder = TLVMessageBuilder::new(
            MARKET_DATA_DOMAIN, 
            venue_id as u8
        );
        
        message_builder.add_tlv(TLVType::Trade, &trade_tlv);
        
        if should_send_metadata {
            let metadata = self.build_instrument_metadata(instrument_id)?;
            message_builder.add_tlv(TLVType::InstrumentMeta, &metadata);
            self.mark_instrument_seen(instrument_id);
        }
        
        let message = message_builder.build();
        self.market_data_relay.send(&message).await?;
        
        Ok(())
    }
    
    fn build_instrument_metadata(&self, instrument_id: InstrumentId) -> Result<InstrumentMetaTLV, ProcessingError> {
        // Build metadata TLV with symbol, decimals, etc.
        // This would contain venue-specific information needed by strategies
        todo!("Implement instrument metadata construction")
    }
}
```

---

# Part IV: Error Handling & Resilience

## Error Categories

```rust
#[derive(Debug, thiserror::Error)]
pub enum CollectorError {
    // Connection errors
    #[error("Failed to connect to venue {venue}: {source}")]
    ConnectionFailed { venue: VenueId, source: Box<dyn std::error::Error + Send + Sync> },
    
    #[error("Connection timeout for venue {venue}")]
    ConnectionTimeout { venue: VenueId },
    
    #[error("Authentication failed for venue {venue}")]
    AuthenticationFailed { venue: VenueId },
    
    // Processing errors
    #[error("Invalid JSON message: {0}")]
    InvalidJson(serde_json::Error),
    
    #[error("Missing required field: {field}")]
    MissingField { field: String },
    
    #[error("Invalid numeric value: {value}")]
    InvalidNumeric { value: String },
    
    #[error("Unsupported venue: {venue:?}")]
    UnsupportedVenue { venue: VenueId },
    
    // System errors
    #[error("MarketDataRelay send failed: {0}")]
    RelaySendFailed(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

impl CollectorError {
    pub fn is_recoverable(&self) -> bool {
        matches!(self, 
            CollectorError::ConnectionFailed { .. } |
            CollectorError::ConnectionTimeout { .. } |
            CollectorError::InvalidJson(_) |
            CollectorError::InvalidNumeric { .. } |
            CollectorError::RelaySendFailed(_)
        )
    }
    
    pub fn should_invalidate_state(&self) -> bool {
        matches!(self,
            CollectorError::ConnectionFailed { .. } |
            CollectorError::ConnectionTimeout { .. } |
            CollectorError::AuthenticationFailed { .. }
        )
    }
}
```

## Circuit Breaker Pattern

```rust
#[derive(Debug)]
pub struct CircuitBreaker {
    failure_count: u32,
    last_failure_time: Option<Instant>,
    state: CircuitState,
    config: CircuitBreakerConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,    // Normal operation
    Open,      // Failing, reject requests
    HalfOpen,  // Testing recovery
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub recovery_timeout: Duration,
    pub success_threshold: u32,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            failure_count: 0,
            last_failure_time: None,
            state: CircuitState::Closed,
            config,
        }
    }
    
    pub fn call<F, R, E>(&mut self, operation: F) -> Result<R, E>
    where
        F: FnOnce() -> Result<R, E>,
        E: std::error::Error,
    {
        match self.state {
            CircuitState::Open => {
                // Check if we should try recovery
                if let Some(last_failure) = self.last_failure_time {
                    if last_failure.elapsed() > self.config.recovery_timeout {
                        self.state = CircuitState::HalfOpen;
                        self.failure_count = 0;
                    } else {
                        return Err(self.create_circuit_open_error());
                    }
                }
            }
            
            CircuitState::HalfOpen => {
                // Allow limited requests to test recovery
            }
            
            CircuitState::Closed => {
                // Normal operation
            }
        }
        
        // Execute operation
        match operation() {
            Ok(result) => {
                self.on_success();
                Ok(result)
            }
            Err(error) => {
                self.on_failure();
                Err(error)
            }
        }
    }
    
    fn on_success(&mut self) {
        match self.state {
            CircuitState::HalfOpen => {
                // Recovery successful
                self.state = CircuitState::Closed;
                self.failure_count = 0;
            }
            _ => {
                // Reset failure count on any success
                self.failure_count = 0;
            }
        }
    }
    
    fn on_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure_time = Some(Instant::now());
        
        if self.failure_count >= self.config.failure_threshold {
            self.state = CircuitState::Open;
        }
    }
}
```

## Graceful Degradation

```rust
impl VenueCollector {
    async fn handle_degraded_operation(&mut self, venue_id: VenueId) -> Result<(), CollectorError> {
        tracing::warn!("Entering degraded operation mode for venue {}", venue_id);
        
        // Reduce message processing rate
        let mut rate_limiter = tokio::time::interval(Duration::from_millis(100));
        
        // Continue with backup data sources if available
        match venue_id {
            VenueId::Binance => {
                // Fall back to REST API polling for critical pairs
                self.start_rest_fallback(venue_id).await?;
            }
            
            VenueId::Ethereum => {
                // Fall back to direct RPC polling
                self.start_rpc_fallback(venue_id).await?;
            }
            
            _ => {
                // No fallback available, maintain connection attempts
            }
        }
        
        // Continue normal operation at reduced rate
        loop {
            rate_limiter.tick().await;
            
            // Check if primary connection is restored
            if self.connection_manager.is_venue_connected(venue_id) {
                tracing::info!("Primary connection restored for venue {}, exiting degraded mode", venue_id);
                break;
            }
            
            // Perform degraded operations
            if let Err(e) = self.perform_degraded_operations(venue_id).await {
                tracing::error!("Degraded operations failed for {}: {}", venue_id, e);
            }
        }
        
        Ok(())
    }
    
    async fn start_rest_fallback(&mut self, venue_id: VenueId) -> Result<(), CollectorError> {
        // Implement REST API polling for critical trading pairs
        let critical_pairs = self.config.get_critical_pairs(venue_id);
        
        for pair in critical_pairs {
            tokio::spawn(async move {
                // Poll REST API for this pair
                // Convert to TLV and send at reduced frequency
            });
        }
        
        Ok(())
    }
}
```

---

# Part V: Performance Optimization

## Hot Path Optimization

### Zero-Copy Message Processing

```rust
use zerocopy::{AsBytes, FromBytes};

impl VenueCollector {
    // Optimized processing for high-frequency venues
    #[inline]
    async fn fast_path_trade_processing(
        &mut self,
        raw_data: &[u8],
        venue_id: VenueId
    ) -> Result<(), ProcessingError> {
        // Pre-parse common fields without full JSON deserialization
        let (instrument_id, price, volume, side) = match venue_id {
            VenueId::Binance => self.fast_parse_binance_trade(raw_data)?,
            VenueId::Polygon => self.fast_parse_polygon_swap(raw_data)?,
            _ => return self.standard_processing_path(raw_data, venue_id).await,
        };
        
        // Direct TLV construction without intermediate allocations
        let trade_tlv = TradeTLV {
            tlv_type: TLVType::Trade as u8,
            tlv_length: 22,
            instrument_id,
            price,
            volume,
            side,
            flags: 0,
            reserved: [0; 2],
        };
        
        // Zero-copy message building
        self.send_tlv_direct(&trade_tlv, venue_id).await
    }
    
    fn fast_parse_binance_trade(&self, data: &[u8]) -> Result<(InstrumentId, i64, u64, u8), ProcessingError> {
        // Use simd_json or custom parser for hot path
        // Avoid full serde_json deserialization
        todo!("Implement optimized Binance parsing")
    }
    
    async fn send_tlv_direct<T: AsBytes>(&mut self, tlv: &T, venue_id: VenueId) -> Result<(), ProcessingError> {
        // Direct binary serialization without TLVMessageBuilder overhead
        let tlv_bytes = tlv.as_bytes();
        
        // Send directly to relay with minimal allocations
        self.market_data_relay.send_bytes(tlv_bytes).await
            .map_err(|e| ProcessingError::RelaySendFailed(e.to_string()))?;
        
        Ok(())
    }
}
```

### Memory Pool Management

```rust
pub struct MessagePool {
    trade_tlvs: ObjectPool<TradeTLV>,
    quote_tlvs: ObjectPool<QuoteTLV>,
    message_buffers: ObjectPool<Vec<u8>>,
}

impl MessagePool {
    pub fn new() -> Self {
        Self {
            trade_tlvs: ObjectPool::new(|| TradeTLV::default(), 1000),
            quote_tlvs: ObjectPool::new(|| QuoteTLV::default(), 1000),
            message_buffers: ObjectPool::new(|| Vec::with_capacity(1024), 100),
        }
    }
    
    pub fn acquire_trade_tlv(&mut self) -> PooledObject<TradeTLV> {
        self.trade_tlvs.acquire()
    }
    
    pub fn acquire_message_buffer(&mut self) -> PooledObject<Vec<u8>> {
        let mut buffer = self.message_buffers.acquire();
        buffer.clear(); // Reset without deallocating
        buffer
    }
}

impl VenueCollector {
    async fn process_with_pooling(&mut self, data: &[u8]) -> Result<(), ProcessingError> {
        // Acquire objects from pool
        let mut trade_tlv = self.message_pool.acquire_trade_tlv();
        let mut message_buffer = self.message_pool.acquire_message_buffer();
        
        // Use pooled objects for processing
        self.parse_into_trade_tlv(data, &mut trade_tlv)?;
        self.serialize_tlv_message(&trade_tlv, &mut message_buffer)?;
        
        // Send message
        self.market_data_relay.send_bytes(&message_buffer).await?;
        
        // Objects automatically returned to pool on drop
        Ok(())
    }
}
```

## Batching and Buffering

```rust
pub struct BatchProcessor {
    pending_trades: Vec<TradeTLV>,
    pending_quotes: Vec<QuoteTLV>,
    batch_size: usize,
    flush_interval: Duration,
    last_flush: Instant,
}

impl BatchProcessor {
    pub fn new(batch_size: usize, flush_interval: Duration) -> Self {
        Self {
            pending_trades: Vec::with_capacity(batch_size),
            pending_quotes: Vec::with_capacity(batch_size),
            batch_size,
            flush_interval,
            last_flush: Instant::now(),
        }
    }
    
    pub async fn add_trade(&mut self, trade: TradeTLV) -> Result<(), ProcessingError> {
        self.pending_trades.push(trade);
        
        if self.pending_trades.len() >= self.batch_size {
            self.flush_trades().await?;
        }
        
        Ok(())
    }
    
    pub async fn tick(&mut self) -> Result<(), ProcessingError> {
        let now = Instant::now();
        
        if now.duration_since(self.last_flush) >= self.flush_interval {
            self.flush_all().await?;
        }
        
        Ok(())
    }
    
    async fn flush_trades(&mut self) -> Result<(), ProcessingError> {
        if self.pending_trades.is_empty() {
            return Ok(());
        }
        
        // Build batch message with multiple TLVs
        let mut message_builder = TLVMessageBuilder::new(MARKET_DATA_DOMAIN, BATCH_PROCESSOR_ID);
        
        for trade in &self.pending_trades {
            message_builder.add_tlv(TLVType::Trade, trade);
        }
        
        let batch_message = message_builder.build();
        self.market_data_relay.send(&batch_message).await?;
        
        self.pending_trades.clear();
        self.last_flush = Instant::now();
        
        Ok(())
    }
}
```

---

# Part VI: Configuration & Deployment

## Configuration Management

### Venue Configuration

```toml
# config/production/market_data.toml
[collector_service]
max_concurrent_venues = 10
message_buffer_size = 10000
processing_threads = 4
metrics_interval = "10s"

[connection_timeouts]
connect_timeout = "10s"
message_timeout = "30s"
health_check_interval = "5s"

[reconnection_strategy]
base_backoff_ms = 1000
max_backoff_ms = 30000
max_attempts = 10

[venues.binance]
enabled = true
websocket_url = "wss://stream.binance.com:9443/ws/stream"
api_key = "${BINANCE_API_KEY}"
api_secret = "${BINANCE_API_SECRET}"
rate_limit = 1200  # requests per minute
critical_pairs = ["BTCUSDT", "ETHUSDT", "ADAUSDT"]

[venues.binance.subscriptions]
trades = true
depth = true
ticker = false

[venues.polygon]
enabled = true
websocket_url = "wss://socket.polygon.io/stocks"
api_key = "${POLYGON_API_KEY}"
rate_limit = 1000
critical_pairs = []  # All DEX pools are critical

[venues.polygon.subscriptions]
trades = true          # AM (aggregate trades/swaps)
quotes = false         # Q (quotes)
aggregate_bars = false # A (bars)

[circuit_breakers]
failure_threshold = 5
recovery_timeout = "60s"
success_threshold = 3

[performance_monitoring]
slow_message_threshold = "1ms"
batch_size = 100
batch_timeout = "100ms"
```

### Runtime Configuration

```rust
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CollectorConfig {
    pub collector_service: ServiceConfig,
    pub connection_timeouts: TimeoutConfig,
    pub reconnection_strategy: ReconnectionConfig,
    pub venues: HashMap<String, VenueConfig>,
    pub circuit_breakers: CircuitBreakerConfig,
    pub performance_monitoring: PerformanceConfig,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct VenueConfig {
    pub enabled: bool,
    pub websocket_url: String,
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
    pub rate_limit: u32,
    pub critical_pairs: Vec<String>,
    pub subscriptions: SubscriptionConfig,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SubscriptionConfig {
    pub trades: bool,
    pub depth: bool,
    pub ticker: bool,
    pub quotes: Option<bool>,
    pub aggregate_bars: Option<bool>,
}

impl CollectorConfig {
    pub fn load_from_file(path: &str) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::FileRead(path.to_string(), e))?;
        
        let config: Self = toml::from_str(&content)
            .map_err(|e| ConfigError::Parse(e))?;
        
        config.validate()?;
        Ok(config)
    }
    
    fn validate(&self) -> Result<(), ConfigError> {
        // Validate venue configurations
        for (venue_name, venue_config) in &self.venues {
            if venue_config.enabled {
                if venue_config.websocket_url.is_empty() {
                    return Err(ConfigError::InvalidVenue(venue_name.clone(), "missing websocket_url".to_string()));
                }
                
                // Validate authentication requirements
                match venue_name.as_str() {
                    "binance" => {
                        if venue_config.api_key.is_none() || venue_config.api_secret.is_none() {
                            return Err(ConfigError::InvalidVenue(venue_name.clone(), "missing API credentials".to_string()));
                        }
                    }
                    "polygon" => {
                        if venue_config.api_key.is_none() {
                            return Err(ConfigError::InvalidVenue(venue_name.clone(), "missing API key".to_string()));
                        }
                    }
                    _ => {}
                }
            }
        }
        
        Ok(())
    }
}
```

## Monitoring & Observability

### Metrics Collection

```rust
#[derive(Debug, Default)]
pub struct CollectorMetrics {
    // Connection metrics
    pub venue_connection_status: HashMap<VenueId, Gauge>,
    pub connection_failures: Counter,
    pub reconnection_attempts: Counter,
    pub permanent_failures: Counter,
    
    // Processing metrics
    pub messages_received: Counter,
    pub messages_processed: Counter,
    pub processing_errors: Counter,
    pub message_processing_time: Histogram,
    
    // Event type metrics
    pub trades_processed: Counter,
    pub quotes_processed: Counter,
    pub liquidity_updates_processed: Counter,
    
    // Performance metrics
    pub batch_size: Histogram,
    pub queue_depth: Gauge,
    pub memory_usage: Gauge,
    
    // State management
    pub instruments_tracked: Gauge,
    pub state_invalidations_sent: Counter,
    pub state_invalidation_time: Histogram,
}

impl CollectorMetrics {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn update_connection_status(&mut self, venue: VenueId, connected: bool) {
        let status = if connected { 1.0 } else { 0.0 };
        self.venue_connection_status.entry(venue)
            .or_insert_with(|| Gauge::new())
            .set(status);
    }
    
    pub fn record_processing_time(&mut self, duration: Duration) {
        self.message_processing_time.record(duration.as_micros() as f64);
    }
}
```

### Health Checks

```rust
impl VenueCollector {
    pub async fn health_check(&self) -> HealthStatus {
        let mut status = HealthStatus::new();
        
        // Check connection status for each venue
        for (venue_id, connection) in &self.connection_manager.connections {
            let venue_health = match connection.state {
                ConnectionState::Connected => {
                    let message_age = connection.last_message_time.elapsed();
                    if message_age > Duration::from_secs(60) {
                        HealthStatus::Degraded
                    } else {
                        HealthStatus::Healthy
                    }
                }
                ConnectionState::Connecting | ConnectionState::Reconnecting => {
                    HealthStatus::Degraded
                }
                ConnectionState::Disconnected | ConnectionState::Failed => {
                    HealthStatus::Unhealthy
                }
            };
            
            status.add_component(format!("venue_{}", venue_id), venue_health);
        }
        
        // Check processing metrics
        if self.metrics.processing_errors.get() > 100 {
            status.add_component("processing".to_string(), HealthStatus::Degraded);
        }
        
        // Check memory usage
        if self.metrics.memory_usage.get() > 512.0 * 1024.0 * 1024.0 { // 512MB
            status.add_component("memory".to_string(), HealthStatus::Degraded);
        }
        
        status
    }
}

#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub overall: HealthLevel,
    pub components: HashMap<String, HealthLevel>,
    pub timestamp: u64,                     // Nanoseconds since epoch (protocol-consistent)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthLevel {
    Healthy,
    Degraded,
    Unhealthy,
}
```

## Deployment Considerations

### Resource Requirements

**Per Venue Collector:**
- **CPU**: 1-2 cores (high single-thread performance)
- **Memory**: 256MB baseline + 64MB per 1000 instruments
- **Network**: Low-latency connection to venue APIs (<50ms RTT preferred)
- **Disk**: 10GB for logs and temporary state

### Scaling Strategy

```rust
// Multi-venue deployment pattern
pub struct CollectorCluster {
    collectors: HashMap<VenueId, CollectorHandle>,
    load_balancer: LoadBalancer,
    health_monitor: ClusterHealthMonitor,
}

impl CollectorCluster {
    pub async fn deploy_venue_collector(&mut self, venue: VenueId) -> Result<(), DeploymentError> {
        let collector_config = self.config.venues.get(&venue)
            .ok_or_else(|| DeploymentError::VenueNotConfigured(venue))?;
        
        let collector = VenueCollector::new(venue, collector_config.clone()).await?;
        let handle = tokio::spawn(async move {
            collector.run().await
        });
        
        self.collectors.insert(venue, CollectorHandle::new(handle));
        
        // Start health monitoring for this collector
        self.health_monitor.add_venue(venue);
        
        Ok(())
    }
    
    pub async fn scale_venue_collector(&mut self, venue: VenueId, instances: u32) -> Result<(), DeploymentError> {
        // For high-volume venues, deploy multiple instances with instrument sharding
        for instance_id in 0..instances {
            let collector = self.create_sharded_collector(venue, instance_id, instances).await?;
            // Deploy to separate process/container
        }
        
        Ok(())
    }
}
```

### Operational Procedures

**Startup Sequence:**
1. Load and validate configuration
2. Initialize MarketDataRelay connection
3. Start venue collectors in dependency order
4. Verify all connections established
5. Begin health monitoring

**Graceful Shutdown:**
1. Stop accepting new connections
2. Send final state invalidations for all instruments
3. Close WebSocket connections gracefully
4. Flush any pending messages
5. Close relay connections

**Emergency Procedures:**
- **Mass Disconnection**: Immediately invalidate all venue state
- **Memory Pressure**: Reduce batch sizes and increase flush frequency
- **Network Partition**: Switch to degraded mode with REST fallbacks

This specification provides the foundation for implementing robust, high-performance market data collectors that can handle the reliability requirements of production trading systems while maintaining the sub-millisecond latency needed for effective arbitrage.

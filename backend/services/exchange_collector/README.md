# AlphaPulse Connector Development Guide

## Overview

AlphaPulse uses a **high-performance, zero-copy protocol** for streaming financial market data. This guide covers how to develop new exchange/data source connectors that integrate with our relay server, symbol hashing system, and Unix socket architecture.

## Architecture

```
Data Source → Exchange Collector → Unix Socket → Relay Server → WebSocket Bridge → Frontend
```

### Key Components

1. **Exchange Collector**: Connects to external data sources (exchanges, APIs, DeFi protocols)
2. **Protocol Layer**: Zero-copy binary messages with deterministic symbol hashing
3. **Unix Sockets**: Low-latency IPC between services
4. **Relay Server**: Central message routing and caching
5. **WebSocket Bridge**: Real-time data streaming to frontend

## Protocol Fundamentals

### 1. Symbol Hashing System

All symbols are converted to **deterministic 64-bit hashes** using `SymbolDescriptor`:

```rust
use alphapulse_protocol::SymbolDescriptor;

// Crypto pair
let btc_symbol = SymbolDescriptor::spot("coinbase", "BTC", "USD");
let hash = btc_symbol.hash(); // Deterministic u64

// Stock
let aapl_symbol = SymbolDescriptor::stock("alpaca", "AAPL");
let hash = aapl_symbol.hash();

// Option
let spy_call = SymbolDescriptor::option("alpaca", "SPY", 20250117, 600.0, 'C');
let hash = spy_call.hash();
```

### 2. Message Types

Our protocol supports these message types:

- **Trade**: Individual trade executions
- **L2Snapshot**: Full orderbook snapshot
- **L2Delta**: Incremental orderbook updates
- **SymbolMapping**: Hash-to-string mappings
- **Heartbeat**: Keep-alive messages

### 3. Message Structure

All messages follow this structure:

```
MessageHeader (8 bytes) + Payload (variable)
```

**MessageHeader**:
- Magic byte (0xFE)
- Message type (1 byte)
- Flags (1 byte)
- Payload length (2 bytes)
- Sequence number (3 bytes)

## Creating a New Connector

### Step 1: Add Exchange Module

Create a new file in `src/exchanges/your_exchange.rs`:

```rust
use crate::unix_socket::UnixSocketWriter;
use alphapulse_protocol::*;
use anyhow::{Context, Result};
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info};

pub struct YourExchangeCollector {
    socket_writer: Arc<UnixSocketWriter>,
    symbol_cache: Arc<RwLock<std::collections::HashMap<String, u64>>>,
}

impl YourExchangeCollector {
    pub fn new(socket_writer: Arc<UnixSocketWriter>) -> Self {
        Self {
            socket_writer,
            symbol_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    pub async fn start(&self) -> Result<()> {
        info!("Starting YourExchange collector");
        // Implementation here
        Ok(())
    }
}
```

### Step 2: Register in Module System

Update `src/exchanges/mod.rs`:

```rust
pub mod coinbase;
pub mod kraken;
pub mod alpaca;
pub mod your_exchange; // Add this line

pub use coinbase::CoinbaseCollector;
pub use kraken::KrakenCollector;
pub use alpaca::AlpacaCollector;
pub use your_exchange::YourExchangeCollector; // Add this line
```

### Step 3: Add to Main Binary

Update `src/main.rs` to include your exchange:

```rust
match exchange.as_str() {
    "coinbase" => {
        let collector = CoinbaseCollector::new(socket_writer);
        collector.start().await
    }
    "kraken" => {
        let collector = KrakenCollector::new(socket_writer);
        collector.start().await
    }
    "alpaca" => {
        let collector = AlpacaCollector::new(socket_writer);
        collector.start().await
    }
    "your_exchange" => { // Add this case
        let collector = YourExchangeCollector::new(socket_writer);
        collector.start().await
    }
    _ => {
        error!("Unknown exchange: {}", exchange);
        std::process::exit(1);
    }
}
```

## Implementation Patterns

### Pattern 1: WebSocket-Based Exchange

For exchanges with WebSocket APIs (most crypto exchanges):

```rust
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};

impl YourExchangeCollector {
    async fn connect_websocket(&self) -> Result<()> {
        let (ws_stream, _) = connect_async("wss://api.yourexchange.com/ws").await?;
        let (mut write, mut read) = ws_stream.split();

        // Subscribe to channels
        let subscribe_msg = serde_json::json!({
            "method": "subscribe",
            "channels": ["trades", "orderbook"]
        });
        write.send(Message::Text(subscribe_msg.to_string())).await?;

        // Process incoming messages
        while let Some(msg) = read.next().await {
            match msg? {
                Message::Text(text) => {
                    self.handle_message(&text).await?;
                }
                _ => {}
            }
        }
        
        Ok(())
    }

    async fn handle_message(&self, text: &str) -> Result<()> {
        let value: serde_json::Value = serde_json::from_str(text)?;
        
        match value["type"].as_str() {
            Some("trade") => self.handle_trade(value).await?,
            Some("orderbook") => self.handle_orderbook(value).await?,
            _ => {}
        }
        
        Ok(())
    }
}
```

### Pattern 2: REST API Polling

For APIs that require polling:

```rust
impl YourExchangeCollector {
    async fn poll_data(&self) -> Result<()> {
        let client = reqwest::Client::new();
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(100));

        loop {
            interval.tick().await;
            
            // Fetch trades
            let response = client
                .get("https://api.yourexchange.com/trades")
                .send()
                .await?;
            
            let trades: Vec<TradeData> = response.json().await?;
            
            for trade in trades {
                self.process_trade(trade).await?;
            }
        }
    }
}
```

### Pattern 3: DeFi/Blockchain Integration

For blockchain-based data sources:

```rust
use ethers::{providers::{Provider, Ws}, types::*};

impl YourDeFiCollector {
    async fn monitor_blockchain(&self) -> Result<()> {
        let provider = Provider::<Ws>::connect("wss://polygon-mainnet.g.alchemy.com/v2/API_KEY").await?;
        
        // Subscribe to new blocks
        let mut stream = provider.subscribe_blocks().await?;
        
        while let Some(block) = stream.next().await {
            self.process_block(block).await?;
        }
        
        Ok(())
    }

    async fn process_block(&self, block: Block<H256>) -> Result<()> {
        // Extract DEX events, price changes, etc.
        Ok(())
    }
}
```

## Message Creation and Sending

### Sending Trade Data

```rust
async fn send_trade(&self, price: f64, volume: f64, side: &str, symbol: &str) -> Result<()> {
    // Get or create symbol hash
    let symbol_hash = self.get_symbol_hash(symbol).await;
    
    // Convert to fixed-point
    let price_fp = (price * 1e8) as u64;
    let volume_fp = (volume * 1e8) as u64;
    
    // Create trade message
    let trade_side = match side {
        "buy" => TradeSide::Buy,
        "sell" => TradeSide::Sell,
        _ => TradeSide::Unknown,
    };
    
    let timestamp_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    
    let trade = TradeMessage::new(
        timestamp_ns,
        price_fp,
        volume_fp,
        symbol_hash,
        trade_side,
    );
    
    // Send via Unix socket
    self.socket_writer.send_trade(trade).await?;
    
    Ok(())
}
```

### Sending L2 Snapshot

```rust
async fn send_l2_snapshot(&self, symbol: &str, bids: Vec<(f64, f64)>, asks: Vec<(f64, f64)>) -> Result<()> {
    let symbol_hash = self.get_symbol_hash(symbol).await;
    
    let bid_levels: Vec<PriceLevel> = bids.iter()
        .map(|(price, volume)| {
            PriceLevel::new(
                (*price * 1e8) as u64,
                (*volume * 1e8) as u64,
            )
        })
        .collect();
    
    let ask_levels: Vec<PriceLevel> = asks.iter()
        .map(|(price, volume)| {
            PriceLevel::new(
                (*price * 1e8) as u64,
                (*volume * 1e8) as u64,
            )
        })
        .collect();
    
    let snapshot = L2SnapshotMessage {
        timestamp_ns: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64,
        symbol_hash,
        sequence: self.get_next_sequence(),
        bids: bid_levels,
        asks: ask_levels,
    };
    
    self.socket_writer.send_l2_snapshot(snapshot).await?;
    
    Ok(())
}
```

### Symbol Hash Management

```rust
async fn get_symbol_hash(&self, raw_symbol: &str) -> u64 {
    // Check cache first
    {
        let cache = self.symbol_cache.read();
        if let Some(&hash) = cache.get(raw_symbol) {
            return hash;
        }
    }
    
    // Create new symbol descriptor
    let descriptor = match self.parse_symbol(raw_symbol) {
        Some(desc) => desc,
        None => {
            error!("Failed to parse symbol: {}", raw_symbol);
            return 0;
        }
    };
    
    let hash = descriptor.hash();
    
    // Cache the mapping
    {
        let mut cache = self.symbol_cache.write();
        cache.insert(raw_symbol.to_string(), hash);
    }
    
    // Send symbol mapping to relay
    let mapping = SymbolMappingMessage::new(&descriptor);
    self.socket_writer.send_symbol_mapping(mapping).await.ok();
    
    hash
}

fn parse_symbol(&self, raw: &str) -> Option<SymbolDescriptor> {
    // Exchange-specific parsing logic
    // Examples:
    
    // Crypto: "BTC-USD" -> SymbolDescriptor::spot("yourexchange", "BTC", "USD")
    if raw.contains('-') {
        let parts: Vec<&str> = raw.split('-').collect();
        if parts.len() == 2 {
            return Some(SymbolDescriptor::spot("yourexchange", parts[0], parts[1]));
        }
    }
    
    // Stock: "AAPL" -> SymbolDescriptor::stock("yourexchange", "AAPL")
    if raw.chars().all(|c| c.is_alphabetic()) {
        return Some(SymbolDescriptor::stock("yourexchange", raw));
    }
    
    None
}
```

## Unix Socket Integration

The `UnixSocketWriter` handles the low-level communication:

```rust
// This is provided by the framework
impl UnixSocketWriter {
    pub async fn send_trade(&self, trade: TradeMessage) -> Result<()> {
        let header = MessageHeader::new(MessageType::Trade, TradeMessage::SIZE as u16, self.next_sequence());
        let mut buffer = Vec::new();
        buffer.extend_from_slice(header.as_bytes());
        buffer.extend_from_slice(trade.as_bytes());
        self.write_all(&buffer).await
    }
    
    pub async fn send_l2_snapshot(&self, snapshot: L2SnapshotMessage) -> Result<()> {
        let mut buffer = Vec::new();
        snapshot.encode(&mut buffer);
        let header = MessageHeader::new(MessageType::L2Snapshot, buffer.len() as u16, self.next_sequence());
        
        let mut message = Vec::new();
        message.extend_from_slice(header.as_bytes());
        message.extend_from_slice(&buffer);
        self.write_all(&message).await
    }
}
```

## Error Handling and Resilience

### Connection Management

```rust
impl YourExchangeCollector {
    async fn run_with_retry(&self) -> Result<()> {
        let mut retry_count = 0;
        const MAX_RETRIES: u32 = 10;
        
        loop {
            match self.connect_and_run().await {
                Ok(_) => {
                    info!("Connection ended normally");
                    break;
                }
                Err(e) => {
                    retry_count += 1;
                    error!("Connection failed (attempt {}): {}", retry_count, e);
                    
                    if retry_count >= MAX_RETRIES {
                        return Err(e.context("Max retries exceeded"));
                    }
                    
                    let delay = std::cmp::min(1000 * 2_u64.pow(retry_count), 30000);
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                }
            }
        }
        
        Ok(())
    }
}
```

### Heartbeat Implementation

```rust
async fn send_heartbeat(&self) -> Result<()> {
    let heartbeat = HeartbeatMessage::new(
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64,
        self.next_sequence(),
    );
    
    self.socket_writer.send_heartbeat(heartbeat).await
}

// Send heartbeats every 30 seconds
async fn heartbeat_loop(&self) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
    
    loop {
        interval.tick().await;
        if let Err(e) = self.send_heartbeat().await {
            error!("Failed to send heartbeat: {}", e);
        }
    }
}
```

## Testing Your Connector

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::UnixListener;
    
    #[tokio::test]
    async fn test_symbol_parsing() {
        let collector = YourExchangeCollector::new(/*...*/);
        
        let btc_desc = collector.parse_symbol("BTC-USD").unwrap();
        assert_eq!(btc_desc.exchange, "yourexchange");
        assert_eq!(btc_desc.base, "BTC");
        assert_eq!(btc_desc.quote, Some("USD".to_string()));
    }
    
    #[tokio::test]
    async fn test_message_encoding() {
        let trade = TradeMessage::new(12345, 6500000000000, 100000000, 42, TradeSide::Buy);
        assert_eq!(trade.price_f64(), 65000.0);
        assert_eq!(trade.volume_f64(), 1.0);
    }
}
```

### Integration Testing

```bash
# Test your connector
cargo run --bin exchange-collector -- --exchange your_exchange

# Check relay server logs
tail -f logs/relay-server.log

# Monitor Unix socket traffic
./scripts/debug_sockets.sh
```

## Performance Considerations

### Latency Optimization

1. **Zero-copy protocol**: All messages use `#[repr(C)]` structs
2. **Unix sockets**: Lower latency than TCP for local IPC
3. **Batch updates**: Group multiple L2 updates when possible
4. **Symbol caching**: Cache hash lookups to avoid recomputation

### Memory Management

```rust
// Reuse buffers to avoid allocations
struct BufferPool {
    buffers: Vec<Vec<u8>>,
}

impl BufferPool {
    fn get_buffer(&mut self) -> Vec<u8> {
        self.buffers.pop().unwrap_or_else(|| Vec::with_capacity(1024))
    }
    
    fn return_buffer(&mut self, mut buffer: Vec<u8>) {
        buffer.clear();
        if buffer.capacity() <= 4096 {
            self.buffers.push(buffer);
        }
    }
}
```

## Deployment

### Building

```bash
cd backend/services/exchange_collector
cargo build --release
```

### Running

```bash
# Direct execution
./target/release/exchange-collector --exchange your_exchange

# Via daemon manager
./scripts/daemon-manager.sh start your_exchange
```

### Monitoring

Check these log files:
- `logs/exchange-collector-your_exchange.log`
- `logs/exchange-collector-your_exchange.error.log`
- `logs/relay-server.log`

## Common Patterns by Data Source Type

### Centralized Exchanges (CEX)
- WebSocket connections
- Trade and orderbook streams
- Symbol normalization
- Rate limiting handling

### Decentralized Exchanges (DEX)
- Blockchain event monitoring
- Pool state tracking  
- Gas optimization
- MEV protection considerations

## DEX Pool Classification: Event Signature-Based Architecture

### Protocol Standardization Insight

Through analysis of Polygon DEX implementations, we discovered that **most DEXes share the same underlying protocols**, making DEX identification less important than event signature classification:

- **QuickSwap, SushiSwap, and many others** all use the **UniswapV2 protocol**
- **Same event signatures** = **same parsing logic**
- **DEX branding becomes optional metadata** rather than core functionality

### Event Signature Classification

Instead of expensive RPC `factory()` calls for DEX identification, we classify pools by their event signatures:

```rust
// Event signatures for different pool types
pub const UNISWAP_V2_SWAP_SIGNATURE: &str = "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822";
pub const UNISWAP_V3_SWAP_SIGNATURE: &str = "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67";
pub const CURVE_TOKEN_EXCHANGE_SIGNATURE: &str = "0x8b3e96f2b889fa771c53c981b40daf005f63f637f1869f707052d15a3dd97140";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventBasedPoolType {
    UniswapV2Style,  // Includes QuickSwap, SushiSwap, etc.
    UniswapV3Style,
    CurveStyle,
}
```

### Benefits of Event-Based Classification

1. **Eliminates Rate Limiting**: No more expensive RPC `factory()` calls
2. **Maximizes Data Coverage**: Accept all pools with known signatures
3. **Reduces Complexity**: Same parsing logic for protocol-compatible DEXes
4. **Improves Performance**: Event signature lookup vs contract inspection
5. **Future-Proof**: New DEXes using existing protocols work automatically

### Implementation Pattern

```rust
async fn process_swap_event(&self, event: &Value) -> Result<()> {
    // Extract event signature from topics[0]
    let event_signature = event["topics"][0].as_str()
        .ok_or_else(|| anyhow::anyhow!("No event signature in topics[0]"))?;
    
    // Classify pool type by event signature (not DEX name)
    let pool_type = self.pool_factory.classify_by_event_signature(event_signature)
        .ok_or_else(|| anyhow::anyhow!("Unknown event signature: {}", event_signature))?;
    
    // Create pool without expensive DEX identification
    let pool = self.pool_factory.create_pool_by_signature(pool_address, pool_type).await?;
    
    // Parse using protocol-specific logic
    let swap_event = pool.parse_swap_event(swap_data)?;
}
```

### Dashboard Integration

Since multiple DEXes share protocols, the **dashboard should display by pool address** rather than DEX names:

- **Pool-centric view**: `0x1f1e4c845183ef6d50e9609f16f6f9cae43bc1cb`
- **Protocol indicator**: `UniswapV2Style` 
- **Optional DEX metadata**: `QuickSwap` (when identifiable)

This approach provides **accurate arbitrage detection across all protocol-compatible pools** regardless of DEX branding.

### Traditional Markets
- REST API polling
- Market hours handling
- Corporate actions
- Reference data management

### Options/Derivatives
- Greeks calculation
- Expiry handling
- Strike normalization
- Volatility surface updates

## Conclusion

This framework provides a **high-performance foundation** for financial data collection with:

- **Sub-100μs latency** through optimized protocol design
- **Deterministic symbol hashing** for consistent data identification
- **Zero-copy message passing** for maximum throughput
- **Resilient connection management** with automatic retry logic

Follow these patterns to build connectors that integrate seamlessly with the AlphaPulse ecosystem while maintaining our performance standards.
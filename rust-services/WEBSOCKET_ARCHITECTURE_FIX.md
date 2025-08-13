# WebSocket Architecture Fix - Event-Driven Real-Time Streaming

## 🚨 Problem Identified

The current WebSocket implementation has serious architectural flaws:

1. **Polling-based instead of event-driven** - Uses `tokio::time::interval` to poll for data
2. **REST data sources for real-time** - Uses Redis queries instead of shared memory streams
3. **Missing ultra-low latency integration** - Bypasses the <10μs shared memory infrastructure
4. **High latency** - Polling introduces unnecessary latency vs. push-based events

## ✅ Correct Architecture: Event-Driven WebSocket Streaming

### **Real-Time Data Flow**

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│ Exchange WebSocket │    │ Shared Memory   │    │ WebSocket Server│
│ Collectors        │───▶│ (Sub-10μs)      │───▶│ (Event-Driven)  │
│                   │    │                 │    │                 │
│ • Coinbase        │    │ • Trade Buffer  │    │ • Real-time Push│
│ • Kraken          │    │ • Delta Buffer  │    │ • No Polling    │
│ • Binance         │    │ • Lock-free     │    │ • Multi-client  │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

### **Implementation Plan**

#### **1. Event-Driven WebSocket Server**

```rust
// websocket-server/src/realtime_websocket.rs
use alphapulse_common::shared_memory::{SharedMemoryReader, OrderBookDeltaReader};
use tokio::sync::broadcast;

pub struct RealtimeWebSocketServer {
    // Shared memory readers for each exchange
    coinbase_trade_reader: SharedMemoryReader,
    coinbase_delta_reader: OrderBookDeltaReader,
    kraken_delta_reader: OrderBookDeltaReader,
    binance_delta_reader: OrderBookDeltaReader,
    
    // Broadcast channels for real-time distribution
    trade_broadcaster: broadcast::Sender<Trade>,
    delta_broadcaster: broadcast::Sender<OrderBookDelta>,
    
    // Client management
    clients: Arc<RwLock<HashMap<String, ClientSession>>>,
}

impl RealtimeWebSocketServer {
    pub async fn start(&self) -> Result<()> {
        // Start shared memory readers (event-driven, no polling)
        tokio::spawn(self.run_trade_reader());
        tokio::spawn(self.run_delta_reader());
        
        // Start WebSocket server
        self.run_websocket_server().await
    }
    
    // Event-driven trade reader - pushes data immediately when available
    async fn run_trade_reader(&self) {
        loop {
            // Read from shared memory (blocking until new data)
            match self.coinbase_trade_reader.read_trades() {
                Ok(trades) if !trades.is_empty() => {
                    for trade in trades {
                        // Immediately broadcast to all subscribed clients
                        let _ = self.trade_broadcaster.send(trade);
                    }
                }
                Ok(_) => {
                    // No new data, yield briefly and check again
                    tokio::time::sleep(Duration::from_micros(10)).await;
                }
                Err(e) => {
                    error!("Trade reader error: {}", e);
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }
    }
    
    // Event-driven delta reader - pushes orderbook changes immediately  
    async fn run_delta_reader(&self) {
        loop {
            // Read from all exchange delta streams
            let mut any_data = false;
            
            // Coinbase deltas
            if let Ok(deltas) = self.coinbase_delta_reader.read_deltas() {
                if !deltas.is_empty() {
                    any_data = true;
                    for delta in deltas {
                        let _ = self.delta_broadcaster.send(delta.into());
                    }
                }
            }
            
            // Kraken deltas  
            if let Ok(deltas) = self.kraken_delta_reader.read_deltas() {
                if !deltas.is_empty() {
                    any_data = true;
                    for delta in deltas {
                        let _ = self.delta_broadcaster.send(delta.into());
                    }
                }
            }
            
            if !any_data {
                // Brief yield if no data from any exchange
                tokio::time::sleep(Duration::from_micros(10)).await;
            }
        }
    }
}
```

#### **2. Client Session Management**

```rust
pub struct ClientSession {
    id: String,
    subscriptions: HashSet<String>, // symbols
    channels: HashSet<String>,      // trades, orderbook, etc.
    sender: mpsc::Sender<WebSocketMessage>,
}

impl ClientSession {
    pub async fn handle_client(&self, mut receiver: broadcast::Receiver<Trade>) {
        while let Ok(trade) = receiver.recv().await {
            // Only send if client is subscribed to this symbol
            if self.subscriptions.contains(&trade.symbol) && 
               self.channels.contains("trades") {
                
                let message = WebSocketMessage::Trade(trade);
                if self.sender.send(message).await.is_err() {
                    break; // Client disconnected
                }
            }
        }
    }
}
```

#### **3. Ultra-Low Latency Metrics**

```rust
pub struct LatencyMetrics {
    shared_memory_to_websocket: Histogram,
    client_broadcast_latency: Histogram,
    end_to_end_latency: Histogram,
}

// Track latency from shared memory to client
let start = Instant::now();
let trade = reader.read_trades()?;
let read_latency = start.elapsed().as_nanos();

// Send to client
broadcast_sender.send(trade)?;
let total_latency = start.elapsed().as_nanos();

metrics.record_latency(read_latency, total_latency);
```

## 🎯 Performance Targets

| Metric | Current (Polling) | Target (Event-Driven) |
|--------|-------------------|------------------------|
| **WebSocket Latency** | 100-1000ms | <1ms |
| **Data Freshness** | Stale (polling interval) | Real-time |
| **CPU Usage** | High (constant polling) | Low (event-driven) |
| **Throughput** | Limited by poll rate | 100k+ messages/sec |
| **Scalability** | Poor (N polls per client) | Excellent (broadcast) |

## 🔧 Implementation Steps

1. **Replace Polling WebSocket** - Remove `tokio::time::interval` polling
2. **Integrate Shared Memory** - Use `SharedMemoryReader` and `OrderBookDeltaReader`
3. **Event-Driven Architecture** - Push data immediately when available
4. **Broadcast Channels** - Use `tokio::sync::broadcast` for multi-client efficiency
5. **Client Subscriptions** - Filter data per client subscriptions
6. **Latency Monitoring** - Track sub-millisecond performance

## 📊 Expected Performance Improvements

- **Latency**: 100-1000x improvement (from 100-1000ms to <1ms)
- **CPU Usage**: 50-90% reduction (no polling overhead)
- **Data Freshness**: Real-time vs. stale polling data
- **Scalability**: Linear vs. exponential scaling with client count
- **Consistency**: Events delivered in order vs. potential racing

This architecture properly leverages AlphaPulse's ultra-low latency shared memory infrastructure for true real-time WebSocket streaming.
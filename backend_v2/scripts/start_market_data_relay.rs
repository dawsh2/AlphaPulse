//! Simple MarketDataRelay service for live arbitrage pipeline
//! Creates Unix socket listener at /tmp/alphapulse/market_data.sock
//! Accepts connections from Polygon publisher and forwards to flash arbitrage strategy

use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{info, error, warn};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{RwLock, mpsc};

// Add trace event support for distributed tracing
use serde::{Serialize, Deserialize};

// Trace event types for observability (simplified version)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEvent {
    pub trace_id: [u8; 16],
    pub service: String,
    pub event_type: String,
    pub timestamp_ns: u64,
    pub duration_ns: Option<u64>,
    pub metadata: HashMap<String, String>,
}

// Simple trace ID type
type TraceId = [u8; 16];

type ConsumerId = String;
type MessageBuffer = Vec<u8>;

struct MarketDataRelay {
    consumers: Arc<RwLock<HashMap<ConsumerId, mpsc::UnboundedSender<MessageBuffer>>>>,
    message_count: Arc<RwLock<u64>>,
    // Trace socket for sending events to TraceCollector
    trace_socket: Arc<RwLock<Option<UnixStream>>>,
}

impl MarketDataRelay {
    fn new() -> Self {
        Self {
            consumers: Arc::new(RwLock::new(HashMap::new())),
            message_count: Arc::new(RwLock::new(0)),
            trace_socket: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Connect to TraceCollector for distributed tracing
    async fn connect_to_trace_collector(&self) -> Result<(), Box<dyn std::error::Error>> {
        const TRACE_SOCKET_PATH: &str = "/tmp/alphapulse/trace_collector.sock";
        
        match UnixStream::connect(TRACE_SOCKET_PATH).await {
            Ok(stream) => {
                *self.trace_socket.write().await = Some(stream);
                info!("📊 MarketDataRelay connected to TraceCollector");
                Ok(())
            }
            Err(e) => {
                warn!("⚠️ Failed to connect to TraceCollector: {} (traces will be skipped)", e);
                Ok(()) // Don't fail the relay if tracing is unavailable
            }
        }
    }
    
    /// Send trace event to TraceCollector
    async fn emit_trace_event(&self, event: TraceEvent) {
        let json_data = match serde_json::to_string(&event) {
            Ok(json) => json,
            Err(e) => {
                warn!("Failed to serialize trace event: {}", e);
                return;
            }
        };
        
        let mut socket_lock = self.trace_socket.write().await;
        if let Some(socket) = socket_lock.as_mut() {
            let message = format!("{}\n", json_data);
            if let Err(e) = socket.write_all(message.as_bytes()).await {
                warn!("Failed to send trace event: {}", e);
                *socket_lock = None; // Connection broken
            }
        }
    }
    
    /// Extract trace ID from TLV message (simplified - looks for trace patterns)
    fn extract_trace_id_from_message(&self, message: &[u8]) -> Option<TraceId> {
        // In a full implementation, this would parse the TLV message header
        // For now, we'll generate a pseudo trace ID based on message content
        if message.len() >= 16 {
            let mut trace_id = [0u8; 16];
            // Use first 16 bytes as pseudo trace ID (simplified)
            trace_id.copy_from_slice(&message[0..16]);
            Some(trace_id)
        } else {
            None
        }
    }
    
    /// Generate a new trace ID for relay-initiated events
    fn generate_relay_trace_id() -> TraceId {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        
        let mut trace_id = [0u8; 16];
        trace_id[0..8].copy_from_slice(&now.to_be_bytes());
        // Add relay marker in next 4 bytes
        trace_id[8..12].copy_from_slice(b"RELY");
        trace_id
    }
    
    async fn register_consumer(&self, consumer_id: ConsumerId, tx: mpsc::UnboundedSender<MessageBuffer>) {
        let mut consumers = self.consumers.write().await;
        consumers.insert(consumer_id.clone(), tx);
        info!("📡 Registered consumer: {}", consumer_id);
    }
    
    async fn broadcast_message(&self, message: MessageBuffer, source_conn: &str) -> usize {
        let start_time = std::time::Instant::now();
        
        // Extract or generate trace ID for this message flow
        let trace_id = self.extract_trace_id_from_message(&message)
            .unwrap_or_else(|| Self::generate_relay_trace_id());
        
        // Emit MessageReceived trace event
        self.emit_message_received_event(trace_id, &message, source_conn).await;
        
        let consumers = self.consumers.read().await;
        let mut sent_count = 0;
        
        for (consumer_id, tx) in consumers.iter() {
            if tx.send(message.clone()).is_ok() {
                sent_count += 1;
            } else {
                warn!("Failed to send to consumer: {}", consumer_id);
            }
        }
        
        let processing_duration = start_time.elapsed().as_nanos() as u64;
        
        // Emit MessageSent trace event for successful forwards
        self.emit_message_sent_event(trace_id, sent_count, processing_duration).await;
        
        // Update message count
        let mut count = self.message_count.write().await;
        *count += 1;
        
        if *count % 100 == 0 {
            info!("📊 MarketDataRelay processed {} messages, {} active consumers", 
                  *count, consumers.len());
        }
        
        sent_count
    }
    
    /// Emit trace event when message is received from publisher
    async fn emit_message_received_event(&self, trace_id: TraceId, message: &[u8], source: &str) {
        let event = TraceEvent {
            trace_id,
            service: "MarketDataRelay".to_string(),
            event_type: "MessageReceived".to_string(),
            timestamp_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64,
            duration_ns: None,
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("source".to_string(), source.to_string());
                meta.insert("message_size".to_string(), message.len().to_string());
                meta.insert("relay_domain".to_string(), "market_data".to_string());
                meta
            },
        };
        
        self.emit_trace_event(event).await;
    }
    
    /// Emit trace event when message is forwarded to consumers
    async fn emit_message_sent_event(&self, trace_id: TraceId, consumer_count: usize, processing_duration: u64) {
        let event = TraceEvent {
            trace_id,
            service: "MarketDataRelay".to_string(),
            event_type: "MessageSent".to_string(),
            timestamp_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64,
            duration_ns: Some(processing_duration),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("consumer_count".to_string(), consumer_count.to_string());
                meta.insert("destination".to_string(), "downstream_consumers".to_string());
                meta.insert("processing_stage".to_string(), "relay_forward".to_string());
                meta
            },
        };
        
        self.emit_trace_event(event).await;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    info!("🚀 Starting MarketDataRelay (Domain 1)");
    info!("   High-throughput relay for live Polygon DEX events");
    info!("   Unix socket: /tmp/alphapulse/market_data.sock");
    
    // Remove existing socket
    let socket_path = "/tmp/alphapulse/market_data.sock";
    if std::path::Path::new(socket_path).exists() {
        std::fs::remove_file(socket_path)?;
    }
    
    // Create market data relay
    let relay = Arc::new(MarketDataRelay::new());
    
    // Connect to TraceCollector for distributed tracing
    if let Err(e) = relay.connect_to_trace_collector().await {
        warn!("⚠️ TraceCollector connection failed: {} (traces will be disabled)", e);
    }
    
    // Create Unix socket listener
    let listener = UnixListener::bind(socket_path)?;
    info!("✅ MarketDataRelay listening for connections");
    
    // Accept and handle connections
    let mut connection_id = 0;
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                connection_id += 1;
                let conn_id = format!("conn_{}", connection_id);
                
                let relay_clone = Arc::clone(&relay);
                tokio::spawn(async move {
                    handle_connection(stream, conn_id, relay_clone).await;
                });
            }
            Err(e) => {
                error!("❌ Failed to accept connection: {}", e);
            }
        }
    }
}

async fn handle_connection(
    mut stream: UnixStream, 
    conn_id: String,
    relay: Arc<MarketDataRelay>
) {
    info!("📡 New connection: {}", conn_id);
    
    // Check if this is a publisher (sends data) or consumer (subscribes)
    let mut buffer = vec![0u8; 65536]; // 64KB buffer for TLV messages
    let mut is_publisher = false;
    let mut consumer_tx: Option<mpsc::UnboundedReceiver<MessageBuffer>> = None;
    
    loop {
        tokio::select! {
            // Handle incoming messages (from publishers)
            read_result = stream.read(&mut buffer) => {
                match read_result {
                    Ok(0) => {
                        info!("📡 Connection {} disconnected", conn_id);
                        break;
                    }
                    Ok(n) => {
                        if !is_publisher {
                            is_publisher = true;
                            info!("📤 {} identified as PUBLISHER (sending live TLV data)", conn_id);
                        }
                        
                        // Broadcast message to all consumers
                        let message = buffer[..n].to_vec();
                        let consumer_count = relay.broadcast_message(message, &conn_id).await;
                        
                        info!("📨 Relayed {} bytes from {} to {} consumers", 
                              n, conn_id, consumer_count);
                    }
                    Err(e) => {
                        error!("❌ Read error on {}: {}", conn_id, e);
                        break;
                    }
                }
            }
            
            // Handle consumer subscription requests
            message = async {
                if consumer_tx.is_none() {
                    // Create consumer channel on first subscription
                    let (tx, rx) = mpsc::unbounded_channel::<MessageBuffer>();
                    relay.register_consumer(conn_id.clone(), tx).await;
                    consumer_tx = Some(rx);
                    info!("📥 {} identified as CONSUMER (subscribing to TLV data)", conn_id);
                }
                
                consumer_tx.as_mut().unwrap().recv().await
            } => {
                match message {
                    Some(data) => {
                        // Send message to consumer
                        if let Err(e) = stream.write_all(&data).await {
                            warn!("❌ Failed to send to consumer {}: {}", conn_id, e);
                            break;
                        }
                    }
                    None => {
                        info!("📡 Consumer channel closed for {}", conn_id);
                        break;
                    }
                }
            }
        }
    }
    
    info!("📡 Connection {} handler terminated", conn_id);
}
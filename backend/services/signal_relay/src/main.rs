use alphapulse_protocol::{
    // Import only what relay needs - minimal dependencies
    MessageHeader, MESSAGE_MAGIC,
    message_protocol::MessageHeader as NewMessageHeader,
    // Relay should NOT process schemas - just forward bytes
};
// use zerocopy::AsBytes; // Unused
use anyhow::{Context, Result};
use crossbeam_channel::{bounded, Sender, Receiver};
use dashmap::DashMap;
use metrics::{counter, gauge, histogram};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::task;
use tracing::{debug, error, info, warn};
use thiserror::Error;
use serde::{Deserialize, Serialize};

/// Relay-specific error types
#[derive(Debug, Error)]
pub enum RelayError {
    #[error("Invalid message format: magic={magic:08x}")]
    InvalidMagic { magic: u32 },
    
    #[error("Checksum mismatch: expected={expected:08x}, actual={actual:08x}")]
    ChecksumMismatch { expected: u32, actual: u32 },
    
    #[error("Circuit breaker activated: queue size {size} exceeds threshold")]
    CircuitBreakerActive { size: usize },
    
    #[error("Message too small: {size} bytes, minimum {minimum} bytes")]
    MessageTooSmall { size: usize, minimum: usize },
    
    #[error("Socket error: {0}")]
    SocketError(#[from] std::io::Error),
    
    #[error("Channel error: {0}")]
    ChannelError(String),
}

/// Relay configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RelayConfig {
    pub bind_path: String,
    pub max_queue_size: usize,
    pub circuit_breaker_threshold: usize,
    pub validate_checksums: bool,
    pub exchanges: Vec<String>,
    pub message_types: Option<Vec<u8>>,  // Filter for specific message types
    pub relay_name: Option<String>,      // For logging identification
}

/// Transport abstraction for relay - allows Unix socket or shared memory
pub trait MessageTransport: Send + Sync {
    fn send(&self, data: &[u8]) -> Result<(), anyhow::Error>;
    fn receive(&self, buffer: &mut [u8]) -> Result<usize, anyhow::Error>;
    fn flush(&self) -> Result<(), anyhow::Error>;
}

/// Unix socket transport implementation
pub struct UnixSocketTransport {
    stream: UnixStream,
}

impl MessageTransport for UnixSocketTransport {
    fn send(&self, data: &[u8]) -> Result<(), anyhow::Error> {
        use std::io::Write;
        let mut stream = self.stream.try_clone()?;
        stream.write_all(data)?;
        Ok(())
    }
    
    fn receive(&self, buffer: &mut [u8]) -> Result<usize, anyhow::Error> {
        use std::io::Read;
        let mut stream = self.stream.try_clone()?;
        let n = stream.read(buffer)?;
        Ok(n)
    }
    
    fn flush(&self) -> Result<(), anyhow::Error> {
        use std::io::Write;
        let mut stream = self.stream.try_clone()?;
        stream.flush()?;
        Ok(())
    }
}

const DEFAULT_RELAY_BIND_PATH: &str = "/tmp/alphapulse/relay.sock";
const MARKET_DATA_RELAY_PATH: &str = "/tmp/alphapulse/market_data.sock";
const MAX_QUEUE_SIZE: usize = 100000;
const CIRCUIT_BREAKER_THRESHOLD: usize = 80000;

struct ExchangeSocket {
    name: String,
    path: String,
}

impl ExchangeSocket {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            path: format!("/tmp/alphapulse/{}.sock", name),
        }
    }
}

struct SequenceTracker {
    sequences: DashMap<u16, AtomicU32>,
    dropped_messages: AtomicU64,
}

impl SequenceTracker {
    fn new() -> Self {
        Self {
            sequences: DashMap::new(),
            dropped_messages: AtomicU64::new(0),
        }
    }

    fn validate_and_update(&self, exchange_id: u16, sequence: u32) -> bool {
        let entry = self.sequences.entry(exchange_id).or_insert(AtomicU32::new(0));
        let expected = entry.load(Ordering::Acquire) + 1;
        
        if sequence == expected || sequence == 0 {
            entry.store(sequence, Ordering::Release);
            true
        } else if sequence > expected {
            warn!(
                "Detected gap in sequence for exchange {}: expected {}, got {} (dropped {} messages)",
                exchange_id,
                expected,
                sequence,
                sequence - expected
            );
            self.dropped_messages.fetch_add((sequence - expected) as u64, Ordering::Relaxed);
            entry.store(sequence, Ordering::Release);
            counter!("relay.dropped_messages").increment((sequence - expected) as u64);
            false
        } else {
            warn!(
                "Out-of-order message for exchange {}: expected {}, got {}",
                exchange_id, expected, sequence
            );
            false
        }
    }
}

struct RelayServer {
    exchange_sockets: Vec<ExchangeSocket>,
    broadcast_sender: Sender<Vec<u8>>,
    broadcast_receiver: Receiver<Vec<u8>>,
    clients: Arc<DashMap<usize, UnixStream>>,
    sequence_tracker: Arc<SequenceTracker>,
    circuit_breaker_active: Arc<AtomicBool>,
    config: RelayConfig,
    // Relay is a dumb pipe - no schema processing needed
}

impl RelayServer {
    /// Check if message type should be relayed based on configuration
    fn should_relay_message(data: &[u8], allowed_types: &Option<Vec<u8>>) -> bool {
        // If no filter specified, relay all messages
        let Some(allowed) = allowed_types else {
            return true;
        };
        
        // Extract message type from header (byte offset 4)
        if data.len() < 5 {
            return false;
        }
        
        let message_type = data[4];
        allowed.contains(&message_type)
    }
    
    /// Validate message format and checksum
    fn validate_message(data: &[u8], validate_checksum: bool) -> Result<(), RelayError> {
        let header_size = std::mem::size_of::<MessageHeader>();
        
        if data.len() < header_size {
            return Err(RelayError::MessageTooSmall { 
                size: data.len(), 
                minimum: header_size 
            });
        }
        
        // Check magic number
        let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if magic != MESSAGE_MAGIC {
            return Err(RelayError::InvalidMagic { magic });
        }
        
        // Validate checksum if enabled
        if validate_checksum && data.len() >= header_size {
            // Parse header to get checksum
            if let Ok(header) = NewMessageHeader::from_bytes(&data[..header_size]) {
                // Calculate checksum (excluding the checksum field itself)
                // Checksum is last 4 bytes of header
                let checksum_offset = header_size - 4;
                let calculated = crc32fast::hash(&data[..checksum_offset]);
                
                if calculated != header.checksum {
                    return Err(RelayError::ChecksumMismatch {
                        expected: header.checksum,
                        actual: calculated,
                    });
                }
            }
        }
        
        Ok(())
    }
    
    /// Extract message size from header for proper framing
    /// IMPORTANT: Clients should use this same logic to parse messages from the relay!
    /// 1. Read at least size_of::<MessageHeader>() bytes
    /// 2. Parse header to get payload_size
    /// 3. Read exactly (header_size + payload_size) bytes for complete message
    /// 4. Process message, then repeat
    pub fn get_message_size(data: &[u8]) -> Option<usize> {
        if data.len() < std::mem::size_of::<MessageHeader>() {
            return None;
        }
        
        // Parse header to get payload size
        let header_bytes = &data[..std::mem::size_of::<MessageHeader>()];
        if let Ok(header) = NewMessageHeader::from_bytes(header_bytes) {
            Some(std::mem::size_of::<MessageHeader>() + header.payload_size as usize)
        } else {
            None
        }
    }

    fn new(config: RelayConfig) -> Self {
        let (tx, rx) = bounded(config.max_queue_size);
        
        let exchanges = config.exchanges.iter()
            .map(|name| ExchangeSocket::new(name))
            .collect();
        
        Self {
            exchange_sockets: exchanges,
            broadcast_sender: tx,
            broadcast_receiver: rx,
            clients: Arc::new(DashMap::new()),
            sequence_tracker: Arc::new(SequenceTracker::new()),
            circuit_breaker_active: Arc::new(AtomicBool::new(false)),
            config,
        }
    }

    async fn start(&self) -> Result<()> {
        self.ensure_directories().await?;
        self.cleanup_sockets().await?;
        
        // No shared memory initialization needed for Unix socket architecture
        
        let exchange_handles = self.spawn_exchange_listeners();
        
        let _relay_handle = self.spawn_relay_listener();
        
        let _broadcast_handle = self.spawn_broadcaster();
        
        let _monitor_handle = self.spawn_monitor();
        
        info!("Relay server started successfully");
        info!("Listening for exchanges on: {:?}", 
            self.exchange_sockets.iter().map(|e| &e.path).collect::<Vec<_>>());
        let relay_name = self.config.relay_name.clone().unwrap_or_else(|| "relay".to_string());
        info!("Client connections on: {} ({})", self.config.bind_path, relay_name);
        
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("Shutting down relay server");
            }
            result = async {
                for handle in exchange_handles {
                    handle.await?;
                }
                Ok::<(), anyhow::Error>(())
            } => {
                error!("Exchange listener exited: {:?}", result);
            }
        }
        
        Ok(())
    }

    async fn ensure_directories(&self) -> Result<()> {
        tokio::fs::create_dir_all("/tmp/alphapulse").await
            .context("Failed to create socket directory")?;
        Ok(())
    }

    async fn cleanup_sockets(&self) -> Result<()> {
        for socket in &self.exchange_sockets {
            if Path::new(&socket.path).exists() {
                tokio::fs::remove_file(&socket.path).await.ok();
            }
        }
        if Path::new(&self.config.bind_path).exists() {
            tokio::fs::remove_file(&self.config.bind_path).await.ok();
        }
        Ok(())
    }

    // Symbol mapping is handled through the protocol - no shared memory needed

    fn spawn_exchange_listeners(&self) -> Vec<task::JoinHandle<()>> {
        let mut handles = Vec::new();
        
        for (idx, exchange) in self.exchange_sockets.iter().enumerate() {
            let exchange_id = (idx + 1) as u16;
            let socket_path = exchange.path.clone();
            let exchange_name = exchange.name.clone();
            let sender = self.broadcast_sender.clone();
            let sequence_tracker = self.sequence_tracker.clone();
            let circuit_breaker = self.circuit_breaker_active.clone();
            
            let allowed_types = self.config.message_types.clone();
            let validate_checksum = self.config.validate_checksums;
            let circuit_breaker_threshold = self.config.circuit_breaker_threshold;
            
            let handle = task::spawn_blocking(move || {
                Self::exchange_listener_thread(
                    exchange_id,
                    exchange_name,
                    socket_path,
                    sender,
                    sequence_tracker,
                    circuit_breaker,
                    allowed_types,
                    validate_checksum,
                    circuit_breaker_threshold,
                );
            });
            
            handles.push(handle);
        }
        
        handles
    }

    fn exchange_listener_thread(
        _exchange_id: u16,
        exchange_name: String,
        socket_path: String,
        sender: Sender<Vec<u8>>,
        _sequence_tracker: Arc<SequenceTracker>,
        circuit_breaker: Arc<AtomicBool>,
        allowed_message_types: Option<Vec<u8>>,
        validate_checksum: bool,
        circuit_breaker_threshold: usize,
    ) {
        let listener = match UnixListener::bind(&socket_path) {
            Ok(l) => l,
            Err(e) => {
                error!("Failed to bind {} listener at {}: {}", exchange_name, socket_path, e);
                return;
            }
        };
        
        info!("{} listener bound at {}", exchange_name, socket_path);
        
        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    info!("{} collector connected", exchange_name);
                    
                    let mut buffer = vec![0u8; 65536];
                    let mut pending_data = Vec::new();
                    
                    loop {
                        use std::io::Read;
                        
                        match stream.read(&mut buffer) {
                            Ok(0) => {
                                warn!("{} collector disconnected", exchange_name);
                                break;
                            }
                            Ok(n) => {
                                pending_data.extend_from_slice(&buffer[..n]);
                                
                                // Process complete messages
                                while let Some(msg_size) = Self::get_message_size(&pending_data) {
                                    if pending_data.len() < msg_size {
                                        break; // Wait for more data
                                    }
                                    
                                    // Extract complete message
                                    let message_bytes = pending_data[..msg_size].to_vec();
                                    pending_data.drain(..msg_size);
                                    
                                    // Check if this message type should be relayed
                                    if !Self::should_relay_message(&message_bytes, &allowed_message_types) {
                                        debug!("{} filtered out message type {}", exchange_name, 
                                               if message_bytes.len() >= 5 { message_bytes[4] } else { 0 });
                                        continue;
                                    }
                                    
                                    // Validate message (magic number and optionally checksum)
                                    match Self::validate_message(&message_bytes, validate_checksum) {
                                        Ok(()) => {
                                            // RELAY AS DUMB PIPE - Forward original bytes unchanged
                                            let start = Instant::now();
                                            if !circuit_breaker.load(Ordering::Acquire) {
                                                match sender.try_send(message_bytes) {
                                                    Ok(_) => {
                                                        let latency_us = start.elapsed().as_micros() as f64;
                                                        histogram!("relay.forward_latency_us").record(latency_us);
                                                        counter!("relay.messages_forwarded").increment(1);
                                                        debug!("✅ Forwarded message from {} in {:.2}μs", exchange_name, latency_us);
                                                    }
                                                    Err(e) => {
                                                        error!("{} failed to forward message: {}", exchange_name, e);
                                                        // Use configured circuit breaker threshold
                        if sender.len() > circuit_breaker_threshold {
                                                            circuit_breaker.store(true, Ordering::Release);
                                                            warn!("Circuit breaker activated due to queue overflow");
                                                        }
                                                    }
                                                }
                                            } else {
                                                warn!("Circuit breaker active - dropping message from {}", exchange_name);
                                                counter!("relay.circuit_breaker_drops").increment(1);
                                            }
                                        }
                                        Err(e) => {
                                            warn!("{} received invalid message: {}", exchange_name, e);
                                            counter!("relay.invalid_messages").increment(1);
                                            
                                            // Log specific error types for monitoring
                                            match e {
                                                RelayError::InvalidMagic { .. } => {
                                                    counter!("relay.invalid_magic").increment(1);
                                                }
                                                RelayError::ChecksumMismatch { .. } => {
                                                    counter!("relay.checksum_errors").increment(1);
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!("{} read error: {}", exchange_name, e);
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to accept {} connection: {}", exchange_name, e);
                }
            }
        }
    }

    fn spawn_relay_listener(&self) -> task::JoinHandle<()> {
        let clients = self.clients.clone();
        let bind_path = self.config.bind_path.clone();
        let relay_name = self.config.relay_name.clone().unwrap_or_else(|| "relay".to_string());
        
        task::spawn_blocking(move || {
            let listener = match UnixListener::bind(&bind_path) {
                Ok(l) => l,
                Err(e) => {
                    error!("Failed to bind {} listener: {}", relay_name, e);
                    return;
                }
            };
            
            info!("{} listener bound at {}", relay_name, bind_path);
            
            let mut client_id = 0usize;
            
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        client_id += 1;
                        clients.insert(client_id, stream);
                        gauge!("relay.connected_clients").set(clients.len() as f64);
                        info!("Client {} connected (total: {})", client_id, clients.len());
                    }
                    Err(e) => {
                        error!("Failed to accept client connection: {}", e);
                    }
                }
            }
        })
    }

    fn spawn_broadcaster(&self) -> task::JoinHandle<()> {
        let receiver = self.broadcast_receiver.clone();
        let clients = self.clients.clone();
        
        task::spawn_blocking(move || {
            while let Ok(message) = receiver.recv() {
                // Relay validates messages before sending - always broadcast
                // (Invalid messages are filtered at input)
                let mut disconnected = Vec::new();
                
                // CRITICAL: Send complete message with proper framing
                // The message already contains the header with size information
                // Clients can parse using the same get_message_size() logic
                
                for entry in clients.iter() {
                    let client_id = *entry.key();
                    
                    // Send the complete message - it's already properly framed
                    use std::io::Write;
                    match entry.value().try_clone() {
                        Ok(mut stream) => {
                            // Set TCP_NODELAY for low latency (if applicable)
                            // Write complete message - client will parse header for size
                            if let Err(e) = stream.write_all(&message) {
                                debug!("Client {} write failed: {}", client_id, e);
                                disconnected.push(client_id);
                            } else if let Err(e) = stream.flush() {
                                debug!("Client {} flush failed: {}", client_id, e);
                                disconnected.push(client_id);
                            }
                        }
                        Err(e) => {
                            error!("Failed to clone stream for client {}: {}", client_id, e);
                            disconnected.push(client_id);
                        }
                    }
                }
                
                for id in disconnected {
                    clients.remove(&id);
                    info!("Client {} disconnected", id);
                }
                
                gauge!("relay.connected_clients").set(clients.len() as f64);
            }
        })
    }

    fn spawn_monitor(&self) -> task::JoinHandle<()> {
        let _sender = self.broadcast_sender.clone();
        let _sequence_tracker = self.sequence_tracker.clone();
        let _circuit_breaker = self.circuit_breaker_active.clone();
        
        task::spawn(async move {
            // Event-driven monitoring - metrics are collected on events, not polling
            // Remove periodic polling for latency-critical system
            
            loop {
                // Monitor should be event-driven, triggered by actual events
                // For now, just keep the task alive but don't poll
                tokio::time::sleep(Duration::from_secs(3600)).await; // Sleep 1 hour, effectively disabled
            }
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("signal_relay=debug".parse()?)
                .add_directive("info".parse()?),
        )
        .init();

    // SignalRelay defaults to signal mode
    let relay_type = std::env::var("RELAY_TYPE").unwrap_or_else(|_| "signal".to_string());
    
    let config = match relay_type.as_str() {
        "market_data" => RelayConfig {
            bind_path: MARKET_DATA_RELAY_PATH.to_string(),
            max_queue_size: 1_000_000,  // High throughput for market data
            circuit_breaker_threshold: 800_000,
            validate_checksums: false,   // Speed over validation for market data
            message_types: Some(vec![1, 2, 3, 4, 5, 10, 11, 12]), // Market data types
            relay_name: Some("MarketDataRelay".to_string()),
            exchanges: vec![
                "kraken".to_string(), "coinbase".to_string(), "binance".to_string(), 
                "alpaca".to_string(), "polygon".to_string()
            ],
        },
        "signal" => RelayConfig {
            bind_path: "/tmp/alphapulse/signals.sock".to_string(),
            max_queue_size: 100_000,     // Lower throughput for signals
            circuit_breaker_threshold: 80_000,
            validate_checksums: true,    // Validate strategy signals
            message_types: Some(vec![20, 21, 22, 23, 24, 25, 30, 31, 32, 33, 34, 35, 36]), // Signal types
            relay_name: Some("SignalRelay".to_string()),
            exchanges: vec![
                "kraken".to_string(), "coinbase".to_string(), "binance".to_string(), 
                "alpaca".to_string(), "polygon".to_string()
            ],
        },
        _ => RelayConfig {
            bind_path: DEFAULT_RELAY_BIND_PATH.to_string(),
            max_queue_size: MAX_QUEUE_SIZE,
            circuit_breaker_threshold: CIRCUIT_BREAKER_THRESHOLD,
            validate_checksums: false,
            message_types: None, // No filtering
            relay_name: Some("LegacyRelay".to_string()),
            exchanges: vec![
                "kraken".to_string(), "coinbase".to_string(), "binance".to_string(), 
                "alpaca".to_string(), "polygon".to_string()
            ],
        }
    };

    info!("Starting {} at {}", 
          config.relay_name.as_deref().unwrap_or("relay"), 
          config.bind_path);
    
    if let Some(ref types) = config.message_types {
        info!("Filtering message types: {:?}", types);
    } else {
        info!("No message type filtering (legacy mode)");
    }
    
    let server = RelayServer::new(config);
    server.start().await?;
    
    Ok(())
}
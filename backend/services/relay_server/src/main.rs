use alphapulse_protocol::{
    MessageHeader, MessageType, SymbolMappingMessage, TradeMessage, L2SnapshotMessage, L2DeltaMessage,
    SymbolSequenceTracker, SequenceCheck, MAGIC_BYTE
};
use zerocopy::{AsBytes, FromBytes};
use anyhow::{Context, Result};
use crossbeam_channel::{bounded, Sender, Receiver};
use dashmap::DashMap;
use metrics::{counter, gauge, histogram};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::task;
use tracing::{debug, error, info, warn};

const RELAY_BIND_PATH: &str = "/tmp/alphapulse/relay.sock";
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
    l2_snapshots: Arc<DashMap<(u16, u64), Vec<u8>>>, // (exchange_id, symbol_hash) -> encoded snapshot
    symbol_sequences: Arc<RwLock<SymbolSequenceTracker>>,
}

impl RelayServer {
    fn new() -> Self {
        let (tx, rx) = bounded(MAX_QUEUE_SIZE);
        
        let exchanges = vec![
            ExchangeSocket::new("kraken"),
            ExchangeSocket::new("coinbase"),
            ExchangeSocket::new("binance"),
            ExchangeSocket::new("alpaca"),
            ExchangeSocket::new("polygon"),
        ];
        
        Self {
            exchange_sockets: exchanges,
            broadcast_sender: tx,
            broadcast_receiver: rx,
            clients: Arc::new(DashMap::new()),
            sequence_tracker: Arc::new(SequenceTracker::new()),
            circuit_breaker_active: Arc::new(AtomicBool::new(false)),
            l2_snapshots: Arc::new(DashMap::new()),
            symbol_sequences: Arc::new(RwLock::new(SymbolSequenceTracker::new())),
        }
    }

    async fn start(&self) -> Result<()> {
        self.ensure_directories().await?;
        self.cleanup_sockets().await?;
        
        // No shared memory initialization needed for Unix socket architecture
        
        let exchange_handles = self.spawn_exchange_listeners();
        
        let relay_handle = self.spawn_relay_listener();
        
        let broadcast_handle = self.spawn_broadcaster();
        
        let monitor_handle = self.spawn_monitor();
        
        info!("Relay server started successfully");
        info!("Listening for exchanges on: {:?}", 
            self.exchange_sockets.iter().map(|e| &e.path).collect::<Vec<_>>());
        info!("Client connections on: {}", RELAY_BIND_PATH);
        
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
        if Path::new(RELAY_BIND_PATH).exists() {
            tokio::fs::remove_file(RELAY_BIND_PATH).await.ok();
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
            let l2_snapshots = self.l2_snapshots.clone();
            let symbol_sequences = self.symbol_sequences.clone();
            
            let handle = task::spawn_blocking(move || {
                Self::exchange_listener_thread(
                    exchange_id,
                    exchange_name,
                    socket_path,
                    sender,
                    sequence_tracker,
                    circuit_breaker,
                    l2_snapshots,
                    symbol_sequences,
                );
            });
            
            handles.push(handle);
        }
        
        handles
    }

    fn exchange_listener_thread(
        exchange_id: u16,
        exchange_name: String,
        socket_path: String,
        sender: Sender<Vec<u8>>,
        sequence_tracker: Arc<SequenceTracker>,
        circuit_breaker: Arc<AtomicBool>,
        l2_snapshots: Arc<DashMap<(u16, u64), Vec<u8>>>,
        symbol_sequences: Arc<RwLock<SymbolSequenceTracker>>,
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
                                
                                while pending_data.len() >= MessageHeader::SIZE {
                                    // Always log first few bytes to diagnose magic byte issue
                                    if pending_data[0] != MAGIC_BYTE {
                                        error!("{} wrong magic byte! Expected 0xFE, got 0x{:02x}. First 32 bytes: {:02x?}", 
                                            exchange_name, pending_data[0], 
                                            &pending_data[..std::cmp::min(32, pending_data.len())]);
                                    }
                                    
                                    let header = match MessageHeader::read_from_prefix(
                                        &pending_data[..MessageHeader::SIZE]
                                    ) {
                                        Some(h) => h,
                                        None => {
                                            error!("{} failed to parse header from bytes: {:02x?}",
                                                exchange_name, &pending_data[..MessageHeader::SIZE]);
                                            break;
                                        }
                                    };
                                    
                                    if let Err(e) = header.validate() {
                                        error!("Invalid header from {}: {}", exchange_name, e);
                                        // Try to resync by finding next magic byte
                                        if let Some(pos) = pending_data[1..].iter().position(|&b| b == MAGIC_BYTE) {
                                            warn!("{} resyncing: skipping {} bytes to next magic byte", exchange_name, pos + 1);
                                            pending_data.drain(..pos + 1);
                                            continue;
                                        } else {
                                            warn!("{} no magic byte found in buffer, clearing all {} bytes", exchange_name, pending_data.len());
                                            pending_data.clear();
                                            break;
                                        }
                                    }
                                    
                                    let total_size = MessageHeader::SIZE + header.get_length() as usize;
                                    
                                    if pending_data.len() < total_size {
                                        debug!("{} waiting for more data: have {}, need {} total ({} header + {} payload)",
                                            exchange_name, pending_data.len(), total_size, MessageHeader::SIZE, header.get_length());
                                        break;
                                    }
                                    
                                    debug!("{} processing message type {:?}, total {} bytes", 
                                        exchange_name, header.get_type(), total_size);
                                    
                                    let sequence = header.get_sequence();
                                    sequence_tracker.validate_and_update(exchange_id, sequence);
                                    
                                    // Handle L2 snapshots specially
                                    if let Ok(MessageType::L2Snapshot) = header.get_type() {
                                        info!("{} received L2 snapshot message, size: {}", exchange_name, total_size);
                                        match L2SnapshotMessage::decode(&pending_data[MessageHeader::SIZE..total_size]) {
                                            Ok(snapshot) => {
                                                let key = (exchange_id, snapshot.symbol_hash);
                                                let encoded = pending_data[..total_size].to_vec();
                                                l2_snapshots.insert(key, encoded.clone());
                                                info!("Stored L2 snapshot for {}:{} (exchange_id: {}, {} bids, {} asks)", 
                                                    exchange_name, snapshot.symbol_hash, exchange_id,
                                                    snapshot.bids.len(), snapshot.asks.len());
                                            }
                                            Err(e) => {
                                                error!("Failed to decode L2 snapshot from {}: {}", exchange_name, e);
                                            }
                                        }
                                    }
                                    
                                    // Handle SymbolMapping messages specially - ALWAYS forward these!
                                    if let Ok(MessageType::SymbolMapping) = header.get_type() {
                                        info!("{} received SymbolMapping message, size: {} - FORWARDING to WS Bridge", exchange_name, total_size);
                                        match SymbolMappingMessage::decode(&pending_data[MessageHeader::SIZE..total_size]) {
                                            Ok(mapping) => {
                                                info!("Decoded SymbolMapping: hash={}, symbol={}", 
                                                    mapping.symbol_hash, mapping.symbol_string);
                                            }
                                            Err(e) => {
                                                error!("Failed to decode SymbolMapping from {}: {}", exchange_name, e);
                                            }
                                        }
                                        // IMMEDIATELY forward SymbolMapping messages - don't wait for general forwarding section
                                        if !circuit_breaker.load(Ordering::Acquire) {
                                            let message = pending_data[..total_size].to_vec();
                                            let start = Instant::now();
                                            match sender.try_send(message) {
                                                Ok(_) => {
                                                    let latency_us = start.elapsed().as_micros() as f64;
                                                    histogram!("relay.forward_latency_us").record(latency_us);
                                                    counter!("relay.messages_forwarded").increment(1);
                                                    info!("✅ Successfully forwarded SymbolMapping to WS Bridge");
                                                }
                                                Err(e) => {
                                                    error!("❌ Failed to forward SymbolMapping: {}", e);
                                                }
                                            }
                                        }
                                    }
                                    
                                    // Check L2 delta sequences
                                    if let Ok(MessageType::L2Delta) = header.get_type() {
                                        if let Ok(delta) = L2DeltaMessage::decode(&pending_data[MessageHeader::SIZE..total_size]) {
                                            let mut seq_tracker = symbol_sequences.write();
                                            match seq_tracker.check_sequence(delta.symbol_hash, delta.sequence) {
                                                SequenceCheck::Gap(gap) => {
                                                    warn!("L2 sequence gap of {} for {}:{}", gap, exchange_id, delta.symbol_hash);
                                                    // Send reset message to clients
                                                    let reset_header = MessageHeader::new(MessageType::L2Reset, 10, sequence);
                                                    let mut reset_msg = Vec::new();
                                                    reset_msg.extend_from_slice(AsBytes::as_bytes(&reset_header));
                                                    reset_msg.extend_from_slice(&delta.symbol_hash.to_le_bytes());
                                                    reset_msg.extend_from_slice(&exchange_id.to_le_bytes());
                                                    let _ = sender.try_send(reset_msg);
                                                }
                                                SequenceCheck::OutOfOrder => {
                                                    debug!("Out of order L2 message for {}:{}", exchange_id, delta.symbol_hash);
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                    
                                    if !circuit_breaker.load(Ordering::Acquire) {
                                        let mut message = pending_data[..total_size].to_vec();
                                        
                                        if let Ok(MessageType::Trade) = header.get_type() {
                                            // Set relay timestamp for latency tracking
                                            if message.len() >= MessageHeader::SIZE + TradeMessage::SIZE {
                                                if let Some(mut trade) = TradeMessage::read_from_prefix(&message[MessageHeader::SIZE..]) {
                                                    trade.set_relay_timestamp();
                                                    // Copy the updated trade back to the message
                                                    message[MessageHeader::SIZE..MessageHeader::SIZE + TradeMessage::SIZE]
                                                        .copy_from_slice(trade.as_bytes());
                                                }
                                            }
                                            
                                            if message.len() >= MessageHeader::SIZE + 30 {
                                                let exchange_offset = MessageHeader::SIZE + 28;
                                                message[exchange_offset] = (exchange_id & 0xFF) as u8;
                                                message[exchange_offset + 1] = ((exchange_id >> 8) & 0xFF) as u8;
                                            }
                                        }
                                        
                                        let start = Instant::now();
                                        match sender.try_send(message) {
                                            Ok(_) => {
                                                let latency_us = start.elapsed().as_micros() as f64;
                                                histogram!("relay.forward_latency_us").record(latency_us);
                                                counter!("relay.messages_forwarded").increment(1);
                                            }
                                            Err(e) => {
                                                if sender.len() > CIRCUIT_BREAKER_THRESHOLD {
                                                    warn!("Circuit breaker activated - queue size: {}", sender.len());
                                                    circuit_breaker.store(true, Ordering::Release);
                                                }
                                                debug!("Failed to forward message from {}: {}", exchange_name, e);
                                            }
                                        }
                                    } else {
                                        counter!("relay.messages_dropped_circuit_breaker").increment(1);
                                        
                                        if sender.len() < CIRCUIT_BREAKER_THRESHOLD / 2 {
                                            circuit_breaker.store(false, Ordering::Release);
                                            info!("Circuit breaker deactivated");
                                        }
                                    }
                                    
                                    pending_data.drain(..total_size);
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
        
        task::spawn_blocking(move || {
            let listener = match UnixListener::bind(RELAY_BIND_PATH) {
                Ok(l) => l,
                Err(e) => {
                    error!("Failed to bind relay listener: {}", e);
                    return;
                }
            };
            
            info!("Relay listener bound at {}", RELAY_BIND_PATH);
            
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
                let mut disconnected = Vec::new();
                
                for entry in clients.iter() {
                    let client_id = *entry.key();
                    let mut stream = entry.value().try_clone().unwrap();
                    
                    use std::io::Write;
                    if let Err(_) = stream.write_all(&message) {
                        disconnected.push(client_id);
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
                .add_directive("relay_server=debug".parse()?)
                .add_directive("info".parse()?),
        )
        .init();

    info!("Starting relay server");
    
    let server = RelayServer::new();
    server.start().await?;
    
    Ok(())
}
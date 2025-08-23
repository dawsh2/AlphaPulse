use alphapulse_protocol::message_protocol::{
    MESSAGE_MAGIC, MessageType, SourceType, MessageHeader,
};
use alphapulse_protocol::messages::{
    TradeMessage, SwapEventMessage, PoolUpdateMessage,
};
use alphapulse_protocol::{
    StatusUpdateMessage, L2SnapshotMessage, L2DeltaMessage, TokenInfoMessage,
    MessageTraceMessage, SymbolMappingMessage, OrderBookMessage, TradeSide,
};
use zerocopy::{AsBytes, FromBytes};
use anyhow::{Context, Result};
use crossbeam_channel::{bounded, Sender};
use metrics::{counter, histogram};
use parking_lot::Mutex;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::task;
use tracing::{debug, error, info, warn};

pub struct UnixSocketWriter {
    path: String,
    sender: Sender<Vec<u8>>,
    sequence: Arc<AtomicU32>,
    bytes_written: Arc<AtomicU64>,
    messages_written: Arc<AtomicU64>,
}

impl UnixSocketWriter {
    pub fn new(path: &str) -> Self {
        let (sender, receiver) = bounded::<Vec<u8>>(10000);
        
        let path_clone = path.to_string();
        let sequence = Arc::new(AtomicU32::new(0));
        let bytes_written = Arc::new(AtomicU64::new(0));
        let messages_written = Arc::new(AtomicU64::new(0));
        
        let seq_clone = sequence.clone();
        let bytes_clone = bytes_written.clone();
        let msgs_clone = messages_written.clone();
        
        task::spawn_blocking(move || {
            let mut writer_thread = WriterThread {
                path: path_clone,
                receiver,
                stream: None,
                sequence: seq_clone,
                bytes_written: bytes_clone,
                messages_written: msgs_clone,
            };
            writer_thread.run();
        });
        
        Self {
            path: path.to_string(),
            sender,
            sequence,
            bytes_written,
            messages_written,
        }
    }
    
    // No start() method needed - we connect TO the relay server, not create our own socket
    
    pub fn write_trade(&self, trade: &TradeMessage) -> Result<()> {
        let seq = self.sequence.fetch_add(1, Ordering::Relaxed);
        
        // Trade messages now include the new header directly
        let mut buffer = Vec::with_capacity(std::mem::size_of::<TradeMessage>());
        let trade_bytes = AsBytes::as_bytes(trade);
        buffer.extend_from_slice(trade_bytes);
        
        // Debug sizes
        debug!("Trade message with new protocol - Total: {} bytes (magic: 0x{:08x})",
            buffer.len(), MESSAGE_MAGIC);
        
        // Log complete message for debugging
        if seq % 20 == 0 {
            debug!("Sending trade #{}, {} bytes. First 32 bytes: {:02x?}", 
                seq, buffer.len(), &buffer[..std::cmp::min(32, buffer.len())]);
        }
        
        let start = Instant::now();
        
        // Check if channel is getting full (potential bottleneck)
        let channel_len = self.sender.len();
        if channel_len > 5000 {
            warn!("🚨 Unix socket channel is {}% full ({}/10000) - potential bottleneck!", 
                  (channel_len * 100) / 10000, channel_len);
        }
        
        match self.sender.try_send(buffer) {
            Ok(()) => {},
            Err(crossbeam_channel::TrySendError::Full(_)) => {
                error!("🚨 Unix socket channel FULL! Dropping message to prevent blocking");
                counter!("unix_socket.messages_dropped").increment(1);
                return Err(anyhow::anyhow!("Channel full - message dropped"));
            }
            Err(e) => return Err(anyhow::anyhow!("Channel send failed: {}", e))
        }
        
        let latency_us = start.elapsed().as_micros() as f64;
        histogram!("unix_socket.send_latency_us").record(latency_us);
        counter!("unix_socket.trades_sent").increment(1);
        
        Ok(())
    }
    
    pub fn write_swap_event(&self, swap: &SwapEventMessage) -> Result<()> {
        let seq = self.sequence.fetch_add(1, Ordering::Relaxed);
        
        // SwapEvent messages now include the new header directly
        let mut buffer = Vec::with_capacity(std::mem::size_of::<SwapEventMessage>());
        let swap_bytes = AsBytes::as_bytes(swap);
        buffer.extend_from_slice(swap_bytes);
        
        debug!("Sending SwapEvent #{}, {} bytes total (magic: 0x{:08x})", seq, buffer.len(), MESSAGE_MAGIC);
        
        self.sender.send(buffer)
            .context("Failed to send swap event to writer thread")?;
        
        counter!("unix_socket.swap_events_sent").increment(1);
        
        Ok(())
    }
    
    pub fn write_pool_update(&self, pool_update: &PoolUpdateMessage) -> Result<()> {
        let seq = self.sequence.fetch_add(1, Ordering::Relaxed);
        
        // PoolUpdate messages now include the new header directly  
        let mut buffer = Vec::with_capacity(std::mem::size_of::<PoolUpdateMessage>());
        let pool_bytes = AsBytes::as_bytes(pool_update);
        buffer.extend_from_slice(pool_bytes);
        
        debug!("Sending PoolUpdate #{}, {} bytes total (magic: 0x{:08x})", seq, buffer.len(), MESSAGE_MAGIC);
        
        self.sender.send(buffer)
            .context("Failed to send pool update to writer thread")?;
        
        counter!("unix_socket.pool_updates_sent").increment(1);
        
        Ok(())
    }
    
    pub fn write_orderbook(&self, orderbook: &OrderBookMessage) -> Result<()> {
        let seq = self.sequence.fetch_add(1, Ordering::Relaxed);
        let mut encoded_orderbook = Vec::new();
        orderbook.encode(&mut encoded_orderbook);
        let header = MessageHeader::new(
            MessageType::OrderBook, 
            1, // version
            SourceType::PolygonCollector,
            encoded_orderbook.len() as u32,
            seq as u64
        );
        
        let mut buffer = Vec::with_capacity(std::mem::size_of::<MessageHeader>() + encoded_orderbook.len());
        buffer.extend_from_slice(AsBytes::as_bytes(&header));
        buffer.extend_from_slice(&encoded_orderbook);
        
        self.sender.send(buffer)
            .context("Failed to send orderbook to writer thread")?;
        
        counter!("unix_socket.orderbooks_sent").increment(1);
        
        Ok(())
    }
    
    pub fn write_message_trace(&self, trace: &MessageTraceMessage) -> Result<()> {
        let seq = self.sequence.fetch_add(1, Ordering::Relaxed);
        let encoded_trace = trace.encode();
        let header = MessageHeader::new(
            MessageType::Custom, // No MessageTrace in new enum
            1, // version
            SourceType::PolygonCollector,
            encoded_trace.len() as u32,
            seq as u64
        );
        
        let mut buffer = Vec::with_capacity(std::mem::size_of::<MessageHeader>() + encoded_trace.len());
        buffer.extend_from_slice(AsBytes::as_bytes(&header));
        buffer.extend_from_slice(&encoded_trace);
        
        debug!("Sending MessageTrace #{}, {} bytes total", seq, buffer.len());
        
        self.sender.send(buffer)
            .context("Failed to send message trace to writer thread")?;
        
        counter!("unix_socket.message_traces_sent").increment(1);
        
        Ok(())
    }
    
    pub fn write_symbol_mapping(&self, mapping: &SymbolMappingMessage) -> Result<()> {
        let seq = self.sequence.fetch_add(1, Ordering::Relaxed);
        let encoded_mapping = mapping.encode();
        let header = MessageHeader::new(
            MessageType::Custom, // No SymbolMapping in new enum
            1, // version
            SourceType::PolygonCollector,
            encoded_mapping.len() as u32,
            seq as u64
        );
        
        let mut buffer = Vec::with_capacity(std::mem::size_of::<MessageHeader>() + encoded_mapping.len());
        buffer.extend_from_slice(AsBytes::as_bytes(&header));
        buffer.extend_from_slice(&encoded_mapping);
        
        self.sender.send(buffer)
            .context("Failed to send symbol mapping to writer thread")?;
        
        counter!("unix_socket.symbol_mappings_sent").increment(1);
        
        Ok(())
    }
    
    pub fn write_status_update(&self, status: &StatusUpdateMessage) -> Result<()> {
        let seq = self.sequence.fetch_add(1, Ordering::Relaxed);
        let header = MessageHeader::new(
            MessageType::Custom, // No StatusUpdate in new enum  
            1, // version
            SourceType::PolygonCollector,
            StatusUpdateMessage::SIZE as u32,
            seq as u64
        );
        
        let mut buffer = Vec::with_capacity(std::mem::size_of::<MessageHeader>() + StatusUpdateMessage::SIZE);
        buffer.extend_from_slice(AsBytes::as_bytes(&header));
        buffer.extend_from_slice(AsBytes::as_bytes(status));
        
        self.sender.send(buffer)
            .context("Failed to send status update to writer thread")?;
        
        counter!("unix_socket.status_updates_sent").increment(1);
        
        Ok(())
    }
    
    pub fn write_status(&self, status: &StatusUpdateMessage) -> Result<()> {
        let seq = self.sequence.fetch_add(1, Ordering::Relaxed);
        let header = MessageHeader::new(
            MessageType::Custom, // No StatusUpdate in new enum
            1, // version
            SourceType::PolygonCollector,
            StatusUpdateMessage::SIZE as u32,
            seq as u64
        );
        
        let mut buffer = Vec::with_capacity(std::mem::size_of::<MessageHeader>() + StatusUpdateMessage::SIZE);
        buffer.extend_from_slice(AsBytes::as_bytes(&header));
        buffer.extend_from_slice(AsBytes::as_bytes(status));
        
        self.sender.send(buffer)
            .context("Failed to send status to writer thread")?;
        
        counter!("unix_socket.status_sent").increment(1);
        
        Ok(())
    }
    
    pub fn write_token_info(&self, token_info: &TokenInfoMessage) -> Result<()> {
        let seq = self.sequence.fetch_add(1, Ordering::Relaxed);
        let header = MessageHeader::new(
            MessageType::TokenDiscovered,
            1, // version
            SourceType::PolygonCollector,
            128 as u32,
            seq as u64
        );
        
        let mut buffer = Vec::with_capacity(std::mem::size_of::<MessageHeader>() + 128);
        buffer.extend_from_slice(AsBytes::as_bytes(&header));
        buffer.extend_from_slice(AsBytes::as_bytes(token_info));
        
        self.sender.send(buffer)
            .context("Failed to send token info to writer thread")?;
        
        counter!("unix_socket.token_info_sent").increment(1);
        debug!("📤 Broadcast token info: {} ({})", 
               token_info.get_symbol(), token_info.get_token_address());
        
        Ok(())
    }
    
    pub fn write_l2_snapshot(&self, snapshot: &L2SnapshotMessage) -> Result<()> {
        let seq = self.sequence.fetch_add(1, Ordering::Relaxed);
        let mut encoded_snapshot = Vec::new();
        snapshot.encode(&mut encoded_snapshot);
        let header = MessageHeader::new(
            MessageType::Snapshot,
            1, // version
            SourceType::PolygonCollector,
            encoded_snapshot.len() as u32,
            seq as u64
        );
        
        let mut buffer = Vec::with_capacity(std::mem::size_of::<MessageHeader>() + encoded_snapshot.len());
        buffer.extend_from_slice(AsBytes::as_bytes(&header));
        buffer.extend_from_slice(&encoded_snapshot);
        
        self.sender.send(buffer)
            .context("Failed to send L2 snapshot to writer thread")?;
        
        counter!("unix_socket.l2_snapshots_sent").increment(1);
        
        Ok(())
    }
    
    pub fn write_l2_delta(&self, delta: &L2DeltaMessage) -> Result<()> {
        let seq = self.sequence.fetch_add(1, Ordering::Relaxed);
        let mut encoded_delta = Vec::new();
        delta.encode(&mut encoded_delta);
        let header = MessageHeader::new(
            MessageType::Custom, // No L2Delta in new enum
            1, // version
            SourceType::PolygonCollector,
            encoded_delta.len() as u32,
            seq as u64
        );
        
        let mut buffer = Vec::with_capacity(std::mem::size_of::<MessageHeader>() + encoded_delta.len());
        buffer.extend_from_slice(AsBytes::as_bytes(&header));
        buffer.extend_from_slice(&encoded_delta);
        
        self.sender.send(buffer)
            .context("Failed to send L2 delta to writer thread")?;
        
        counter!("unix_socket.l2_deltas_sent").increment(1);
        
        Ok(())
    }
    
    /// Write raw bytes directly (for new protocol messages)
    pub fn write_bytes(&self, bytes: &[u8]) -> Result<()> {
        let start = Instant::now();
        
        // Check if channel is getting full (potential bottleneck)
        let channel_len = self.sender.len();
        if channel_len > 5000 {
            warn!("🚨 Unix socket channel is {}% full ({}/10000) - potential bottleneck!", 
                  (channel_len * 100) / 10000, channel_len);
        }
        
        match self.sender.try_send(bytes.to_vec()) {
            Ok(()) => {},
            Err(crossbeam_channel::TrySendError::Full(_)) => {
                counter!("socket_write_errors", "type" => "channel_full").increment(1);
                return Err(anyhow::anyhow!("Unix socket channel full"));
            },
            Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
                counter!("socket_write_errors", "type" => "disconnected").increment(1);
                return Err(anyhow::anyhow!("Unix socket disconnected"));
            }
        }
        
        let duration = start.elapsed();
        histogram!("socket_write_duration").record(duration.as_micros() as f64);
        counter!("socket_writes", "type" => "raw_bytes").increment(1);
        
        debug!("📤 Sent {} raw bytes to Unix socket", bytes.len());
        
        Ok(())
    }
    
    pub fn stats(&self) -> (u64, u64) {
        (
            self.messages_written.load(Ordering::Relaxed),
            self.bytes_written.load(Ordering::Relaxed),
        )
    }
}

struct WriterThread {
    path: String,
    receiver: crossbeam_channel::Receiver<Vec<u8>>,
    stream: Option<UnixStream>,
    sequence: Arc<AtomicU32>,
    bytes_written: Arc<AtomicU64>,
    messages_written: Arc<AtomicU64>,
}

impl WriterThread {
    fn run(&mut self) {
        info!("Unix socket writer thread started for {}", self.path);
        
        loop {
            // Try to receive message
            match self.receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(buffer) => {
                    // Ensure we have a connection
                    if self.stream.is_none() {
                        self.connect();
                    }
                    
                    // Try to write
                    if let Some(ref mut stream) = self.stream {
                        use std::io::Write;
                        match stream.write_all(&buffer) {
                            Ok(()) => {
                                self.bytes_written.fetch_add(buffer.len() as u64, Ordering::Relaxed);
                                self.messages_written.fetch_add(1, Ordering::Relaxed);
                                
                                // Log every 1000 messages
                                let msg_count = self.messages_written.load(Ordering::Relaxed);
                                if msg_count % 1000 == 0 {
                                    debug!("Sent {} messages via Unix socket", msg_count);
                                }
                            }
                            Err(e) => {
                                error!("Failed to write to Unix socket: {}", e);
                                self.stream = None; // Disconnect on error
                            }
                        }
                    }
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                    // Normal timeout, check connection health
                    if self.stream.is_none() {
                        self.connect();
                    }
                }
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                    info!("Unix socket writer thread shutting down");
                    break;
                }
            }
        }
    }
    
    fn connect(&mut self) {
        match UnixStream::connect(&self.path) {
            Ok(stream) => {
                info!("Connected to Unix socket at {}", self.path);
                self.stream = Some(stream);
            }
            Err(e) => {
                debug!("Failed to connect to Unix socket at {}: {}", self.path, e);
                // Will retry on next iteration
            }
        }
    }
}
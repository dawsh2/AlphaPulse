use alphapulse_protocol::*;
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
        let header = MessageHeader::new(MessageType::Trade, TradeMessage::SIZE as u16, seq);
        
        let mut buffer = Vec::with_capacity(MessageHeader::SIZE + TradeMessage::SIZE);
        let header_bytes = AsBytes::as_bytes(&header);
        let trade_bytes = AsBytes::as_bytes(trade);
        
        // Debug sizes
        debug!("Trade message sizes - Header: {} bytes, Trade: {} bytes, Total expected: {} bytes",
            header_bytes.len(), trade_bytes.len(), header_bytes.len() + trade_bytes.len());
        
        // Verify header is correct
        if header_bytes[0] != 0xFE {
            error!("CRITICAL: Header magic byte is wrong! {:02x?}", &header_bytes[0..8]);
        }
        
        buffer.extend_from_slice(header_bytes);
        buffer.extend_from_slice(trade_bytes);
        
        // CRITICAL: Log actual buffer size
        if buffer.len() != 72 {
            error!("CRITICAL: Trade buffer size is {} bytes, expected 72!", buffer.len());
        }
        
        // Log complete message for debugging
        if seq % 20 == 0 || buffer.len() != 72 {
            debug!("Sending trade #{}, ACTUAL {} bytes. First 32 bytes: {:02x?}", 
                seq, buffer.len(), &buffer[..std::cmp::min(32, buffer.len())]);
        }
        
        let start = Instant::now();
        self.sender.try_send(buffer)
            .map_err(|e| anyhow::anyhow!("Channel send failed: {}", e))?;
        
        let latency_us = start.elapsed().as_micros() as f64;
        histogram!("unix_socket.send_latency_us").record(latency_us);
        counter!("unix_socket.trades_sent").increment(1);
        
        Ok(())
    }
    
    pub fn write_orderbook(&self, orderbook: &OrderBookMessage) -> Result<()> {
        let seq = self.sequence.fetch_add(1, Ordering::Relaxed);
        
        let mut payload = Vec::new();
        orderbook.encode(&mut payload);
        
        let header = MessageHeader::new(MessageType::OrderBook, payload.len() as u16, seq);
        
        let mut buffer = Vec::with_capacity(MessageHeader::SIZE + payload.len());
        buffer.extend_from_slice(AsBytes::as_bytes(&header));
        buffer.extend_from_slice(&payload);
        
        let start = Instant::now();
        self.sender.try_send(buffer)
            .map_err(|e| anyhow::anyhow!("Channel send failed: {}", e))?;
        
        let latency_us = start.elapsed().as_micros() as f64;
        histogram!("unix_socket.send_latency_us").record(latency_us);
        counter!("unix_socket.orderbooks_sent").increment(1);
        
        Ok(())
    }
    
    pub fn write_l2_snapshot(&self, snapshot: &alphapulse_protocol::L2SnapshotMessage) -> Result<()> {
        let seq = self.sequence.fetch_add(1, Ordering::Relaxed);
        
        let mut payload = Vec::new();
        snapshot.encode(&mut payload);
        
        let header = MessageHeader::new(MessageType::L2Snapshot, payload.len() as u16, seq);
        
        let mut buffer = Vec::with_capacity(MessageHeader::SIZE + payload.len());
        buffer.extend_from_slice(AsBytes::as_bytes(&header));
        buffer.extend_from_slice(&payload);
        
        debug!("Sending L2Snapshot message, header+payload = {} bytes, type = L2Snapshot", buffer.len());
        
        let start = Instant::now();
        self.sender.try_send(buffer)
            .map_err(|e| anyhow::anyhow!("Channel send failed: {}", e))?;
        
        let latency_us = start.elapsed().as_micros() as f64;
        histogram!("unix_socket.send_latency_us").record(latency_us);
        counter!("unix_socket.l2_snapshots_sent").increment(1);
        
        Ok(())
    }
    
    pub fn write_l2_delta(&self, delta: &alphapulse_protocol::L2DeltaMessage) -> Result<()> {
        let seq = self.sequence.fetch_add(1, Ordering::Relaxed);
        
        let mut payload = Vec::new();
        delta.encode(&mut payload);
        
        let header = MessageHeader::new(MessageType::L2Delta, payload.len() as u16, seq);
        
        let mut buffer = Vec::with_capacity(MessageHeader::SIZE + payload.len());
        buffer.extend_from_slice(AsBytes::as_bytes(&header));
        buffer.extend_from_slice(&payload);
        
        // Debug logging
        debug!("Sending L2Delta message, header+payload = {} bytes, type = {:?}", 
            buffer.len(), MessageType::L2Delta);
        
        let start = Instant::now();
        self.sender.try_send(buffer)
            .map_err(|e| anyhow::anyhow!("Channel send failed: {}", e))?;
        
        let latency_us = start.elapsed().as_micros() as f64;
        histogram!("unix_socket.send_latency_us").record(latency_us);
        counter!("unix_socket.l2_deltas_sent").increment(1);
        
        Ok(())
    }
    
    pub fn write_symbol_mapping(&self, mapping: &alphapulse_protocol::SymbolMappingMessage) -> Result<()> {
        let seq = self.sequence.fetch_add(1, Ordering::Relaxed);
        
        let payload = mapping.encode();
        let header = MessageHeader::new(MessageType::SymbolMapping, payload.len() as u16, seq);
        
        let mut buffer = Vec::with_capacity(MessageHeader::SIZE + payload.len());
        buffer.extend_from_slice(AsBytes::as_bytes(&header));
        buffer.extend_from_slice(&payload);
        
        let start = Instant::now();
        self.sender.try_send(buffer)
            .map_err(|e| anyhow::anyhow!("Channel send failed: {}", e))?;
        
        let latency_us = start.elapsed().as_micros() as f64;
        histogram!("unix_socket.send_latency_us").record(latency_us);
        counter!("unix_socket.symbol_mappings_sent").increment(1);
        
        Ok(())
    }
    
    pub fn write_arbitrage_opportunity(&self, arb: &alphapulse_protocol::ArbitrageOpportunityMessage) -> Result<()> {
        let seq = self.sequence.fetch_add(1, Ordering::Relaxed);
        
        let payload = arb.encode();
        let header = MessageHeader::new(MessageType::ArbitrageOpportunity, payload.len() as u16, seq);
        
        let mut buffer = Vec::with_capacity(MessageHeader::SIZE + payload.len());
        buffer.extend_from_slice(AsBytes::as_bytes(&header));
        buffer.extend_from_slice(&payload);
        
        info!("📊 Sending arbitrage opportunity: {} ({} bytes)", arb.pair, buffer.len());
        
        let start = Instant::now();
        self.sender.try_send(buffer)
            .map_err(|e| anyhow::anyhow!("Channel send failed: {}", e))?;
        
        let latency_us = start.elapsed().as_micros() as f64;
        histogram!("unix_socket.send_latency_us").record(latency_us);
        counter!("unix_socket.arbitrage_opportunities_sent").increment(1);
        
        Ok(())
    }
    
    pub fn stats(&self) -> (u64, u64) {
        (
            self.bytes_written.load(Ordering::Relaxed),
            self.messages_written.load(Ordering::Relaxed),
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
        info!("Unix socket writer thread started");
        let mut last_heartbeat = Instant::now();
        let mut retry_count = 0u32;
        let max_retry_delay = 30; // Maximum retry delay in seconds
        
        loop {
            // Process all pending messages first before sending heartbeat
            let mut messages_processed = 0;
            while let Ok(data) = self.receiver.try_recv() {
                if let Err(e) = self.write_data(&data) {
                    error!("Failed to write to unix socket: {}", e);
                    self.stream = None;
                    retry_count += 1;
                    let delay = std::cmp::min(2u32.pow(retry_count.min(5)), max_retry_delay);
                    warn!("Connection lost, retrying in {}s (attempt {})", delay, retry_count);
                    std::thread::sleep(Duration::from_secs(delay as u64));
                    continue;
                } else {
                    retry_count = 0; // Reset on successful write
                }
                messages_processed += 1;
                // Process ALL messages, not just 100
                // This ensures L2Delta messages get through
            }
            
            // Only send heartbeat if no messages for a while (1 second)
            if messages_processed == 0 && last_heartbeat.elapsed() > Duration::from_secs(1) {
                self.send_heartbeat();
                last_heartbeat = Instant::now();
            }
            
            // Only sleep if we didn't process any messages
            if messages_processed == 0 {
                std::thread::sleep(Duration::from_millis(1));
            }
            
            // Check if channel is disconnected
            if self.receiver.is_empty() && self.receiver.len() == 0 {
                match self.receiver.recv_timeout(Duration::from_millis(10)) {
                    Ok(data) => {
                        if let Err(e) = self.write_data(&data) {
                            error!("Failed to write to unix socket: {}", e);
                            self.stream = None;
                        }
                    }
                    Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                        info!("Writer thread shutting down");
                        break;
                    }
                    Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                        // Normal timeout, continue
                    }
                }
            }
        }
    }
    
    fn ensure_connected(&mut self) -> Result<()> {
        if self.stream.is_some() {
            return Ok(());
        }
        
        // Connect TO the relay server instead of listening
        // Note: On macOS, this connects to exchange-specific sockets
        // managed by launchd. On Linux, systemd would manage the sockets.
        match UnixStream::connect(&self.path) {
            Ok(stream) => {
                info!("Connected to socket at {}", self.path);
                self.stream = Some(stream);
                Ok(())
            }
            Err(e) => {
                debug!("Failed to connect to socket at {}: {} (will retry)", self.path, e);
                Err(anyhow::anyhow!("Failed to connect: {}", e))
            }
        }
    }
    
    fn write_data(&mut self, data: &[u8]) -> Result<()> {
        self.ensure_connected()?;
        
        if let Some(ref mut stream) = self.stream {
            use std::io::Write;
            
            // CRITICAL: Log exact message type and size by reading header
            let msg_type = if data.len() >= 8 {
                match data[1] { // Message type is at byte 1
                    0x01 => "TRADE",
                    0x02 => "ORDERBOOK", 
                    0x03 => "HEARTBEAT",
                    0x04 => "METRICS",
                    0x05 => "L2SNAPSHOT",
                    0x06 => "L2DELTA",
                    0x07 => "L2RESET",
                    0x08 => "SYMBOL_MAPPING",
                    0x09 => "ARBITRAGE_OPPORTUNITY",
                    0x0A => "STATUS_UPDATE",
                    _ => "UNKNOWN_TYPE",
                }
            } else { "TOO_SHORT" };
            
            debug!("Writing {} ({} bytes), first 8: {:02x?}", msg_type, data.len(), &data[..std::cmp::min(8, data.len())]);
            
            if data.len() == 64 && data[0] != 0xFE {
                error!("CRITICAL BUG: Sending 64-byte message without header! First bytes: {:02x?}", &data[..8]);
            }
            
            let start = Instant::now();
            // Write atomically - don't split the message
            let bytes_written = stream.write(data)?;
            if bytes_written != data.len() {
                error!("Partial write: wrote {} of {} bytes", bytes_written, data.len());
                return Err(anyhow::anyhow!("Partial write to socket"));
            }
            stream.flush()?;
            
            let latency_us = start.elapsed().as_micros() as f64;
            histogram!("unix_socket.write_latency_us").record(latency_us);
            
            self.bytes_written.fetch_add(data.len() as u64, Ordering::Relaxed);
            self.messages_written.fetch_add(1, Ordering::Relaxed);
            
            debug!("Successfully wrote {} bytes to Unix socket", data.len());
        }
        
        Ok(())
    }
    
    fn send_heartbeat(&mut self) {
        let seq = self.sequence.fetch_add(1, Ordering::Relaxed);
        let timestamp_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        
        let heartbeat = HeartbeatMessage::new(timestamp_ns, seq);
        let header = MessageHeader::new(MessageType::Heartbeat, HeartbeatMessage::SIZE as u16, seq);
        
        let mut buffer = Vec::with_capacity(MessageHeader::SIZE + HeartbeatMessage::SIZE);
        buffer.extend_from_slice(AsBytes::as_bytes(&header));
        buffer.extend_from_slice(AsBytes::as_bytes(&heartbeat));
        
        if let Err(e) = self.write_data(&buffer) {
            debug!("Failed to send heartbeat: {}", e);
        }
    }
}
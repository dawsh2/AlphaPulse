use alphapulse_protocol::{*, StatusUpdateMessage};
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
        let header = MessageHeader::new(MessageType::SwapEvent, SwapEventMessage::SIZE as u16, seq);
        
        let mut buffer = Vec::with_capacity(MessageHeader::SIZE + SwapEventMessage::SIZE);
        let header_bytes = AsBytes::as_bytes(&header);
        let swap_bytes = AsBytes::as_bytes(swap);
        
        buffer.extend_from_slice(header_bytes);
        buffer.extend_from_slice(swap_bytes);
        
        debug!("Sending SwapEvent #{}, {} bytes total", seq, buffer.len());
        
        self.sender.send(buffer)
            .context("Failed to send swap event to writer thread")?;
        
        counter!("unix_socket.swap_events_sent").increment(1);
        
        Ok(())
    }
    
    pub fn write_orderbook(&self, orderbook: &OrderBookMessage) -> Result<()> {
        let seq = self.sequence.fetch_add(1, Ordering::Relaxed);
        let header = MessageHeader::new(MessageType::OrderBook, OrderBookMessage::SIZE as u16, seq);
        
        let mut buffer = Vec::with_capacity(MessageHeader::SIZE + OrderBookMessage::SIZE);
        buffer.extend_from_slice(AsBytes::as_bytes(&header));
        buffer.extend_from_slice(AsBytes::as_bytes(orderbook));
        
        self.sender.send(buffer)
            .context("Failed to send orderbook to writer thread")?;
        
        counter!("unix_socket.orderbooks_sent").increment(1);
        
        Ok(())
    }
    
    pub fn write_status(&self, status: &StatusUpdateMessage) -> Result<()> {
        let seq = self.sequence.fetch_add(1, Ordering::Relaxed);
        let header = MessageHeader::new(MessageType::StatusUpdate, StatusUpdateMessage::SIZE as u16, seq);
        
        let mut buffer = Vec::with_capacity(MessageHeader::SIZE + StatusUpdateMessage::SIZE);
        buffer.extend_from_slice(AsBytes::as_bytes(&header));
        buffer.extend_from_slice(AsBytes::as_bytes(status));
        
        self.sender.send(buffer)
            .context("Failed to send status to writer thread")?;
        
        counter!("unix_socket.status_sent").increment(1);
        
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
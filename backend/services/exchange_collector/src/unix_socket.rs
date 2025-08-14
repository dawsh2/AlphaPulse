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
    
    pub async fn start(&self) -> Result<()> {
        let socket_path = Path::new(&self.path);
        if let Some(parent) = socket_path.parent() {
            tokio::fs::create_dir_all(parent).await
                .context("Failed to create socket directory")?;
        }
        
        if socket_path.exists() {
            tokio::fs::remove_file(&socket_path).await
                .context("Failed to remove existing socket")?;
        }
        
        info!("Unix socket writer initialized at {}", self.path);
        Ok(())
    }
    
    pub fn write_trade(&self, trade: &TradeMessage) -> Result<()> {
        let seq = self.sequence.fetch_add(1, Ordering::Relaxed);
        let header = MessageHeader::new(MessageType::Trade, TradeMessage::SIZE as u16, seq);
        
        let mut buffer = Vec::with_capacity(MessageHeader::SIZE + TradeMessage::SIZE);
        buffer.extend_from_slice(AsBytes::as_bytes(&header));
        buffer.extend_from_slice(AsBytes::as_bytes(trade));
        
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
        
        loop {
            match self.receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(data) => {
                    if let Err(e) = self.write_data(&data) {
                        error!("Failed to write to unix socket: {}", e);
                        self.stream = None;
                    }
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                    self.send_heartbeat();
                }
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                    info!("Writer thread shutting down");
                    break;
                }
            }
        }
    }
    
    fn ensure_connected(&mut self) -> Result<()> {
        if self.stream.is_some() {
            return Ok(());
        }
        
        let socket_path = Path::new(&self.path);
        
        if !socket_path.exists() {
            use std::os::unix::net::UnixListener;
            
            if let Some(parent) = socket_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            
            let listener = UnixListener::bind(&self.path)
                .context("Failed to bind Unix socket")?;
            
            info!("Waiting for connection on {}", self.path);
            
            let (stream, _) = listener.accept()
                .context("Failed to accept connection")?;
            
            stream.set_nonblocking(false)?;
            
            // Just drop the listener to close it
            drop(listener);
            
            std::fs::remove_file(&self.path).ok();
            
            self.stream = Some(stream);
            info!("Client connected to Unix socket");
        } else {
            let stream = UnixStream::connect(&self.path)
                .context("Failed to connect to Unix socket")?;
            
            stream.set_nonblocking(false)?;
            self.stream = Some(stream);
            info!("Connected to existing Unix socket");
        }
        
        Ok(())
    }
    
    fn write_data(&mut self, data: &[u8]) -> Result<()> {
        self.ensure_connected()?;
        
        if let Some(ref mut stream) = self.stream {
            use std::io::Write;
            
            let start = Instant::now();
            stream.write_all(data)?;
            stream.flush()?;
            
            let latency_us = start.elapsed().as_micros() as f64;
            histogram!("unix_socket.write_latency_us").record(latency_us);
            
            self.bytes_written.fetch_add(data.len() as u64, Ordering::Relaxed);
            self.messages_written.fetch_add(1, Ordering::Relaxed);
            
            debug!("Wrote {} bytes to Unix socket", data.len());
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
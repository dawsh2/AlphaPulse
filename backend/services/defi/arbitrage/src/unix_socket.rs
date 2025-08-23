// Unix socket integration for connecting to relay server
// Provides same data stream as frontend dashboard via relay server

use alphapulse_protocol::{*, StatusUpdateMessage};
use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::net::UnixStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Unix socket client that connects to the relay server
/// Receives the same data stream as the frontend dashboard
pub struct UnixSocketClient {
    socket_path: String,
    stream: Option<UnixStream>,
}

impl UnixSocketClient {
    pub fn new() -> Self {
        Self {
            socket_path: "/tmp/alphapulse/relay.sock".to_string(),
            stream: None,
        }
    }
    
    /// Connect to the relay server via Unix socket
    pub async fn connect(&mut self) -> Result<()> {
        info!("Connecting to relay server at {}", self.socket_path);
        
        let stream = UnixStream::connect(&self.socket_path).await
            .context(format!("Failed to connect to relay server at {}", self.socket_path))?;
            
        self.stream = Some(stream);
        info!("✅ Connected to relay server via Unix socket");
        
        Ok(())
    }
    
    /// Start receiving messages from the relay server
    /// Returns a channel receiver for incoming messages
    pub async fn start_receiving(&mut self) -> Result<mpsc::UnboundedReceiver<RelayMessage>> {
        let stream = self.stream.take()
            .ok_or_else(|| anyhow::anyhow!("Not connected to relay server"))?;
            
        let (tx, rx) = mpsc::unbounded_channel();
        
        // Spawn task to read messages from Unix socket
        tokio::spawn(async move {
            if let Err(e) = Self::receive_loop(stream, tx).await {
                error!("Unix socket receive loop failed: {}", e);
            }
        });
        
        Ok(rx)
    }
    
    /// Main receive loop for reading messages from Unix socket
    async fn receive_loop(
        mut stream: UnixStream,
        tx: mpsc::UnboundedSender<RelayMessage>,
    ) -> Result<()> {
        let mut buffer = vec![0u8; 65536]; // 64KB buffer
        
        loop {
            // Read message length (4 bytes)
            let mut len_buf = [0u8; 4];
            stream.read_exact(&mut len_buf).await
                .context("Failed to read message length")?;
            
            let msg_len = u32::from_le_bytes(len_buf) as usize;
            
            if msg_len > buffer.len() {
                buffer.resize(msg_len, 0);
            }
            
            // Read message data
            stream.read_exact(&mut buffer[..msg_len]).await
                .context("Failed to read message data")?;
            
            // Parse message based on type byte
            if msg_len > 0 {
                match buffer[0] {
                    0x01 => {
                        // Trade message
                        if msg_len == 64 {
                            if let Some(trade) = TradeMessage::read_from(&buffer[..64]) {
                                tx.send(RelayMessage::Trade(trade.clone()))?;
                            }
                        }
                    }
                    0x02 => {
                        // OrderBook message
                        if let Some(orderbook) = OrderBookMessage::read_from(&buffer[..msg_len]) {
                            tx.send(RelayMessage::OrderBook(orderbook.clone()))?;
                        }
                    }
                    0x03 => {
                        // L2 Snapshot
                        if let Some(snapshot) = L2SnapshotMessage::read_from(&buffer[..msg_len]) {
                            tx.send(RelayMessage::L2Snapshot(snapshot.clone()))?;
                        }
                    }
                    0x04 => {
                        // L2 Delta
                        if let Some(delta) = L2DeltaMessage::read_from(&buffer[..msg_len]) {
                            tx.send(RelayMessage::L2Delta(delta.clone()))?;
                        }
                    }
                    0x05 => {
                        // Symbol mapping
                        if msg_len == 64 {
                            if let Some(mapping) = SymbolMappingMessage::read_from(&buffer[..64]) {
                                tx.send(RelayMessage::SymbolMapping(mapping.clone()))?;
                            }
                        }
                    }
                    0x06 => {
                        // Arbitrage opportunity
                        if msg_len == 64 {
                            if let Some(arb) = ArbitrageOpportunityMessage::read_from(&buffer[..64]) {
                                tx.send(RelayMessage::ArbitrageOpportunity(arb.clone()))?;
                            }
                        }
                    }
                    0x07 => {
                        // Status update
                        if msg_len == 64 {
                            if let Some(status) = StatusUpdateMessage::read_from(&buffer[..64]) {
                                tx.send(RelayMessage::StatusUpdate(status.clone()))?;
                            }
                        }
                    }
                    _ => {
                        debug!("Unknown message type: 0x{:02x}", buffer[0]);
                    }
                }
            }
        }
    }
    
    /// Send an arbitrage opportunity to the relay server
    pub async fn send_arbitrage_opportunity(&mut self, opportunity: &ArbitrageOpportunityMessage) -> Result<()> {
        let stream = self.stream.as_mut()
            .ok_or_else(|| anyhow::anyhow!("Not connected to relay server"))?;
            
        let bytes = opportunity.as_bytes();
        
        // Send length prefix
        let len = bytes.len() as u32;
        stream.write_all(&len.to_le_bytes()).await?;
        
        // Send message
        stream.write_all(bytes).await?;
        stream.flush().await?;
        
        debug!("Sent arbitrage opportunity to relay server");
        Ok(())
    }
    
    /// Check if connected to relay server
    pub fn is_connected(&self) -> bool {
        self.stream.is_some()
    }
    
    /// Disconnect from relay server
    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(mut stream) = self.stream.take() {
            stream.shutdown().await?;
            info!("Disconnected from relay server");
        }
        Ok(())
    }
}

/// Enum for different types of messages from relay
#[derive(Debug, Clone)]
pub enum RelayMessage {
    Trade(TradeMessage),
    OrderBook(OrderBookMessage),
    L2Snapshot(L2SnapshotMessage),
    L2Delta(L2DeltaMessage),
    SymbolMapping(SymbolMappingMessage),
    ArbitrageOpportunity(ArbitrageOpportunityMessage),
    StatusUpdate(StatusUpdateMessage),
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_unix_socket_client_creation() {
        let client = UnixSocketClient::new();
        assert!(!client.is_connected());
    }
    
    #[tokio::test]
    #[ignore] // Requires relay server running
    async fn test_unix_socket_connection() {
        let mut client = UnixSocketClient::new();
        
        // This will fail unless relay server is running
        let result = client.connect().await;
        
        if result.is_ok() {
            assert!(client.is_connected());
            client.disconnect().await.unwrap();
            assert!(!client.is_connected());
        }
    }
}
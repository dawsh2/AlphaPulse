//! Simple SignalRelay service for live arbitrage pipeline
//! Creates Unix socket listener at /tmp/alphapulse/signals.sock
//! Accepts connections from flash arbitrage strategy and forwards to dashboard

use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{info, error, warn};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{RwLock, mpsc};

type ConsumerId = String;
type MessageBuffer = Vec<u8>;

struct SignalRelay {
    consumers: Arc<RwLock<HashMap<ConsumerId, mpsc::UnboundedSender<MessageBuffer>>>>,
    message_count: Arc<RwLock<u64>>,
}

impl SignalRelay {
    fn new() -> Self {
        Self {
            consumers: Arc::new(RwLock::new(HashMap::new())),
            message_count: Arc::new(RwLock::new(0)),
        }
    }
    
    async fn register_consumer(&self, consumer_id: ConsumerId, tx: mpsc::UnboundedSender<MessageBuffer>) {
        let mut consumers = self.consumers.write().await;
        consumers.insert(consumer_id.clone(), tx);
        info!("🔔 Registered signal consumer: {}", consumer_id);
    }
    
    async fn broadcast_signal(&self, signal: MessageBuffer) -> usize {
        let consumers = self.consumers.read().await;
        let mut sent_count = 0;
        
        for (consumer_id, tx) in consumers.iter() {
            if tx.send(signal.clone()).is_ok() {
                sent_count += 1;
            } else {
                warn!("Failed to send signal to consumer: {}", consumer_id);
            }
        }
        
        // Update signal count
        let mut count = self.message_count.write().await;
        *count += 1;
        
        info!("🎯 SignalRelay broadcasted signal #{} to {} consumers", 
              *count, sent_count);
        
        sent_count
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    info!("🔔 Starting SignalRelay (Domain 2)");
    info!("   Reliable signal relay for arbitrage opportunities");
    info!("   Unix socket: /tmp/alphapulse/signals.sock");
    
    // Remove existing socket
    let socket_path = "/tmp/alphapulse/signals.sock";
    if std::path::Path::new(socket_path).exists() {
        std::fs::remove_file(socket_path)?;
    }
    
    // Create signal relay
    let relay = Arc::new(SignalRelay::new());
    
    // Create Unix socket listener
    let listener = UnixListener::bind(socket_path)?;
    info!("✅ SignalRelay listening for connections");
    
    // Accept and handle connections
    let mut connection_id = 0;
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                connection_id += 1;
                let conn_id = format!("signal_conn_{}", connection_id);
                
                let relay_clone = Arc::clone(&relay);
                tokio::spawn(async move {
                    handle_signal_connection(stream, conn_id, relay_clone).await;
                });
            }
            Err(e) => {
                error!("❌ Failed to accept signal connection: {}", e);
            }
        }
    }
}

async fn handle_signal_connection(
    mut stream: UnixStream, 
    conn_id: String,
    relay: Arc<SignalRelay>
) {
    info!("🔔 New signal connection: {}", conn_id);
    
    // Check if this is a signal producer (strategy) or consumer (dashboard)
    let mut buffer = vec![0u8; 32768]; // 32KB buffer for signal messages
    let mut is_producer = false;
    let mut consumer_tx: Option<mpsc::UnboundedReceiver<MessageBuffer>> = None;
    
    loop {
        tokio::select! {
            // Handle incoming signals (from strategies)
            read_result = stream.read(&mut buffer) => {
                match read_result {
                    Ok(0) => {
                        info!("🔔 Signal connection {} disconnected", conn_id);
                        break;
                    }
                    Ok(n) => {
                        if !is_producer {
                            is_producer = true;
                            info!("🎯 {} identified as SIGNAL PRODUCER (arbitrage strategy)", conn_id);
                        }
                        
                        // Broadcast signal to all consumers (dashboard)
                        let signal = buffer[..n].to_vec();
                        let consumer_count = relay.broadcast_signal(signal).await;
                        
                        info!("🚀 Relayed arbitrage signal ({} bytes) from {} to {} dashboards", 
                              n, conn_id, consumer_count);
                    }
                    Err(e) => {
                        error!("❌ Signal read error on {}: {}", conn_id, e);
                        break;
                    }
                }
            }
            
            // Handle signal subscription (from dashboard)
            signal = async {
                if consumer_tx.is_none() {
                    // Create consumer channel on first subscription
                    let (tx, rx) = mpsc::unbounded_channel::<MessageBuffer>();
                    relay.register_consumer(conn_id.clone(), tx).await;
                    consumer_tx = Some(rx);
                    info!("📊 {} identified as SIGNAL CONSUMER (dashboard)", conn_id);
                }
                
                consumer_tx.as_mut().unwrap().recv().await
            } => {
                match signal {
                    Some(data) => {
                        // Send signal to dashboard
                        if let Err(e) = stream.write_all(&data).await {
                            warn!("❌ Failed to send signal to consumer {}: {}", conn_id, e);
                            break;
                        }
                    }
                    None => {
                        info!("🔔 Signal consumer channel closed for {}", conn_id);
                        break;
                    }
                }
            }
        }
    }
    
    info!("🔔 Signal connection {} handler terminated", conn_id);
}
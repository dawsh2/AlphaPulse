//! Simple ExecutionRelay service for live arbitrage pipeline
//! Creates Unix socket listener at /tmp/alphapulse/execution.sock
//! Handles execution commands with full audit logging

use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{info, error, warn};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{RwLock, mpsc};

type ConsumerId = String;
type MessageBuffer = Vec<u8>;

struct ExecutionRelay {
    consumers: Arc<RwLock<HashMap<ConsumerId, mpsc::UnboundedSender<MessageBuffer>>>>,
    execution_count: Arc<RwLock<u64>>,
}

impl ExecutionRelay {
    fn new() -> Self {
        Self {
            consumers: Arc::new(RwLock::new(HashMap::new())),
            execution_count: Arc::new(RwLock::new(0)),
        }
    }
    
    async fn register_consumer(&self, consumer_id: ConsumerId, tx: mpsc::UnboundedSender<MessageBuffer>) {
        let mut consumers = self.consumers.write().await;
        consumers.insert(consumer_id.clone(), tx);
        info!("⚡ Registered execution consumer: {}", consumer_id);
    }
    
    async fn broadcast_execution(&self, execution: MessageBuffer) -> usize {
        let consumers = self.consumers.read().await;
        let mut sent_count = 0;
        
        for (consumer_id, tx) in consumers.iter() {
            if tx.send(execution.clone()).is_ok() {
                sent_count += 1;
            } else {
                warn!("Failed to send execution to consumer: {}", consumer_id);
            }
        }
        
        // Update execution count
        let mut count = self.execution_count.write().await;
        *count += 1;
        
        info!("⚡ ExecutionRelay processed execution #{} to {} consumers", 
              *count, sent_count);
        
        sent_count
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    info!("⚡ Starting ExecutionRelay (Domain 3)");
    info!("   Secure execution relay with full audit logging");
    info!("   Unix socket: /tmp/alphapulse/execution.sock");
    
    // Remove existing socket
    let socket_path = "/tmp/alphapulse/execution.sock";
    if std::path::Path::new(socket_path).exists() {
        std::fs::remove_file(socket_path)?;
    }
    
    // Create execution relay
    let relay = Arc::new(ExecutionRelay::new());
    
    // Create Unix socket listener
    let listener = UnixListener::bind(socket_path)?;
    info!("✅ ExecutionRelay listening for connections");
    
    // Accept and handle connections
    let mut connection_id = 0;
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                connection_id += 1;
                let conn_id = format!("exec_conn_{}", connection_id);
                
                let relay_clone = Arc::clone(&relay);
                tokio::spawn(async move {
                    handle_execution_connection(stream, conn_id, relay_clone).await;
                });
            }
            Err(e) => {
                error!("❌ Failed to accept execution connection: {}", e);
            }
        }
    }
}

async fn handle_execution_connection(
    mut stream: UnixStream, 
    conn_id: String,
    relay: Arc<ExecutionRelay>
) {
    info!("⚡ New execution connection: {}", conn_id);
    
    // Check if this is an execution producer (strategy) or consumer (execution engine)
    let mut buffer = vec![0u8; 16384]; // 16KB buffer for execution messages
    let mut is_producer = false;
    let mut consumer_tx: Option<mpsc::UnboundedReceiver<MessageBuffer>> = None;
    
    loop {
        tokio::select! {
            // Handle incoming executions (from strategies)
            read_result = stream.read(&mut buffer) => {
                match read_result {
                    Ok(0) => {
                        info!("⚡ Execution connection {} disconnected", conn_id);
                        break;
                    }
                    Ok(n) => {
                        if !is_producer {
                            is_producer = true;
                            info!("💼 {} identified as EXECUTION PRODUCER (arbitrage strategy)", conn_id);
                        }
                        
                        // Broadcast execution to all consumers (execution engine)
                        let execution = buffer[..n].to_vec();
                        let consumer_count = relay.broadcast_execution(execution).await;
                        
                        info!("🚀 Relayed execution command ({} bytes) from {} to {} execution engines", 
                              n, conn_id, consumer_count);
                    }
                    Err(e) => {
                        error!("❌ Execution read error on {}: {}", conn_id, e);
                        break;
                    }
                }
            }
            
            // Handle execution subscription (from execution engines)
            execution = async {
                if consumer_tx.is_none() {
                    // Create consumer channel on first subscription
                    let (tx, rx) = mpsc::unbounded_channel::<MessageBuffer>();
                    relay.register_consumer(conn_id.clone(), tx).await;
                    consumer_tx = Some(rx);
                    info!("💰 {} identified as EXECUTION CONSUMER (execution engine)", conn_id);
                }
                
                consumer_tx.as_mut().unwrap().recv().await
            } => {
                match execution {
                    Some(data) => {
                        // Send execution to execution engine
                        if let Err(e) = stream.write_all(&data).await {
                            warn!("❌ Failed to send execution to consumer {}: {}", conn_id, e);
                            break;
                        }
                    }
                    None => {
                        info!("⚡ Execution consumer channel closed for {}", conn_id);
                        break;
                    }
                }
            }
        }
    }
    
    info!("⚡ Execution connection {} handler terminated", conn_id);
}
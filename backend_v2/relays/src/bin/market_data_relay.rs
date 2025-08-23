//! Production Market Data Relay
//! Direct socket-to-socket forwarding without timing-based classification

use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    info!("🚀 Starting Production Market Data Relay");
    info!("📋 Direct socket-to-socket forwarding (no timing heuristics)");

    // Create directory
    std::fs::create_dir_all("/tmp/alphapulse")?;

    // Remove existing socket
    let socket_path = "/tmp/alphapulse/market_data.sock";
    if std::path::Path::new(socket_path).exists() {
        std::fs::remove_file(socket_path)?;
    }

    // Create Unix socket listener
    let listener = UnixListener::bind(socket_path)?;
    info!("✅ Market Data Relay listening on: {}", socket_path);

    // Create broadcast channel for direct forwarding
    let (message_tx, _) = broadcast::channel::<Vec<u8>>(10000);
    let message_tx = Arc::new(message_tx);

    // Track connections
    let connection_id = Arc::new(std::sync::atomic::AtomicU64::new(0));

    // Accept all connections and handle them uniformly
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let id = connection_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                info!("📡 Connection {} established", id);

                let message_tx_clone = message_tx.clone();

                tokio::spawn(async move {
                    handle_connection(stream, id, message_tx_clone).await;
                });
            }
            Err(e) => {
                error!("Failed to accept connection: {}", e);
            }
        }
    }
}

async fn handle_connection(
    mut stream: tokio::net::UnixStream,
    connection_id: u64,
    message_tx: Arc<broadcast::Sender<Vec<u8>>>,
) {
    info!("🔗 Handling connection {} with direct forwarding", connection_id);
    
    let mut buffer = vec![0u8; 65536]; // 64KB buffer for TLV messages
    let mut message_count = 0u64;
    let mut consumer_rx = message_tx.subscribe();

    // Start both reading and writing tasks simultaneously
    let (mut read_stream, mut write_stream) = stream.into_split();
    
    // Reading task: forward any incoming data to broadcast channel
    let read_task = {
        let message_tx = message_tx.clone();
        tokio::spawn(async move {
            let mut read_buffer = vec![0u8; 65536];
            let mut read_count = 0u64;
            
            loop {
                match read_stream.read(&mut read_buffer).await {
                    Ok(0) => {
                        debug!("Connection {} read stream closed", connection_id);
                        break;
                    }
                    Ok(n) => {
                        read_count += 1;
                        
                        // Forward to broadcast channel immediately
                        let message_data = read_buffer[..n].to_vec();
                        if let Err(e) = message_tx.send(message_data) {
                            debug!("No broadcast subscribers: {}", e);
                        }

                        if read_count <= 5 {
                            let preview = &read_buffer[..std::cmp::min(32, n)];
                            info!("📨 Connection {} forwarded message {}: {} bytes, preview: {:?}",
                                  connection_id, read_count, n, preview);
                        } else if read_count % 1000 == 0 {
                            info!("📊 Connection {}: {} messages forwarded", connection_id, read_count);
                        }
                    }
                    Err(e) => {
                        error!("Connection {} read error: {}", connection_id, e);
                        break;
                    }
                }
            }
            
            info!("📤 Connection {} read task ended after {} messages", connection_id, read_count);
        })
    };

    // Writing task: send broadcast messages to this connection
    let write_task = {
        tokio::spawn(async move {
            let mut write_count = 0u64;
            
            loop {
                match consumer_rx.recv().await {
                    Ok(message_data) => {
                        if let Err(e) = write_stream.write_all(&message_data).await {
                            warn!("Failed to write to connection {}: {}", connection_id, e);
                            break;
                        }

                        write_count += 1;

                        if write_count <= 5 {
                            debug!("📤 Sent message {} to connection {}: {} bytes",
                                   write_count, connection_id, message_data.len());
                        } else if write_count % 1000 == 0 {
                            info!("📊 Connection {}: {} messages sent", connection_id, write_count);
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(dropped)) => {
                        warn!("Connection {} lagged, dropped {} messages", connection_id, dropped);
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        info!("📥 Broadcast channel closed for connection {}", connection_id);
                        break;
                    }
                }
            }
            
            info!("📥 Connection {} write task ended after {} messages", connection_id, write_count);
        })
    };

    // Wait for either task to complete
    tokio::select! {
        _ = read_task => {
            info!("🔗 Connection {} read task completed", connection_id);
        }
        _ = write_task => {
            info!("🔗 Connection {} write task completed", connection_id);
        }
    }

    info!("🔗 Connection {} fully closed", connection_id);
}

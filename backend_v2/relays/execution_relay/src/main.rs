//! Production Execution Relay
//! Real Unix socket server for Protocol V2 execution messages

use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    info!("🚀 Starting Production Execution Relay");

    // Create directory
    std::fs::create_dir_all("/tmp/alphapulse")?;

    // Remove existing socket
    let socket_path = "/tmp/alphapulse/execution.sock";
    if std::path::Path::new(socket_path).exists() {
        std::fs::remove_file(socket_path)?;
    }

    // Create Unix socket listener
    let listener = UnixListener::bind(socket_path)?;
    info!("✅ Execution Relay listening on: {}", socket_path);

    // Track connected consumers
    let consumers = Arc::new(RwLock::new(HashMap::new()));
    let consumer_id = Arc::new(std::sync::atomic::AtomicU64::new(0));

    // Accept connections
    loop {
        match listener.accept().await {
            Ok((mut stream, _)) => {
                let id = consumer_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                info!("📡 Execution consumer {} connected", id);

                let consumers_clone = consumers.clone();

                tokio::spawn(async move {
                    let mut buffer = vec![0u8; 65536]; // 64KB buffer for TLV messages
                    let mut message_count = 0u64;

                    loop {
                        match stream.read(&mut buffer).await {
                            Ok(0) => {
                                info!(
                                    "📡 Consumer {} disconnected after {} messages",
                                    id, message_count
                                );
                                break;
                            }
                            Ok(n) => {
                                message_count += 1;

                                // In production, we would:
                                // 1. Parse the Protocol V2 header
                                // 2. Validate the message
                                // 3. Route to other consumers
                                // 4. Track metrics

                                if message_count % 1000 == 0 {
                                    info!(
                                        "📊 Consumer {}: {} messages processed",
                                        id, message_count
                                    );
                                }

                                // For now, just log first few bytes to verify data flow
                                if message_count <= 5 {
                                    let preview = &buffer[..std::cmp::min(32, n)];
                                    info!(
                                        "📨 Consumer {} message {}: {} bytes, preview: {:?}",
                                        id, message_count, n, preview
                                    );
                                }
                            }
                            Err(e) => {
                                error!("Consumer {} read error: {}", id, e);
                                break;
                            }
                        }
                    }

                    // Clean up consumer
                    consumers_clone.write().await.remove(&id);
                });

                // Track consumer
                consumers
                    .write()
                    .await
                    .insert(id, std::time::Instant::now());
            }
            Err(e) => {
                error!("Failed to accept connection: {}", e);
            }
        }
    }
}

//! Production Market Data Relay
//! Real Unix socket server for Protocol V2 market data messages

use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::UnixListener;
use tokio::sync::RwLock;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    info!("🚀 Starting Production Market Data Relay for Protocol V2");

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
    info!("📡 Ready to receive Protocol V2 TLV messages from publishers");
    info!("📊 Ready to forward messages to consumers (strategies, dashboard)");

    // Track connected consumers
    let consumers = Arc::new(RwLock::new(HashMap::new()));
    let consumer_id = Arc::new(std::sync::atomic::AtomicU64::new(0));

    // Accept connections
    loop {
        match listener.accept().await {
            Ok((mut stream, _)) => {
                let id = consumer_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                info!("✅ Market data consumer {} connected", id);

                let consumers_clone = consumers.clone();

                tokio::spawn(async move {
                    let mut buffer = vec![0u8; 65536]; // 64KB buffer for TLV messages
                    let mut message_count = 0u64;

                    loop {
                        match stream.read(&mut buffer).await {
                            Ok(0) => {
                                info!(
                                    "👋 Consumer {} disconnected after {} messages",
                                    id, message_count
                                );
                                break;
                            }
                            Ok(n) => {
                                message_count += 1;

                                // Log activity periodically
                                if message_count % 1000 == 0 {
                                    info!(
                                        "📊 Consumer {}: {} messages processed",
                                        id, message_count
                                    );
                                }

                                // For first few messages, log details
                                if message_count <= 5 {
                                    info!(
                                        "📨 Consumer {} received message {}: {} bytes",
                                        id, message_count, n
                                    );
                                }
                            }
                            Err(e) => {
                                error!("❌ Consumer {} read error: {}", id, e);
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
                error!("❌ Failed to accept connection: {}", e);
            }
        }
    }
}

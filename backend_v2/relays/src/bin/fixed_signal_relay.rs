//! Fixed Signal Relay - Properly handles bidirectional message forwarding
//! Fixes the stream locking issue in the original implementation

use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    info!("🚀 Starting Fixed Signal Relay");

    // Create directory
    std::fs::create_dir_all("/tmp/alphapulse")?;

    // Remove existing socket
    let socket_path = "/tmp/alphapulse/signals.sock";
    if std::path::Path::new(socket_path).exists() {
        std::fs::remove_file(socket_path)?;
    }

    // Create Unix socket listener
    let listener = UnixListener::bind(socket_path)?;
    info!("✅ Signal Relay listening on: {}", socket_path);

    // Track connected consumers with their channels
    let consumers: Arc<RwLock<HashMap<u64, mpsc::UnboundedSender<Vec<u8>>>>> =
        Arc::new(RwLock::new(HashMap::new()));
    let consumer_id = Arc::new(std::sync::atomic::AtomicU64::new(0));

    // Accept connections
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let id = consumer_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                info!("📡 Signal consumer {} connected", id);

                let consumers_clone = consumers.clone();

                // Split the stream into read and write halves - THIS IS THE FIX!
                let (read_half, write_half) = stream.into_split();

                // Create a channel for this specific consumer
                let (consumer_tx, mut consumer_rx) = mpsc::unbounded_channel::<Vec<u8>>();

                // Track this consumer
                consumers.write().await.insert(id, consumer_tx);

                // Spawn read task - reads from this consumer and broadcasts to others
                let consumers_for_read = consumers_clone.clone();
                tokio::spawn(async move {
                    let mut reader = tokio::io::BufReader::new(read_half);
                    let mut buffer = vec![0u8; 65536]; // 64KB buffer
                    let mut message_count = 0u64;

                    loop {
                        match reader.read(&mut buffer).await {
                            Ok(0) => {
                                info!(
                                    "📡 Consumer {} disconnected after {} messages",
                                    id, message_count
                                );
                                break;
                            }
                            Ok(n) => {
                                message_count += 1;

                                // Log message details for debugging
                                if message_count <= 5 || message_count % 100 == 0 {
                                    let preview = &buffer[..std::cmp::min(32, n)];
                                    info!(
                                        "📨 Consumer {} message {}: {} bytes, preview: {:02x?}",
                                        id, message_count, n, preview
                                    );
                                }

                                // Forward message to all OTHER consumers
                                let message_data = buffer[..n].to_vec();
                                let consumers_read = consumers_for_read.read().await;
                                let mut forwarded_count = 0;

                                for (other_id, other_tx) in consumers_read.iter() {
                                    if *other_id != id {
                                        // Don't send back to sender
                                        if let Err(e) = other_tx.send(message_data.clone()) {
                                            warn!(
                                                "Failed to forward to consumer {}: {:?}",
                                                other_id, e
                                            );
                                        } else {
                                            forwarded_count += 1;
                                            debug!(
                                                "Forwarded {} bytes to consumer {}",
                                                n, other_id
                                            );
                                        }
                                    }
                                }

                                if forwarded_count > 0 {
                                    info!(
                                        "📡 Forwarded message from consumer {} to {} others",
                                        id, forwarded_count
                                    );
                                } else if consumers_read.len() > 1 {
                                    debug!(
                                        "No other consumers to forward to (total consumers: {})",
                                        consumers_read.len()
                                    );
                                }
                            }
                            Err(e) => {
                                error!("Consumer {} read error: {}", id, e);
                                break;
                            }
                        }
                    }

                    // Clean up consumer on disconnect
                    consumers_for_read.write().await.remove(&id);
                    info!("🔌 Consumer {} removed from registry", id);
                });

                // Spawn write task - writes messages from other consumers to this one
                tokio::spawn(async move {
                    let mut writer = tokio::io::BufWriter::new(write_half);
                    let mut write_count = 0u64;

                    while let Some(message_data) = consumer_rx.recv().await {
                        write_count += 1;

                        // Debug logging for write operations
                        if write_count <= 5 || write_count % 100 == 0 {
                            debug!(
                                "Writing message {} ({} bytes) to consumer {}",
                                write_count,
                                message_data.len(),
                                id
                            );
                        }

                        if let Err(e) = writer.write_all(&message_data).await {
                            error!("Failed to write to consumer {}: {}", id, e);
                            break;
                        }

                        // IMPORTANT: Flush the writer to ensure data is sent immediately!
                        if let Err(e) = writer.flush().await {
                            error!("Failed to flush writer for consumer {}: {}", id, e);
                            break;
                        }

                        debug!(
                            "Successfully wrote and flushed {} bytes to consumer {}",
                            message_data.len(),
                            id
                        );
                    }

                    info!(
                        "📡 Consumer {} write task ended after {} messages",
                        id, write_count
                    );
                });
            }
            Err(e) => {
                error!("Failed to accept connection: {}", e);
            }
        }
    }
}

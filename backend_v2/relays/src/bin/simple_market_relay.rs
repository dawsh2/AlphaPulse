//! Simple market data relay server for testing
//! Just creates the Unix socket that the flash arbitrage strategy needs

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    info!("🚀 Starting Simple Market Data Relay");

    // Create directory
    std::fs::create_dir_all("/tmp/alphapulse")?;

    // Remove existing socket
    if std::path::Path::new("/tmp/alphapulse/market_data.sock").exists() {
        std::fs::remove_file("/tmp/alphapulse/market_data.sock")?;
    }

    // Create Unix socket listener
    let listener = UnixListener::bind("/tmp/alphapulse/market_data.sock")?;
    info!("✅ Market data relay listening on: /tmp/alphapulse/market_data.sock");

    // Accept connections
    loop {
        match listener.accept().await {
            Ok((mut stream, _)) => {
                info!("📡 Market data consumer connected");

                tokio::spawn(async move {
                    let mut buffer = vec![0u8; 4096];
                    loop {
                        match stream.read(&mut buffer).await {
                            Ok(0) => {
                                info!("📡 Market data consumer disconnected");
                                break;
                            }
                            Ok(n) => {
                                info!("📨 Received market data message: {} bytes", n);
                                // Just acknowledge receipt for testing
                                if let Err(e) = stream.write_all(b"OK").await {
                                    warn!("Failed to ack market data: {}", e);
                                    break;
                                }
                            }
                            Err(e) => {
                                error!("Market data read error: {}", e);
                                break;
                            }
                        }
                    }
                });
            }
            Err(e) => {
                error!("Failed to accept market data connection: {}", e);
            }
        }
    }
}

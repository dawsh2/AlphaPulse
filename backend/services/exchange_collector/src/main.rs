mod exchanges;
mod unix_socket;

use alphapulse_protocol::*;
use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("exchange_collector=debug".parse()?)
                .add_directive("info".parse()?),
        )
        .init();

    info!("Starting exchange collector service");

    // Use exchange-specific socket path
    let exchange_name = std::env::var("EXCHANGE_NAME").unwrap_or_else(|_| "coinbase".to_string());
    let socket_path = format!("/tmp/alphapulse/{}.sock", exchange_name);
    
    let socket_writer = Arc::new(unix_socket::UnixSocketWriter::new(&socket_path));
    socket_writer.start().await?;

    let symbol_mapper = Arc::new(parking_lot::RwLock::new(SymbolMapper::new()));

    let collector_handle = match exchange_name.as_str() {
        "kraken" => {
            let kraken = exchanges::kraken::KrakenCollector::new(
                socket_writer.clone(),
                symbol_mapper.clone(),
            );
            tokio::spawn(async move {
                loop {
                    match kraken.connect_and_stream().await {
                        Ok(_) => {
                            info!("Kraken collector disconnected, attempting immediate reconnect");
                        }
                        Err(e) => {
                            error!("Kraken collector error: {}", e);
                        }
                    }
                    // No sleep - immediate reconnect for event-driven system
                    // Connection failures will be handled by tokio-tungstenite's built-in backoff
                }
            })
        }
        "coinbase" => {
            let coinbase = exchanges::coinbase::CoinbaseCollector::new(
                socket_writer.clone(),
                symbol_mapper.clone(),
            );
            tokio::spawn(async move {
                loop {
                    match coinbase.connect_and_stream().await {
                        Ok(_) => {
                            info!("Coinbase collector disconnected, attempting immediate reconnect");
                        }
                        Err(e) => {
                            error!("Coinbase collector error: {}", e);
                        }
                    }
                    // No sleep - immediate reconnect for event-driven system
                    // Connection failures will be handled by tokio-tungstenite's built-in backoff
                }
            })
        }
        _ => {
            error!("Unsupported exchange: {}", exchange_name);
            return Ok(());
        }
    };

    let metrics_handle = tokio::spawn(async move {
        metrics_server().await;
    });

    info!("Exchange collector running. Press Ctrl+C to stop.");
    
    signal::ctrl_c().await?;
    
    info!("Shutting down exchange collector");
    collector_handle.abort();
    metrics_handle.abort();
    
    Ok(())
}

async fn metrics_server() {
    use metrics_exporter_prometheus::PrometheusBuilder;
    use std::net::SocketAddr;

    let addr: SocketAddr = "127.0.0.1:9090".parse().unwrap();
    let builder = PrometheusBuilder::new();
    
    match builder.with_http_listener(addr).install() {
        Ok(_) => info!("Metrics server listening on {}", addr),
        Err(e) => error!("Failed to start metrics server: {}", e),
    }
}
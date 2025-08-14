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
    let exchange_name = std::env::var("EXCHANGE_NAME").unwrap_or_else(|_| "kraken".to_string());
    let socket_path = format!("/tmp/alphapulse/{}.sock", exchange_name);
    
    let socket_writer = Arc::new(unix_socket::UnixSocketWriter::new(&socket_path));
    socket_writer.start().await?;

    let symbol_mapper = Arc::new(parking_lot::RwLock::new(SymbolMapper::new()));

    let kraken = exchanges::kraken::KrakenCollector::new(
        socket_writer.clone(),
        symbol_mapper.clone(),
    );

    let kraken_handle = tokio::spawn(async move {
        loop {
            match kraken.connect_and_stream().await {
                Ok(_) => {
                    info!("Kraken collector disconnected, reconnecting in 5s");
                }
                Err(e) => {
                    error!("Kraken collector error: {}", e);
                }
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });

    let metrics_handle = tokio::spawn(async move {
        metrics_server().await;
    });

    info!("Exchange collector running. Press Ctrl+C to stop.");
    
    signal::ctrl_c().await?;
    
    info!("Shutting down exchange collector");
    kraken_handle.abort();
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
mod exchanges;
mod instruments;
mod unix_socket;
mod validation;
mod token_registry;
mod pool_discovery;
mod dex_registry;
mod graph_client;

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

    // TEMPORARY: Force Polygon mode for debugging
    let exchange_name = std::env::var("EXCHANGE_NAME").unwrap_or_else(|_| "polygon".to_string());
    // Exchange collectors write to their specific socket
    let socket_path = format!("/tmp/alphapulse/{}.sock", exchange_name);
    
    let socket_writer = Arc::new(unix_socket::UnixSocketWriter::new(&socket_path));
    // Don't call start() - we connect TO the relay server, not create our own socket

    // Placeholder for legacy signature - will be removed
    let symbol_mapper = Arc::new(parking_lot::RwLock::new(std::collections::HashMap::<String, u32>::new()));

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
        "alpaca" => {
            match exchanges::alpaca::AlpacaCollector::new(
                socket_writer.clone(),
                symbol_mapper.clone(),
            ) {
                Ok(alpaca) => {
                    tokio::spawn(async move {
                        loop {
                            match alpaca.connect_and_stream().await {
                                Ok(_) => {
                                    info!("Alpaca collector disconnected, attempting immediate reconnect");
                                }
                                Err(e) => {
                                    error!("Alpaca collector error: {}", e);
                                }
                            }
                            // No sleep - immediate reconnect for event-driven system
                            // Connection failures will be handled by tokio-tungstenite's built-in backoff
                        }
                    })
                }
                Err(e) => {
                    error!("Failed to create Alpaca collector: {}", e);
                    return Ok(());
                }
            }
        }
        "polygon" => {
            let polygon = exchanges::polygon::PolygonCollector::new(socket_writer.clone());
            tokio::spawn(async move {
                loop {
                    match polygon.start().await {
                        Ok(_) => {
                            info!("Polygon collector disconnected, attempting immediate reconnect");
                        }
                        Err(e) => {
                            let error_msg = e.to_string();
                            error!("Polygon collector error: {}", error_msg);
                            
                            // Add delay for rate limiting errors
                            if error_msg.contains("429") || error_msg.contains("Too Many Requests") {
                                info!("Rate limited - waiting 10 seconds before reconnect");
                                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                            } else {
                                // Short delay for other errors
                                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                            }
                        }
                    }
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
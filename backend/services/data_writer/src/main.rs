use alphapulse_protocol::{
    MessageHeader, MessageType, TradeMessage, L2DeltaMessage, L2Update
};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use deadpool_postgres::{Config as DbConfig, Pool, Runtime};
use metrics::{counter, histogram};
use serde::Deserialize;
use serde_json;
use std::collections::HashMap;
use std::io::Read;
use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{error, info, warn};
use zerocopy::FromBytes;

#[derive(Debug, Deserialize)]
struct AppConfig {
    data_writer: DataWriterConfig,
    monitoring: MonitoringConfig,
    logging: LoggingConfig,
}

#[derive(Debug, Deserialize)]
struct DataWriterConfig {
    enabled: bool,
    relay: RelayConfig,
    capture: CaptureConfig,
    database: DatabaseConfig,
    parquet_export: ParquetExportConfig,
    symbols: SymbolConfig,
}

#[derive(Debug, Deserialize)]
struct RelayConfig {
    socket_path: String,
    reconnect_attempts: u32,
    reconnect_delay_ms: u64,
}

#[derive(Debug, Deserialize)]
struct CaptureConfig {
    trades: bool,
    l2_deltas: bool,
    l2_snapshots: bool,
    heartbeats: bool,
    metrics: bool,
    symbol_mappings: bool,
}

#[derive(Debug, Deserialize)]
struct DatabaseConfig {
    connection_string: String,
    max_connections: usize,
    min_connections: usize,
    connection_timeout_ms: u64,
    batch_size: usize,
    batch_timeout_ms: u64,
    max_queue_size: usize,
}

#[derive(Debug, Deserialize)]
struct ParquetExportConfig {
    enabled: bool,
    base_path: String,
    export_schedule: String,
    export_time: String,
    compression: String,
    cleanup_after_export: bool,
    retention_days: u32,
}

#[derive(Debug, Deserialize)]
struct SymbolConfig {
    resolve_hashes: bool,
    unknown_symbol_prefix: String,
}

#[derive(Debug, Deserialize)]
struct MonitoringConfig {
    enabled: bool,
    prometheus_port: u16,
    health_check_interval_seconds: u64,
}

#[derive(Debug, Deserialize)]
struct LoggingConfig {
    level: String,
    format: String,
    output: Vec<String>,
    file_path: String,
    max_file_size_mb: u32,
    max_files: u32,
}

#[derive(Debug, Clone)]
struct Trade {
    timestamp: DateTime<Utc>,
    exchange: String,
    symbol_hash: u64,
    symbol: String,
    price: Decimal,
    volume: Decimal,
    side: String,
}

#[derive(Debug, Clone)]
struct L2Delta {
    timestamp: DateTime<Utc>,
    exchange: String,
    symbol_hash: u64,
    symbol: String,
    sequence: u64,
    updates: serde_json::Value,
}

struct DataWriter {
    config: DataWriterConfig,
    db_pool: Pool,
    symbol_mappings: HashMap<u64, String>,
    trade_batch: Vec<Trade>,
    l2_delta_batch: Vec<L2Delta>,
    last_batch_time: Instant,
}

impl DataWriter {
    async fn new(config: DataWriterConfig) -> Result<Self> {
        // Setup database connection pool
        let mut db_config = DbConfig::new();
        db_config.url = Some(config.database.connection_string.clone());
        if let Some(pool_config) = &mut db_config.pool {
            pool_config.max_size = config.database.max_connections;
        }
        
        let db_pool = db_config
            .create_pool(Some(Runtime::Tokio1), tokio_postgres::NoTls)
            .context("Failed to create database pool")?;

        // Test database connection
        let client = db_pool.get().await.context("Failed to get database connection")?;
        client.execute("SELECT 1", &[]).await.context("Database health check failed")?;
        info!("Database connection established successfully");

        // Initialize symbol mappings (from existing hash generator)
        let symbol_mappings = Self::load_symbol_mappings();

        Ok(Self {
            config,
            db_pool,
            symbol_mappings,
            trade_batch: Vec::with_capacity(1000),
            l2_delta_batch: Vec::with_capacity(1000),
            last_batch_time: Instant::now(),
        })
    }

    fn load_symbol_mappings() -> HashMap<u64, String> {
        // Load from the same hash mappings we generated earlier
        let mut mappings = HashMap::new();
        mappings.insert(16842681295735137662, "coinbase:BTC-USD".to_string());
        mappings.insert(7334401999635196894, "coinbase:ETH-USD".to_string());
        mappings.insert(940696374048161387, "coinbase:SOL-USD".to_string());
        mappings.insert(2928176905300374322, "coinbase:LINK-USD".to_string());
        mappings.insert(1022169821381239205, "kraken:BTC-USD".to_string());
        mappings.insert(6206069765414077566, "kraken:ETH-USD".to_string());
        // Add more as needed
        mappings
    }

    async fn connect_to_relay(&self) -> Result<UnixStream> {
        for attempt in 1..=self.config.relay.reconnect_attempts {
            match UnixStream::connect(&self.config.relay.socket_path) {
                Ok(stream) => {
                    info!("Connected to relay server at {}", self.config.relay.socket_path);
                    return Ok(stream);
                }
                Err(e) => {
                    warn!("Failed to connect to relay (attempt {}/{}): {}", 
                          attempt, self.config.relay.reconnect_attempts, e);
                    
                    if attempt < self.config.relay.reconnect_attempts {
                        sleep(Duration::from_millis(self.config.relay.reconnect_delay_ms)).await;
                    }
                }
            }
        }
        
        anyhow::bail!("Failed to connect to relay after {} attempts", 
                      self.config.relay.reconnect_attempts);
    }

    async fn process_message(&mut self, header: MessageHeader, payload: &[u8]) -> Result<()> {
        match header.get_type()? {
            MessageType::Trade if self.config.capture.trades => {
                self.handle_trade_message(payload).await?;
            }
            MessageType::L2Delta if self.config.capture.l2_deltas => {
                self.handle_l2_delta_message(payload).await?;
            }
            _ => {
                // Skip other message types based on configuration
            }
        }

        // Check if we should flush batches
        if self.should_flush_batch() {
            self.flush_batches().await?;
        }

        Ok(())
    }

    async fn handle_trade_message(&mut self, payload: &[u8]) -> Result<()> {
        let trade_msg = TradeMessage::read_from_prefix(payload)
            .context("Failed to parse trade message")?;

        let symbol_hash = trade_msg.symbol_hash();
        let symbol = self.symbol_mappings.get(&symbol_hash)
            .cloned()
            .unwrap_or_else(|| format!("UNKNOWN_{}", symbol_hash));

        let exchange = symbol.split(':').next().unwrap_or("unknown").to_string();

        let trade = Trade {
            timestamp: DateTime::from_timestamp_nanos(trade_msg.timestamp_ns() as i64),
            exchange,
            symbol_hash,
            symbol: symbol.split(':').nth(1).unwrap_or(&symbol).to_string(),
            price: Decimal::from_f64_retain(trade_msg.price_f64()).unwrap_or_default(),
            volume: Decimal::from_f64_retain(trade_msg.volume_f64()).unwrap_or_default(),
            side: match trade_msg.side() {
                alphapulse_protocol::TradeSide::Buy => "buy".to_string(),
                alphapulse_protocol::TradeSide::Sell => "sell".to_string(),
                _ => "unknown".to_string(),
            },
        };

        self.trade_batch.push(trade);
        counter!("data_writer.trades_received").increment(1);

        Ok(())
    }

    async fn handle_l2_delta_message(&mut self, payload: &[u8]) -> Result<()> {
        let delta_msg = L2DeltaMessage::decode(payload)?;

        let symbol = self.symbol_mappings.get(&delta_msg.symbol_hash)
            .cloned()
            .unwrap_or_else(|| format!("UNKNOWN_{}", delta_msg.symbol_hash));

        let exchange = symbol.split(':').next().unwrap_or("unknown").to_string();

        // Convert updates to JSON for storage
        let bids: Vec<[f64; 2]> = delta_msg.updates.iter()
            .filter(|u| u.side == 0) // 0 = bid
            .map(|u| [u.price() as f64 / 1e8, u.volume() as f64 / 1e8])
            .collect();
        
        let asks: Vec<[f64; 2]> = delta_msg.updates.iter()
            .filter(|u| u.side == 1) // 1 = ask
            .map(|u| [u.price() as f64 / 1e8, u.volume() as f64 / 1e8])
            .collect();
        
        let updates_json = serde_json::json!({
            "bids": bids,
            "asks": asks
        });

        let l2_delta = L2Delta {
            timestamp: DateTime::from_timestamp_nanos(delta_msg.timestamp_ns as i64),
            exchange,
            symbol_hash: delta_msg.symbol_hash,
            symbol: symbol.split(':').nth(1).unwrap_or(&symbol).to_string(),
            sequence: delta_msg.sequence,
            updates: updates_json,
        };

        self.l2_delta_batch.push(l2_delta);
        counter!("data_writer.l2_deltas_received").increment(1);

        Ok(())
    }

    fn should_flush_batch(&self) -> bool {
        let batch_size_exceeded = self.trade_batch.len() >= self.config.database.batch_size ||
                                 self.l2_delta_batch.len() >= self.config.database.batch_size;
        
        let batch_timeout_exceeded = self.last_batch_time.elapsed() >= 
                                   Duration::from_millis(self.config.database.batch_timeout_ms);

        batch_size_exceeded || batch_timeout_exceeded
    }

    async fn flush_batches(&mut self) -> Result<()> {
        let start_time = Instant::now();

        if !self.trade_batch.is_empty() {
            self.write_trades_batch().await?;
            self.trade_batch.clear();
        }

        if !self.l2_delta_batch.is_empty() {
            self.write_l2_deltas_batch().await?;
            self.l2_delta_batch.clear();
        }

        self.last_batch_time = Instant::now();
        
        let flush_duration = start_time.elapsed();
        histogram!("data_writer.batch_flush_duration").record(flush_duration.as_secs_f64());

        Ok(())
    }

    async fn write_trades_batch(&mut self) -> Result<()> {
        let client = self.db_pool.get().await?;

        let stmt = "INSERT INTO market_data.trades (time, exchange, symbol_hash, symbol, price, volume, side) 
                   VALUES ($1, $2, $3, $4, $5, $6, $7)";

        for trade in &self.trade_batch {
            client.execute(stmt, &[
                &trade.timestamp,
                &trade.exchange,
                &(trade.symbol_hash as i64),
                &trade.symbol,
                &trade.price,
                &trade.volume,
                &trade.side,
            ]).await?;
        }

        counter!("data_writer.trades_written").increment(self.trade_batch.len() as u64);
        info!("Wrote {} trades to database", self.trade_batch.len());

        Ok(())
    }

    async fn write_l2_deltas_batch(&mut self) -> Result<()> {
        let client = self.db_pool.get().await?;

        let stmt = "INSERT INTO market_data.l2_deltas (time, exchange, symbol_hash, symbol, sequence, updates) 
                   VALUES ($1, $2, $3, $4, $5, $6)";

        for delta in &self.l2_delta_batch {
            client.execute(stmt, &[
                &delta.timestamp,
                &delta.exchange,
                &(delta.symbol_hash as i64),
                &delta.symbol,
                &(delta.sequence as i64),
                &delta.updates,
            ]).await?;
        }

        counter!("data_writer.l2_deltas_written").increment(self.l2_delta_batch.len() as u64);
        info!("Wrote {} L2 deltas to database", self.l2_delta_batch.len());

        Ok(())
    }

    async fn run(&mut self) -> Result<()> {
        info!("Starting data writer service");

        loop {
            match self.connect_to_relay().await {
                Ok(mut stream) => {
                    info!("Connected to relay server, starting message processing");
                    
                    loop {
                        // Read message header
                        let mut header_buf = [0u8; 8];
                        match stream.read_exact(&mut header_buf) {
                            Ok(_) => {
                                let header = MessageHeader::read_from_prefix(&header_buf)
                                    .context("Failed to parse message header")?;

                                header.validate()?;

                                // Read payload
                                let payload_len = header.get_length() as usize;
                                let mut payload_buf = vec![0u8; payload_len];
                                stream.read_exact(&mut payload_buf)?;

                                // Process message
                                if let Err(e) = self.process_message(header, &payload_buf).await {
                                    error!("Failed to process message: {}", e);
                                    counter!("data_writer.processing_errors").increment(1);
                                }
                            }
                            Err(e) => {
                                error!("Failed to read from relay socket: {}", e);
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to connect to relay: {}", e);
                    sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting AlphaPulse Data Writer");

    // Load configuration
    let config_content = std::fs::read_to_string("config/data_writer.yaml")
        .context("Failed to read config file")?;
    let config: AppConfig = serde_yaml::from_str(&config_content)
        .context("Failed to parse config file")?;

    if !config.data_writer.enabled {
        info!("Data writer is disabled in configuration");
        return Ok(());
    }

    // Initialize metrics exporter
    if config.monitoring.enabled {
        let builder = metrics_exporter_prometheus::PrometheusBuilder::new();
        builder.with_http_listener(([0, 0, 0, 0], config.monitoring.prometheus_port))
               .install()
               .context("Failed to install Prometheus exporter")?;
        info!("Prometheus metrics enabled on port {}", config.monitoring.prometheus_port);
    }

    // Create and run data writer
    let mut data_writer = DataWriter::new(config.data_writer).await?;
    data_writer.run().await?;

    Ok(())
}

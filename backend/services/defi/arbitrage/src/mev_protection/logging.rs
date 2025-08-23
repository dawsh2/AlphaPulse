use anyhow::Result;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_postgres::{Client, NoTls};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::{Strategy};
use super::production_mev::MarketContext;

/// MEV logging coordinator that writes to Redis for real-time access and TimescaleDB for analytics
pub struct MevLogger {
    redis_client: redis::Client,
    postgres_client: Option<Client>,
    table_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MevDecisionLog {
    pub id: String,
    pub timestamp_ns: i64,
    pub trade_id: Option<String>,
    pub profit_usd: f64,
    pub gas_gwei: f64,
    pub native_usd: f64,
    pub profit_ratio: f64,
    pub strategy: String,
    pub extraction_p70: f64,
    pub expected_mev_loss: f64,
    pub protection_cost: f64,
    pub market_context: MarketContextLog,
    pub bin_key: String,
    pub bin_samples: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketContextLog {
    pub gas_gwei: f64,
    pub block_fullness: f64,
    pub recent_mev_count: u32,
    pub mempool_latency_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MevOutcomeLog {
    pub id: String,
    pub timestamp_ns: i64,
    pub trade_id: String,
    pub decision_id: String,
    pub quoted_profit: f64,
    pub realized_profit: f64,
    pub used_protection: bool,
    pub protection_succeeded: bool,
    pub extraction_rate: f64,
    pub gas_gwei: f64,
    pub native_usd: f64,
    pub profit_ratio: f64,
    pub market_context: MarketContextLog,
    pub execution_time_ms: u64,
    pub block_number: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MevTransactionLog {
    pub id: String,
    pub timestamp_ns: i64,
    pub tx_hash: String,
    pub gas_used: u64,
    pub gas_price_gwei: f64,
    pub extracted_value_usd: f64,
    pub target_trade_size_usd: f64,
    pub extraction_method: String, // "sandwich", "front_run", "back_run", etc.
    pub block_number: u64,
    pub mempool_first_seen_ns: Option<i64>,
}

impl MevLogger {
    pub async fn new(redis_url: &str, postgres_url: Option<&str>) -> Result<Self> {
        let redis_client = redis::Client::open(redis_url)?;
        
        let postgres_client = if let Some(url) = postgres_url {
            match tokio_postgres::connect(url, NoTls).await {
                Ok((client, connection)) => {
                    // Spawn connection handling
                    tokio::spawn(async move {
                        if let Err(e) = connection.await {
                            error!("TimescaleDB connection error: {}", e);
                        }
                    });
                    
                    // Initialize tables
                    Self::init_timescale_tables(&client).await?;
                    Some(client)
                }
                Err(e) => {
                    warn!("Failed to connect to TimescaleDB: {}, continuing with Redis only", e);
                    None
                }
            }
        } else {
            None
        };

        info!("MEV logger initialized - Redis: connected, TimescaleDB: {}", 
              if postgres_client.is_some() { "connected" } else { "disabled" });

        Ok(Self {
            redis_client,
            postgres_client,
            table_name: "mev_logs".to_string(),
        })
    }

    /// Log MEV protection decision (called during decision making)
    pub async fn log_decision(
        &self,
        profit_usd: f64,
        gas_gwei: f64,
        native_usd: f64,
        profit_ratio: f64,
        strategy: Strategy,
        extraction_p70: f64,
        expected_mev_loss: f64,
        protection_cost: f64,
        market_context: &MarketContext,
        bin_key: &str,
        bin_samples: u32,
        trade_id: Option<String>,
    ) -> Result<String> {
        let decision_id = Uuid::new_v4().to_string();
        let timestamp_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_nanos() as i64;

        let log_entry = MevDecisionLog {
            id: decision_id.clone(),
            timestamp_ns,
            trade_id,
            profit_usd,
            gas_gwei,
            native_usd,
            profit_ratio,
            strategy: match strategy {
                Strategy::PublicFast => "public_fast".to_string(),
                Strategy::PrivateRelay => "private_relay".to_string(),
                Strategy::HybridAdaptive => "hybrid_adaptive".to_string(),
            },
            extraction_p70,
            expected_mev_loss,
            protection_cost,
            market_context: MarketContextLog {
                gas_gwei: market_context.current_gas_gwei,
                block_fullness: market_context.block_fullness,
                recent_mev_count: market_context.estimated_competitors,
                mempool_latency_ms: market_context.execution_speed_ms as u32,
            },
            bin_key: bin_key.to_string(),
            bin_samples,
        };

        // Write to Redis for real-time access
        self.write_to_redis("mev:decisions", &decision_id, &log_entry).await?;

        // Write to TimescaleDB for historical analysis
        if let Some(client) = &self.postgres_client {
            self.write_decision_to_timescale(client, &log_entry).await?;
        }

        debug!("Logged MEV decision: {} - strategy: {:?}, profit_ratio: {:.2}, expected_loss: ${:.2}",
               decision_id, strategy, profit_ratio, expected_mev_loss);

        Ok(decision_id)
    }

    /// Log MEV protection outcome (called after trade execution)
    pub async fn log_outcome(
        &self,
        decision_id: &str,
        trade_id: &str,
        quoted_profit: f64,
        realized_profit: f64,
        used_protection: bool,
        protection_succeeded: bool,
        gas_gwei: f64,
        native_usd: f64,
        market_context: &MarketContext,
        execution_time_ms: u64,
        block_number: Option<u64>,
    ) -> Result<String> {
        let outcome_id = Uuid::new_v4().to_string();
        let timestamp_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_nanos() as i64;

        let break_even = gas_gwei * 300_000.0 * 1e-9 * native_usd * 1.3; // Same calc as hybrid_mev
        let profit_ratio = quoted_profit / break_even;
        let extraction_rate = if quoted_profit > 0.0 {
            (1.0 - (realized_profit / quoted_profit).clamp(0.0, 1.0)).max(0.0)
        } else {
            0.0
        };

        let log_entry = MevOutcomeLog {
            id: outcome_id.clone(),
            timestamp_ns,
            trade_id: trade_id.to_string(),
            decision_id: decision_id.to_string(),
            quoted_profit,
            realized_profit,
            used_protection,
            protection_succeeded,
            extraction_rate,
            gas_gwei,
            native_usd,
            profit_ratio,
            market_context: MarketContextLog {
                gas_gwei: market_context.current_gas_gwei,
                block_fullness: market_context.block_fullness,
                recent_mev_count: market_context.estimated_competitors,
                mempool_latency_ms: market_context.execution_speed_ms as u32,
            },
            execution_time_ms,
            block_number,
        };

        // Write to Redis with expiration (keep recent outcomes for 1 hour)
        self.write_to_redis_with_expiry("mev:outcomes", &outcome_id, &log_entry, 3600).await?;

        // Write to TimescaleDB for historical analysis
        if let Some(client) = &self.postgres_client {
            self.write_outcome_to_timescale(client, &log_entry).await?;
        }

        info!("Logged MEV outcome: {} - extracted: {:.1}%, protection: {}, profit: ${:.2} -> ${:.2}",
              outcome_id, extraction_rate * 100.0, used_protection, quoted_profit, realized_profit);

        Ok(outcome_id)
    }

    /// Log observed MEV transaction (called when detecting MEV in mempool/blocks)
    pub async fn log_mev_transaction(
        &self,
        tx_hash: &str,
        gas_used: u64,
        gas_price_gwei: f64,
        extracted_value_usd: f64,
        target_trade_size_usd: f64,
        extraction_method: &str,
        block_number: u64,
        mempool_first_seen_ns: Option<i64>,
    ) -> Result<String> {
        let log_id = Uuid::new_v4().to_string();
        let timestamp_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_nanos() as i64;

        let log_entry = MevTransactionLog {
            id: log_id.clone(),
            timestamp_ns,
            tx_hash: tx_hash.to_string(),
            gas_used,
            gas_price_gwei,
            extracted_value_usd,
            target_trade_size_usd,
            extraction_method: extraction_method.to_string(),
            block_number,
            mempool_first_seen_ns,
        };

        // Write to Redis with longer expiration (keep MEV data for 24 hours)
        self.write_to_redis_with_expiry("mev:transactions", &log_id, &log_entry, 86400).await?;

        // Write to TimescaleDB for long-term analysis
        if let Some(client) = &self.postgres_client {
            self.write_mev_transaction_to_timescale(client, &log_entry).await?;
        }

        debug!("Logged MEV transaction: {} - method: {}, extracted: ${:.2}, gas: {}",
               tx_hash, extraction_method, extracted_value_usd, gas_used);

        Ok(log_id)
    }

    /// Get recent MEV decisions for real-time monitoring
    pub async fn get_recent_decisions(&self, limit: usize) -> Result<Vec<MevDecisionLog>> {
        let mut conn = self.redis_client.get_async_connection().await?;
        let keys: Vec<String> = conn.keys("mev:decisions:*").await?;
        
        let mut decisions = Vec::new();
        for key in keys.iter().take(limit) {
            if let Ok(data) = conn.get::<String, String>(key.to_string()).await {
                if let Ok(decision) = serde_json::from_str::<MevDecisionLog>(&data) {
                    decisions.push(decision);
                }
            }
        }

        // Sort by timestamp, newest first
        decisions.sort_by(|a, b| b.timestamp_ns.cmp(&a.timestamp_ns));
        decisions.truncate(limit);

        Ok(decisions)
    }

    /// Get MEV extraction statistics for threshold updates
    pub async fn get_extraction_stats(&self, hours_back: u32) -> Result<(f64, u32)> {
        if let Some(client) = &self.postgres_client {
            let cutoff_ns = SystemTime::now()
                .duration_since(UNIX_EPOCH)?
                .as_nanos() as i64 - (hours_back as i64 * 3600 * 1_000_000_000);

            let row = client
                .query_one(
                    "SELECT AVG(extraction_rate), COUNT(*) 
                     FROM mev_outcomes 
                     WHERE timestamp_ns > $1 AND extraction_rate > 0.1",
                    &[&cutoff_ns],
                )
                .await?;

            let avg_extraction: f64 = row.get(0);
            let count: i64 = row.get(1);

            Ok((avg_extraction, count as u32))
        } else {
            // Fallback to Redis approximation
            let mut conn = self.redis_client.get_async_connection().await?;
            let keys: Vec<String> = conn.keys("mev:outcomes:*").await?;
            
            let mut extractions = Vec::new();
            for key in keys {
                if let Ok(data) = conn.get::<String, String>(key).await {
                    if let Ok(outcome) = serde_json::from_str::<MevOutcomeLog>(&data) {
                        if outcome.extraction_rate > 0.1 {
                            extractions.push(outcome.extraction_rate);
                        }
                    }
                }
            }

            let avg = if extractions.is_empty() {
                0.0
            } else {
                extractions.iter().sum::<f64>() / extractions.len() as f64
            };

            Ok((avg, extractions.len() as u32))
        }
    }

    // Private helper methods

    async fn write_to_redis<T: Serialize>(&self, prefix: &str, id: &str, data: &T) -> Result<()> {
        let mut conn = self.redis_client.get_async_connection().await?;
        let key = format!("{}:{}", prefix, id);
        let json = serde_json::to_string(data)?;
        conn.set(&key, json).await?;
        Ok(())
    }

    async fn write_to_redis_with_expiry<T: Serialize>(
        &self, 
        prefix: &str, 
        id: &str, 
        data: &T, 
        expiry_seconds: usize
    ) -> Result<()> {
        let mut conn = self.redis_client.get_async_connection().await?;
        let key = format!("{}:{}", prefix, id);
        let json = serde_json::to_string(data)?;
        conn.set_ex(&key, json, expiry_seconds as u64).await?;
        Ok(())
    }

    async fn init_timescale_tables(client: &Client) -> Result<()> {
        // Create MEV decisions table
        client
            .execute(
                "CREATE TABLE IF NOT EXISTS mev_decisions (
                    id TEXT PRIMARY KEY,
                    timestamp_ns BIGINT NOT NULL,
                    trade_id TEXT,
                    profit_usd DOUBLE PRECISION,
                    gas_gwei DOUBLE PRECISION,
                    native_usd DOUBLE PRECISION,
                    profit_ratio DOUBLE PRECISION,
                    strategy TEXT,
                    extraction_p70 DOUBLE PRECISION,
                    expected_mev_loss DOUBLE PRECISION,
                    protection_cost DOUBLE PRECISION,
                    market_gas_gwei DOUBLE PRECISION,
                    market_block_fullness DOUBLE PRECISION,
                    market_recent_mev_count INTEGER,
                    market_mempool_latency_ms INTEGER,
                    bin_key TEXT,
                    bin_samples INTEGER
                )",
                &[],
            )
            .await?;

        // Create MEV outcomes table
        client
            .execute(
                "CREATE TABLE IF NOT EXISTS mev_outcomes (
                    id TEXT PRIMARY KEY,
                    timestamp_ns BIGINT NOT NULL,
                    trade_id TEXT NOT NULL,
                    decision_id TEXT NOT NULL,
                    quoted_profit DOUBLE PRECISION,
                    realized_profit DOUBLE PRECISION,
                    used_protection BOOLEAN,
                    protection_succeeded BOOLEAN,
                    extraction_rate DOUBLE PRECISION,
                    gas_gwei DOUBLE PRECISION,
                    native_usd DOUBLE PRECISION,
                    profit_ratio DOUBLE PRECISION,
                    market_gas_gwei DOUBLE PRECISION,
                    market_block_fullness DOUBLE PRECISION,
                    market_recent_mev_count INTEGER,
                    market_mempool_latency_ms INTEGER,
                    execution_time_ms BIGINT,
                    block_number BIGINT
                )",
                &[],
            )
            .await?;

        // Create MEV transactions table
        client
            .execute(
                "CREATE TABLE IF NOT EXISTS mev_transactions (
                    id TEXT PRIMARY KEY,
                    timestamp_ns BIGINT NOT NULL,
                    tx_hash TEXT NOT NULL,
                    gas_used BIGINT,
                    gas_price_gwei DOUBLE PRECISION,
                    extracted_value_usd DOUBLE PRECISION,
                    target_trade_size_usd DOUBLE PRECISION,
                    extraction_method TEXT,
                    block_number BIGINT,
                    mempool_first_seen_ns BIGINT
                )",
                &[],
            )
            .await?;

        // Create hypertables for time-series optimization
        let _ = client
            .execute(
                "SELECT create_hypertable('mev_decisions', 'timestamp_ns', if_not_exists => TRUE)",
                &[],
            )
            .await;

        let _ = client
            .execute(
                "SELECT create_hypertable('mev_outcomes', 'timestamp_ns', if_not_exists => TRUE)",
                &[],
            )
            .await;

        let _ = client
            .execute(
                "SELECT create_hypertable('mev_transactions', 'timestamp_ns', if_not_exists => TRUE)",
                &[],
            )
            .await;

        info!("TimescaleDB tables initialized for MEV logging");
        Ok(())
    }

    async fn write_decision_to_timescale(&self, client: &Client, log: &MevDecisionLog) -> Result<()> {
        client
            .execute(
                "INSERT INTO mev_decisions (
                    id, timestamp_ns, trade_id, profit_usd, gas_gwei, native_usd, profit_ratio,
                    strategy, extraction_p70, expected_mev_loss, protection_cost,
                    market_gas_gwei, market_block_fullness, market_recent_mev_count, 
                    market_mempool_latency_ms, bin_key, bin_samples
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)",
                &[
                    &log.id,
                    &log.timestamp_ns,
                    &log.trade_id,
                    &log.profit_usd,
                    &log.gas_gwei,
                    &log.native_usd,
                    &log.profit_ratio,
                    &log.strategy,
                    &log.extraction_p70,
                    &log.expected_mev_loss,
                    &log.protection_cost,
                    &log.market_context.gas_gwei,
                    &log.market_context.block_fullness,
                    &(log.market_context.recent_mev_count as i32),
                    &(log.market_context.mempool_latency_ms as i32),
                    &log.bin_key,
                    &(log.bin_samples as i32),
                ],
            )
            .await?;
        Ok(())
    }

    async fn write_outcome_to_timescale(&self, client: &Client, log: &MevOutcomeLog) -> Result<()> {
        client
            .execute(
                "INSERT INTO mev_outcomes (
                    id, timestamp_ns, trade_id, decision_id, quoted_profit, realized_profit,
                    used_protection, protection_succeeded, extraction_rate, gas_gwei, native_usd,
                    profit_ratio, market_gas_gwei, market_block_fullness, market_recent_mev_count,
                    market_mempool_latency_ms, execution_time_ms, block_number
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)",
                &[
                    &log.id,
                    &log.timestamp_ns,
                    &log.trade_id,
                    &log.decision_id,
                    &log.quoted_profit,
                    &log.realized_profit,
                    &log.used_protection,
                    &log.protection_succeeded,
                    &log.extraction_rate,
                    &log.gas_gwei,
                    &log.native_usd,
                    &log.profit_ratio,
                    &log.market_context.gas_gwei,
                    &log.market_context.block_fullness,
                    &(log.market_context.recent_mev_count as i32),
                    &(log.market_context.mempool_latency_ms as i32),
                    &(log.execution_time_ms as i64),
                    &log.block_number.map(|n| n as i64),
                ],
            )
            .await?;
        Ok(())
    }

    async fn write_mev_transaction_to_timescale(&self, client: &Client, log: &MevTransactionLog) -> Result<()> {
        client
            .execute(
                "INSERT INTO mev_transactions (
                    id, timestamp_ns, tx_hash, gas_used, gas_price_gwei, extracted_value_usd,
                    target_trade_size_usd, extraction_method, block_number, mempool_first_seen_ns
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
                &[
                    &log.id,
                    &log.timestamp_ns,
                    &log.tx_hash,
                    &(log.gas_used as i64),
                    &log.gas_price_gwei,
                    &log.extracted_value_usd,
                    &log.target_trade_size_usd,
                    &log.extraction_method,
                    &(log.block_number as i64),
                    &log.mempool_first_seen_ns,
                ],
            )
            .await?;
        Ok(())
    }
}
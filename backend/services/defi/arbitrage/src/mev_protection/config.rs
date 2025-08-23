use serde::{Deserialize, Serialize};
use std::env;

/// Configuration for MEV protection logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MevLoggingConfig {
    pub redis_url: String,
    pub postgres_url: Option<String>,
    pub log_decisions: bool,
    pub log_outcomes: bool,
    pub log_mev_transactions: bool,
    pub decision_expiry_seconds: usize,
    pub outcome_expiry_seconds: usize,
    pub mev_transaction_expiry_seconds: usize,
}

impl Default for MevLoggingConfig {
    fn default() -> Self {
        Self {
            redis_url: "redis://localhost:6379".to_string(),
            postgres_url: None,
            log_decisions: true,
            log_outcomes: true,
            log_mev_transactions: true,
            decision_expiry_seconds: 3600,      // 1 hour
            outcome_expiry_seconds: 3600,       // 1 hour  
            mev_transaction_expiry_seconds: 86400, // 24 hours
        }
    }
}

impl MevLoggingConfig {
    pub fn from_env() -> Self {
        let mut config = Self::default();
        
        if let Ok(redis_url) = env::var("MEV_REDIS_URL") {
            config.redis_url = redis_url;
        }
        
        if let Ok(postgres_url) = env::var("MEV_POSTGRES_URL") {
            config.postgres_url = Some(postgres_url);
        }
        
        if let Ok(log_decisions) = env::var("MEV_LOG_DECISIONS") {
            config.log_decisions = log_decisions.parse().unwrap_or(true);
        }
        
        if let Ok(log_outcomes) = env::var("MEV_LOG_OUTCOMES") {
            config.log_outcomes = log_outcomes.parse().unwrap_or(true);
        }
        
        if let Ok(log_mev_txs) = env::var("MEV_LOG_TRANSACTIONS") {
            config.log_mev_transactions = log_mev_txs.parse().unwrap_or(true);
        }
        
        config
    }
}
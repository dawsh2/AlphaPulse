// Application state for the API server
use alphapulse_common::{Result, MetricsCollector};
use crate::redis_client::RedisClient;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub redis: Arc<RedisClient>,
    pub metrics: Arc<MetricsCollector>,
}

impl AppState {
    pub async fn new() -> Result<Self> {
        let redis_url = std::env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://localhost:6379".to_string());
        
        let redis = Arc::new(RedisClient::new(&redis_url).await?);
        let metrics = Arc::new(MetricsCollector::new());
        
        Ok(Self { redis, metrics })
    }
}
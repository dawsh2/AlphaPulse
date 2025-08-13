// Health check handler
use axum::{extract::State, Json};
use serde_json::{json, Value};
use crate::state::AppState;

pub async fn health_check(State(state): State<AppState>) -> Json<Value> {
    // Test Redis connection
    let redis_healthy = state.redis.get_available_symbols("coinbase").await.is_ok();
    
    let response = json!({
        "status": "ok",
        "service": "alphapulse-api-server",
        "version": "0.1.0",
        "timestamp": chrono::Utc::now().timestamp(),
        "components": {
            "redis": if redis_healthy { "healthy" } else { "unhealthy" }
        }
    });
    
    // Record health check metric
    state.metrics.record_http_request("GET", "/health", 200);
    
    Json(response)
}
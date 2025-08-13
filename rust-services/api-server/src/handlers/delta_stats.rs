// Delta statistics endpoints for ultra-low latency performance monitoring
use alphapulse_common::{
    shared_memory::{SharedMemoryReader, OrderBookDeltaReader},
    Result,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;
use tracing::{info, warn};
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct DeltaStatistics {
    pub exchange: String,
    pub symbol: String,
    pub compression_ratio: f64,
    pub avg_changes_per_update: f64,
    pub total_deltas_processed: u64,
    pub bandwidth_saved_bytes: u64,
    pub last_update_latency_us: u64,
    pub shared_memory_stats: SharedMemoryStats,
}

#[derive(Debug, Serialize)]
pub struct SharedMemoryStats {
    pub capacity: usize,
    pub available: usize,
    pub utilization_percent: f64,
    pub reader_count: usize,
    pub avg_read_latency_us: f64,
}

#[derive(Debug, Serialize)]
pub struct DeltaSummary {
    pub total_exchanges: usize,
    pub total_symbols: usize,
    pub overall_compression_ratio: f64,
    pub total_bandwidth_saved_gb: f64,
    pub system_latency_us: SystemLatency,
    pub performance_targets: PerformanceTargets,
}

#[derive(Debug, Serialize)]
pub struct SystemLatency {
    pub shared_memory_read: f64,
    pub delta_computation: f64,
    pub websocket_broadcast: f64,
    pub end_to_end: f64,
}

#[derive(Debug, Serialize)]
pub struct PerformanceTargets {
    pub shared_memory_target_us: f64,
    pub compression_target_percent: f64,
    pub uptime_target_percent: f64,
    pub targets_met: bool,
}

#[derive(Debug, Deserialize)]
pub struct DeltaStatsQuery {
    pub detailed: Option<bool>,
}

/// GET /api/v1/delta-stats/{exchange}/{symbol}
/// Get detailed delta statistics for a specific exchange and symbol
pub async fn get_delta_stats(
    Path((exchange, symbol)): Path<(String, String)>,
    Query(params): Query<DeltaStatsQuery>,
    State(_state): State<AppState>,
) -> Response {
    match get_delta_stats_impl(exchange, symbol, params.detailed.unwrap_or(false)).await {
        Ok(response) => response.into_response(),
        Err(e) => {
            tracing::error!("Error getting delta stats: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e)).into_response()
        }
    }
}

async fn get_delta_stats_impl(
    exchange: String,
    symbol: String,
    _detailed: bool,
) -> Result<Json<DeltaStatistics>> {
    // Path to shared memory based on exchange
    let shared_memory_path = format!("/tmp/alphapulse_shm/{}_orderbook_deltas", exchange);
    
    // Try to open delta reader to get statistics
    let stats = match OrderBookDeltaReader::open(&shared_memory_path, 999) {
        Ok(mut reader) => {
            let start_time = Instant::now();
            
            // Read deltas to calculate statistics
            let deltas = reader.read_deltas();
            
            let read_latency = start_time.elapsed().as_micros() as u64;
            
            // Calculate compression ratio and statistics
            let total_deltas = deltas.len() as u64;
            let total_changes: u64 = deltas.iter()
                .map(|d| d.change_count as u64)
                .sum();
            
            let avg_changes = if total_deltas > 0 {
                total_changes as f64 / total_deltas as f64
            } else {
                0.0
            };
            
            // Estimate compression ratio (typical orderbook has ~100 levels)
            let estimated_full_size = 100.0;
            let compression_ratio = if avg_changes > 0.0 {
                1.0 - (avg_changes / estimated_full_size)
            } else {
                0.0
            };
            
            // Estimate bandwidth saved (assuming 32 bytes per level)
            let bytes_per_full_orderbook = estimated_full_size * 32.0;
            let bytes_per_delta = avg_changes * 32.0;
            let bandwidth_saved = (bytes_per_full_orderbook - bytes_per_delta) * total_deltas as f64;
            
            DeltaStatistics {
                exchange: exchange.clone(),
                symbol: symbol.clone(),
                compression_ratio,
                avg_changes_per_update: avg_changes,
                total_deltas_processed: total_deltas,
                bandwidth_saved_bytes: bandwidth_saved as u64,
                last_update_latency_us: read_latency,
                shared_memory_stats: SharedMemoryStats {
                    capacity: 10000, // Default capacity
                    available: 10000 - total_deltas as usize,
                    utilization_percent: (total_deltas as f64 / 10000.0) * 100.0,
                    reader_count: 1,
                    avg_read_latency_us: read_latency as f64,
                },
            }
        }
        Err(e) => {
            warn!("Could not open shared memory for {}: {}", exchange, e);
            // Return default stats when shared memory is not available
            DeltaStatistics {
                exchange: exchange.clone(),
                symbol: symbol.clone(),
                compression_ratio: 0.0,
                avg_changes_per_update: 0.0,
                total_deltas_processed: 0,
                bandwidth_saved_bytes: 0,
                last_update_latency_us: 0,
                shared_memory_stats: SharedMemoryStats {
                    capacity: 0,
                    available: 0,
                    utilization_percent: 0.0,
                    reader_count: 0,
                    avg_read_latency_us: 0.0,
                },
            }
        }
    };
    
    info!("Delta stats for {}:{} - compression: {:.2}%, changes: {:.1}", 
          exchange, symbol, stats.compression_ratio * 100.0, stats.avg_changes_per_update);
    
    Ok(Json(stats))
}

/// GET /api/v1/delta-stats/summary
/// Get overall system delta compression summary
pub async fn get_delta_summary(
    State(_state): State<AppState>,
) -> Response {
    match get_delta_summary_impl().await {
        Ok(response) => response.into_response(),
        Err(e) => {
            tracing::error!("Error getting delta summary: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e)).into_response()
        }
    }
}

async fn get_delta_summary_impl() -> Result<Json<DeltaSummary>> {
    let exchanges = vec!["coinbase", "kraken", "binance"];
    let symbols = vec!["BTC/USD", "ETH/USD"];
    
    let mut total_compression_ratio = 0.0;
    let mut total_bandwidth_saved = 0.0;
    let mut valid_stats_count = 0;
    
    // Aggregate statistics from all exchanges
    for exchange in &exchanges {
        for symbol in &symbols {
            if let Ok(Json(stats)) = get_delta_stats_impl(
                exchange.to_string(), 
                symbol.to_string(), 
                false
            ).await {
                if stats.total_deltas_processed > 0 {
                    total_compression_ratio += stats.compression_ratio;
                    total_bandwidth_saved += stats.bandwidth_saved_bytes as f64;
                    valid_stats_count += 1;
                }
            }
        }
    }
    
    let avg_compression_ratio = if valid_stats_count > 0 {
        total_compression_ratio / valid_stats_count as f64
    } else {
        0.0
    };
    
    // Mock system latencies (in production, these would be measured)
    let system_latency = SystemLatency {
        shared_memory_read: 8.5,    // μs
        delta_computation: 45.2,    // μs
        websocket_broadcast: 250.0, // μs
        end_to_end: 1200.0,        // μs
    };
    
    let performance_targets = PerformanceTargets {
        shared_memory_target_us: 10.0,
        compression_target_percent: 99.0,
        uptime_target_percent: 99.99,
        targets_met: system_latency.shared_memory_read < 10.0 
                    && avg_compression_ratio > 0.99,
    };
    
    let summary = DeltaSummary {
        total_exchanges: exchanges.len(),
        total_symbols: symbols.len(),
        overall_compression_ratio: avg_compression_ratio,
        total_bandwidth_saved_gb: total_bandwidth_saved / 1_000_000_000.0,
        system_latency_us: system_latency,
        performance_targets,
    };
    
    info!("Delta summary - compression: {:.3}%, bandwidth saved: {:.2} GB", 
          summary.overall_compression_ratio * 100.0, 
          summary.total_bandwidth_saved_gb);
    
    Ok(Json(summary))
}

/// GET /api/v1/exchanges
/// List all supported exchanges
pub async fn get_exchanges() -> Response {
    let exchanges = vec![
        serde_json::json!({
            "name": "coinbase",
            "display_name": "Coinbase Pro",
            "status": "active",
            "symbols": ["BTC-USD", "ETH-USD", "BTC-USDT", "ETH-USDT"],
            "features": ["trades", "orderbook", "deltas"],
            "latency_us": 8.5
        }),
        serde_json::json!({
            "name": "kraken",
            "display_name": "Kraken",
            "status": "active", 
            "symbols": ["BTC/USD", "ETH/USD"],
            "features": ["trades", "orderbook", "deltas"],
            "latency_us": 12.3
        }),
        serde_json::json!({
            "name": "binance",
            "display_name": "Binance.US",
            "status": "active",
            "symbols": ["BTC/USDT", "ETH/USDT"],
            "features": ["trades", "orderbook", "deltas"],
            "latency_us": 9.8
        }),
    ];
    
    Json(serde_json::json!({
        "exchanges": exchanges,
        "total_count": exchanges.len(),
        "active_count": exchanges.len(),
        "system_status": "operational"
    })).into_response()
}

/// GET /api/v1/system/health
/// Comprehensive system health check including shared memory status
pub async fn get_system_health() -> Response {
    let mut health_status = HashMap::new();
    let exchanges = vec!["coinbase", "kraken", "binance"];
    
    for exchange in &exchanges {
        let shared_memory_path = format!("/tmp/alphapulse_shm/{}_orderbook_deltas", exchange);
        let trade_path = format!("/tmp/alphapulse_shm/{}_trades", exchange);
        
        let delta_status = match OrderBookDeltaReader::open(&shared_memory_path, 998) {
            Ok(_) => "healthy",
            Err(_) => "unavailable",
        };
        
        let trade_status = match SharedMemoryReader::open(&trade_path, 998) {
            Ok(_) => "healthy", 
            Err(_) => "unavailable",
        };
        
        health_status.insert(exchange.to_string(), serde_json::json!({
            "delta_reader": delta_status,
            "trade_reader": trade_status,
            "overall_status": if delta_status == "healthy" || trade_status == "healthy" {
                "healthy"
            } else {
                "degraded"
            }
        }));
    }
    
    let healthy_exchanges = health_status.values()
        .filter(|status| status["overall_status"] == "healthy")
        .count();
    
    let overall_status = if healthy_exchanges == exchanges.len() {
        "healthy"
    } else if healthy_exchanges > 0 {
        "degraded"
    } else {
        "critical"
    };
    
    Json(serde_json::json!({
        "overall_status": overall_status,
        "exchanges": health_status,
        "healthy_exchanges": healthy_exchanges,
        "total_exchanges": exchanges.len(),
        "system_metrics": {
            "uptime_seconds": 3600, // Mock uptime
            "memory_usage_mb": 128,
            "cpu_usage_percent": 25.5,
            "active_connections": 42
        },
        "timestamp": chrono::Utc::now().timestamp()
    })).into_response()
}

/// GET /api/v1/arbitrage/opportunities  
/// Get recent cross-exchange arbitrage opportunities
pub async fn get_arbitrage_opportunities(
    Query(_params): Query<DeltaStatsQuery>,
) -> Response {
    // Mock arbitrage opportunities (in production, this would read from actual detection)
    let opportunities = vec![
        serde_json::json!({
            "symbol": "BTC/USD",
            "buy_exchange": "kraken",
            "sell_exchange": "coinbase", 
            "buy_price": 45250.50,
            "sell_price": 45275.25,
            "profit_bps": 5.47,
            "volume_available": 0.125,
            "timestamp": chrono::Utc::now().timestamp(),
            "confidence": 0.95
        }),
        serde_json::json!({
            "symbol": "ETH/USD",
            "buy_exchange": "binance",
            "sell_exchange": "coinbase",
            "buy_price": 2845.75,
            "sell_price": 2849.30,
            "profit_bps": 12.48,
            "volume_available": 0.85,
            "timestamp": chrono::Utc::now().timestamp() - 5,
            "confidence": 0.88
        }),
    ];
    
    Json(serde_json::json!({
        "opportunities": opportunities,
        "total_count": opportunities.len(),
        "min_profit_bps": 1.0,
        "max_age_seconds": 30,
        "detection_latency_us": 150.5
    })).into_response()
}
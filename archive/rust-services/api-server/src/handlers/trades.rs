// Trade data handlers - compatible with Python MarketDataRepository interface
use alphapulse_common::{Trade, DataSummary};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Instant;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct TradeQuery {
    exchange: Option<String>,
    start_time: Option<f64>,
    end_time: Option<f64>,
    limit: Option<usize>,
}

#[derive(Deserialize)]
pub struct OhlcvQuery {
    exchange: Option<String>,
    interval: Option<String>,
    start_time: Option<f64>,
    end_time: Option<f64>,
}

#[derive(Serialize)]
pub struct TradeResponse {
    pub data: Vec<Trade>,
    pub symbol: String,
    pub exchange: String,
    pub count: usize,
    pub processing_time_ms: f64,
}

#[derive(Serialize)]
pub struct OhlcvResponse {
    pub data: Vec<Value>, // OHLCV bars as JSON
    pub symbol: String,
    pub exchange: String,
    pub interval: String,
    pub count: usize,
    pub processing_time_ms: f64,
}

pub async fn get_trades(
    Path(symbol): Path<String>,
    Query(params): Query<TradeQuery>,
    State(state): State<AppState>,
) -> Result<Json<TradeResponse>, StatusCode> {
    let start_time = Instant::now();
    let exchange = params.exchange.as_deref().unwrap_or("coinbase");
    
    // Convert symbol format if needed (BTC-USD -> BTC/USD for standardization)
    let normalized_symbol = symbol.replace("-", "/");
    
    let trades = if params.start_time.is_some() || params.end_time.is_some() {
        // Range query
        state.redis
            .get_trades_in_range(
                &normalized_symbol,
                exchange,
                params.start_time,
                params.end_time,
                params.limit,
            )
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    } else {
        // Recent trades
        let limit = params.limit.unwrap_or(100);
        state.redis
            .get_recent_trades(&normalized_symbol, exchange, limit)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };
    
    let processing_time = start_time.elapsed().as_millis() as f64;
    
    // Record metrics
    state.metrics.record_http_request("/trades", 200);
    state.metrics.record_http_latency(processing_time, "/trades");
    
    let response = TradeResponse {
        data: trades.clone(),
        symbol: normalized_symbol,
        exchange: exchange.to_string(),
        count: trades.len(),
        processing_time_ms: processing_time,
    };
    
    Ok(Json(response))
}

pub async fn get_recent_trades(
    Path(symbol): Path<String>,
    Query(params): Query<TradeQuery>,
    State(state): State<AppState>,
) -> Result<Json<TradeResponse>, StatusCode> {
    let start_time = Instant::now();
    let exchange = params.exchange.as_deref().unwrap_or("coinbase");
    let limit = params.limit.unwrap_or(100);
    
    let normalized_symbol = symbol.replace("-", "/");
    
    let trades = state.redis
        .get_recent_trades(&normalized_symbol, exchange, limit)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let processing_time = start_time.elapsed().as_millis() as f64;
    
    // Record metrics
    state.metrics.record_http_request("/trades/recent", 200);
    state.metrics.record_http_latency(processing_time, "/trades/recent");
    
    let response = TradeResponse {
        data: trades.clone(),
        symbol: normalized_symbol,
        exchange: exchange.to_string(),
        count: trades.len(),
        processing_time_ms: processing_time,
    };
    
    Ok(Json(response))
}

pub async fn get_ohlcv(
    Path(symbol): Path<String>,
    Query(params): Query<OhlcvQuery>,
    State(state): State<AppState>,
) -> Result<Json<OhlcvResponse>, StatusCode> {
    let start_time = Instant::now();
    let exchange = params.exchange.as_deref().unwrap_or("coinbase");
    let interval = params.interval.as_deref().unwrap_or("1m");
    
    let normalized_symbol = symbol.replace("-", "/");
    
    // For now, convert trades to OHLCV (in production, you might store OHLCV separately)
    let trades = state.redis
        .get_trades_in_range(
            &normalized_symbol,
            exchange,
            params.start_time,
            params.end_time,
            Some(10000), // Limit for OHLCV calculation
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Convert trades to OHLCV bars (simplified implementation)
    let ohlcv_bars = convert_trades_to_ohlcv(&trades, interval);
    
    let processing_time = start_time.elapsed().as_millis() as f64;
    
    // Record metrics
    state.metrics.record_http_request("/ohlcv", 200);
    state.metrics.record_http_latency(processing_time, "/ohlcv");
    
    let response = OhlcvResponse {
        data: ohlcv_bars.clone(),
        symbol: normalized_symbol,
        exchange: exchange.to_string(),
        interval: interval.to_string(),
        count: ohlcv_bars.len(),
        processing_time_ms: processing_time,
    };
    
    Ok(Json(response))
}

pub async fn get_symbols(
    Path(exchange): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Value>, StatusCode> {
    let start_time = Instant::now();
    
    let symbols = state.redis
        .get_available_symbols(&exchange)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let processing_time = start_time.elapsed().as_millis() as f64;
    
    // Record metrics
    state.metrics.record_http_request("/symbols", 200);
    state.metrics.record_http_latency(processing_time, "/symbols");
    
    let response = json!({
        "symbols": symbols,
        "exchange": exchange,
        "count": symbols.len(),
        "processing_time_ms": processing_time
    });
    
    Ok(Json(response))
}

pub async fn get_data_summary(
    State(state): State<AppState>,
) -> Result<Json<DataSummary>, StatusCode> {
    let start_time = Instant::now();
    
    // Get symbols for each exchange
    let coinbase_symbols = state.redis
        .get_available_symbols("coinbase")
        .await
        .unwrap_or_default();
    
    let kraken_symbols = state.redis
        .get_available_symbols("kraken")
        .await
        .unwrap_or_default();
    
    let total_symbols = coinbase_symbols.len() + kraken_symbols.len();
    let total_exchanges = 2; // Coinbase and Kraken
    
    // Create symbols_by_exchange map
    let mut symbols_by_exchange = HashMap::new();
    symbols_by_exchange.insert("coinbase".to_string(), coinbase_symbols.clone());
    symbols_by_exchange.insert("kraken".to_string(), kraken_symbols.clone());
    
    // Create record count map (placeholder - in production, get actual counts)
    let mut record_count_by_symbol = HashMap::new();
    for symbol in &coinbase_symbols {
        record_count_by_symbol.insert(symbol.clone(), 0);
    }
    for symbol in &kraken_symbols {
        record_count_by_symbol.insert(symbol.clone(), 0);
    }
    
    let processing_time = start_time.elapsed().as_millis() as f64;
    
    // Record metrics
    state.metrics.record_http_request("/summary", 200);
    state.metrics.record_http_latency(processing_time, "/summary");
    
    let summary = DataSummary {
        total_trades: 0,
        total_orderbooks: 0, 
        start_time: 0.0,
        end_time: 0.0,
        total_symbols: Some(total_symbols as u64),
        total_exchanges: Some(total_exchanges),
        total_records: Some(0),
        date_range: Some("{}".to_string()),
    };
    
    Ok(Json(summary))
}

// Helper function to convert trades to OHLCV bars
fn convert_trades_to_ohlcv(trades: &[Trade], interval: &str) -> Vec<Value> {
    if trades.is_empty() {
        return Vec::new();
    }
    
    // Simple implementation: group trades by time intervals
    // In production, you'd want a more sophisticated implementation
    let interval_seconds = match interval {
        "1m" => 60,
        "5m" => 300,
        "15m" => 900,
        "1h" => 3600,
        "1d" => 86400,
        _ => 60, // Default to 1 minute
    };
    
    let mut bars = Vec::new();
    let mut current_bar: Option<Value> = None;
    let mut current_interval_start = 0;
    
    for trade in trades {
        let trade_interval_start = (trade.timestamp as i64 / interval_seconds) * interval_seconds;
        
        if current_interval_start != trade_interval_start {
            // Save previous bar if exists
            if let Some(bar) = current_bar.take() {
                bars.push(bar);
            }
            
            // Start new bar
            current_interval_start = trade_interval_start;
            current_bar = Some(json!({
                "timestamp": trade_interval_start,
                "open": trade.price,
                "high": trade.price,
                "low": trade.price,
                "close": trade.price,
                "volume": trade.volume
            }));
        } else if let Some(ref mut bar) = current_bar {
            // Update existing bar
            let high = bar["high"].as_f64().unwrap_or(0.0).max(trade.price);
            let low = bar["low"].as_f64().unwrap_or(f64::MAX).min(trade.price);
            let volume = bar["volume"].as_f64().unwrap_or(0.0) + trade.volume;
            
            bar["high"] = json!(high);
            bar["low"] = json!(low);
            bar["close"] = json!(trade.price);
            bar["volume"] = json!(volume);
        }
    }
    
    // Add final bar
    if let Some(bar) = current_bar {
        bars.push(bar);
    }
    
    bars
}
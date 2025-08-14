// Candle/OHLC data handlers for chart rendering
use alphapulse_common::{Trade, Result};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use crate::state::AppState;
use crate::parquet_reader::ParquetReader;
use tracing::info;

#[derive(Debug, Deserialize)]
pub struct CandleQuery {
    pub exchange: Option<String>,
    pub start: Option<i64>,  // Unix timestamp in seconds
    pub end: Option<i64>,    // Unix timestamp in seconds
    pub granularity: Option<u32>, // Seconds per candle (60=1m, 300=5m, 3600=1h)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Candle {
    pub time: i64,    // Unix timestamp in seconds
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Debug, Serialize)]
pub struct CandleResponse {
    pub candles: Vec<Candle>,
    pub symbol: String,
    pub exchange: String,
    pub granularity: u32,
}

// Get candles for a symbol
pub async fn get_candles(
    Path(symbol): Path<String>,
    Query(params): Query<CandleQuery>,
    State(state): State<AppState>,
) -> Response {
    match get_candles_impl(symbol, params, state).await {
        Ok(response) => response.into_response(),
        Err(e) => {
            tracing::error!("Error getting candles: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e)).into_response()
        }
    }
}

async fn get_candles_impl(
    symbol: String,
    params: CandleQuery,
    state: AppState,
) -> Result<Json<CandleResponse>> {
    let exchange = params.exchange.as_deref().unwrap_or("coinbase");
    let granularity = params.granularity.unwrap_or(60); // Default 1 minute
    
    // Default time range: last 7 days
    let end = params.end.unwrap_or_else(|| Utc::now().timestamp());
    let start = params.start.unwrap_or_else(|| end - 7 * 24 * 3600);
    
    // Normalize symbol (BTC-USD -> BTC/USD for internal use)
    let normalized_symbol = symbol.replace("-", "/");
    
    // First try to get historical data from Parquet files
    // Load config to get data directory
    let config = alphapulse_common::Config::load()
        .unwrap_or_else(|_| alphapulse_common::Config::default());
    let parquet_reader = ParquetReader::new(config.data.base_dir);
    let mut candles = parquet_reader
        .read_historical_candles(&symbol, exchange, Some(start), Some(end))
        .await
        .unwrap_or_else(|e| {
            info!("Failed to read parquet files: {}", e);
            Vec::new()
        });
    
    info!("Read {} candles from parquet files", candles.len());
    
    // If we don't have enough historical data, also get recent trades from Redis
    if candles.is_empty() || candles.last().map_or(true, |c| c.time < end - 3600) {
        info!("Fetching recent trades from Redis to supplement historical data");
        
        let trades = state.redis
            .get_trades_in_range(
                &normalized_symbol,
                exchange,
                Some(start as f64),
                Some(end as f64),
                Some(10000), // Max trades to process
            )
            .await?;
        
        // Convert trades to candles and merge with historical data
        let redis_candles = trades_to_candles(trades, granularity);
        
        // Merge candles, using BTreeMap to deduplicate and sort
        let mut all_candles = BTreeMap::new();
        for candle in candles {
            all_candles.insert(candle.time, candle);
        }
        for candle in redis_candles {
            all_candles.insert(candle.time, candle);
        }
        
        candles = all_candles.into_values().collect();
        info!("Total candles after merging: {}", candles.len());
    }
    
    Ok(Json(CandleResponse {
        candles,
        symbol: symbol.clone(),
        exchange: exchange.to_string(),
        granularity,
    }))
}

// Convert trades to OHLC candles
fn trades_to_candles(trades: Vec<Trade>, granularity: u32) -> Vec<Candle> {
    if trades.is_empty() {
        return Vec::new();
    }
    
    // Group trades by time bucket
    let mut buckets: BTreeMap<i64, Vec<Trade>> = BTreeMap::new();
    
    for trade in trades {
        let bucket = (trade.timestamp as i64 / granularity as i64) * granularity as i64;
        buckets.entry(bucket).or_insert_with(Vec::new).push(trade);
    }
    
    // Convert each bucket to a candle
    let mut candles = Vec::new();
    for (time, bucket_trades) in buckets {
        if bucket_trades.is_empty() {
            continue;
        }
        
        let open = bucket_trades.first().unwrap().price;
        let close = bucket_trades.last().unwrap().price;
        let high = bucket_trades.iter().map(|t| t.price).fold(f64::NEG_INFINITY, f64::max);
        let low = bucket_trades.iter().map(|t| t.price).fold(f64::INFINITY, f64::min);
        let volume: f64 = bucket_trades.iter().map(|t| t.volume).sum();
        
        candles.push(Candle {
            time,
            open,
            high,
            low,
            close,
            volume,
        });
    }
    
    candles
}

// Batch endpoint for multiple requests (mimics Python backend)
#[derive(Debug, Deserialize)]
pub struct BatchCandleRequest {
    pub symbol: String,
    pub exchange: String,
    pub start: i64,
    pub end: i64,
    pub granularity: u32,
}

pub async fn get_candles_batch(
    State(state): State<AppState>,
    Json(requests): Json<Vec<BatchCandleRequest>>,
) -> Response {
    match get_candles_batch_impl(state, requests).await {
        Ok(response) => response.into_response(),
        Err(e) => {
            tracing::error!("Error getting candles batch: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e)).into_response()
        }
    }
}

async fn get_candles_batch_impl(
    state: AppState,
    requests: Vec<BatchCandleRequest>,
) -> Result<Json<Vec<CandleResponse>>> {
    let mut responses = Vec::new();
    
    for req in requests {
        let normalized_symbol = req.symbol.replace("-", "/");
        
        // Get trades from Redis
        let trades = state.redis
            .get_trades_in_range(
                &normalized_symbol,
                &req.exchange,
                Some(req.start as f64),
                Some(req.end as f64),
                Some(10000),
            )
            .await?;
        
        let candles = trades_to_candles(trades, req.granularity);
        
        responses.push(CandleResponse {
            candles,
            symbol: req.symbol,
            exchange: req.exchange,
            granularity: req.granularity,
        });
    }
    
    Ok(Json(responses))
}

// Save endpoint (for compatibility - we don't actually need to save as we store trades)
#[derive(Debug, Deserialize)]
pub struct SaveDataRequest {
    pub symbol: String,
    pub exchange: String,
    pub data: Vec<Candle>,
}

pub async fn save_market_data(
    Json(_request): Json<SaveDataRequest>,
) -> Response {
    // We don't need to save candles as we generate them from trades
    // This endpoint exists for compatibility with the frontend
    Json(serde_json::json!({
        "status": "success",
        "message": "Data acknowledged (generated from trades)"
    })).into_response()
}
//! Test fixtures for Kraken signals testing

use alphapulse_strategies::kraken_signals::StrategyConfig;
use rust_decimal::Decimal;

/// Sample Kraken WebSocket trade message for testing
pub const KRAKEN_TRADE_MESSAGE: &str = r#"
{
    "channel": "trade",
    "type": "update",
    "data": [{
        "symbol": "BTC/USD",
        "side": "buy",
        "qty": 0.25,
        "price": 45230.50,
        "ts": 1700000000000
    }]
}
"#;

/// Sample Kraken WebSocket ticker message for testing
pub const KRAKEN_TICKER_MESSAGE: &str = r#"
{
    "channel": "ticker",
    "type": "update",
    "data": [{
        "symbol": "BTC/USD",
        "bid": 45225.00,
        "ask": 45235.00,
        "last": 45230.50,
        "volume": 125.50,
        "vwap": 45180.25
    }]
}
"#;

/// Create a test configuration for Kraken signals
pub fn test_config() -> StrategyConfig {
    StrategyConfig {
        target_instruments: vec!["BTC-USD".to_string(), "ETH-USD".to_string()],
        rsi_period: 14,
        macd_fast_period: 12,
        macd_slow_period: 26,
        macd_signal_period: 9,
        momentum_threshold_bps: 200,
        min_signal_confidence: 70,
        max_position_size_pct: Decimal::from(10),
        stop_loss_pct: Decimal::from(5),
    }
}

/// Sample price data for indicator testing
pub fn sample_price_data() -> Vec<f64> {
    vec![
        44.34, 44.09, 44.15, 43.61, 44.33, 44.83, 45.85, 45.92,
        45.68, 46.19, 46.23, 46.08, 46.03, 46.83, 47.69, 46.49,
        46.26, 47.09, 46.55, 46.23, 46.08, 46.03, 46.83, 47.69,
    ]
}
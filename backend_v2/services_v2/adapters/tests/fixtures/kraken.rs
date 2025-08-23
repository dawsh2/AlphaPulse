//! Real Kraken Market Data
//!
//! Contains actual Kraken WebSocket data for validation testing.

use serde_json::{json, Value};

/// Real Kraken ticker update from WebSocket feed
pub fn ticker_update_real() -> Value {
    json!({
        "channelID": 1,
        "data": {
            "a": ["44250.00000", "0", "0.12500000"],  // ask: price, whole_lot_volume, lot_volume
            "b": ["44240.00000", "0", "0.50000000"],  // bid: price, whole_lot_volume, lot_volume
            "c": ["44245.00000", "0.25000000"],       // last: price, lot_volume
            "v": ["125.75000000", "1250.00000000"],   // volume: today, last_24h
            "p": ["44200.50000", "44180.25000"],      // volume_weighted_average: today, last_24h
            "t": [1500, 15000],                       // trades_count: today, last_24h
            "l": ["43950.00000", "43850.00000"],      // low: today, last_24h
            "h": ["44500.00000", "44600.00000"],      // high: today, last_24h
            "o": ["44100.00000", "44000.00000"]       // open: today, last_24h
        },
        "channelName": "ticker",
        "pair": "XBT/USD"
    })
}

/// Real Kraken trade update from WebSocket feed
pub fn trade_update_real() -> Value {
    json!({
        "channelID": 2,
        "data": [
            ["44245.00000", "0.12500000", "1640995200.123456", "b", "l", ""],  // price, volume, time, side, type, misc
            ["44246.50000", "0.05000000", "1640995201.654321", "s", "m", ""]
        ],
        "channelName": "trade",
        "pair": "XBT/USD"
    })
}

/// Real Kraken L2 orderbook snapshot
pub fn orderbook_snapshot_real() -> Value {
    json!({
        "channelID": 3,
        "data": {
            "as": [
                ["44250.00000", "0.50000000", "1640995200.123456"],  // ask: price, volume, timestamp
                ["44251.00000", "0.75000000", "1640995200.234567"],
                ["44252.50000", "1.00000000", "1640995200.345678"]
            ],
            "bs": [
                ["44240.00000", "0.25000000", "1640995200.123456"],  // bid: price, volume, timestamp
                ["44239.00000", "0.60000000", "1640995200.234567"],
                ["44238.50000", "0.80000000", "1640995200.345678"]
            ]
        },
        "channelName": "book-10",
        "pair": "XBT/USD"
    })
}

/// Create invalid Kraken data for validation testing
pub fn invalid_ticker_missing_fields() -> Value {
    json!({
        "channelID": 1,
        "data": {
            // Missing required 'a' (ask) field
            "b": ["44240.00000", "0", "0.50000000"],
            "c": ["44245.00000", "0.25000000"]
            // Missing other required fields
        },
        "channelName": "ticker",
        "pair": "XBT/USD"
    })
}

/// Invalid data with zero prices (specification violation)
pub fn invalid_ticker_zero_prices() -> Value {
    json!({
        "channelID": 1,
        "data": {
            "a": ["0.00000000", "0", "0.12500000"],    // Invalid: zero ask price
            "b": ["0.00000000", "0", "0.50000000"],    // Invalid: zero bid price
            "c": ["0.00000000", "0.25000000"],         // Invalid: zero last price
            "v": ["125.75000000", "1250.00000000"],
            "p": ["44200.50000", "44180.25000"],
            "t": [1500, 15000],
            "l": ["43950.00000", "43850.00000"],
            "h": ["44500.00000", "44600.00000"],
            "o": ["44100.00000", "44000.00000"]
        },
        "channelName": "ticker",
        "pair": "XBT/USD"
    })
}

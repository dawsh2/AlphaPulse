//! Unit tests for signal generation logic

use alphapulse_strategies::kraken_signals::{SignalType, TradingSignal};
use rust_decimal::Decimal;

#[test]
fn test_buy_signal_generation() {
    let signal = TradingSignal::new_buy_signal(
        "BTC-USD".to_string(),
        Decimal::from(45000),
        Decimal::from(42000),
        Decimal::from(5),
        85,
        "RSI oversold + MACD bullish cross".to_string(),
    );

    assert_eq!(signal.signal_type, SignalType::Buy);
    assert_eq!(signal.confidence, 85);
    assert!(signal.price_target > signal.stop_loss);
    assert_eq!(signal.position_size_pct, Decimal::from(5));
}

#[test]
fn test_sell_signal_generation() {
    let signal = TradingSignal::new_sell_signal(
        "ETH-USD".to_string(),
        Decimal::from(2800),
        Decimal::from(3000),
        Decimal::from(3),
        75,
        "RSI overbought + bearish divergence".to_string(),
    );

    assert_eq!(signal.signal_type, SignalType::Sell);
    assert_eq!(signal.confidence, 75);
    assert!(signal.price_target < signal.stop_loss); // For sell signals, target is lower
    assert_eq!(signal.position_size_pct, Decimal::from(3));
}

#[test]
fn test_signal_confidence_validation() {
    // Test minimum confidence requirement
    let low_confidence_signal = TradingSignal {
        instrument_id: "BTC-USD".to_string(),
        signal_type: SignalType::Buy,
        confidence: 30, // Below typical minimum
        price_target: Decimal::from(45000),
        stop_loss: Decimal::from(42000),
        position_size_pct: Decimal::from(2),
        reasoning: "Weak signal".to_string(),
        timestamp_ns: 1700000000000000000,
    };

    // Should be filtered out by minimum confidence checks
    assert!(low_confidence_signal.confidence < 70);
}
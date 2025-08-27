//! Unit tests for KrakenSignalStrategy core functionality

use alphapulse_strategies::kraken_signals::{KrakenSignalStrategy, StrategyConfig};
use alphapulse_strategies::kraken_signals::{SignalType, TradingSignal};
use rust_decimal::Decimal;

#[tokio::test]
async fn test_strategy_initialization() {
    let config = StrategyConfig {
        target_instruments: vec!["BTC-USD".to_string(), "ETH-USD".to_string()],
        rsi_period: 14,
        macd_fast_period: 12,
        macd_slow_period: 26,
        macd_signal_period: 9,
        momentum_threshold_bps: 200, // 2%
        min_signal_confidence: 70,
        max_position_size_pct: Decimal::from(10), // 10%
        stop_loss_pct: Decimal::from(5), // 5%
    };

    let strategy = KrakenSignalStrategy::new(config.clone());
    
    // Verify strategy is properly initialized
    assert_eq!(strategy.config().target_instruments.len(), 2);
    assert_eq!(strategy.config().rsi_period, 14);
    assert_eq!(strategy.config().min_signal_confidence, 70);
}

#[tokio::test]
async fn test_signal_validation() {
    let signal = TradingSignal {
        instrument_id: "BTC-USD".to_string(),
        signal_type: SignalType::Buy,
        confidence: 85,
        price_target: Decimal::from(45000),
        stop_loss: Decimal::from(42000),
        position_size_pct: Decimal::from(5),
        reasoning: "RSI oversold + volume spike".to_string(),
        timestamp_ns: 1700000000000000000,
    };

    // Test signal validation
    assert!(signal.confidence >= 70);
    assert!(signal.price_target > signal.stop_loss);
    assert!(signal.position_size_pct <= Decimal::from(10));
}
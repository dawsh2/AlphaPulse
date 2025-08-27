//! Trading signal definitions and generation

use alphapulse_types::InstrumentId;
use rust_decimal::Decimal;
use alphapulse_network::time::safe_system_timestamp_ns;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalType {
    Buy,
    Sell,
    Hold,
}

#[derive(Debug, Clone)]
pub struct TradingSignal {
    /// Unique signal identifier
    pub signal_id: u64,

    /// Instrument this signal applies to
    pub instrument_id: InstrumentId,

    /// Signal type (Buy/Sell/Hold)
    pub signal_type: SignalType,

    /// Signal confidence (0-100)
    pub confidence: u8,

    /// Current price when signal generated
    pub current_price: Decimal,

    /// Suggested position size
    pub position_size_usd: Decimal,

    /// Expected profit potential
    pub expected_profit_usd: Decimal,

    /// Signal generation timestamp
    pub timestamp_ns: u64,

    /// Strategy that generated this signal
    pub strategy_id: u16,

    /// Human-readable reason for signal
    pub reason: String,
}

/// Configuration for creating TradingSignal
#[derive(Debug, Clone)]
pub struct SignalConfig {
    pub signal_id: u64,
    pub instrument_id: InstrumentId,
    pub signal_type: SignalType,
    pub confidence: u8,
    pub current_price: Decimal,
    pub position_size_usd: Decimal,
    pub expected_profit_usd: Decimal,
    pub strategy_id: u16,
    pub reason: String,
}

impl TradingSignal {
    /// Create a new trading signal from config
    pub fn new(config: SignalConfig) -> Self {
        let timestamp_ns = safe_system_timestamp_ns();

        Self {
            signal_id: config.signal_id,
            instrument_id: config.instrument_id,
            signal_type: config.signal_type,
            confidence: config.confidence,
            current_price: config.current_price,
            position_size_usd: config.position_size_usd,
            expected_profit_usd: config.expected_profit_usd,
            timestamp_ns,
            strategy_id: config.strategy_id,
            reason: config.reason,
        }
    }

    /// Check if signal is still valid (not too old)
    pub fn is_valid(&self, max_age_secs: u64) -> bool {
        let current_time = safe_system_timestamp_ns();

        let age_ns = current_time.saturating_sub(self.timestamp_ns);
        let age_secs = age_ns / 1_000_000_000;

        age_secs <= max_age_secs
    }

    /// Get signal strength based on confidence
    pub fn strength(&self) -> SignalStrength {
        match self.confidence {
            90..=100 => SignalStrength::VeryStrong,
            80..=89 => SignalStrength::Strong,
            70..=79 => SignalStrength::Moderate,
            60..=69 => SignalStrength::Weak,
            _ => SignalStrength::VeryWeak,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalStrength {
    VeryWeak,
    Weak,
    Moderate,
    Strong,
    VeryStrong,
}

/// Signal generation statistics
#[derive(Debug, Default, Clone)]
pub struct SignalStats {
    pub total_signals: u64,
    pub buy_signals: u64,
    pub sell_signals: u64,
    pub hold_signals: u64,
    pub avg_confidence: f64,
    pub last_signal_timestamp: Option<u64>,
}

impl SignalStats {
    /// Update stats with a new signal
    pub fn record_signal(&mut self, signal: &TradingSignal) {
        self.total_signals += 1;

        match signal.signal_type {
            SignalType::Buy => self.buy_signals += 1,
            SignalType::Sell => self.sell_signals += 1,
            SignalType::Hold => self.hold_signals += 1,
        }

        // Update rolling average confidence
        let total_confidence =
            self.avg_confidence * (self.total_signals - 1) as f64 + signal.confidence as f64;
        self.avg_confidence = total_confidence / self.total_signals as f64;

        self.last_signal_timestamp = Some(signal.timestamp_ns);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alphapulse_types::VenueId;
    use rust_decimal_macros::dec;

    #[test]
    fn test_signal_creation() {
        let instrument = InstrumentId::stock(VenueId::Kraken, "BTCUSD").unwrap();
        let signal = TradingSignal::new(
            1,
            instrument,
            SignalType::Buy,
            85,
            dec!(50000),
            dec!(1000),
            dec!(50),
            1,
            "Momentum breakout".to_string(),
        );

        assert_eq!(signal.signal_type, SignalType::Buy);
        assert_eq!(signal.confidence, 85);
        assert_eq!(signal.strength(), SignalStrength::Strong);
    }

    #[test]
    fn test_signal_validity() {
        let instrument = InstrumentId::stock(VenueId::Kraken, "BTCUSD").unwrap();
        let signal = TradingSignal::new(
            1,
            instrument,
            SignalType::Buy,
            75,
            dec!(50000),
            dec!(1000),
            dec!(50),
            1,
            "Test".to_string(),
        );

        assert!(signal.is_valid(60)); // Should be valid within 60 seconds
    }

    #[test]
    fn test_signal_stats() {
        let mut stats = SignalStats::default();
        let instrument = InstrumentId::stock(VenueId::Kraken, "BTCUSD").unwrap();

        let signal1 = TradingSignal::new(
            1,
            instrument,
            SignalType::Buy,
            80,
            dec!(50000),
            dec!(1000),
            dec!(50),
            1,
            "Test".to_string(),
        );
        let signal2 = TradingSignal::new(
            2,
            instrument,
            SignalType::Sell,
            90,
            dec!(51000),
            dec!(1000),
            dec!(50),
            1,
            "Test".to_string(),
        );

        stats.record_signal(&signal1);
        stats.record_signal(&signal2);

        assert_eq!(stats.total_signals, 2);
        assert_eq!(stats.buy_signals, 1);
        assert_eq!(stats.sell_signals, 1);
        assert_eq!(stats.avg_confidence, 85.0);
    }
}

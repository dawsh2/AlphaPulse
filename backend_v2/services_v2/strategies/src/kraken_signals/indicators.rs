//! Technical indicators for signal generation

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::VecDeque;

/// Simple Moving Average calculator
#[derive(Debug, Clone)]
pub struct MovingAverage {
    period: usize,
    values: VecDeque<Decimal>,
    sum: Decimal,
}

impl MovingAverage {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            values: VecDeque::with_capacity(period),
            sum: dec!(0),
        }
    }

    /// Add a new value and return current MA
    pub fn update(&mut self, value: Decimal) -> Option<Decimal> {
        self.values.push_back(value);
        self.sum += value;

        // Remove old values if we exceed the period
        if self.values.len() > self.period {
            if let Some(old_value) = self.values.pop_front() {
                self.sum -= old_value;
            }
        }

        // Return MA only if we have enough values
        if self.values.len() == self.period {
            Some(self.sum / Decimal::from(self.period))
        } else {
            None
        }
    }

    /// Get current moving average without adding new value
    pub fn current(&self) -> Option<Decimal> {
        if self.values.len() == self.period {
            Some(self.sum / Decimal::from(self.period))
        } else {
            None
        }
    }

    /// Check if indicator is ready (has enough data points)
    pub fn is_ready(&self) -> bool {
        self.values.len() == self.period
    }

    /// Get the number of data points
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Check if the indicator has no data points
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// Momentum indicator (rate of change)
#[derive(Debug, Clone)]
pub struct Momentum {
    period: usize,
    values: VecDeque<Decimal>,
}

impl Momentum {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            values: VecDeque::with_capacity(period + 1),
        }
    }

    /// Add new value and return momentum (current / past * 100)
    pub fn update(&mut self, value: Decimal) -> Option<Decimal> {
        self.values.push_back(value);

        // Keep only the values we need
        if self.values.len() > self.period + 1 {
            self.values.pop_front();
        }

        // Calculate momentum if we have enough data
        if self.values.len() == self.period + 1 {
            let current = self.values.back().unwrap();
            let past = self.values.front().unwrap();

            if *past != dec!(0) {
                Some((*current / *past) * dec!(100))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Check if indicator is ready
    pub fn is_ready(&self) -> bool {
        self.values.len() == self.period + 1
    }
}

/// Price volatility calculator (standard deviation)
#[derive(Debug, Clone)]
pub struct Volatility {
    period: usize,
    values: VecDeque<Decimal>,
}

impl Volatility {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            values: VecDeque::with_capacity(period),
        }
    }

    /// Add new value and return volatility (standard deviation)
    pub fn update(&mut self, value: Decimal) -> Option<Decimal> {
        self.values.push_back(value);

        if self.values.len() > self.period {
            self.values.pop_front();
        }

        if self.values.len() == self.period {
            self.calculate_std_dev()
        } else {
            None
        }
    }

    fn calculate_std_dev(&self) -> Option<Decimal> {
        if self.values.is_empty() {
            return None;
        }

        // Calculate mean
        let sum: Decimal = self.values.iter().sum();
        let mean = sum / Decimal::from(self.values.len());

        // Calculate variance
        let variance_sum: Decimal = self
            .values
            .iter()
            .map(|&x| {
                let diff = x - mean;
                diff * diff
            })
            .sum();

        let variance = variance_sum / Decimal::from(self.values.len());

        // Simple square root approximation for Decimal
        // In production, you'd want a more precise implementation
        let std_dev_f64 = variance.to_f64()?.sqrt();
        Decimal::try_from(std_dev_f64).ok()
    }

    /// Check if indicator is ready
    pub fn is_ready(&self) -> bool {
        self.values.len() == self.period
    }
}

/// Composite indicator that combines multiple signals
#[derive(Debug, Clone)]
pub struct CompositeIndicator {
    pub short_ma: MovingAverage,
    pub long_ma: MovingAverage,
    pub momentum: Momentum,
    pub volatility: Volatility,
}

impl CompositeIndicator {
    pub fn new(short_period: usize, long_period: usize, momentum_period: usize) -> Self {
        Self {
            short_ma: MovingAverage::new(short_period),
            long_ma: MovingAverage::new(long_period),
            momentum: Momentum::new(momentum_period),
            volatility: Volatility::new(long_period),
        }
    }

    /// Update all indicators with new price
    pub fn update(&mut self, price: Decimal) -> IndicatorSignal {
        let short_ma = self.short_ma.update(price);
        let long_ma = self.long_ma.update(price);
        let momentum = self.momentum.update(price);
        let volatility = self.volatility.update(price);

        IndicatorSignal {
            short_ma,
            long_ma,
            momentum,
            volatility,
            current_price: price,
        }
    }

    /// Check if all indicators are ready
    pub fn is_ready(&self) -> bool {
        self.short_ma.is_ready()
            && self.long_ma.is_ready()
            && self.momentum.is_ready()
            && self.volatility.is_ready()
    }
}

/// Combined indicator signal
#[derive(Debug, Clone)]
pub struct IndicatorSignal {
    pub short_ma: Option<Decimal>,
    pub long_ma: Option<Decimal>,
    pub momentum: Option<Decimal>,
    pub volatility: Option<Decimal>,
    pub current_price: Decimal,
}

impl IndicatorSignal {
    /// Determine trend direction from moving averages
    pub fn trend_direction(&self) -> Option<TrendDirection> {
        match (self.short_ma, self.long_ma) {
            (Some(short), Some(long)) => {
                if short > long {
                    Some(TrendDirection::Up)
                } else if short < long {
                    Some(TrendDirection::Down)
                } else {
                    Some(TrendDirection::Sideways)
                }
            }
            _ => None,
        }
    }

    /// Get momentum strength
    pub fn momentum_strength(&self) -> Option<MomentumStrength> {
        self.momentum.map(|m| {
            if m > dec!(105) {
                MomentumStrength::Strong
            } else if m > dec!(102) {
                MomentumStrength::Moderate
            } else if m < dec!(95) {
                MomentumStrength::Weak
            } else if m < dec!(98) {
                MomentumStrength::Moderate
            } else {
                MomentumStrength::Neutral
            }
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrendDirection {
    Up,
    Down,
    Sideways,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MomentumStrength {
    Strong,
    Moderate,
    Neutral,
    Weak,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_moving_average() {
        let mut ma = MovingAverage::new(3);

        assert_eq!(ma.update(dec!(10)), None); // Not enough data
        assert_eq!(ma.update(dec!(20)), None); // Still not enough
        assert_eq!(ma.update(dec!(30)), Some(dec!(20))); // (10+20+30)/3 = 20
        assert_eq!(ma.update(dec!(40)), Some(dec!(30))); // (20+30+40)/3 = 30
    }

    #[test]
    fn test_momentum() {
        let mut momentum = Momentum::new(2);

        assert_eq!(momentum.update(dec!(100)), None);
        assert_eq!(momentum.update(dec!(110)), None);
        assert_eq!(momentum.update(dec!(120)), Some(dec!(120))); // 120/100*100 = 120
    }

    #[test]
    fn test_composite_indicator() {
        let mut indicator = CompositeIndicator::new(2, 4, 3);

        // Feed some test data
        for price in [100, 105, 110, 115, 120] {
            let signal = indicator.update(Decimal::from(price));
            if indicator.is_ready() {
                assert!(signal.trend_direction().is_some());
            }
        }
    }
}

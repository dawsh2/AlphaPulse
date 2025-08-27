//! Unit tests for technical indicators used in Kraken signals

use alphapulse_strategies::kraken_signals::indicators::*;
use rust_decimal::Decimal;
use approx::assert_relative_eq;

#[test]
fn test_rsi_calculation() {
    let mut rsi = RSI::new(14);
    
    // Sample price data for RSI calculation
    let prices = vec![
        44.34, 44.09, 44.15, 43.61, 44.33, 44.83, 45.85, 45.92,
        45.68, 46.19, 46.23, 46.08, 46.03, 46.83, 47.69, 46.49,
    ];
    
    for price in prices {
        rsi.update(Decimal::from_f64_retain(price).unwrap());
    }
    
    // RSI should be calculated and within valid range
    let rsi_value = rsi.current_value();
    assert!(rsi_value >= Decimal::ZERO);
    assert!(rsi_value <= Decimal::from(100));
    
    // Test oversold/overbought conditions
    assert!(!rsi.is_oversold()); // Should not be oversold with this data
    assert!(!rsi.is_overbought()); // Should not be overbought with this data
}

#[test]
fn test_macd_calculation() {
    let mut macd = MACD::new(12, 26, 9);
    
    // Feed price data
    let prices = vec![
        22.27, 22.19, 22.08, 22.17, 22.18, 22.13, 22.23, 22.43,
        22.24, 22.29, 22.15, 22.39, 22.38, 22.61, 23.36, 24.05,
    ];
    
    for price in prices {
        macd.update(Decimal::from_f64_retain(price).unwrap());
    }
    
    let (macd_line, signal_line, histogram) = macd.current_values();
    
    // Verify MACD components are calculated
    assert!(macd_line.is_some());
    assert!(signal_line.is_some());
    assert!(histogram.is_some());
    
    // Test bullish/bearish cross detection
    // (Would need more sophisticated data to trigger actual crosses)
}

#[test]
fn test_moving_average() {
    let mut sma = MovingAverage::new(5);
    let prices = vec![10.0, 12.0, 13.0, 12.0, 15.0];
    
    for price in prices {
        sma.update(Decimal::from_f64_retain(price).unwrap());
    }
    
    // Average should be (10+12+13+12+15)/5 = 12.4
    let expected = Decimal::from_f64_retain(12.4).unwrap();
    assert_relative_eq!(sma.current_value().to_f64().unwrap(), expected.to_f64().unwrap(), epsilon = 0.01);
}
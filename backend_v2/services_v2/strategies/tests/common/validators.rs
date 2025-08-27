//! Common validation functions for strategy tests

use alphapulse_types::InstrumentId;
use rust_decimal::Decimal;

/// Validate that a price is reasonable for testing
pub fn validate_test_price(price: f64, min: f64, max: f64) -> Result<(), String> {
    if price <= 0.0 {
        return Err("Price must be positive".to_string());
    }
    if price < min || price > max {
        return Err(format!("Price {} out of range [{}, {}]", price, min, max));
    }
    Ok(())
}

/// Validate decimal precision for financial calculations
pub fn validate_decimal_precision(value: Decimal, max_scale: u32) -> Result<(), String> {
    if value.scale() > max_scale {
        return Err(format!(
            "Decimal scale {} exceeds maximum {}",
            value.scale(),
            max_scale
        ));
    }
    Ok(())
}

/// Validate timestamp is reasonable (not too old, not in future)
pub fn validate_timestamp_ns(timestamp_ns: u64) -> Result<(), String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_nanos() as u64;

    // Allow 1 hour in the past, 1 minute in the future
    let one_hour_ago = now - (3600 * 1_000_000_000);
    let one_minute_future = now + (60 * 1_000_000_000);

    if timestamp_ns < one_hour_ago {
        return Err("Timestamp is too old".to_string());
    }
    if timestamp_ns > one_minute_future {
        return Err("Timestamp is in the future".to_string());
    }
    Ok(())
}

/// Validate arbitrage opportunity parameters
pub fn validate_arbitrage_opportunity(
    profit_wei: u128,
    gas_cost_wei: u128,
    min_profit_wei: u128,
) -> Result<(), String> {
    if profit_wei == 0 {
        return Err("Profit cannot be zero".to_string());
    }
    if gas_cost_wei >= profit_wei {
        return Err("Gas cost exceeds profit".to_string());
    }
    let net_profit = profit_wei - gas_cost_wei;
    if net_profit < min_profit_wei {
        return Err(format!(
            "Net profit {} below minimum {}",
            net_profit, min_profit_wei
        ));
    }
    Ok(())
}

/// Validate trading signal parameters
pub fn validate_trading_signal(
    confidence: u8,
    position_size_pct: Decimal,
    stop_loss: Decimal,
    price_target: Decimal,
) -> Result<(), String> {
    if confidence == 0 || confidence > 100 {
        return Err(format!("Confidence {} must be between 1-100", confidence));
    }
    
    if position_size_pct <= Decimal::ZERO || position_size_pct > Decimal::from(100) {
        return Err("Position size must be between 0-100%".to_string());
    }
    
    if stop_loss <= Decimal::ZERO || price_target <= Decimal::ZERO {
        return Err("Stop loss and price target must be positive".to_string());
    }
    
    Ok(())
}

/// Validate pool reserves for realistic testing
pub fn validate_pool_reserves(reserve_0: u128, reserve_1: u128) -> Result<(), String> {
    if reserve_0 == 0 || reserve_1 == 0 {
        return Err("Pool reserves cannot be zero".to_string());
    }
    
    // Check for reasonable reserve ratios (avoid extreme imbalances)
    let ratio = if reserve_0 > reserve_1 {
        reserve_0 / reserve_1
    } else {
        reserve_1 / reserve_0
    };
    
    if ratio > 1000 {
        return Err("Pool reserves are extremely imbalanced".to_string());
    }
    
    Ok(())
}

/// Validate gas price is reasonable
pub fn validate_gas_price_gwei(gas_price_gwei: f64) -> Result<(), String> {
    if gas_price_gwei <= 0.0 {
        return Err("Gas price must be positive".to_string());
    }
    if gas_price_gwei > 1000.0 {
        return Err(format!("Gas price {} gwei is unreasonably high", gas_price_gwei));
    }
    if gas_price_gwei < 1.0 {
        return Err(format!("Gas price {} gwei is unreasonably low", gas_price_gwei));
    }
    Ok(())
}

/// Validate instrument ID format
pub fn validate_instrument_id(instrument_id: &str) -> Result<(), String> {
    if instrument_id.is_empty() {
        return Err("Instrument ID cannot be empty".to_string());
    }
    if instrument_id.len() > 50 {
        return Err("Instrument ID too long".to_string());
    }
    if !instrument_id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == ':') {
        return Err("Instrument ID contains invalid characters".to_string());
    }
    Ok(())
}
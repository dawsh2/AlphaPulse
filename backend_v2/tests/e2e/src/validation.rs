//! Data validation utilities for E2E tests

use anyhow::{anyhow, Result};
use serde_json::Value;
use std::collections::HashMap;

pub struct DataFlowValidator {
    expected_fields: HashMap<String, Vec<&'static str>>,
}

impl DataFlowValidator {
    pub fn new() -> Self {
        let mut expected_fields = HashMap::new();
        
        // Trade message fields
        expected_fields.insert("trade".to_string(), vec![
            "type", "instrument", "price", "volume", "side", "timestamp"
        ]);
        
        // Signal message fields
        expected_fields.insert("trading_signal".to_string(), vec![
            "type", "signal_id", "strategy_id", "confidence", "expected_profit_usd", "timestamp"
        ]);
        
        // Heartbeat message fields
        expected_fields.insert("heartbeat".to_string(), vec![
            "type", "timestamp"
        ]);
        
        Self { expected_fields }
    }
    
    pub fn validate_message(&self, message: &Value) -> Result<()> {
        let msg_type = message.get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Message missing 'type' field"))?;
        
        if let Some(expected) = self.expected_fields.get(msg_type) {
            for &field in expected {
                if !message.get(field).is_some() {
                    return Err(anyhow!("Message type '{}' missing required field '{}'", msg_type, field));
                }
            }
        }
        
        // Validate specific message types
        match msg_type {
            "trade" => self.validate_trade(message),
            "trading_signal" => self.validate_signal(message),
            "heartbeat" => self.validate_heartbeat(message),
            _ => Ok(()), // Unknown message types are allowed
        }
    }
    
    fn validate_trade(&self, message: &Value) -> Result<()> {
        // Validate price is positive number
        let price = message.get("price")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| anyhow!("Trade price must be a number"))?;
        
        if price <= 0.0 {
            return Err(anyhow!("Trade price must be positive, got {}", price));
        }
        
        // Validate volume is positive
        let volume = message.get("volume")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| anyhow!("Trade volume must be a number"))?;
        
        if volume <= 0.0 {
            return Err(anyhow!("Trade volume must be positive, got {}", volume));
        }
        
        // Validate side
        let side = message.get("side")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Trade side must be a string"))?;
        
        if !["buy", "sell", "unknown"].contains(&side) {
            return Err(anyhow!("Invalid trade side: {}", side));
        }
        
        // Validate timestamp
        let timestamp = message.get("timestamp")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow!("Trade timestamp must be a number"))?;
        
        // Check timestamp is reasonable (within last hour to future 1 minute)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        
        let one_hour_ago = now - (3600 * 1_000_000_000);
        let one_minute_future = now + (60 * 1_000_000_000);
        
        if timestamp < one_hour_ago || timestamp > one_minute_future {
            return Err(anyhow!("Trade timestamp {} outside reasonable range", timestamp));
        }
        
        Ok(())
    }
    
    fn validate_signal(&self, message: &Value) -> Result<()> {
        // Validate signal ID
        let _signal_id = message.get("signal_id")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow!("Signal ID must be a number"))?;
        
        // Validate strategy ID
        let _strategy_id = message.get("strategy_id")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow!("Strategy ID must be a number"))?;
        
        // Validate confidence (0-100)
        let confidence = message.get("confidence")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow!("Confidence must be a number"))?;
        
        if confidence > 100 {
            return Err(anyhow!("Confidence must be <= 100, got {}", confidence));
        }
        
        // Validate profit expectation
        let _profit = message.get("expected_profit_usd")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| anyhow!("Expected profit must be a number"))?;
        
        Ok(())
    }
    
    fn validate_heartbeat(&self, message: &Value) -> Result<()> {
        // Just validate timestamp exists and is reasonable
        let timestamp = message.get("timestamp")
            .ok_or_else(|| anyhow!("Heartbeat missing timestamp"))?;
        
        // Accept both number and string timestamps
        match timestamp {
            Value::Number(_) => Ok(()),
            Value::String(_) => Ok(()),
            _ => Err(anyhow!("Heartbeat timestamp must be number or string")),
        }
    }
}

/// Validates message latency end-to-end
pub struct LatencyValidator {
    max_acceptable_latency_ns: u64,
}

impl LatencyValidator {
    pub fn new(max_latency_ms: u64) -> Self {
        Self {
            max_acceptable_latency_ns: max_latency_ms * 1_000_000,
        }
    }
    
    pub fn validate_latency(&self, message_timestamp_ns: u64, received_timestamp_ns: u64) -> Result<()> {
        if received_timestamp_ns < message_timestamp_ns {
            return Err(anyhow!("Received timestamp is before message timestamp"));
        }
        
        let latency_ns = received_timestamp_ns - message_timestamp_ns;
        
        if latency_ns > self.max_acceptable_latency_ns {
            return Err(anyhow!(
                "Latency {}ms exceeds maximum {}ms",
                latency_ns / 1_000_000,
                self.max_acceptable_latency_ns / 1_000_000
            ));
        }
        
        Ok(())
    }
}

/// Validates precision preservation through the pipeline
pub struct PrecisionValidator;

impl PrecisionValidator {
    pub fn validate_price_precision(&self, original: f64, processed: f64) -> Result<()> {
        let precision_loss = (original - processed).abs() / original;
        
        // Allow up to 0.01% precision loss (1 basis point)
        if precision_loss > 0.0001 {
            return Err(anyhow!(
                "Precision loss {:.6}% exceeds threshold for price {} -> {}",
                precision_loss * 100.0,
                original,
                processed
            ));
        }
        
        Ok(())
    }
    
    pub fn validate_volume_precision(&self, original: f64, processed: f64) -> Result<()> {
        let precision_loss = (original - processed).abs() / original;
        
        // Allow up to 0.001% precision loss for volume
        if precision_loss > 0.00001 {
            return Err(anyhow!(
                "Volume precision loss {:.6}% exceeds threshold for {} -> {}",
                precision_loss * 100.0,
                original,
                processed
            ));
        }
        
        Ok(())
    }
}

/// Validate arbitrage opportunity message received by dashboard
pub fn assert_dashboard_received_arbitrage(
    message: &Value,
    expected_opportunity: &crate::fixtures::ArbitrageSignalFixture,
) -> Result<()> {
    // Check message type
    let msg_type = message.get("msg_type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Message missing 'msg_type' field"))?;
    
    if msg_type != "arbitrage_opportunity" {
        return Err(anyhow!("Expected 'arbitrage_opportunity', got '{}'", msg_type));
    }
    
    // Check strategy ID is flash arbitrage (21)
    let strategy_id = message.get("strategy_id")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow!("Missing strategy_id"))?;
    
    if strategy_id != 21 {
        return Err(anyhow!("Expected strategy_id 21 (Flash Arbitrage), got {}", strategy_id));
    }
    
    // Check profit amount
    let estimated_profit = message.get("estimated_profit")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| anyhow!("Missing estimated_profit"))?;
    
    let profit_diff = (estimated_profit - expected_opportunity.expected_profit_usd).abs();
    if profit_diff > 0.01 {
        return Err(anyhow!(
            "Profit mismatch: expected {:.2}, got {:.2}",
            expected_opportunity.expected_profit_usd,
            estimated_profit
        ));
    }
    
    // Check required fields exist
    let required_fields = [
        "signal_id", "confidence_score", "max_trade_size", 
        "profit_percent", "executable", "pair", "token_a", "token_b",
        "dex_buy", "dex_sell", "detected_at"
    ];
    
    for field in &required_fields {
        if !message.get(field).is_some() {
            return Err(anyhow!("Missing required field: {}", field));
        }
    }
    
    // Check executable is true
    let executable = message.get("executable")
        .and_then(|v| v.as_bool())
        .ok_or_else(|| anyhow!("Missing or invalid executable field"))?;
    
    if !executable {
        return Err(anyhow!("Expected executable to be true"));
    }
    
    Ok(())
}
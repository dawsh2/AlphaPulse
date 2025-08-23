/// PHASE 3: Reverse transformation engine for deep equality validation
/// 
/// This module provides functionality to trace a frontend message back to its original 
/// Polygon API response, enabling complete end-to-end validation of data integrity.

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use alphapulse_protocol::MessageTraceMessage;

/// Frontend trade message structure (what the user sees)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendTradeMessage {
    pub symbol_hash: String,
    pub symbol: Option<String>,
    pub timestamp: u64,
    pub price: f64,
    pub volume: f64,
    pub side: String,
    pub message_id: Option<String>,  // UUID for tracing back
}

/// Original Polygon API response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolygonSwapEvent {
    pub transaction_hash: String,
    pub block_number: u64,
    pub log_index: u64,
    pub address: String,  // Pool contract address
    pub topics: Vec<String>,
    pub data: String,
    pub timestamp: u64,
    // Raw response fields
    pub amount0_in: String,
    pub amount1_in: String, 
    pub amount0_out: String,
    pub amount1_out: String,
    pub to: String,
    pub sender: String,
}

/// Reverse transformation engine
pub struct ReverseTransformEngine {
    /// Cache of original API responses by message ID
    original_data_cache: HashMap<String, Value>,
    /// Cache of transformation steps by message ID
    transformation_log: HashMap<String, Vec<TransformationStep>>,
}

/// Individual transformation step in the pipeline
#[derive(Debug, Clone, Serialize)]
pub struct TransformationStep {
    pub stage: String,
    pub timestamp_ns: u64,
    pub input_hash: String,
    pub output_hash: String,
    pub transformation_type: String,
    pub details: Value,
}

/// Deep equality validation result
#[derive(Debug, Clone, Serialize)]
pub struct ValidationResult {
    pub message_id: String,
    pub success: bool,
    pub error: Option<String>,
    pub stages_validated: usize,
    pub data_integrity_preserved: bool,
    pub transformation_chain: Vec<TransformationStep>,
    pub original_data: Option<Value>,
    pub final_data: Option<Value>,
    pub precision_loss_detected: bool,
    pub anomalies: Vec<String>,
}

impl ReverseTransformEngine {
    pub fn new() -> Self {
        Self {
            original_data_cache: HashMap::new(),
            transformation_log: HashMap::new(),
        }
    }

    /// Store original API response data
    pub fn store_original_data(&mut self, message_id: String, original_data: Value) {
        self.original_data_cache.insert(message_id, original_data);
    }

    /// Log a transformation step
    pub fn log_transformation(&mut self, message_id: String, step: TransformationStep) {
        self.transformation_log
            .entry(message_id)
            .or_insert_with(Vec::new)
            .push(step);
    }

    /// Validate a frontend message against its original data
    pub fn validate_message(&self, frontend_msg: &FrontendTradeMessage) -> Result<ValidationResult> {
        let message_id = frontend_msg.message_id
            .as_ref()
            .ok_or_else(|| anyhow!("Frontend message missing message_id"))?;

        // Get original data
        let original_data = self.original_data_cache
            .get(message_id)
            .ok_or_else(|| anyhow!("Original data not found for message_id: {}", message_id))?;

        // Get transformation chain
        let transformation_chain = self.transformation_log
            .get(message_id)
            .cloned()
            .unwrap_or_default();

        // Perform reverse transformation validation
        let validation_result = self.perform_reverse_validation(
            frontend_msg,
            original_data,
            &transformation_chain,
        )?;

        Ok(validation_result)
    }

    /// Perform the actual reverse validation
    fn perform_reverse_validation(
        &self,
        frontend_msg: &FrontendTradeMessage,
        original_data: &Value,
        transformation_chain: &[TransformationStep],
    ) -> Result<ValidationResult> {
        let mut anomalies = Vec::new();
        let mut precision_loss_detected = false;
        let message_id = frontend_msg.message_id.as_ref().unwrap().clone();

        // Step 1: Parse original Polygon data
        let original_swap = self.parse_polygon_swap_event(original_data)?;

        // Step 2: Recreate transformation pipeline
        let reconstructed_trade = self.recreate_transformation_pipeline(&original_swap)?;

        // Step 3: Compare frontend message with reconstructed data
        let data_integrity_preserved = self.compare_trade_messages(
            frontend_msg,
            &reconstructed_trade,
            &mut anomalies,
            &mut precision_loss_detected,
        );

        // Step 4: Validate transformation chain integrity
        let chain_integrity = self.validate_transformation_chain(transformation_chain);
        if !chain_integrity {
            anomalies.push("Transformation chain integrity compromised".to_string());
        }

        Ok(ValidationResult {
            message_id,
            success: data_integrity_preserved && chain_integrity,
            error: if anomalies.is_empty() { None } else { Some(anomalies.join("; ")) },
            stages_validated: transformation_chain.len(),
            data_integrity_preserved,
            transformation_chain: transformation_chain.to_vec(),
            original_data: Some(original_data.clone()),
            final_data: Some(serde_json::to_value(frontend_msg)?),
            precision_loss_detected,
            anomalies,
        })
    }

    /// Parse Polygon swap event from original API response
    fn parse_polygon_swap_event(&self, data: &Value) -> Result<PolygonSwapEvent> {
        // This should match the exact parsing logic used in the Polygon collector
        let transaction_hash = data["transactionHash"].as_str()
            .ok_or_else(|| anyhow!("Missing transactionHash"))?;
        let block_number = data["blockNumber"].as_str()
            .ok_or_else(|| anyhow!("Missing blockNumber"))?
            .trim_start_matches("0x");
        let block_number = u64::from_str_radix(block_number, 16)?;

        let log_index = data["logIndex"].as_str()
            .ok_or_else(|| anyhow!("Missing logIndex"))?
            .trim_start_matches("0x");
        let log_index = u64::from_str_radix(log_index, 16)?;

        let address = data["address"].as_str()
            .ok_or_else(|| anyhow!("Missing address"))?;

        let topics: Vec<String> = data["topics"].as_array()
            .ok_or_else(|| anyhow!("Missing topics"))?
            .iter()
            .map(|v| v.as_str().unwrap_or("").to_string())
            .collect();

        let data_field = data["data"].as_str()
            .ok_or_else(|| anyhow!("Missing data"))?;

        // Parse the swap event data (this should match the DEX parsing logic)
        let (amount0_in, amount1_in, amount0_out, amount1_out, to, sender) = 
            self.parse_swap_event_data(data_field)?;

        Ok(PolygonSwapEvent {
            transaction_hash: transaction_hash.to_string(),
            block_number,
            log_index,
            address: address.to_string(),
            topics,
            data: data_field.to_string(),
            timestamp: 0, // Will be filled from block timestamp
            amount0_in,
            amount1_in,
            amount0_out,
            amount1_out,
            to,
            sender,
        })
    }

    /// Parse swap event data field (mimics DEX parsing logic)
    fn parse_swap_event_data(&self, data: &str) -> Result<(String, String, String, String, String, String)> {
        // This should exactly match the parsing logic in the DEX modules
        let data_without_prefix = data.trim_start_matches("0x");
        
        if data_without_prefix.len() < 384 { // 6 * 64 hex chars
            return Err(anyhow!("Invalid swap event data length"));
        }

        // Parse each 32-byte chunk (64 hex chars)
        let amount0_in = &data_without_prefix[0..64];
        let amount1_in = &data_without_prefix[64..128];
        let amount0_out = &data_without_prefix[128..192];
        let amount1_out = &data_without_prefix[192..256];
        let to = &data_without_prefix[256..320];
        let sender = &data_without_prefix[320..384];

        Ok((
            format!("0x{}", amount0_in),
            format!("0x{}", amount1_in),
            format!("0x{}", amount0_out),
            format!("0x{}", amount1_out),
            format!("0x{}", to),
            format!("0x{}", sender),
        ))
    }

    /// Recreate the transformation pipeline to get expected output
    fn recreate_transformation_pipeline(&self, swap_event: &PolygonSwapEvent) -> Result<FrontendTradeMessage> {
        // This should recreate the exact transformation steps:
        // 1. Polygon API response -> Parsed swap event
        // 2. Parsed swap event -> TradeMessage (protocol)
        // 3. TradeMessage -> WebSocket broadcast format
        // 4. WebSocket format -> Frontend trade message

        // Convert amounts from hex to decimal (matching DEX logic)
        let amount0_in = u128::from_str_radix(
            swap_event.amount0_in.trim_start_matches("0x"), 16
        )? as f64;
        let amount1_in = u128::from_str_radix(
            swap_event.amount1_in.trim_start_matches("0x"), 16
        )? as f64;
        let amount0_out = u128::from_str_radix(
            swap_event.amount0_out.trim_start_matches("0x"), 16
        )? as f64;
        let amount1_out = u128::from_str_radix(
            swap_event.amount1_out.trim_start_matches("0x"), 16
        )? as f64;

        // Calculate price and volume (matching DEX price calculation logic)
        let (price, volume) = if amount0_in > 0.0 && amount1_out > 0.0 {
            // Swapping token0 -> token1
            let price = amount1_out / amount0_in;
            let volume = amount0_in * price;
            (price, volume)
        } else if amount1_in > 0.0 && amount0_out > 0.0 {
            // Swapping token1 -> token0  
            let price = amount1_in / amount0_out;
            let volume = amount0_out * price;
            (price, volume)
        } else {
            (0.0, 0.0)
        };

        // Determine trade side
        let side = if amount0_in > 0.0 { "sell" } else { "buy" }.to_string();

        // Generate symbol hash (matching SymbolDescriptor logic)
        let symbol_hash = format!("{}", swap_event.address.len()); // Simplified for now

        Ok(FrontendTradeMessage {
            symbol_hash,
            symbol: Some(format!("polygon:{}:TOKEN0/TOKEN1", swap_event.address)),
            timestamp: swap_event.timestamp,
            price,
            volume,
            side,
            message_id: None, // Will be filled by validation logic
        })
    }

    /// Compare frontend message with reconstructed data
    fn compare_trade_messages(
        &self,
        frontend: &FrontendTradeMessage,
        reconstructed: &FrontendTradeMessage,
        anomalies: &mut Vec<String>,
        precision_loss_detected: &mut bool,
    ) -> bool {
        let mut integrity_preserved = true;

        // Price comparison with tolerance for floating point precision
        let price_diff = (frontend.price - reconstructed.price).abs();
        let price_tolerance = reconstructed.price * 0.0001; // 0.01% tolerance
        if price_diff > price_tolerance {
            anomalies.push(format!(
                "Price deviation: frontend={}, reconstructed={}, diff={}",
                frontend.price, reconstructed.price, price_diff
            ));
            integrity_preserved = false;
            if price_diff > reconstructed.price * 0.01 {
                *precision_loss_detected = true;
            }
        }

        // Volume comparison
        let volume_diff = (frontend.volume - reconstructed.volume).abs();
        let volume_tolerance = reconstructed.volume * 0.0001;
        if volume_diff > volume_tolerance {
            anomalies.push(format!(
                "Volume deviation: frontend={}, reconstructed={}, diff={}",
                frontend.volume, reconstructed.volume, volume_diff
            ));
            integrity_preserved = false;
        }

        // Side comparison
        if frontend.side != reconstructed.side {
            anomalies.push(format!(
                "Side mismatch: frontend={}, reconstructed={}",
                frontend.side, reconstructed.side
            ));
            integrity_preserved = false;
        }

        integrity_preserved
    }

    /// Validate transformation chain integrity
    fn validate_transformation_chain(&self, chain: &[TransformationStep]) -> bool {
        if chain.is_empty() {
            return false;
        }

        // Check that output hash of each step matches input hash of next step
        for window in chain.windows(2) {
            if window[0].output_hash != window[1].input_hash {
                return false;
            }
        }

        // Check timestamps are increasing
        for window in chain.windows(2) {
            if window[0].timestamp_ns >= window[1].timestamp_ns {
                return false;
            }
        }

        true
    }

    /// Get statistics about validation cache
    pub fn get_cache_stats(&self) -> (usize, usize) {
        (self.original_data_cache.len(), self.transformation_log.len())
    }

    /// Clean up old cache entries to prevent memory leaks
    pub fn cleanup_cache(&mut self, max_entries: usize) {
        if self.original_data_cache.len() > max_entries {
            let keys_to_remove: Vec<_> = self.original_data_cache.keys()
                .take(self.original_data_cache.len() - max_entries)
                .cloned()
                .collect();
            for key in keys_to_remove {
                self.original_data_cache.remove(&key);
                self.transformation_log.remove(&key);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reverse_transformation_basic() {
        let mut engine = ReverseTransformEngine::new();
        
        // Store some test data
        let message_id = "test-uuid-123".to_string();
        let original_data = serde_json::json!({
            "transactionHash": "0xabc123",
            "blockNumber": "0x1234",
            "logIndex": "0x0",
            "address": "0xpool123",
            "topics": ["0xtopic1"],
            "data": "0x000000000000000000000000000000000000000000000000016345785d8a0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"
        });
        
        engine.store_original_data(message_id.clone(), original_data);
        
        // Test validation
        let frontend_msg = FrontendTradeMessage {
            symbol_hash: "test_hash".to_string(),
            symbol: Some("test:symbol".to_string()),
            timestamp: 1234567890,
            price: 1.5,
            volume: 1000.0,
            side: "buy".to_string(),
            message_id: Some(message_id),
        };
        
        let result = engine.validate_message(&frontend_msg);
        assert!(result.is_ok());
    }
}
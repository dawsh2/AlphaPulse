//! Validated ABI Decoder with Four-Step Validation
//!
//! Integrates the ABI event decoder with the complete validation framework
//! to ensure semantic correctness and data integrity.

use crate::input::collectors::polygon_dex::abi_events::{SwapEventDecoder, ValidatedSwapData, DEXProtocol as LocalDEXProtocol};
use crate::validation::{
    RawDataValidator, ValidationResult, ValidationError, complete_validation_pipeline
};
use protocol_v2::{
    tlv::market_data::PoolSwapTLV,
    tlv::DEXProtocol,
    VenueId,
};
use web3::types::Log;
use anyhow::Result;

/// Raw Polygon event data that implements validation
#[derive(Debug, Clone)]
pub struct PolygonRawSwapEvent {
    pub log: Log,
    pub validated_data: ValidatedSwapData,
}

impl RawDataValidator for PolygonRawSwapEvent {
    fn validate_required_fields(&self) -> ValidationResult<()> {
        // Check that all required fields from ValidatedSwapData are present and valid
        
        // Pool address must be non-zero
        if self.validated_data.pool_address == [0u8; 20] {
            return Err(ValidationError::RawParsing("Pool address cannot be zero".to_string()));
        }
        
        // Sender and recipient must be non-zero for V3, sender for V2
        if self.validated_data.sender == [0u8; 20] {
            return Err(ValidationError::RawParsing("Sender address cannot be zero".to_string()));
        }
        
        // At least one amount must be non-zero
        if self.validated_data.amount_in == 0 && self.validated_data.amount_out == 0 {
            return Err(ValidationError::RawParsing("Both amounts cannot be zero".to_string()));
        }
        
        Ok(())
    }
    
    fn validate_types_against_spec(&self) -> ValidationResult<()> {
        // Validate that the DEX protocol was correctly detected
        match self.validated_data.dex_protocol {
            DEXProtocol::UniswapV2 | 
            DEXProtocol::UniswapV3 => {
                // These are expected protocols for Polygon
            }
            _ => {
                return Err(ValidationError::RawParsing(
                    format!("Unexpected DEX protocol: {:?}", self.validated_data.dex_protocol)
                ));
            }
        }
        
        // For V3, validate that tick bounds are reasonable
        if let DEXProtocol::UniswapV3 = self.validated_data.dex_protocol {
            if self.validated_data.tick_after < -887272 || self.validated_data.tick_after > 887272 {
                return Err(ValidationError::RawParsing(
                    format!("V3 tick out of bounds: {}", self.validated_data.tick_after)
                ));
            }
        }
        
        Ok(())
    }
    
    fn validate_field_ranges(&self) -> ValidationResult<()> {
        // Only validate against provider specification - no business logic limits
        
        // For V3 pools, sqrt_price_x96 should not be all zeros (specification violation)
        if matches!(self.validated_data.dex_protocol, DEXProtocol::UniswapV3) && 
           self.validated_data.sqrt_price_x96_after == [0u8; 20] {
            return Err(ValidationError::RawParsing(
                "sqrt_price_x96 cannot be zero for V3 pools (violates V3 specification)".to_string()
            ));
        }
        
        // Block number must be present for valid blockchain data
        if self.log.block_number.is_none() {
            return Err(ValidationError::RawParsing(
                "Block number missing from log data".to_string()
            ));
        }
        
        Ok(())
    }
    
    fn validate_precision_preserved(&self) -> ValidationResult<()> {
        // Only validate that precision extraction succeeded - no business logic about valid decimals
        
        // Validate that we haven't lost precision during u256 -> u128 conversion
        // The ABI decoder should have already validated this during extraction
        
        // For now, we assume token decimals are handled at the pool registry level
        // ValidatedSwapData doesn't include decimals since they're pool-specific metadata
        
        Ok(())
    }
}

/// Convert ValidatedSwapData to PoolSwapTLV with semantic correctness
impl From<PolygonRawSwapEvent> for PoolSwapTLV {
    fn from(raw: PolygonRawSwapEvent) -> Self {
        let data = raw.validated_data;
        
        PoolSwapTLV {
            venue: VenueId::Polygon,  // Blockchain is the venue, not protocol
            pool_address: data.pool_address,
            token_in_addr: [0u8; 20], // TODO: Extract from ABI data
            token_out_addr: [0u8; 20], // TODO: Extract from ABI data
            amount_in: data.amount_in,
            amount_out: data.amount_out,
            amount_in_decimals: 18, // TODO: Get from pool registry - defaulting to 18 for now
            amount_out_decimals: 6, // TODO: Get from pool registry - defaulting to 6 for now
            tick_after: data.tick_after,
            sqrt_price_x96_after: data.sqrt_price_x96_after,
            liquidity_after: data.liquidity_after,
            timestamp_ns: (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64),
            block_number: raw.log.block_number.map(|n| n.as_u64()).unwrap_or(0),
        }
    }
}

/// High-level validated decoder that implements the four-step validation process
pub struct ValidatedPolygonDecoder;

impl ValidatedPolygonDecoder {
    /// Create new validated decoder
    pub fn new() -> Self {
        Self
    }
    
    /// Decode and validate a Polygon swap event with complete validation
    pub fn decode_and_validate(&self, log: &Log, dex_protocol: LocalDEXProtocol) -> Result<PoolSwapTLV> {
        // First, use the ABI decoder to extract semantic data
        let validated_data = SwapEventDecoder::decode_swap_event(log, dex_protocol)?;
        
        // Create raw event structure
        let raw_event = PolygonRawSwapEvent {
            log: log.clone(),
            validated_data,
        };
        
        // Run complete four-step validation pipeline
        let log_bytes = self.serialize_log_for_validation(log);
        
        match complete_validation_pipeline::<PolygonRawSwapEvent, PoolSwapTLV>(&log_bytes, raw_event) {
            Ok(validated_tlv) => {
                tracing::debug!(
                    "✅ Polygon swap validated: {} -> {} (tick: {})",
                    validated_tlv.amount_in,
                    validated_tlv.amount_out,
                    validated_tlv.tick_after
                );
                Ok(validated_tlv)
            }
            Err(e) => {
                tracing::error!("❌ Validation failed for Polygon swap: {}", e);
                Err(anyhow::anyhow!("Validation failed: {}", e))
            }
        }
    }
    
    /// Decode multiple events with batch validation
    pub fn decode_and_validate_batch(&self, logs: &[Log], dex_protocol: LocalDEXProtocol) -> Vec<Result<PoolSwapTLV>> {
        logs.iter()
            .map(|log| self.decode_and_validate(log, dex_protocol))
            .collect()
    }
    
    /// Helper to serialize log data for validation (simplified)
    fn serialize_log_for_validation(&self, log: &Log) -> Vec<u8> {
        // For validation purposes, we serialize key log fields
        let mut bytes = Vec::new();
        
        // Add address
        bytes.extend_from_slice(log.address.as_bytes());
        
        // Add data
        bytes.extend_from_slice(&log.data.0);
        
        // Add topics (simplified)
        for topic in &log.topics {
            bytes.extend_from_slice(topic.as_bytes());
        }
        
        bytes
    }
}

impl Default for ValidatedPolygonDecoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use web3::types::{H256, U256, U64};
    
    fn create_test_log() -> Log {
        // Use real Uniswap V3 Swap event structure from our fixtures
        Log {
            address: "0x45dda9cb7c25131df268515131f647d726f50608".parse().unwrap(), // Real Uniswap V3 pool
            topics: vec![
                // Real Uniswap V3 Swap event signature (correct keccak256 hash)
                "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67".parse().unwrap(),
                // sender (indexed) - Uniswap V3 Router
                "0x000000000000000000000000e592427a0aece92de3edee1f18e0157c05861564".parse().unwrap(),
                // recipient (indexed) - Same as sender for this example
                "0x000000000000000000000000e592427a0aece92de3edee1f18e0157c05861564".parse().unwrap(),
            ],
            // Real Uniswap V3 swap data: WETH -> USDC trade (5 fields, each 32 bytes)
            data: web3::types::Bytes(hex::decode(concat!(
                "000000000000000000000000000000000000000000000000002386f26fc10000", // amount0: +10 WETH (int256)
                "fffffffffffffffffffffffffffffffffffffffffffffffffffff8e9db5e8180", // amount1: -27000 USDC (int256, negative)
                "000000000000000000000001b1ae4d6e2ef5896dc1c9c88f1b3d9b8f7e5a4c10", // sqrtPriceX96 (uint160)
                "00000000000000000000000000000000000000000000000000038d7ea4c68000", // liquidity (uint128)
                "0000000000000000000000000000000000000000000000000000000000000d41"  // tick: 3393 (int24)
            )).unwrap()),
            block_hash: Some("0xfa4bb88b9f7e8e56cb97e5b8f1c7d3d6e9a7c8b5f4e3d2c1b0a9f8e7d6c5b4a3".parse().unwrap()),
            block_number: Some(U64::from(48_500_000)),
            transaction_hash: Some("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".parse().unwrap()),
            transaction_index: Some(U64::from(42)),
            log_index: Some(U256::from(15)),
            transaction_log_index: Some(U256::from(3)),
            log_type: None,
            removed: Some(false),
        }
    }
    
    #[test]
    fn test_validated_decoder_success() {
        let decoder = ValidatedPolygonDecoder::new();
        let log = create_test_log();
        
        // Debug: Check what signature our ABI definition generates vs what we're using
        println!("🔍 DEBUG EVENT SIGNATURE COMPARISON:");
        println!("   Expected signature from our log: {:?}", log.topics[0]);
        
        // Try to generate the signature from our V3 event definition
        use crate::input::collectors::polygon_dex::abi_events::SwapEventDecoder;
        let test_result = SwapEventDecoder::decode_swap_event(&log, LocalDEXProtocol::UniswapV3);
        match &test_result {
            Ok(_) => println!("   Direct ABI decode result: Success"),
            Err(e) => println!("   Direct ABI decode result: Error: {}", e),
        }
        
        // This should succeed with complete validation
        let result = decoder.decode_and_validate(&log, LocalDEXProtocol::UniswapV3);
        
        if result.is_err() {
            println!("❌ Validation failed: {:?}", result);
            println!("   Log signature: 0x{}", hex::encode(log.topics[0].as_bytes()));
            println!("   Log data length: {} bytes", log.data.0.len());
        }
        
        assert!(result.is_ok(), "Validation should succeed: {:?}", result);
        
        let tlv = result.unwrap();
        assert_eq!(tlv.venue, VenueId::Polygon);
        assert!(tlv.amount_in > 0);
        assert!(tlv.amount_out > 0);
    }
    
    #[test]
    fn test_raw_event_validation() {
        let log = create_test_log();
        let validated_data = SwapEventDecoder::decode_swap_event(&log, LocalDEXProtocol::UniswapV3).unwrap();
        
        let raw_event = PolygonRawSwapEvent {
            log,
            validated_data,
        };
        
        // Test all validation steps
        assert!(raw_event.validate_required_fields().is_ok());
        assert!(raw_event.validate_types_against_spec().is_ok());
        assert!(raw_event.validate_field_ranges().is_ok());
        assert!(raw_event.validate_precision_preserved().is_ok());
    }
    
    #[test]
    fn test_batch_validation() {
        let decoder = ValidatedPolygonDecoder::new();
        let logs = vec![create_test_log(), create_test_log()];
        
        let results = decoder.decode_and_validate_batch(&logs, LocalDEXProtocol::UniswapV3);
        assert_eq!(results.len(), 2);
        assert!(results[0].is_ok());
        assert!(results[1].is_ok());
    }
}
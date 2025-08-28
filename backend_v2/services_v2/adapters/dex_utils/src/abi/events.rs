//! DEX event structures and decoders
//!
//! Provides semantic validation and type-safe decoding of blockchain events
//! using ethabi, preventing manual byte parsing errors and data truncation.

use super::uniswap_v2;
use super::uniswap_v3;
use super::DEXProtocol;
use ethabi::RawLog;
use web3::types::{Log, H160, U256};

/// Error types for ABI decoding
#[derive(Debug, thiserror::Error)]
pub enum DecodingError {
    #[error("Unknown event signature: {0}")]
    UnknownEventSignature(String),

    #[error("ABI parsing failed: {0}")]
    AbiParsingError(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Value overflow: {value} exceeds i64::MAX")]
    ValueOverflow { value: String },

    #[error("Invalid token order in event")]
    InvalidTokenOrder,

    #[error("Unsupported DEX protocol: {0:?}")]
    UnsupportedProtocol(DEXProtocol),
}

/// Validated swap data with semantic correctness
#[derive(Debug, Clone)]
pub struct ValidatedSwap {
    pub pool_address: [u8; 20],
    pub amount_in: i64,
    pub amount_out: i64,
    pub token_in_is_token0: bool,
    pub sqrt_price_x96_after: u128,
    pub tick_after: i32,
    pub liquidity_after: u128,
    pub dex_protocol: DEXProtocol,
}

/// Validated mint data
#[derive(Debug, Clone)]
pub struct ValidatedMint {
    pub pool_address: [u8; 20],
    pub liquidity_provider: [u8; 20],
    pub liquidity_delta: u128,
    pub amount0: u128,
    pub amount1: u128,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub dex_protocol: DEXProtocol,
}

/// Validated burn data
#[derive(Debug, Clone)]
pub struct ValidatedBurn {
    pub pool_address: [u8; 20],
    pub liquidity_provider: [u8; 20],
    pub liquidity_delta: u128,
    pub amount0: u128,
    pub amount1: u128,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub dex_protocol: DEXProtocol,
}

/// Detect DEX protocol from pool address and log structure
pub fn detect_dex_protocol(pool_address: &H160, log: &Log) -> DEXProtocol {
    // Check factory addresses or specific patterns first
    let addr_bytes = pool_address.as_bytes();

    // Check address patterns first for more specific protocol detection
    if addr_bytes[0] == 0x5C {
        // QuickSwap V3 pools often start with 0x5C
        DEXProtocol::QuickswapV3
    } else if addr_bytes[0] == 0xc3 {
        // SushiSwap pools pattern
        DEXProtocol::SushiswapV2
    } else if log.topics.len() >= 3 && log.data.0.len() > 100 {
        // V3 swaps have more data (sqrtPriceX96, liquidity, tick)
        DEXProtocol::UniswapV3
    } else {
        // Default to V2
        DEXProtocol::UniswapV2
    }
}

/// ABI decoder for Swap events
pub struct SwapEventDecoder;

impl SwapEventDecoder {
    /// Decode swap event based on protocol type
    pub fn decode_swap_event(
        log: &Log,
        protocol: DEXProtocol,
    ) -> Result<ValidatedSwap, DecodingError> {
        let raw_log = RawLog {
            topics: log.topics.clone(),
            data: log.data.0.clone(),
        };

        match protocol {
            DEXProtocol::UniswapV3 | DEXProtocol::QuickswapV3 => {
                Self::decode_v3_swap(log.address, raw_log, protocol)
            }
            DEXProtocol::UniswapV2 | DEXProtocol::SushiswapV2 | DEXProtocol::QuickswapV2 => {
                Self::decode_v2_swap(log.address, raw_log, protocol)
            }
        }
    }

    /// Decode V3 swap event
    fn decode_v3_swap(
        pool_address: H160,
        raw_log: RawLog,
        protocol: DEXProtocol,
    ) -> Result<ValidatedSwap, DecodingError> {
        let event = uniswap_v3::swap_event();
        let decoded = event
            .parse_log(raw_log)
            .map_err(|e| DecodingError::AbiParsingError(e.to_string()))?;

        // Extract amounts (can be negative in V3)
        let amount0 = decoded
            .params
            .get(2)
            .and_then(|p| p.value.clone().into_int())
            .ok_or(DecodingError::MissingField("amount0".to_string()))?;

        let amount1 = decoded
            .params
            .get(3)
            .and_then(|p| p.value.clone().into_int())
            .ok_or(DecodingError::MissingField("amount1".to_string()))?;

        // Determine trade direction based on signs
        let (amount_in, amount_out, token_in_is_token0) = if amount0 > U256::zero() {
            // Token0 in (positive), Token1 out (negative)
            (amount0, amount1.overflowing_neg().0, true)
        } else {
            // Token1 in (positive), Token0 out (negative)
            (amount1, amount0.overflowing_neg().0, false)
        };

        // Check for overflow before converting to i64
        let amount_in_i64 = Self::safe_u256_to_i64(amount_in)?;
        let amount_out_i64 = Self::safe_u256_to_i64(amount_out)?;

        // Extract V3-specific fields
        let sqrt_price_x96 = decoded
            .params
            .get(4)
            .and_then(|p| p.value.clone().into_uint())
            .ok_or(DecodingError::MissingField("sqrtPriceX96".to_string()))?;

        let liquidity = decoded
            .params
            .get(5)
            .and_then(|p| p.value.clone().into_uint())
            .ok_or(DecodingError::MissingField("liquidity".to_string()))?;

        let tick = decoded
            .params
            .get(6)
            .and_then(|p| p.value.clone().into_int())
            .ok_or(DecodingError::MissingField("tick".to_string()))?;

        Ok(ValidatedSwap {
            pool_address: pool_address.0,
            amount_in: amount_in_i64,
            amount_out: amount_out_i64,
            token_in_is_token0,
            sqrt_price_x96_after: sqrt_price_x96.as_u128(),
            tick_after: tick.as_u32() as i32,
            liquidity_after: liquidity.as_u128(),
            dex_protocol: protocol,
        })
    }

    /// Decode V2 swap event
    fn decode_v2_swap(
        pool_address: H160,
        raw_log: RawLog,
        protocol: DEXProtocol,
    ) -> Result<ValidatedSwap, DecodingError> {
        let event = uniswap_v2::swap_event();
        let decoded = event
            .parse_log(raw_log)
            .map_err(|e| DecodingError::AbiParsingError(e.to_string()))?;

        // Extract all amounts
        let amount0_in = decoded
            .params
            .get(1)
            .and_then(|p| p.value.clone().into_uint())
            .ok_or(DecodingError::MissingField("amount0In".to_string()))?;

        let amount1_in = decoded
            .params
            .get(2)
            .and_then(|p| p.value.clone().into_uint())
            .ok_or(DecodingError::MissingField("amount1In".to_string()))?;

        let amount0_out = decoded
            .params
            .get(3)
            .and_then(|p| p.value.clone().into_uint())
            .ok_or(DecodingError::MissingField("amount0Out".to_string()))?;

        let amount1_out = decoded
            .params
            .get(4)
            .and_then(|p| p.value.clone().into_uint())
            .ok_or(DecodingError::MissingField("amount1Out".to_string()))?;

        // Determine trade direction
        let (amount_in, amount_out, token_in_is_token0) = if amount0_in > U256::zero() {
            (amount0_in, amount1_out, true)
        } else if amount1_in > U256::zero() {
            (amount1_in, amount0_out, false)
        } else {
            return Err(DecodingError::InvalidTokenOrder);
        };

        // Safe conversion with overflow check
        let amount_in_i64 = Self::safe_u256_to_i64(amount_in)?;
        let amount_out_i64 = Self::safe_u256_to_i64(amount_out)?;

        Ok(ValidatedSwap {
            pool_address: pool_address.0,
            amount_in: amount_in_i64,
            amount_out: amount_out_i64,
            token_in_is_token0,
            sqrt_price_x96_after: 0, // V2 doesn't have this
            tick_after: 0,           // V2 doesn't have ticks
            liquidity_after: 0,      // V2 doesn't expose this in swap
            dex_protocol: protocol,
        })
    }

    /// Safely convert U256 to i64 with overflow detection
    pub fn safe_u256_to_i64(value: U256) -> Result<i64, DecodingError> {
        if value > U256::from(i64::MAX) {
            // For very large values, truncate to i64::MAX with warning
            tracing::warn!("Value overflow: {} > i64::MAX, truncating", value);
            Ok(i64::MAX)
        } else {
            Ok(value.as_u64() as i64)
        }
    }
}

/// ABI decoder for Mint events
pub struct MintEventDecoder;

impl MintEventDecoder {
    /// Decode mint event based on protocol
    pub fn decode_mint_event(
        log: &Log,
        protocol: DEXProtocol,
    ) -> Result<ValidatedMint, DecodingError> {
        let raw_log = RawLog {
            topics: log.topics.clone(),
            data: log.data.0.clone(),
        };

        match protocol {
            DEXProtocol::UniswapV3 | DEXProtocol::QuickswapV3 => {
                Self::decode_v3_mint(log.address, raw_log, protocol)
            }
            DEXProtocol::UniswapV2 | DEXProtocol::SushiswapV2 | DEXProtocol::QuickswapV2 => {
                Self::decode_v2_mint(log.address, raw_log, protocol)
            }
        }
    }

    /// Decode V3 mint event
    fn decode_v3_mint(
        pool_address: H160,
        raw_log: RawLog,
        protocol: DEXProtocol,
    ) -> Result<ValidatedMint, DecodingError> {
        let event = uniswap_v3::mint_event();
        let decoded = event
            .parse_log(raw_log)
            .map_err(|e| DecodingError::AbiParsingError(e.to_string()))?;

        // Extract liquidity provider from owner (indexed)
        let owner = decoded
            .params
            .get(1)
            .and_then(|p| p.value.clone().into_address())
            .ok_or(DecodingError::MissingField("owner".to_string()))?;

        // Extract tick range (indexed)
        let tick_lower = decoded
            .params
            .get(2)
            .and_then(|p| p.value.clone().into_int())
            .ok_or(DecodingError::MissingField("tickLower".to_string()))?;

        let tick_upper = decoded
            .params
            .get(3)
            .and_then(|p| p.value.clone().into_int())
            .ok_or(DecodingError::MissingField("tickUpper".to_string()))?;

        // Extract liquidity and amounts
        let liquidity = decoded
            .params
            .get(4)
            .and_then(|p| p.value.clone().into_uint())
            .ok_or(DecodingError::MissingField("amount".to_string()))?;

        let amount0 = decoded
            .params
            .get(5)
            .and_then(|p| p.value.clone().into_uint())
            .ok_or(DecodingError::MissingField("amount0".to_string()))?;

        let amount1 = decoded
            .params
            .get(6)
            .and_then(|p| p.value.clone().into_uint())
            .ok_or(DecodingError::MissingField("amount1".to_string()))?;

        Ok(ValidatedMint {
            pool_address: pool_address.0,
            liquidity_provider: owner.0,
            liquidity_delta: liquidity.as_u128(),
            amount0: amount0.as_u128(),
            amount1: amount1.as_u128(),
            tick_lower: tick_lower.as_u32() as i32,
            tick_upper: tick_upper.as_u32() as i32,
            dex_protocol: protocol,
        })
    }

    /// Decode V2 mint event
    fn decode_v2_mint(
        pool_address: H160,
        raw_log: RawLog,
        protocol: DEXProtocol,
    ) -> Result<ValidatedMint, DecodingError> {
        let event = uniswap_v2::mint_event();
        let decoded = event
            .parse_log(raw_log)
            .map_err(|e| DecodingError::AbiParsingError(e.to_string()))?;

        // Extract sender as liquidity provider
        let sender = decoded
            .params
            .get(0)
            .and_then(|p| p.value.clone().into_address())
            .ok_or(DecodingError::MissingField("sender".to_string()))?;

        // Extract amounts
        let amount0 = decoded
            .params
            .get(1)
            .and_then(|p| p.value.clone().into_uint())
            .ok_or(DecodingError::MissingField("amount0".to_string()))?;

        let amount1 = decoded
            .params
            .get(2)
            .and_then(|p| p.value.clone().into_uint())
            .ok_or(DecodingError::MissingField("amount1".to_string()))?;

        // V2 doesn't have ticks, use full range
        Ok(ValidatedMint {
            pool_address: pool_address.0,
            liquidity_provider: sender.0,
            liquidity_delta: 0, // V2 doesn't expose liquidity in mint
            amount0: amount0.as_u128(),
            amount1: amount1.as_u128(),
            tick_lower: -887272, // MIN_TICK for V2
            tick_upper: 887272,  // MAX_TICK for V2
            dex_protocol: protocol,
        })
    }
}

/// ABI decoder for Burn events
pub struct BurnEventDecoder;

impl BurnEventDecoder {
    /// Decode burn event based on protocol
    pub fn decode_burn_event(
        log: &Log,
        protocol: DEXProtocol,
    ) -> Result<ValidatedBurn, DecodingError> {
        let raw_log = RawLog {
            topics: log.topics.clone(),
            data: log.data.0.clone(),
        };

        match protocol {
            DEXProtocol::UniswapV3 | DEXProtocol::QuickswapV3 => {
                Self::decode_v3_burn(log.address, raw_log, protocol)
            }
            DEXProtocol::UniswapV2 | DEXProtocol::SushiswapV2 | DEXProtocol::QuickswapV2 => {
                Self::decode_v2_burn(log.address, raw_log, protocol)
            }
        }
    }

    /// Decode V3 burn event
    fn decode_v3_burn(
        pool_address: H160,
        raw_log: RawLog,
        protocol: DEXProtocol,
    ) -> Result<ValidatedBurn, DecodingError> {
        let event = uniswap_v3::burn_event();
        let decoded = event
            .parse_log(raw_log)
            .map_err(|e| DecodingError::AbiParsingError(e.to_string()))?;

        // Extract owner
        let owner = decoded
            .params
            .get(0)
            .and_then(|p| p.value.clone().into_address())
            .ok_or(DecodingError::MissingField("owner".to_string()))?;

        // Extract tick range
        let tick_lower = decoded
            .params
            .get(1)
            .and_then(|p| p.value.clone().into_int())
            .ok_or(DecodingError::MissingField("tickLower".to_string()))?;

        let tick_upper = decoded
            .params
            .get(2)
            .and_then(|p| p.value.clone().into_int())
            .ok_or(DecodingError::MissingField("tickUpper".to_string()))?;

        // Extract liquidity and amounts
        let liquidity = decoded
            .params
            .get(3)
            .and_then(|p| p.value.clone().into_uint())
            .ok_or(DecodingError::MissingField("amount".to_string()))?;

        let amount0 = decoded
            .params
            .get(4)
            .and_then(|p| p.value.clone().into_uint())
            .ok_or(DecodingError::MissingField("amount0".to_string()))?;

        let amount1 = decoded
            .params
            .get(5)
            .and_then(|p| p.value.clone().into_uint())
            .ok_or(DecodingError::MissingField("amount1".to_string()))?;

        Ok(ValidatedBurn {
            pool_address: pool_address.0,
            liquidity_provider: owner.0,
            liquidity_delta: liquidity.as_u128(),
            amount0: amount0.as_u128(),
            amount1: amount1.as_u128(),
            tick_lower: tick_lower.as_u32() as i32,
            tick_upper: tick_upper.as_u32() as i32,
            dex_protocol: protocol,
        })
    }

    /// Decode V2 burn event
    fn decode_v2_burn(
        pool_address: H160,
        raw_log: RawLog,
        protocol: DEXProtocol,
    ) -> Result<ValidatedBurn, DecodingError> {
        let event = uniswap_v2::burn_event();
        let decoded = event
            .parse_log(raw_log)
            .map_err(|e| DecodingError::AbiParsingError(e.to_string()))?;

        // Extract sender
        let _sender = decoded
            .params
            .get(0)
            .and_then(|p| p.value.clone().into_address())
            .ok_or(DecodingError::MissingField("sender".to_string()))?;

        // Extract amounts
        let amount0 = decoded
            .params
            .get(1)
            .and_then(|p| p.value.clone().into_uint())
            .ok_or(DecodingError::MissingField("amount0".to_string()))?;

        let amount1 = decoded
            .params
            .get(2)
            .and_then(|p| p.value.clone().into_uint())
            .ok_or(DecodingError::MissingField("amount1".to_string()))?;

        // Extract recipient
        let to = decoded
            .params
            .get(3)
            .and_then(|p| p.value.clone().into_address())
            .ok_or(DecodingError::MissingField("to".to_string()))?;

        Ok(ValidatedBurn {
            pool_address: pool_address.0,
            liquidity_provider: to.0, // Use recipient as LP
            liquidity_delta: 0,       // V2 doesn't expose liquidity in burn
            amount0: amount0.as_u128(),
            amount1: amount1.as_u128(),
            tick_lower: -887272, // MIN_TICK for V2
            tick_upper: 887272,  // MAX_TICK for V2
            dex_protocol: protocol,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use web3::types::{Bytes, H256};

    fn create_test_log(topics: Vec<H256>, data: Vec<u8>) -> Log {
        Log {
            address: H160::from_low_u64_be(0x1234),
            topics,
            data: Bytes(data),
            block_hash: None,
            block_number: None,
            transaction_hash: None,
            transaction_index: None,
            log_index: None,
            transaction_log_index: None,
            log_type: None,
            removed: None,
        }
    }

    #[test]
    fn test_safe_u256_to_i64() {
        // Test normal value
        let normal = U256::from(1000000);
        assert_eq!(SwapEventDecoder::safe_u256_to_i64(normal).unwrap(), 1000000);

        // Test max i64 value
        let max_safe = U256::from(i64::MAX);
        assert_eq!(
            SwapEventDecoder::safe_u256_to_i64(max_safe).unwrap(),
            i64::MAX
        );

        // Test overflow
        let overflow = U256::from(i64::MAX) + U256::from(1);
        assert_eq!(
            SwapEventDecoder::safe_u256_to_i64(overflow).unwrap(),
            i64::MAX
        );
    }

    #[test]
    fn test_dex_protocol_detection() {
        let v3_log = create_test_log(vec![H256::zero(); 3], vec![0u8; 150]);
        let v2_log = create_test_log(vec![H256::zero(); 3], vec![0u8; 64]);

        let v3_addr = H160::from_low_u64_be(0x1234);
        let v2_addr = H160::from_low_u64_be(0x5678);

        assert_eq!(
            detect_dex_protocol(&v3_addr, &v3_log),
            DEXProtocol::UniswapV3
        );
        assert_eq!(
            detect_dex_protocol(&v2_addr, &v2_log),
            DEXProtocol::UniswapV2
        );
    }
}

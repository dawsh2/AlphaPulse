//! ABI-based event decoding for semantic validation
//!
//! This module provides semantic validation by using official DEX contract ABIs
//! to decode events instead of manual byte parsing. This prevents semantic errors
//! like storing fees in profit fields or amount_in/amount_out confusion.

use ethabi::{Event, EventParam, ParamType, RawLog, Token, decode, Address as EthAddress};
use web3::types::{Log, H160, H256};
use anyhow::{Result, anyhow};
use std::collections::HashMap;
use once_cell::sync::Lazy;

// Import the actual DEXProtocol from alphapulse_protocol_v2
use protocol_v2::tlv::DEXProtocol as ProtocolDEXProtocol;

// =============================================================================
// DEX EVENT ABI DEFINITIONS
// =============================================================================

/// Uniswap V3 Swap event ABI definition
/// event Swap(address indexed sender, address indexed recipient, int256 amount0, int256 amount1, uint160 sqrtPriceX96, uint128 liquidity, int24 tick)
static UNISWAP_V3_SWAP_EVENT: Lazy<Event> = Lazy::new(|| {
    Event {
        name: "Swap".to_string(),
        inputs: vec![
            EventParam { name: "sender".to_string(), kind: ParamType::Address, indexed: true },
            EventParam { name: "recipient".to_string(), kind: ParamType::Address, indexed: true },
            EventParam { name: "amount0".to_string(), kind: ParamType::Int(256), indexed: false },
            EventParam { name: "amount1".to_string(), kind: ParamType::Int(256), indexed: false },
            EventParam { name: "sqrtPriceX96".to_string(), kind: ParamType::Uint(160), indexed: false },
            EventParam { name: "liquidity".to_string(), kind: ParamType::Uint(128), indexed: false },
            EventParam { name: "tick".to_string(), kind: ParamType::Int(24), indexed: false },
        ],
        anonymous: false,
    }
});

/// Uniswap V2/QuickSwap Swap event ABI definition  
/// event Swap(address indexed sender, uint256 amount0In, uint256 amount1In, uint256 amount0Out, uint256 amount1Out, address indexed to)
static UNISWAP_V2_SWAP_EVENT: Lazy<Event> = Lazy::new(|| {
    Event {
        name: "Swap".to_string(),
        inputs: vec![
            EventParam { name: "sender".to_string(), kind: ParamType::Address, indexed: true },
            EventParam { name: "amount0In".to_string(), kind: ParamType::Uint(256), indexed: false },
            EventParam { name: "amount1In".to_string(), kind: ParamType::Uint(256), indexed: false },
            EventParam { name: "amount0Out".to_string(), kind: ParamType::Uint(256), indexed: false },
            EventParam { name: "amount1Out".to_string(), kind: ParamType::Uint(256), indexed: false },
            EventParam { name: "to".to_string(), kind: ParamType::Address, indexed: true },
        ],
        anonymous: false,
    }
});

/// Uniswap V3 Mint event ABI definition
/// event Mint(address sender, address indexed owner, int24 indexed tickLower, int24 indexed tickUpper, uint128 amount, uint256 amount0, uint256 amount1)
static UNISWAP_V3_MINT_EVENT: Lazy<Event> = Lazy::new(|| {
    Event {
        name: "Mint".to_string(),
        inputs: vec![
            EventParam { name: "sender".to_string(), kind: ParamType::Address, indexed: false },
            EventParam { name: "owner".to_string(), kind: ParamType::Address, indexed: true },
            EventParam { name: "tickLower".to_string(), kind: ParamType::Int(24), indexed: true },
            EventParam { name: "tickUpper".to_string(), kind: ParamType::Int(24), indexed: true },
            EventParam { name: "amount".to_string(), kind: ParamType::Uint(128), indexed: false },
            EventParam { name: "amount0".to_string(), kind: ParamType::Uint(256), indexed: false },
            EventParam { name: "amount1".to_string(), kind: ParamType::Uint(256), indexed: false },
        ],
        anonymous: false,
    }
});

/// Uniswap V3 Burn event ABI definition
/// event Burn(address indexed owner, int24 indexed tickLower, int24 indexed tickUpper, uint128 amount, uint256 amount0, uint256 amount1)
static UNISWAP_V3_BURN_EVENT: Lazy<Event> = Lazy::new(|| {
    Event {
        name: "Burn".to_string(),
        inputs: vec![
            EventParam { name: "owner".to_string(), kind: ParamType::Address, indexed: true },
            EventParam { name: "tickLower".to_string(), kind: ParamType::Int(24), indexed: true },
            EventParam { name: "tickUpper".to_string(), kind: ParamType::Int(24), indexed: true },
            EventParam { name: "amount".to_string(), kind: ParamType::Uint(128), indexed: false },
            EventParam { name: "amount0".to_string(), kind: ParamType::Uint(256), indexed: false },
            EventParam { name: "amount1".to_string(), kind: ParamType::Uint(256), indexed: false },
        ],
        anonymous: false,
    }
});

// =============================================================================
// SEMANTIC SWAP DATA STRUCTURES
// =============================================================================

/// Semantically validated swap data from ABI decoding
#[derive(Debug, Clone)]
pub struct ValidatedSwapData {
    /// Pool contract address that emitted this event
    pub pool_address: [u8; 20],
    /// Address that initiated the swap transaction
    pub sender: [u8; 20],
    /// Address that received the output tokens
    pub recipient: [u8; 20],
    /// Input amount in token's native precision (positive = money flowing into pool)
    pub amount_in: u128,
    /// Output amount in token's native precision (positive = money flowing out of pool) 
    pub amount_out: u128,
    /// Whether token0 was the input token (true) or token1 was input (false)
    pub token_in_is_token0: bool,
    /// Price after swap for V3 pools (0 for V2)
    pub sqrt_price_x96_after: [u8; 20],
    /// Active liquidity in the pool for V3 pools (0 for V2)
    pub liquidity_after: u128,
    /// Current tick for V3 pools (0 for V2)
    pub tick_after: i32,
    /// DEX protocol that emitted this event
    pub dex_protocol: ProtocolDEXProtocol,
}

/// Semantically validated mint data from ABI decoding
#[derive(Debug, Clone)]
pub struct ValidatedMintData {
    pub pool_address: [u8; 20],
    pub liquidity_provider: [u8; 20],
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub liquidity_delta: u128,
    pub amount0: u128,
    pub amount1: u128,
    pub dex_protocol: ProtocolDEXProtocol,
}

/// Semantically validated burn data from ABI decoding
#[derive(Debug, Clone)]
pub struct ValidatedBurnData {
    pub pool_address: [u8; 20],
    pub liquidity_provider: [u8; 20],
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub liquidity_delta: u128,
    pub amount0: u128,
    pub amount1: u128,
    pub dex_protocol: ProtocolDEXProtocol,
}

/// DEX protocol identification for semantic mapping (local enum for detection logic)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DEXProtocol {
    UniswapV2,
    UniswapV3,
    QuickSwapV2,
    QuickSwapV3,
    SushiSwapV2,
}

impl DEXProtocol {
    /// Convert to protocol DEXProtocol enum
    pub fn to_protocol_enum(self) -> ProtocolDEXProtocol {
        match self {
            DEXProtocol::UniswapV2 => ProtocolDEXProtocol::UniswapV2,
            DEXProtocol::UniswapV3 => ProtocolDEXProtocol::UniswapV3,
            DEXProtocol::QuickSwapV2 => ProtocolDEXProtocol::UniswapV2, // QuickSwap V2 compatible
            DEXProtocol::QuickSwapV3 => ProtocolDEXProtocol::UniswapV3, // QuickSwap V3 compatible  
            DEXProtocol::SushiSwapV2 => ProtocolDEXProtocol::SushiswapV2,
        }
    }
}

// =============================================================================
// EVENT SIGNATURE DETECTION
// =============================================================================

/// Known event signatures mapped to DEX protocols and event types
static EVENT_SIGNATURE_MAP: Lazy<HashMap<H256, (DEXProtocol, EventType)>> = Lazy::new(|| {
    let mut map = HashMap::new();
    
    // Uniswap V3 event signatures (different from V2!)
    map.insert(
        H256::from_slice(&hex::decode("c42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67").unwrap()),
        (DEXProtocol::UniswapV3, EventType::Swap)
    );
    map.insert(
        H256::from_slice(&hex::decode("7a53080ba414158be7ec69b987b5fb7d07dee101fe85488f0853ae16239d0bde").unwrap()),
        (DEXProtocol::UniswapV3, EventType::Mint)
    );
    map.insert(
        H256::from_slice(&hex::decode("0c396cd989a39f4459b5fa1aed6a9a8dcdbc45908acfd67e028cd568da98982c").unwrap()),
        (DEXProtocol::UniswapV3, EventType::Burn)
    );
    
    // Uniswap V2/QuickSwap/SushiSwap share same signatures (different from V3)
    map.insert(
        H256::from_slice(&hex::decode("d78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822").unwrap()),
        (DEXProtocol::UniswapV2, EventType::Swap) // V2 signature - different parameters than V3
    );
    
    map
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EventType {
    Swap,
    Mint,
    Burn,
}

// =============================================================================
// ABI-BASED EVENT DECODERS
// =============================================================================

/// Swap event decoder with semantic validation
pub struct SwapEventDecoder;

impl SwapEventDecoder {
    /// Decode swap event using appropriate ABI based on pool type detection
    pub fn decode_swap_event(log: &Log, dex_protocol: DEXProtocol) -> Result<ValidatedSwapData> {
        let pool_address = Self::h160_to_bytes(&log.address);
        
        // Convert Web3 Log to ethabi RawLog
        let raw_log = Self::web3_log_to_raw_log(log)?;
        
        match dex_protocol {
            DEXProtocol::UniswapV3 | DEXProtocol::QuickSwapV3 => {
                Self::decode_v3_swap(raw_log, pool_address, dex_protocol)
            }
            DEXProtocol::UniswapV2 | DEXProtocol::QuickSwapV2 | DEXProtocol::SushiSwapV2 => {
                Self::decode_v2_swap(raw_log, pool_address, dex_protocol)
            }
        }
    }
    
    /// Decode Uniswap V3 swap event with semantic validation
    fn decode_v3_swap(raw_log: RawLog, pool_address: [u8; 20], dex_protocol: DEXProtocol) -> Result<ValidatedSwapData> {
        let decoded = UNISWAP_V3_SWAP_EVENT.parse_log(raw_log)
            .map_err(|e| anyhow!("Failed to decode V3 swap event: {}", e))?;
        
        // Extract semantically correct fields
        let sender = Self::extract_address(&decoded.params, "sender")?;
        let recipient = Self::extract_address(&decoded.params, "recipient")?;
        let amount0 = Self::extract_int256(&decoded.params, "amount0")?;
        let amount1 = Self::extract_int256(&decoded.params, "amount1")?;
        let sqrt_price_x96_after = Self::extract_uint160(&decoded.params, "sqrtPriceX96")?;
        let liquidity_after = Self::extract_uint128(&decoded.params, "liquidity")?;
        let tick_after = Self::extract_int24(&decoded.params, "tick")?;
        
        // Semantic validation: determine input/output amounts based on signs
        // V3 swaps: one amount is positive (in), one is negative (out)
        // Convert to our schema where both are positive
        let (amount_in, amount_out, token_in_is_token0) = if amount0 > 0 && amount1 <= 0 {
            // Token0 in (positive), Token1 out (negative or zero)
            let out_amount = if amount1 < 0 { (-amount1) as u128 } else { 0 };
            let in_amount = amount0 as u128;
            (in_amount, out_amount, true)
        } else if amount0 <= 0 && amount1 > 0 {
            // Token1 in (positive), Token0 out (negative or zero)
            let out_amount = if amount0 < 0 { (-amount0) as u128 } else { 0 };
            let in_amount = amount1 as u128;
            (in_amount, out_amount, false)
        } else if amount0 == 0 && amount1 == 0 {
            // Both zero - shouldn't happen in real swaps but handle gracefully
            return Err(anyhow!("Invalid V3 swap: both amounts are zero"));
        } else {
            // Both positive or both negative - invalid
            return Err(anyhow!("Invalid V3 swap amounts: amount0={}, amount1={}", amount0, amount1));
        };
        
        Ok(ValidatedSwapData {
            pool_address,
            sender,
            recipient,
            amount_in,
            amount_out,
            token_in_is_token0,
            sqrt_price_x96_after,
            liquidity_after,
            tick_after,
            dex_protocol: dex_protocol.to_protocol_enum(),
        })
    }
    
    /// Decode Uniswap V2 swap event with semantic validation
    fn decode_v2_swap(raw_log: RawLog, pool_address: [u8; 20], dex_protocol: DEXProtocol) -> Result<ValidatedSwapData> {
        let decoded = UNISWAP_V2_SWAP_EVENT.parse_log(raw_log)
            .map_err(|e| anyhow!("Failed to decode V2 swap event: {}", e))?;
        
        // Extract semantically correct fields
        let sender = Self::extract_address(&decoded.params, "sender")?;
        let recipient = Self::extract_address(&decoded.params, "to")?;
        let amount0_in = Self::extract_uint256(&decoded.params, "amount0In")?;
        let amount1_in = Self::extract_uint256(&decoded.params, "amount1In")?;
        let amount0_out = Self::extract_uint256(&decoded.params, "amount0Out")?;
        let amount1_out = Self::extract_uint256(&decoded.params, "amount1Out")?;
        
        // Semantic validation: exactly one of (amount0_in, amount1_in) should be > 0
        // and exactly one of (amount0_out, amount1_out) should be > 0
        let (amount_in, amount_out, token_in_is_token0) = 
            if amount0_in > 0 && amount1_in == 0 && amount0_out == 0 && amount1_out > 0 {
                // Token0 in, Token1 out - use u128 directly
                (amount0_in, amount1_out, true)
            } else if amount0_in == 0 && amount1_in > 0 && amount0_out > 0 && amount1_out == 0 {
                // Token1 in, Token0 out - use u128 directly
                (amount1_in, amount0_out, false)
            } else {
                return Err(anyhow!("Invalid V2 swap amounts: amount0_in={}, amount1_in={}, amount0_out={}, amount1_out={}", 
                                 amount0_in, amount1_in, amount0_out, amount1_out));
            };
        
        Ok(ValidatedSwapData {
            pool_address,
            sender,
            recipient,
            amount_in,
            amount_out,
            token_in_is_token0,
            sqrt_price_x96_after: [0u8; 20], // V2 doesn't have price tracking
            liquidity_after: 0,      // V2 doesn't have active liquidity
            tick_after: 0,           // V2 doesn't have ticks
            dex_protocol: dex_protocol.to_protocol_enum(),
        })
    }
    
    // Helper methods for extracting typed parameters from decoded event
    fn extract_address(params: &[ethabi::LogParam], name: &str) -> Result<[u8; 20]> {
        let param = params.iter()
            .find(|p| p.name == name)
            .ok_or_else(|| anyhow!("Missing parameter: {}", name))?;
        
        match &param.value {
            Token::Address(addr) => Ok(addr.0),
            _ => Err(anyhow!("Parameter {} is not an address", name)),
        }
    }
    
    fn extract_int256(params: &[ethabi::LogParam], name: &str) -> Result<i128> {
        let param = params.iter()
            .find(|p| p.name == name)
            .ok_or_else(|| anyhow!("Missing parameter: {}", name))?;
        
        match &param.value {
            Token::Int(value) => {
                // Convert U256 to i128, handling the sign bit
                if value.bit(255) {
                    // Negative number (two's complement)
                    // Use bitwise NOT and add 1 for two's complement
                    let mut bytes = [0u8; 32];
                    value.to_big_endian(&mut bytes);
                    
                    // Flip all bits
                    for byte in &mut bytes {
                        *byte = !*byte;
                    }
                    
                    // Add 1 for two's complement
                    let mut carry = 1u8;
                    for byte in bytes.iter_mut().rev() {
                        let sum = *byte as u16 + carry as u16;
                        *byte = (sum & 0xFF) as u8;
                        carry = (sum >> 8) as u8;
                    }
                    
                    // Convert to i128 (take last 16 bytes)
                    let mut result_bytes = [0u8; 16];
                    result_bytes.copy_from_slice(&bytes[16..]);
                    let positive = u128::from_be_bytes(result_bytes);
                    Ok(-(positive as i128))
                } else {
                    // Positive number - check if it fits in u128
                    let mut bytes = [0u8; 32];
                    value.to_big_endian(&mut bytes);
                    
                    // Check if upper 16 bytes are zero (fits in u128)
                    if bytes[..16].iter().all(|&b| b == 0) {
                        let mut result_bytes = [0u8; 16];
                        result_bytes.copy_from_slice(&bytes[16..]);
                        Ok(u128::from_be_bytes(result_bytes) as i128)
                    } else {
                        // Value too large for i128, truncate
                        Ok(i128::MAX)
                    }
                }
            }
            _ => Err(anyhow!("Parameter {} is not an int256", name)),
        }
    }
    
    fn extract_uint256(params: &[ethabi::LogParam], name: &str) -> Result<u128> {
        let param = params.iter()
            .find(|p| p.name == name)
            .ok_or_else(|| anyhow!("Missing parameter: {}", name))?;
        
        match &param.value {
            Token::Uint(value) => {
                // Check if value fits in u128
                let mut bytes = [0u8; 32];
                value.to_big_endian(&mut bytes);
                
                // Check if upper 16 bytes are zero (fits in u128)
                if bytes[..16].iter().all(|&b| b == 0) {
                    let mut result_bytes = [0u8; 16];
                    result_bytes.copy_from_slice(&bytes[16..]);
                    Ok(u128::from_be_bytes(result_bytes))
                } else {
                    // Value too large for u128, truncate
                    Ok(u128::MAX)
                }
            }
            _ => Err(anyhow!("Parameter {} is not a uint256", name)),
        }
    }
    
    /// Safely convert u128 to i64 with proper bounds checking
    fn safe_u128_to_i64(value: u128) -> Result<i64> {
        if value > i64::MAX as u128 {
            // For production, we should reject overflow rather than truncate
            // This prevents incorrect values from propagating through the system
            return Err(anyhow!(
                "Value {} exceeds i64::MAX ({}). This likely indicates an error in the source data.",
                value, i64::MAX
            ));
        }
        Ok(value as i64)
    }
    
    /// Safely convert i128 to i64 with proper bounds checking
    fn safe_i128_to_i64(value: i128) -> Result<i64> {
        if value > i64::MAX as i128 {
            return Err(anyhow!(
                "Value {} exceeds i64::MAX ({}). This likely indicates an error in the source data.",
                value, i64::MAX
            ));
        }
        if value < i64::MIN as i128 {
            return Err(anyhow!(
                "Value {} is less than i64::MIN ({}). This likely indicates an error in the source data.",
                value, i64::MIN
            ));
        }
        Ok(value as i64)
    }
    
    fn extract_uint128(params: &[ethabi::LogParam], name: &str) -> Result<u128> {
        let param = params.iter()
            .find(|p| p.name == name)
            .ok_or_else(|| anyhow!("Missing parameter: {}", name))?;
        
        match &param.value {
            Token::Uint(value) => {
                // Extract as u128 safely
                let mut bytes = [0u8; 32];
                value.to_big_endian(&mut bytes);
                
                // Take last 16 bytes (should fit in u128)
                let mut result_bytes = [0u8; 16];
                result_bytes.copy_from_slice(&bytes[16..]);
                Ok(u128::from_be_bytes(result_bytes))
            }
            _ => Err(anyhow!("Parameter {} is not a uint128", name)),
        }
    }
    
    fn extract_uint160(params: &[ethabi::LogParam], name: &str) -> Result<[u8; 20]> {
        let param = params.iter()
            .find(|p| p.name == name)
            .ok_or_else(|| anyhow!("Missing parameter: {}", name))?;
        
        match &param.value {
            Token::Uint(value) => {
                // uint160 is exactly 20 bytes - preserve full precision
                let mut bytes = [0u8; 32];
                value.to_big_endian(&mut bytes);
                
                // Take the last 20 bytes (uint160 is 160 bits = 20 bytes)
                let mut uint160_bytes = [0u8; 20];
                uint160_bytes.copy_from_slice(&bytes[12..32]);
                Ok(uint160_bytes)
            }
            _ => Err(anyhow!("Parameter {} is not a uint", name)),
        }
    }
    
    fn extract_int24(params: &[ethabi::LogParam], name: &str) -> Result<i32> {
        let param = params.iter()
            .find(|p| p.name == name)
            .ok_or_else(|| anyhow!("Missing parameter: {}", name))?;
        
        match &param.value {
            Token::Int(value) => {
                let val = value.as_u32() as i32;
                // Handle 24-bit signed integer properly
                if val > 0x7FFFFF {
                    Ok(val - 0x1000000) // Convert from unsigned to signed 24-bit
                } else {
                    Ok(val)
                }
            }
            _ => Err(anyhow!("Parameter {} is not an int24", name)),
        }
    }
    
    fn h160_to_bytes(address: &H160) -> [u8; 20] {
        address.0
    }
    
    fn web3_log_to_raw_log(log: &Log) -> Result<RawLog> {
        Ok(RawLog {
            topics: log.topics.clone(),
            data: log.data.0.clone(),
        })
    }
}

/// Mint event decoder with semantic validation
pub struct MintEventDecoder;

impl MintEventDecoder {
    /// Decode mint event using appropriate ABI
    pub fn decode_mint_event(log: &Log, dex_protocol: DEXProtocol) -> Result<ValidatedMintData> {
        let pool_address = SwapEventDecoder::h160_to_bytes(&log.address);
        let raw_log = SwapEventDecoder::web3_log_to_raw_log(log)?;
        
        match dex_protocol {
            DEXProtocol::UniswapV3 | DEXProtocol::QuickSwapV3 => {
                Self::decode_v3_mint(raw_log, pool_address, dex_protocol)
            }
            DEXProtocol::UniswapV2 | DEXProtocol::QuickSwapV2 | DEXProtocol::SushiSwapV2 => {
                // V2 mints are different - implement separately if needed
                Err(anyhow!("V2 mint decoding not yet implemented"))
            }
        }
    }
    
    fn decode_v3_mint(raw_log: RawLog, pool_address: [u8; 20], dex_protocol: DEXProtocol) -> Result<ValidatedMintData> {
        let decoded = UNISWAP_V3_MINT_EVENT.parse_log(raw_log)
            .map_err(|e| anyhow!("Failed to decode V3 mint event: {}", e))?;
        
        let liquidity_provider = SwapEventDecoder::extract_address(&decoded.params, "owner")?;
        let tick_lower = SwapEventDecoder::extract_int24(&decoded.params, "tickLower")?;
        let tick_upper = SwapEventDecoder::extract_int24(&decoded.params, "tickUpper")?;
        let liquidity_delta = SwapEventDecoder::extract_uint128(&decoded.params, "amount")?;
        let amount0 = SwapEventDecoder::extract_uint256(&decoded.params, "amount0")?;
        let amount1 = SwapEventDecoder::extract_uint256(&decoded.params, "amount1")?;
        
        Ok(ValidatedMintData {
            pool_address,
            liquidity_provider,
            tick_lower,
            tick_upper,
            liquidity_delta,
            amount0,
            amount1,
            dex_protocol: dex_protocol.to_protocol_enum(),
        })
    }
}

/// Burn event decoder with semantic validation
pub struct BurnEventDecoder;

impl BurnEventDecoder {
    /// Decode burn event using appropriate ABI
    pub fn decode_burn_event(log: &Log, dex_protocol: DEXProtocol) -> Result<ValidatedBurnData> {
        let pool_address = SwapEventDecoder::h160_to_bytes(&log.address);
        let raw_log = SwapEventDecoder::web3_log_to_raw_log(log)?;
        
        match dex_protocol {
            DEXProtocol::UniswapV3 | DEXProtocol::QuickSwapV3 => {
                Self::decode_v3_burn(raw_log, pool_address, dex_protocol)
            }
            DEXProtocol::UniswapV2 | DEXProtocol::QuickSwapV2 | DEXProtocol::SushiSwapV2 => {
                // V2 burns are different - implement separately if needed
                Err(anyhow!("V2 burn decoding not yet implemented"))
            }
        }
    }
    
    fn decode_v3_burn(raw_log: RawLog, pool_address: [u8; 20], dex_protocol: DEXProtocol) -> Result<ValidatedBurnData> {
        let decoded = UNISWAP_V3_BURN_EVENT.parse_log(raw_log)
            .map_err(|e| anyhow!("Failed to decode V3 burn event: {}", e))?;
        
        let liquidity_provider = SwapEventDecoder::extract_address(&decoded.params, "owner")?;
        let tick_lower = SwapEventDecoder::extract_int24(&decoded.params, "tickLower")?;
        let tick_upper = SwapEventDecoder::extract_int24(&decoded.params, "tickUpper")?;
        let liquidity_delta = SwapEventDecoder::extract_uint128(&decoded.params, "amount")?;
        let amount0 = SwapEventDecoder::extract_uint256(&decoded.params, "amount0")?;
        let amount1 = SwapEventDecoder::extract_uint256(&decoded.params, "amount1")?;
        
        Ok(ValidatedBurnData {
            pool_address,
            liquidity_provider,
            tick_lower,
            tick_upper,
            liquidity_delta,
            amount0,
            amount1,
            dex_protocol: dex_protocol.to_protocol_enum(),
        })
    }
}

// =============================================================================
// POOL TYPE DETECTION UTILITIES
// =============================================================================

/// Detect DEX protocol from pool address and contract interaction patterns
pub fn detect_dex_protocol(_pool_address: &H160, log: &Log) -> DEXProtocol {
    // This is a simplified detection - in production would use:
    // 1. Registry of known factory addresses
    // 2. Contract method calls to detect V2 vs V3
    // 3. Pool creation event analysis
    
    // Detect based on event data structure
    // V3 Swap has 5 data fields (160 bytes): amount0, amount1, sqrtPriceX96, liquidity, tick
    // V2 Swap has 4 data fields (128 bytes): amount0In, amount1In, amount0Out, amount1Out
    
    if log.data.0.len() >= 160 {
        // V3 events have 5 x 32-byte parameters = 160 bytes minimum
        DEXProtocol::UniswapV3
    } else if log.data.0.len() >= 128 {
        // V2 events have 4 x 32-byte parameters = 128 bytes
        DEXProtocol::UniswapV2
    } else {
        // Default to V2 for unknown/malformed
        DEXProtocol::UniswapV2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use web3::types::{H256, Bytes};
    
    #[test]
    fn test_event_signature_detection() {
        let swap_sig = H256::from_slice(&hex::decode("d78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822").unwrap());
        
        if let Some((protocol, event_type)) = EVENT_SIGNATURE_MAP.get(&swap_sig) {
            assert_eq!(*event_type, EventType::Swap);
            // Note: Same signature used by V2 and V3, differentiated by detection logic
        }
    }
    
    #[test]
    fn test_address_conversion() {
        let h160_addr = H160::from_slice(&[1u8; 20]);
        let bytes_addr = SwapEventDecoder::h160_to_bytes(&h160_addr);
        assert_eq!(bytes_addr, [1u8; 20]);
    }
}
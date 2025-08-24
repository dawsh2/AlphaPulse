//! Market Data TLV Structures
//!
//! Defines concrete TLV structures for market data messages

use crate::{InstrumentId, VenueId}; // TLVType removed with legacy TLV system
                                    // Legacy TLV types removed - using Protocol V2 MessageHeader + TLV extensions
use std::convert::TryInto;
use zerocopy::{AsBytes, FromBytes, FromZeroes};

/// Trade TLV structure - simplified for serialization
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, AsBytes, FromBytes, FromZeroes)]
pub struct TradeTLV {
    pub venue_id: u16,     // VenueId as primitive
    pub asset_type: u8,    // AssetType as primitive
    pub reserved: u8,      // Reserved byte for alignment
    pub asset_id: u64,     // Asset identifier
    pub price: i64,        // Fixed-point with 8 decimals
    pub volume: i64,       // Fixed-point with 8 decimals
    pub side: u8,          // 0 = buy, 1 = sell
    pub timestamp_ns: u64, // Nanoseconds since epoch
}

impl TradeTLV {
    /// Create from high-level types
    pub fn new(
        venue: VenueId,
        instrument_id: InstrumentId,
        price: i64,
        volume: i64,
        side: u8,
        timestamp_ns: u64,
    ) -> Self {
        Self {
            venue_id: venue as u16,
            asset_type: instrument_id.asset_type,
            reserved: instrument_id.reserved,
            asset_id: instrument_id.asset_id,
            price,
            volume,
            side,
            timestamp_ns,
        }
    }

    /// Convert to InstrumentId
    pub fn instrument_id(&self) -> InstrumentId {
        InstrumentId {
            venue: self.venue_id,
            asset_type: self.asset_type,
            reserved: self.reserved,
            asset_id: self.asset_id,
        }
    }

    /// Convert to VenueId  
    pub fn venue(&self) -> Result<VenueId, crate::ProtocolError> {
        VenueId::try_from(self.venue_id).map_err(|_| crate::ProtocolError::InvalidInstrument)
    }

    // Legacy to_tlv_message removed - use Protocol V2 TLVMessageBuilder instead

    /// Parse from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        if data.len() < std::mem::size_of::<Self>() {
            return Err("Data too short for TradeTLV".to_string());
        }

        use zerocopy::Ref;
        let tlv_ref = Ref::<_, Self>::new(data).ok_or("Failed to parse TradeTLV from bytes")?;
        Ok(*tlv_ref.into_ref())
    }
}

/// Quote TLV structure (best bid/ask) - optimized for zero-copy serialization
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, AsBytes, FromBytes, FromZeroes)]
pub struct QuoteTLV {
    pub venue_id: u16,     // VenueId as primitive
    pub asset_type: u8,    // AssetType as primitive
    pub reserved: u8,      // Reserved byte for alignment
    pub asset_id: u64,     // Asset identifier
    pub bid_price: i64,    // Fixed-point with 8 decimals
    pub bid_size: i64,     // Fixed-point with 8 decimals
    pub ask_price: i64,    // Fixed-point with 8 decimals
    pub ask_size: i64,     // Fixed-point with 8 decimals
    pub timestamp_ns: u64, // Nanoseconds since epoch
}

impl QuoteTLV {
    /// Create from high-level types
    pub fn new(
        venue: VenueId,
        instrument_id: InstrumentId,
        bid_price: i64,
        bid_size: i64,
        ask_price: i64,
        ask_size: i64,
        timestamp_ns: u64,
    ) -> Self {
        Self {
            venue_id: venue as u16,
            asset_type: instrument_id.asset_type,
            reserved: instrument_id.reserved,
            asset_id: instrument_id.asset_id,
            bid_price,
            bid_size,
            ask_price,
            ask_size,
            timestamp_ns,
        }
    }

    /// Convert to InstrumentId
    pub fn instrument_id(&self) -> InstrumentId {
        InstrumentId {
            venue: self.venue_id,
            asset_type: self.asset_type,
            reserved: self.reserved,
            asset_id: self.asset_id,
        }
    }

    /// Convert to VenueId  
    pub fn venue(&self) -> Result<VenueId, crate::ProtocolError> {
        VenueId::try_from(self.venue_id).map_err(|_| crate::ProtocolError::InvalidInstrument)
    }

    // Legacy to_tlv_message removed - use Protocol V2 TLVMessageBuilder instead

    /// Parse from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        if data.len() < std::mem::size_of::<Self>() {
            return Err("Data too short for QuoteTLV".to_string());
        }

        use zerocopy::Ref;
        let tlv_ref = Ref::<_, Self>::new(data).ok_or("Failed to parse QuoteTLV from bytes")?;
        Ok(*tlv_ref.into_ref())
    }
}

/// State invalidation TLV structure
#[derive(Debug, Clone, PartialEq)]
pub struct StateInvalidationTLV {
    pub venue: VenueId,
    pub sequence: u64,
    pub instrument_count: u16,
    pub instruments: Vec<InstrumentId>,
    pub reason: InvalidationReason,
    pub timestamp_ns: u64,
}

/// Pool liquidity update TLV structure
///
/// Tracks only liquidity changes - fee rates come from PoolStateTLV
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct PoolLiquidityTLV {
    pub venue: VenueId,
    pub pool_address: [u8; 20], // Full pool contract address
    pub reserves: Vec<u128>, // Token reserves (native precision, no scaling) - u128 for DEX amounts
    pub timestamp_ns: u64,   // Nanoseconds since epoch
}

/// Pool swap event TLV structure
///
/// Records individual swaps with full token addresses for execution capability
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct PoolSwapTLV {
    pub venue: VenueId,
    pub pool_address: [u8; 20],   // Full pool contract address
    pub token_in_addr: [u8; 20],  // Full input token address (for execution)
    pub token_out_addr: [u8; 20], // Full output token address (for execution)
    pub amount_in: u128, // Amount in (native precision, no scaling) - u128 for blockchain amounts
    pub amount_out: u128, // Amount out (native precision, no scaling) - u128 for blockchain amounts
    pub amount_in_decimals: u8, // Decimals for amount_in (e.g., WMATIC=18)
    pub amount_out_decimals: u8, // Decimals for amount_out (e.g., USDC=6)
    // V3 state updates (0 for V2 pools):
    pub sqrt_price_x96_after: [u8; 20], // New sqrt price after swap (V3) - [u8; 20] for full uint160 precision
    pub tick_after: i32,                // New tick after swap (V3)
    pub liquidity_after: u128, // Active liquidity after swap (V3) - u128 for large liquidity
    pub timestamp_ns: u64,     // Nanoseconds since epoch
    pub block_number: u64,     // Block number of swap
}

impl PoolSwapTLV {
    /// Convert sqrt_price_x96_after from [u8; 20] to u128 for backward compatibility
    /// Note: This truncates to lower 128 bits for internal calculations while preserving full precision in TLV
    pub fn sqrt_price_x96_as_u128(&self) -> u128 {
        let mut u128_bytes = [0u8; 16];
        // Take the lower 16 bytes (128 bits) for calculations
        u128_bytes.copy_from_slice(&self.sqrt_price_x96_after[4..20]);
        u128::from_be_bytes(u128_bytes)
    }

    /// Create sqrt_price_x96_after from u128 value (for testing/backward compatibility)
    pub fn sqrt_price_from_u128(value: u128) -> [u8; 20] {
        let mut result = [0u8; 20];
        result[4..20].copy_from_slice(&value.to_be_bytes());
        result
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Venue (2 bytes)
        bytes.extend_from_slice(&(self.venue as u16).to_le_bytes());

        // Pool address (20 bytes)
        bytes.extend_from_slice(&self.pool_address);

        // Token addresses (20 bytes each)
        bytes.extend_from_slice(&self.token_in_addr);
        bytes.extend_from_slice(&self.token_out_addr);

        // Amounts (native precision) - now u128 (16 bytes each)
        bytes.extend_from_slice(&self.amount_in.to_le_bytes());
        bytes.extend_from_slice(&self.amount_out.to_le_bytes());

        // Token decimals
        bytes.push(self.amount_in_decimals);
        bytes.push(self.amount_out_decimals);

        // V3 state (0 for V2)
        bytes.extend_from_slice(&self.sqrt_price_x96_after); // 20 bytes for uint160 ([u8; 20])
        bytes.extend_from_slice(&self.tick_after.to_le_bytes());
        bytes.extend_from_slice(&self.liquidity_after.to_le_bytes()); // u128 (16 bytes)

        // Timestamps
        bytes.extend_from_slice(&self.timestamp_ns.to_le_bytes());
        bytes.extend_from_slice(&self.block_number.to_le_bytes());

        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        if data.len() < 152 {
            // 2 + 20 + 20 + 20 + 16 + 16 + 1 + 1 + 20 + 4 + 16 + 8 + 8 = 152
            return Err(format!("Invalid PoolSwapTLV size: {}", data.len()));
        }

        let mut offset = 0;

        // Venue
        let venue = VenueId::try_from(u16::from_le_bytes([data[0], data[1]]))
            .map_err(|_| "Invalid venue ID")?;
        offset += 2;

        // Pool address (20 bytes)
        let mut pool_address = [0u8; 20];
        pool_address.copy_from_slice(&data[offset..offset + 20]);
        offset += 20;

        // Token addresses (20 bytes each)
        let mut token_in_addr = [0u8; 20];
        token_in_addr.copy_from_slice(&data[offset..offset + 20]);
        offset += 20;

        let mut token_out_addr = [0u8; 20];
        token_out_addr.copy_from_slice(&data[offset..offset + 20]);
        offset += 20;

        // Amounts (native precision) - u128 (16 bytes each)
        let amount_in = u128::from_le_bytes(data[offset..offset + 16].try_into().unwrap());
        offset += 16;
        let amount_out = u128::from_le_bytes(data[offset..offset + 16].try_into().unwrap());
        offset += 16;

        // Token decimals
        let amount_in_decimals = data[offset];
        offset += 1;
        let amount_out_decimals = data[offset];
        offset += 1;

        // V3 state
        let mut sqrt_price_x96_after = [0u8; 20];
        sqrt_price_x96_after.copy_from_slice(&data[offset..offset + 20]);
        offset += 20;
        let tick_after = i32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
        offset += 4;
        let liquidity_after = u128::from_le_bytes(data[offset..offset + 16].try_into().unwrap());
        offset += 16;

        // Timestamps
        let timestamp_ns = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
        offset += 8;
        let block_number = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());

        Ok(Self {
            venue,
            pool_address,
            token_in_addr,
            token_out_addr,
            amount_in,
            amount_out,
            amount_in_decimals,
            amount_out_decimals,
            sqrt_price_x96_after,
            tick_after,
            liquidity_after,
            timestamp_ns,
            block_number,
        })
    }

    // Legacy TLV message methods removed - use Protocol V2 TLVMessageBuilder instead
}

/// Pool Sync event TLV structure (V2 pools)
///
/// V2 pools emit Sync events after every state change with complete reserves
#[derive(Debug, Clone, PartialEq)]
pub struct PoolSyncTLV {
    pub venue: VenueId,
    pub pool_address: [u8; 20], // Full pool contract address
    pub token0_addr: [u8; 20],  // Full token0 address
    pub token1_addr: [u8; 20],  // Full token1 address
    pub reserve0: u128, // Complete reserve0 (native precision) - u128 for blockchain amounts
    pub reserve1: u128, // Complete reserve1 (native precision) - u128 for blockchain amounts
    pub token0_decimals: u8, // Decimals for token0 (e.g., WMATIC=18)
    pub token1_decimals: u8, // Decimals for token1 (e.g., USDC=6)
    pub timestamp_ns: u64, // Nanoseconds since epoch
    pub block_number: u64, // Block number of sync
}

impl PoolSyncTLV {
    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Venue (2 bytes)
        bytes.extend_from_slice(&(self.venue as u16).to_le_bytes());

        // Pool and token addresses (20 bytes each)
        bytes.extend_from_slice(&self.pool_address);
        bytes.extend_from_slice(&self.token0_addr);
        bytes.extend_from_slice(&self.token1_addr);

        // Reserves (native precision)
        bytes.extend_from_slice(&self.reserve0.to_le_bytes());
        bytes.extend_from_slice(&self.reserve1.to_le_bytes());

        // Token decimals
        bytes.push(self.token0_decimals);
        bytes.push(self.token1_decimals);

        // Timestamps
        bytes.extend_from_slice(&self.timestamp_ns.to_le_bytes());
        bytes.extend_from_slice(&self.block_number.to_le_bytes());

        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        if data.len() < 110 {
            // 2 + 20*3 + 16*2 + 1*2 + 8*2 = 110
            return Err(format!("Invalid PoolSyncTLV size: {}", data.len()));
        }

        let mut offset = 0;

        // Venue
        let venue = VenueId::try_from(u16::from_le_bytes([data[0], data[1]]))
            .map_err(|_| "Invalid venue ID")?;
        offset += 2;

        // Pool and token addresses (20 bytes each)
        let mut pool_address = [0u8; 20];
        pool_address.copy_from_slice(&data[offset..offset + 20]);
        offset += 20;

        let mut token0_addr = [0u8; 20];
        token0_addr.copy_from_slice(&data[offset..offset + 20]);
        offset += 20;

        let mut token1_addr = [0u8; 20];
        token1_addr.copy_from_slice(&data[offset..offset + 20]);
        offset += 20;

        // Check remaining size for fixed fields (16 + 16 + 1 + 1 + 8 + 8 = 50 bytes)
        if data.len() < offset + 50 {
            return Err("Insufficient data for PoolSyncTLV fields".to_string());
        }

        // Reserves (native precision) - u128 (16 bytes each)
        let reserve0 = u128::from_le_bytes(data[offset..offset + 16].try_into().unwrap());
        offset += 16;
        let reserve1 = u128::from_le_bytes(data[offset..offset + 16].try_into().unwrap());
        offset += 16;

        // Token decimals
        let token0_decimals = data[offset];
        offset += 1;
        let token1_decimals = data[offset];
        offset += 1;

        // Timestamps
        let timestamp_ns = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
        offset += 8;
        let block_number = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());

        Ok(Self {
            venue,
            pool_address,
            token0_addr,
            token1_addr,
            reserve0,
            reserve1,
            token0_decimals,
            token1_decimals,
            timestamp_ns,
            block_number,
        })
    }

    // Legacy TLV message methods removed - use Protocol V2 TLVMessageBuilder instead
}

/// Pool Mint (liquidity add) event TLV structure
///
/// Records when liquidity providers add liquidity to a pool
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct PoolMintTLV {
    pub venue: VenueId,
    pub pool_address: [u8; 20],  // Full pool contract address
    pub provider_addr: [u8; 20], // Full LP provider address
    pub token0_addr: [u8; 20],   // Full token0 address
    pub token1_addr: [u8; 20],   // Full token1 address
    pub tick_lower: i32,         // Lower tick boundary (for concentrated liquidity)
    pub tick_upper: i32,         // Upper tick boundary
    pub liquidity_delta: u128,   // Liquidity added (native precision) - u128 for large liquidity
    pub amount0: u128, // Token0 deposited (native precision) - u128 for blockchain amounts
    pub amount1: u128, // Token1 deposited (native precision) - u128 for blockchain amounts
    pub token0_decimals: u8, // Decimals for token0 (e.g., WMATIC=18)
    pub token1_decimals: u8, // Decimals for token1 (e.g., USDC=6)
    pub timestamp_ns: u64, // Nanoseconds since epoch
}

/// Pool Burn (liquidity remove) event TLV structure
///
/// Records when liquidity providers remove liquidity from a pool
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct PoolBurnTLV {
    pub venue: VenueId,
    pub pool_address: [u8; 20],  // Full pool contract address
    pub provider_addr: [u8; 20], // Full LP provider address
    pub token0_addr: [u8; 20],   // Full token0 address
    pub token1_addr: [u8; 20],   // Full token1 address
    pub tick_lower: i32,         // Lower tick boundary
    pub tick_upper: i32,         // Upper tick boundary
    pub liquidity_delta: u128,   // Liquidity removed (native precision) - u128 for large liquidity
    pub amount0: u128, // Token0 withdrawn (native precision) - u128 for blockchain amounts
    pub amount1: u128, // Token1 withdrawn (native precision) - u128 for blockchain amounts
    pub token0_decimals: u8, // Decimals for token0 (e.g., WMATIC=18)
    pub token1_decimals: u8, // Decimals for token1 (e.g., USDC=6)
    pub timestamp_ns: u64, // Nanoseconds since epoch
}

/// Pool Tick crossing event TLV structure
///
/// Records when price crosses tick boundaries (important for concentrated liquidity)
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct PoolTickTLV {
    pub venue: VenueId,
    pub pool_address: [u8; 20], // Full pool contract address
    pub tick: i32,              // The tick that was crossed
    pub liquidity_net: i64,     // Net liquidity change at this tick
    pub price_sqrt: u64,        // Square root price (X96 format)
    pub timestamp_ns: u64,      // Nanoseconds since epoch
}

impl PoolMintTLV {
    /// Serialize to binary format
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Venue (2 bytes)
        bytes.extend_from_slice(&(self.venue as u16).to_le_bytes());

        // Pool address (20 bytes)
        bytes.extend_from_slice(&self.pool_address);

        // Token addresses (20 bytes each)
        bytes.extend_from_slice(&self.token0_addr);
        bytes.extend_from_slice(&self.token1_addr);

        // Provider address (20 bytes)
        bytes.extend_from_slice(&self.provider_addr);

        // Ticks (4 bytes each)
        bytes.extend_from_slice(&self.tick_lower.to_le_bytes());
        bytes.extend_from_slice(&self.tick_upper.to_le_bytes());

        // Amounts (native precision, 8 bytes each)
        bytes.extend_from_slice(&self.liquidity_delta.to_le_bytes());
        bytes.extend_from_slice(&self.amount0.to_le_bytes());
        bytes.extend_from_slice(&self.amount1.to_le_bytes());

        // Token decimals (1 byte each)
        bytes.push(self.token0_decimals);
        bytes.push(self.token1_decimals);

        // Timestamp (8 bytes)
        bytes.extend_from_slice(&self.timestamp_ns.to_le_bytes());

        bytes
    }

    /// Deserialize from binary format
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        let mut offset = 0;

        // Venue (2 bytes)
        if data.len() < 2 {
            return Err("Insufficient data for venue".to_string());
        }
        let venue = VenueId::try_from(u16::from_le_bytes(data[0..2].try_into().unwrap()))
            .map_err(|_| "Invalid venue ID")?;
        offset += 2;

        // Pool address (20 bytes)
        if offset + 20 > data.len() {
            return Err("Insufficient data for pool address".to_string());
        }
        let mut pool_address = [0u8; 20];
        pool_address.copy_from_slice(&data[offset..offset + 20]);
        offset += 20;

        // Token addresses (20 bytes each)
        if offset + 40 > data.len() {
            return Err("Insufficient data for token addresses".to_string());
        }
        let mut token0_addr = [0u8; 20];
        token0_addr.copy_from_slice(&data[offset..offset + 20]);
        offset += 20;

        let mut token1_addr = [0u8; 20];
        token1_addr.copy_from_slice(&data[offset..offset + 20]);
        offset += 20;

        // Provider address (20 bytes)
        if offset + 20 > data.len() {
            return Err("Insufficient data for provider address".to_string());
        }
        let mut provider_addr = [0u8; 20];
        provider_addr.copy_from_slice(&data[offset..offset + 20]);
        offset += 20;

        // Fixed fields (4 + 4 + 16 + 16 + 16 + 1 + 1 + 8 = 66 bytes)
        if offset + 66 > data.len() {
            return Err("Insufficient data for mint fields".to_string());
        }

        let tick_lower = i32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
        offset += 4;

        let tick_upper = i32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
        offset += 4;

        let liquidity_delta = u128::from_le_bytes(data[offset..offset + 16].try_into().unwrap());
        offset += 16;

        let amount0 = u128::from_le_bytes(data[offset..offset + 16].try_into().unwrap());
        offset += 16;

        let amount1 = u128::from_le_bytes(data[offset..offset + 16].try_into().unwrap());
        offset += 16;

        // Token decimals (1 byte each)
        let token0_decimals = data[offset];
        offset += 1;
        let token1_decimals = data[offset];
        offset += 1;

        let timestamp_ns = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());

        Ok(PoolMintTLV {
            venue,
            pool_address,
            provider_addr,
            token0_addr,
            token1_addr,
            tick_lower,
            tick_upper,
            liquidity_delta,
            amount0,
            amount1,
            token0_decimals,
            token1_decimals,
            timestamp_ns,
        })
    }

    // Legacy to_tlv_message removed - use Protocol V2 TLVMessageBuilder instead
}

impl PoolBurnTLV {
    /// Serialize to binary format
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Venue (2 bytes)
        bytes.extend_from_slice(&(self.venue as u16).to_le_bytes());

        // Pool address (20 bytes)
        bytes.extend_from_slice(&self.pool_address);

        // Token addresses (20 bytes each)
        bytes.extend_from_slice(&self.token0_addr);
        bytes.extend_from_slice(&self.token1_addr);

        // Provider address (20 bytes)
        bytes.extend_from_slice(&self.provider_addr);

        // Ticks (4 bytes each)
        bytes.extend_from_slice(&self.tick_lower.to_le_bytes());
        bytes.extend_from_slice(&self.tick_upper.to_le_bytes());

        // Amounts (native precision, 8 bytes each)
        bytes.extend_from_slice(&self.liquidity_delta.to_le_bytes());
        bytes.extend_from_slice(&self.amount0.to_le_bytes());
        bytes.extend_from_slice(&self.amount1.to_le_bytes());

        // Token decimals (1 byte each)
        bytes.push(self.token0_decimals);
        bytes.push(self.token1_decimals);

        // Timestamp (8 bytes)
        bytes.extend_from_slice(&self.timestamp_ns.to_le_bytes());

        bytes
    }

    /// Deserialize from binary format
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        // Same parsing logic as PoolMintTLV
        let mint = PoolMintTLV::from_bytes(data)?;

        Ok(PoolBurnTLV {
            venue: mint.venue,
            pool_address: mint.pool_address,
            provider_addr: mint.provider_addr,
            token0_addr: mint.token0_addr,
            token1_addr: mint.token1_addr,
            tick_lower: mint.tick_lower,
            tick_upper: mint.tick_upper,
            liquidity_delta: mint.liquidity_delta,
            amount0: mint.amount0,
            amount1: mint.amount1,
            token0_decimals: mint.token0_decimals,
            token1_decimals: mint.token1_decimals,
            timestamp_ns: mint.timestamp_ns,
        })
    }

    // Legacy to_tlv_message removed - use Protocol V2 TLVMessageBuilder instead
}

impl PoolTickTLV {
    /// Serialize to binary format
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Venue (2 bytes)
        bytes.extend_from_slice(&(self.venue as u16).to_le_bytes());

        // Pool address (20 bytes)
        bytes.extend_from_slice(&self.pool_address);

        // Tick (4 bytes)
        bytes.extend_from_slice(&self.tick.to_le_bytes());

        // Liquidity net (8 bytes)
        bytes.extend_from_slice(&self.liquidity_net.to_le_bytes());

        // Square root price (8 bytes)
        bytes.extend_from_slice(&self.price_sqrt.to_le_bytes());

        // Timestamp (8 bytes)
        bytes.extend_from_slice(&self.timestamp_ns.to_le_bytes());

        bytes
    }

    /// Deserialize from binary format
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        let mut offset = 0;

        // Venue (2 bytes)
        if data.len() < 2 {
            return Err("Insufficient data for venue".to_string());
        }
        let venue = VenueId::try_from(u16::from_le_bytes(data[0..2].try_into().unwrap()))
            .map_err(|_| "Invalid venue ID")?;
        offset += 2;

        // Pool address (20 bytes)
        if offset + 20 > data.len() {
            return Err("Insufficient data for pool address".to_string());
        }
        let mut pool_address = [0u8; 20];
        pool_address.copy_from_slice(&data[offset..offset + 20]);
        offset += 20;

        // Fixed fields (4 + 8 + 8 + 8 = 28 bytes)
        if offset + 28 > data.len() {
            return Err("Insufficient data for tick fields".to_string());
        }

        let tick = i32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
        offset += 4;

        let liquidity_net = i64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
        offset += 8;

        let price_sqrt = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
        offset += 8;

        let timestamp_ns = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());

        Ok(PoolTickTLV {
            venue,
            pool_address,
            tick,
            liquidity_net,
            price_sqrt,
            timestamp_ns,
        })
    }

    // Legacy to_tlv_message removed - use Protocol V2 TLVMessageBuilder instead
}

/// Reasons for state invalidation
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidationReason {
    Disconnection = 0,
    AuthenticationFailure = 1,
    RateLimited = 2,
    Staleness = 3,
    Maintenance = 4,
    Recovery = 5,
}

impl PoolLiquidityTLV {
    /// Serialize to binary format
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Venue (2 bytes)
        bytes.extend_from_slice(&(self.venue as u16).to_le_bytes());

        // Pool address (20 bytes)
        bytes.extend_from_slice(&self.pool_address);

        // Reserves count and values
        bytes.extend_from_slice(&(self.reserves.len() as u8).to_le_bytes());
        for reserve in &self.reserves {
            bytes.extend_from_slice(&reserve.to_le_bytes());
        }

        // Timestamp only (removed total_supply and fee_rate)
        bytes.extend_from_slice(&self.timestamp_ns.to_le_bytes());

        bytes
    }

    /// Deserialize from binary format
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        if data.len() < 4 {
            return Err(format!(
                "Invalid payload size: need at least 4 bytes, got {}",
                data.len()
            ));
        }

        let mut offset = 0;

        // Venue (2 bytes)
        let venue = VenueId::try_from(u16::from_le_bytes(
            data[offset..offset + 2].try_into().unwrap(),
        ))
        .map_err(|_| "Invalid venue ID")?;
        offset += 2;

        // Pool address (20 bytes)
        if offset + 20 > data.len() {
            return Err("Insufficient data for pool address".to_string());
        }
        let mut pool_address = [0u8; 20];
        pool_address.copy_from_slice(&data[offset..offset + 20]);
        offset += 20;

        // Reserves count (1 byte)
        if offset + 1 > data.len() {
            return Err("Insufficient data for reserves count".to_string());
        }
        let reserves_count = data[offset] as usize;
        offset += 1;

        // Reserves (16 bytes each for u128)
        if offset + reserves_count * 16 > data.len() {
            return Err(format!(
                "Insufficient data for reserves: need {} bytes",
                reserves_count * 16
            ));
        }
        let mut reserves = Vec::with_capacity(reserves_count);
        for _ in 0..reserves_count {
            let reserve = u128::from_le_bytes(data[offset..offset + 16].try_into().unwrap());
            reserves.push(reserve);
            offset += 16;
        }

        // Timestamp only (8 bytes) - removed total_supply and fee_rate
        if offset + 8 != data.len() {
            return Err(format!(
                "Invalid remaining data size: expected 8 bytes, got {}",
                data.len() - offset
            ));
        }

        let timestamp_ns = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());

        Ok(PoolLiquidityTLV {
            venue,
            pool_address,
            reserves,
            timestamp_ns,
        })
    }

    // Legacy to_tlv_message removed - use Protocol V2 TLVMessageBuilder instead
}

impl StateInvalidationTLV {
    /// Serialize to bytes for TLV encoding
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Venue (2 bytes)
        bytes.extend_from_slice(&(self.venue as u16).to_be_bytes());

        // Sequence (8 bytes)
        bytes.extend_from_slice(&self.sequence.to_be_bytes());

        // Instrument count (2 bytes)
        bytes.extend_from_slice(&self.instrument_count.to_be_bytes());

        // Instruments (variable length)
        for instrument in &self.instruments {
            bytes.extend_from_slice(&instrument.venue.to_be_bytes());
            bytes.push(instrument.asset_type);
            bytes.push(instrument.reserved);
            bytes.extend_from_slice(&instrument.asset_id.to_be_bytes());
        }

        // Reason (1 byte)
        bytes.push(self.reason as u8);

        // Timestamp (8 bytes)
        bytes.extend_from_slice(&self.timestamp_ns.to_be_bytes());

        bytes
    }
}

impl TryFrom<u8> for InvalidationReason {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(InvalidationReason::Disconnection),
            1 => Ok(InvalidationReason::AuthenticationFailure),
            2 => Ok(InvalidationReason::RateLimited),
            3 => Ok(InvalidationReason::Staleness),
            4 => Ok(InvalidationReason::Maintenance),
            5 => Ok(InvalidationReason::Recovery),
            _ => Err(format!("Unknown invalidation reason: {}", value)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trade_tlv_roundtrip() {
        let trade = TradeTLV::new(
            VenueId::Binance,
            InstrumentId::from_u64(0x12345678),
            4512350000000, // $45,123.50
            12345678,      // 0.12345678
            0,             // buy
            1700000000000000000,
        );

        let bytes = trade.as_bytes().to_vec();
        let recovered = TradeTLV::from_bytes(&bytes).unwrap();

        assert_eq!(trade, recovered);
    }

    #[test]
    fn test_trade_tlv_message_roundtrip() {
        let trade = TradeTLV::new(
            VenueId::Binance,
            InstrumentId::from_u64(0x12345678),
            4512350000000,
            12345678,
            0,
            1700000000000000000,
        );

        // Legacy TLV message test removed - use Protocol V2 TLVMessageBuilder for testing
        let recovered = TradeTLV::from_bytes(&trade.as_bytes().to_vec()).unwrap();
        assert_eq!(trade, recovered);
    }

    #[test]
    fn test_quote_tlv_roundtrip() {
        // Create a proper InstrumentId using a symbol that fits in 40 bits
        let btc_usd_id = InstrumentId::stock(VenueId::Kraken, "BTCUSD");

        let quote = QuoteTLV::new(
            VenueId::Kraken,
            btc_usd_id,
            4512350000000, // Bid price $45,123.50
            50000000,      // Bid size 0.50000000
            4512450000000, // Ask price $45,124.50
            25000000,      // Ask size 0.25000000
            1700000000000000000,
        );

        let bytes = quote.as_bytes().to_vec();
        let recovered = QuoteTLV::from_bytes(&bytes).unwrap();

        assert_eq!(quote, recovered);
        assert_eq!(quote.venue().unwrap(), VenueId::Kraken);
        let instrument = quote.instrument_id();
        let expected_instrument = btc_usd_id;
        assert_eq!(instrument, expected_instrument);
    }

    #[test]
    fn test_quote_tlv_message_roundtrip() {
        // Create a proper InstrumentId using a symbol
        let eth_usd_id = InstrumentId::stock(VenueId::Kraken, "ETHUSD");

        let quote = QuoteTLV::new(
            VenueId::Kraken,
            eth_usd_id,
            350025000000, // Bid price $3,500.25
            100000000,    // Bid size 1.00000000
            350050000000, // Ask price $3,500.50
            75000000,     // Ask size 0.75000000
            1700000000000000000,
        );

        // Legacy TLV message test removed - use Protocol V2 TLVMessageBuilder for testing
        let recovered = QuoteTLV::from_bytes(&quote.as_bytes().to_vec()).unwrap();
        assert_eq!(quote, recovered);
    }

    #[test]
    fn test_quote_tlv_size() {
        // Verify QuoteTLV has the expected size
        use std::mem::size_of;
        assert_eq!(size_of::<QuoteTLV>(), 52);
    }
}

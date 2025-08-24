//! Market Data TLV Structures
//!
//! Defines concrete TLV structures for market data messages

use crate::{InstrumentId, VenueId}; // TLVType removed with legacy TLV system
                                    // Legacy TLV types removed - using Protocol V2 MessageHeader + TLV extensions
use zerocopy::{AsBytes, FromBytes, FromZeroes};

/// Trade TLV structure - simplified for serialization
///
/// Fields are ordered to eliminate padding: u64/i64 → u16 → u8
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, AsBytes, FromBytes, FromZeroes)]
pub struct TradeTLV {
    // Group 64-bit fields first for natural alignment
    pub asset_id: u64,     // Asset identifier
    pub price: i64,        // Fixed-point with 8 decimals
    pub volume: i64,       // Fixed-point with 8 decimals
    pub timestamp_ns: u64, // Nanoseconds since epoch

    // Then 16-bit field
    pub venue_id: u16, // VenueId as primitive

    // Finally 8-bit fields (need 6 bytes total to reach 40 bytes)
    pub asset_type: u8,    // AssetType as primitive
    pub reserved: u8,      // Reserved byte for alignment
    pub side: u8,          // 0 = buy, 1 = sell
    pub _padding: [u8; 3], // Padding to reach 40 bytes (multiple of 8)
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
            // 64-bit fields
            asset_id: instrument_id.asset_id,
            price,
            volume,
            timestamp_ns,
            // 16-bit field
            venue_id: venue as u16,
            // 8-bit fields
            asset_type: instrument_id.asset_type,
            reserved: instrument_id.reserved,
            side,
            _padding: [0; 3],
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
///
/// Padded to 56 bytes for 8-byte alignment
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, AsBytes, FromBytes, FromZeroes)]
pub struct QuoteTLV {
    // Group 64-bit fields first for natural alignment
    pub asset_id: u64,     // Asset identifier
    pub bid_price: i64,    // Fixed-point with 8 decimals
    pub bid_size: i64,     // Fixed-point with 8 decimals
    pub ask_price: i64,    // Fixed-point with 8 decimals
    pub ask_size: i64,     // Fixed-point with 8 decimals
    pub timestamp_ns: u64, // Nanoseconds since epoch

    // Then 16-bit field
    pub venue_id: u16, // VenueId as primitive

    // Finally 8-bit fields
    pub asset_type: u8, // AssetType as primitive
    pub reserved: u8,   // Reserved byte for alignment

    // EXPLICIT PADDING - DO NOT DELETE!
    pub _padding: [u8; 4], // Required for 8-byte alignment to 56 bytes
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
            _padding: [0; 4],
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

/// State invalidation TLV structure - Zero-copy with fixed-size array
///
/// Supports up to 16 instruments per invalidation (more than sufficient for real-world usage)
/// Most invalidations affect 1-5 instruments, with 16 providing generous headroom.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, AsBytes, FromBytes, FromZeroes)]
pub struct StateInvalidationTLV {
    // Group 64-bit fields first (16 bytes)
    pub sequence: u64,     // Sequence number
    pub timestamp_ns: u64, // Nanoseconds since epoch

    // Fixed-size array for instruments (16 * 8 = 128 bytes)
    pub instruments: [InstrumentId; 16], // Fixed array - unused slots are zeroed

    // Then smaller fields (8 bytes total)
    pub venue: u16,            // VenueId as u16 (2 bytes)
    pub instrument_count: u16, // Actual count (0-16)
    pub reason: u8,            // InvalidationReason as u8 (1 byte)
    pub _padding: [u8; 3],     // Explicit padding to align to 16-byte boundary

                               // Total: 16 + 128 + 8 = 152 bytes (aligned)
}

/// Pool liquidity update TLV structure - Zero-copy with fixed-size array
///
/// Tracks only liquidity changes - fee rates come from PoolStateTLV
/// Supports up to 8 token reserves (sufficient for even complex Balancer/Curve pools)
/// Most pools have 2 tokens, with 8 providing generous headroom.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoolLiquidityTLV {
    // Group 64-bit fields first (8 bytes)
    pub timestamp_ns: u64, // Nanoseconds since epoch

    // Fixed-size array for reserves (8 * 16 = 128 bytes)
    pub reserves: [u128; 8], // Token reserves (native precision) - unused slots are zero

    // Pool address (32 bytes - padded from 20-byte Ethereum address)
    pub pool_address: [u8; 32], // Full pool contract address (20 bytes + 12 padding)

    // Then smaller fields (8 bytes total to align properly)
    pub venue: u16,        // VenueId as u16 (2 bytes)
    pub reserve_count: u8, // Actual number of reserves (1-8)
    pub _padding: [u8; 5], // Explicit padding to reach 8 bytes

                           // Total: 8 + 128 + 32 + 8 = 176 bytes (all u64/u128 aligned)
}

/// Pool swap event TLV structure
///
/// Records individual swaps with full token addresses for execution capability
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, AsBytes, FromBytes, FromZeroes)]
pub struct PoolSwapTLV {
    // u128 fields (16-byte aligned) - 48 bytes
    pub amount_in: u128,       // Amount in (native precision, no scaling)
    pub amount_out: u128,      // Amount out (native precision, no scaling)
    pub liquidity_after: u128, // Active liquidity after swap (V3)

    // u64 fields (8-byte aligned) - 16 bytes
    pub timestamp_ns: u64, // Nanoseconds since epoch
    pub block_number: u64, // Block number of swap

    // i32 fields (4-byte aligned) - 4 bytes
    pub tick_after: i32, // New tick after swap (V3)

    // u16 fields (2-byte aligned) - 2 bytes
    pub venue: u16, // NOT VenueId enum! Direct u16 for zero-copy

    // u8 fields (1-byte aligned) - 2 bytes
    pub amount_in_decimals: u8, // Decimals for amount_in (e.g., WMATIC=18)
    pub amount_out_decimals: u8, // Decimals for amount_out (e.g., USDC=6)

    // [u8; 32] arrays - 128 bytes
    pub pool_address: [u8; 32], // Full pool contract address (first 20 bytes = address, last 12 = padding)
    pub token_in_addr: [u8; 32], // Full input token address (first 20 bytes = address, last 12 = padding)
    pub token_out_addr: [u8; 32], // Full output token address (first 20 bytes = address, last 12 = padding)
    pub sqrt_price_x96_after: [u8; 32], // New sqrt price after swap (V3) - padded for alignment

    // EXPLICIT PADDING - DO NOT DELETE!
    pub _padding: [u8; 8], // Required for alignment to 208 bytes
                           // Total: 208 bytes (13 × 16) ✅
}

impl PoolSwapTLV {
    /// Create a new PoolSwapTLV from components
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        pool: [u8; 20],
        token_in: [u8; 20],
        token_out: [u8; 20],
        venue_id: VenueId,
        amount_in: u128,
        amount_out: u128,
        liquidity_after: u128,
        timestamp_ns: u64,
        block_number: u64,
        tick_after: i32,
        amount_in_decimals: u8,
        amount_out_decimals: u8,
        sqrt_price_x96_after: u128,
    ) -> Self {
        use super::AddressConversion;

        Self {
            pool_address: pool.to_padded(),
            token_in_addr: token_in.to_padded(),
            token_out_addr: token_out.to_padded(),
            venue: venue_id as u16,
            amount_in,
            amount_out,
            liquidity_after,
            timestamp_ns,
            block_number,
            tick_after,
            amount_in_decimals,
            amount_out_decimals,
            sqrt_price_x96_after: Self::sqrt_price_from_u128(sqrt_price_x96_after),
            _padding: [0u8; 8], // Always initialize to zeros
        }
    }

    /// Get the pool address as a 20-byte array
    #[inline(always)]
    pub fn pool_address_eth(&self) -> [u8; 20] {
        use super::AddressExtraction;
        self.pool_address.to_eth_address()
    }

    /// Get the token_in address as a 20-byte array
    #[inline(always)]
    pub fn token_in_addr_eth(&self) -> [u8; 20] {
        use super::AddressExtraction;
        self.token_in_addr.to_eth_address()
    }

    /// Get the token_out address as a 20-byte array
    #[inline(always)]
    pub fn token_out_addr_eth(&self) -> [u8; 20] {
        use super::AddressExtraction;
        self.token_out_addr.to_eth_address()
    }

    /// Convert sqrt_price_x96_after from [u8; 32] to u128 for backward compatibility
    /// Note: This truncates to lower 128 bits for internal calculations while preserving full precision in TLV
    pub fn sqrt_price_x96_as_u128(&self) -> u128 {
        let mut u128_bytes = [0u8; 16];
        // Take the first 16 bytes (128 bits) for calculations
        u128_bytes.copy_from_slice(&self.sqrt_price_x96_after[..16]);
        u128::from_le_bytes(u128_bytes)
    }

    /// Create sqrt_price_x96_after from u128 value (for testing/backward compatibility)
    pub fn sqrt_price_from_u128(value: u128) -> [u8; 32] {
        let mut result = [0u8; 32];
        result[..16].copy_from_slice(&value.to_le_bytes());
        result
    }

    // Manual serialization methods removed - use zero-copy AsBytes trait:
    // let bytes = swap.as_bytes(); // Zero-copy serialization!
    // let swap_ref = PoolSwapTLV::ref_from(bytes)?; // Zero-copy deserialization!

    // Legacy TLV message methods removed - use Protocol V2 TLVMessageBuilder instead
}

/// Pool Sync event TLV structure (V2 pools)
///
/// V2 pools emit Sync events after every state change with complete reserves
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, AsBytes, FromBytes, FromZeroes)]
pub struct PoolSyncTLV {
    // u128 fields first (16-byte aligned) - 32 bytes
    pub reserve0: u128, // Complete reserve0 (native precision)
    pub reserve1: u128, // Complete reserve1 (native precision)

    // u64 fields (8-byte aligned) - 16 bytes
    pub timestamp_ns: u64, // Nanoseconds since epoch
    pub block_number: u64, // Block number of sync

    // u16 fields (2-byte aligned) - 2 bytes
    pub venue: u16, // NOT VenueId enum! Direct u16 for zero-copy

    // u8 fields (1-byte aligned) - 2 bytes
    pub token0_decimals: u8, // Decimals for token0 (e.g., WMATIC=18)
    pub token1_decimals: u8, // Decimals for token1 (e.g., USDC=6)

    // [u8; 32] arrays - 96 bytes
    pub pool_address: [u8; 32], // Full pool contract address (first 20 bytes = address, last 12 = padding)
    pub token0_addr: [u8; 32],  // Full token0 address (first 20 bytes = address, last 12 = padding)
    pub token1_addr: [u8; 32],  // Full token1 address (first 20 bytes = address, last 12 = padding)

    // EXPLICIT PADDING - DO NOT DELETE!
    pub _padding: [u8; 12], // Required for alignment to 160 bytes
                            // Total: 160 bytes (10 × 16) ✅
}

impl PoolSyncTLV {
    /// Create a new PoolSyncTLV from components
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        pool: [u8; 20],
        token0: [u8; 20],
        token1: [u8; 20],
        venue_id: VenueId,
        reserve0: u128,
        reserve1: u128,
        token0_decimals: u8,
        token1_decimals: u8,
        timestamp_ns: u64,
        block_number: u64,
    ) -> Self {
        use super::AddressConversion;

        Self {
            pool_address: pool.to_padded(),
            token0_addr: token0.to_padded(),
            token1_addr: token1.to_padded(),
            venue: venue_id as u16,
            reserve0,
            reserve1,
            token0_decimals,
            token1_decimals,
            timestamp_ns,
            block_number,
            _padding: [0u8; 12], // Always initialize to zeros
        }
    }

    /// Get the venue as VenueId enum
    #[inline(always)]
    pub fn venue_id(&self) -> Result<VenueId, crate::ProtocolError> {
        VenueId::try_from(self.venue).map_err(|_| crate::ProtocolError::InvalidInstrument)
    }

    // Manual serialization methods removed - use zero-copy AsBytes trait:
    // let bytes = sync.as_bytes(); // Zero-copy serialization!
    // let sync_ref = PoolSyncTLV::ref_from(bytes)?; // Zero-copy deserialization!

    // Legacy TLV message methods removed - use Protocol V2 TLVMessageBuilder instead
}

/// Pool Mint (liquidity add) event TLV structure
///
/// Records when liquidity providers add liquidity to a pool
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, AsBytes, FromBytes, FromZeroes)]
pub struct PoolMintTLV {
    // u128 fields first (16-byte aligned) - 48 bytes
    pub liquidity_delta: u128, // Liquidity added (native precision)
    pub amount0: u128,         // Token0 deposited (native precision)
    pub amount1: u128,         // Token1 deposited (native precision)

    // u64 fields (8-byte aligned) - 8 bytes
    pub timestamp_ns: u64, // Nanoseconds since epoch

    // i32 fields (4-byte aligned) - 8 bytes
    pub tick_lower: i32, // Lower tick boundary (for concentrated liquidity)
    pub tick_upper: i32, // Upper tick boundary

    // u16 fields (2-byte aligned) - 2 bytes
    pub venue: u16, // NOT VenueId enum! Direct u16 for zero-copy

    // u8 fields (1-byte aligned) - 2 bytes
    pub token0_decimals: u8, // Decimals for token0 (e.g., WMATIC=18)
    pub token1_decimals: u8, // Decimals for token1 (e.g., USDC=6)

    // [u8; 32] arrays - 128 bytes
    pub pool_address: [u8; 32], // Full pool contract address (first 20 bytes = address, last 12 = padding)
    pub provider_addr: [u8; 32], // Full LP provider address (first 20 bytes = address, last 12 = padding)
    pub token0_addr: [u8; 32], // Full token0 address (first 20 bytes = address, last 12 = padding)
    pub token1_addr: [u8; 32], // Full token1 address (first 20 bytes = address, last 12 = padding)

    // EXPLICIT PADDING - DO NOT DELETE!
    pub _padding: [u8; 12], // Required for alignment to 208 bytes
                            // Total: 208 bytes (13 × 16) ✅
}

/// Pool Burn (liquidity remove) event TLV structure
///
/// Records when liquidity providers remove liquidity from a pool
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, AsBytes, FromBytes, FromZeroes)]
pub struct PoolBurnTLV {
    // u128 fields first (16-byte aligned) - 48 bytes
    pub liquidity_delta: u128, // Liquidity removed (native precision)
    pub amount0: u128,         // Token0 withdrawn (native precision)
    pub amount1: u128,         // Token1 withdrawn (native precision)

    // u64 fields (8-byte aligned) - 8 bytes
    pub timestamp_ns: u64, // Nanoseconds since epoch

    // i32 fields (4-byte aligned) - 8 bytes
    pub tick_lower: i32, // Lower tick boundary
    pub tick_upper: i32, // Upper tick boundary

    // u16 fields (2-byte aligned) - 2 bytes
    pub venue: u16, // NOT VenueId enum! Direct u16 for zero-copy

    // u8 fields (1-byte aligned) - 2 bytes
    pub token0_decimals: u8, // Decimals for token0 (e.g., WMATIC=18)
    pub token1_decimals: u8, // Decimals for token1 (e.g., USDC=6)

    // [u8; 32] arrays - 128 bytes
    pub pool_address: [u8; 32], // Full pool contract address (first 20 bytes = address, last 12 = padding)
    pub provider_addr: [u8; 32], // Full LP provider address (first 20 bytes = address, last 12 = padding)
    pub token0_addr: [u8; 32], // Full token0 address (first 20 bytes = address, last 12 = padding)
    pub token1_addr: [u8; 32], // Full token1 address (first 20 bytes = address, last 12 = padding)

    // EXPLICIT PADDING - DO NOT DELETE!
    pub _padding: [u8; 12], // Required for alignment to 208 bytes
                            // Total: 208 bytes (13 × 16) ✅
}

/// Pool Tick crossing event TLV structure
///
/// Records when price crosses tick boundaries (important for concentrated liquidity)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, AsBytes, FromBytes, FromZeroes)]
pub struct PoolTickTLV {
    // u64/i64 fields first (8-byte aligned) - 24 bytes
    pub liquidity_net: i64, // Net liquidity change at this tick
    pub price_sqrt: u64,    // Square root price (X96 format)
    pub timestamp_ns: u64,  // Nanoseconds since epoch

    // i32 fields (4-byte aligned) - 4 bytes
    pub tick: i32, // The tick that was crossed

    // u16 fields (2-byte aligned) - 2 bytes
    pub venue: u16, // NOT VenueId enum! Direct u16 for zero-copy

    // [u8; 32] arrays - 32 bytes
    pub pool_address: [u8; 32], // Full pool contract address (first 20 bytes = address, last 12 = padding)

    // EXPLICIT PADDING - DO NOT DELETE!
    pub _padding: [u8; 2], // Required for alignment to 64 bytes
                           // Total: 64 bytes (4 × 16) ✅
}

impl PoolMintTLV {
    /// Create a new PoolMintTLV from components
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        pool: [u8; 20],
        provider: [u8; 20],
        token0: [u8; 20],
        token1: [u8; 20],
        venue_id: VenueId,
        liquidity_delta: u128,
        amount0: u128,
        amount1: u128,
        tick_lower: i32,
        tick_upper: i32,
        token0_decimals: u8,
        token1_decimals: u8,
        timestamp_ns: u64,
    ) -> Self {
        use super::AddressConversion;

        Self {
            pool_address: pool.to_padded(),
            provider_addr: provider.to_padded(),
            token0_addr: token0.to_padded(),
            token1_addr: token1.to_padded(),
            venue: venue_id as u16,
            liquidity_delta,
            amount0,
            amount1,
            tick_lower,
            tick_upper,
            token0_decimals,
            token1_decimals,
            timestamp_ns,
            _padding: [0u8; 12], // Always initialize to zeros
        }
    }

    /// DEPRECATED: Serialize to binary format
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Venue (2 bytes)
        bytes.extend_from_slice(&self.venue.to_le_bytes());

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
    /// Parse from bytes using zero-copy
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        if data.len() < std::mem::size_of::<Self>() {
            return Err("Data too short for PoolMintTLV".to_string());
        }

        use zerocopy::Ref;
        let tlv_ref = Ref::<_, Self>::new(data).ok_or("Failed to parse PoolMintTLV from bytes")?;
        Ok(*tlv_ref.into_ref())
    }

    // Legacy to_tlv_message removed - use Protocol V2 TLVMessageBuilder instead
}

impl PoolBurnTLV {
    /// Create a new PoolBurnTLV from components
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        pool: [u8; 20],
        provider: [u8; 20],
        token0: [u8; 20],
        token1: [u8; 20],
        venue_id: VenueId,
        liquidity_delta: u128,
        amount0: u128,
        amount1: u128,
        tick_lower: i32,
        tick_upper: i32,
        token0_decimals: u8,
        token1_decimals: u8,
        timestamp_ns: u64,
    ) -> Self {
        use super::address::AddressConversion;

        Self {
            pool_address: pool.to_padded(),
            provider_addr: provider.to_padded(),
            token0_addr: token0.to_padded(),
            token1_addr: token1.to_padded(),
            venue: venue_id as u16,
            liquidity_delta,
            amount0,
            amount1,
            tick_lower,
            tick_upper,
            token0_decimals,
            token1_decimals,
            timestamp_ns,
            _padding: [0u8; 12],
        }
    }

    /// Serialize to binary format
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Venue (2 bytes)
        bytes.extend_from_slice(&self.venue.to_le_bytes());

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

    /// Parse from bytes using zero-copy
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        if data.len() < std::mem::size_of::<Self>() {
            return Err("Data too short for PoolBurnTLV".to_string());
        }

        use zerocopy::Ref;
        let tlv_ref = Ref::<_, Self>::new(data).ok_or("Failed to parse PoolBurnTLV from bytes")?;
        Ok(*tlv_ref.into_ref())
    }

    // Legacy to_tlv_message removed - use Protocol V2 TLVMessageBuilder instead
}

impl PoolTickTLV {
    /// Create a new PoolTickTLV from components
    pub fn new(
        pool: [u8; 20],
        venue_id: VenueId,
        tick: i32,
        liquidity_net: i64,
        price_sqrt: u64,
        timestamp_ns: u64,
    ) -> Self {
        use super::address::AddressConversion;

        Self {
            pool_address: pool.to_padded(),
            venue: venue_id as u16,
            tick,
            liquidity_net,
            price_sqrt,
            timestamp_ns,
            _padding: [0u8; 2],
        }
    }

    /// Serialize to binary format
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Venue (2 bytes)
        bytes.extend_from_slice(&self.venue.to_le_bytes());

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

    /// Parse from bytes using zero-copy
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        if data.len() < std::mem::size_of::<Self>() {
            return Err("Data too short for PoolTickTLV".to_string());
        }

        use zerocopy::Ref;
        let tlv_ref = Ref::<_, Self>::new(data).ok_or("Failed to parse PoolTickTLV from bytes")?;
        Ok(*tlv_ref.into_ref())
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
    /// Create new liquidity update with dynamic reserves array
    pub fn new(
        venue: VenueId,
        pool_address: [u8; 20], // Input as 20-byte address
        reserves: &[u128],      // Pass slice of reserves
        timestamp_ns: u64,
    ) -> Result<Self, String> {
        if reserves.len() > 8 {
            return Err(format!("Too many reserves: {} (max 8)", reserves.len()));
        }

        if reserves.is_empty() {
            return Err("Reserves cannot be empty".to_string());
        }

        let mut fixed_reserves = [0u128; 8];
        // Copy actual reserves to fixed array
        for (i, reserve) in reserves.iter().enumerate() {
            fixed_reserves[i] = *reserve;
        }

        // Convert 20-byte address to 32-byte padded format
        use super::address::AddressConversion;
        let pool_address_32 = pool_address.to_padded();

        Ok(Self {
            timestamp_ns,
            reserves: fixed_reserves,
            pool_address: pool_address_32,
            venue: venue as u16,
            reserve_count: reserves.len() as u8,
            _padding: [0u8; 5],
        })
    }

    /// Get slice of actual reserves (excluding unused slots)
    pub fn get_reserves(&self) -> &[u128] {
        &self.reserves[..self.reserve_count as usize]
    }

    /// Get the 20-byte pool address (extracting from 32-byte padded format)
    pub fn get_pool_address(&self) -> [u8; 20] {
        use super::address::AddressExtraction;
        self.pool_address.to_eth_address()
    }

    /// Convert valid reserves to Vec (perfect bijection preservation)
    ///
    /// This method enables perfect bijection: Vec<u128> → PoolLiquidityTLV → Vec<u128>
    /// where the output Vec is identical to the original input Vec.
    pub fn to_reserves_vec(&self) -> Vec<u128> {
        self.get_reserves().to_vec()
    }

    /// Create from Vec with bijection validation (convenience method)
    ///
    /// Equivalent to new() but takes Vec directly for cleaner API.
    /// Validates perfect roundtrip: original_vec == tlv.to_reserves_vec()
    pub fn from_reserves_vec(
        venue: VenueId,
        pool_address: [u8; 20],
        reserves: Vec<u128>,
        timestamp_ns: u64,
    ) -> Result<Self, String> {
        Self::new(venue, pool_address, &reserves, timestamp_ns)
    }

    /// Add reserve to the liquidity update (if space available)
    pub fn add_reserve(&mut self, reserve: u128) -> Result<(), String> {
        if self.reserve_count >= 8 {
            return Err("Cannot add more reserves: array full".to_string());
        }

        self.reserves[self.reserve_count as usize] = reserve;
        self.reserve_count += 1;
        Ok(())
    }

    /// Validate bijection property (for testing and debugging)
    ///
    /// Ensures that conversion preserves exact data: Vec → TLV → Vec produces identical result
    #[cfg(test)]
    pub fn validate_bijection(&self, original_reserves: &[u128]) -> bool {
        let recovered = self.to_reserves_vec();
        recovered == original_reserves
    }

    // Zero-copy serialization now available via AsBytes trait:
    // let bytes: &[u8] = liquidity.as_bytes();
    // let tlv_ref = PoolLiquidityTLV::ref_from(bytes)?;

    // Zero-copy deserialization available via zerocopy traits:
    // let tlv_ref = zerocopy::Ref::<_, PoolLiquidityTLV>::new(bytes).unwrap();
    // Direct access without allocation: tlv_ref.reserves[0..tlv_ref.reserve_count as usize]

    // Legacy to_tlv_message removed - use Protocol V2 TLVMessageBuilder instead
}

impl StateInvalidationTLV {
    /// Create new state invalidation with dynamic instrument array
    pub fn new(
        venue: VenueId,
        sequence: u64,
        instruments: &[InstrumentId], // Pass slice of instruments
        reason: InvalidationReason,
        timestamp_ns: u64,
    ) -> Result<Self, String> {
        if instruments.len() > 16 {
            return Err(format!(
                "Too many instruments: {} (max 16)",
                instruments.len()
            ));
        }

        let mut fixed_instruments = [InstrumentId::new_zeroed(); 16];
        // Copy actual instruments to fixed array
        for (i, instrument) in instruments.iter().enumerate() {
            fixed_instruments[i] = *instrument;
        }

        Ok(Self {
            sequence,
            timestamp_ns,
            instruments: fixed_instruments,
            venue: venue as u16,
            instrument_count: instruments.len() as u16,
            reason: reason as u8,
            _padding: [0u8; 3],
        })
    }

    /// Get slice of actual instruments (excluding unused slots)
    pub fn get_instruments(&self) -> &[InstrumentId] {
        &self.instruments[..self.instrument_count as usize]
    }

    /// Add instrument to the invalidation (if space available)
    pub fn add_instrument(&mut self, instrument: InstrumentId) -> Result<(), String> {
        if self.instrument_count >= 16 {
            return Err("Cannot add more instruments: array full".to_string());
        }

        self.instruments[self.instrument_count as usize] = instrument;
        self.instrument_count += 1;
        Ok(())
    }

    /// Convert valid instruments to Vec (perfect bijection preservation)
    ///
    /// This method enables perfect bijection: Vec<InstrumentId> → StateInvalidationTLV → Vec<InstrumentId>
    /// where the output Vec is identical to the original input Vec.
    pub fn to_instruments_vec(&self) -> Vec<InstrumentId> {
        self.get_instruments().to_vec()
    }

    /// Create from Vec with bijection validation (convenience method)
    ///
    /// Equivalent to new() but takes Vec directly for cleaner API.
    /// Validates perfect roundtrip: original_vec == tlv.to_instruments_vec()
    pub fn from_instruments_vec(
        venue: VenueId,
        sequence: u64,
        instruments: Vec<InstrumentId>,
        reason: InvalidationReason,
        timestamp_ns: u64,
    ) -> Result<Self, String> {
        Self::new(venue, sequence, &instruments, reason, timestamp_ns)
    }

    /// Validate bijection property (for testing and debugging)
    ///
    /// Ensures that conversion preserves exact data: Vec → TLV → Vec produces identical result
    #[cfg(test)]
    pub fn validate_bijection(&self, original_instruments: &[InstrumentId]) -> bool {
        let recovered = self.to_instruments_vec();
        recovered == original_instruments
    }

    // Zero-copy serialization now available via AsBytes trait:
    // let bytes: &[u8] = invalidation.as_bytes();
    // let tlv_ref = StateInvalidationTLV::ref_from(bytes)?;
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

        let bytes = trade.as_bytes();
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
        let recovered = TradeTLV::from_bytes(trade.as_bytes()).unwrap();
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

        let bytes = quote.as_bytes();
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
        let recovered = QuoteTLV::from_bytes(quote.as_bytes()).unwrap();
        assert_eq!(quote, recovered);
    }

    #[test]
    fn test_quote_tlv_size() {
        // Verify QuoteTLV has the expected size
        use std::mem::size_of;
        assert_eq!(size_of::<QuoteTLV>(), 52);
    }
}

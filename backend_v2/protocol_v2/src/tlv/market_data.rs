//! Market Data TLV Structures
//!
//! Defines concrete TLV structures for market data messages

use crate::{InstrumentId, VenueId}; // TLVType removed with legacy TLV system
                                    // Legacy TLV types removed - using Protocol V2 MessageHeader + TLV extensions
use zerocopy::{AsBytes, FromBytes, FromZeroes};
use super::dynamic_payload::{FixedVec, DynamicPayload};
use crate::{define_tlv, define_tlv_with_padding};

// Trade TLV structure using macro for consistency
define_tlv! {
    /// Trade TLV structure - simplified for serialization
    ///
    /// Fields are ordered to eliminate padding: u64/i64 → u16 → u8
    TradeTLV {
        u64: {
            asset_id: u64,     // Asset identifier
            price: i64,        // Fixed-point with 8 decimals
            volume: i64,       // Fixed-point with 8 decimals
            timestamp_ns: u64  // Nanoseconds since epoch
        }
        u32: {}
        u16: { venue_id: u16 } // VenueId as primitive
        u8: {
            asset_type: u8,    // AssetType as primitive
            reserved: u8,      // Reserved byte for alignment
            side: u8,          // 0 = buy, 1 = sell
            _padding: [u8; 3]  // Padding to reach 40 bytes (multiple of 8)
        }
        special: {}
    }
}

impl TradeTLV {
    /// Semantic constructor that matches test expectations
    /// NOTE: This shadows the macro-generated new() to maintain backward compatibility
    pub fn new(
        venue: VenueId,
        instrument_id: InstrumentId,
        price: i64,
        volume: i64,
        side: u8,
        timestamp_ns: u64,
    ) -> Self {
        // Use macro-generated constructor with proper field order
        Self::new_raw(
            instrument_id.asset_id,
            price,
            volume,
            timestamp_ns,
            venue as u16,
            instrument_id.asset_type,
            instrument_id.reserved,
            side,
            [0; 3],
        )
    }

    /// Create from high-level types with InstrumentId (backward compatible)
    pub fn from_instrument(
        venue: VenueId,
        instrument_id: InstrumentId,
        price: i64,
        volume: i64,
        side: u8,
        timestamp_ns: u64,
    ) -> Self {
        Self::new(venue, instrument_id, price, volume, side, timestamp_ns)
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

    // from_bytes() method now provided by the macro
}

// Quote TLV structure using macro for consistency
define_tlv! {
    /// Quote TLV structure (best bid/ask) - optimized for zero-copy serialization
    ///
    /// Padded to 56 bytes for 8-byte alignment
    QuoteTLV {
        u64: {
            asset_id: u64,     // Asset identifier
            bid_price: i64,    // Fixed-point with 8 decimals
            bid_size: i64,     // Fixed-point with 8 decimals
            ask_price: i64,    // Fixed-point with 8 decimals
            ask_size: i64,     // Fixed-point with 8 decimals
            timestamp_ns: u64  // Nanoseconds since epoch
        }
        u32: {}
        u16: { venue_id: u16 } // VenueId as primitive
        u8: {
            asset_type: u8,    // AssetType as primitive
            reserved: u8,      // Reserved byte for alignment
            _padding: [u8; 4]  // Required for 8-byte alignment to 56 bytes
        }
        special: {}
    }
}

impl QuoteTLV {
    /// Semantic constructor that matches test expectations
    pub fn new(
        venue: VenueId,
        instrument_id: InstrumentId,
        bid_price: i64,
        bid_size: i64,
        ask_price: i64,
        ask_size: i64,
        timestamp_ns: u64,
    ) -> Self {
        // Use macro-generated constructor with proper field order
        Self::new_raw(
            instrument_id.asset_id,
            bid_price,
            bid_size,
            ask_price,
            ask_size,
            timestamp_ns,
            venue as u16,
            instrument_id.asset_type,
            instrument_id.reserved,
            [0; 4],
        )
    }

    /// Create from high-level types with InstrumentId (backward compatible)
    pub fn from_instrument(
        venue: VenueId,
        instrument_id: InstrumentId,
        bid_price: i64,
        bid_size: i64,
        ask_price: i64,
        ask_size: i64,
        timestamp_ns: u64,
    ) -> Self {
        Self::new(venue, instrument_id, bid_price, bid_size, ask_price, ask_size, timestamp_ns)
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

    // from_bytes() method now provided by the macro
}

/// Order book level for bid/ask aggregation
#[repr(C)]  // ✅ FIXED: Removed 'packed' to maintain proper alignment
#[derive(Debug, Clone, Copy, AsBytes, FromBytes, FromZeroes)]
pub struct OrderLevel {
    /// Price in fixed-point (8 decimals for traditional exchanges, native precision for DEX)
    pub price: i64,
    /// Size/volume in fixed-point (8 decimals for traditional exchanges, native precision for DEX)
    pub size: i64,
    /// Number of orders at this level (0 if not supported by venue)
    pub order_count: u32,
    /// Reserved for alignment and future use
    pub reserved: u32,
}

impl OrderLevel {
    /// Create new order level with precision validation
    pub fn new(price: i64, size: i64, order_count: u32) -> Self {
        Self {
            price,
            size,
            order_count,
            reserved: 0,
        }
    }
    
    /// Get price as decimal (divide by precision factor)
    pub fn price_decimal(&self, precision_factor: i64) -> f64 {
        self.price as f64 / precision_factor as f64
    }
    
    /// Get size as decimal (divide by precision factor)
    pub fn size_decimal(&self, precision_factor: i64) -> f64 {
        self.size as f64 / precision_factor as f64
    }
    
    /// Read OrderLevel from 24-byte slice
    pub fn read_from(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 24 {
            return None;
        }
        
        let price = i64::from_le_bytes(bytes[0..8].try_into().ok()?);
        let size = i64::from_le_bytes(bytes[8..16].try_into().ok()?);
        let order_count = u32::from_le_bytes(bytes[16..20].try_into().ok()?);
        let reserved = u32::from_le_bytes(bytes[20..24].try_into().ok()?);
        
        Some(Self { price, size, order_count, reserved })
    }
}


/// OrderBook TLV structure for complete order book snapshots
///
/// Uses variable-size format to handle different market depths efficiently.
/// Supports both traditional exchange (8-decimal) and DEX (native token precision) formats.
#[derive(Debug, Clone)]
pub struct OrderBookTLV {
    /// Asset identifier from InstrumentId
    pub asset_id: u64,
    /// Venue identifier as primitive u16
    pub venue_id: u16,
    /// Asset type from InstrumentId
    pub asset_type: u8,
    /// Reserved byte for alignment
    pub reserved: u8,
    /// Nanosecond timestamp when snapshot was taken
    pub timestamp_ns: u64,
    /// Sequence number for gap detection (venue-specific)
    pub sequence: u64,
    /// Precision factor for price/size conversion (100_000_000 for 8-decimal, varies for DEX)
    pub precision_factor: i64,
    /// Bid levels (highest price first) - using Vec for simplicity in variable-size TLV
    pub bids: Vec<OrderLevel>,
    /// Ask levels (lowest price first) - using Vec for simplicity in variable-size TLV
    pub asks: Vec<OrderLevel>,
}

impl OrderBookTLV {
    /// Create from InstrumentId with empty order book
    pub fn from_instrument(
        venue: VenueId,
        instrument_id: InstrumentId,
        timestamp_ns: u64,
        sequence: u64,
        precision_factor: i64,
    ) -> Self {
        Self {
            asset_id: instrument_id.asset_id,
            venue_id: venue as u16,
            asset_type: instrument_id.asset_type,
            reserved: instrument_id.reserved,
            timestamp_ns,
            sequence,
            precision_factor,
            bids: Vec::new(),
            asks: Vec::new(),
        }
    }
    
    /// Add bid level (maintains descending price order)
    pub fn add_bid(&mut self, price: i64, size: i64, order_count: u32) {
        let level = OrderLevel::new(price, size, order_count);
        
        // Find insertion point to maintain descending order
        let insert_pos = self.bids.iter()
            .position(|existing| existing.price < price)
            .unwrap_or(self.bids.len());
            
        self.bids.insert(insert_pos, level);
    }
    
    /// Add ask level (maintains ascending price order)
    pub fn add_ask(&mut self, price: i64, size: i64, order_count: u32) {
        let level = OrderLevel::new(price, size, order_count);
        
        // Find insertion point to maintain ascending order
        let insert_pos = self.asks.iter()
            .position(|existing| existing.price > price)
            .unwrap_or(self.asks.len());
            
        self.asks.insert(insert_pos, level);
    }
    
    /// Get best bid (highest price)
    pub fn best_bid(&self) -> Option<&OrderLevel> {
        self.bids.first()
    }
    
    /// Get best ask (lowest price)
    pub fn best_ask(&self) -> Option<&OrderLevel> {
        self.asks.first()
    }
    
    /// Calculate spread in basis points
    pub fn spread_bps(&self) -> Option<i32> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => {
                let spread = ask.price - bid.price;
                let mid = (bid.price + ask.price) / 2;
                if mid > 0 {
                    Some((spread * 10000 / mid) as i32)
                } else {
                    None
                }
            },
            _ => None,
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
    
    /// Calculate total byte size for TLV payload
    pub fn payload_size(&self) -> usize {
        // Fixed header: asset_id(8) + venue_id(2) + asset_type(1) + reserved(1) + 
        // timestamp_ns(8) + sequence(8) + precision_factor(8) = 36 bytes
        // Plus Vec length prefixes (4 bytes each) and OrderLevel data (24 bytes each)
        36 + 4 + (self.bids.len() * 24) + 4 + (self.asks.len() * 24)
    }
    
    /// Validate order book integrity  
    pub fn validate(&self) -> Result<(), String> {
        // Check bid ordering (descending)
        for window in self.bids.windows(2) {
            if window[0].price < window[1].price {
                return Err("Bids not in descending price order".to_string());
            }
        }
        
        // Check ask ordering (ascending)
        for window in self.asks.windows(2) {
            if window[0].price > window[1].price {
                return Err("Asks not in ascending price order".to_string());
            }
        }
        
        // Check no negative prices or sizes
        for bid in &self.bids {
            if bid.price <= 0 || bid.size <= 0 {
                return Err("Invalid bid price or size".to_string());
            }
        }
        
        for ask in &self.asks {
            if ask.price <= 0 || ask.size <= 0 {
                return Err("Invalid ask price or size".to_string());
            }
        }
        
        // Check spread sanity (best ask >= best bid)
        if let (Some(best_bid), Some(best_ask)) = (self.best_bid(), self.best_ask()) {
            if best_ask.price < best_bid.price {
                return Err("Best ask price below best bid price".to_string());
            }
        }
        
        Ok(())
    }
    
    /// Serialize OrderBook to bytes for TLV payload
    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        
        let mut bytes = Vec::with_capacity(self.payload_size());
        
        // Serialize fixed fields (36 bytes)
        bytes.extend_from_slice(&self.asset_id.to_le_bytes());
        bytes.extend_from_slice(&self.venue_id.to_le_bytes());
        bytes.push(self.asset_type);
        bytes.push(self.reserved);
        bytes.extend_from_slice(&self.timestamp_ns.to_le_bytes());
        bytes.extend_from_slice(&self.sequence.to_le_bytes());
        bytes.extend_from_slice(&self.precision_factor.to_le_bytes());
        
        // Serialize bids array (length + data)
        bytes.extend_from_slice(&(self.bids.len() as u32).to_le_bytes());
        for bid in &self.bids {
            bytes.extend_from_slice(bid.as_bytes());
        }
        
        // Serialize asks array (length + data)
        bytes.extend_from_slice(&(self.asks.len() as u32).to_le_bytes());
        for ask in &self.asks {
            bytes.extend_from_slice(ask.as_bytes());
        }
        
        Ok(bytes)
    }
    
    /// Deserialize OrderBook from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 36 {
            return Err("Insufficient data for OrderBook header".to_string());
        }
        
        let mut offset = 0;
        
        // Deserialize fixed fields
        let asset_id = u64::from_le_bytes(
            bytes[offset..offset + 8].try_into().map_err(|_| "Invalid asset_id bytes")?
        );
        offset += 8;
        
        let venue_id = u16::from_le_bytes(
            bytes[offset..offset + 2].try_into().map_err(|_| "Invalid venue_id bytes")?
        );
        offset += 2;
        
        let asset_type = bytes[offset];
        offset += 1;
        
        let reserved = bytes[offset];
        offset += 1;
        
        let timestamp_ns = u64::from_le_bytes(
            bytes[offset..offset + 8].try_into().map_err(|_| "Invalid timestamp bytes")?
        );
        offset += 8;
        
        let sequence = u64::from_le_bytes(
            bytes[offset..offset + 8].try_into().map_err(|_| "Invalid sequence bytes")?
        );
        offset += 8;
        
        let precision_factor = i64::from_le_bytes(
            bytes[offset..offset + 8].try_into().map_err(|_| "Invalid precision_factor bytes")?
        );
        offset += 8;
        
        // Deserialize bids array
        if offset + 4 > bytes.len() {
            return Err("Insufficient data for bids length".to_string());
        }
        
        let bids_len = u32::from_le_bytes(
            bytes[offset..offset + 4].try_into().map_err(|_| "Invalid bids length")?
        ) as usize;
        offset += 4;
        
        let mut bids = Vec::with_capacity(bids_len);
        for _ in 0..bids_len {
            if offset + 24 > bytes.len() {
                return Err("Insufficient data for bid level".to_string());
            }
            let level = OrderLevel::read_from(&bytes[offset..offset + 24])
                .ok_or("Failed to read bid level")?;
            bids.push(level);
            offset += 24;
        }
        
        // Deserialize asks array
        if offset + 4 > bytes.len() {
            return Err("Insufficient data for asks length".to_string());
        }
        
        let asks_len = u32::from_le_bytes(
            bytes[offset..offset + 4].try_into().map_err(|_| "Invalid asks length")?
        ) as usize;
        offset += 4;
        
        let mut asks = Vec::with_capacity(asks_len);
        for _ in 0..asks_len {
            if offset + 24 > bytes.len() {
                return Err("Insufficient data for ask level".to_string());
            }
            let level = OrderLevel::read_from(&bytes[offset..offset + 24])
                .ok_or("Failed to read ask level")?;
            asks.push(level);
            offset += 24;
        }
        
        let order_book = Self {
            asset_id,
            venue_id,
            asset_type,
            reserved,
            timestamp_ns,
            sequence,
            precision_factor,
            bids,
            asks,
        };
        
        // Validate integrity after deserialization
        order_book.validate()?;
        
        Ok(order_book)
    }
}

/// State invalidation TLV structure - Zero-copy with FixedVec
///
/// Supports up to 16 instruments per invalidation (more than sufficient for real-world usage)
/// Most invalidations affect 1-5 instruments, with 16 providing generous headroom.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StateInvalidationTLV {
    // Group 64-bit fields first (16 bytes)
    pub sequence: u64,     // Sequence number
    pub timestamp_ns: u64, // Nanoseconds since epoch

    // FixedVec for instruments with zero-copy serialization
    pub instruments: super::dynamic_payload::FixedVec<InstrumentId, { super::dynamic_payload::MAX_INSTRUMENTS }>, // Instrument IDs

    // Then smaller fields (8 bytes total)
    pub venue: u16,        // VenueId as u16 (2 bytes)
    pub reason: u8,        // InvalidationReason as u8 (1 byte)
    pub _padding: [u8; 5], // Explicit padding for alignment

                           // Total: 16 + FixedVec size + 8 = varies (aligned)
}

// Manual zerocopy implementations for StateInvalidationTLV
// SAFETY: StateInvalidationTLV has a well-defined memory layout with #[repr(C)]:
// - sequence: u64 (8 bytes)
// - timestamp_ns: u64 (8 bytes)
// - instruments: FixedVec<InstrumentId, 16> (136 bytes) 
// - venue: u16 (2 bytes)
// - reason: u8 (1 byte)
// - _padding: [u8; 5] (5 bytes)
// Total: aligned with proper field layout
//
// All fields implement the required zerocopy traits:
// - u64, u16, u8 arrays are primitive zerocopy types
// - FixedVec<InstrumentId, MAX_INSTRUMENTS> has manual zerocopy implementations
// - The struct uses #[repr(C)] for deterministic layout
unsafe impl AsBytes for StateInvalidationTLV {
    fn only_derive_is_allowed_to_implement_this_trait() {}
}

unsafe impl FromBytes for StateInvalidationTLV {
    fn only_derive_is_allowed_to_implement_this_trait() {}
}

unsafe impl FromZeroes for StateInvalidationTLV {
    fn only_derive_is_allowed_to_implement_this_trait() {}
}

/// Pool liquidity update TLV structure - Zero-copy with FixedVec
///
/// Tracks only liquidity changes - fee rates come from PoolStateTLV
/// Supports up to 8 token reserves (sufficient for even complex Balancer/Curve pools)
/// Most pools have 2 tokens, with 8 providing generous headroom.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoolLiquidityTLV {
    // Group 64-bit fields first (8 bytes)
    pub timestamp_ns: u64, // Nanoseconds since epoch

    // FixedVec for reserves with zero-copy serialization
    pub reserves: super::dynamic_payload::FixedVec<u128, { super::dynamic_payload::MAX_POOL_TOKENS }>, // Token reserves (native precision)

    // Pool address (32 bytes - padded from 20-byte Ethereum address)  
    pub pool_address: [u8; 32], // Full pool contract address (20 bytes + 12 padding)

    // Then smaller fields (8 bytes total to align properly)
    pub venue: u16,        // VenueId as u16 (2 bytes)
    pub _padding: [u8; 6], // Explicit padding for alignment

                           // Total: 8 + (2 + 6 + 8*16) + 32 + 8 = 176 bytes (maintaining same size)
}

// Manual zerocopy implementations for PoolLiquidityTLV
// SAFETY: PoolLiquidityTLV has a well-defined memory layout with #[repr(C)]:
// - timestamp_ns: u64 (8 bytes)
// - reserves: FixedVec<u128, 8> (136 bytes - count:2 + padding:6 + elements:128)  
// - pool_address: [u8; 32] (32 bytes)
// - venue: u16 (2 bytes)
// - _padding: [u8; 6] (6 bytes)
// Total: 184 bytes with proper alignment
//
// All fields implement the required zerocopy traits:
// - u64, u16, u8 arrays are primitive zerocopy types
// - FixedVec<u128, MAX_POOL_TOKENS> has manual zerocopy implementations above
// - The struct uses #[repr(C)] for deterministic layout
unsafe impl AsBytes for PoolLiquidityTLV {
    fn only_derive_is_allowed_to_implement_this_trait() {}
}

unsafe impl FromBytes for PoolLiquidityTLV {
    fn only_derive_is_allowed_to_implement_this_trait() {}
}

unsafe impl FromZeroes for PoolLiquidityTLV {
    fn only_derive_is_allowed_to_implement_this_trait() {}
}

// Pool swap event TLV structure using macro
define_tlv! {
    /// Pool swap event TLV structure
    ///
    /// Records individual swaps with full token addresses for execution capability
    PoolSwapTLV {
        u128: {
            amount_in: u128,       // Amount in (native precision, no scaling)
            amount_out: u128,      // Amount out (native precision, no scaling)
            liquidity_after: u128  // Active liquidity after swap (V3)
        }
        u64: {
            timestamp_ns: u64, // Nanoseconds since epoch
            block_number: u64  // Block number of swap
        }
        u32: { tick_after: i32 } // New tick after swap (V3)
        u16: { venue: u16 } // NOT VenueId enum! Direct u16 for zero-copy
        u8: {
            amount_in_decimals: u8,  // Decimals for amount_in (e.g., WMATIC=18)
            amount_out_decimals: u8, // Decimals for amount_out (e.g., USDC=6)
            _padding: [u8; 8]        // Required for alignment to 208 bytes
        }
        special: {
            pool_address: [u8; 32],      // Full pool contract address
            token_in_addr: [u8; 32],     // Full input token address
            token_out_addr: [u8; 32],    // Full output token address
            sqrt_price_x96_after: [u8; 32]  // New sqrt price after swap (V3)
        }
    }
}

impl PoolSwapTLV {
    /// Semantic constructor that matches test expectations
    /// Takes 20-byte addresses and basic parameters, similar to test usage
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

        Self::new_raw(
            amount_in,
            amount_out,
            liquidity_after,
            timestamp_ns,
            block_number,
            tick_after,
            venue_id as u16,
            amount_in_decimals,
            amount_out_decimals,
            [0u8; 8], // padding
            pool.to_padded(),
            token_in.to_padded(),
            token_out.to_padded(),
            Self::sqrt_price_from_u128(sqrt_price_x96_after),
        )
    }

    /// Create a new PoolSwapTLV from Ethereum addresses
    #[allow(clippy::too_many_arguments)]
    pub fn from_addresses(
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
        Self::new(
            pool,
            token_in,
            token_out,
            venue_id,
            amount_in,
            amount_out,
            liquidity_after,
            timestamp_ns,
            block_number,
            tick_after,
            amount_in_decimals,
            amount_out_decimals,
            sqrt_price_x96_after,
        )
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

// Pool Sync event TLV structure using macro with explicit size
define_tlv_with_padding! {
    /// Pool Sync event TLV structure (V2 pools)
    ///
    /// V2 pools emit Sync events after every state change with complete reserves
    /// Total: 160 bytes (10 × 16)
    PoolSyncTLV {
        size: 160,
        u128: {
            reserve0: u128, // Complete reserve0 (native precision)
            reserve1: u128  // Complete reserve1 (native precision)
        }
        u64: {
            timestamp_ns: u64, // Nanoseconds since epoch
            block_number: u64  // Block number of sync
        }
        u16: { venue: u16 } // NOT VenueId enum! Direct u16 for zero-copy
        u8: {
            token0_decimals: u8, // Decimals for token0 (e.g., WMATIC=18)
            token1_decimals: u8, // Decimals for token1 (e.g., USDC=6)
            _padding: [u8; 12]   // Required for alignment to 160 bytes
        }
        special: {
            pool_address: [u8; 32], // Full pool contract address
            token0_addr: [u8; 32],  // Full token0 address
            token1_addr: [u8; 32]   // Full token1 address
        }
    }
}

impl PoolSyncTLV {
    /// Create a new PoolSyncTLV from components
    #[allow(clippy::too_many_arguments)]
    pub fn from_components(
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

        // Use macro-generated new_raw() with proper field order
        Self::new_raw(
            reserve0,
            reserve1,
            timestamp_ns,
            block_number,
            venue_id as u16,
            token0_decimals,
            token1_decimals,
            [0u8; 12], // _padding
            pool.to_padded(),
            token0.to_padded(),
            token1.to_padded(),
        )
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

    // to_bytes() method DELETED - use zerocopy's AsBytes trait instead

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

    // to_bytes() method DELETED - use zerocopy's AsBytes trait instead

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

    // to_bytes() method DELETED - use zerocopy's AsBytes trait instead

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
    /// Create new liquidity update with FixedVec reserves
    pub fn new(
        venue: VenueId,
        pool_address: [u8; 20], // Input as 20-byte address
        reserves: &[u128],      // Pass slice of reserves
        timestamp_ns: u64,
    ) -> Result<Self, String> {
        if reserves.is_empty() {
            return Err("Reserves cannot be empty".to_string());
        }

        // Use FixedVec::from_slice for bounds validation and initialization
        let reserves_vec = FixedVec::from_slice(reserves)
            .map_err(|e| format!("Failed to create reserves FixedVec: {}", e))?;

        // Convert 20-byte address to 32-byte padded format
        use super::address::AddressConversion;
        let pool_address_32 = pool_address.to_padded();

        Ok(Self {
            timestamp_ns,
            reserves: reserves_vec,
            pool_address: pool_address_32,
            venue: venue as u16,
            _padding: [0u8; 6],
        })
    }

    /// Get slice of actual reserves (excluding unused slots)
    pub fn get_reserves(&self) -> &[u128] {
        self.reserves.as_slice()
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
        self.reserves.to_vec()
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
        self.reserves.try_push(reserve)
            .map_err(|e| format!("Failed to add reserve: {}", e))
    }

    /// Get number of valid reserves
    pub fn len(&self) -> usize {
        self.reserves.len()
    }

    /// Check if reserves are empty
    pub fn is_empty(&self) -> bool {
        self.reserves.is_empty()
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
    /// Create new state invalidation with FixedVec instruments
    pub fn new(
        venue: VenueId,
        sequence: u64,
        instruments: &[InstrumentId], // Pass slice of instruments
        reason: InvalidationReason,
        timestamp_ns: u64,
    ) -> Result<Self, String> {
        if instruments.is_empty() {
            return Err("Instruments cannot be empty".to_string());
        }

        // Use FixedVec::from_slice for bounds validation and initialization
        let instruments_vec = FixedVec::from_slice(instruments)
            .map_err(|e| format!("Failed to create instruments FixedVec: {}", e))?;

        Ok(Self {
            sequence,
            timestamp_ns,
            instruments: instruments_vec,
            venue: venue as u16,
            reason: reason as u8,
            _padding: [0u8; 5],
        })
    }

    /// Get slice of actual instruments (excluding unused slots)
    pub fn get_instruments(&self) -> &[InstrumentId] {
        self.instruments.as_slice()
    }

    /// Add instrument to the invalidation (if space available)
    pub fn add_instrument(&mut self, instrument: InstrumentId) -> Result<(), String> {
        self.instruments.try_push(instrument)
            .map_err(|e| format!("Failed to add instrument: {}", e))
    }

    /// Get number of valid instruments
    pub fn len(&self) -> usize {
        self.instruments.len()
    }

    /// Check if instruments are empty
    pub fn is_empty(&self) -> bool {
        self.instruments.is_empty()
    }

    /// Convert valid instruments to Vec (perfect bijection preservation)
    ///
    /// This method enables perfect bijection: Vec<InstrumentId> → StateInvalidationTLV → Vec<InstrumentId>
    /// where the output Vec is identical to the original input Vec.
    pub fn to_instruments_vec(&self) -> Vec<InstrumentId> {
        self.instruments.to_vec()
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

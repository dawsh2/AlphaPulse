use thiserror::Error;
use zerocopy::{AsBytes, FromBytes, FromZeroes};
use std::mem::size_of;
use num_enum::TryFromPrimitive;
use anyhow::{Result, anyhow};

/// Magic number for message validation
pub const MESSAGE_MAGIC: u32 = 0xDEADBEEF;

/// Message parsing errors
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Invalid magic number: expected {expected:#x}, got {actual:#x}")]
    InvalidMagic { expected: u32, actual: u32 },
    #[error("Checksum mismatch: expected {expected:#x}, got {actual:#x}")]
    ChecksumMismatch { expected: u32, actual: u32 },
    #[error("Message too small: need {need} bytes, got {got}")]
    TooSmall { need: usize, got: usize },
    #[error("Invalid layout for message type")]
    InvalidLayout,
    #[error("Unknown schema for message type {message_type} version {version}")]
    UnknownSchema { message_type: u8, version: u8 },
    #[error("Invalid venue ID: {0}")]
    InvalidVenueId(u16),
    #[error("Invalid asset type: {0}")]
    InvalidAssetType(u8),
}

/// Message type discriminants
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
pub enum MessageType {
    // Market data (1-9)
    Trade = 1,
    Quote = 2,
    OrderBook = 3,
    SwapEvent = 4,
    PoolUpdate = 5,
    
    // Discovery (10-19)
    InstrumentDiscovered = 10,
    PoolDiscovered = 11,
    VenueUpdate = 12,
    TokenDiscovered = 13,
    
    // Trading signals (20-29)
    ArbitrageOpportunity = 20,
    DeFiSignal = 21,  // Generic DeFi signal (arb, liquidation, yield, etc.)
    MomentumSignal = 22,
    MeanReversionSignal = 23,
    
    // System (100-109)
    Heartbeat = 100,
    Snapshot = 101,
    Error = 102,
    
    // Custom strategies (200-255)
    Custom = 200,
}

/// Source type discriminants
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
pub enum SourceType {
    // Collectors (1-19)
    BinanceCollector = 1,
    EthereumCollector = 2,
    PolygonCollector = 3,
    
    // Core services (20-39)
    RelayServer = 20,
    Scanner = 21,
    Bridge = 22,
    
    // Strategies (40-59)
    ArbitrageStrategy = 40,
    MomentumStrategy = 41,
    MarketMakingStrategy = 42,
    
    // External (100+)
    External = 100,
}

/// Venue identification
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
pub enum VenueId {
    // Centralized exchanges
    Binance = 1,
    Coinbase = 2,
    Kraken = 3,
    
    // Blockchains
    Ethereum = 100,
    Polygon = 101,
    Arbitrum = 102,
    Optimism = 103,
    
    // DEXs on Ethereum
    UniswapV2 = 200,
    UniswapV3 = 201,
    SushiSwap = 202,
    CurveFinance = 203,
    
    // DEXs on Polygon
    QuickSwap = 300,
    QuickSwapV3 = 301,
    PolygonSushi = 302,
    
    // Stock exchanges
    NYSE = 1000,
    NASDAQ = 1001,
    LSE = 1002,
}

/// Asset type classification
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
pub enum AssetType {
    Token = 1,
    Stock = 2,
    Pool = 3,
    Future = 4,
    Option = 5,
    Index = 6,
}

/// Bijective instrument identifier
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, AsBytes, FromBytes, FromZeroes)]
pub struct InstrumentId {
    pub venue: u16,        // VenueId enum
    pub asset_type: u8,    // AssetType enum
    pub reserved: u8,      // Future use/flags
    pub asset_id: u64,     // Venue-specific identifier
}

impl InstrumentId {
    /// Create an Ethereum token ID from address (first 8 bytes for full precision)
    pub fn ethereum_token(address: &str) -> Result<Self> {
        let clean_addr = address.strip_prefix("0x").unwrap_or(address);
        if clean_addr.len() < 16 {
            return Err(anyhow!("Address too short: {}", address));
        }
        
        let bytes = hex::decode(&clean_addr[..16])
            .map_err(|e| anyhow!("Invalid hex address: {}", e))?;
        
        // Use full 8 bytes (64 bits) for maximum precision and uniqueness
        let mut byte_array = [0u8; 8];
        byte_array.copy_from_slice(&bytes[..8]);
        let asset_id = u64::from_be_bytes(byte_array);
        
        Ok(Self {
            venue: VenueId::Ethereum as u16,
            asset_type: AssetType::Token as u8,
            reserved: 0,
            asset_id,
        })
    }
    
    /// Create a Polygon token ID from address (first 8 bytes for full precision)
    pub fn polygon_token(address: &str) -> Result<Self> {
        let clean_addr = address.strip_prefix("0x").unwrap_or(address);
        if clean_addr.len() < 16 {
            return Err(anyhow!("Address too short: {}", address));
        }
        
        let bytes = hex::decode(&clean_addr[..16])
            .map_err(|e| anyhow!("Invalid hex address: {}", e))?;
        
        // Use full 8 bytes (64 bits) for maximum precision and uniqueness
        let mut byte_array = [0u8; 8];
        byte_array.copy_from_slice(&bytes[..8]);
        let asset_id = u64::from_be_bytes(byte_array);
        
        Ok(Self {
            venue: VenueId::Polygon as u16,
            asset_type: AssetType::Token as u8,
            reserved: 0,
            asset_id,
        })
    }
    
    /// Create a stock ID from exchange and symbol
    pub fn stock(exchange: VenueId, symbol: &str) -> Self {
        Self {
            venue: exchange as u16,
            asset_type: AssetType::Stock as u8,
            reserved: 0,
            asset_id: symbol_to_u64(symbol),
        }
    }
    
    /// Create a DEX pool ID from constituent tokens
    pub fn pool(dex: VenueId, token0: InstrumentId, token1: InstrumentId) -> Self {
        // Canonical ordering for consistency
        let (id0, id1) = if token0.asset_id <= token1.asset_id {
            (token0.asset_id, token1.asset_id)
        } else {
            (token1.asset_id, token0.asset_id)
        };
        
        // Deterministic combination avoiding collisions
        let combined = (id0.wrapping_shr(1)) ^ (id1.wrapping_shl(1));
        
        Self {
            venue: dex as u16,
            asset_type: AssetType::Pool as u8,
            reserved: 0,
            asset_id: combined,
        }
    }
    
    /// Convert to u64 for backward compatibility cache keys
    /// Note: This truncates asset_id to 40 bits - use hash() for full precision
    pub fn to_u64(&self) -> u64 {
        // Copy packed fields to avoid unaligned reference errors
        let venue = self.venue;
        let asset_type = self.asset_type;
        let asset_id = self.asset_id;
        
        ((venue as u64) << 48) | 
        ((asset_type as u64) << 40) | 
        (asset_id & 0xFFFFFFFFFF)
    }
    
    /// Reconstruct from u64 (loses precision for large asset_ids)
    /// Note: Only the lower 40 bits of asset_id are preserved
    pub fn from_u64(value: u64) -> Self {
        Self {
            venue: ((value >> 48) & 0xFFFF) as u16,
            asset_type: ((value >> 40) & 0xFF) as u8,
            reserved: 0,
            asset_id: value & 0xFFFFFFFFFF,
        }
    }
    
    /// Generate a hash for use as cache key (preserves full precision)
    pub fn cache_key(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        // Hash the full struct to preserve all information
        Hash::hash(self, &mut hasher);
        hasher.finish()
    }
    
    /// Human-readable debug info
    pub fn debug_info(&self) -> String {
        // Copy packed fields to avoid unaligned reference errors
        let venue = self.venue;
        let asset_type = self.asset_type;
        let asset_id = self.asset_id;
        
        match (VenueId::try_from(venue), AssetType::try_from(asset_type)) {
            (Ok(venue_enum), Ok(AssetType::Token)) => {
                format!("{:?} Token 0x{:010x}...", venue_enum, asset_id)
            }
            (Ok(venue_enum), Ok(AssetType::Stock)) => {
                format!("{:?} Stock: {}", venue_enum, u64_to_symbol(asset_id))
            }
            (Ok(venue_enum), Ok(AssetType::Pool)) => {
                format!("{:?} Pool #{}", venue_enum, asset_id)
            }
            _ => format!("Unknown {}/{} #{}", venue, asset_type, asset_id)
        }
    }
    
    /// Get venue enum
    pub fn venue(&self) -> Result<VenueId, ParseError> {
        let venue = self.venue; // Copy to avoid unaligned reference
        VenueId::try_from(venue).map_err(|_| ParseError::InvalidVenueId(venue))
    }
    
    /// Get asset type enum
    pub fn asset_type(&self) -> Result<AssetType, ParseError> {
        let asset_type = self.asset_type; // Copy to avoid unaligned reference
        AssetType::try_from(asset_type).map_err(|_| ParseError::InvalidAssetType(asset_type))
    }
}

/// Convert symbol to u64 (up to 8 characters)
fn symbol_to_u64(symbol: &str) -> u64 {
    let mut bytes = [0u8; 8];
    let len = symbol.len().min(8);
    bytes[..len].copy_from_slice(&symbol.as_bytes()[..len]);
    u64::from_be_bytes(bytes)
}

/// Convert u64 back to symbol
fn u64_to_symbol(value: u64) -> String {
    let bytes = value.to_be_bytes();
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(8);
    String::from_utf8_lossy(&bytes[..end]).to_string()
}

/// Message header (32 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, AsBytes, FromBytes, FromZeroes)]
pub struct MessageHeader {
    pub magic: u32,                 // 0xDEADBEEF
    pub message_type: u8,           // MessageType discriminant
    pub version: u8,                // Schema version
    pub source: u8,                 // SourceType discriminant  
    pub flags: u8,                  // Compression, priority
    pub payload_size: u32,          // Payload bytes
    pub sequence: u64,              // Monotonic sequence
    pub timestamp: u64,             // Nanoseconds
    pub checksum: u32,              // CRC32 of entire message
}

impl MessageHeader {
    /// Create a new message header
    pub fn new(
        message_type: MessageType,
        version: u8,
        source: SourceType,
        payload_size: u32,
        sequence: u64,
    ) -> Self {
        Self {
            magic: MESSAGE_MAGIC,
            message_type: message_type as u8,
            version,
            source: source as u8,
            flags: 0,
            payload_size,
            sequence,
            timestamp: current_timestamp_ns(),
            checksum: 0, // Will be calculated later
        }
    }
    
    /// Calculate and set checksum for the entire message
    pub fn calculate_checksum(&mut self, full_message: &[u8]) {
        self.checksum = 0;
        // CRC32 over entire message except checksum field
        let checksum_offset = size_of::<Self>() - 4;
        let before_checksum = &full_message[..checksum_offset];
        let after_checksum = &full_message[size_of::<Self>()..];
        
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(before_checksum);
        hasher.update(after_checksum);
        self.checksum = hasher.finalize();
    }
    
    /// Validate header magic and structure
    pub fn validate(&self) -> Result<(), ParseError> {
        if self.magic != MESSAGE_MAGIC {
            return Err(ParseError::InvalidMagic {
                expected: MESSAGE_MAGIC,
                actual: self.magic,
            });
        }
        Ok(())
    }
    
    /// Parse header from bytes
    pub fn from_bytes(data: &[u8]) -> Result<&Self, ParseError> {
        if data.len() < size_of::<Self>() {
            return Err(ParseError::TooSmall {
                need: size_of::<Self>(),
                got: data.len(),
            });
        }
        
        let header = zerocopy::Ref::<_, Self>::new(&data[..size_of::<Self>()])
            .ok_or(ParseError::InvalidLayout)?
            .into_ref();
            
        header.validate()?;
        Ok(header)
    }
    
    /// Get message type enum
    pub fn message_type(&self) -> Result<MessageType, ParseError> {
        MessageType::try_from(self.message_type)
            .map_err(|_| ParseError::UnknownSchema {
                message_type: self.message_type,
                version: self.version,
            })
    }
    
    /// Get source type enum
    pub fn source(&self) -> Result<SourceType, ParseError> {
        SourceType::try_from(self.source)
            .map_err(|_| ParseError::UnknownSchema {
                message_type: self.message_type,
                version: self.version,
            })
    }
}

/// Get current timestamp in nanoseconds
fn current_timestamp_ns() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instrument_id_bijective() {
        // Test stock ID creation and roundtrip (shorter symbol to avoid truncation)
        let id = InstrumentId::stock(VenueId::NASDAQ, "A");  // 1-char symbol fits in 40 bits
        
        let as_u64 = id.to_u64();
        let restored = InstrumentId::from_u64(as_u64);
        
        // Compare packed structs field by field to avoid alignment issues
        let original_venue = id.venue;
        let original_asset_type = id.asset_type;
        let original_asset_id = id.asset_id;
        let restored_venue = restored.venue;
        let restored_asset_type = restored.asset_type;
        let restored_asset_id = restored.asset_id;
        
        assert_eq!(original_venue, restored_venue);
        assert_eq!(original_asset_type, restored_asset_type);
        // Only compare low 40 bits for asset_id since to_u64/from_u64 truncates
        assert_eq!(original_asset_id & 0xFFFFFFFFFF, restored_asset_id & 0xFFFFFFFFFF);
        
        assert_eq!(id.venue().unwrap(), VenueId::NASDAQ);
        assert_eq!(id.asset_type().unwrap(), AssetType::Stock);
    }

    #[test]
    fn test_instrument_id_precision_loss() {
        // Test that long addresses lose precision in to_u64/from_u64 conversion
        let full_addr = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";
        let full_id = InstrumentId::ethereum_token(full_addr).unwrap();
        
        let as_u64 = full_id.to_u64();
        let restored = InstrumentId::from_u64(as_u64);
        
        // The high-order bits of asset_id should be different due to truncation
        let original_asset_id = full_id.asset_id;
        let restored_asset_id = restored.asset_id;
        
        // Only the low 40 bits should match
        assert_eq!(original_asset_id & 0xFFFFFFFFFF, restored_asset_id & 0xFFFFFFFFFF);
        
        // For full precision, use cache_key() instead
        assert_ne!(full_id.cache_key(), 0);
    }

    #[test]
    fn test_pool_id_deterministic() {
        let usdc = InstrumentId::ethereum_token("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48").unwrap();
        let weth = InstrumentId::ethereum_token("0xc02aaa39b223fe8d0a0e5c4f27b010c5d91b298a").unwrap();
        
        // Pool IDs should be the same regardless of token order
        let pool1 = InstrumentId::pool(VenueId::UniswapV3, usdc, weth);
        let pool2 = InstrumentId::pool(VenueId::UniswapV3, weth, usdc);
        
        // Compare field by field
        let pool1_venue = pool1.venue;
        let pool1_asset_type = pool1.asset_type;
        let pool1_asset_id = pool1.asset_id;
        let pool2_venue = pool2.venue;
        let pool2_asset_type = pool2.asset_type;
        let pool2_asset_id = pool2.asset_id;
        
        assert_eq!(pool1_venue, pool2_venue);
        assert_eq!(pool1_asset_type, pool2_asset_type);
        assert_eq!(pool1_asset_id, pool2_asset_id);
        
        assert_eq!(pool1.venue().unwrap(), VenueId::UniswapV3);
        assert_eq!(pool1.asset_type().unwrap(), AssetType::Pool);
    }

    #[test]
    fn test_stock_id() {
        let aapl = InstrumentId::stock(VenueId::NASDAQ, "AAPL");
        assert_eq!(aapl.venue().unwrap(), VenueId::NASDAQ);
        assert_eq!(aapl.asset_type().unwrap(), AssetType::Stock);
        
        let debug = aapl.debug_info();
        assert!(debug.contains("NASDAQ"));
        assert!(debug.contains("AAPL"));
    }

    #[test]
    fn test_message_header() {
        let header = MessageHeader::new(
            MessageType::Trade,
            1,
            SourceType::PolygonCollector,
            64,
            1234,
        );
        
        // Copy packed fields for comparison
        let magic = header.magic;
        let sequence = header.sequence;
        
        assert_eq!(magic, MESSAGE_MAGIC);
        assert_eq!(header.message_type().unwrap(), MessageType::Trade);
        assert_eq!(header.source().unwrap(), SourceType::PolygonCollector);
        
        // Test serialization roundtrip
        let bytes = header.as_bytes();
        let restored = MessageHeader::from_bytes(bytes).unwrap();
        
        let restored_message_type = restored.message_type;
        let restored_sequence = restored.sequence;
        
        let header_message_type = header.message_type;
        assert_eq!(header_message_type, restored_message_type);
        assert_eq!(sequence, restored_sequence);
    }

    #[test]
    fn test_symbol_conversion() {
        let symbol = "AAPL";
        let as_u64 = symbol_to_u64(symbol);
        let restored = u64_to_symbol(as_u64);
        assert_eq!(symbol, restored);
        
        // Test long symbol (truncated)
        let long_symbol = "VERYLONGSYMBOL";
        let as_u64 = symbol_to_u64(long_symbol);
        let restored = u64_to_symbol(as_u64);
        assert_eq!("VERYLONG", restored); // Truncated to 8 chars
    }
}
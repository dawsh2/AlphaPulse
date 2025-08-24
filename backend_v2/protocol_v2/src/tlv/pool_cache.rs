//! Pool Cache TLV Structures
//!
//! Binary TLV format for persisting discovered pool → token mappings.
//! Uses full 20-byte addresses for execution compatibility.

use crate::VenueId;
use zerocopy::{AsBytes, FromBytes, FromZeroes};

/// Pool type for cache records (renamed to avoid conflict with pool_state)
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CachePoolType {
    UniswapV2 = 1,
    UniswapV3 = 2,
    QuickSwapV2 = 3,
    QuickSwapV3 = 4,
    SushiSwapV2 = 5,
    CurveV2 = 6,
    BalancerV2 = 7,
}

impl TryFrom<u8> for CachePoolType {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(CachePoolType::UniswapV2),
            2 => Ok(CachePoolType::UniswapV3),
            3 => Ok(CachePoolType::QuickSwapV2),
            4 => Ok(CachePoolType::QuickSwapV3),
            5 => Ok(CachePoolType::SushiSwapV2),
            6 => Ok(CachePoolType::CurveV2),
            7 => Ok(CachePoolType::BalancerV2),
            _ => Err(format!("Invalid pool type: {}", value)),
        }
    }
}

/// Individual pool information record (Type 200)
/// Stores full 20-byte addresses for pool and both tokens
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, AsBytes, FromBytes, FromZeroes)]
pub struct PoolInfoTLV {
    pub tlv_type: u8,             // 200
    pub tlv_length: u8,           // 83 bytes
    pub pool_address: [u8; 20],   // Full pool contract address
    pub token0_address: [u8; 20], // Full token0 address
    pub token1_address: [u8; 20], // Full token1 address
    pub token0_decimals: u8,      // Token0 decimal places (e.g., 18 for WETH, 6 for USDC)
    pub token1_decimals: u8,      // Token1 decimal places
    pub pool_type: u8,            // CachePoolType enum discriminant
    pub fee_tier: u32,            // Fee in basis points (30 = 0.3%)
    pub venue: u16,               // VenueId enum
    pub discovered_at: u64,       // Unix timestamp when discovered
    pub last_seen: u64,           // Last time this pool had activity
}

/// Configuration for creating PoolInfoTLV
#[derive(Debug, Clone)]
pub struct PoolInfoConfig {
    pub pool_address: [u8; 20],
    pub token0_address: [u8; 20],
    pub token1_address: [u8; 20],
    pub token0_decimals: u8,
    pub token1_decimals: u8,
    pub pool_type: CachePoolType,
    pub fee_tier: u32,
    pub venue: VenueId,
    pub discovered_at: u64,
    pub last_seen: u64,
}

impl PoolInfoTLV {
    pub const TYPE: u8 = 200;
    pub const SIZE: usize = std::mem::size_of::<Self>(); // Actual size with padding

    /// Create a new pool info record from config
    pub fn new(config: PoolInfoConfig) -> Self {
        Self {
            tlv_type: Self::TYPE,
            tlv_length: (Self::SIZE - 2) as u8, // Exclude type and length fields
            pool_address: config.pool_address,
            token0_address: config.token0_address,
            token1_address: config.token1_address,
            token0_decimals: config.token0_decimals,
            token1_decimals: config.token1_decimals,
            pool_type: config.pool_type as u8,
            fee_tier: config.fee_tier,
            venue: config.venue as u16,
            discovered_at: config.discovered_at,
            last_seen: config.last_seen,
        }
    }

    /// Validate the TLV structure
    pub fn validate(&self) -> Result<(), String> {
        if self.tlv_type != Self::TYPE {
            return Err(format!(
                "Invalid TLV type: expected {}, got {}",
                Self::TYPE,
                self.tlv_type
            ));
        }

        if self.tlv_length != (Self::SIZE - 2) as u8 {
            return Err(format!(
                "Invalid TLV length: expected {}, got {}",
                Self::SIZE - 2,
                self.tlv_length
            ));
        }

        // Validate decimals are reasonable
        if self.token0_decimals > 30 || self.token1_decimals > 30 {
            return Err(format!(
                "Invalid decimals: token0={}, token1={}",
                self.token0_decimals, self.token1_decimals
            ));
        }

        // Validate fee tier is reasonable (0-10000 basis points = 0-100%)
        if self.fee_tier > 10000 {
            let fee_tier = self.fee_tier; // Copy to avoid packed field reference
            return Err(format!("Invalid fee tier: {} basis points", fee_tier));
        }

        Ok(())
    }
}

/// Pool cache file header (Not a TLV itself, but file metadata)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, AsBytes, FromBytes, FromZeroes)]
pub struct PoolCacheFileHeader {
    pub magic: [u8; 4],     // "POOL" magic bytes
    pub version: u32,       // Cache format version (currently 1)
    pub chain_id: u64,      // Blockchain chain ID (137 for Polygon)
    pub pool_count: u32,    // Number of pools in cache
    pub last_updated: u64,  // Unix timestamp of last update
    pub checksum: u32,      // CRC32 checksum of all pool data
    pub reserved: [u8; 32], // Reserved for future use
}

impl PoolCacheFileHeader {
    pub const MAGIC: [u8; 4] = *b"POOL";
    pub const VERSION: u32 = 1;
    pub const SIZE: usize = std::mem::size_of::<Self>();

    /// Create a new cache file header
    pub fn new(chain_id: u64, pool_count: u32, checksum: u32) -> Self {
        Self {
            magic: Self::MAGIC,
            version: Self::VERSION,
            chain_id,
            pool_count,
            last_updated: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            checksum,
            reserved: [0; 32],
        }
    }

    /// Validate the header
    pub fn validate(&self) -> Result<(), String> {
        if self.magic != Self::MAGIC {
            return Err(format!(
                "Invalid magic bytes: expected {:?}, got {:?}",
                Self::MAGIC,
                self.magic
            ));
        }

        if self.version != Self::VERSION {
            let version = self.version; // Copy to avoid packed field reference
            return Err(format!("Unsupported version: {}", version));
        }

        Ok(())
    }
}

/// Journal entry for incremental updates
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, AsBytes, FromBytes, FromZeroes)]
pub struct PoolCacheJournalEntry {
    pub operation: u8,          // 1=add, 2=update, 3=delete
    pub timestamp: u64,         // When this operation occurred
    pub pool_info: PoolInfoTLV, // The pool data
}

impl PoolCacheJournalEntry {
    pub const OP_ADD: u8 = 1;
    pub const OP_UPDATE: u8 = 2;
    pub const OP_DELETE: u8 = 3;
    pub const SIZE: usize = std::mem::size_of::<Self>();

    pub fn new_add(pool_info: PoolInfoTLV) -> Self {
        Self {
            operation: Self::OP_ADD,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            pool_info,
        }
    }

    pub fn new_update(pool_info: PoolInfoTLV) -> Self {
        Self {
            operation: Self::OP_UPDATE,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            pool_info,
        }
    }

    /// Validate the journal entry
    pub fn validate(&self) -> Result<(), String> {
        if self.operation < 1 || self.operation > 3 {
            return Err(format!("Invalid operation: {}", self.operation));
        }

        // Validate embedded pool info
        self.pool_info.validate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_info_tlv_size() {
        assert_eq!(std::mem::size_of::<PoolInfoTLV>(), PoolInfoTLV::SIZE);
    }

    #[test]
    fn test_pool_cache_header_size() {
        assert_eq!(
            std::mem::size_of::<PoolCacheFileHeader>(),
            PoolCacheFileHeader::SIZE
        );
    }

    #[test]
    fn test_pool_info_validation() {
        let pool_info = PoolInfoTLV::new(
            [0x45; 20], // pool address
            [0x11; 20], // token0
            [0x22; 20], // token1
            18,         // WETH decimals
            6,          // USDC decimals
            CachePoolType::UniswapV3,
            30, // 0.3% fee
            VenueId::Polygon,
            1700000000,
            1700000000,
        );

        assert!(pool_info.validate().is_ok());
    }

    #[test]
    fn test_header_validation() {
        let header = PoolCacheFileHeader::new(137, 1000, 0xDEADBEEF);
        assert!(header.validate().is_ok());

        // Test invalid magic
        let mut bad_header = header;
        bad_header.magic = *b"BADM";
        assert!(bad_header.validate().is_err());
    }

    #[test]
    fn test_journal_entry_creation() {
        let pool_info = PoolInfoTLV::new(
            [0x45; 20], // pool address
            [0x11; 20], // token0
            [0x22; 20], // token1
            18,         // WETH decimals
            6,          // USDC decimals
            CachePoolType::UniswapV3,
            30, // 0.3% fee
            VenueId::Polygon,
            1700000000,
            1700000000,
        );

        let journal_entry = PoolCacheJournalEntry::new_add(pool_info);
        assert_eq!(journal_entry.operation, PoolCacheJournalEntry::OP_ADD);
        assert!(journal_entry.validate().is_ok());
    }

    #[test]
    fn test_journal_entry_size() {
        // Verify the journal entry has the expected size
        assert_eq!(
            std::mem::size_of::<PoolCacheJournalEntry>(),
            PoolCacheJournalEntry::SIZE
        );

        // Size includes padding for struct alignment
        assert!(
            PoolCacheJournalEntry::SIZE >= 94,
            "Should be at least 94 bytes (1 + 8 + 85), actual: {}",
            PoolCacheJournalEntry::SIZE
        );
    }

    #[test]
    fn test_pool_info_bijection() {
        let original = PoolInfoTLV::new(
            [0x45; 20], // pool address
            [0x11; 20], // token0
            [0x22; 20], // token1
            18,         // WETH decimals
            6,          // USDC decimals
            CachePoolType::UniswapV3,
            30, // 0.3% fee
            VenueId::Polygon,
            1700000000,
            1700000000,
        );

        // Convert to bytes and back
        let bytes = original.as_bytes();
        let reconstructed = zerocopy::Ref::<_, PoolInfoTLV>::new_from_prefix(bytes)
            .unwrap()
            .0;

        // Should be identical
        assert_eq!(original.pool_address, reconstructed.pool_address);
        assert_eq!(original.token0_address, reconstructed.token0_address);
        assert_eq!(original.token1_address, reconstructed.token1_address);

        // Copy values from packed structs to avoid alignment issues
        let original_fee_tier = original.fee_tier;
        let reconstructed_fee_tier = reconstructed.fee_tier;
        assert_eq!(original_fee_tier, reconstructed_fee_tier);
    }
}

//! Pool Type Definitions
//!
//! This module provides pool type and protocol definitions used throughout the system.
//! The PoolInstrumentId system has been removed in favor of using full 20-byte addresses directly.

/// Different types of liquidity pools
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PoolType {
    TwoToken,   // Standard pair (Uniswap V2, etc.)
    Triangular, // Three-token pool (some Balancer pools)
    Weighted,   // Multi-token with custom weights
}

/// Pool protocol type (V2, V3, Curve, etc.)
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolProtocol {
    V2 = 0,        // Constant product (x*y=k)
    V3 = 1,        // Concentrated liquidity
    Curve = 2,     // StableSwap
    Balancer = 3,  // Weighted pools
    Unknown = 255, // Unknown protocol
}

/// Legacy functions for backward compatibility (deprecated)
/// These now return constant values since we no longer use hash-based IDs

/// Generate a canonical pool ID from two token asset_ids (DEPRECATED)
/// Returns a constant value - use full addresses instead
#[deprecated(note = "Use full 20-byte addresses instead of hash-based IDs")]
pub fn canonical_pool_id(_token0_asset_id: u64, _token1_asset_id: u64) -> u64 {
    0 // Constant return - use full addresses instead
}

/// Generate a canonical triangular pool ID from three token asset_ids (DEPRECATED)
#[deprecated(note = "Use full 20-byte addresses instead of hash-based IDs")]
pub fn canonical_triangular_pool_id(
    _token0_asset_id: u64,
    _token1_asset_id: u64,
    _token2_asset_id: u64,
) -> u64 {
    0 // Constant return - use full addresses instead
}

/// Pool metadata (legacy compatibility)
#[derive(Debug, Clone, PartialEq)]
pub struct PoolMetadata {
    pub token_ids: Vec<u64>,
    pub pool_type: PoolType,
}

impl PoolMetadata {
    /// Create pool metadata when you know the constituent tokens
    pub fn new(token_ids: Vec<u64>, pool_type: PoolType) -> Self {
        let mut sorted_tokens = token_ids;
        sorted_tokens.sort_unstable();
        sorted_tokens.dedup();
        PoolMetadata {
            token_ids: sorted_tokens,
            pool_type,
        }
    }

    /// Check if this pool contains a specific token
    pub fn contains_token(&self, token_asset_id: u64) -> bool {
        self.token_ids.binary_search(&token_asset_id).is_ok()
    }

    /// Get the other token(s) in the pool (excluding the specified one)
    pub fn other_tokens(&self, token_asset_id: u64) -> Vec<u64> {
        self.token_ids
            .iter()
            .copied()
            .filter(|&id| id != token_asset_id)
            .collect()
    }

    /// Extract pool metadata from a full InstrumentId (DEPRECATED)
    pub fn from_instrument_id(_instrument: &super::core::InstrumentId) -> Self {
        PoolMetadata {
            token_ids: vec![], // Cannot recover from hash
            pool_type: PoolType::TwoToken,
        }
    }

    /// Legacy method - cannot recover tokens from hash (DEPRECATED)
    pub fn from_pool_asset_id(_pool_asset_id: u64) -> Self {
        PoolMetadata {
            token_ids: vec![],
            pool_type: PoolType::TwoToken,
        }
    }
}

/// Hash function for distributing pool IDs across shards/partitions (DEPRECATED)
#[deprecated(note = "Use address-based sharding instead")]
pub fn pool_shard_hash(pool_asset_id: u64, num_shards: usize) -> usize {
    (pool_asset_id as usize) % num_shards
}

// Legacy Cantor pairing functions - kept for compatibility but deprecated

/// Legacy Cantor pairing (DEPRECATED - use full addresses instead)
#[deprecated(note = "Use full 20-byte addresses instead of hash-based IDs")]
pub fn cantor_pairing(_x: u64, _y: u64) -> u64 {
    0 // Constant return
}

/// Legacy inverse Cantor pairing (DEPRECATED - not truly bijective)
#[deprecated(note = "Cannot recover tokens from hash - use full addresses")]
pub fn inverse_cantor_pairing(_z: u64) -> (u64, u64) {
    // Cannot recover original values from hash
    (0, 0)
}

/// Legacy triple Cantor pairing (DEPRECATED)
#[deprecated(note = "Use full 20-byte addresses instead of hash-based IDs")]
pub fn cantor_pairing_triple(_x: u64, _y: u64, _z: u64) -> u64 {
    0 // Constant return
}

/// Legacy inverse triple pairing (DEPRECATED)
#[deprecated(note = "Cannot recover tokens from hash - use full addresses")]
pub fn inverse_cantor_pairing_triple(_w: u64) -> (u64, u64, u64) {
    // Cannot recover original values from hash
    (0, 0, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_metadata_creation() {
        let metadata = PoolMetadata::new(vec![1000, 2000], PoolType::TwoToken);

        assert_eq!(metadata.pool_type, PoolType::TwoToken);
        assert_eq!(metadata.token_ids.len(), 2);
        assert!(metadata.contains_token(1000));
        assert!(metadata.contains_token(2000));

        let others = metadata.other_tokens(1000);
        assert_eq!(others.len(), 1);
        assert_eq!(others[0], 2000);
    }

    #[test]
    fn test_triangular_pool_metadata() {
        let tri_metadata = PoolMetadata::new(vec![1000, 2000, 3000], PoolType::Triangular);

        assert_eq!(tri_metadata.pool_type, PoolType::Triangular);
        assert_eq!(tri_metadata.token_ids.len(), 3);
        assert!(tri_metadata.contains_token(1000));
        assert!(tri_metadata.contains_token(2000));
        assert!(tri_metadata.contains_token(3000));
    }

    #[test]
    fn test_legacy_functions() {
        // Test that deprecated functions don't panic
        assert_eq!(canonical_pool_id(1, 2), 0);
        assert_eq!(canonical_triangular_pool_id(1, 2, 3), 0);
        assert_eq!(cantor_pairing(1, 2), 0);
        assert_eq!(inverse_cantor_pairing(100), (0, 0));
        assert_eq!(cantor_pairing_triple(1, 2, 3), 0);
        assert_eq!(inverse_cantor_pairing_triple(100), (0, 0, 0));
    }
}

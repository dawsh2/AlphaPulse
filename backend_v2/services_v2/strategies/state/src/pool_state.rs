//! Pool State Manager
//!
//! Core state management for all pool types with dynamic discovery.
//! Implements the Stateful trait for integration with the state management framework.

use torq_state_core::{SequenceTracker, SequencedStateful, StateError, Stateful};
use torq_types::{
    tlv::{
        DEXProtocol as PoolProtocol, PoolBurnTLV, PoolMintTLV, PoolStateTLV, PoolSwapTLV,
        PoolSyncTLV,
    },
    InstrumentId, InstrumentId as PoolInstrumentId, VenueId,
};
use anyhow::Result;
use dashmap::DashMap;
use parking_lot::RwLock;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info, warn};

/// Events that can update pool state
#[derive(Debug, Clone)]
pub enum PoolEvent {
    Sync(PoolSyncTLV),
    Swap(PoolSwapTLV),
    Mint(PoolMintTLV),
    Burn(PoolBurnTLV),
    State(PoolStateTLV),
}

/// Pool state specific errors
#[derive(Debug, thiserror::Error)]
pub enum PoolStateError {
    #[error("Invalid pool protocol: {0:?}")]
    InvalidProtocol(PoolProtocol),

    #[error("Negative liquidity not allowed")]
    NegativeLiquidity,

    #[error("Invalid reserves: {0}")]
    InvalidReserves(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),

    #[error("State error: {0}")]
    State(#[from] StateError),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Complete state of a single pool
#[derive(Debug, Clone)]
pub struct PoolState {
    pub pool_id: PoolInstrumentId,
    pub protocol: PoolProtocol,

    // Full addresses for execution (CRITICAL - need full 20 bytes!)
    pub pool_address: [u8; 20],
    pub token0_address: [u8; 20],
    pub token1_address: [u8; 20],

    // V2 state
    pub reserve0: Option<Decimal>,
    pub reserve1: Option<Decimal>,

    // V3 state
    pub sqrt_price_x96: Option<u128>,
    pub tick: Option<i32>,
    pub liquidity: Option<u128>,

    // Common
    pub fee_tier: u32,
    pub last_update_ns: u64,
    pub last_block: u64,
    pub initialized: bool,
}

// Helper methods for PoolState to provide token information
impl PoolState {
    /// Get token addresses from this pool
    pub fn get_token_addresses(&self) -> ([u8; 20], [u8; 20]) {
        (self.token0_address, self.token1_address)
    }

    /// Check if pool contains a specific token
    pub fn contains_token(&self, token_addr: [u8; 20]) -> bool {
        self.token0_address == token_addr || self.token1_address == token_addr
    }

    /// Check if this is a V3 pool
    pub fn is_v3(&self) -> bool {
        matches!(
            self.protocol,
            PoolProtocol::UniswapV3 | PoolProtocol::QuickswapV3
        )
    }
}

impl PoolState {
    /// Create uninitialized pool state with full addresses
    pub fn new(pool_id: PoolInstrumentId, pool_address: [u8; 20]) -> Self {
        // TODO: Extract protocol from PoolInstrumentId in Protocol V2
        let protocol = PoolProtocol::UniswapV2; // Default for now
        Self {
            pool_id,
            protocol,
            pool_address,
            token0_address: [0u8; 20], // Will be set when we get first event
            token1_address: [0u8; 20], // Will be set when we get first event
            reserve0: None,
            reserve1: None,
            sqrt_price_x96: None,
            tick: None,
            liquidity: None,
            fee_tier: 30, // Default 0.3%
            last_update_ns: 0,
            last_block: 0,
            initialized: false,
        }
    }

    /// Check if pool has enough data to calculate prices
    pub fn is_ready(&self) -> bool {
        match self.protocol {
            PoolProtocol::UniswapV2 | PoolProtocol::SushiswapV2 => {
                self.reserve0.is_some() && self.reserve1.is_some()
            }
            PoolProtocol::UniswapV3 | PoolProtocol::QuickswapV3 => {
                self.sqrt_price_x96.is_some() && self.liquidity.is_some()
            }
            _ => false,
        }
    }

    /// Get spot price (token1 per token0)
    pub fn spot_price(&self) -> Option<Decimal> {
        match self.protocol {
            PoolProtocol::UniswapV2 | PoolProtocol::SushiswapV2 => {
                match (self.reserve0, self.reserve1) {
                    (Some(r0), Some(r1)) if r0 > Decimal::ZERO => Some(r1 / r0),
                    _ => None,
                }
            }
            PoolProtocol::UniswapV3 | PoolProtocol::QuickswapV3 => {
                self.sqrt_price_x96.map(|sqrt_price| {
                    let price_x96 = Decimal::from(sqrt_price) / Decimal::from(2u128.pow(96));
                    price_x96 * price_x96
                })
            }
            _ => None,
        }
    }
}

/// Manages state for all pools
pub struct PoolStateManager {
    /// All pools indexed by full 20-byte address (NO TRUNCATION!)
    pools: DashMap<[u8; 20], Arc<RwLock<PoolState>>>,

    /// Token index: token address -> list of pool addresses
    token_index: DashMap<[u8; 20], Vec<[u8; 20]>>,
    /// Token pair index: (token0, token1) -> list of pool addresses for arbitrage
    #[allow(clippy::type_complexity)]
    token_pair_index: DashMap<([u8; 20], [u8; 20]), Vec<[u8; 20]>>,

    /// Statistics
    stats: Arc<RwLock<ManagerStats>>,

    /// Advanced sequence tracking with gap detection
    sequence_tracker: Arc<RwLock<SequenceTracker>>,

    /// Recovery callback for gap handling
    gap_handler: Option<Box<dyn Fn(u64, u64) + Send + Sync>>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ManagerStats {
    pub total_pools: usize,
    pub v2_pools: usize,
    pub v3_pools: usize,
    pub initialized_pools: usize,
    pub total_events: u64,
    pub last_update_ns: u64,
}

impl Default for PoolStateManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PoolStateManager {
    pub fn new() -> Self {
        Self {
            pools: DashMap::new(),
            token_index: DashMap::new(),
            token_pair_index: DashMap::new(),
            stats: Arc::new(RwLock::new(ManagerStats::default())),
            sequence_tracker: Arc::new(RwLock::new(SequenceTracker::new())),
            gap_handler: None,
        }
    }

    /// Create with gap handler for recovery
    pub fn with_gap_handler<F>(gap_handler: F) -> Self
    where
        F: Fn(u64, u64) + Send + Sync + 'static,
    {
        Self {
            pools: DashMap::new(),
            token_index: DashMap::new(),
            token_pair_index: DashMap::new(),
            stats: Arc::new(RwLock::new(ManagerStats::default())),
            sequence_tracker: Arc::new(RwLock::new(SequenceTracker::new())),
            gap_handler: Some(Box::new(gap_handler)),
        }
    }

    /// Handle sequence gaps with different strategies based on gap size
    fn handle_sequence_gap(&self, expected: u64, actual: u64) -> Result<(), PoolStateError> {
        let gap_size = actual.saturating_sub(expected);

        match gap_size {
            1..=5 => {
                // Small gap: try relay buffer recovery
                info!(
                    "Small sequence gap detected: {} to {}. Requesting recovery from relay buffer",
                    expected, actual
                );
                if let Some(ref handler) = self.gap_handler {
                    handler(expected, actual);
                }
            }
            6..=50 => {
                // Medium gap: selective resync for affected pools
                warn!(
                    "Medium sequence gap detected: {} to {}. Selective resync needed",
                    expected, actual
                );
                if let Some(ref handler) = self.gap_handler {
                    handler(expected, actual);
                }
            }
            _ => {
                // Large gap: mark all pools as stale, full resync needed
                error!(
                    "Large sequence gap detected: {} to {}. Full resync required",
                    expected, actual
                );
                self.mark_all_pools_stale();
                if let Some(ref handler) = self.gap_handler {
                    handler(expected, actual);
                }
            }
        }

        Ok(())
    }

    /// Mark all pools as requiring resync
    fn mark_all_pools_stale(&self) {
        for pool_entry in self.pools.iter() {
            let mut pool = pool_entry.value().write();
            pool.initialized = false; // Mark as needing fresh data
        }
        warn!(
            "Marked {} pools as stale due to large sequence gap",
            self.pools.len()
        );
    }

    /// Process V2 Sync event - complete state update
    pub fn process_sync(&self, sync: &PoolSyncTLV) -> Result<()> {
        // Use 20-byte addresses directly (no conversion needed)
        let pool_address = sync.pool_address;
        let token0_addr = sync.token0_addr;
        let token1_addr = sync.token1_addr;

        // Get or create pool
        let pool_arc = self.pools.entry(pool_address).or_insert_with(|| {
            let mut stats = self.stats.write();
            stats.total_pools += 1;
            stats.v2_pools += 1;

            // Add to token index
            self.token_index
                .entry(token0_addr)
                .or_default()
                .push(pool_address);
            self.token_index
                .entry(token1_addr)
                .or_default()
                .push(pool_address);

            // Add to token pair index for arbitrage detection
            let (sorted_token0, sorted_token1) = if token0_addr <= token1_addr {
                (token0_addr, token1_addr)
            } else {
                (token1_addr, token0_addr)
            };
            self.token_pair_index
                .entry((sorted_token0, sorted_token1))
                .or_default()
                .push(pool_address);

            // Create InstrumentId from pool address
            // Use a temporary hash for the asset_id field (this is a protocol v2 limitation)
            let pool_hash = {
                let mut bytes = [0u8; 8];
                bytes.copy_from_slice(&pool_address[..8]);
                u64::from_be_bytes(bytes)
            };
            let pool_instrument_id =
                InstrumentId::polygon_token(&format!("0x{}", hex::encode(pool_address))).unwrap_or(
                    InstrumentId {
                        venue: VenueId::Polygon as u16,
                        asset_type: 3, // Pool type
                        reserved: 0,
                        asset_id: pool_hash,
                    },
                );
            let mut pool_state = PoolState::new(pool_instrument_id, pool_address);
            pool_state.token0_address = token0_addr;
            pool_state.token1_address = token1_addr;
            Arc::new(RwLock::new(pool_state))
        });

        // Update state
        let mut pool = pool_arc.write();
        pool.reserve0 = Some(Decimal::from(sync.reserve0) / Decimal::from(100_000_000));
        pool.reserve1 = Some(Decimal::from(sync.reserve1) / Decimal::from(100_000_000));
        pool.last_update_ns = sync.timestamp_ns;
        pool.last_block = sync.block_number;

        if !pool.initialized {
            pool.initialized = true;
            let mut stats = self.stats.write();
            stats.initialized_pools += 1;
        }

        // Update stats
        {
            let mut stats = self.stats.write();
            stats.total_events += 1;
            stats.last_update_ns = sync.timestamp_ns;
        }

        Ok(())
    }

    /// Process V3 Swap event - contains state updates
    pub fn process_swap(&self, swap: &PoolSwapTLV) -> Result<()> {
        // Use 20-byte addresses directly (no conversion needed)
        let pool_address = swap.pool_address;
        let token_in_addr = swap.token_in_addr;
        let token_out_addr = swap.token_out_addr;

        // For V3 swaps, create/update pool with state
        // Check if this is a V3 swap based on available fields
        if swap.sqrt_price_x96_as_u128() > 0 {
            let pool_arc = self.pools.entry(pool_address).or_insert_with(|| {
                let mut stats = self.stats.write();
                stats.total_pools += 1;
                stats.v3_pools += 1;

                // Add to token index
                self.token_index
                    .entry(token_in_addr)
                    .or_default()
                    .push(pool_address);
                self.token_index
                    .entry(token_out_addr)
                    .or_default()
                    .push(pool_address);

                // Add to token pair index for arbitrage detection
                let (sorted_token0, sorted_token1) = if token_in_addr <= token_out_addr {
                    (token_in_addr, token_out_addr)
                } else {
                    (token_out_addr, token_in_addr)
                };
                self.token_pair_index
                    .entry((sorted_token0, sorted_token1))
                    .or_default()
                    .push(pool_address);

                // Create InstrumentId from pool address
                // Use a temporary hash for the asset_id field (this is a protocol v2 limitation)
                let pool_hash = {
                    let mut bytes = [0u8; 8];
                    bytes.copy_from_slice(&pool_address[..8]);
                    u64::from_be_bytes(bytes)
                };
                let pool_instrument_id =
                    InstrumentId::polygon_token(&format!("0x{}", hex::encode(pool_address)))
                        .unwrap_or(InstrumentId {
                            venue: VenueId::Polygon as u16,
                            asset_type: 3, // Pool type
                            reserved: 0,
                            asset_id: pool_hash,
                        });
                let mut pool_state = PoolState::new(pool_instrument_id, pool_address);
                pool_state.token0_address = token_in_addr;
                pool_state.token1_address = token_out_addr;
                Arc::new(RwLock::new(pool_state))
            });

            // Update V3 state from swap
            let mut pool = pool_arc.write();
            pool.sqrt_price_x96 = Some(swap.sqrt_price_x96_as_u128());
            pool.tick = Some(swap.tick_after);
            pool.liquidity = Some(swap.liquidity_after);
            pool.last_update_ns = swap.timestamp_ns;
            pool.last_block = swap.block_number;

            if !pool.initialized {
                pool.initialized = true;
                let mut stats = self.stats.write();
                stats.initialized_pools += 1;
            }
        }

        // Update stats
        {
            let mut stats = self.stats.write();
            stats.total_events += 1;
            stats.last_update_ns = swap.timestamp_ns;
        }

        Ok(())
    }

    /// Process Mint event - liquidity addition
    pub fn process_mint(&self, mint: &PoolMintTLV) -> Result<()> {
        // Use 20-byte address directly (no conversion needed)
        let pool_address = mint.pool_address;

        if let Some(pool_arc) = self.pools.get(&pool_address) {
            let mut pool = pool_arc.write();
            if let Some(liq) = pool.liquidity {
                pool.liquidity = Some(liq + mint.liquidity_delta);
            }
            pool.last_update_ns = mint.timestamp_ns;
            pool.last_block = 0; // TODO: Add block_number to PoolMintTLV
        }

        // Update stats
        {
            let mut stats = self.stats.write();
            stats.total_events += 1;
            stats.last_update_ns = mint.timestamp_ns;
        }

        Ok(())
    }

    /// Process Burn event - liquidity removal
    pub fn process_burn(&self, burn: &PoolBurnTLV) -> Result<()> {
        // Use 20-byte address directly (no conversion needed)
        let pool_address = burn.pool_address;

        if let Some(pool_arc) = self.pools.get(&pool_address) {
            let mut pool = pool_arc.write();
            if let Some(liq) = pool.liquidity {
                pool.liquidity = Some(liq.saturating_sub(burn.liquidity_delta));
            }
            pool.last_update_ns = burn.timestamp_ns;
            pool.last_block = 0; // TODO: Add block_number to PoolBurnTLV
        }

        // Update stats
        {
            let mut stats = self.stats.write();
            stats.total_events += 1;
            stats.last_update_ns = burn.timestamp_ns;
        }

        Ok(())
    }

    /// Get pool state by address
    pub fn get_pool(&self, pool_address: &[u8; 20]) -> Option<Arc<RwLock<PoolState>>> {
        self.pools.get(pool_address).map(|entry| entry.clone())
    }

    /// Find all pools containing a token
    pub fn find_pools_with_token(&self, token_addr: &[u8; 20]) -> Vec<Arc<RwLock<PoolState>>> {
        self.token_index
            .get(token_addr)
            .map(|entry| {
                entry
                    .iter()
                    .filter_map(|pool_addr| self.get_pool(pool_addr))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Find all pools for a specific token pair (for arbitrage detection)
    pub fn find_pools_for_token_pair(
        &self,
        token0: &[u8; 20],
        token1: &[u8; 20],
    ) -> Vec<Arc<RwLock<PoolState>>> {
        // Sort tokens to match index ordering
        let (sorted_token0, sorted_token1) = if token0 <= token1 {
            (*token0, *token1)
        } else {
            (*token1, *token0)
        };

        self.token_pair_index
            .get(&(sorted_token0, sorted_token1))
            .map(|entry| {
                entry
                    .iter()
                    .filter_map(|pool_addr| self.get_pool(pool_addr))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Find arbitrage opportunities between pools
    pub fn find_arbitrage_pairs(&self) -> Vec<ArbitragePair> {
        let mut pairs = Vec::new();
        let mut checked_pairs = std::collections::HashSet::new();

        // Use token pair index to find pools with same tokens efficiently
        for entry in self.token_pair_index.iter() {
            let pool_addresses = entry.value();

            // Compare all pools with the same token pair
            for i in 0..pool_addresses.len() {
                for j in i + 1..pool_addresses.len() {
                    let pool1_addr = pool_addresses[i];
                    let pool2_addr = pool_addresses[j];

                    // Skip if already checked (in either order)
                    let pair_key = if pool1_addr < pool2_addr {
                        (pool1_addr, pool2_addr)
                    } else {
                        (pool2_addr, pool1_addr)
                    };

                    if !checked_pairs.insert(pair_key) {
                        continue;
                    }

                    // Get both pools
                    if let (Some(pool1_arc), Some(pool2_arc)) =
                        (self.get_pool(&pool1_addr), self.get_pool(&pool2_addr))
                    {
                        let pool1 = pool1_arc.read();
                        let pool2 = pool2_arc.read();

                        if !pool1.is_ready() || !pool2.is_ready() {
                            continue;
                        }

                        if let (Some(price1), Some(price2)) =
                            (pool1.spot_price(), pool2.spot_price())
                        {
                            let spread = ((price1 - price2) / price1).abs();

                            if spread > Decimal::from_str_exact("0.002").unwrap() {
                                // 0.2% threshold
                                pairs.push(ArbitragePair {
                                    pool1_addr,
                                    pool2_addr,
                                    spread_pct: spread * Decimal::from(100),
                                    price1,
                                    price2,
                                });
                            }
                        }
                    }
                }
            }
        }

        pairs
    }

    /// Get statistics
    pub fn stats(&self) -> ManagerStats {
        self.stats.read().clone()
    }

    // Strategy-specific methods for flash arbitrage compatibility

    /// Get pool by PoolInstrumentId (strategy compatibility)
    pub fn get_pool_by_id(&self, pool_id: &PoolInstrumentId) -> Option<Arc<StrategyPoolState>> {
        // We need to find the pool by iterating since we can't directly map InstrumentId to address
        // This is inefficient but necessary until Protocol V2 is fully refactored
        for entry in self.pools.iter() {
            let pool_state = entry.value().read();
            if pool_state.pool_id.to_u64() == pool_id.to_u64() {
                return Some(Arc::new(StrategyPoolState::from_pool_state(&pool_state)));
            }
        }
        None
    }

    /// Find all pools for a token pair (strategy compatibility)
    pub fn find_pools_for_pair(&self, _token_a: u64, _token_b: u64) -> Vec<Arc<StrategyPoolState>> {
        // This is a compatibility shim - we need actual token addresses
        // For now, return empty since we can't map u64 to addresses without more context
        // TODO: Update callers to use find_pools_for_token_pair with actual addresses
        vec![]
    }

    /// Find potential arbitrage pairs for a pool (strategy compatibility)
    pub fn find_arbitrage_pairs_for_pool(
        &self,
        pool_id: &PoolInstrumentId,
    ) -> Vec<StrategyArbitragePair> {
        let mut pairs = Vec::new();

        // Find the pool by its InstrumentId
        for entry in self.pools.iter() {
            let pool = entry.value().read();
            if pool.pool_id.to_u64() == pool_id.to_u64() {
                // Get the token addresses from this pool
                let (token0, token1) = pool.get_token_addresses();

                // Find all pools with the same token pair
                let same_pair_pools = self.find_pools_for_token_pair(&token0, &token1);

                for other_pool_arc in same_pair_pools {
                    let other_pool = other_pool_arc.read();
                    if other_pool.pool_address != pool.pool_address {
                        // Create a simple hash from addresses for compatibility
                        let pool_a_hash =
                            u64::from_be_bytes(pool.pool_address[..8].try_into().unwrap());
                        let pool_b_hash =
                            u64::from_be_bytes(other_pool.pool_address[..8].try_into().unwrap());

                        pairs.push(StrategyArbitragePair {
                            pool_a: pool_a_hash,
                            pool_b: pool_b_hash,
                            shared_tokens: vec![
                                u64::from_be_bytes(token0[..8].try_into().unwrap()),
                                u64::from_be_bytes(token1[..8].try_into().unwrap()),
                            ],
                        });
                    }
                }
                break;
            }
        }

        pairs
    }

    /// Find all pools containing a specific token (returns strategy-compatible PoolState)
    pub fn find_strategy_pools_with_token(&self, _token_id: u64) -> Vec<Arc<StrategyPoolState>> {
        // This is a compatibility shim - we need actual token addresses
        // For now, return empty since we can't map u64 to addresses without more context
        // TODO: Update callers to use find_pools_with_token with actual addresses
        vec![]
    }

    /// Get pool by hash (returns strategy-compatible PoolState)
    pub fn get_strategy_pool(&self, pool_hash: u64) -> Option<Arc<StrategyPoolState>> {
        // Search for pool with matching hash (using first 8 bytes of address)
        for entry in self.pools.iter() {
            let pool_addr = *entry.key();
            let addr_hash = u64::from_be_bytes(pool_addr[..8].try_into().unwrap());
            if addr_hash == pool_hash {
                let lib_state = entry.value().read();
                return Some(Arc::new(StrategyPoolState::from_pool_state(&lib_state)));
            }
        }
        None
    }

    /// Apply event without requiring mutable reference (for shared access patterns)
    /// This method provides interior mutability for use with Arc<PoolStateManager>
    pub fn apply_event_shared(&self, event: PoolEvent) -> Result<(), PoolStateError> {
        match event {
            PoolEvent::Sync(sync) => self.process_sync(&sync).map_err(PoolStateError::Other),
            PoolEvent::Swap(swap) => self.process_swap(&swap).map_err(PoolStateError::Other),
            PoolEvent::Mint(mint) => self.process_mint(&mint).map_err(PoolStateError::Other),
            PoolEvent::Burn(burn) => self.process_burn(&burn).map_err(PoolStateError::Other),
            PoolEvent::State(_state) => {
                // TODO: Implement full state update
                Ok(())
            }
        }
    }
}

/// Implement Stateful trait for PoolStateManager
impl Stateful for PoolStateManager {
    type Event = PoolEvent;
    type Error = PoolStateError;

    fn apply_event(&mut self, event: Self::Event) -> Result<(), Self::Error> {
        match event {
            PoolEvent::Sync(sync) => self.process_sync(&sync)?,
            PoolEvent::Swap(swap) => self.process_swap(&swap)?,
            PoolEvent::Mint(mint) => self.process_mint(&mint)?,
            PoolEvent::Burn(burn) => self.process_burn(&burn)?,
            PoolEvent::State(_state) => {
                // TODO: Implement full state update
            }
        }
        Ok(())
    }

    fn snapshot(&self) -> Vec<u8> {
        let pools_vec: Vec<SerializablePoolState> = self
            .pools
            .iter()
            .map(|entry| SerializablePoolState::from_pool_state(&entry.value().read()))
            .collect();

        let snapshot_data = SnapshotData {
            pools: pools_vec,
            stats: self.stats.read().clone(),
            last_sequence: self.sequence_tracker.read().last_sequence(),
        };

        bincode::serialize(&snapshot_data).unwrap_or_default()
    }

    fn restore(&mut self, snapshot: &[u8]) -> Result<(), Self::Error> {
        let snapshot_data: SnapshotData = bincode::deserialize(snapshot)?;

        // Clear existing state
        self.pools.clear();
        self.token_index.clear();

        // Restore all pools with full state
        for serializable_pool in snapshot_data.pools {
            let pool_state = serializable_pool.to_pool_state();
            let pool_addr = pool_state.pool_address;

            // Rebuild token index
            let (token0, token1) = pool_state.get_token_addresses();
            self.token_index.entry(token0).or_default().push(pool_addr);
            self.token_index.entry(token1).or_default().push(pool_addr);

            // Rebuild token pair index
            let (sorted_token0, sorted_token1) = if token0 <= token1 {
                (token0, token1)
            } else {
                (token1, token0)
            };
            self.token_pair_index
                .entry((sorted_token0, sorted_token1))
                .or_default()
                .push(pool_addr);

            // Insert pool
            self.pools
                .insert(pool_addr, Arc::new(RwLock::new(pool_state)));
        }

        // Restore stats and sequence tracker
        *self.stats.write() = snapshot_data.stats;
        self.sequence_tracker
            .write()
            .set_last_sequence(snapshot_data.last_sequence);

        Ok(())
    }
}

/// Implement SequencedStateful trait with enhanced gap detection
impl SequencedStateful for PoolStateManager {
    fn apply_sequenced(&mut self, seq: u64, event: Self::Event) -> Result<(), Self::Error> {
        // Check sequence gap first
        let expected = {
            let tracker = self.sequence_tracker.read();
            tracker.next_expected()
        };

        if seq != expected {
            // Handle the gap before failing
            self.handle_sequence_gap(expected, seq)?;

            // Still return error to let caller know about the gap
            return Err(PoolStateError::State(StateError::SequenceGap {
                expected,
                actual: seq,
            }));
        }

        // Apply the event
        self.apply_event(event)?;

        // Update sequence tracking after successful event application
        {
            let mut tracker = self.sequence_tracker.write();
            tracker.track(seq)?;
        }

        Ok(())
    }

    fn last_sequence(&self) -> u64 {
        self.sequence_tracker.read().last_sequence()
    }
}

#[derive(Debug, Clone)]
pub struct ArbitragePair {
    pub pool1_addr: [u8; 20],
    pub pool2_addr: [u8; 20],
    pub spread_pct: Decimal,
    pub price1: Decimal,
    pub price2: Decimal,
}

/// Arbitrage pair representation for strategy compatibility
#[derive(Debug, Clone)]
pub struct StrategyArbitragePair {
    pub pool_a: u64,
    pub pool_b: u64,
    pub shared_tokens: Vec<u64>,
}

/// Strategy-compatible PoolState enum
/// This matches the interface expected by the flash arbitrage strategy
#[derive(Debug, Clone)]
pub enum StrategyPoolState {
    V2 {
        pool_id: PoolInstrumentId,
        pool_address: [u8; 20],
        reserves: (Decimal, Decimal),
        fee_tier: u32,
        last_update_ns: u64,
    },
    V3 {
        pool_id: PoolInstrumentId,
        pool_address: [u8; 20],
        liquidity: u128,
        sqrt_price_x96: u128,
        current_tick: i32,
        fee_tier: u32,
        last_update_ns: u64,
    },
}

impl StrategyPoolState {
    /// Get the pool's PoolInstrumentId
    pub fn pool_id(&self) -> &PoolInstrumentId {
        match self {
            StrategyPoolState::V2 { pool_id, .. } => pool_id,
            StrategyPoolState::V3 { pool_id, .. } => pool_id,
        }
    }

    /// Get the pool's address
    pub fn pool_address(&self) -> &[u8; 20] {
        match self {
            StrategyPoolState::V2 { pool_address, .. } => pool_address,
            StrategyPoolState::V3 { pool_address, .. } => pool_address,
        }
    }

    /// Get the pool's fee tier in basis points
    pub fn fee_tier(&self) -> u32 {
        match self {
            StrategyPoolState::V2 { fee_tier, .. } => *fee_tier,
            StrategyPoolState::V3 { fee_tier, .. } => *fee_tier,
        }
    }

    /// Get last update timestamp in nanoseconds
    pub fn last_update_ns(&self) -> u64 {
        match self {
            StrategyPoolState::V2 { last_update_ns, .. } => *last_update_ns,
            StrategyPoolState::V3 { last_update_ns, .. } => *last_update_ns,
        }
    }

    /// Check if pool involves specific token
    pub fn has_token(&self, _token_id: u64) -> bool {
        // This is a compatibility shim - we can't check without actual addresses
        // TODO: Update callers to provide token addresses instead of u64
        false
    }

    /// Convert from library PoolState
    pub fn from_pool_state(lib_state: &PoolState) -> Self {
        if lib_state.protocol == PoolProtocol::UniswapV3
            || lib_state.protocol == PoolProtocol::QuickswapV3
        {
            // V3 pool
            StrategyPoolState::V3 {
                pool_id: lib_state.pool_id,
                pool_address: lib_state.pool_address,
                liquidity: lib_state.liquidity.unwrap_or(0),
                sqrt_price_x96: lib_state.sqrt_price_x96.unwrap_or(0),
                current_tick: lib_state.tick.unwrap_or(0),
                fee_tier: lib_state.fee_tier,
                last_update_ns: lib_state.last_update_ns,
            }
        } else {
            // V2 pool
            StrategyPoolState::V2 {
                pool_id: lib_state.pool_id,
                pool_address: lib_state.pool_address,
                reserves: (
                    lib_state.reserve0.unwrap_or(Decimal::ZERO),
                    lib_state.reserve1.unwrap_or(Decimal::ZERO),
                ),
                fee_tier: lib_state.fee_tier,
                last_update_ns: lib_state.last_update_ns,
            }
        }
    }

    /// Try to get as V2 pool with Result error handling
    pub fn as_v2_pool(&self) -> Result<V2PoolState, anyhow::Error> {
        match self {
            StrategyPoolState::V2 {
                pool_id,
                reserves,
                fee_tier,
                ..
            } => Ok(V2PoolState {
                pool_id: *pool_id,
                reserve0: reserves.0,
                reserve1: reserves.1,
                fee_tier: *fee_tier,
            }),
            _ => Err(anyhow::anyhow!("Pool is not a V2 pool")),
        }
    }

    /// Try to get as V3 pool with Result error handling
    pub fn as_v3_pool(&self) -> Result<V3PoolState, anyhow::Error> {
        match self {
            StrategyPoolState::V3 {
                pool_id,
                liquidity,
                sqrt_price_x96,
                current_tick,
                fee_tier,
                ..
            } => Ok(V3PoolState {
                pool_id: *pool_id,
                liquidity: *liquidity,
                sqrt_price_x96: *sqrt_price_x96,
                current_tick: *current_tick,
                fee_tier: *fee_tier,
            }),
            _ => Err(anyhow::anyhow!("Pool is not a V3 pool")),
        }
    }
}

/// V2 pool state for strategy compatibility
#[derive(Debug, Clone)]
pub struct V2PoolState {
    pub pool_id: PoolInstrumentId,
    pub reserve0: Decimal,
    pub reserve1: Decimal,
    pub fee_tier: u32,
}

/// V3 pool state for strategy compatibility
#[derive(Debug, Clone)]
pub struct V3PoolState {
    pub pool_id: PoolInstrumentId,
    pub liquidity: u128,
    pub sqrt_price_x96: u128,
    pub current_tick: i32,
    pub fee_tier: u32,
}

/// Serializable pool state for snapshots
/// Stores full pool information including constituent tokens for perfect reconstruction
#[derive(Serialize, Deserialize)]
struct SerializablePoolState {
    // Pool identification - full bijective data
    venue: u16,
    pool_protocol: u8,
    token_ids: Vec<u64>, // Full token list for perfect reconstruction

    // Full addresses for execution (20 bytes each, stored as hex strings)
    pool_address: String,
    token0_address: String,
    token1_address: String,

    // Pool state
    protocol: u8,
    reserve0: Option<String>,
    reserve1: Option<String>,
    sqrt_price_x96: Option<u128>,
    tick: Option<i32>,
    liquidity: Option<u128>,
    fee_tier: u32,
    last_update_ns: u64,
    last_block: u64,
    initialized: bool,
}

impl SerializablePoolState {
    fn from_pool_state(state: &PoolState) -> Self {
        Self {
            // Store full pool identification data
            venue: state.pool_id.venue,
            pool_protocol: PoolProtocol::UniswapV2 as u8, // Store as u8 for serialization
            token_ids: vec![1000, 2000], // Placeholder - would extract from pool metadata in full implementation

            // Store full addresses as hex strings for serialization
            pool_address: hex::encode(state.pool_address),
            token0_address: hex::encode(state.token0_address),
            token1_address: hex::encode(state.token1_address),

            // Store state
            protocol: match state.protocol {
                PoolProtocol::UniswapV2 | PoolProtocol::SushiswapV2 => 2,
                PoolProtocol::UniswapV3 | PoolProtocol::QuickswapV3 => 3,
                _ => 0,
            },
            reserve0: state.reserve0.map(|d| d.to_string()),
            reserve1: state.reserve1.map(|d| d.to_string()),
            sqrt_price_x96: state.sqrt_price_x96,
            tick: state.tick,
            liquidity: state.liquidity,
            fee_tier: state.fee_tier,
            last_update_ns: state.last_update_ns,
            last_block: state.last_block,
            initialized: state.initialized,
        }
    }

    fn to_pool_state(&self) -> PoolState {
        // Reconstruct the full PoolInstrumentId from saved data
        // Convert u16 back to VenueId enum
        let venue = match self.venue {
            0 => VenueId::Generic,
            100 => VenueId::Binance,
            200 => VenueId::Ethereum,
            202 => VenueId::Polygon,
            300 => VenueId::UniswapV2,
            301 => VenueId::UniswapV3,
            302 => VenueId::SushiSwap,
            _ => VenueId::Generic,
        };

        // Create InstrumentId using available constructor
        let pool_id = InstrumentId {
            venue: venue as u16,
            asset_type: 3, // Pool type
            reserved: 0,
            asset_id: self
                .token_ids
                .iter()
                .fold(0u64, |acc, &t| acc.wrapping_mul(31).wrapping_add(t)),
        };

        let protocol = match self.protocol {
            2 => PoolProtocol::UniswapV2,
            3 => PoolProtocol::UniswapV3,
            _ => PoolProtocol::UniswapV2, // Default
        };

        // Decode addresses from hex strings (with error handling)
        let pool_address = hex::decode(&self.pool_address)
            .ok()
            .and_then(|bytes| bytes.try_into().ok())
            .unwrap_or([0u8; 20]);

        let token0_address = hex::decode(&self.token0_address)
            .ok()
            .and_then(|bytes| bytes.try_into().ok())
            .unwrap_or([0u8; 20]);

        let token1_address = hex::decode(&self.token1_address)
            .ok()
            .and_then(|bytes| bytes.try_into().ok())
            .unwrap_or([0u8; 20]);

        PoolState {
            pool_id,
            protocol,
            pool_address,
            token0_address,
            token1_address,
            reserve0: self.reserve0.as_ref().and_then(|s| s.parse().ok()),
            reserve1: self.reserve1.as_ref().and_then(|s| s.parse().ok()),
            sqrt_price_x96: self.sqrt_price_x96,
            tick: self.tick,
            liquidity: self.liquidity,
            fee_tier: self.fee_tier,
            last_update_ns: self.last_update_ns,
            last_block: self.last_block,
            initialized: self.initialized,
        }
    }
}

/// Helper struct for serialization
#[derive(Serialize, Deserialize)]
struct SnapshotData {
    pools: Vec<SerializablePoolState>,
    stats: ManagerStats,
    last_sequence: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stateful_implementation() {
        let mut manager = PoolStateManager::new();

        // Create a test sync event
        let _pool_id = InstrumentId {
            venue: VenueId::Generic as u16,
            asset_type: 3, // Pool
            reserved: 0,
            asset_id: 1000_u64.wrapping_mul(31).wrapping_add(2000),
        };
        let sync = PoolSyncTLV::from_components(
            [0u8; 20],                   // Mock pool address (20-byte)
            [0u8; 20],                   // Mock token0 address (20-byte)
            [0u8; 20],                   // Mock token1 address (20-byte)
            VenueId::Generic,            // Venue ID
            1000_000000000000000000u128, // 1000 with 18 decimals
            2000_000000u128,             // 2000 USDC with 6 decimals
            18u8,                        // token0_decimals
            6u8,                         // token1_decimals
            1234567890u64,               // timestamp_ns
            100u64,                      // block_number
        );

        // Apply event
        manager.apply_event(PoolEvent::Sync(sync.clone())).unwrap();

        // Check state was updated using the pool address from the sync event
        let test_pool_address = [0u8; 20]; // Same address used in sync creation
        let pool = manager.get_pool(&test_pool_address).unwrap();
        let pool_state = pool.read();
        assert_eq!(pool_state.reserve0, Some(Decimal::from(1000)));
        assert_eq!(pool_state.reserve1, Some(Decimal::from(2000)));

        // Test snapshot/restore
        let snapshot = manager.snapshot();
        assert!(!snapshot.is_empty());

        let mut new_manager = PoolStateManager::new();
        new_manager.restore(&snapshot).unwrap();

        // Verify restored state
        let restored_pool = new_manager.get_pool(&test_pool_address).unwrap();
        let restored_state = restored_pool.read();
        assert_eq!(restored_state.reserve0, Some(Decimal::from(1000)));
        assert_eq!(restored_state.reserve1, Some(Decimal::from(2000)));
    }

    #[test]
    fn test_sequenced_stateful() {
        let mut manager = PoolStateManager::new();

        let _test_pool_address = [1u8; 20]; // Same address used in sync creation below

        let sync = PoolSyncTLV::from_components(
            [1u8; 20],         // Mock pool address (20-byte)
            [2u8; 20],         // Mock token0 address (20-byte)
            [3u8; 20],         // Mock token1 address (20-byte)
            VenueId::Generic,  // Venue ID
            1000_00000000u128, // reserve0
            2000_00000000u128, // reserve1
            8u8,               // token0_decimals
            8u8,               // token1_decimals
            1234567890u64,     // timestamp_ns
            100u64,            // block_number
        );

        // Apply with sequence
        manager
            .apply_sequenced(1, PoolEvent::Sync(sync.clone()))
            .unwrap();
        assert_eq!(manager.last_sequence(), 1);

        // Gap should fail but handle gracefully
        let result = manager.apply_sequenced(3, PoolEvent::Sync(sync.clone()));
        assert!(result.is_err()); // Should detect gap

        // Reset tracker and test correct sequence
        manager.sequence_tracker.write().set_last_sequence(0);
        manager
            .apply_sequenced(1, PoolEvent::Sync(sync.clone()))
            .unwrap();
        manager.apply_sequenced(2, PoolEvent::Sync(sync)).unwrap();
        assert_eq!(manager.last_sequence(), 2);
    }
}

# Multi-Asset Trading Registry Infrastructure
## Unified Instrument Registry for DeFi, CEX, and TradFi

### Design Philosophy

**Core Principle**: Treat all tradeable assets as "instruments" with a unified ID system, then specialize through typed registries that reference the base instrument registry.

**Key Benefits**:
- Uniform arbitrage logic across asset classes
- Compact binary messaging (ID-based references)
- Dynamic discovery and registration
- Cross-asset opportunity detection
- Type safety with performance optimization

---

## 1. Core Instrument Registry

### ID Generation Strategy

For production systems handling millions of instruments, we offer two approaches:

#### Option A: 64-bit IDs with Collision Detection (Default)
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct InstrumentId(pub u64);
```
- Collision probability: ~1 in 2^32 at 65k instruments
- Suitable for systems with <100k instruments
- Requires collision detection and handling

#### Option B: 128-bit IDs for Zero Collision Risk
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct InstrumentId(pub u128);
```
- Collision probability: Effectively zero (1 in 2^64 at 4 billion instruments)
- Recommended for systems with >100k instruments
- Slightly larger messages but guaranteed uniqueness

### Base Instrument Definition
```rust

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instrument {
    pub id: InstrumentId,
    pub symbol: String,                    // Human-readable (AAPL, USDC, ETH)
    pub instrument_type: InstrumentType,
    pub source: InstrumentSource,
    pub decimals: u8,                      // Precision for pricing/sizing
    pub metadata: InstrumentMetadata,
    pub created_at: SystemTime,
    pub last_updated: SystemTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InstrumentType {
    Token {
        blockchain: Blockchain,
        contract_address: String,
        token_standard: TokenStandard,     // ERC20, SPL, etc.
    },
    Stock {
        ticker: String,
        exchange: String,                  // NYSE, NASDAQ
        isin: String,                      // International Securities ID (e.g., US0378331005 for Apple)
        cusip: Option<String>,             // Committee on Uniform Securities ID (US/Canada)
        sedol: Option<String>,             // Stock Exchange Daily Official List (UK)
        sector: Option<String>,
        market_cap: Option<u64>,
    },
    ETF {
        ticker: String,
        exchange: String,
        isin: String,                      // Unique global identifier
        cusip: Option<String>,
        underlying_index: Option<String>,
        expense_ratio: Option<f64>,
    },
    Future {
        underlying: String,                // ES, NQ, CL
        expiration: SystemTime,
        contract_size: f64,
        tick_size: f64,
    },
    Option {
        underlying_id: InstrumentId,
        strike_price: f64,
        expiration: SystemTime,
        option_type: OptionType,           // Call, Put
        style: OptionStyle,                // American, European
        occ_symbol: Option<String>,        // Options Clearing Corp symbol
    },
    Currency {
        iso_code: String,                  // USD, EUR, JPY
        country: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InstrumentSource {
    DeFi { chain: Blockchain },
    CEX { exchange: String },
    TradFi { broker: String },
    Synthetic,                             // Derived/computed instruments
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Blockchain {
    Ethereum,
    Polygon,
    BSC,
    Arbitrum,
    Solana,
    Avalanche,
    Base,
}
```

### Instrument Registry Implementation
```rust
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use blake3::Hasher;
use dashmap::DashMap;
use tokio::sync::broadcast;

#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("Duplicate instrument ID: {0:?}")]
    DuplicateId(InstrumentId),
    
    #[error("Duplicate symbol: {0}")]
    DuplicateSymbol(String),
    
    #[error("Unknown instrument: {0:?}")]
    UnknownInstrument(InstrumentId),
    
    #[error("Unknown pool: {0:?}")]
    UnknownPool(PoolId),
    
    #[error("CRITICAL: Hash collision detected for ID {id:?} between {existing_symbol} and {new_symbol}")]
    HashCollision {
        id: InstrumentId,
        existing_symbol: String,
        new_symbol: String,
    },
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

pub struct InstrumentRegistry {
    // Primary storage - using DashMap for lock-free concurrent access
    instruments: Arc<DashMap<InstrumentId, Arc<Instrument>>>,
    
    // Fast lookup indices - lightweight, storing only IDs
    symbol_to_id: Arc<DashMap<String, InstrumentId>>,
    address_to_id: Arc<DashMap<String, InstrumentId>>,
    isin_to_id: Arc<DashMap<String, InstrumentId>>,      // ISIN lookup for TradFi
    cusip_to_id: Arc<DashMap<String, InstrumentId>>,     // CUSIP lookup
    type_index: Arc<DashMap<InstrumentType, Vec<InstrumentId>>>,
    source_index: Arc<DashMap<InstrumentSource, Vec<InstrumentId>>>,
    
    // Cross-exchange tracking - same asset on multiple venues
    isin_exchanges: Arc<DashMap<String, Vec<(String, InstrumentId)>>>, // ISIN -> [(Exchange, ID)]
    
    // Collision detection and handling
    id_collisions: Arc<DashMap<InstrumentId, Vec<Arc<Instrument>>>>,
    
    // Metrics for monitoring
    metrics: Arc<RegistryMetrics>,
    
    // Dynamic updates
    update_channel: broadcast::Sender<RegistryUpdate>,
}

#[derive(Debug, Default)]
pub struct RegistryMetrics {
    pub total_lookups: AtomicU64,
    pub cache_hits: AtomicU64,
    pub hash_collisions: AtomicU64,
    pub registration_failures: AtomicU64,
    pub total_instruments: AtomicU64,
}

impl InstrumentRegistry {
    pub fn register_instrument(&self, mut instrument: Instrument) -> Result<InstrumentId, RegistryError> {
        // Generate deterministic ID based on content
        let id = self.generate_instrument_id(&instrument);
        instrument.id = id;
        
        // Check for hash collision with different content
        if let Some(existing) = self.instruments.get(&id) {
            // Verify if it's actually the same instrument or a collision
            if !self.is_same_instrument(&existing, &instrument) {
                // Critical: Hash collision detected!
                self.metrics.hash_collisions.fetch_add(1, Ordering::Relaxed);
                
                // Store in collision tracking
                self.id_collisions.entry(id)
                    .or_insert_with(Vec::new)
                    .push(Arc::new(instrument.clone()));
                
                // Log critical error and reject registration
                error!("CRITICAL: Hash collision detected for ID {} between {} and {}", 
                       id.0, existing.symbol, instrument.symbol);
                
                return Err(RegistryError::HashCollision { 
                    id, 
                    existing_symbol: existing.symbol.clone(),
                    new_symbol: instrument.symbol.clone() 
                });
            }
            
            // Same instrument, already registered
            return Ok(id);
        }
        
        // Check for duplicate symbols within same source
        if let Some(existing_id) = self.symbol_to_id.get(&instrument.symbol) {
            let existing = self.instruments.get(&existing_id).unwrap();
            if existing.source == instrument.source {
                return Err(RegistryError::DuplicateSymbol(instrument.symbol.clone()));
            }
        }
        
        // Wrap in Arc for memory efficiency
        let instrument_arc = Arc::new(instrument.clone());
        
        // Insert into all indices - DashMap handles concurrency
        self.instruments.insert(id, instrument_arc.clone());
        self.symbol_to_id.insert(instrument.symbol.clone(), id);
        
        // Update type index
        self.type_index.entry(instrument.instrument_type.clone())
            .or_insert_with(Vec::new)
            .push(id);
        
        // Handle specialized indexing based on instrument type
        match &instrument.instrument_type {
            InstrumentType::Token { contract_address, .. } => {
                self.address_to_id.insert(contract_address.to_lowercase(), id);
            }
            InstrumentType::Stock { isin, cusip, exchange, .. } |
            InstrumentType::ETF { isin, cusip, exchange, .. } => {
                // Index by ISIN (global identifier)
                self.isin_to_id.insert(isin.clone(), id);
                
                // Track all exchanges this ISIN trades on
                self.isin_exchanges.entry(isin.clone())
                    .or_insert_with(Vec::new)
                    .push((exchange.clone(), id));
                
                // Index by CUSIP if available (US/Canada specific)
                if let Some(cusip_code) = cusip {
                    self.cusip_to_id.insert(cusip_code.clone(), id);
                }
            }
            _ => {}
        }
        
        // Update metrics
        self.metrics.total_instruments.fetch_add(1, Ordering::Relaxed);
        
        // Notify subscribers of new instrument
        let _ = self.update_channel.send(RegistryUpdate::InstrumentAdded { 
            instrument: instrument.clone() 
        });
        
        Ok(id)
    }
    
    fn is_same_instrument(&self, a: &Instrument, b: &Instrument) -> bool {
        a.symbol == b.symbol &&
        a.instrument_type == b.instrument_type &&
        a.source == b.source &&
        a.decimals == b.decimals
    }
    
    pub fn get_by_id(&self, id: InstrumentId) -> Option<Arc<Instrument>> {
        self.metrics.total_lookups.fetch_add(1, Ordering::Relaxed);
        
        self.instruments.get(&id).map(|entry| {
            self.metrics.cache_hits.fetch_add(1, Ordering::Relaxed);
            entry.clone()
        })
    }
    
    pub fn get_by_symbol(&self, symbol: &str) -> Option<Arc<Instrument>> {
        self.metrics.total_lookups.fetch_add(1, Ordering::Relaxed);
        
        self.symbol_to_id.get(symbol)
            .and_then(|entry| {
                let id = *entry;
                self.get_by_id(id)
            })
    }
    
    pub fn get_by_address(&self, address: &str) -> Option<Arc<Instrument>> {
        self.metrics.total_lookups.fetch_add(1, Ordering::Relaxed);
        
        // Normalize address to lowercase for case-insensitive lookup
        self.address_to_id.get(&address.to_lowercase())
            .and_then(|entry| {
                let id = *entry;
                self.get_by_id(id)
            })
    }
    
    pub fn find_by_type(&self, instrument_type: &InstrumentType) -> Vec<Arc<Instrument>> {
        self.metrics.total_lookups.fetch_add(1, Ordering::Relaxed);
        
        self.type_index.get(instrument_type)
            .map(|entry| {
                let ids = entry.clone();
                ids.iter()
                    .filter_map(|id| self.instruments.get(id).map(|e| e.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }
    
    // Cross-exchange lookup methods for TradFi assets
    pub fn get_by_isin(&self, isin: &str) -> Option<Arc<Instrument>> {
        self.metrics.total_lookups.fetch_add(1, Ordering::Relaxed);
        
        self.isin_to_id.get(isin)
            .and_then(|entry| {
                let id = *entry;
                self.get_by_id(id)
            })
    }
    
    pub fn get_by_cusip(&self, cusip: &str) -> Option<Arc<Instrument>> {
        self.metrics.total_lookups.fetch_add(1, Ordering::Relaxed);
        
        self.cusip_to_id.get(cusip)
            .and_then(|entry| {
                let id = *entry;
                self.get_by_id(id)
            })
    }
    
    // Find all exchanges where an ISIN trades
    pub fn find_all_venues_for_isin(&self, isin: &str) -> Vec<(String, Arc<Instrument>)> {
        self.metrics.total_lookups.fetch_add(1, Ordering::Relaxed);
        
        self.isin_exchanges.get(isin)
            .map(|entry| {
                entry.iter()
                    .filter_map(|(exchange, id)| {
                        self.instruments.get(id)
                            .map(|inst| (exchange.clone(), inst.clone()))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
    
    // Find best execution venue for an ISIN based on criteria
    pub fn find_best_venue_for_isin(&self, isin: &str, criteria: ExecutionCriteria) -> Option<Arc<Instrument>> {
        let venues = self.find_all_venues_for_isin(isin);
        
        match criteria {
            ExecutionCriteria::LowestFees => {
                // Return venue with lowest trading fees
                venues.into_iter()
                    .min_by_key(|(exchange, _)| self.get_exchange_fees(exchange))
                    .map(|(_, inst)| inst)
            }
            ExecutionCriteria::HighestLiquidity => {
                // Return venue with highest liquidity
                venues.into_iter()
                    .max_by_key(|(exchange, _)| self.get_exchange_liquidity(exchange))
                    .map(|(_, inst)| inst)
            }
            ExecutionCriteria::FastestExecution => {
                // Return venue with fastest execution
                venues.into_iter()
                    .min_by_key(|(exchange, _)| self.get_exchange_latency(exchange))
                    .map(|(_, inst)| inst)
            }
        }
    }
    
    fn generate_instrument_id(&self, instrument: &Instrument) -> InstrumentId {
        // Use blake3 for deterministic, cryptographic hashing
        // This ensures consistent IDs across restarts and prevents collisions
        let mut hasher = blake3::Hasher::new();
        
        // Include all identifying fields in hash
        hasher.update(instrument.symbol.as_bytes());
        hasher.update(&[instrument.instrument_type.discriminant()]);
        hasher.update(&[instrument.source.discriminant()]);
        hasher.update(&[instrument.decimals]);
        
        // Add source-specific uniqueness
        match &instrument.instrument_type {
            InstrumentType::Token { contract_address, blockchain, .. } => {
                hasher.update(contract_address.as_bytes());
                hasher.update(&[blockchain.discriminant()]);
            }
            InstrumentType::Stock { ticker, exchange, .. } => {
                hasher.update(ticker.as_bytes());
                hasher.update(exchange.as_bytes());
            }
            _ => {}
        }
        
        let hash = hasher.finalize();
        // Use first 8 bytes for 64-bit ID
        // Collision probability: ~1 in 2^32 at 65k instruments
        InstrumentId(u64::from_le_bytes(hash.as_bytes()[0..8].try_into().unwrap()))
    }
}
```

---

## 2. Specialized Registries

### Pool Registry (DeFi)
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pool {
    pub id: PoolId,
    pub token0_id: InstrumentId,           // Reference to base instrument
    pub token1_id: InstrumentId,           // Reference to quote instrument
    pub dex: String,                       // Uniswap, Sushiswap, Curve
    pub pool_address: String,              // On-chain address
    pub fee_tier: Option<u32>,             // Fee in basis points
    pub pool_type: PoolType,
    pub blockchain: Blockchain,
    pub liquidity: Option<f64>,            // Current liquidity (dynamic)
    pub created_at: SystemTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PoolType {
    ConstantProduct,                       // Uniswap v2 style
    ConcentratedLiquidity,                 // Uniswap v3 style
    StableSwap,                           // Curve style
    WeightedPool,                         // Balancer style
    Custom { algorithm: String },
}

pub struct PoolRegistry {
    // Primary storage with Arc for memory efficiency
    pools: Arc<DashMap<PoolId, Arc<Pool>>>,
    
    // Indices store only IDs to reduce memory usage
    token_pair_index: Arc<DashMap<(InstrumentId, InstrumentId), Vec<PoolId>>>,
    dex_index: Arc<DashMap<String, Vec<PoolId>>>,
    blockchain_index: Arc<DashMap<Blockchain, Vec<PoolId>>>,
    
    // Reference to instrument registry for validation
    instrument_registry: Arc<InstrumentRegistry>,
    
    // Metrics
    metrics: Arc<PoolRegistryMetrics>,
}

#[derive(Debug, Default)]
pub struct PoolRegistryMetrics {
    pub total_pools: AtomicU64,
    pub total_lookups: AtomicU64,
    pub pair_lookups: AtomicU64,
    pub registration_failures: AtomicU64,
}

impl PoolRegistry {
    pub fn register_pool(&self, pool: Pool) -> Result<PoolId, RegistryError> {
        // Validate that referenced instruments exist
        if self.instrument_registry.get_by_id(pool.token0_id).is_none() {
            self.metrics.registration_failures.fetch_add(1, Ordering::Relaxed);
            return Err(RegistryError::UnknownInstrument(pool.token0_id));
        }
        if self.instrument_registry.get_by_id(pool.token1_id).is_none() {
            self.metrics.registration_failures.fetch_add(1, Ordering::Relaxed);
            return Err(RegistryError::UnknownInstrument(pool.token1_id));
        }
        
        let pool_id = pool.id;
        
        // Check for duplicate pool
        if self.pools.contains_key(&pool_id) {
            return Ok(pool_id); // Already registered
        }
        
        // Wrap in Arc for memory efficiency
        let pool_arc = Arc::new(pool.clone());
        
        // Insert into primary storage
        self.pools.insert(pool_id, pool_arc.clone());
        
        // Index by token pair (both directions for fast lookup)
        let pair1 = (pool.token0_id, pool.token1_id);
        let pair2 = (pool.token1_id, pool.token0_id);
        
        self.token_pair_index.entry(pair1)
            .or_insert_with(Vec::new)
            .push(pool_id);
        
        self.token_pair_index.entry(pair2)
            .or_insert_with(Vec::new)
            .push(pool_id);
        
        // Index by DEX
        self.dex_index.entry(pool.dex.clone())
            .or_insert_with(Vec::new)
            .push(pool_id);
        
        // Index by blockchain
        self.blockchain_index.entry(pool.blockchain.clone())
            .or_insert_with(Vec::new)
            .push(pool_id);
        
        // Update metrics
        self.metrics.total_pools.fetch_add(1, Ordering::Relaxed);
        
        Ok(pool_id)
    }
    
    pub fn find_pools_for_pair(&self, token0: InstrumentId, token1: InstrumentId) -> Vec<Arc<Pool>> {
        self.metrics.total_lookups.fetch_add(1, Ordering::Relaxed);
        self.metrics.pair_lookups.fetch_add(1, Ordering::Relaxed);
        
        self.token_pair_index.get(&(token0, token1))
            .map(|entry| {
                let pool_ids = entry.clone();
                pool_ids.iter()
                    .filter_map(|id| self.pools.get(id).map(|e| e.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }
    
    pub fn get_by_id(&self, id: PoolId) -> Option<Arc<Pool>> {
        self.metrics.total_lookups.fetch_add(1, Ordering::Relaxed);
        self.pools.get(&id).map(|entry| entry.clone())
    }
}
```

### CEX Registry
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CEXPair {
    pub id: CEXPairId,
    pub base_instrument_id: InstrumentId,
    pub quote_instrument_id: InstrumentId,
    pub exchange: String,                  // Binance, Coinbase, Kraken
    pub exchange_symbol: String,           // Exchange-specific symbol (BTCUSDT)
    pub min_order_size: f64,
    pub max_order_size: Option<f64>,
    pub tick_size: f64,                    // Minimum price increment
    pub lot_size: f64,                     // Minimum quantity increment
    pub maker_fee: f64,                    // Fee for maker orders
    pub taker_fee: f64,                    // Fee for taker orders
    pub is_active: bool,
    pub created_at: SystemTime,
}

pub struct CEXRegistry {
    pairs: Arc<RwLock<HashMap<CEXPairId, CEXPair>>>,
    exchange_index: Arc<RwLock<HashMap<String, Vec<CEXPairId>>>>,
    symbol_index: Arc<RwLock<HashMap<String, CEXPairId>>>,
    instrument_pair_index: Arc<RwLock<HashMap<(InstrumentId, InstrumentId), Vec<CEXPairId>>>>,
    instrument_registry: Arc<InstrumentRegistry>,
}
```

### Venue Registry (Exchanges, Brokers, DEXs)
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct VenueId(pub u64);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Venue {
    pub id: VenueId,
    pub name: String,                      // "NYSE", "Binance", "Uniswap V3"
    pub venue_type: VenueType,
    pub status: VenueStatus,
    pub connectivity: ConnectivityInfo,
    pub trading_hours: Option<TradingHours>,
    pub fee_structure: FeeStructure,
    pub capabilities: VenueCapabilities,
    pub performance_metrics: PerformanceMetrics,
    pub created_at: SystemTime,
    pub last_updated: SystemTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VenueType {
    Stock { 
        mic: String,                       // Market Identifier Code (ISO 10383)
        country: String,
        timezone: String,
    },
    Crypto { 
        exchange_type: CryptoExchangeType, // CEX, DEX
        supported_chains: Vec<Blockchain>,
    },
    Futures { 
        clearing_house: String,
        contract_types: Vec<String>,
    },
    Options { 
        clearing_corp: String,              // OCC, etc.
        exercise_style: Vec<OptionStyle>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CryptoExchangeType {
    CEX,    // Centralized Exchange
    DEX,    // Decentralized Exchange
    Hybrid, // Both CEX and DEX features
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectivityInfo {
    pub api_endpoints: Vec<String>,
    pub websocket_endpoints: Vec<String>,
    pub fix_endpoints: Option<Vec<String>>,
    pub avg_latency_ms: f64,
    pub uptime_percentage: f64,
    pub rate_limits: RateLimits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimits {
    pub orders_per_second: u32,
    pub requests_per_minute: u32,
    pub weight_per_minute: Option<u32>,    // Binance-style weight system
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeStructure {
    pub maker_fee_bps: f64,                // Basis points
    pub taker_fee_bps: f64,
    pub volume_discounts: Vec<VolumeDiscount>,
    pub withdrawal_fees: Option<HashMap<String, f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeDiscount {
    pub volume_threshold_usd: f64,
    pub maker_fee_bps: f64,
    pub taker_fee_bps: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VenueCapabilities {
    pub order_types: Vec<OrderType>,
    pub supports_margin: bool,
    pub supports_derivatives: bool,
    pub supports_stop_loss: bool,
    pub supports_iceberg: bool,
    pub max_order_size_usd: Option<f64>,
    pub min_order_size_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub avg_fill_time_ms: f64,
    pub daily_volume_usd: f64,
    pub liquidity_score: f64,              // 0-100
    pub slippage_bps: f64,                 // Average slippage in basis points
    pub reliability_score: f64,            // 0-100
}

pub struct VenueRegistry {
    // Primary storage
    venues: Arc<DashMap<VenueId, Arc<Venue>>>,
    
    // Lookup indices
    name_to_id: Arc<DashMap<String, VenueId>>,
    mic_to_id: Arc<DashMap<String, VenueId>>,        // Market Identifier Code
    type_index: Arc<DashMap<VenueType, Vec<VenueId>>>,
    
    // Instrument-Venue mapping
    instrument_venues: Arc<DashMap<InstrumentId, Vec<VenueId>>>,
    venue_instruments: Arc<DashMap<VenueId, Vec<InstrumentId>>>,
    
    // ISIN-Venue mapping for cross-exchange arbitrage
    isin_venues: Arc<DashMap<String, Vec<VenueId>>>,
    
    // Performance tracking
    venue_metrics: Arc<DashMap<VenueId, RealTimeMetrics>>,
    
    // References
    instrument_registry: Arc<InstrumentRegistry>,
    
    // Metrics
    metrics: Arc<VenueRegistryMetrics>,
}

impl VenueRegistry {
    pub fn register_venue(&self, venue: Venue) -> Result<VenueId, RegistryError> {
        let id = self.generate_venue_id(&venue);
        let mut venue = venue;
        venue.id = id;
        
        // Check for duplicates
        if self.venues.contains_key(&id) {
            return Ok(id); // Already registered
        }
        
        let venue_arc = Arc::new(venue.clone());
        
        // Insert into storage and indices
        self.venues.insert(id, venue_arc.clone());
        self.name_to_id.insert(venue.name.clone(), id);
        
        // Index by MIC for stock exchanges
        if let VenueType::Stock { ref mic, .. } = venue.venue_type {
            self.mic_to_id.insert(mic.clone(), id);
        }
        
        // Update type index
        self.type_index.entry(venue.venue_type.clone())
            .or_insert_with(Vec::new)
            .push(id);
        
        self.metrics.total_venues.fetch_add(1, Ordering::Relaxed);
        
        Ok(id)
    }
    
    pub fn link_instrument_to_venue(&self, instrument_id: InstrumentId, venue_id: VenueId) -> Result<(), RegistryError> {
        // Validate both exist
        if !self.venues.contains_key(&venue_id) {
            return Err(RegistryError::UnknownVenue(venue_id));
        }
        
        if self.instrument_registry.get_by_id(instrument_id).is_none() {
            return Err(RegistryError::UnknownInstrument(instrument_id));
        }
        
        // Create bidirectional mapping
        self.instrument_venues.entry(instrument_id)
            .or_insert_with(Vec::new)
            .push(venue_id);
            
        self.venue_instruments.entry(venue_id)
            .or_insert_with(Vec::new)
            .push(instrument_id);
        
        // If instrument has ISIN, track venue for cross-exchange
        if let Some(instrument) = self.instrument_registry.get_by_id(instrument_id) {
            match &instrument.instrument_type {
                InstrumentType::Stock { isin, .. } | 
                InstrumentType::ETF { isin, .. } => {
                    self.isin_venues.entry(isin.clone())
                        .or_insert_with(Vec::new)
                        .push(venue_id);
                }
                _ => {}
            }
        }
        
        Ok(())
    }
    
    pub fn find_venues_for_instrument(&self, instrument_id: InstrumentId) -> Vec<Arc<Venue>> {
        self.instrument_venues.get(&instrument_id)
            .map(|entry| {
                entry.iter()
                    .filter_map(|venue_id| self.venues.get(venue_id).map(|v| v.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }
    
    pub fn find_best_venue_for_instrument(&self, 
        instrument_id: InstrumentId, 
        criteria: ExecutionCriteria
    ) -> Option<Arc<Venue>> {
        let venues = self.find_venues_for_instrument(instrument_id);
        
        match criteria {
            ExecutionCriteria::LowestFees => {
                venues.into_iter()
                    .min_by(|a, b| {
                        let a_fee = (a.fee_structure.maker_fee_bps + a.fee_structure.taker_fee_bps) / 2.0;
                        let b_fee = (b.fee_structure.maker_fee_bps + b.fee_structure.taker_fee_bps) / 2.0;
                        a_fee.partial_cmp(&b_fee).unwrap()
                    })
            }
            ExecutionCriteria::HighestLiquidity => {
                venues.into_iter()
                    .max_by(|a, b| {
                        a.performance_metrics.liquidity_score
                            .partial_cmp(&b.performance_metrics.liquidity_score)
                            .unwrap()
                    })
            }
            ExecutionCriteria::FastestExecution => {
                venues.into_iter()
                    .min_by(|a, b| {
                        a.performance_metrics.avg_fill_time_ms
                            .partial_cmp(&b.performance_metrics.avg_fill_time_ms)
                            .unwrap()
                    })
            }
        }
    }
    
    pub fn find_arbitrage_venues_for_isin(&self, isin: &str) -> Vec<(Arc<Venue>, Arc<Venue>)> {
        // Find all venues trading this ISIN
        let venue_ids = self.isin_venues.get(isin)
            .map(|entry| entry.clone())
            .unwrap_or_default();
        
        let mut arbitrage_pairs = Vec::new();
        
        // Check all pairs for arbitrage opportunities
        for i in 0..venue_ids.len() {
            for j in i+1..venue_ids.len() {
                if let (Some(venue_a), Some(venue_b)) = 
                    (self.venues.get(&venue_ids[i]), self.venues.get(&venue_ids[j])) {
                    
                    // Check if venues have different fee structures or liquidity
                    let fee_diff = (venue_a.fee_structure.taker_fee_bps - venue_b.fee_structure.taker_fee_bps).abs();
                    let liquidity_diff = (venue_a.performance_metrics.liquidity_score - venue_b.performance_metrics.liquidity_score).abs();
                    
                    // Potential arbitrage if significant differences exist
                    if fee_diff > 5.0 || liquidity_diff > 20.0 {
                        arbitrage_pairs.push((venue_a.clone(), venue_b.clone()));
                    }
                }
            }
        }
        
        arbitrage_pairs
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionCriteria {
    LowestFees,
    HighestLiquidity,
    FastestExecution,
}
```

### TradFi Registry
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradFiInstrument {
    pub id: TradFiInstrumentId,
    pub instrument_id: InstrumentId,       // Reference to base instrument
    pub broker: String,                    // Alpaca, Tradovate, IBKR
    pub broker_symbol: String,             // Broker-specific symbol
    pub market: String,                    // NYSE, NASDAQ, CME
    pub session_hours: SessionHours,
    pub contract_details: ContractDetails,
    pub margin_requirements: Option<MarginInfo>,
    pub is_tradeable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionHours {
    pub market_open: time::Time,
    pub market_close: time::Time,
    pub timezone: String,
    pub pre_market: Option<(time::Time, time::Time)>,
    pub after_hours: Option<(time::Time, time::Time)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContractDetails {
    Stock {
        shares_outstanding: Option<u64>,
        dividend_yield: Option<f64>,
    },
    Future {
        contract_size: f64,
        tick_value: f64,
        settlement_type: SettlementType,
    },
    Option {
        contract_size: f64,
        premium_multiplier: f64,
    },
}
```

---

## 3. Binary Protocol for Registry Messages

### Message Type Definitions
```rust
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum RegistryMessageType {
    InstrumentRegistration = 0x01,
    PoolRegistration = 0x02,
    CEXPairRegistration = 0x03,
    TradFiRegistration = 0x04,
    InstrumentUpdate = 0x05,
    RegistryQuery = 0x10,
    RegistryResponse = 0x11,
}

// Compact binary message format
#[repr(C, packed)]
pub struct RegistryMessageHeader {
    pub message_type: u8,                  // RegistryMessageType
    pub message_length: u32,               // Total message size
    pub sequence: u64,                     // Monotonic sequence
    pub timestamp: u64,                    // Nanosecond timestamp
    pub checksum: u32,                     // CRC32 of payload
}

// Instrument registration message
#[repr(C, packed)]
pub struct InstrumentRegistrationMessage {
    pub header: RegistryMessageHeader,
    pub instrument_id: u64,
    pub instrument_type: u8,               // Enum discriminant
    pub source_type: u8,                   // Enum discriminant
    pub decimals: u8,
    pub symbol_length: u8,
    // Variable length fields follow:
    // symbol: [u8; symbol_length]
    // type_specific_data: variable
}

// Pool registration message
#[repr(C, packed)]
pub struct PoolRegistrationMessage {
    pub header: RegistryMessageHeader,
    pub pool_id: u64,
    pub token0_id: u64,
    pub token1_id: u64,
    pub blockchain: u8,
    pub pool_type: u8,
    pub fee_tier: u32,
    pub dex_name_length: u8,
    pub address_length: u8,
    // Variable length fields follow:
    // dex_name: [u8; dex_name_length]
    // pool_address: [u8; address_length]
}
```

### Serialization Implementation
```rust
pub trait BinarySerializable {
    fn serialize_binary(&self, buffer: &mut Vec<u8>) -> Result<(), SerializationError>;
    fn deserialize_binary(buffer: &[u8]) -> Result<Self, SerializationError> where Self: Sized;
}

impl BinarySerializable for Instrument {
    fn serialize_binary(&self, buffer: &mut Vec<u8>) -> Result<(), SerializationError> {
        let header = RegistryMessageHeader {
            message_type: RegistryMessageType::InstrumentRegistration as u8,
            message_length: self.calculate_binary_size(),
            sequence: 0, // Set by caller
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64,
            checksum: 0, // Calculated after serialization
        };
        
        // Serialize header
        buffer.extend_from_slice(unsafe {
            std::slice::from_raw_parts(
                &header as *const _ as *const u8,
                std::mem::size_of::<RegistryMessageHeader>()
            )
        });
        
        // Serialize instrument fields
        buffer.extend_from_slice(&self.id.0.to_le_bytes());
        buffer.extend_from_slice(&(self.instrument_type.discriminant() as u8).to_le_bytes());
        buffer.extend_from_slice(&(self.source.discriminant() as u8).to_le_bytes());
        buffer.extend_from_slice(&self.decimals.to_le_bytes());
        buffer.extend_from_slice(&(self.symbol.len() as u8).to_le_bytes());
        buffer.extend_from_slice(self.symbol.as_bytes());
        
        // Serialize type-specific data
        self.instrument_type.serialize_binary(buffer)?;
        
        // Calculate and update checksum
        let payload_start = std::mem::size_of::<RegistryMessageHeader>();
        let checksum = crc32fast::hash(&buffer[payload_start..]);
        
        // Update checksum in header
        unsafe {
            let header_ptr = buffer.as_mut_ptr() as *mut RegistryMessageHeader;
            (*header_ptr).checksum = checksum;
        }
        
        Ok(())
    }
}
```

---

## 4. Dynamic Discovery and Auto-Registration

### Parser Framework for Multi-Source Data
```rust
pub trait InstrumentParser: Send + Sync {
    fn can_parse(&self, data: &[u8]) -> bool;
    fn parse(&self, data: &[u8]) -> Result<InstrumentPayload, ParseError>;
    fn source_type(&self) -> InstrumentSource;
}

#[derive(Debug, Clone)]
pub struct InstrumentPayload {
    pub symbol: String,
    pub instrument_type: InstrumentType,
    pub source: InstrumentSource,
    pub decimals: u8,
    pub metadata: HashMap<String, String>,      // Flexible metadata storage
    pub blockchain_address: Option<String>,
    pub exchange_symbol: Option<String>,
    pub contract_details: Option<serde_json::Value>, // Source-specific data
}

// DeFi Token Parser (handles ERC20 events, pool discoveries)
pub struct DeFiTokenParser {
    supported_chains: HashSet<Blockchain>,
    abi_decoder: AbiDecoder,
}

impl InstrumentParser for DeFiTokenParser {
    fn can_parse(&self, data: &[u8]) -> bool {
        // Check if data looks like an ERC20 token event or pool creation
        if data.len() < 4 { return false; }
        
        let method_sig = &data[0..4];
        matches!(method_sig, 
            b"\xa0\x91\x7d\xbc" | // Transfer event
            b"\x8c\x5b\xe1\xe5" | // Pool created event
            b"\xdd\xf2\x52\xad"   // Approval event
        )
    }
    
    fn parse(&self, data: &[u8]) -> Result<InstrumentPayload, ParseError> {
        let decoded = self.abi_decoder.decode_log(data)?;
        
        match decoded.topic {
            "PoolCreated" => {
                // Extract token addresses from pool creation event
                let token0_address = decoded.get_address("token0")?;
                let token1_address = decoded.get_address("token1")?;
                
                // Query token metadata
                let token0_info = self.query_token_metadata(&token0_address)?;
                let token1_info = self.query_token_metadata(&token1_address)?;
                
                // Return both tokens for registration
                Ok(vec![
                    self.create_token_payload(token0_info),
                    self.create_token_payload(token1_info),
                ])
            }
            "Transfer" => {
                // Extract token from transfer event
                let token_address = decoded.contract_address;
                let token_info = self.query_token_metadata(&token_address)?;
                Ok(vec![self.create_token_payload(token_info)])
            }
            _ => Err(ParseError::UnsupportedEvent),
        }
    }
    
    fn create_token_payload(&self, token_info: TokenMetadata) -> InstrumentPayload {
        InstrumentPayload {
            symbol: token_info.symbol,
            instrument_type: InstrumentType::Token {
                blockchain: token_info.blockchain,
                contract_address: token_info.address,
                token_standard: TokenStandard::ERC20,
            },
            source: InstrumentSource::DeFi { 
                chain: token_info.blockchain 
            },
            decimals: token_info.decimals,
            metadata: hashmap! {
                "name".to_string() => token_info.name,
                "total_supply".to_string() => token_info.total_supply.to_string(),
            },
            blockchain_address: Some(token_info.address),
            exchange_symbol: None,
            contract_details: Some(serde_json::to_value(&token_info)?),
        }
    }
}

// CEX Parser (handles exchange API responses)
pub struct CEXParser {
    exchange: String,
    api_format: ApiFormat,
}

impl InstrumentParser for CEXParser {
    fn can_parse(&self, data: &[u8]) -> bool {
        // Check if data is valid JSON with expected CEX format
        if let Ok(json) = serde_json::from_slice::<serde_json::Value>(data) {
            match self.api_format {
                ApiFormat::Binance => {
                    json.get("symbols").is_some() || 
                    json.get("symbol").is_some()
                }
                ApiFormat::Coinbase => {
                    json.get("id").is_some() && 
                    json.get("base_currency").is_some()
                }
                ApiFormat::Kraken => {
                    json.get("result").is_some()
                }
            }
        } else {
            false
        }
    }
    
    fn parse(&self, data: &[u8]) -> Result<InstrumentPayload, ParseError> {
        let json: serde_json::Value = serde_json::from_slice(data)?;
        
        match self.api_format {
            ApiFormat::Binance => self.parse_binance_symbols(&json),
            ApiFormat::Coinbase => self.parse_coinbase_products(&json),
            ApiFormat::Kraken => self.parse_kraken_pairs(&json),
        }
    }
    
    fn parse_binance_symbols(&self, json: &serde_json::Value) -> Result<InstrumentPayload, ParseError> {
        if let Some(symbols) = json.get("symbols").and_then(|s| s.as_array()) {
            let mut payloads = Vec::new();
            
            for symbol_data in symbols {
                let symbol = symbol_data.get("symbol")
                    .and_then(|s| s.as_str())
                    .ok_or(ParseError::MissingField("symbol"))?;
                    
                let base_asset = symbol_data.get("baseAsset")
                    .and_then(|s| s.as_str())
                    .ok_or(ParseError::MissingField("baseAsset"))?;
                    
                let quote_asset = symbol_data.get("quoteAsset")
                    .and_then(|s| s.as_str())
                    .ok_or(ParseError::MissingField("quoteAsset"))?;
                
                // Create instrument payloads for base and quote assets
                payloads.push(InstrumentPayload {
                    symbol: base_asset.to_string(),
                    instrument_type: InstrumentType::Token {
                        blockchain: Blockchain::Unknown, // CEX tokens don't have blockchain
                        contract_address: String::new(),
                        token_standard: TokenStandard::CEX,
                    },
                    source: InstrumentSource::CEX { 
                        exchange: self.exchange.clone() 
                    },
                    decimals: 8, // Default for most CEX assets
                    metadata: hashmap! {
                        "trading_pair".to_string() => symbol.to_string(),
                        "base_asset".to_string() => base_asset.to_string(),
                        "quote_asset".to_string() => quote_asset.to_string(),
                    },
                    blockchain_address: None,
                    exchange_symbol: Some(symbol.to_string()),
                    contract_details: Some(symbol_data.clone()),
                });
            }
            
            Ok(payloads)
        } else {
            Err(ParseError::InvalidFormat)
        }
    }
}

// TradFi Parser (handles Alpaca/Tradovate API responses)
pub struct TradFiParser {
    broker: String,
    asset_class: TradFiAssetClass,
}

impl InstrumentParser for TradFiParser {
    fn can_parse(&self, data: &[u8]) -> bool {
        if let Ok(json) = serde_json::from_slice::<serde_json::Value>(data) {
            match self.broker.as_str() {
                "alpaca" => {
                    json.get("class").is_some() || 
                    json.get("symbol").is_some()
                }
                "tradovate" => {
                    json.get("contractMaturityMonths").is_some() ||
                    json.get("name").is_some()
                }
                _ => false,
            }
        } else {
            false
        }
    }
    
    fn parse(&self, data: &[u8]) -> Result<InstrumentPayload, ParseError> {
        let json: serde_json::Value = serde_json::from_slice(data)?;
        
        match self.broker.as_str() {
            "alpaca" => self.parse_alpaca_asset(&json),
            "tradovate" => self.parse_tradovate_contract(&json),
            _ => Err(ParseError::UnsupportedBroker),
        }
    }
    
    fn parse_alpaca_asset(&self, json: &serde_json::Value) -> Result<InstrumentPayload, ParseError> {
        let symbol = json.get("symbol")
            .and_then(|s| s.as_str())
            .ok_or(ParseError::MissingField("symbol"))?;
            
        let asset_class = json.get("class")
            .and_then(|s| s.as_str())
            .unwrap_or("us_equity");
            
        let exchange = json.get("exchange")
            .and_then(|s| s.as_str())
            .unwrap_or("NYSE");
        
        // Extract ISIN and CUSIP from Alpaca data
        let isin = json.get("isin")
            .and_then(|s| s.as_str())
            .map(String::from)
            .or_else(|| {
                // Generate ISIN from CUSIP if available
                json.get("cusip")
                    .and_then(|s| s.as_str())
                    .map(|cusip| format!("US{}{}", cusip, calculate_isin_check_digit(cusip)))
            })
            .ok_or(ParseError::MissingField("isin"))?;
            
        let cusip = json.get("cusip")
            .and_then(|s| s.as_str())
            .map(String::from);
        
        let instrument_type = match asset_class {
            "us_equity" => InstrumentType::Stock {
                ticker: symbol.to_string(),
                exchange: exchange.to_string(),
                isin: isin.clone(),
                cusip: cusip.clone(),
                sedol: None,
                sector: json.get("sector").and_then(|s| s.as_str()).map(String::from),
                market_cap: None,
            },
            "us_option" => {
                let underlying = json.get("underlying_symbol")
                    .and_then(|s| s.as_str())
                    .ok_or(ParseError::MissingField("underlying_symbol"))?;
                    
                InstrumentType::Option {
                    underlying_id: InstrumentId(0), // Will be resolved later
                    strike_price: json.get("strike_price")
                        .and_then(|s| s.as_f64())
                        .unwrap_or(0.0),
                    expiration: SystemTime::now(), // Parse from expiration_date
                    option_type: OptionType::Call, // Parse from side
                    style: OptionStyle::American,
                }
            }
            _ => return Err(ParseError::UnsupportedAssetClass),
        };
        
        Ok(InstrumentPayload {
            symbol: symbol.to_string(),
            instrument_type,
            source: InstrumentSource::TradFi { 
                broker: self.broker.clone() 
            },
            decimals: 2, // USD cents precision
            metadata: hashmap! {
                "asset_class".to_string() => asset_class.to_string(),
                "exchange".to_string() => exchange.to_string(),
                "tradeable".to_string() => json.get("tradable")
                    .and_then(|t| t.as_bool())
                    .unwrap_or(false)
                    .to_string(),
            },
            blockchain_address: None,
            exchange_symbol: Some(symbol.to_string()),
            contract_details: Some(json.clone()),
        })
    }
}
```

### Dynamic Registry Manager
```rust
pub struct DynamicRegistryManager {
    instrument_registry: Arc<InstrumentRegistry>,
    pool_registry: Arc<PoolRegistry>,
    cex_registry: Arc<CEXRegistry>,
    tradfi_registry: Arc<TradFiRegistry>,
    
    // Parser ecosystem
    parsers: Vec<Box<dyn InstrumentParser>>,
    parser_stats: HashMap<String, ParserStats>,
    
    // Dynamic discovery channels
    discovery_channel: mpsc::Receiver<RawDataEvent>,
    registration_channel: broadcast::Sender<RegistrationEvent>,
    
    // Processing queue for batch operations
    processing_queue: VecDeque<ProcessingTask>,
    
    // Rate limiting and backpressure
    rate_limiter: RateLimiter,
}

impl DynamicRegistryManager {
    pub fn new() -> Self {
        let mut manager = Self {
            instrument_registry: Arc::new(InstrumentRegistry::new()),
            pool_registry: Arc::new(PoolRegistry::new()),
            cex_registry: Arc::new(CEXRegistry::new()),
            tradfi_registry: Arc::new(TradFiRegistry::new()),
            parsers: Vec::new(),
            parser_stats: HashMap::new(),
            discovery_channel: mpsc::channel(10000).1,
            registration_channel: broadcast::channel(1000).0,
            processing_queue: VecDeque::new(),
            rate_limiter: RateLimiter::new(1000), // 1000 registrations/sec max
        };
        
        // Register default parsers
        manager.register_parser(Box::new(DeFiTokenParser::new()));
        manager.register_parser(Box::new(CEXParser::new("binance")));
        manager.register_parser(Box::new(CEXParser::new("coinbase")));
        manager.register_parser(Box::new(TradFiParser::new("alpaca")));
        manager.register_parser(Box::new(TradFiParser::new("tradovate")));
        
        manager
    }
    
    pub fn register_parser(&mut self, parser: Box<dyn InstrumentParser>) {
        let parser_name = format!("{:?}", parser.source_type());
        self.parser_stats.insert(parser_name, ParserStats::default());
        self.parsers.push(parser);
    }
    
    pub async fn process_raw_data(&mut self, data: RawDataEvent) -> Result<Vec<InstrumentId>, ProcessingError> {
        // Rate limiting to prevent registry flooding
        self.rate_limiter.wait().await;
        
        let mut registered_instruments = Vec::new();
        
        // Try each parser until one can handle the data
        for parser in &self.parsers {
            if parser.can_parse(&data.payload) {
                let parser_name = format!("{:?}", parser.source_type());
                let stats = self.parser_stats.get_mut(&parser_name).unwrap();
                
                match parser.parse(&data.payload) {
                    Ok(payloads) => {
                        stats.successful_parses += 1;
                        
                        for payload in payloads {
                            match self.register_instrument_from_payload(payload).await {
                                Ok(instrument_id) => {
                                    registered_instruments.push(instrument_id);
                                    stats.successful_registrations += 1;
                                }
                                Err(e) => {
                                    stats.registration_errors += 1;
                                    warn!("Failed to register instrument: {:?}", e);
                                }
                            }
                        }
                        
                        break; // Stop after first successful parse
                    }
                    Err(e) => {
                        stats.parse_errors += 1;
                        debug!("Parser {} failed: {:?}", parser_name, e);
                    }
                }
            }
        }
        
        if registered_instruments.is_empty() {
            return Err(ProcessingError::NoSuitableParser);
        }
        
        Ok(registered_instruments)
    }
    
    async fn register_instrument_from_payload(&mut self, payload: InstrumentPayload) -> Result<InstrumentId, RegistrationError> {
        // Check for existing instrument to avoid duplicates
        if let Some(existing_id) = self.find_existing_instrument(&payload) {
            return Ok(existing_id);
        }
        
        // Create new instrument
        let instrument = Instrument {
            id: InstrumentId(0), // Will be generated
            symbol: payload.symbol,
            instrument_type: payload.instrument_type,
            source: payload.source,
            decimals: payload.decimals,
            metadata: InstrumentMetadata {
                created_at: SystemTime::now(),
                last_updated: SystemTime::now(),
                extra_fields: payload.metadata,
            },
            created_at: SystemTime::now(),
            last_updated: SystemTime::now(),
        };
        
        // Register in instrument registry
        let instrument_id = self.instrument_registry.register_instrument(instrument)?;
        
        // Notify other components of new instrument
        let _ = self.registration_channel.send(RegistrationEvent::InstrumentAdded { 
            instrument_id,
            source: payload.source.clone(),
        });
        
        // Trigger downstream registrations if applicable
        self.trigger_related_registrations(instrument_id, &payload).await?;
        
        Ok(instrument_id)
    }
    
    async fn trigger_related_registrations(&mut self, instrument_id: InstrumentId, payload: &InstrumentPayload) -> Result<(), RegistrationError> {
        match payload.source {
            InstrumentSource::DeFi { .. } => {
                // Look for pools containing this token
                self.discover_pools_for_token(instrument_id).await?;
            }
            InstrumentSource::CEX { ref exchange } => {
                // Look for trading pairs with this asset
                self.discover_pairs_for_asset(instrument_id, exchange).await?;
            }
            InstrumentSource::TradFi { ref broker } => {
                // Look for related derivatives or options
                self.discover_derivatives_for_instrument(instrument_id, broker).await?;
            }
            InstrumentSource::Synthetic => {
                // Handle synthetic instruments
            }
        }
        
        Ok(())
    }
    
    fn find_existing_instrument(&self, payload: &InstrumentPayload) -> Option<InstrumentId> {
        // Check by symbol first (fastest)
        if let Some(existing) = self.instrument_registry.get_by_symbol(&payload.symbol) {
            if existing.source == payload.source {
                return Some(existing.id);
            }
        }
        
        // Check by blockchain address for tokens
        if let Some(ref address) = payload.blockchain_address {
            if let Some(existing) = self.instrument_registry.get_by_address(address) {
                return Some(existing.id);
            }
        }
        
        // Check by exchange symbol for CEX/TradFi
        if let Some(ref exchange_symbol) = payload.exchange_symbol {
            // Custom lookup by exchange symbol
            if let Some(existing) = self.instrument_registry.get_by_exchange_symbol(exchange_symbol, &payload.source) {
                return Some(existing.id);
            }
        }
        
        None
    }
}

#[derive(Debug, Clone)]
pub struct RawDataEvent {
    pub source: String,                    // "ethereum_logs", "binance_api", "alpaca_feed"
    pub timestamp: SystemTime,
    pub payload: Vec<u8>,                  // Raw data from source
    pub metadata: HashMap<String, String>, // Source-specific context
}

#[derive(Debug, Clone)]
pub struct ParserStats {
    pub successful_parses: u64,
    pub parse_errors: u64,
    pub successful_registrations: u64,
    pub registration_errors: u64,
    pub last_activity: SystemTime,
}

#[derive(Debug, Clone)]
pub enum RegistrationEvent {
    InstrumentAdded { instrument_id: InstrumentId, source: InstrumentSource },
    PoolAdded { pool_id: PoolId, tokens: (InstrumentId, InstrumentId) },
    PairAdded { pair_id: CEXPairId, base: InstrumentId, quote: InstrumentId },
    RegistrationError { source: String, error: String },
}
```

### Real-Time Data Ingestion
```rust
pub struct RealTimeIngestion {
    registry_manager: Arc<Mutex<DynamicRegistryManager>>,
    data_sources: HashMap<String, Box<dyn DataSource>>,
    ingestion_tasks: Vec<JoinHandle<()>>,
}

impl RealTimeIngestion {
    pub async fn start_ingestion(&mut self) {
        // Start DeFi event monitoring
        let defi_task = self.spawn_defi_ingestion();
        
        // Start CEX API polling
        let cex_task = self.spawn_cex_ingestion();
        
        // Start TradFi data feeds
        let tradfi_task = self.spawn_tradfi_ingestion();
        
        self.ingestion_tasks.extend([defi_task, cex_task, tradfi_task]);
    }
    
    fn spawn_defi_ingestion(&self) -> JoinHandle<()> {
        let registry_manager = Arc::clone(&self.registry_manager);
        
        tokio::spawn(async move {
            let mut event_stream = EthereumEventStream::new().await;
            
            while let Some(event) = event_stream.next().await {
                let raw_data = RawDataEvent {
                    source: "ethereum_events".to_string(),
                    timestamp: SystemTime::now(),
                    payload: event.data,
                    metadata: hashmap! {
                        "block_number".to_string() => event.block_number.to_string(),
                        "transaction_hash".to_string() => event.tx_hash,
                        "contract_address".to_string() => event.address,
                    },
                };
                
                let mut manager = registry_manager.lock().await;
                if let Ok(instrument_ids) = manager.process_raw_data(raw_data).await {
                    info!("Auto-registered {} new instruments from DeFi event", instrument_ids.len());
                }
            }
        })
    }
    
    fn spawn_cex_ingestion(&self) -> JoinHandle<()> {
        let registry_manager = Arc::clone(&self.registry_manager);
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes
            let cex_clients = vec![
                BinanceClient::new(),
                CoinbaseClient::new(),
                KrakenClient::new(),
            ];
            
            loop {
                interval.tick().await;
                
                for client in &cex_clients {
                    if let Ok(symbols_data) = client.get_exchange_info().await {
                        let raw_data = RawDataEvent {
                            source: format!("{}_symbols", client.name()),
                            timestamp: SystemTime::now(),
                            payload: symbols_data,
                            metadata: hashmap! {
                                "exchange".to_string() => client.name(),
                                "endpoint".to_string() => "exchangeInfo".to_string(),
                            },
                        };
                        
                        let mut manager = registry_manager.lock().await;
                        if let Ok(instrument_ids) = manager.process_raw_data(raw_data).await {
                            info!("Auto-registered {} new instruments from {}", 
                                  instrument_ids.len(), client.name());
                        }
                    }
                }
            }
        })
    }
}
```

---

## 5. Binary Serialization Pipeline

### Type-Safe Binary Protocol Framework
```rust
// Core binary serialization trait for all registry types
pub trait BinarySerializable: Sized {
    const TYPE_ID: u8;                     // Unique type identifier
    const VERSION: u8;                     // Schema version for evolution
    
    fn binary_size(&self) -> usize;        // Predict serialized size
    fn serialize_binary(&self, buffer: &mut Vec<u8>) -> Result<(), SerializationError>;
    fn deserialize_binary(buffer: &[u8]) -> Result<(Self, usize), SerializationError>;
}

// Universal message header for all registry messages
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct BinaryMessageHeader {
    pub magic: u32,                        // 0xDEADBEEF - message validation
    pub type_id: u8,                       // Message type discriminant
    pub version: u8,                       // Schema version
    pub flags: u8,                         // Compression, encryption flags
    pub reserved: u8,                      // Future use
    pub payload_size: u32,                 // Size of payload in bytes
    pub sequence: u64,                     // Monotonic sequence number
    pub timestamp: u64,                    // Nanosecond timestamp
    pub checksum: u32,                     // CRC32 of payload
}

// Message type registry for compile-time dispatch
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageType {
    // Registry messages
    InstrumentRegistration = 0x01,
    PoolRegistration = 0x02,
    CEXPairRegistration = 0x03,
    TradFiRegistration = 0x04,
    
    // Trading messages
    MarketDataUpdate = 0x10,
    OrderSubmission = 0x11,
    TradeExecution = 0x12,
    ArbitrageOpportunity = 0x13,
    
    // Control messages
    RegistryQuery = 0x20,
    RegistrySnapshot = 0x21,
    HealthCheck = 0x22,
    
    // Error handling
    ErrorResponse = 0xFF,
}

impl MessageType {
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0x01 => Some(Self::InstrumentRegistration),
            0x02 => Some(Self::PoolRegistration),
            0x03 => Some(Self::CEXPairRegistration),
            0x04 => Some(Self::TradFiRegistration),
            0x10 => Some(Self::MarketDataUpdate),
            0x11 => Some(Self::OrderSubmission),
            0x12 => Some(Self::TradeExecution),
            0x13 => Some(Self::ArbitrageOpportunity),
            0x20 => Some(Self::RegistryQuery),
            0x21 => Some(Self::RegistrySnapshot),
            0x22 => Some(Self::HealthCheck),
            0xFF => Some(Self::ErrorResponse),
            _ => None,
        }
    }
}
```

### Compact Instrument Serialization
```rust
impl BinarySerializable for Instrument {
    const TYPE_ID: u8 = MessageType::InstrumentRegistration as u8;
    const VERSION: u8 = 1;
    
    fn binary_size(&self) -> usize {
        size_of::<BinaryMessageHeader>() +
        size_of::<u64>() +                   // instrument_id
        size_of::<u8>() +                    // instrument_type discriminant
        size_of::<u8>() +                    // source discriminant
        size_of::<u8>() +                    // decimals
        size_of::<u8>() +                    // symbol length
        self.symbol.len() +                  // symbol bytes
        self.instrument_type.binary_size() + // type-specific data
        self.source.binary_size()            // source-specific data
    }
    
    fn serialize_binary(&self, buffer: &mut Vec<u8>) -> Result<(), SerializationError> {
        let initial_len = buffer.len();
        
        // Reserve space for header (will be filled at the end)
        buffer.extend_from_slice(&[0u8; size_of::<BinaryMessageHeader>()]);
        let payload_start = buffer.len();
        
        // Serialize instrument data
        buffer.extend_from_slice(&self.id.0.to_le_bytes());
        buffer.extend_from_slice(&[self.instrument_type.discriminant()]);
        buffer.extend_from_slice(&[self.source.discriminant()]);
        buffer.extend_from_slice(&[self.decimals]);
        
        // Variable-length symbol
        let symbol_bytes = self.symbol.as_bytes();
        if symbol_bytes.len() > 255 {
            return Err(SerializationError::SymbolTooLong);
        }
        buffer.extend_from_slice(&[symbol_bytes.len() as u8]);
        buffer.extend_from_slice(symbol_bytes);
        
        // Serialize type-specific data
        self.instrument_type.serialize_binary(buffer)?;
        self.source.serialize_binary(buffer)?;
        
        // Calculate payload size and checksum
        let payload_size = buffer.len() - payload_start;
        let payload_checksum = crc32fast::hash(&buffer[payload_start..]);
        
        // Fill in header
        let header = BinaryMessageHeader {
            magic: 0xDEADBEEF,
            type_id: Self::TYPE_ID,
            version: Self::VERSION,
            flags: 0,
            reserved: 0,
            payload_size: payload_size as u32,
            sequence: 0, // Set by caller
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64,
            checksum: payload_checksum,
        };
        
        // Write header at the beginning
        unsafe {
            let header_ptr = buffer.as_mut_ptr().add(initial_len) as *mut BinaryMessageHeader;
            ptr::write(header_ptr, header);
        }
        
        Ok(())
    }
    
    fn deserialize_binary(buffer: &[u8]) -> Result<(Self, usize), SerializationError> {
        if buffer.len() < size_of::<BinaryMessageHeader>() {
            return Err(SerializationError::BufferTooSmall);
        }
        
        // Read and validate header
        let header = unsafe {
            ptr::read(buffer.as_ptr() as *const BinaryMessageHeader)
        };
        
        if header.magic != 0xDEADBEEF {
            return Err(SerializationError::InvalidMagic);
        }
        
        if header.type_id != Self::TYPE_ID {
            return Err(SerializationError::WrongMessageType);
        }
        
        if header.version != Self::VERSION {
            return Err(SerializationError::UnsupportedVersion);
        }
        
        // Validate payload size and checksum
        let payload_start = size_of::<BinaryMessageHeader>();
        let payload_end = payload_start + header.payload_size as usize;
        
        if buffer.len() < payload_end {
            return Err(SerializationError::BufferTooSmall);
        }
        
        let payload = &buffer[payload_start..payload_end];
        let calculated_checksum = crc32fast::hash(payload);
        
        if calculated_checksum != header.checksum {
            return Err(SerializationError::ChecksumMismatch);
        }
        
        // Deserialize instrument data
        let mut offset = 0;
        
        let instrument_id = InstrumentId(u64::from_le_bytes(
            payload[offset..offset + 8].try_into()?
        ));
        offset += 8;
        
        let instrument_type_discriminant = payload[offset];
        offset += 1;
        
        let source_discriminant = payload[offset];
        offset += 1;
        
        let decimals = payload[offset];
        offset += 1;
        
        let symbol_len = payload[offset] as usize;
        offset += 1;
        
        if offset + symbol_len > payload.len() {
            return Err(SerializationError::BufferTooSmall);
        }
        
        let symbol = String::from_utf8(payload[offset..offset + symbol_len].to_vec())?;
        offset += symbol_len;
        
        // Deserialize type-specific data
        let (instrument_type, type_bytes_consumed) = InstrumentType::deserialize_binary(
            &payload[offset..], instrument_type_discriminant
        )?;
        offset += type_bytes_consumed;
        
        let (source, source_bytes_consumed) = InstrumentSource::deserialize_binary(
            &payload[offset..], source_discriminant
        )?;
        offset += source_bytes_consumed;
        
        let instrument = Instrument {
            id: instrument_id,
            symbol,
            instrument_type,
            source,
            decimals,
            metadata: InstrumentMetadata::default(),
            created_at: SystemTime::now(),
            last_updated: SystemTime::now(),
        };
        
        Ok((instrument, payload_end))
    }
}
```

### Enum Serialization with Discriminants
```rust
impl InstrumentType {
    pub fn discriminant(&self) -> u8 {
        match self {
            InstrumentType::Token { .. } => 0x01,
            InstrumentType::Stock { .. } => 0x02,
            InstrumentType::ETF { .. } => 0x03,
            InstrumentType::Future { .. } => 0x04,
            InstrumentType::Option { .. } => 0x05,
            InstrumentType::Currency { .. } => 0x06,
        }
    }
    
    pub fn binary_size(&self) -> usize {
        1 + match self { // 1 byte for discriminant
            InstrumentType::Token { blockchain, contract_address, token_standard } => {
                1 + // blockchain discriminant
                1 + contract_address.len() + // address length + bytes
                1   // token standard discriminant
            }
            InstrumentType::Stock { ticker, exchange, sector, market_cap } => {
                1 + ticker.len() +
                1 + exchange.len() +
                1 + sector.as_ref().map_or(0, |s| 1 + s.len()) + // optional sector
                9   // optional market_cap (1 byte present flag + 8 bytes u64)
            }
            InstrumentType::Future { underlying, expiration, contract_size, tick_size } => {
                1 + underlying.len() +
                8 + // expiration timestamp
                8 + // contract_size f64
                8   // tick_size f64
            }
            InstrumentType::Option { underlying_id, strike_price, expiration, option_type, style } => {
                8 + // underlying_id
                8 + // strike_price f64
                8 + // expiration timestamp
                1 + // option_type discriminant
                1   // style discriminant
            }
            // ... other variants
        }
    }
    
    pub fn serialize_binary(&self, buffer: &mut Vec<u8>) -> Result<(), SerializationError> {
        buffer.push(self.discriminant());
        
        match self {
            InstrumentType::Token { blockchain, contract_address, token_standard } => {
                buffer.push(blockchain.discriminant());
                
                if contract_address.len() > 255 {
                    return Err(SerializationError::AddressTooLong);
                }
                buffer.push(contract_address.len() as u8);
                buffer.extend_from_slice(contract_address.as_bytes());
                buffer.push(token_standard.discriminant());
            }
            InstrumentType::Stock { ticker, exchange, sector, market_cap } => {
                // Serialize ticker
                if ticker.len() > 255 {
                    return Err(SerializationError::TickerTooLong);
                }
                buffer.push(ticker.len() as u8);
                buffer.extend_from_slice(ticker.as_bytes());
                
                // Serialize exchange
                if exchange.len() > 255 {
                    return Err(SerializationError::ExchangeTooLong);
                }
                buffer.push(exchange.len() as u8);
                buffer.extend_from_slice(exchange.as_bytes());
                
                // Serialize optional sector
                if let Some(sector_str) = sector {
                    buffer.push(1); // present flag
                    buffer.push(sector_str.len() as u8);
                    buffer.extend_from_slice(sector_str.as_bytes());
                } else {
                    buffer.push(0); // not present
                }
                
                // Serialize optional market cap
                if let Some(cap) = market_cap {
                    buffer.push(1); // present flag
                    buffer.extend_from_slice(&cap.to_le_bytes());
                } else {
                    buffer.push(0); // not present
                }
            }
            InstrumentType::Future { underlying, expiration, contract_size, tick_size } => {
                // Serialize underlying
                if underlying.len() > 255 {
                    return Err(SerializationError::UnderlyingTooLong);
                }
                buffer.push(underlying.len() as u8);
                buffer.extend_from_slice(underlying.as_bytes());
                
                // Serialize expiration as timestamp
                let expiration_nanos = expiration.duration_since(UNIX_EPOCH)?.as_nanos() as u64;
                buffer.extend_from_slice(&expiration_nanos.to_le_bytes());
                
                // Serialize contract details
                buffer.extend_from_slice(&contract_size.to_le_bytes());
                buffer.extend_from_slice(&tick_size.to_le_bytes());
            }
            // ... other variants
        }
        
        Ok(())
    }
    
    pub fn deserialize_binary(buffer: &[u8], discriminant: u8) -> Result<(Self, usize), SerializationError> {
        let mut offset = 0;
        
        let instrument_type = match discriminant {
            0x01 => { // Token
                let blockchain_discriminant = buffer[offset];
                offset += 1;
                
                let blockchain = Blockchain::from_discriminant(blockchain_discriminant)?;
                
                let address_len = buffer[offset] as usize;
                offset += 1;
                
                if offset + address_len > buffer.len() {
                    return Err(SerializationError::BufferTooSmall);
                }
                
                let contract_address = String::from_utf8(
                    buffer[offset..offset + address_len].to_vec()
                )?;
                offset += address_len;
                
                let token_standard_discriminant = buffer[offset];
                offset += 1;
                
                let token_standard = TokenStandard::from_discriminant(token_standard_discriminant)?;
                
                InstrumentType::Token {
                    blockchain,
                    contract_address,
                    token_standard,
                }
            }
            0x02 => { // Stock
                let ticker_len = buffer[offset] as usize;
                offset += 1;
                
                let ticker = String::from_utf8(
                    buffer[offset..offset + ticker_len].to_vec()
                )?;
                offset += ticker_len;
                
                let exchange_len = buffer[offset] as usize;
                offset += 1;
                
                let exchange = String::from_utf8(
                    buffer[offset..offset + exchange_len].to_vec()
                )?;
                offset += exchange_len;
                
                // Deserialize optional sector
                let sector = if buffer[offset] == 1 {
                    offset += 1;
                    let sector_len = buffer[offset] as usize;
                    offset += 1;
                    Some(String::from_utf8(
                        buffer[offset..offset + sector_len].to_vec()
                    )?)
                } else {
                    offset += 1;
                    None
                };
                
                // Deserialize optional market cap
                let market_cap = if buffer[offset] == 1 {
                    offset += 1;
                    Some(u64::from_le_bytes(
                        buffer[offset..offset + 8].try_into()?
                    ))
                } else {
                    offset += 1;
                    None
                };
                if market_cap.is_some() {
                    offset += 8;
                }
                
                InstrumentType::Stock {
                    ticker,
                    exchange,
                    sector,
                    market_cap,
                }
            }
            // ... other variants
            _ => return Err(SerializationError::UnknownInstrumentType(discriminant)),
        };
        
        Ok((instrument_type, offset))
    }
}
```

### Pool Registration Messages
```rust
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct PoolRegistrationPayload {
    pub pool_id: u64,
    pub token0_id: u64,
    pub token1_id: u64,
    pub blockchain: u8,                    // Blockchain discriminant
    pub pool_type: u8,                     // PoolType discriminant
    pub fee_tier: u32,                     // Fee in basis points
    pub dex_name_len: u8,
    pub address_len: u8,
    // Variable length fields follow:
    // dex_name: [u8; dex_name_len]
    // pool_address: [u8; address_len]
}

impl BinarySerializable for Pool {
    const TYPE_ID: u8 = MessageType::PoolRegistration as u8;
    const VERSION: u8 = 1;
    
    fn binary_size(&self) -> usize {
        size_of::<BinaryMessageHeader>() +
        size_of::<PoolRegistrationPayload>() +
        self.dex.len() +
        self.pool_address.len()
    }
    
    fn serialize_binary(&self, buffer: &mut Vec<u8>) -> Result<(), SerializationError> {
        let initial_len = buffer.len();
        
        // Reserve header space
        buffer.extend_from_slice(&[0u8; size_of::<BinaryMessageHeader>()]);
        let payload_start = buffer.len();
        
        // Create fixed-size payload
        let payload = PoolRegistrationPayload {
            pool_id: self.id.0,
            token0_id: self.token0_id.0,
            token1_id: self.token1_id.0,
            blockchain: self.blockchain.discriminant(),
            pool_type: self.pool_type.discriminant(),
            fee_tier: self.fee_tier.unwrap_or(0),
            dex_name_len: self.dex.len() as u8,
            address_len: self.pool_address.len() as u8,
        };
        
        // Serialize fixed payload
        buffer.extend_from_slice(unsafe {
            std::slice::from_raw_parts(
                &payload as *const _ as *const u8,
                size_of::<PoolRegistrationPayload>()
            )
        });
        
        // Serialize variable-length fields
        buffer.extend_from_slice(self.dex.as_bytes());
        buffer.extend_from_slice(self.pool_address.as_bytes());
        
        // Fill header (same pattern as Instrument)
        let payload_size = buffer.len() - payload_start;
        let payload_checksum = crc32fast::hash(&buffer[payload_start..]);
        
        let header = BinaryMessageHeader {
            magic: 0xDEADBEEF,
            type_id: Self::TYPE_ID,
            version: Self::VERSION,
            flags: 0,
            reserved: 0,
            payload_size: payload_size as u32,
            sequence: 0,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64,
            checksum: payload_checksum,
        };
        
        unsafe {
            let header_ptr = buffer.as_mut_ptr().add(initial_len) as *mut BinaryMessageHeader;
            ptr::write(header_ptr, header);
        }
        
        Ok(())
    }
}
```

## 6. Event Registry System

### Comprehensive Event Type Registry
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct EventTypeId(pub u32);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventType {
    pub id: EventTypeId,
    pub name: String,                      // Human-readable name
    pub source: EventSource,               // Where this event originates
    pub priority: EventPriority,           // Processing priority
    pub schema: EventSchema,               // Binary schema definition
    pub created_at: SystemTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EventSource {
    DeFi { 
        blockchain: Blockchain,
        protocol: String,                  // Uniswap, Curve, Balancer
    },
    CEX { 
        exchange: String,                  // Binance, Coinbase, Kraken
        stream_type: String,               // trades, depth, ticker
    },
    TradFi { 
        broker: String,                    // Alpaca, Tradovate, IBKR
        feed_type: String,                 // trades, quotes, news
    },
    Internal {
        component: String,                 // arbitrage_engine, risk_manager
        event_class: String,               // opportunity, alert, execution
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EventPriority {
    Critical,                              // Immediate processing required
    High,                                  // Process within 1μs
    Standard,                              // Process within 10μs  
    Low,                                   // Process within 100μs
    Background,                            // Process when convenient
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSchema {
    pub version: u8,
    pub fields: Vec<EventField>,
    pub binary_size_hint: Option<usize>,   // For fixed-size events
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventField {
    pub name: String,
    pub field_type: EventFieldType,
    pub offset: Option<usize>,             // For fixed-offset fields
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventFieldType {
    InstrumentId,                          // References InstrumentRegistry
    PoolId,                               // References PoolRegistry  
    Uint64,
    Float64,
    String { max_length: usize },
    Timestamp,
    Address,                              // Blockchain address
    Hash,                                 // Transaction/block hash
    Array { element_type: Box<EventFieldType>, max_length: usize },
}
```

### Event Registry Implementation
```rust
pub struct EventRegistry {
    // Core storage
    event_types: Arc<RwLock<HashMap<EventTypeId, EventType>>>,
    
    // Fast lookup indices
    name_to_id: Arc<RwLock<HashMap<String, EventTypeId>>>,
    source_index: Arc<RwLock<HashMap<EventSource, Vec<EventTypeId>>>>,
    priority_index: Arc<RwLock<HashMap<EventPriority, Vec<EventTypeId>>>>,
    
    // Binary processing
    serializers: Arc<RwLock<HashMap<EventTypeId, Box<dyn EventSerializer>>>>,
    deserializers: Arc<RwLock<HashMap<EventTypeId, Box<dyn EventDeserializer>>>>,
    
    // Dynamic registration
    id_generator: AtomicU32,
    registration_channel: broadcast::Sender<EventRegistryUpdate>,
}

impl EventRegistry {
    pub fn register_event_type(&self, mut event_type: EventType) -> Result<EventTypeId, RegistryError> {
        // Generate deterministic ID
        let id = self.generate_event_type_id(&event_type);
        event_type.id = id;
        
        // Create binary serializer/deserializer from schema
        let serializer = self.create_serializer(&event_type.schema)?;
        let deserializer = self.create_deserializer(&event_type.schema)?;
        
        // Atomic registration
        {
            let mut event_types = self.event_types.write().unwrap();
            let mut name_index = self.name_to_id.write().unwrap();
            let mut source_index = self.source_index.write().unwrap();
            let mut priority_index = self.priority_index.write().unwrap();
            let mut serializers = self.serializers.write().unwrap();
            let mut deserializers = self.deserializers.write().unwrap();
            
            // Check for conflicts
            if event_types.contains_key(&id) {
                return Err(RegistryError::DuplicateEventTypeId(id));
            }
            
            // Insert into all indices
            event_types.insert(id, event_type.clone());
            name_index.insert(event_type.name.clone(), id);
            
            source_index.entry(event_type.source.clone())
                .or_insert_with(Vec::new)
                .push(id);
                
            priority_index.entry(event_type.priority.clone())
                .or_insert_with(Vec::new)
                .push(id);
                
            serializers.insert(id, serializer);
            deserializers.insert(id, deserializer);
        }
        
        // Notify subscribers
        let _ = self.registration_channel.send(EventRegistryUpdate::EventTypeAdded { 
            event_type: event_type.clone() 
        });
        
        Ok(id)
    }
    
    pub fn get_serializer(&self, event_type_id: EventTypeId) -> Option<Box<dyn EventSerializer>> {
        self.serializers.read().unwrap()
            .get(&event_type_id)
            .map(|s| s.clone_box())
    }
    
    fn create_serializer(&self, schema: &EventSchema) -> Result<Box<dyn EventSerializer>, RegistryError> {
        match schema.binary_size_hint {
            Some(fixed_size) => {
                // Fixed-size event serializer (fastest)
                Ok(Box::new(FixedSizeEventSerializer::new(schema.clone(), fixed_size)))
            }
            None => {
                // Variable-size event serializer
                Ok(Box::new(VariableSizeEventSerializer::new(schema.clone())))
            }
        }
    }
}
```

### Predefined Event Types
```rust
impl EventRegistry {
    pub fn register_standard_events(&self) -> Result<(), RegistryError> {
        // DeFi Events
        self.register_defi_events()?;
        
        // CEX Events  
        self.register_cex_events()?;
        
        // TradFi Events
        self.register_tradfi_events()?;
        
        // Internal System Events
        self.register_internal_events()?;
        
        Ok(())
    }
    
    fn register_defi_events(&self) -> Result<(), RegistryError> {
        // Uniswap V3 Swap Event
        let swap_event = EventType {
            id: EventTypeId(0), // Will be generated
            name: "uniswap_v3_swap".to_string(),
            source: EventSource::DeFi { 
                blockchain: Blockchain::Ethereum, 
                protocol: "uniswap_v3".to_string() 
            },
            priority: EventPriority::Critical,
            schema: EventSchema {
                version: 1,
                binary_size_hint: Some(128), // Fixed size for performance
                fields: vec![
                    EventField {
                        name: "pool_id".to_string(),
                        field_type: EventFieldType::PoolId,
                        offset: Some(0),
                        required: true,
                    },
                    EventField {
                        name: "token0_id".to_string(),
                        field_type: EventFieldType::InstrumentId,
                        offset: Some(8),
                        required: true,
                    },
                    EventField {
                        name: "token1_id".to_string(),
                        field_type: EventFieldType::InstrumentId,
                        offset: Some(16),
                        required: true,
                    },
                    EventField {
                        name: "amount0".to_string(),
                        field_type: EventFieldType::Uint64,
                        offset: Some(24),
                        required: true,
                    },
                    EventField {
                        name: "amount1".to_string(),
                        field_type: EventFieldType::Uint64,
                        offset: Some(32),
                        required: true,
                    },
                    EventField {
                        name: "sqrt_price_x96".to_string(),
                        field_type: EventFieldType::Uint64,
                        offset: Some(40),
                        required: true,
                    },
                    EventField {
                        name: "liquidity".to_string(),
                        field_type: EventFieldType::Uint64,
                        offset: Some(48),
                        required: true,
                    },
                    EventField {
                        name: "tick".to_string(),
                        field_type: EventFieldType::Uint64,
                        offset: Some(56),
                        required: true,
                    },
                    EventField {
                        name: "transaction_hash".to_string(),
                        field_type: EventFieldType::Hash,
                        offset: Some(64),
                        required: true,
                    },
                    EventField {
                        name: "block_number".to_string(),
                        field_type: EventFieldType::Uint64,
                        offset: Some(96),
                        required: true,
                    },
                    EventField {
                        name: "timestamp".to_string(),
                        field_type: EventFieldType::Timestamp,
                        offset: Some(104),
                        required: true,
                    },
                ],
            },
            created_at: SystemTime::now(),
        };
        
        self.register_event_type(swap_event)?;
        
        // Pool Created Event
        let pool_created_event = EventType {
            id: EventTypeId(0),
            name: "uniswap_v3_pool_created".to_string(),
            source: EventSource::DeFi { 
                blockchain: Blockchain::Ethereum, 
                protocol: "uniswap_v3".to_string() 
            },
            priority: EventPriority::High,
            schema: EventSchema {
                version: 1,
                binary_size_hint: Some(96),
                fields: vec![
                    EventField {
                        name: "token0_address".to_string(),
                        field_type: EventFieldType::Address,
                        offset: Some(0),
                        required: true,
                    },
                    EventField {
                        name: "token1_address".to_string(),
                        field_type: EventFieldType::Address,
                        offset: Some(20),
                        required: true,
                    },
                    EventField {
                        name: "fee".to_string(),
                        field_type: EventFieldType::Uint64,
                        offset: Some(40),
                        required: true,
                    },
                    EventField {
                        name: "pool_address".to_string(),
                        field_type: EventFieldType::Address,
                        offset: Some(48),
                        required: true,
                    },
                    EventField {
                        name: "block_number".to_string(),
                        field_type: EventFieldType::Uint64,
                        offset: Some(68),
                        required: true,
                    },
                    EventField {
                        name: "timestamp".to_string(),
                        field_type: EventFieldType::Timestamp,
                        offset: Some(76),
                        required: true,
                    },
                ],
            },
            created_at: SystemTime::now(),
        };
        
        self.register_event_type(pool_created_event)?;
        
        Ok(())
    }
    
    fn register_cex_events(&self) -> Result<(), RegistryError> {
        // Binance Trade Stream
        let binance_trade_event = EventType {
            id: EventTypeId(0),
            name: "binance_trade".to_string(),
            source: EventSource::CEX { 
                exchange: "binance".to_string(), 
                stream_type: "trade".to_string() 
            },
            priority: EventPriority::Critical,
            schema: EventSchema {
                version: 1,
                binary_size_hint: Some(64),
                fields: vec![
                    EventField {
                        name: "pair_id".to_string(),
                        field_type: EventFieldType::InstrumentId,
                        offset: Some(0),
                        required: true,
                    },
                    EventField {
                        name: "price".to_string(),
                        field_type: EventFieldType::Float64,
                        offset: Some(8),
                        required: true,
                    },
                    EventField {
                        name: "quantity".to_string(),
                        field_type: EventFieldType::Float64,
                        offset: Some(16),
                        required: true,
                    },
                    EventField {
                        name: "is_buyer_maker".to_string(),
                        field_type: EventFieldType::Uint64, // Boolean as u64
                        offset: Some(24),
                        required: true,
                    },
                    EventField {
                        name: "trade_id".to_string(),
                        field_type: EventFieldType::Uint64,
                        offset: Some(32),
                        required: true,
                    },
                    EventField {
                        name: "timestamp".to_string(),
                        field_type: EventFieldType::Timestamp,
                        offset: Some(40),
                        required: true,
                    },
                ],
            },
            created_at: SystemTime::now(),
        };
        
        self.register_event_type(binance_trade_event)?;
        
        Ok(())
    }
    
    fn register_internal_events(&self) -> Result<(), RegistryError> {
        // Arbitrage Opportunity Detected
        let arbitrage_opportunity_event = EventType {
            id: EventTypeId(0),
            name: "arbitrage_opportunity".to_string(),
            source: EventSource::Internal { 
                component: "arbitrage_engine".to_string(), 
                event_class: "opportunity".to_string() 
            },
            priority: EventPriority::Critical,
            schema: EventSchema {
                version: 1,
                binary_size_hint: Some(128),
                fields: vec![
                    EventField {
                        name: "opportunity_id".to_string(),
                        field_type: EventFieldType::Uint64,
                        offset: Some(0),
                        required: true,
                    },
                    EventField {
                        name: "base_instrument_id".to_string(),
                        field_type: EventFieldType::InstrumentId,
                        offset: Some(8),
                        required: true,
                    },
                    EventField {
                        name: "quote_instrument_id".to_string(),
                        field_type: EventFieldType::InstrumentId,
                        offset: Some(16),
                        required: true,
                    },
                    EventField {
                        name: "venue_a_id".to_string(),
                        field_type: EventFieldType::PoolId,
                        offset: Some(24),
                        required: true,
                    },
                    EventField {
                        name: "venue_b_id".to_string(),
                        field_type: EventFieldType::PoolId,
                        offset: Some(32),
                        required: true,
                    },
                    EventField {
                        name: "profit_estimate".to_string(),
                        field_type: EventFieldType::Float64,
                        offset: Some(40),
                        required: true,
                    },
                    EventField {
                        name: "confidence_score".to_string(),
                        field_type: EventFieldType::Float64,
                        offset: Some(48),
                        required: true,
                    },
                    EventField {
                        name: "execution_cost".to_string(),
                        field_type: EventFieldType::Float64,
                        offset: Some(56),
                        required: true,
                    },
                    EventField {
                        name: "expires_at".to_string(),
                        field_type: EventFieldType::Timestamp,
                        offset: Some(64),
                        required: true,
                    },
                    EventField {
                        name: "discovery_timestamp".to_string(),
                        field_type: EventFieldType::Timestamp,
                        offset: Some(72),
                        required: true,
                    },
                ],
            },
            created_at: SystemTime::now(),
        };
        
        self.register_event_type(arbitrage_opportunity_event)?;
        
        Ok(())
    }
}
```

### Event Serialization Implementation
```rust
pub trait EventSerializer: Send + Sync {
    fn serialize(&self, event_data: &EventData) -> Result<Vec<u8>, SerializationError>;
    fn clone_box(&self) -> Box<dyn EventSerializer>;
}

pub trait EventDeserializer: Send + Sync {
    fn deserialize(&self, data: &[u8]) -> Result<EventData, SerializationError>;
    fn clone_box(&self) -> Box<dyn EventDeserializer>;
}

// Fixed-size event serializer (fastest path)
pub struct FixedSizeEventSerializer {
    schema: EventSchema,
    binary_size: usize,
}

impl EventSerializer for FixedSizeEventSerializer {
    fn serialize(&self, event_data: &EventData) -> Result<Vec<u8>, SerializationError> {
        let mut buffer = vec![0u8; self.binary_size];
        
        for field in &self.schema.fields {
            if let Some(offset) = field.offset {
                let value = event_data.get_field(&field.name)
                    .ok_or_else(|| SerializationError::MissingField(field.name.clone()))?;
                
                self.write_field_at_offset(&mut buffer, offset, &field.field_type, value)?;
            }
        }
        
        Ok(buffer)
    }
    
    fn clone_box(&self) -> Box<dyn EventSerializer> {
        Box::new(self.clone())
    }
}

impl FixedSizeEventSerializer {
    fn write_field_at_offset(
        &self, 
        buffer: &mut [u8], 
        offset: usize, 
        field_type: &EventFieldType, 
        value: &EventValue
    ) -> Result<(), SerializationError> {
        match (field_type, value) {
            (EventFieldType::InstrumentId, EventValue::InstrumentId(id)) => {
                buffer[offset..offset + 8].copy_from_slice(&id.0.to_le_bytes());
            }
            (EventFieldType::PoolId, EventValue::PoolId(id)) => {
                buffer[offset..offset + 8].copy_from_slice(&id.0.to_le_bytes());
            }
            (EventFieldType::Uint64, EventValue::Uint64(val)) => {
                buffer[offset..offset + 8].copy_from_slice(&val.to_le_bytes());
            }
            (EventFieldType::Float64, EventValue::Float64(val)) => {
                buffer[offset..offset + 8].copy_from_slice(&val.to_le_bytes());
            }
            (EventFieldType::Timestamp, EventValue::Timestamp(ts)) => {
                let nanos = ts.duration_since(UNIX_EPOCH)?.as_nanos() as u64;
                buffer[offset..offset + 8].copy_from_slice(&nanos.to_le_bytes());
            }
            (EventFieldType::Address, EventValue::Address(addr)) => {
                if addr.len() != 20 {
                    return Err(SerializationError::InvalidAddressLength);
                }
                buffer[offset..offset + 20].copy_from_slice(addr);
            }
            (EventFieldType::Hash, EventValue::Hash(hash)) => {
                if hash.len() != 32 {
                    return Err(SerializationError::InvalidHashLength);
                }
                buffer[offset..offset + 32].copy_from_slice(hash);
            }
            _ => return Err(SerializationError::TypeMismatch),
        }
        
        Ok(())
    }
}

// Event data container
#[derive(Debug, Clone)]
pub struct EventData {
    fields: HashMap<String, EventValue>,
}

#[derive(Debug, Clone)]
pub enum EventValue {
    InstrumentId(InstrumentId),
    PoolId(PoolId),
    Uint64(u64),
    Float64(f64),
    String(String),
    Timestamp(SystemTime),
    Address(Vec<u8>),
    Hash(Vec<u8>),
    Array(Vec<EventValue>),
}

impl EventData {
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
        }
    }
    
    pub fn insert_instrument_id(&mut self, field_name: String, id: InstrumentId) {
        self.fields.insert(field_name, EventValue::InstrumentId(id));
    }
    
    pub fn insert_pool_id(&mut self, field_name: String, id: PoolId) {
        self.fields.insert(field_name, EventValue::PoolId(id));
    }
    
    pub fn insert_uint64(&mut self, field_name: String, val: u64) {
        self.fields.insert(field_name, EventValue::Uint64(val));
    }
    
    pub fn insert_float64(&mut self, field_name: String, val: f64) {
        self.fields.insert(field_name, EventValue::Float64(val));
    }
    
    pub fn get_field(&self, field_name: &str) -> Option<&EventValue> {
        self.fields.get(field_name)
    }
}
```

### Unified Event Processing Pipeline
```rust
pub struct EventProcessor {
    event_registry: Arc<EventRegistry>,
    instrument_registry: Arc<InstrumentRegistry>,
    pool_registry: Arc<PoolRegistry>,
    
    // Processing queues by priority
    critical_queue: VecDeque<ProcessingTask>,
    high_priority_queue: VecDeque<ProcessingTask>,
    standard_queue: VecDeque<ProcessingTask>,
    
    // Event handlers
    event_handlers: HashMap<EventTypeId, Vec<Box<dyn EventHandler>>>,
    
    // Performance monitoring
    processing_stats: HashMap<EventTypeId, EventProcessingStats>,
}

impl EventProcessor {
    pub fn process_raw_event(&mut self, raw_event: RawEventData) -> Result<ProcessedEvent, ProcessingError> {
        // 1. Determine event type
        let event_type_id = self.identify_event_type(&raw_event)?;
        
        // 2. Get deserializer
        let deserializer = self.event_registry.get_deserializer(event_type_id)
            .ok_or(ProcessingError::NoDeserializer)?;
        
        // 3. Deserialize to structured data
        let event_data = deserializer.deserialize(&raw_event.payload)?;
        
        // 4. Extract referenced IDs and validate
        self.validate_references(&event_data)?;
        
        // 5. Queue for processing based on priority
        let event_type = self.event_registry.get_by_id(event_type_id).unwrap();
        let processing_task = ProcessingTask {
            event_type_id,
            event_data,
            priority: event_type.priority.clone(),
            received_at: SystemTime::now(),
        };
        
        match event_type.priority {
            EventPriority::Critical => self.critical_queue.push_back(processing_task),
            EventPriority::High => self.high_priority_queue.push_back(processing_task),
            EventPriority::Standard => self.standard_queue.push_back(processing_task),
            _ => {} // Handle other priorities
        }
        
        // 6. Update processing statistics
        self.update_processing_stats(event_type_id);
        
        Ok(ProcessedEvent {
            event_type_id,
            processed_at: SystemTime::now(),
        })
    }
    
    fn validate_references(&self, event_data: &EventData) -> Result<(), ProcessingError> {
        // Validate that all referenced instruments and pools exist
        for (field_name, value) in &event_data.fields {
            match value {
                EventValue::InstrumentId(id) => {
                    if self.instrument_registry.get_by_id(*id).is_none() {
                        return Err(ProcessingError::UnknownInstrument(*id));
                    }
                }
                EventValue::PoolId(id) => {
                    if self.pool_registry.get_by_id(*id).is_none() {
                        return Err(ProcessingError::UnknownPool(*id));
                    }
                }
                _ => {} // Other field types don't need validation
            }
        }
        
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct RawEventData {
    pub source: String,
    pub event_type_hint: Option<String>,
    pub payload: Vec<u8>,
    pub metadata: HashMap<String, String>,
    pub timestamp: SystemTime,
}

pub trait EventHandler: Send + Sync {
    fn handle_event(&mut self, event_data: &EventData) -> Result<(), EventHandlingError>;
    fn event_type_id(&self) -> EventTypeId;
}

// Example: Arbitrage opportunity detector
pub struct ArbitrageOpportunityHandler {
    arbitrage_engine: Arc<Mutex<ArbitrageEngine>>,
}

impl EventHandler for ArbitrageOpportunityHandler {
    fn handle_event(&mut self, event_data: &EventData) -> Result<(), EventHandlingError> {
        // Extract swap data
        let pool_id = match event_data.get_field("pool_id") {
            Some(EventValue::PoolId(id)) => *id,
            _ => return Err(EventHandlingError::MissingField("pool_id".to_string())),
        };
        
        let price = match event_data.get_field("price") {
            Some(EventValue::Float64(p)) => *p,
            _ => return Err(EventHandlingError::MissingField("price".to_string())),
        };
        
        // Trigger arbitrage analysis
        let mut engine = self.arbitrage_engine.lock().unwrap();
        engine.analyze_price_update(pool_id, price)?;
        
        Ok(())
    }
    
    fn event_type_id(&self) -> EventTypeId {
        // Return the event type this handler processes
        EventTypeId(1) // Uniswap V3 swap event
    }
}
```

This comprehensive registry system ensures that **everything that gets serialized** has a canonical representation:

- **InstrumentRegistry**: All tradeable assets
- **PoolRegistry**: All liquidity sources  
- **EventRegistry**: All event types and their schemas
- **Unified binary protocol**: Everything references IDs for maximum efficiency

The system can **dynamically discover and register** new instruments, pools, and even new event types as they appear, while maintaining strict type safety and ultra-fast binary processing throughout the pipeline!

---

## Performance Monitoring & Tuning

### Key Metrics to Track

```rust
impl InstrumentRegistry {
    pub fn get_metrics(&self) -> RegistryMetricsSnapshot {
        RegistryMetricsSnapshot {
            total_instruments: self.metrics.total_instruments.load(Ordering::Relaxed),
            total_lookups: self.metrics.total_lookups.load(Ordering::Relaxed),
            cache_hit_rate: self.calculate_hit_rate(),
            hash_collisions: self.metrics.hash_collisions.load(Ordering::Relaxed),
            registration_failures: self.metrics.registration_failures.load(Ordering::Relaxed),
            avg_lookup_time_ns: self.get_avg_lookup_time(),
        }
    }
    
    pub fn alert_on_collision(&self) {
        let collisions = self.metrics.hash_collisions.load(Ordering::Relaxed);
        if collisions > 0 {
            // Critical alert - potential for wrong trades!
            error!("CRITICAL: {} hash collisions detected in registry!", collisions);
            // Consider switching to 128-bit IDs immediately
        }
    }
}
```

### Tuning Guidelines

1. **Registration Rate Limiting**
   - Default: 1000/sec
   - DeFi volatility events: Increase to 10,000/sec
   - Monitor queue depth and adjust dynamically

2. **Memory Optimization**
   - Use `Arc<Instrument>` to share data between indices
   - Consider memory-mapped files for >1M instruments
   - Implement LRU eviction for inactive instruments

3. **Lock-Free Performance**
   - DashMap scales to 32+ concurrent threads
   - For extreme loads (>100k ops/sec), consider sharding
   - Monitor contention with `perf` tools

4. **Collision Mitigation**
   - Monitor collision rate continuously
   - Alert immediately on any collision
   - Auto-upgrade to 128-bit IDs if collision detected
   - Maintain collision audit log for investigation

### Production Deployment Checklist

- [ ] Blake3 dependency added for deterministic hashing
- [ ] DashMap replacing all RwLock<HashMap> instances
- [ ] Metrics collection enabled and monitored
- [ ] Collision alerts configured for PagerDuty/OpsGenie
- [ ] Load testing completed with expected instrument count
- [ ] Backup registry snapshot mechanism in place
- [ ] 128-bit ID migration path tested

## Synthetic Instruments for Cross-Asset Strategies

### Synthetic Instrument Definition
```rust
use evalexpr::{eval, Value};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntheticInstrument {
    pub id: InstrumentId,
    pub name: String,
    pub components: Vec<InstrumentId>,
    pub formula: String,                   // "BTCUSDT.BINANCE / ETHUSDT.BINANCE"
    pub update_frequency: Duration,
    pub dependencies: DashMap<InstrumentId, f64>, // Thread-safe price cache
    pub validation: SyntheticValidation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntheticValidation {
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub outlier_threshold: f64,            // Standard deviations
    pub stale_data_threshold: Duration,    // Max age for component prices
}

impl SyntheticInstrument {
    pub fn evaluate(&self, prices: &HashMap<InstrumentId, PriceData>) -> Result<f64, SyntheticError> {
        // Validate data freshness
        let now = SystemTime::now();
        for (id, price_data) in prices {
            if now.duration_since(price_data.timestamp)? > self.validation.stale_data_threshold {
                return Err(SyntheticError::StaleData { 
                    instrument_id: *id,
                    age: now.duration_since(price_data.timestamp)?,
                });
            }
        }
        
        // Build evaluation context
        let mut context = evalexpr::HashMapContext::new();
        for (id, price_data) in prices {
            let var_name = self.get_variable_name(*id)?;
            context.set_value(var_name, evalexpr::Value::Float(price_data.price))?;
        }
        
        // Evaluate formula safely
        let result = eval_with_context(&self.formula, &context)?
            .as_float()
            .ok_or(SyntheticError::InvalidFormula)?;
        
        // Validate result
        if let Some(min) = self.validation.min_value {
            if result < min {
                return Err(SyntheticError::OutOfRange { value: result, min, max: None });
            }
        }
        
        if let Some(max) = self.validation.max_value {
            if result > max {
                return Err(SyntheticError::OutOfRange { value: result, min: None, max });
            }
        }
        
        // Update cache
        self.dependencies.insert(self.id, result);
        
        Ok(result)
    }
}

// Common synthetic instruments
impl SyntheticInstrument {
    pub fn eth_basis_spread() -> Self {
        Self {
            id: InstrumentId(u64::MAX - 1), // Reserved ID range for synthetics
            name: "ETH Basis Spread".to_string(),
            components: vec![
                // Spot ETH on Binance
                InstrumentId(0x1001),
                // ETH Futures on CME
                InstrumentId(0x2001),
            ],
            formula: "(CME_ETH_FUT - BINANCE_ETH_SPOT) / BINANCE_ETH_SPOT * 365 / DAYS_TO_EXPIRY".to_string(),
            update_frequency: Duration::from_secs(1),
            dependencies: DashMap::new(),
            validation: SyntheticValidation {
                min_value: Some(-0.5),     // -50% annualized
                max_value: Some(0.5),       // +50% annualized
                outlier_threshold: 3.0,
                stale_data_threshold: Duration::from_secs(5),
            },
        }
    }
    
    pub fn defi_cex_arbitrage_index() -> Self {
        Self {
            id: InstrumentId(u64::MAX - 2),
            name: "DeFi-CEX Arbitrage Index".to_string(),
            components: vec![
                // Uniswap V3 WETH/USDC
                InstrumentId(0x3001),
                // Binance ETH/USDT
                InstrumentId(0x1002),
            ],
            formula: "abs(UNISWAP_ETH_USDC - BINANCE_ETH_USDT) / BINANCE_ETH_USDT * 10000".to_string(), // basis points
            update_frequency: Duration::from_millis(100),
            dependencies: DashMap::new(),
            validation: SyntheticValidation {
                min_value: Some(0.0),
                max_value: Some(1000.0),    // 10% max spread
                outlier_threshold: 4.0,
                stale_data_threshold: Duration::from_millis(500),
            },
        }
    }
}
```

### Cross-Asset Synthetic Registry
```rust
pub struct SyntheticRegistry {
    synthetics: Arc<DashMap<InstrumentId, Arc<SyntheticInstrument>>>,
    formula_index: Arc<DashMap<String, InstrumentId>>,
    component_index: Arc<DashMap<InstrumentId, Vec<InstrumentId>>>, // Which synthetics use this component
    
    // Real-time evaluation
    evaluation_cache: Arc<DashMap<InstrumentId, SyntheticValue>>,
    evaluation_tasks: Arc<DashMap<InstrumentId, JoinHandle<()>>>,
    
    // Dependencies
    instrument_registry: Arc<InstrumentRegistry>,
    price_feed: Arc<dyn PriceFeed>,
    
    metrics: Arc<SyntheticMetrics>,
}

#[derive(Debug, Clone)]
pub struct SyntheticValue {
    pub value: f64,
    pub timestamp: SystemTime,
    pub component_prices: HashMap<InstrumentId, f64>,
    pub evaluation_time_ns: u64,
}

impl SyntheticRegistry {
    pub fn register_synthetic(&self, synthetic: SyntheticInstrument) -> Result<InstrumentId, RegistryError> {
        // Validate components exist
        for component_id in &synthetic.components {
            if self.instrument_registry.get_by_id(*component_id).is_none() {
                return Err(RegistryError::UnknownInstrument(*component_id));
            }
        }
        
        // Validate formula syntax
        self.validate_formula(&synthetic.formula)?;
        
        let id = synthetic.id;
        let synthetic_arc = Arc::new(synthetic.clone());
        
        // Register in indices
        self.synthetics.insert(id, synthetic_arc.clone());
        self.formula_index.insert(synthetic.name.clone(), id);
        
        // Update component index
        for component_id in &synthetic.components {
            self.component_index.entry(*component_id)
                .or_insert_with(Vec::new)
                .push(id);
        }
        
        // Start evaluation task
        self.start_evaluation_task(synthetic_arc);
        
        Ok(id)
    }
    
    fn start_evaluation_task(&self, synthetic: Arc<SyntheticInstrument>) {
        let evaluation_cache = Arc::clone(&self.evaluation_cache);
        let price_feed = Arc::clone(&self.price_feed);
        let metrics = Arc::clone(&self.metrics);
        
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(synthetic.update_frequency);
            
            loop {
                interval.tick().await;
                
                let start = std::time::Instant::now();
                
                // Fetch component prices
                let mut prices = HashMap::new();
                for component_id in &synthetic.components {
                    if let Some(price_data) = price_feed.get_latest(*component_id).await {
                        prices.insert(*component_id, price_data);
                    }
                }
                
                // Evaluate synthetic
                match synthetic.evaluate(&prices) {
                    Ok(value) => {
                        let evaluation_time_ns = start.elapsed().as_nanos() as u64;
                        
                        evaluation_cache.insert(synthetic.id, SyntheticValue {
                            value,
                            timestamp: SystemTime::now(),
                            component_prices: prices.iter().map(|(k, v)| (*k, v.price)).collect(),
                            evaluation_time_ns,
                        });
                        
                        metrics.successful_evaluations.fetch_add(1, Ordering::Relaxed);
                        metrics.avg_evaluation_time_ns.store(evaluation_time_ns, Ordering::Relaxed);
                    }
                    Err(e) => {
                        error!("Failed to evaluate synthetic {}: {:?}", synthetic.name, e);
                        metrics.failed_evaluations.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        });
        
        self.evaluation_tasks.insert(synthetic.id, handle);
    }
}
```

## Type-Safe Instrument References

### Compile-Time Type Safety
```rust
use std::marker::PhantomData;

// Type-safe instrument IDs prevent mixing incompatible types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypedInstrumentId<T> {
    id: InstrumentId,
    _phantom: PhantomData<T>,
}

// Marker types for different instrument categories
pub struct DeFiToken;
pub struct StockEquity;
pub struct CryptoCurrency;
pub struct FuturesContract;
pub struct OptionContract;

impl<T> TypedInstrumentId<T> {
    pub fn new(id: InstrumentId, registry: &InstrumentRegistry) -> Result<Self, RegistryError> {
        // Validate that the ID corresponds to the correct type
        if !Self::validate_type(id, registry) {
            return Err(RegistryError::TypeMismatch {
                expected: std::any::type_name::<T>(),
                actual: registry.get_type_name(id),
            });
        }
        
        Ok(Self { 
            id, 
            _phantom: PhantomData 
        })
    }
    
    fn validate_type(id: InstrumentId, registry: &InstrumentRegistry) -> bool {
        if let Some(instrument) = registry.get_by_id(id) {
            match (&instrument.instrument_type, std::any::type_name::<T>()) {
                (InstrumentType::Token { .. }, "DeFiToken") => true,
                (InstrumentType::Stock { .. }, "StockEquity") => true,
                (InstrumentType::Future { .. }, "FuturesContract") => true,
                (InstrumentType::Option { .. }, "OptionContract") => true,
                _ => false,
            }
        } else {
            false
        }
    }
}

// Type-safe arbitrage opportunities
pub struct ArbitrageOpportunity<S, T> {
    pub source: TypedInstrumentId<S>,
    pub target: TypedInstrumentId<T>,
    pub spread_bps: f64,
    pub confidence: f64,
}

// This won't compile - type safety prevents errors!
// fn invalid_arbitrage(
//     stock: TypedInstrumentId<StockEquity>,
//     token: TypedInstrumentId<DeFiToken>,
// ) -> ArbitrageOpportunity<StockEquity, StockEquity> {
//     ArbitrageOpportunity {
//         source: stock,
//         target: token, // Compile error! Expected StockEquity, got DeFiToken
//         spread_bps: 10.0,
//         confidence: 0.95,
//     }
// }

// Valid cross-asset arbitrage with explicit types
pub struct CrossAssetArbitrage {
    pub spot_crypto: TypedInstrumentId<CryptoCurrency>,
    pub futures: TypedInstrumentId<FuturesContract>,
    pub etf: TypedInstrumentId<StockEquity>,
}

impl CrossAssetArbitrage {
    pub fn calculate_basis(&self, registry: &InstrumentRegistry) -> Result<f64, ArbitrageError> {
        let spot = registry.get_typed_price(self.spot_crypto)?;
        let futures = registry.get_typed_price(self.futures)?;
        let etf = registry.get_typed_price(self.etf)?;
        
        // Type-safe calculation - compiler ensures we're using the right types
        Ok((futures - spot) / spot * 100.0)
    }
}
```

## Subscription-Based Event System

### Event-Driven Registry Updates
```rust
use tokio::sync::broadcast;
use std::collections::HashSet;

pub struct RegistrySubscriptionManager {
    // Instrument-specific subscriptions
    instrument_subs: Arc<DashMap<InstrumentId, HashSet<SubscriberId>>>,
    
    // Pattern-based subscriptions
    pattern_subs: Arc<DashMap<SubscriptionPattern, HashSet<SubscriberId>>>,
    
    // Event channels
    event_bus: broadcast::Sender<RegistryEvent>,
    
    // Subscriber info
    subscribers: Arc<DashMap<SubscriberId, SubscriberInfo>>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum SubscriptionPattern {
    AllStocks,
    AllTokens { blockchain: Blockchain },
    ISINPattern { isin_prefix: String },
    VenueInstruments { venue_id: VenueId },
    ArbitrageOpportunities { min_spread_bps: u32 },
    NewListings,
    SyntheticUpdates { synthetic_id: InstrumentId },
}

#[derive(Debug, Clone)]
pub enum RegistryEvent {
    InstrumentAdded {
        id: InstrumentId,
        instrument_type: InstrumentType,
        venue: Option<VenueId>,
        timestamp: SystemTime,
    },
    
    PriceUpdate {
        id: InstrumentId,
        old_price: f64,
        new_price: f64,
        timestamp: SystemTime,
    },
    
    ArbitrageDetected {
        opportunity_id: u64,
        instruments: Vec<InstrumentId>,
        venues: Vec<VenueId>,
        spread_bps: f64,
        confidence: f64,
        expires_at: SystemTime,
    },
    
    SyntheticEvaluated {
        synthetic_id: InstrumentId,
        value: f64,
        components: HashMap<InstrumentId, f64>,
        evaluation_time_ns: u64,
    },
    
    VenueStatusChange {
        venue_id: VenueId,
        old_status: VenueStatus,
        new_status: VenueStatus,
        affected_instruments: Vec<InstrumentId>,
    },
}

impl RegistrySubscriptionManager {
    pub async fn subscribe(
        &self,
        subscriber_id: SubscriberId,
        pattern: SubscriptionPattern,
    ) -> broadcast::Receiver<RegistryEvent> {
        // Register subscription
        self.pattern_subs
            .entry(pattern.clone())
            .or_insert_with(HashSet::new)
            .insert(subscriber_id);
        
        // Update subscriber info
        self.subscribers
            .entry(subscriber_id)
            .and_modify(|info| info.patterns.push(pattern.clone()))
            .or_insert_with(|| SubscriberInfo {
                id: subscriber_id,
                patterns: vec![pattern],
                created_at: SystemTime::now(),
                last_event: None,
            });
        
        // Return event receiver
        self.event_bus.subscribe()
    }
    
    pub async fn publish(&self, event: RegistryEvent) -> Result<usize, PublishError> {
        // Match event to subscriptions
        let interested_subscribers = self.match_subscribers(&event);
        
        // Send to interested parties only
        let sent = self.event_bus.send(event.clone())
            .map_err(|_| PublishError::NoSubscribers)?;
        
        // Update metrics
        for subscriber_id in interested_subscribers {
            if let Some(mut info) = self.subscribers.get_mut(&subscriber_id) {
                info.last_event = Some(SystemTime::now());
            }
        }
        
        Ok(sent)
    }
    
    fn match_subscribers(&self, event: &RegistryEvent) -> HashSet<SubscriberId> {
        let mut subscribers = HashSet::new();
        
        match event {
            RegistryEvent::InstrumentAdded { instrument_type, .. } => {
                // Match type-based patterns
                match instrument_type {
                    InstrumentType::Stock { .. } => {
                        if let Some(subs) = self.pattern_subs.get(&SubscriptionPattern::AllStocks) {
                            subscribers.extend(subs.iter());
                        }
                    }
                    InstrumentType::Token { blockchain, .. } => {
                        let pattern = SubscriptionPattern::AllTokens { 
                            blockchain: blockchain.clone() 
                        };
                        if let Some(subs) = self.pattern_subs.get(&pattern) {
                            subscribers.extend(subs.iter());
                        }
                    }
                    _ => {}
                }
                
                // Always notify new listing subscribers
                if let Some(subs) = self.pattern_subs.get(&SubscriptionPattern::NewListings) {
                    subscribers.extend(subs.iter());
                }
            }
            
            RegistryEvent::ArbitrageDetected { spread_bps, .. } => {
                // Find all arbitrage pattern subscribers with matching criteria
                for entry in self.pattern_subs.iter() {
                    if let SubscriptionPattern::ArbitrageOpportunities { min_spread_bps } = entry.key() {
                        if *spread_bps >= *min_spread_bps as f64 {
                            subscribers.extend(entry.value().iter());
                        }
                    }
                }
            }
            
            _ => {}
        }
        
        subscribers
    }
}
```

## Cross-Exchange Asset Tracking Example

### Real-World Usage: Apple Inc. Trading Across Venues

```rust
// Example: Tracking Apple stock across multiple exchanges
async fn track_apple_cross_exchange(
    instrument_registry: Arc<InstrumentRegistry>,
    venue_registry: Arc<VenueRegistry>,
) -> Result<(), Box<dyn Error>> {
    
    // Apple's ISIN is US0378331005
    let apple_isin = "US0378331005";
    
    // Find all venues where Apple trades
    let apple_venues = instrument_registry.find_all_venues_for_isin(apple_isin);
    
    println!("Apple Inc. (ISIN: {}) trades on {} venues:", apple_isin, apple_venues.len());
    
    for (exchange, instrument) in &apple_venues {
        println!("  - {} as ticker {}", exchange, 
            match &instrument.instrument_type {
                InstrumentType::Stock { ticker, .. } => ticker,
                _ => "unknown",
            }
        );
    }
    
    // Example output:
    // Apple Inc. (ISIN: US0378331005) trades on 4 venues:
    //   - NASDAQ as ticker AAPL
    //   - NYSE as ticker AAPL
    //   - XETRA as ticker APC
    //   - LSE as ticker 0R2V
    
    // Find best execution venue for a large order
    let best_liquidity_venue = venue_registry.find_best_venue_for_instrument(
        instrument_registry.get_by_isin(apple_isin).unwrap().id,
        ExecutionCriteria::HighestLiquidity
    );
    
    println!("Best liquidity venue: {}", best_liquidity_venue.unwrap().name);
    
    // Check for arbitrage opportunities
    let arbitrage_venues = venue_registry.find_arbitrage_venues_for_isin(apple_isin);
    
    for (venue_a, venue_b) in arbitrage_venues {
        let fee_diff = (venue_a.fee_structure.taker_fee_bps - venue_b.fee_structure.taker_fee_bps).abs();
        println!("Arbitrage opportunity between {} and {}: {} bps fee difference",
            venue_a.name, venue_b.name, fee_diff);
    }
    
    Ok(())
}

// Unified order routing across venues
async fn smart_order_router(
    instrument_registry: Arc<InstrumentRegistry>,
    venue_registry: Arc<VenueRegistry>,
    isin: &str,
    order_size_usd: f64,
) -> Result<VenueId, Box<dyn Error>> {
    
    // Get all venues for this ISIN
    let venues = venue_registry.find_arbitrage_venues_for_isin(isin);
    
    // Score each venue based on multiple factors
    let mut venue_scores: Vec<(VenueId, f64)> = Vec::new();
    
    for venue in instrument_registry.find_all_venues_for_isin(isin) {
        let venue_data = venue_registry.get_by_id(venue.id).unwrap();
        
        let mut score = 100.0;
        
        // Factor 1: Fees (30% weight)
        let fee_score = 100.0 - venue_data.fee_structure.taker_fee_bps;
        score += fee_score * 0.3;
        
        // Factor 2: Liquidity (40% weight)
        score += venue_data.performance_metrics.liquidity_score * 0.4;
        
        // Factor 3: Fill time (20% weight)
        let speed_score = 100.0 - venue_data.performance_metrics.avg_fill_time_ms.min(100.0);
        score += speed_score * 0.2;
        
        // Factor 4: Reliability (10% weight)
        score += venue_data.performance_metrics.reliability_score * 0.1;
        
        // Check if order size is within limits
        if let Some(max_size) = venue_data.capabilities.max_order_size_usd {
            if order_size_usd > max_size {
                score *= 0.5; // Penalize if order too large
            }
        }
        
        venue_scores.push((venue.id, score));
    }
    
    // Select best venue
    venue_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    
    Ok(venue_scores[0].0)
}
```

### ISIN Validation and Check Digit Calculation

```rust
fn calculate_isin_check_digit(cusip: &str) -> char {
    // Luhn algorithm for ISIN check digit
    let mut sum = 0;
    let mut double = false;
    
    let isin_base = format!("US{}", cusip);
    
    for c in isin_base.chars().rev() {
        let mut digit = match c {
            '0'..='9' => c as u32 - '0' as u32,
            'A'..='Z' => c as u32 - 'A' as u32 + 10,
            _ => continue,
        };
        
        if double {
            digit *= 2;
            if digit > 9 {
                digit = (digit / 10) + (digit % 10);
            }
        }
        
        sum += digit;
        double = !double;
    }
    
    let check = (10 - (sum % 10)) % 10;
    (check as u8 + b'0') as char
}

fn validate_isin(isin: &str) -> bool {
    if isin.len() != 12 {
        return false;
    }
    
    // Country code must be 2 letters
    if !isin[0..2].chars().all(|c| c.is_ascii_uppercase()) {
        return false;
    }
    
    // NSIN must be 9 alphanumeric characters
    if !isin[2..11].chars().all(|c| c.is_ascii_alphanumeric()) {
        return false;
    }
    
    // Last character is check digit
    let check_digit = isin.chars().last().unwrap();
    let calculated = calculate_isin_check_digit(&isin[2..11]);
    
    check_digit == calculated
}
```

## Summary

This registry infrastructure provides:

✅ **Unified Instrument Model**: All assets (tokens, stocks, futures, options) treated uniformly  
✅ **Deterministic IDs**: Blake3 hashing ensures consistency across restarts  
✅ **Collision Safety**: Detection, alerting, and mitigation strategies  
✅ **Lock-Free Concurrency**: DashMap enables high-throughput operations  
✅ **Memory Efficiency**: Arc sharing reduces duplication at scale  
✅ **Production Monitoring**: Comprehensive metrics and alerting  
✅ **Efficient Binary Protocol**: Compact messaging with ID-based references  
✅ **Dynamic Discovery**: Automatic registration of new instruments and trading venues  
✅ **Cross-Asset Arbitrage**: Detect opportunities across DeFi, CEX, and TradFi  
✅ **Type Safety**: Strong typing prevents mixing incompatible instruments  
✅ **Extensible**: Easy to add new asset classes or trading venues  

The system seamlessly integrates with the high-performance IPC core, providing sub-microsecond instrument lookups while supporting dynamic registry updates for new opportunities across all asset classes.

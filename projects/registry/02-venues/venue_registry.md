# Venue Registry

Comprehensive venue management for exchanges, brokers, and DEXs with performance tracking and smart order routing capabilities.

## Venue Definition

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
```

## Connectivity and Performance

```rust
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
pub struct PerformanceMetrics {
    pub avg_fill_time_ms: f64,
    pub daily_volume_usd: f64,
    pub liquidity_score: f64,              // 0-100
    pub slippage_bps: f64,                 // Average slippage in basis points
    pub reliability_score: f64,            // 0-100
}
```

## Fee Structure

```rust
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
```

## Registry Implementation

```rust
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
}
```

## Instrument-Venue Linking

```rust
impl VenueRegistry {
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
}
```

## Smart Order Routing

```rust
impl VenueRegistry {
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
}
```

## Arbitrage Detection

```rust
impl VenueRegistry {
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
```

## Execution Criteria

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionCriteria {
    LowestFees,
    HighestLiquidity,
    FastestExecution,
}
```

## Pool Registry (DeFi Specific)

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
```
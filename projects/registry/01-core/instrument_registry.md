# Core Instrument Registry

The foundation of the multi-asset trading registry system, providing unified instrument management with deterministic IDs and lock-free concurrent access.

## ID Generation Strategy

### Option A: 64-bit IDs with Collision Detection (Default)
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct InstrumentId(pub u64);
```
- Collision probability: ~1 in 2^32 at 65k instruments
- Suitable for systems with <100k instruments
- Requires collision detection and handling

### Option B: 128-bit IDs for Zero Collision Risk
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct InstrumentId(pub u128);
```
- Collision probability: Effectively zero (1 in 2^64 at 4 billion instruments)
- Recommended for systems with >100k instruments
- Slightly larger messages but guaranteed uniqueness

## Base Instrument Definition

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

## Registry Implementation

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
```

## Registration and Lookup Methods

```rust
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

## Cross-Exchange Lookup Methods

```rust
impl InstrumentRegistry {
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
}
```

## Performance Monitoring

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
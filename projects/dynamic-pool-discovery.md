# Dynamic Instrument & Pool Discovery System

## Problem Statement

The current system uses hardcoded mappings for both crypto DEX pools and traditional financial instruments. This approach has several limitations:

1. **Not Scalable**: New instruments (stocks, options, crypto pools, forex pairs) require manual code updates
2. **Maintenance Overhead**: Instrument identifiers can change, new exchanges launch, new assets are listed
3. **Missing Opportunities**: Unknown instruments are silently ignored 
4. **Fragile**: Hardcoded mappings become stale and require constant maintenance
5. **Cross-Asset Inconsistency**: Different discovery mechanisms for crypto vs traditional assets

## Current Implementation Issues

### Crypto DEX Pools (Hardcoded)
```rust
async fn identify_pool(&self, pool_address: &str) -> Option<(String, String, String)> {
    match pool_address.to_lowercase().as_str() {
        "0x6e7a5fafcec6bb1e78bae2a1f0b612012bf14827" => Some(("quickswap".to_string(), "WMATIC".to_string(), "USDC".to_string())),
        // ... 20+ more hardcoded mappings
        _ => {
            debug!("❓ Unknown pool address: {} - skipping", pool_address);
            None
        }
    }
}
```

### Traditional Instruments (Hardcoded)
```rust
fn register_known_instruments(&mut self) {
    // Hardcoded stock symbols
    let stocks = vec!["AAPL", "GOOGL", "MSFT", "TSLA", "NVDA", ...];
    for stock in stocks {
        self.register(SymbolDescriptor::stock("alpaca", stock));
    }
    
    // Hardcoded crypto pairs  
    let coinbase_pairs = vec![("BTC", "USD"), ("ETH", "USD"), ...];
    for (base, quote) in coinbase_pairs {
        self.register(SymbolDescriptor::spot("coinbase", base, quote));
    }
}
```

## Proposed Solution: Universal Dynamic Discovery

### Phase 1: Universal Instrument Discovery
Implement automatic discovery for all asset classes:

```rust
struct UniversalInstrumentRegistry {
    // Crypto DEX pools
    dex_factories: HashMap<String, String>, // DEX name -> factory address
    pool_cache: Arc<RwLock<HashMap<String, PoolInfo>>>,
    
    // Traditional markets
    exchange_apis: HashMap<String, Box<dyn ExchangeAPI>>, // exchange -> API client
    instrument_cache: Arc<RwLock<HashMap<String, InstrumentInfo>>>,
    
    // Parquet storage for persistence
    storage: Arc<InstrumentStorage>,
    token_resolver: TokenResolver,
}

struct InstrumentInfo {
    exchange: String,
    symbol: String,
    asset_type: AssetType, // Stock, Option, Crypto, Forex, Future
    metadata: InstrumentMetadata,
    created_timestamp: SystemTime,
    last_updated: SystemTime,
}

enum AssetType {
    Stock { sector: Option<String>, market_cap: Option<u64> },
    Option { underlying: String, strike: f64, expiry: NaiveDate, option_type: OptionType },
    Crypto { token0_address: String, token1_address: String, pool_address: String, dex: String },
    Forex { base: String, quote: String },
    Future { underlying: String, expiry: NaiveDate, contract_size: f64 },
}

impl UniversalInstrumentRegistry {
    async fn discover_instrument(&self, exchange: &str, identifier: &str) -> Result<Option<InstrumentInfo>> {
        // 1. Check cache first (in-memory)
        let cache_key = format!("{}:{}", exchange, identifier);
        if let Some(info) = self.instrument_cache.read().get(&cache_key) {
            return Ok(Some(info.clone()));
        }
        
        // 2. Check persistent storage (Parquet)
        if let Some(info) = self.storage.load_instrument(exchange, identifier).await? {
            // Update cache
            self.instrument_cache.write().insert(cache_key.clone(), info.clone());
            return Ok(Some(info));
        }
        
        // 3. Dynamic discovery based on exchange type
        let discovered_info = match exchange {
            // Crypto DEXs
            ex if self.dex_factories.contains_key(ex) => {
                self.discover_crypto_pool(ex, identifier).await?
            },
            // Traditional brokers
            "alpaca" | "ibkr" | "schwab" => {
                self.discover_traditional_instrument(exchange, identifier).await?
            },
            // Centralized crypto exchanges
            "coinbase" | "binance" | "kraken" => {
                self.discover_crypto_pair(exchange, identifier).await?
            },
            _ => None
        };
        
        // 4. Store discovered instrument
        if let Some(info) = &discovered_info {
            self.storage.save_instrument(info).await?;
            self.instrument_cache.write().insert(cache_key, info.clone());
        }
        
        Ok(discovered_info)
    }
    
    async fn discover_crypto_pool(&self, dex: &str, pool_address: &str) -> Result<Option<InstrumentInfo>> {
        // Query factory contracts to resolve pool -> tokens
        if let Some(factory_addr) = self.dex_factories.get(dex) {
            if let Some(pool_info) = self.query_factory(factory_addr, pool_address).await? {
                let token0 = self.token_resolver.resolve(&pool_info.token0_addr).await?;
                let token1 = self.token_resolver.resolve(&pool_info.token1_addr).await?;
                
                return Ok(Some(InstrumentInfo {
                    exchange: dex.to_string(),
                    symbol: format!("{}-{}", token0.symbol, token1.symbol),
                    asset_type: AssetType::Crypto {
                        token0_address: pool_info.token0_addr,
                        token1_address: pool_info.token1_addr,
                        pool_address: pool_address.to_string(),
                        dex: dex.to_string(),
                    },
                    metadata: InstrumentMetadata {
                        decimals: (token0.decimals, token1.decimals),
                        created_block: Some(pool_info.created_block),
                        ..Default::default()
                    },
                    created_timestamp: SystemTime::now(),
                    last_updated: SystemTime::now(),
                }));
            }
        }
        Ok(None)
    }
    
    async fn discover_traditional_instrument(&self, exchange: &str, symbol: &str) -> Result<Option<InstrumentInfo>> {
        // Use exchange APIs to discover stocks, options, etc.
        if let Some(api) = self.exchange_apis.get(exchange) {
            if let Some(instrument_data) = api.lookup_instrument(symbol).await? {
                let asset_type = match instrument_data.instrument_type.as_str() {
                    "stock" => AssetType::Stock {
                        sector: instrument_data.sector.clone(),
                        market_cap: instrument_data.market_cap,
                    },
                    "option" => AssetType::Option {
                        underlying: instrument_data.underlying.unwrap(),
                        strike: instrument_data.strike.unwrap(),
                        expiry: instrument_data.expiry.unwrap(),
                        option_type: instrument_data.option_type.unwrap(),
                    },
                    _ => return Ok(None),
                };
                
                return Ok(Some(InstrumentInfo {
                    exchange: exchange.to_string(),
                    symbol: symbol.to_string(),
                    asset_type,
                    metadata: InstrumentMetadata {
                        company_name: instrument_data.company_name,
                        currency: instrument_data.currency,
                        ..Default::default()
                    },
                    created_timestamp: SystemTime::now(),
                    last_updated: SystemTime::now(),
                }));
            }
        }
        Ok(None)
    }
}
```

### Phase 2: Parquet-Based Instrument Storage
Store all discovered instruments in Parquet files for fast columnar access and historical tracking:

```rust
use arrow::array::*;
use arrow::datatypes::*;
use parquet::arrow::{ArrowReader, ArrowWriter, ParquetFileArrowReader};
use chrono::{DateTime, Utc};

struct InstrumentStorage {
    base_path: PathBuf,
    schema: SchemaRef,
}

impl InstrumentStorage {
    fn new(base_path: PathBuf) -> Self {
        let schema = Schema::new(vec![
            Field::new("exchange", DataType::Utf8, false),
            Field::new("symbol", DataType::Utf8, false),
            Field::new("asset_type", DataType::Utf8, false),
            Field::new("identifier", DataType::Utf8, false), // pool address, ticker, etc.
            
            // Crypto-specific
            Field::new("token0_address", DataType::Utf8, true),
            Field::new("token1_address", DataType::Utf8, true),
            Field::new("pool_address", DataType::Utf8, true),
            Field::new("dex", DataType::Utf8, true),
            Field::new("token0_decimals", DataType::UInt8, true),
            Field::new("token1_decimals", DataType::UInt8, true),
            
            // Traditional instruments
            Field::new("company_name", DataType::Utf8, true),
            Field::new("sector", DataType::Utf8, true),
            Field::new("market_cap", DataType::UInt64, true),
            Field::new("currency", DataType::Utf8, true),
            
            // Options-specific
            Field::new("underlying", DataType::Utf8, true),
            Field::new("strike_price", DataType::Float64, true),
            Field::new("expiry_date", DataType::Date32, true),
            Field::new("option_type", DataType::Utf8, true), // "call" or "put"
            
            // Common metadata
            Field::new("created_block", DataType::UInt64, true),
            Field::new("first_seen", DataType::Timestamp(TimeUnit::Millisecond, None), false),
            Field::new("last_updated", DataType::Timestamp(TimeUnit::Millisecond, None), false),
            Field::new("hash", DataType::UInt64, false), // Symbol hash for quick lookup
        ]);
        
        Self {
            base_path,
            schema: Arc::new(schema),
        }
    }
    
    async fn save_instrument(&self, instrument: &InstrumentInfo) -> Result<()> {
        let file_path = self.get_file_path(&instrument.exchange, &instrument.asset_type);
        
        // Convert InstrumentInfo to Arrow record batch
        let batch = self.instrument_to_record_batch(instrument)?;
        
        // Append to existing Parquet file or create new one
        if file_path.exists() {
            self.append_to_parquet(&file_path, batch).await?;
        } else {
            self.create_parquet_file(&file_path, batch).await?;
        }
        
        Ok(())
    }
    
    async fn load_instrument(&self, exchange: &str, identifier: &str) -> Result<Option<InstrumentInfo>> {
        // Try different file paths based on potential asset types
        let asset_types = ["crypto", "stock", "option", "forex", "future"];
        
        for asset_type in &asset_types {
            let file_path = self.base_path
                .join("instruments")
                .join(exchange)
                .join(format!("{}.parquet", asset_type));
                
            if file_path.exists() {
                if let Some(instrument) = self.search_parquet_file(&file_path, identifier).await? {
                    return Ok(Some(instrument));
                }
            }
        }
        
        Ok(None)
    }
    
    async fn load_instruments_by_hash(&self, hash: u64) -> Result<Vec<InstrumentInfo>> {
        let mut results = Vec::new();
        
        // Search across all parquet files for matching hash
        for entry in walkdir::WalkDir::new(&self.base_path) {
            let entry = entry?;
            if entry.path().extension() == Some(std::ffi::OsStr::new("parquet")) {
                if let Some(instruments) = self.search_parquet_by_hash(entry.path(), hash).await? {
                    results.extend(instruments);
                }
            }
        }
        
        Ok(results)
    }
    
    fn get_file_path(&self, exchange: &str, asset_type: &AssetType) -> PathBuf {
        let type_name = match asset_type {
            AssetType::Stock { .. } => "stock",
            AssetType::Option { .. } => "option", 
            AssetType::Crypto { .. } => "crypto",
            AssetType::Forex { .. } => "forex",
            AssetType::Future { .. } => "future",
        };
        
        self.base_path
            .join("instruments")
            .join(exchange)
            .join(format!("{}.parquet", type_name))
    }
}

// Partitioning strategy:
// - instruments/
//   - alpaca/
//     - stock.parquet (all stocks)
//     - option.parquet (all options)
//   - coinbase/
//     - crypto.parquet (all spot pairs)
//   - quickswap/
//     - crypto.parquet (all pools)
//   - etc.
```

### Phase 3: Pool Discovery Analytics
Add metrics and monitoring for pool discovery:

```rust
struct PoolDiscoveryMetrics {
    pools_discovered: Counter,
    cache_hits: Counter,
    cache_misses: Counter,
    rpc_calls: Counter,
    discovery_latency: Histogram,
}
```

### Phase 4: Auto-Update Pool Mappings
Implement background tasks to keep pool mappings fresh:

```rust
impl PoolRegistry {
    async fn background_refresh(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(3600)); // Hourly
        
        loop {
            interval.tick().await;
            
            // Refresh stale entries
            let stale_pools = self.get_stale_pools().await;
            for pool_addr in stale_pools {
                if let Err(e) = self.refresh_pool(&pool_addr).await {
                    warn!("Failed to refresh pool {}: {}", pool_addr, e);
                }
            }
        }
    }
    
    fn get_stale_pools(&self) -> Vec<String> {
        self.pool_cache.read()
            .iter()
            .filter(|(_, info)| info.last_updated.elapsed().unwrap_or_default() > Duration::from_secs(86400))
            .map(|(addr, _)| addr.clone())
            .collect()
    }
}
```

## Implementation Benefits

1. **Automatic Discovery**: No manual intervention for new pools
2. **Real-time Updates**: Pools discovered as they're encountered
3. **Fault Tolerance**: Graceful handling of unknown pools
4. **Performance**: Caching reduces RPC calls
5. **Observability**: Metrics for monitoring discovery process
6. **Data Persistence**: Historical pool information retained

## Migration Strategy

1. **Backward Compatibility**: Keep current hardcoded mappings as fallback
2. **Gradual Rollout**: Enable dynamic discovery alongside static mappings
3. **Validation**: Compare dynamic results with known good mappings
4. **Performance Testing**: Ensure RPC call overhead is acceptable
5. **Full Migration**: Remove hardcoded mappings once system is proven

## Files to Modify

- `backend/services/exchange_collector/src/exchanges/polygon.rs`
- `backend/services/exchange_collector/src/pool_registry.rs` (new)
- `backend/services/exchange_collector/src/token_resolver.rs` (new)
- `backend/services/exchange_collector/Cargo.toml` (add sqlx dependency)
- `backend/schema/pools.sql` (new)

## Estimated Effort

- **Phase 1**: 2-3 days (core dynamic discovery)
- **Phase 2**: 1-2 days (database persistence)
- **Phase 3**: 1 day (metrics and monitoring)
- **Phase 4**: 1 day (background refresh)
- **Testing & Migration**: 2 days

**Total**: ~1-1.5 weeks

## Priority

**Medium-High** - While not critical for current operations, this will significantly improve system reliability and reduce maintenance overhead as the platform scales.
# Synthetic Instruments

Dynamic, formula-based instruments for complex cross-asset trading strategies with real-time evaluation and validation.

## Core Definition

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
```

## Evaluation Engine

```rust
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
```

## Common Synthetic Instruments

### ETH Basis Spread
```rust
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
}
```

### DeFi-CEX Arbitrage Index
```rust
impl SyntheticInstrument {
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

### Cross-Asset Correlation Index
```rust
impl SyntheticInstrument {
    pub fn btc_spy_correlation() -> Self {
        Self {
            id: InstrumentId(u64::MAX - 3),
            name: "BTC-SPY Correlation Index".to_string(),
            components: vec![
                // Bitcoin on Coinbase
                InstrumentId(0x4001),
                // SPY ETF on NYSE
                InstrumentId(0x5001),
            ],
            formula: "(BTC_RETURN - BTC_MA) * (SPY_RETURN - SPY_MA) / (BTC_STD * SPY_STD)".to_string(),
            update_frequency: Duration::from_secs(60),
            dependencies: DashMap::new(),
            validation: SyntheticValidation {
                min_value: Some(-1.0),      // Perfect negative correlation
                max_value: Some(1.0),        // Perfect positive correlation
                outlier_threshold: 5.0,
                stale_data_threshold: Duration::from_secs(120),
            },
        }
    }
}
```

## Synthetic Registry

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
```

## Registration and Management

```rust
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
}
```

## Real-Time Evaluation

```rust
impl SyntheticRegistry {
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

## Type-Safe Synthetic References

```rust
use std::marker::PhantomData;

// Type-safe synthetic types
pub struct BasisSpread;
pub struct ArbitrageIndex;
pub struct CorrelationIndex;

#[derive(Debug, Clone)]
pub struct TypedSyntheticId<T> {
    id: InstrumentId,
    _phantom: PhantomData<T>,
}

impl<T> TypedSyntheticId<T> {
    pub fn new(id: InstrumentId, registry: &SyntheticRegistry) -> Result<Self, RegistryError> {
        // Validate that the ID corresponds to the correct synthetic type
        if !Self::validate_type(id, registry) {
            return Err(RegistryError::TypeMismatch {
                expected: std::any::type_name::<T>(),
                actual: "unknown",
            });
        }
        
        Ok(Self { 
            id, 
            _phantom: PhantomData 
        })
    }
}

// Type-safe synthetic calculations
impl TypedSyntheticId<BasisSpread> {
    pub fn annualized_rate(&self, registry: &SyntheticRegistry, days_to_expiry: f64) -> Result<f64, SyntheticError> {
        let value = registry.get_latest_value(self.id)?;
        Ok(value.value * 365.0 / days_to_expiry)
    }
}

impl TypedSyntheticId<ArbitrageIndex> {
    pub fn is_profitable(&self, registry: &SyntheticRegistry, min_spread_bps: f64) -> Result<bool, SyntheticError> {
        let value = registry.get_latest_value(self.id)?;
        Ok(value.value > min_spread_bps)
    }
}
```

## Advanced Formulas

### Options Greeks Synthetics
```rust
impl SyntheticInstrument {
    pub fn option_delta_synthetic(option_id: InstrumentId, underlying_id: InstrumentId) -> Self {
        Self {
            id: InstrumentId(u64::MAX - 10),
            name: "Option Delta".to_string(),
            components: vec![option_id, underlying_id],
            formula: "(OPTION_PRICE_UP - OPTION_PRICE_DOWN) / (2 * UNDERLYING_MOVE)".to_string(),
            update_frequency: Duration::from_secs(5),
            dependencies: DashMap::new(),
            validation: SyntheticValidation {
                min_value: Some(-1.0),      // Put delta
                max_value: Some(1.0),        // Call delta
                outlier_threshold: 3.0,
                stale_data_threshold: Duration::from_secs(10),
            },
        }
    }
}
```

### Volatility Arbitrage
```rust
impl SyntheticInstrument {
    pub fn implied_vs_realized_vol() -> Self {
        Self {
            id: InstrumentId(u64::MAX - 11),
            name: "IV vs RV Spread".to_string(),
            components: vec![
                // VIX Index
                InstrumentId(0x6001),
                // SPY 30-day realized vol
                InstrumentId(0x6002),
            ],
            formula: "VIX - SPY_REALIZED_VOL".to_string(),
            update_frequency: Duration::from_secs(60),
            dependencies: DashMap::new(),
            validation: SyntheticValidation {
                min_value: Some(-50.0),
                max_value: Some(50.0),
                outlier_threshold: 4.0,
                stale_data_threshold: Duration::from_secs(120),
            },
        }
    }
}
```
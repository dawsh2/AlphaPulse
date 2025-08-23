# Universal Strategy Support

How the registry and massive data collection architecture enables ANY trading strategy, not just arbitrage.

## Strategy Data Engine

The registry-aware data engine enriches raw market data for all strategy types simultaneously.

```rust
pub struct StrategyDataEngine {
    // Raw data from relay
    relay_subscriber: Arc<RelaySubscriber>,
    
    // Registry integration
    instrument_registry: Arc<InstrumentRegistry>,
    venue_registry: Arc<VenueRegistry>,
    synthetic_registry: Arc<SyntheticRegistry>,
    
    // Strategy-specific engines
    momentum_engine: Arc<MomentumDataEngine>,
    mean_reversion_engine: Arc<MeanReversionEngine>,
    market_making_engine: Arc<MarketMakingEngine>,
    macro_engine: Arc<MacroStrategyEngine>,
    ml_feature_engine: Arc<MLFeatureEngine>,
    
    // Cross-asset correlation tracking
    correlation_tracker: Arc<CrossAssetCorrelationTracker>,
    
    // Multi-timeframe analytics
    timeframe_aggregator: Arc<MultiTimeframeAggregator>,
}

impl StrategyDataEngine {
    pub async fn enrich_for_all_strategies(&self, raw_event: &ProcessedEvent) -> StrategyEnrichedEvent {
        // Get instrument details from registry
        let instrument = self.instrument_registry.get_by_id(raw_event.instrument_id).await?;
        let venues = self.venue_registry.find_venues_for_instrument(raw_event.instrument_id).await;
        
        // Base market data
        let market_data = self.extract_market_data(raw_event);
        
        // Enrich with all strategy signals
        StrategyEnrichedEvent {
            base_data: market_data,
            instrument_details: instrument,
            available_venues: venues,
            momentum: self.momentum_engine.calculate_signals(&market_data).await,
            mean_reversion: self.mean_reversion_engine.calculate_signals(&market_data).await,
            market_making: self.market_making_engine.analyze_order_book(&market_data).await,
            macro_context: self.macro_engine.get_macro_context(&market_data).await,
            ml_features: self.ml_feature_engine.extract_features(&market_data).await,
            correlations: self.correlation_tracker.get_correlations(&market_data.instrument_id).await,
            timestamp: raw_event.timestamp,
        }
    }
}
```

## Momentum/Trend Following Strategies

### Cross-Asset Momentum with Registry
```rust
pub struct MomentumDataEngine {
    registry: Arc<InstrumentRegistry>,
    venue_registry: Arc<VenueRegistry>,
    
    // Multi-timeframe momentum tracking
    timeframes: Vec<Duration>,  // 1m, 5m, 15m, 1h, 4h, 1d
    
    // Cross-asset momentum
    sector_momentum: SectorMomentumTracker,
    asset_class_momentum: AssetClassMomentumTracker,
}

impl MomentumDataEngine {
    pub async fn calculate_signals(&self, market_data: &MarketData) -> MomentumSignals {
        // Get instrument context from registry
        let instrument = self.registry.get_by_id(market_data.instrument_id).await?;
        
        // Cross-asset momentum confirmation
        let sector_confirmation = self.get_sector_momentum_confirmation(&instrument).await;
        let cross_venue_momentum = self.calculate_cross_venue_momentum(&instrument).await;
        
        MomentumSignals {
            strength: self.calculate_composite_momentum_strength(market_data).await,
            direction: self.determine_momentum_direction(market_data).await,
            
            // Registry-enhanced signals
            sector_confirmation,
            cross_venue_confirmation: cross_venue_momentum,
            related_asset_momentum: self.get_related_asset_momentum(&instrument).await,
            
            // Cross-asset validation
            btc_correlation: self.calculate_btc_correlation(&instrument).await,
            spy_correlation: self.calculate_spy_correlation(&instrument).await,
        }
    }
    
    async fn get_sector_momentum_confirmation(&self, instrument: &Instrument) -> SectorMomentum {
        match &instrument.instrument_type {
            InstrumentType::Stock { sector: Some(sector), isin, .. } => {
                // Find all stocks in same sector using registry
                let sector_instruments = self.registry.find_by_sector(sector).await;
                self.calculate_sector_breadth(sector_instruments).await
            }
            InstrumentType::Token { blockchain, .. } => {
                // Find all tokens on same blockchain
                let chain_tokens = self.registry.find_by_blockchain(blockchain).await;
                self.calculate_defi_sector_momentum(chain_tokens).await
            }
            _ => SectorMomentum::default(),
        }
    }
    
    async fn calculate_cross_venue_momentum(&self, instrument: &Instrument) -> CrossVenueMomentum {
        // Use ISIN to find same asset on different exchanges
        if let InstrumentType::Stock { isin, .. } = &instrument.instrument_type {
            let venues = self.registry.find_all_venues_for_isin(isin).await;
            
            // Check if momentum is consistent across venues
            let mut venue_momentums = Vec::new();
            for (exchange, inst) in venues {
                let momentum = self.calculate_single_venue_momentum(&inst).await;
                venue_momentums.push((exchange, momentum));
            }
            
            CrossVenueMomentum {
                consistency_score: self.calculate_consistency(&venue_momentums),
                leading_venue: self.identify_leading_venue(&venue_momentums),
                lagging_venues: self.identify_lagging_venues(&venue_momentums),
            }
        } else {
            CrossVenueMomentum::default()
        }
    }
}
```

## Mean Reversion Strategies

### Registry-Enhanced Mean Reversion
```rust
pub struct MeanReversionEngine {
    registry: Arc<InstrumentRegistry>,
    synthetic_registry: Arc<SyntheticRegistry>,
    
    // Statistical measures
    z_score_calculator: ZScoreCalculator,
    cointegration_tracker: CointegrationTracker,
    
    // Cross-asset mean reversion
    pairs_tracker: PairsTracker,
    basket_tracker: BasketMeanReversionTracker,
}

impl MeanReversionEngine {
    pub async fn calculate_signals(&self, market_data: &MarketData) -> MeanReversionSignals {
        let instrument = self.registry.get_by_id(market_data.instrument_id).await?;
        
        // Find cointegrated pairs using registry
        let cointegrated_pairs = self.find_cointegrated_instruments(&instrument).await;
        
        // Check synthetic instruments for mean reversion
        let synthetic_opportunities = self.check_synthetic_mean_reversion(&instrument).await;
        
        MeanReversionSignals {
            z_score: self.calculate_z_score(market_data).await,
            
            // Registry-enhanced opportunities
            pairs_opportunities: cointegrated_pairs,
            synthetic_reversion: synthetic_opportunities,
            cross_venue_reversion: self.find_cross_venue_reversion(&instrument).await,
            
            // Cross-asset mean reversion
            etf_nav_reversion: self.check_etf_nav_reversion(&instrument).await,
            futures_basis_reversion: self.check_futures_basis_reversion(&instrument).await,
        }
    }
    
    async fn find_cointegrated_instruments(&self, instrument: &Instrument) -> Vec<PairsOpportunity> {
        let mut opportunities = Vec::new();
        
        // Use registry to find related instruments
        let related = match &instrument.instrument_type {
            InstrumentType::Stock { sector, .. } => {
                // Find stocks in same sector
                self.registry.find_by_sector(sector).await
            }
            InstrumentType::Token { .. } if instrument.symbol == "ETH" => {
                // Find ETH-correlated tokens
                vec![
                    self.registry.get_by_symbol("UNI").await,
                    self.registry.get_by_symbol("AAVE").await,
                    self.registry.get_by_symbol("SNX").await,
                ].into_iter().flatten().collect()
            }
            _ => vec![],
        };
        
        // Test cointegration with each related instrument
        for related_inst in related {
            if let Some(coint_score) = self.test_cointegration(instrument.id, related_inst.id).await {
                if coint_score > COINTEGRATION_THRESHOLD {
                    opportunities.push(PairsOpportunity {
                        instrument_a: instrument.id,
                        instrument_b: related_inst.id,
                        cointegration_score: coint_score,
                        current_spread: self.calculate_spread(instrument.id, related_inst.id).await,
                        mean_spread: self.get_historical_mean_spread(instrument.id, related_inst.id).await,
                    });
                }
            }
        }
        
        opportunities
    }
}
```

## Market Making Strategies

### Cross-Venue Market Making with Registry
```rust
pub struct MarketMakingEngine {
    registry: Arc<InstrumentRegistry>,
    venue_registry: Arc<VenueRegistry>,
    
    // Order book analytics
    order_book_analyzer: OrderBookAnalyzer,
    
    // Cross-venue market making
    venue_analyzer: CrossVenueAnalyzer,
    inventory_manager: MultiVenueInventoryManager,
}

impl MarketMakingEngine {
    pub async fn analyze_order_book(&self, market_data: &MarketData) -> MarketMakingSignals {
        let instrument = self.registry.get_by_id(market_data.instrument_id).await?;
        let venues = self.venue_registry.find_venues_for_instrument(market_data.instrument_id).await;
        
        // Cross-venue order book analysis
        let cross_venue_imbalances = self.analyze_cross_venue_order_books(&venues).await;
        
        // Find best venues for market making
        let optimal_venues = self.select_optimal_mm_venues(&venues, &instrument).await;
        
        MarketMakingSignals {
            order_flow_imbalance: self.calculate_imbalance(market_data).await,
            
            // Registry-enhanced signals
            cross_venue_opportunities: cross_venue_imbalances,
            optimal_venues,
            inventory_hedging_venues: self.find_hedging_venues(&instrument).await,
            
            // Cross-asset hedging opportunities
            correlated_instruments: self.find_correlated_hedges(&instrument).await,
            synthetic_hedges: self.find_synthetic_hedges(&instrument).await,
        }
    }
    
    async fn analyze_cross_venue_order_books(&self, venues: &[Arc<Venue>]) -> Vec<CrossVenueMMOpportunity> {
        let mut opportunities = Vec::new();
        
        for venue_a in venues {
            for venue_b in venues {
                if venue_a.id == venue_b.id { continue; }
                
                // Compare order books between venues
                let ob_a = self.get_order_book(venue_a.id).await;
                let ob_b = self.get_order_book(venue_b.id).await;
                
                // Check for imbalances that create MM opportunities
                if let Some(opportunity) = self.detect_mm_opportunity(&ob_a, &ob_b, venue_a, venue_b) {
                    opportunities.push(opportunity);
                }
            }
        }
        
        opportunities
    }
}
```

## Macro/Fundamental Strategies

### Registry-Aware Macro Analysis
```rust
pub struct MacroStrategyEngine {
    registry: Arc<InstrumentRegistry>,
    
    // Cross-asset macro relationships
    yield_curve_analyzer: YieldCurveAnalyzer,
    currency_strength_tracker: CurrencyStrengthTracker,
    
    // DeFi macro factors
    defi_tvl_tracker: DeFiTVLTracker,
    stablecoin_flows_tracker: StablecoinFlowsTracker,
}

impl MacroStrategyEngine {
    pub async fn get_macro_context(&self, market_data: &MarketData) -> MacroContext {
        let instrument = self.registry.get_by_id(market_data.instrument_id).await?;
        
        // Build macro context using registry relationships
        MacroContext {
            // Traditional macro
            fed_policy_impact: self.assess_fed_impact(&instrument).await,
            yield_curve_signal: self.analyze_yield_curve_impact(&instrument).await,
            
            // Cross-asset macro using registry
            correlated_fx_impact: self.analyze_fx_correlations(&instrument).await,
            commodity_correlations: self.analyze_commodity_impact(&instrument).await,
            
            // DeFi-TradFi interactions
            defi_tradfi_divergence: self.measure_defi_tradfi_divergence(&instrument).await,
            stablecoin_flow_impact: self.analyze_stablecoin_impact(&instrument).await,
        }
    }
    
    async fn analyze_fx_correlations(&self, instrument: &Instrument) -> FXImpact {
        // Use registry to find FX pairs that affect this instrument
        match &instrument.instrument_type {
            InstrumentType::Stock { isin, .. } => {
                // Get country from ISIN
                let country = isin_to_country(&isin[0..2]);
                
                // Find relevant FX pairs in registry
                let fx_pairs = self.registry.find_fx_pairs_for_country(country).await;
                
                // Analyze correlation and impact
                self.calculate_fx_impact(instrument, fx_pairs).await
            }
            InstrumentType::Token { .. } => {
                // Crypto correlates with DXY, EURUSD, etc.
                let major_fx = vec!["DXY", "EURUSD", "USDJPY"];
                self.calculate_crypto_fx_correlation(instrument, major_fx).await
            }
            _ => FXImpact::default(),
        }
    }
}
```

## Machine Learning Features

### Registry-Enhanced Feature Engineering
```rust
pub struct MLFeatureEngine {
    registry: Arc<InstrumentRegistry>,
    venue_registry: Arc<VenueRegistry>,
    synthetic_registry: Arc<SyntheticRegistry>,
    
    // Feature extractors
    technical_features: TechnicalFeatureExtractor,
    microstructure_features: OrderBookFeatureExtractor,
    cross_asset_features: CrossAssetFeatureExtractor,
}

impl MLFeatureEngine {
    pub async fn extract_features(&self, market_data: &MarketData) -> MLFeatures {
        let instrument = self.registry.get_by_id(market_data.instrument_id).await?;
        let venues = self.venue_registry.find_venues_for_instrument(market_data.instrument_id).await;
        
        MLFeatures {
            // Standard features
            technical: self.technical_features.extract(market_data).await,
            microstructure: self.microstructure_features.extract(market_data).await,
            
            // Registry-enhanced features
            instrument_features: self.extract_instrument_features(&instrument),
            venue_features: self.extract_venue_features(&venues),
            cross_venue_features: self.extract_cross_venue_features(&venues, market_data).await,
            
            // Cross-asset features using registry
            correlation_features: self.extract_correlation_features(&instrument).await,
            synthetic_features: self.extract_synthetic_features(&instrument).await,
            sector_features: self.extract_sector_features(&instrument).await,
            
            // ISIN-based features for stocks
            isin_features: self.extract_isin_features(&instrument).await,
        }
    }
    
    async fn extract_cross_venue_features(&self, venues: &[Arc<Venue>], market_data: &MarketData) -> Vec<f64> {
        let mut features = Vec::new();
        
        // Venue diversity score
        features.push(venues.len() as f64);
        
        // Fee variance across venues
        let fees: Vec<f64> = venues.iter()
            .map(|v| v.fee_structure.taker_fee_bps)
            .collect();
        features.push(calculate_variance(&fees));
        
        // Liquidity distribution
        let liquidities: Vec<f64> = venues.iter()
            .map(|v| v.performance_metrics.liquidity_score)
            .collect();
        features.push(calculate_gini_coefficient(&liquidities));
        
        // Price discrepancy across venues
        let prices = self.get_prices_across_venues(market_data.instrument_id).await;
        features.push(calculate_price_dispersion(&prices));
        
        features
    }
}
```

## Strategy Router

### Universal Strategy Data Distribution
```rust
pub struct StrategyDataRouter {
    strategy_subscriptions: HashMap<StrategyType, Vec<StrategySubscriber>>,
    data_enricher: Arc<StrategyDataEngine>,
    registry: Arc<InstrumentRegistry>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum StrategyType {
    Momentum,
    MeanReversion,
    MarketMaking,
    Arbitrage,
    Macro,
    MachineLearning,
    EventDriven,
    StatisticalArbitrage,
    OptionsMarketMaking,
    CryptoMacro,
    CrossAssetMomentum,
    DeFiYieldFarming,
}

impl StrategyDataRouter {
    pub async fn route_to_strategies(&self, raw_event: &ProcessedEvent) -> Result<(), RoutingError> {
        // Enrich data for all strategy types using registry
        let enriched_event = self.data_enricher.enrich_for_all_strategies(raw_event).await;
        
        // Route to each strategy type with appropriate data
        for (strategy_type, subscribers) in &self.strategy_subscriptions {
            let strategy_data = self.prepare_strategy_data(&strategy_type, &enriched_event).await;
            
            for subscriber in subscribers {
                subscriber.send_data(strategy_data.clone()).await?;
            }
        }
        
        Ok(())
    }
    
    async fn prepare_strategy_data(&self, strategy_type: &StrategyType, enriched: &StrategyEnrichedEvent) -> StrategyData {
        match strategy_type {
            StrategyType::Momentum => {
                StrategyData::Momentum {
                    base: enriched.base_data.clone(),
                    signals: enriched.momentum.clone(),
                    cross_asset_confirmation: enriched.correlations.clone(),
                }
            }
            StrategyType::CrossAssetMomentum => {
                // Special handling for cross-asset strategies
                let related_instruments = self.registry
                    .find_correlated_instruments(enriched.base_data.instrument_id)
                    .await;
                    
                StrategyData::CrossAssetMomentum {
                    primary: enriched.base_data.clone(),
                    momentum: enriched.momentum.clone(),
                    related_instruments,
                    sector_breadth: enriched.macro_context.sector_rotation_signals.clone(),
                }
            }
            _ => StrategyData::Generic(enriched.clone()),
        }
    }
}
```

## Performance Benefits

### Why This Architecture Enables ANY Strategy

1. **Unified Asset Universe**
   - Every tradeable asset in one registry
   - Cross-reference by ISIN, symbol, address
   - Relationships tracked automatically

2. **Multi-Venue Visibility**
   - Same asset tracked across all venues
   - Cross-venue opportunities visible to all strategies
   - Best execution routing built-in

3. **Rich Feature Generation**
   - 400+ features generated automatically
   - Cross-asset correlations computed in real-time
   - Synthetic instruments for complex relationships

4. **Strategy-Agnostic Design**
   - Raw data enriched for all strategies simultaneously
   - Each strategy subscribes to relevant patterns
   - No strategy-specific code in core pipeline

5. **Horizontal Scalability**
   - Add new data sources without touching strategies
   - Add new strategies without touching collectors
   - Registry grows dynamically with discoveries

## Example Strategy Implementations

### Multi-Asset Momentum
```rust
// Tracks momentum across crypto, stocks, and commodities simultaneously
let momentum_strategy = MultiAssetMomentumStrategy::new(
    registry.clone(),
    vec![
        StrategySubscription::AllStocks,
        StrategySubscription::AllCrypto,
        StrategySubscription::Commodities,
    ]
);
```

### Cross-Exchange Market Making
```rust
// Makes markets on Binance while hedging on CME futures
let mm_strategy = CrossExchangeMarketMaker::new(
    registry.clone(),
    venue_registry.clone(),
    vec![
        (VenueId::Binance, InstrumentType::Spot),
        (VenueId::CME, InstrumentType::Future),
    ]
);
```

### DeFi-TradFi Arbitrage
```rust
// Arbitrages between DeFi yields and TradFi rates
let arb_strategy = DeFiTradFiArbitrage::new(
    registry.clone(),
    synthetic_registry.clone(),
    vec![
        SyntheticInstrument::defi_lending_rate(),
        SyntheticInstrument::treasury_yield_curve(),
    ]
);
```

The registry isn't just infrastructure - it's the foundation that makes sophisticated multi-asset, cross-venue strategies possible!
# Cross-Exchange Asset Tracking

Real-world examples and implementation patterns for tracking assets across multiple exchanges using ISIN, CUSIP, and smart order routing.

## ISIN-Based Asset Tracking

### Apple Inc. Across Global Markets
```rust
use registry::{InstrumentRegistry, VenueRegistry, ExecutionCriteria};

async fn track_apple_cross_exchange(
    instrument_registry: Arc<InstrumentRegistry>,
    venue_registry: Arc<VenueRegistry>,
) -> Result<CrossExchangeReport, TrackingError> {
    
    // Apple's ISIN is US0378331005
    let apple_isin = "US0378331005";
    
    // Find all venues where Apple trades
    let apple_venues = instrument_registry.find_all_venues_for_isin(apple_isin);
    
    println!("Apple Inc. (ISIN: {}) trades on {} venues:", apple_isin, apple_venues.len());
    
    let mut venue_details = Vec::new();
    
    for (exchange, instrument) in &apple_venues {
        let ticker = match &instrument.instrument_type {
            InstrumentType::Stock { ticker, .. } => ticker.clone(),
            _ => "unknown".to_string(),
        };
        
        // Get venue details
        if let Some(venue) = venue_registry.get_by_name(exchange) {
            let metrics = venue.performance_metrics.clone();
            
            venue_details.push(VenueDetail {
                exchange: exchange.clone(),
                ticker,
                liquidity_score: metrics.liquidity_score,
                avg_spread_bps: metrics.slippage_bps,
                daily_volume_usd: metrics.daily_volume_usd,
                trading_hours: venue.trading_hours.clone(),
            });
            
            println!("  - {} as ticker {} (Liquidity: {:.1}, Volume: ${:.0}M)",
                exchange, ticker, 
                metrics.liquidity_score,
                metrics.daily_volume_usd / 1_000_000.0
            );
        }
    }
    
    // Example output:
    // Apple Inc. (ISIN: US0378331005) trades on 8 venues:
    //   - NASDAQ as ticker AAPL (Liquidity: 98.5, Volume: $65,432M)
    //   - NYSE as ticker AAPL (Liquidity: 95.2, Volume: $12,345M)
    //   - XETRA as ticker APC (Liquidity: 72.3, Volume: $543M)
    //   - LSE as ticker 0R2V (Liquidity: 68.9, Volume: $234M)
    //   - TSE as ticker AAPL (Liquidity: 45.6, Volume: $123M)
    //   - SIX as ticker AAPL (Liquidity: 41.2, Volume: $98M)
    //   - ASX as ticker AAPL (Liquidity: 38.7, Volume: $87M)
    //   - HKEX as ticker 0865 (Liquidity: 52.1, Volume: $456M)
    
    Ok(CrossExchangeReport {
        isin: apple_isin.to_string(),
        venues: venue_details,
        primary_exchange: "NASDAQ".to_string(),
        total_daily_volume_usd: venue_details.iter()
            .map(|v| v.daily_volume_usd)
            .sum(),
    })
}
```

## Smart Order Router Implementation

```rust
pub struct SmartOrderRouter {
    instrument_registry: Arc<InstrumentRegistry>,
    venue_registry: Arc<VenueRegistry>,
    execution_engine: Arc<ExecutionEngine>,
    risk_manager: Arc<RiskManager>,
}

impl SmartOrderRouter {
    pub async fn route_order(&self, order: Order) -> Result<ExecutionReport, RoutingError> {
        // Step 1: Identify instrument by ISIN
        let instrument = match &order.identifier {
            InstrumentIdentifier::ISIN(isin) => {
                self.instrument_registry.get_by_isin(isin)
                    .ok_or(RoutingError::UnknownInstrument)?
            }
            InstrumentIdentifier::CUSIP(cusip) => {
                self.instrument_registry.get_by_cusip(cusip)
                    .ok_or(RoutingError::UnknownInstrument)?
            }
            InstrumentIdentifier::Symbol(symbol) => {
                self.instrument_registry.get_by_symbol(symbol)
                    .ok_or(RoutingError::UnknownInstrument)?
            }
        };
        
        // Step 2: Find all available venues
        let venues = self.venue_registry.find_venues_for_instrument(instrument.id);
        
        // Step 3: Score venues based on order requirements
        let scored_venues = self.score_venues_for_order(&venues, &order).await?;
        
        // Step 4: Split order if beneficial
        let execution_plan = if order.size_usd > LARGE_ORDER_THRESHOLD {
            self.create_split_execution_plan(scored_venues, &order)?
        } else {
            self.create_single_venue_plan(scored_venues, &order)?
        };
        
        // Step 5: Execute with monitoring
        self.execute_with_monitoring(execution_plan).await
    }
    
    async fn score_venues_for_order(
        &self,
        venues: &[Arc<Venue>],
        order: &Order,
    ) -> Result<Vec<ScoredVenue>, RoutingError> {
        let mut scored = Vec::new();
        
        for venue in venues {
            let mut score = 0.0;
            
            // Factor 1: Fees (30% weight)
            let fee = match order.order_type {
                OrderType::Market => venue.fee_structure.taker_fee_bps,
                OrderType::Limit => venue.fee_structure.maker_fee_bps,
                _ => (venue.fee_structure.taker_fee_bps + venue.fee_structure.maker_fee_bps) / 2.0,
            };
            score += (100.0 - fee) * 0.3;
            
            // Factor 2: Liquidity (40% weight)
            score += venue.performance_metrics.liquidity_score * 0.4;
            
            // Factor 3: Fill probability (20% weight)
            let fill_probability = self.calculate_fill_probability(venue, order).await?;
            score += fill_probability * 100.0 * 0.2;
            
            // Factor 4: Latency (10% weight)
            let latency_score = 100.0 - venue.connectivity.avg_latency_ms.min(100.0);
            score += latency_score * 0.1;
            
            // Apply constraints
            if !self.venue_meets_constraints(venue, order) {
                score *= 0.1; // Heavily penalize but don't exclude
            }
            
            scored.push(ScoredVenue {
                venue: venue.clone(),
                score,
                estimated_impact_bps: self.estimate_market_impact(venue, order),
                expected_fill_time_ms: venue.performance_metrics.avg_fill_time_ms,
            });
        }
        
        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        Ok(scored)
    }
}
```

## Cross-Asset Arbitrage Detection

```rust
pub struct CrossAssetArbitrageScanner {
    instrument_registry: Arc<InstrumentRegistry>,
    venue_registry: Arc<VenueRegistry>,
    price_feed: Arc<PriceFeed>,
    opportunity_broadcaster: broadcast::Sender<ArbitrageOpportunity>,
}

impl CrossAssetArbitrageScanner {
    pub async fn scan_etf_nav_arbitrage(&self) -> Vec<ArbitrageOpportunity> {
        let mut opportunities = Vec::new();
        
        // Find all ETFs
        let etfs = self.instrument_registry.find_by_type(&InstrumentType::ETF { 
            ticker: String::new(),
            exchange: String::new(),
            isin: String::new(),
            cusip: None,
            underlying_index: None,
            expense_ratio: None,
        });
        
        for etf in etfs {
            if let InstrumentType::ETF { isin, underlying_index, .. } = &etf.instrument_type {
                // Get ETF price across all venues
                let etf_prices = self.get_prices_across_venues(&etf.id).await;
                
                // Calculate NAV from underlying components
                if let Some(nav) = self.calculate_etf_nav(underlying_index).await {
                    for (venue_id, etf_price) in &etf_prices {
                        let discount_premium = ((etf_price - nav) / nav * 10000.0).abs();
                        
                        if discount_premium > MIN_ETF_ARB_BPS {
                            opportunities.push(ArbitrageOpportunity {
                                opportunity_type: ArbitrageType::ETFNav,
                                instruments: vec![etf.id],
                                venues: vec![*venue_id],
                                spread_bps: discount_premium as u32,
                                nav_price: nav,
                                market_price: *etf_price,
                                confidence: self.calculate_confidence(&etf_prices),
                                expires_at: SystemTime::now() + Duration::from_secs(30),
                            });
                        }
                    }
                }
            }
        }
        
        opportunities
    }
    
    pub async fn scan_cross_listing_arbitrage(&self) -> Vec<ArbitrageOpportunity> {
        let mut opportunities = Vec::new();
        
        // Get all ISINs that trade on multiple venues
        let multi_venue_isins = self.instrument_registry.get_multi_venue_isins();
        
        for isin in multi_venue_isins {
            let venues = self.instrument_registry.find_all_venues_for_isin(&isin);
            
            if venues.len() < 2 {
                continue;
            }
            
            // Get current prices from all venues
            let mut prices = Vec::new();
            for (exchange, instrument) in &venues {
                if let Some(price) = self.price_feed.get_latest(instrument.id).await {
                    prices.push((exchange.clone(), instrument.id, price.price));
                }
            }
            
            // Find price discrepancies
            if let Some((min_venue, min_price)) = prices.iter()
                .min_by(|a, b| a.2.partial_cmp(&b.2).unwrap()) {
                
                if let Some((max_venue, max_price)) = prices.iter()
                    .max_by(|a, b| a.2.partial_cmp(&b.2).unwrap()) {
                    
                    let spread_bps = ((max_price.2 - min_price.2) / min_price.2 * 10000.0) as u32;
                    
                    if spread_bps > MIN_CROSS_LISTING_ARB_BPS {
                        opportunities.push(ArbitrageOpportunity {
                            opportunity_type: ArbitrageType::CrossListing,
                            instruments: vec![min_price.1, max_price.1],
                            venues: vec![
                                self.venue_registry.get_by_name(&min_venue.0).unwrap().id,
                                self.venue_registry.get_by_name(&max_venue.0).unwrap().id,
                            ],
                            spread_bps,
                            buy_venue: min_venue.0.clone(),
                            sell_venue: max_venue.0.clone(),
                            confidence: 0.95,
                            expires_at: SystemTime::now() + Duration::from_secs(5),
                        });
                    }
                }
            }
        }
        
        opportunities
    }
}
```

## ISIN Validation and Utilities

```rust
pub fn calculate_isin_check_digit(nsin: &str) -> char {
    // Luhn algorithm for ISIN check digit
    let mut sum = 0;
    let mut double = false;
    
    let isin_base = format!("US{}", nsin); // Assuming US for this example
    
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

pub fn validate_isin(isin: &str) -> Result<(), ValidationError> {
    if isin.len() != 12 {
        return Err(ValidationError::InvalidLength {
            expected: 12,
            actual: isin.len(),
        });
    }
    
    // Country code must be 2 letters
    if !isin[0..2].chars().all(|c| c.is_ascii_uppercase()) {
        return Err(ValidationError::InvalidCountryCode);
    }
    
    // NSIN must be 9 alphanumeric characters
    if !isin[2..11].chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(ValidationError::InvalidNSIN);
    }
    
    // Validate check digit
    let check_digit = isin.chars().last().unwrap();
    let calculated = calculate_isin_check_digit(&isin[2..11]);
    
    if check_digit != calculated {
        return Err(ValidationError::InvalidCheckDigit {
            expected: calculated,
            actual: check_digit,
        });
    }
    
    Ok(())
}

pub fn cusip_to_isin(cusip: &str, country_code: &str) -> String {
    let check_digit = calculate_isin_check_digit(cusip);
    format!("{}{}{}", country_code, cusip, check_digit)
}

pub fn isin_to_country(isin: &str) -> Option<String> {
    if isin.len() < 2 {
        return None;
    }
    
    match &isin[0..2] {
        "US" => Some("United States".to_string()),
        "GB" => Some("United Kingdom".to_string()),
        "DE" => Some("Germany".to_string()),
        "JP" => Some("Japan".to_string()),
        "CN" => Some("China".to_string()),
        "HK" => Some("Hong Kong".to_string()),
        "SG" => Some("Singapore".to_string()),
        "AU" => Some("Australia".to_string()),
        "CA" => Some("Canada".to_string()),
        "FR" => Some("France".to_string()),
        "CH" => Some("Switzerland".to_string()),
        "NL" => Some("Netherlands".to_string()),
        _ => None,
    }
}
```

## Real-Time Cross-Exchange Monitoring

```rust
pub struct CrossExchangeMonitor {
    registries: RegistrySystem,
    alert_sender: mpsc::Sender<TradingAlert>,
    monitoring_tasks: Vec<JoinHandle<()>>,
}

impl CrossExchangeMonitor {
    pub async fn start_monitoring(&mut self) {
        // Monitor price discrepancies
        let price_monitor = self.spawn_price_discrepancy_monitor();
        
        // Monitor liquidity imbalances
        let liquidity_monitor = self.spawn_liquidity_imbalance_monitor();
        
        // Monitor regulatory changes
        let regulatory_monitor = self.spawn_regulatory_monitor();
        
        self.monitoring_tasks.extend([
            price_monitor,
            liquidity_monitor,
            regulatory_monitor,
        ]);
    }
    
    fn spawn_price_discrepancy_monitor(&self) -> JoinHandle<()> {
        let registries = self.registries.clone();
        let alert_sender = self.alert_sender.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(100));
            
            loop {
                interval.tick().await;
                
                // Get all multi-venue ISINs
                let isins = registries.instrument_registry.get_multi_venue_isins();
                
                for isin in isins {
                    let prices = registries.get_all_prices_for_isin(&isin).await;
                    
                    if let Some(discrepancy) = detect_price_discrepancy(&prices) {
                        if discrepancy.spread_bps > ALERT_THRESHOLD_BPS {
                            let alert = TradingAlert {
                                alert_type: AlertType::PriceDiscrepancy,
                                isin: isin.clone(),
                                venues: discrepancy.venues,
                                spread_bps: discrepancy.spread_bps,
                                timestamp: SystemTime::now(),
                                action_required: discrepancy.spread_bps > ACTION_THRESHOLD_BPS,
                            };
                            
                            let _ = alert_sender.send(alert).await;
                        }
                    }
                }
            }
        })
    }
}
```

## Unified Trade Execution

```rust
pub struct UnifiedExecutor {
    registries: RegistrySystem,
    venue_connections: HashMap<VenueId, Box<dyn VenueConnection>>,
    execution_log: Arc<ExecutionLog>,
}

impl UnifiedExecutor {
    pub async fn execute_cross_venue_trade(
        &self,
        isin: &str,
        buy_venue: VenueId,
        sell_venue: VenueId,
        quantity: f64,
    ) -> Result<CrossVenueExecution, ExecutionError> {
        // Get instrument IDs for each venue
        let buy_instrument = self.registries.get_instrument_for_venue(isin, buy_venue)?;
        let sell_instrument = self.registries.get_instrument_for_venue(isin, sell_venue)?;
        
        // Validate pre-trade
        self.validate_cross_venue_trade(&buy_instrument, &sell_instrument, quantity)?;
        
        // Execute simultaneously
        let (buy_result, sell_result) = tokio::join!(
            self.execute_single_venue(buy_venue, buy_instrument.id, Side::Buy, quantity),
            self.execute_single_venue(sell_venue, sell_instrument.id, Side::Sell, quantity)
        );
        
        // Handle partial fills
        match (buy_result, sell_result) {
            (Ok(buy_exec), Ok(sell_exec)) => {
                Ok(CrossVenueExecution {
                    isin: isin.to_string(),
                    buy_execution: buy_exec,
                    sell_execution: sell_exec,
                    net_pnl: self.calculate_pnl(&buy_exec, &sell_exec),
                    timestamp: SystemTime::now(),
                })
            }
            (Ok(buy_exec), Err(sell_err)) => {
                // Unwind buy position
                self.unwind_position(buy_venue, buy_instrument.id, buy_exec.filled_quantity).await?;
                Err(ExecutionError::PartialExecution {
                    completed: vec![buy_exec],
                    failed: vec![sell_err],
                })
            }
            (Err(buy_err), Ok(sell_exec)) => {
                // Unwind sell position
                self.unwind_position(sell_venue, sell_instrument.id, sell_exec.filled_quantity).await?;
                Err(ExecutionError::PartialExecution {
                    completed: vec![sell_exec],
                    failed: vec![buy_err],
                })
            }
            (Err(buy_err), Err(sell_err)) => {
                Err(ExecutionError::BothFailed {
                    buy_error: Box::new(buy_err),
                    sell_error: Box::new(sell_err),
                })
            }
        }
    }
}
```

## Performance Metrics

```rust
pub struct CrossExchangeMetrics {
    pub total_isins_tracked: u64,
    pub multi_venue_isins: u64,
    pub avg_venues_per_isin: f64,
    pub total_arbitrage_opportunities: u64,
    pub successful_arbitrages: u64,
    pub failed_arbitrages: u64,
    pub total_pnl_usd: f64,
    pub avg_spread_captured_bps: f64,
    pub best_performing_route: String,
    pub worst_performing_route: String,
}

impl CrossExchangeMetrics {
    pub fn generate_report(&self) -> String {
        format!(
            r#"Cross-Exchange Trading Report
=============================
Total ISINs Tracked: {}
Multi-Venue ISINs: {} ({:.1}%)
Average Venues per ISIN: {:.2}

Arbitrage Performance:
- Opportunities Found: {}
- Successful Executions: {} ({:.1}%)
- Failed Executions: {}
- Total P&L: ${:.2}
- Average Spread Captured: {:.1} bps

Best Route: {}
Worst Route: {}
"#,
            self.total_isins_tracked,
            self.multi_venue_isins,
            self.multi_venue_isins as f64 / self.total_isins_tracked as f64 * 100.0,
            self.avg_venues_per_isin,
            self.total_arbitrage_opportunities,
            self.successful_arbitrages,
            self.successful_arbitrages as f64 / self.total_arbitrage_opportunities as f64 * 100.0,
            self.failed_arbitrages,
            self.total_pnl_usd,
            self.avg_spread_captured_bps,
            self.best_performing_route,
            self.worst_performing_route
        )
    }
}
```
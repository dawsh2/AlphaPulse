# Portfolio Manager Module Specification

## Executive Summary

The Portfolio Manager is a stateful service responsible for maintaining accurate position tracking, P&L calculation, and trade attribution across all asset classes and chains. It acts as the system's financial state keeper, providing real-time portfolio information to the Risk Manager while maintaining comprehensive audit trails for post-trade analysis.

## Core Requirements

### Performance Targets
- **Position Update Latency**: <10ms from execution result to updated state
- **State Query Response**: <1ms for portfolio state requests
- **Concurrent Positions**: Support 10,000+ active positions across all chains
- **Attribution Granularity**: Track every trade with strategy/venue/token attribution

### Reliability Requirements
- **State Consistency**: Perfect reconciliation between execution results and positions
- **Multi-Chain Coordination**: Accurate cross-chain position tracking
- **Audit Trail**: Complete history of all position changes for compliance
- **Recovery**: Rebuild state from execution event history

---

# Part I: Architecture Overview

## Service Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        PORTFOLIO MANAGER SERVICE                            │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────────┐ │
│  │                     Event Processing Pipeline                           │ │
│  │                                                                         │ │
│  │  ExecutionRelay ──→ Result Parser ──→ Position Engine ──→ State Store   │ │
│  │       │                 │                    │               │          │ │
│  │       ↓                 ↓                    ↓               ↓          │ │
│  │  [ExecutionResult]  [Trade Analysis]   [Position Δ]    [Updated State] │ │
│  │  [Fill TLV]         [Attribution]      [P&L Calc]     [Event Archive]  │ │
│  └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────────┐ │
│  │                      Position State Manager                             │ │
│  │                                                                         │ │
│  │  Active Positions: BTreeMap<u64, Position>  // key = instrument_id.to_u64() │ │
│  │  Chain Balances: BTreeMap<u64, Balance>    // key = (chain_id << 32) | token_id │ │
│  │  Strategy Allocations: BTreeMap<u16, StrategyPortfolio>  // by strategy_id │ │
│  │  Temporary Positions: BTreeMap<u128, Vec<IntermediatePosition>>  // by trace_id │ │
│  │                                                                         │ │
│  │  • Real-time position tracking across all chains                       │ │
│  │  • Strategy-level portfolio attribution                                │ │
│  │  • Intermediate state tracking for multi-step trades                   │ │
│  └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────────┐ │
│  │                        Query Interface                                  │ │
│  │                                                                         │ │
│  │  Risk Manager ←── Portfolio Queries ←── Position Engine                │ │
│  │       │                    │                    │                      │ │
│  │       ↓                    ↓                    ↓                      │ │
│  │  [Position State]    [Portfolio Summary]  [Real-time Updates]         │ │
│  │  [Risk Metrics]      [Strategy P&L]       [Balance Changes]           │ │
│  └─────────────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Data Flow Integration

```
ExecutionRelay ──→ Portfolio Manager ──→ Risk Manager
     │                    │                     │
     ↓                    ↓                     ↓
[ExecutionResult TLV]  [Position Updates]  [Portfolio State]
[Fill TLV]             [P&L Calculation]   [Risk Queries]
[OrderStatus TLV]      [Attribution]       [Position Limits]
     │                    │                     │
     ↓                    ↓                     ↓
   Archive          Event Sourcing        Risk Decisions
```

---

# Part II: Position Data Model

## Unified Position Structure

### Core Position Definition

```rust
#[derive(Debug, Clone)]
pub struct Position {
    // Identity
    pub instrument_id: InstrumentId,        // Bijective instrument identifier
    pub chain_id: u32,                     // Chain where position exists
    pub strategy_id: u16,                  // Strategy that created position
    
    // Quantity and Valuation
    pub quantity: i128,                    // Signed quantity (+ long, - short)
    pub avg_cost_basis: u128,              // Average cost per unit
    pub market_value: u128,                // Current market value
    pub unrealized_pnl: i128,              // Mark-to-market P&L
    
    // Timing
    pub first_trade_time: u64,             // When position was opened
    pub last_update_time: u64,             // Last modification time
    
    // Attribution
    pub total_trades: u32,                 // Number of trades in position
    pub realized_pnl: i128,                // Realized P&L from closes
    pub fees_paid: u128,                   // Total fees on this position
    
    // Asset-specific metadata
    pub asset_metadata: AssetMetadata,     // Asset type-specific data
    
    // Risk management
    pub risk_weight: f64,                  // Risk weighting for portfolio
    pub correlation_group: u16,            // Asset correlation grouping
}

#[derive(Debug, Clone)]
pub enum AssetMetadata {
    Crypto {
        decimals: u8,
        is_stablecoin: bool,
    },
    Equity {
        dividend_yield: f64,
        ex_dividend_date: Option<u64>,
        sector: u16,
    },
    Option {
        underlying: InstrumentId,
        strike_price: u128,
        expiry_time: u64,
        option_type: OptionType,
        implied_volatility: f64,
    },
    Future {
        underlying: InstrumentId,
        contract_size: u128,
        settlement_date: u64,
        margin_requirement: u128,
        tick_size: u128,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionType {
    Call,
    Put,
}
```

### Chain Balance Tracking

```rust
#[derive(Debug, Clone)]
pub struct Balance {
    pub token_id: InstrumentId,     // What token
    pub chain_id: u32,              // Which chain
    pub available: u128,            // Available for trading
    pub reserved: u128,             // Reserved for pending orders
    pub total: u128,                // Total balance
    pub last_update: u64,           // Last update timestamp
}

impl Balance {
    pub fn reserve(&mut self, amount: u128) -> Result<(), InsufficientBalance> {
        if self.available >= amount {
            self.available -= amount;
            self.reserved += amount;
            Ok(())
        } else {
            Err(InsufficientBalance { requested: amount, available: self.available })
        }
    }
    
    pub fn release_reservation(&mut self, amount: u128) {
        let release_amount = amount.min(self.reserved);
        self.reserved -= release_amount;
        self.available += release_amount;
    }
}
```

### Strategy Portfolio Attribution

```rust
#[derive(Debug, Clone)]
pub struct StrategyPortfolio {
    pub strategy_id: u16,
    pub positions: BTreeMap<u64, Position>,     // instrument_id.to_u64() -> position
    pub total_pnl: i128,                        // Total realized + unrealized P&L
    pub total_fees: u128,                       // Total fees paid
    pub trade_count: u32,                       // Number of trades executed
    pub max_drawdown: i128,                     // Maximum drawdown observed
    pub sharpe_ratio: f64,                      // Risk-adjusted returns
    pub inception_time: u64,                    // When strategy started
}
```

---

# Part III: Event Processing Architecture

## Execution Result Processing

### Main Event Loop

```rust
pub struct PortfolioManager {
    // Position storage
    positions: Arc<RwLock<BTreeMap<u64, Position>>>,            // Main position store
    balances: Arc<RwLock<BTreeMap<u64, Balance>>>,              // Chain balances
    strategy_portfolios: Arc<RwLock<BTreeMap<u16, StrategyPortfolio>>>,
    temporary_positions: Arc<RwLock<BTreeMap<u128, Vec<IntermediatePosition>>>>,
    
    // Communication
    execution_relay_connection: UnixStream,
    query_server: QueryServer,
    
    // Configuration
    config: PortfolioConfig,
    
    // Metrics and monitoring
    metrics: PortfolioMetrics,
}

impl PortfolioManager {
    pub async fn run(&mut self) -> Result<(), PortfolioError> {
        let mut execution_receiver = self.execution_relay_connection.clone();
        let mut cleanup_interval = tokio::time::interval(Duration::from_secs(60));
        
        loop {
            tokio::select! {
                // Process execution results from ExecutionRelay
                msg = read_tlv_message(&mut execution_receiver) => {
                    if let Ok(message) = msg {
                        self.process_execution_message(message).await?;
                    }
                }
                
                // Handle portfolio queries from Risk Manager
                query = self.query_server.next_query() => {
                    if let Some(query) = query {
                        self.handle_portfolio_query(query).await?;
                    }
                }
                
                // Periodic cleanup of temporary positions
                _ = cleanup_interval.tick() => {
                    self.cleanup_expired_temporary_positions().await;
                }
                
                // Graceful shutdown
                _ = tokio::signal::ctrl_c() => {
                    self.graceful_shutdown().await?;
                    break;
                }
            }
        }
        
        Ok(())
    }
    
    async fn process_execution_message(&mut self, message: TLVMessage) -> Result<(), PortfolioError> {
        let header = parse_header(&message.data)?;
        let tlv_payload = &message.data[32..32 + header.payload_size as usize];
        let tlvs = parse_tlv_extensions(tlv_payload)?;
        
        // Extract trace context for temporary position tracking
        let trace_context = tlvs.iter()
            .find(|tlv| tlv.header.tlv_type == TLVType::TraceContext as u8)
            .and_then(|tlv| parse_trace_context(&tlv.payload).ok());
        
        for tlv in tlvs {
            match TLVType::try_from(tlv.header.tlv_type)? {
                TLVType::ExecutionResult => {
                    let result = parse_execution_result(&tlv.payload)?;
                    self.process_execution_result(result, trace_context.as_ref()).await?;
                }
                TLVType::Fill => {
                    let fill = parse_fill(&tlv.payload)?;
                    self.process_fill_event(fill, trace_context.as_ref()).await?;
                }
                TLVType::OrderStatus => {
                    let status = parse_order_status(&tlv.payload)?;
                    self.process_status_update(status).await?;
                }
                _ => {
                    // Ignore unknown TLVs for forward compatibility
                }
            }
        }
        
        Ok(())
    }
}
```

### Position Update Logic

```rust
impl PortfolioManager {
    async fn process_execution_result(
        &mut self, 
        result: ExecutionResultTLV, 
        trace_context: Option<&TraceContextTLV>
    ) -> Result<(), PortfolioError> {
        let processing_start = current_nanos();
        
        // Extract execution details
        let instrument_key = result.instrument_id.to_u64();
        let strategy_id = self.extract_strategy_from_order_id(result.order_id)?;
        
        // Update position based on execution result
        match result.result_type {
            1 => {  // Filled
                self.apply_fill_to_position(
                    instrument_key,
                    strategy_id,
                    result.quantity_filled as i128,  // Assuming buys are positive
                    result.average_price,
                    result.total_fees,
                    result.execution_time
                ).await?;
                
                // Track temporary positions if this is part of multi-step trade
                if let Some(trace_ctx) = trace_context {
                    if trace_ctx.flags & TRACE_FLAG_ARBITRAGE_OPPORTUNITY != 0 {
                        self.track_temporary_position(trace_ctx, &result).await?;
                    }
                }
            }
            2 => {  // Rejected
                // No position impact, but update strategy metrics
                self.record_failed_execution(strategy_id, result.order_id).await?;
            }
            _ => {
                // Unknown result type - log but don't fail
                tracing::warn!("Unknown execution result type: {}", result.result_type);
            }
        }
        
        // Update performance metrics
        let processing_time = current_nanos() - processing_start;
        self.metrics.position_update_time.record(Duration::from_nanos(processing_time));
        
        Ok(())
    }
    
    async fn apply_fill_to_position(
        &mut self,
        instrument_key: u64,
        strategy_id: u16,
        quantity_delta: i128,
        fill_price: u128,
        fees: u128,
        fill_time: u64
    ) -> Result<(), PortfolioError> {
        // Update main position
        {
            let mut positions = self.positions.write().await;
            let position = positions.entry(instrument_key).or_insert_with(|| {
                Position::new(
                    InstrumentId::from_u64(instrument_key),
                    strategy_id,
                    fill_time
                )
            });
            
            // Apply quantity and cost basis changes
            if position.quantity == 0 {
                // New position
                position.quantity = quantity_delta;
                position.avg_cost_basis = fill_price;
            } else if position.quantity.signum() == quantity_delta.signum() {
                // Adding to existing position
                let total_cost = (position.avg_cost_basis * position.quantity.unsigned_abs()) 
                    + (fill_price * quantity_delta.unsigned_abs());
                position.quantity += quantity_delta;
                position.avg_cost_basis = total_cost / position.quantity.unsigned_abs();
            } else {
                // Reducing or reversing position
                let abs_delta = quantity_delta.unsigned_abs();
                if abs_delta >= position.quantity.unsigned_abs() {
                    // Position reversal
                    let realized_pnl = self.calculate_realized_pnl(
                        position.quantity.unsigned_abs(),
                        position.avg_cost_basis,
                        fill_price
                    );
                    position.realized_pnl += realized_pnl;
                    position.quantity = quantity_delta + position.quantity;
                    position.avg_cost_basis = fill_price;
                } else {
                    // Partial reduction
                    let realized_pnl = self.calculate_realized_pnl(
                        abs_delta,
                        position.avg_cost_basis,
                        fill_price
                    );
                    position.realized_pnl += realized_pnl;
                    position.quantity += quantity_delta;
                }
            }
            
            // Update position metadata
            position.total_trades += 1;
            position.fees_paid += fees;
            position.last_update_time = fill_time;
        }
        
        // Update strategy portfolio
        {
            let mut strategy_portfolios = self.strategy_portfolios.write().await;
            let strategy_portfolio = strategy_portfolios.entry(strategy_id).or_insert_with(|| {
                StrategyPortfolio::new(strategy_id, fill_time)
            });
            
            strategy_portfolio.trade_count += 1;
            strategy_portfolio.total_fees += fees;
            // P&L will be recalculated during mark-to-market updates
        }
        
        // Emit position change event
        self.emit_position_change_event(instrument_key, strategy_id).await;
        
        Ok(())
    }
}
```

---

# Part IV: Query Interface

## Risk Manager Integration

### Portfolio Query Protocol

```rust
#[derive(Debug, Clone)]
pub enum PortfolioQuery {
    GetPosition {
        instrument_id: InstrumentId,
        strategy_id: Option<u16>,
    },
    GetStrategyPortfolio {
        strategy_id: u16,
    },
    GetChainBalances {
        chain_id: u32,
    },
    GetPortfolioSummary,
    GetRiskMetrics {
        lookback_period: Duration,
    },
}

#[derive(Debug, Clone)]
pub enum PortfolioResponse {
    Position(Option<Position>),
    StrategyPortfolio(StrategyPortfolio),
    ChainBalances(Vec<Balance>),
    PortfolioSummary(PortfolioSummary),
    RiskMetrics(RiskMetrics),
    Error(String),
}

impl PortfolioManager {
    async fn handle_portfolio_query(&mut self, query: PortfolioQuery) -> PortfolioResponse {
        match query {
            PortfolioQuery::GetPosition { instrument_id, strategy_id } => {
                let key = instrument_id.to_u64();
                let positions = self.positions.read().await;
                
                if let Some(position) = positions.get(&key) {
                    // Filter by strategy if specified
                    if let Some(sid) = strategy_id {
                        if position.strategy_id == sid {
                            PortfolioResponse::Position(Some(position.clone()))
                        } else {
                            PortfolioResponse::Position(None)
                        }
                    } else {
                        PortfolioResponse::Position(Some(position.clone()))
                    }
                } else {
                    PortfolioResponse::Position(None)
                }
            }
            
            PortfolioQuery::GetStrategyPortfolio { strategy_id } => {
                let strategy_portfolios = self.strategy_portfolios.read().await;
                
                if let Some(portfolio) = strategy_portfolios.get(&strategy_id) {
                    PortfolioResponse::StrategyPortfolio(portfolio.clone())
                } else {
                    PortfolioResponse::StrategyPortfolio(StrategyPortfolio::empty(strategy_id))
                }
            }
            
            PortfolioQuery::GetPortfolioSummary => {
                let summary = self.calculate_portfolio_summary().await;
                PortfolioResponse::PortfolioSummary(summary)
            }
            
            PortfolioQuery::GetRiskMetrics { lookback_period } => {
                let metrics = self.calculate_risk_metrics(lookback_period).await;
                PortfolioResponse::RiskMetrics(metrics)
            }
            
            PortfolioQuery::GetChainBalances { chain_id } => {
                let balances = self.get_balances_for_chain(chain_id).await;
                PortfolioResponse::ChainBalances(balances)
            }
        }
    }
}
```

### Real-Time Portfolio Updates

```rust
pub struct PortfolioUpdateStream {
    receiver: mpsc::Receiver<PortfolioUpdate>,
}

#[derive(Debug, Clone)]
pub struct PortfolioUpdate {
    pub update_type: UpdateType,
    pub instrument_id: InstrumentId,
    pub strategy_id: u16,
    pub old_position: Option<Position>,
    pub new_position: Option<Position>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateType {
    PositionOpened,
    PositionClosed,
    PositionModified,
    BalanceChanged,
}

impl PortfolioManager {
    async fn emit_position_change_event(&self, instrument_key: u64, strategy_id: u16) {
        let positions = self.positions.read().await;
        let position = positions.get(&instrument_key).cloned();
        
        let update = PortfolioUpdate {
            update_type: if position.as_ref().map_or(true, |p| p.quantity == 0) {
                UpdateType::PositionClosed
            } else {
                UpdateType::PositionModified
            },
            instrument_id: InstrumentId::from_u64(instrument_key),
            strategy_id,
            old_position: None,  // Could store previous state if needed
            new_position: position,
            timestamp: current_nanos(),
        };
        
        // Send to any subscribed Risk Managers
        if let Err(e) = self.update_sender.send(update).await {
            tracing::error!("Failed to send portfolio update: {}", e);
        }
    }
}
```

---

# Part V: Trade Attribution and Analysis

## Temporary Position Tracking

### Multi-Step Trade Correlation

```rust
#[derive(Debug, Clone)]
pub struct IntermediatePosition {
    pub step_number: u16,           // Order within multi-step trade
    pub instrument_id: InstrumentId,
    pub quantity_delta: i128,       // Position change at this step
    pub price: u128,                // Execution price
    pub timestamp: u64,             // When this step occurred
    pub venue_id: u16,              // Where this step executed
    pub step_type: StepType,        // Type of operation
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepType {
    Entry,          // Opening a position
    Intermediate,   // Middle step in arbitrage
    Exit,           // Closing a position
    Bridge,         // Cross-chain transfer
}

impl PortfolioManager {
    async fn track_temporary_position(
        &mut self,
        trace_context: &TraceContextTLV,
        execution_result: &ExecutionResultTLV
    ) -> Result<(), PortfolioError> {
        let trace_id = trace_context.trace_id;
        
        let intermediate_position = IntermediatePosition {
            step_number: 0,  // Will be set based on existing steps
            instrument_id: execution_result.instrument_id,
            quantity_delta: execution_result.quantity_filled as i128,
            price: execution_result.average_price,
            timestamp: execution_result.execution_time,
            venue_id: self.extract_venue_from_execution(execution_result)?,
            step_type: self.determine_step_type(trace_context)?,
        };
        
        {
            let mut temp_positions = self.temporary_positions.write().await;
            let positions = temp_positions.entry(trace_id).or_insert_with(Vec::new);
            
            // Set step number based on existing steps
            intermediate_position.step_number = positions.len() as u16;
            positions.push(intermediate_position);
        }
        
        Ok(())
    }
    
    async fn cleanup_expired_temporary_positions(&mut self) {
        let cutoff_time = current_nanos() - Duration::from_secs(300).as_nanos() as u64; // 5 minutes
        
        let mut temp_positions = self.temporary_positions.write().await;
        temp_positions.retain(|_trace_id, positions| {
            positions.iter().any(|pos| pos.timestamp > cutoff_time)
        });
    }
}
```

## Performance Attribution

### Strategy-Level P&L Analysis

```rust
#[derive(Debug, Clone)]
pub struct PerformanceAttribution {
    pub strategy_id: u16,
    pub time_period: TimeRange,
    pub total_pnl: i128,
    pub realized_pnl: i128,
    pub unrealized_pnl: i128,
    pub fees_paid: u128,
    pub trade_count: u32,
    pub win_rate: f64,
    pub average_trade_pnl: i128,
    pub sharpe_ratio: f64,
    pub max_drawdown: i128,
    pub venue_attribution: BTreeMap<u16, VenueAttribution>,
    pub token_pair_attribution: BTreeMap<(u64, u64), TokenPairAttribution>,
}

#[derive(Debug, Clone)]
pub struct VenueAttribution {
    pub venue_id: u16,
    pub trade_count: u32,
    pub total_pnl: i128,
    pub average_latency: Duration,
    pub success_rate: f64,
}

#[derive(Debug, Clone)]
pub struct TokenPairAttribution {
    pub base_token: InstrumentId,
    pub quote_token: InstrumentId,
    pub trade_count: u32,
    pub total_pnl: i128,
    pub total_volume: u128,
    pub average_spread_captured: f64,
}

impl PortfolioManager {
    pub async fn generate_performance_attribution(
        &self,
        strategy_id: u16,
        time_range: TimeRange
    ) -> Result<PerformanceAttribution, PortfolioError> {
        // This would query the archived trade data and compute attribution
        // Implementation details depend on your specific analysis requirements
        
        let strategy_portfolios = self.strategy_portfolios.read().await;
        let portfolio = strategy_portfolios.get(&strategy_id)
            .ok_or(PortfolioError::StrategyNotFound(strategy_id))?;
        
        // Calculate basic metrics from current state
        let mut attribution = PerformanceAttribution {
            strategy_id,
            time_period: time_range,
            total_pnl: portfolio.total_pnl,
            realized_pnl: 0,  // Would calculate from historical data
            unrealized_pnl: 0, // Would calculate from current positions
            fees_paid: portfolio.total_fees,
            trade_count: portfolio.trade_count,
            win_rate: 0.0,    // Would calculate from trade history
            average_trade_pnl: if portfolio.trade_count > 0 { 
                portfolio.total_pnl / portfolio.trade_count as i128 
            } else { 0 },
            sharpe_ratio: portfolio.sharpe_ratio,
            max_drawdown: portfolio.max_drawdown,
            venue_attribution: BTreeMap::new(),
            token_pair_attribution: BTreeMap::new(),
        };
        
        // Additional detailed attribution would require querying archived data
        // This is where you'd implement venue-specific and token-pair analysis
        
        Ok(attribution)
    }
}
```

---

# Part VI: Configuration and Deployment

## Configuration Management

```toml
# config/production/portfolio.toml
[portfolio_manager]
max_positions = 10000
position_update_timeout_ms = 10
query_response_timeout_ms = 1
mark_to_market_interval_seconds = 60

[attribution]
track_temporary_positions = true
temporary_position_ttl_seconds = 300
enable_detailed_attribution = true
attribution_granularity = "trade_level"  # "trade_level" | "daily" | "position_level"

[risk_metrics]
var_confidence_level = 0.95
var_holding_period_days = 1
correlation_lookback_days = 30
volatility_window_days = 30

[storage]
archive_completed_trades = true
archive_compression = "lz4"
hot_data_retention_days = 30
cold_storage_transition_days = 90

[performance]
position_cache_size = 1000
query_cache_ttl_seconds = 5
enable_query_caching = true
max_concurrent_queries = 100

[monitoring]
emit_position_metrics = true
metrics_update_interval_seconds = 10
slow_query_threshold_ms = 10
position_reconciliation_interval_seconds = 300
```

## Health Monitoring

```rust
#[derive(Debug, Default)]
pub struct PortfolioMetrics {
    // Performance metrics
    pub position_updates_processed: Counter,
    pub position_update_time: Histogram,
    pub query_response_time: Histogram,
    pub queries_processed: Counter,
    
    // Position metrics
    pub total_positions: Gauge,
    pub positions_by_strategy: BTreeMap<u16, Gauge>,
    pub total_portfolio_value: Gauge,
    pub unrealized_pnl: Gauge,
    pub realized_pnl_today: Gauge,
    
    // Attribution metrics
    pub temporary_positions_tracked: Gauge,
    pub attribution_calculations: Counter,
    pub attribution_calculation_time: Histogram,
    
    // Error metrics
    pub position_update_errors: Counter,
    pub query_errors: Counter,
    pub reconciliation_errors: Counter,
    
    // Business metrics
    pub trades_processed_today: Counter,
    pub fees_paid_today: Gauge,
    pub active_strategies: Gauge,
}

impl PortfolioManager {
    pub async fn health_check(&self) -> HealthStatus {
        let mut status = HealthStatus::new();
        
        // Check position update performance
        if self.metrics.position_update_time.p99() > Duration::from_millis(10) {
            status.add_component("position_updates".to_string(), HealthLevel::Degraded);
        }
        
        // Check query performance
        if self.metrics.query_response_time.p99() > Duration::from_millis(1) {
            status.add_component("query_performance".to_string(), HealthLevel::Degraded);
        }
        
        // Check for position reconciliation issues
        if self.metrics.reconciliation_errors.get() > 0 {
            status.add_component("position_reconciliation".to_string(), HealthLevel::Degraded);
        }
        
        // Check memory usage (position count)
        if self.metrics.total_positions.get() > 8000.0 {  // 80% of 10k limit
            status.add_component("position_capacity".to_string(), HealthLevel::Degraded);
        }
        
        status
    }
}
```

This Portfolio Manager specification provides:

1. **Unified Position Tracking** across all asset types and chains
2. **Strategy-Level Attribution** for detailed performance analysis  
3. **Temporary Position Tracking** tied to distributed tracing for debugging
4. **Real-Time Query Interface** for Risk Manager integration
5. **Comprehensive Trade Attribution** for post-trade analysis
6. **Scalable Storage Design** using bijective ID-based BTreeMaps

The design is stateful but passive - it receives execution results, updates positions, and serves queries to the Risk Manager. This clean separation enables independent testing and scaling of each component.

Ready to move on to the Risk Manager module next?

# Strategy-Based Architecture Refactor

## Current Problem

The DeFi scanner currently has mixed concerns - the `OpportunityDetector` handles scanning, gas estimation, contract interaction, and execution all in one monolithic component. This makes it difficult to:

- Add new arbitrage strategies
- Test strategies independently  
- Optimize specific execution methods
- Scale different components independently
- Maintain clean separation of concerns

## Proposed Architecture

### Directory Structure
```
backend/services/defi/
├── strategies/                    # Strategy implementations
│   ├── mod.rs                    # Strategy trait + factory
│   ├── arbitrage_v2.rs           # V2 DEX arbitrage strategy
│   ├── arbitrage_v3.rs           # V3 DEX arbitrage strategy  
│   ├── flash_arbitrage.rs        # Flash loan arbitrage
│   ├── cross_chain_arbitrage.rs  # Bridge arbitrage
│   └── compound_arbitrage.rs     # Multi-hop complex paths
├── execution/                    # Contract interaction layer
│   ├── mod.rs                    # Execution trait
│   ├── huff_executor.rs          # Huff contract execution
│   ├── solidity_executor.rs      # Standard contract execution
│   ├── simulation.rs             # Pre-execution simulation
│   └── flash_loan_provider.rs    # Flash loan integration
├── infrastructure/               # Cross-cutting concerns  
│   ├── gas_estimation.rs         # Gas cost calculation
│   ├── pool_aggregator.rs        # Cross-DEX pool data
│   ├── price_feed.rs             # Price oracles
│   ├── metrics.rs                # Performance monitoring
│   └── risk_assessment.rs        # Risk scoring
├── scanner/                      # Core scanning engine
│   ├── opportunity_scanner.rs    # Strategy-agnostic scanner
│   ├── pool_monitor.rs           # Pool data monitoring (existing)
│   └── event_processor.rs        # Blockchain event handling
└── config/                       # Configuration
    ├── strategy_config.rs        # Strategy-specific config
    └── network_config.rs         # Network/RPC config
```

## Core Strategy Pattern

### Strategy Trait
```rust
#[async_trait]
pub trait ArbitrageStrategy: Send + Sync {
    /// Strategy identifier
    fn name(&self) -> &str;
    
    /// Check if this strategy can handle the given pools
    fn can_execute(&self, pools: &[PoolInfo]) -> bool;
    
    /// Scan for opportunities using this strategy
    async fn scan_opportunities(
        &self,
        pools: &[PoolInfo],
        context: &MarketContext,
    ) -> Result<Vec<ArbitrageOpportunity>>;
    
    /// Validate an opportunity before execution
    async fn validate_opportunity(
        &self,
        opportunity: &ArbitrageOpportunity,
    ) -> Result<ValidationResult>;
    
    /// Get estimated gas cost for this strategy
    async fn estimate_gas_cost(
        &self,
        opportunity: &ArbitrageOpportunity,
    ) -> Result<GasEstimate>;
    
    /// Get strategy-specific configuration
    fn config(&self) -> &StrategyConfig;
}
```

### Strategy Configuration
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    pub name: String,
    pub enabled: bool,
    pub min_profit_usd: Decimal,
    pub min_profit_percentage: Decimal,
    pub max_gas_cost_usd: Decimal,
    pub risk_tolerance: RiskLevel,
    pub execution_method: ExecutionMethod,
    pub target_tokens: Vec<String>, // Strategy-specific token focus
    pub max_hops: usize,           // Maximum pools in path
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Conservative,  // High confidence, lower profits
    Moderate,      // Balanced risk/reward
    Aggressive,    // Higher risk, higher potential profits
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionMethod {
    Simulation,        // Dry run only
    HuffContract,      // Ultra-efficient Huff execution
    StandardContract,  // Standard Solidity execution
    FlashLoan,         // Flash loan based execution
    Hybrid,            // Multiple methods based on opportunity
}
```

## Strategy Implementations

### 1. V2 Arbitrage Strategy
**Focus**: Traditional AMM pools (Uniswap V2, SushiSwap)
- Simple reserve-based calculations
- Direct token swaps
- Lower complexity, faster execution
- **Gas Target**: 345,200 gas with Huff optimization

```rust
pub struct ArbitrageV2Strategy {
    config: StrategyConfig,
    gas_estimator: Arc<dyn GasEstimator>,
    execution_interface: Arc<dyn ExecutionInterface>,
}

impl ArbitrageV2Strategy {
    // Scan pairs of V2 pools for price differences
    // Calculate optimal trade size considering slippage
    // Validate liquidity depth
}
```

### 2. V3 Arbitrage Strategy  
**Focus**: Concentrated liquidity pools (Uniswap V3)
- Tick-based liquidity calculations
- Complex price curves
- Higher gas but potentially better rates
- **Gas Target**: 415,200 gas

```rust
pub struct ArbitrageV3Strategy {
    config: StrategyConfig,
    tick_math: Arc<V3TickMath>,
    liquidity_calculator: Arc<V3LiquidityCalculator>,
}

impl ArbitrageV3Strategy {
    // Handle tick boundaries and concentrated liquidity
    // Calculate price impact across tick ranges
    // Optimize for V3 pool efficiency
}
```

### 3. Flash Arbitrage Strategy
**Focus**: Capital-free arbitrage using flash loans
- AAVE, dYdX flash loan integration
- Larger trade sizes possible
- Higher profit potential but more gas
- **Gas Target**: 478,100+ gas

```rust
pub struct FlashArbitrageStrategy {
    config: StrategyConfig,
    flash_providers: Vec<Arc<dyn FlashLoanProvider>>,
    liquidation_buffer: Decimal,
}

impl FlashArbitrageStrategy {
    // Route through cheapest flash loan provider
    // Calculate optimal borrowed amount
    // Account for flash loan fees
}
```

### 4. Cross-Chain Arbitrage Strategy
**Focus**: L1 ↔ L2 bridge arbitrage
- Monitor bridge rates vs DEX rates
- Account for bridge time and fees
- Higher complexity, longer settlement
- **Target**: Opportunities > $100 profit

### 5. Compound Arbitrage Strategy
**Focus**: Multi-hop complex paths (4+ pools)
- Multi-DEX routing optimization
- Circular arbitrage paths
- Long-tail token opportunities
- **Enabled by**: Ultra-low gas costs from Huff

## Execution Layer Separation

### Execution Interface
```rust
#[async_trait]
pub trait ExecutionInterface: Send + Sync {
    /// Execute an arbitrage opportunity
    async fn execute(
        &self,
        opportunity: &ArbitrageOpportunity,
        strategy_metadata: &StrategyMetadata,
    ) -> Result<ExecutionResult>;
    
    /// Simulate execution without submitting transaction
    async fn simulate(
        &self,
        opportunity: &ArbitrageOpportunity,
    ) -> Result<SimulationResult>;
    
    /// Get current gas price and execution cost
    async fn get_execution_cost(&self) -> Result<GasEstimate>;
}
```

### Huff Executor
```rust
pub struct HuffExecutor {
    contract_address: Address,
    provider: Arc<Provider<Http>>,
    wallet: Arc<LocalWallet>,
    gas_estimator: Arc<HuffGasEstimator>,
}

impl HuffExecutor {
    // Ultra-efficient execution using deployed Huff contracts
    // Minimal gas overhead, maximum profit extraction
    // Optimized for high-frequency opportunities
}
```

## Infrastructure Components

### Gas Estimation Service
```rust
pub struct GasEstimationService {
    huff_estimator: Option<Arc<HuffGasEstimator>>,
    fallback_estimator: Arc<StaticGasEstimator>,
    network_config: NetworkConfig,
}

impl GasEstimationService {
    // Strategy-aware gas estimation
    // Network-specific optimizations
    // Real-time gas price feeds
}
```

### Pool Aggregator
```rust
pub struct PoolAggregator {
    pool_sources: Vec<Arc<dyn PoolDataSource>>,
    cache: Arc<PoolCache>,
    update_scheduler: Arc<UpdateScheduler>,
}

impl PoolAggregator {
    // Aggregate pools from multiple DEXs
    // Maintain real-time reserve data
    // Filter by strategy requirements
}
```

## Migration Strategy

### Phase 1: Core Strategy Framework
1. Create strategy trait and factory
2. Move current arbitrage logic to V2Strategy
3. Update scanner to use strategy factory
4. Maintain existing functionality

### Phase 2: Execution Separation  
1. Extract execution logic to ExecutionInterface
2. Create HuffExecutor with current gas estimator
3. Add simulation capabilities
4. Clean up circular dependencies

### Phase 3: Infrastructure Services
1. Move gas estimation to infrastructure layer
2. Create pool aggregation service
3. Add risk assessment module
4. Implement metrics collection

### Phase 4: New Strategies
1. Implement V3 strategy with tick math
2. Add flash arbitrage strategy
3. Create compound/multi-hop strategy
4. Add cross-chain bridge arbitrage

### Phase 5: Advanced Features
1. Dynamic strategy selection
2. Portfolio optimization across strategies
3. MEV-aware execution ordering
4. Machine learning profit prediction

## Benefits of This Architecture

### 1. **Modularity**
- Each strategy can be developed, tested, and deployed independently
- Easy to add new arbitrage methods without touching existing code
- Clean separation between scanning, validation, and execution

### 2. **Scalability**
- Different strategies can run on different threads/processes
- Infrastructure services can be scaled independently
- Strategy-specific optimizations don't affect others

### 3. **Testability**
- Mock execution interfaces for strategy testing
- Independent unit tests for each component
- Strategy-specific integration tests

### 4. **Flexibility**
- Runtime strategy configuration changes
- A/B testing of different approaches
- Strategy-specific risk management

### 5. **Performance**
- Hot path optimization per strategy type
- Parallel opportunity scanning
- Strategy-aware resource allocation

## Configuration Example

```toml
[strategies.arbitrage_v2]
enabled = true
min_profit_usd = 5.0
min_profit_percentage = 0.0008  # 0.08%
max_gas_cost_usd = 2.0
risk_tolerance = "moderate"
execution_method = "huff_contract"
target_tokens = ["WETH", "USDC", "WMATIC", "USDT"]
max_hops = 2

[strategies.flash_arbitrage]
enabled = true
min_profit_usd = 25.0
min_profit_percentage = 0.0015  # 0.15%
max_gas_cost_usd = 5.0
risk_tolerance = "aggressive"
execution_method = "flash_loan"
target_tokens = ["WETH", "USDC", "WBTC", "DAI"]
max_hops = 4

[execution.huff]
contract_address = "0x..."
bot_address = "0x..."
max_gas_price_gwei = 100
slippage_tolerance = 0.005  # 0.5%

[infrastructure.gas_estimation]
provider = "huff"
fallback_gas_floor = 345200
update_interval_ms = 1000
```

This architecture enables the system to evolve from a monolithic arbitrage detector into a comprehensive, strategy-driven MEV extraction platform while maintaining the performance gains from Huff optimization.
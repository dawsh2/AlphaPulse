# Crypto Arbitrage System - Recommended Architecture

## Overview

A modular, layered architecture for crypto arbitrage bots that promotes code reuse, testability, and scalability.

## Proposed Directory Structure

```
backend/
├── trading/                       # Core trading infrastructure
│   ├── Cargo.toml                # Workspace for all trading components
│   │
│   ├── core/                     # Shared core functionality
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── types.rs          # Common types (Opportunity, ExecutionResult, etc.)
│   │       ├── metrics.rs        # Unified metrics collection
│   │       ├── config.rs         # Configuration management
│   │       └── errors.rs         # Custom error types
│   │
│   ├── dex/                      # DEX interaction layer
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── router.rs         # Generic DEX router interface
│   │       ├── uniswap_v2.rs     # UniswapV2-compatible implementation
│   │       ├── uniswap_v3.rs     # UniswapV3 implementation
│   │       ├── curve.rs          # Curve implementation
│   │       ├── balancer.rs       # Balancer implementation
│   │       └── aggregator.rs     # 1inch/0x aggregator support
│   │
│   ├── strategies/               # Trading strategies
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── arbitrage/
│   │       │   ├── mod.rs
│   │       │   ├── two_hop.rs    # Simple A->B->A arbitrage
│   │       │   ├── multi_hop.rs  # Complex path arbitrage
│   │       │   └── triangular.rs # Three-token arbitrage
│   │       ├── market_making/
│   │       │   └── mod.rs
│   │       └── liquidation/
│   │           └── mod.rs
│   │
│   ├── execution/                # Execution engines
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── capital.rs        # Use own capital
│   │       ├── flash_loan.rs     # Flash loan execution
│   │       ├── flash_swap.rs     # UniswapV3 flash swaps
│   │       └── simulator.rs      # Transaction simulation
│   │
│   ├── validation/               # Opportunity validation
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── profitability.rs  # Profit calculation
│   │       ├── gas_estimator.rs  # Gas cost estimation
│   │       ├── slippage.rs       # Slippage calculation
│   │       └── risk.rs           # Risk assessment
│   │
│   ├── collectors/               # Market data collection
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── mempool.rs        # Mempool monitoring
│   │       ├── events.rs         # DEX event monitoring
│   │       └── scanner.rs        # Active price scanning
│   │
│   └── bots/                     # Actual bot implementations
│       ├── flash_arb_bot/        # Flash loan arbitrage bot
│       │   ├── Cargo.toml
│       │   └── src/
│       │       └── main.rs       # Thin wrapper using shared components
│       │
│       ├── capital_arb_bot/      # Capital-based arbitrage bot
│       │   ├── Cargo.toml
│       │   └── src/
│       │       └── main.rs
│       │
│       └── mev_bot/              # MEV extraction bot
│           ├── Cargo.toml
│           └── src/
│               └── main.rs
│
├── contracts/                     # Smart contracts
│   ├── core/
│   │   ├── FlashLoanReceiver.sol # Base flash loan receiver
│   │   ├── Multicall.sol         # Batch operations
│   │   └── GasOptimized.sol      # Gas optimization helpers
│   │
│   ├── strategies/
│   │   ├── FlashArbitrage.sol    # Flash loan arbitrage
│   │   ├── FlashSwapArbitrage.sol# Flash swap arbitrage
│   │   └── Liquidator.sol        # Liquidation bot
│   │
│   └── interfaces/
│       ├── IDEX.sol              # Common DEX interface
│       ├── IFlashLoanProvider.sol
│       └── IOracle.sol
```

## Key Architectural Components

### 1. Core Trading Library (`trading/core`)

Shared types and utilities used across all trading bots:

```rust
// types.rs
pub struct ArbitrageOpportunity {
    pub id: Uuid,
    pub strategy: StrategyType,
    pub path: TradePath,
    pub estimated_profit: U256,
    pub gas_estimate: U256,
    pub confidence: f64,
    pub expiry: Instant,
    pub metadata: OpportunityMetadata,
}

pub struct ExecutionResult {
    pub opportunity_id: Uuid,
    pub tx_hash: H256,
    pub actual_profit: U256,
    pub gas_used: U256,
    pub slippage: f64,
    pub execution_time_ms: u64,
}

pub trait OpportunityValidator {
    async fn validate(&self, opp: &ArbitrageOpportunity) -> Result<ValidationResult>;
}

pub trait ExecutionEngine {
    async fn execute(&self, opp: &ArbitrageOpportunity) -> Result<ExecutionResult>;
    async fn simulate(&self, opp: &ArbitrageOpportunity) -> Result<SimulationResult>;
}
```

### 2. DEX Abstraction Layer (`trading/dex`)

Unified interface for interacting with different DEX protocols:

```rust
// router.rs
#[async_trait]
pub trait DexRouter {
    async fn get_quote(&self, params: &QuoteParams) -> Result<Quote>;
    async fn swap(&self, params: &SwapParams) -> Result<SwapResult>;
    async fn get_liquidity(&self, pair: &TradingPair) -> Result<Liquidity>;
    fn supports_flash_swap(&self) -> bool;
    fn protocol_name(&self) -> &str;
}

// Factory pattern for router creation
pub struct DexRouterFactory;

impl DexRouterFactory {
    pub fn create(protocol: DexProtocol, config: DexConfig) -> Box<dyn DexRouter> {
        match protocol {
            DexProtocol::UniswapV2 => Box::new(UniswapV2Router::new(config)),
            DexProtocol::UniswapV3 => Box::new(UniswapV3Router::new(config)),
            DexProtocol::Curve => Box::new(CurveRouter::new(config)),
            // ...
        }
    }
}
```

### 3. Strategy Engine (`trading/strategies`)

Modular strategy implementations:

```rust
// arbitrage/two_hop.rs
pub struct TwoHopArbitrage {
    validator: Arc<dyn OpportunityValidator>,
    router_a: Arc<dyn DexRouter>,
    router_b: Arc<dyn DexRouter>,
}

impl TwoHopArbitrage {
    pub async fn find_opportunities(&self, pair: &TradingPair) -> Vec<ArbitrageOpportunity> {
        // Get quotes from both DEXs
        let quote_a = self.router_a.get_quote(&pair.quote_params()).await?;
        let quote_b = self.router_b.get_quote(&pair.reverse().quote_params()).await?;
        
        // Calculate potential profit
        if let Some(profit) = self.calculate_profit(&quote_a, &quote_b) {
            // Validate opportunity
            if self.validator.validate(&opportunity).await?.is_profitable {
                opportunities.push(opportunity);
            }
        }
        
        opportunities
    }
}
```

### 4. Execution Engines (`trading/execution`)

Different execution methods with common interface:

```rust
// flash_loan.rs
pub struct FlashLoanExecutor {
    provider: Arc<Provider<Http>>,
    contract: FlashArbitrage,
    gas_oracle: Arc<GasOracle>,
}

#[async_trait]
impl ExecutionEngine for FlashLoanExecutor {
    async fn execute(&self, opp: &ArbitrageOpportunity) -> Result<ExecutionResult> {
        // Build flash loan transaction
        let tx = self.build_flash_loan_tx(opp)?;
        
        // Execute with optimal gas price
        let gas_price = self.gas_oracle.get_optimal_price().await?;
        let receipt = self.send_transaction(tx, gas_price).await?;
        
        // Parse results
        self.parse_execution_result(receipt)
    }
}

// capital.rs
pub struct CapitalExecutor {
    wallet: Arc<Wallet>,
    routers: HashMap<String, Arc<dyn DexRouter>>,
    risk_manager: Arc<RiskManager>,
}

#[async_trait]
impl ExecutionEngine for CapitalExecutor {
    async fn execute(&self, opp: &ArbitrageOpportunity) -> Result<ExecutionResult> {
        // Check risk limits
        self.risk_manager.check_limits(opp)?;
        
        // Execute trades sequentially
        let result = self.execute_sequential_swaps(opp).await?;
        
        // Update risk metrics
        self.risk_manager.record_execution(result.clone());
        
        result
    }
}
```

### 5. Validation Framework (`trading/validation`)

Comprehensive validation before execution:

```rust
// profitability.rs
pub struct ProfitabilityValidator {
    min_profit_usd: f64,
    min_profit_percent: f64,
    gas_price_oracle: Arc<GasOracle>,
    price_oracle: Arc<PriceOracle>,
}

impl ProfitabilityValidator {
    pub async fn validate(&self, opp: &ArbitrageOpportunity) -> ValidationResult {
        // Get current gas price
        let gas_price = self.gas_price_oracle.get_current().await?;
        let gas_cost_usd = self.calculate_gas_cost(opp.gas_estimate, gas_price);
        
        // Calculate net profit
        let gross_profit_usd = self.to_usd(opp.estimated_profit).await?;
        let net_profit_usd = gross_profit_usd - gas_cost_usd;
        
        // Check thresholds
        let is_profitable = net_profit_usd >= self.min_profit_usd &&
                           (net_profit_usd / opp.trade_size_usd) >= self.min_profit_percent;
        
        ValidationResult {
            is_profitable,
            net_profit_usd,
            gas_cost_usd,
            confidence: self.calculate_confidence(opp),
        }
    }
}
```

### 6. Bot Implementations (`trading/bots/*`)

Thin wrappers that compose the modular components:

```rust
// bots/flash_arb_bot/src/main.rs
#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_env()?;
    
    // Initialize components
    let dex_factory = DexRouterFactory::new();
    let validator = Arc::new(ProfitabilityValidator::new(config.validation));
    let executor = Arc::new(FlashLoanExecutor::new(config.execution).await?);
    
    // Create strategy
    let strategy = TwoHopArbitrage::new(
        validator,
        dex_factory.create(DexProtocol::UniswapV2, config.dex_a),
        dex_factory.create(DexProtocol::Sushiswap, config.dex_b),
    );
    
    // Create bot
    let bot = ArbitrageBot::builder()
        .strategy(strategy)
        .executor(executor)
        .metrics(MetricsCollector::new())
        .build();
    
    // Run
    bot.run().await
}
```

## Configuration Management

Unified configuration with environment-specific overrides:

```yaml
# config/base.yaml
trading:
  min_profit_usd: 10.0
  min_profit_percent: 0.002
  max_gas_price_gwei: 100
  max_slippage_percent: 0.5

dex:
  uniswap_v2:
    router: "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D"
    factory: "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f"
  sushiswap:
    router: "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F"

execution:
  simulation_mode: true
  max_trade_size_usd: 10000
  wallet_address: "${WALLET_ADDRESS}"

# config/production.yaml
trading:
  min_profit_usd: 50.0
  
execution:
  simulation_mode: false
  max_trade_size_usd: 100000
```

## Testing Strategy

### 1. Unit Tests
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_profit_calculation() {
        let calculator = ProfitCalculator::new();
        let profit = calculator.calculate(/* params */);
        assert_eq!(profit, expected);
    }
}
```

### 2. Integration Tests
```rust
#[tokio::test]
async fn test_dex_router_integration() {
    let router = UniswapV2Router::new(test_config());
    let quote = router.get_quote(&test_params()).await.unwrap();
    assert!(quote.amount_out > 0);
}
```

### 3. Simulation Tests
```rust
#[tokio::test]
async fn test_arbitrage_simulation() {
    let bot = create_test_bot();
    let result = bot.simulate(test_opportunity()).await.unwrap();
    assert!(result.is_profitable);
}
```

## Deployment Strategy

### Development
```bash
# Run with simulation mode
SIMULATION_MODE=true cargo run --bin flash-arb-bot
```

### Staging
```bash
# Run with small amounts on testnet
NETWORK=polygon-mumbai MAX_TRADE_SIZE=100 cargo run --release
```

### Production
```bash
# Full production with monitoring
docker-compose up -d
```

## Monitoring & Observability

### Metrics Collection
```rust
impl MetricsCollector {
    pub fn record_opportunity(&self, opp: &ArbitrageOpportunity) {
        self.opportunities_total.inc();
        self.opportunity_profit.observe(opp.estimated_profit);
    }
    
    pub fn record_execution(&self, result: &ExecutionResult) {
        self.executions_total.inc();
        self.execution_profit.observe(result.actual_profit);
        self.execution_latency.observe(result.execution_time_ms);
    }
}
```

### Logging
```rust
use tracing::{info, warn, error, instrument};

#[instrument(skip(self))]
pub async fn execute(&self, opp: &ArbitrageOpportunity) -> Result<ExecutionResult> {
    info!(opportunity_id = %opp.id, "Starting execution");
    // ...
}
```

### Alerting
```yaml
alerts:
  - name: high_failure_rate
    condition: failure_rate > 0.1
    action: notify_slack
    
  - name: low_profit_margin
    condition: avg_profit_percent < 0.001
    action: notify_email
```

## Security Considerations

1. **Private Key Management**: Use hardware wallets or KMS
2. **Contract Upgradability**: Use proxy patterns for contracts
3. **MEV Protection**: Use flashbots or similar private mempools
4. **Rate Limiting**: Implement circuit breakers
5. **Audit Trail**: Log all transactions and decisions

## Migration Plan

### Phase 1: Core Libraries (Week 1-2)
- [ ] Create `trading/core` with shared types
- [ ] Implement `trading/dex` abstraction layer
- [ ] Write comprehensive tests

### Phase 2: Strategy Implementation (Week 3-4)
- [ ] Port existing strategies to new framework
- [ ] Implement additional strategies
- [ ] Add simulation capabilities

### Phase 3: Bot Migration (Week 5-6)
- [ ] Migrate flash loan bot
- [ ] Migrate capital-based bot
- [ ] Add new MEV bot

### Phase 4: Production Deployment (Week 7-8)
- [ ] Deploy to testnet
- [ ] Run parallel with existing bots
- [ ] Gradual migration of volume
- [ ] Full production deployment

## Benefits of This Architecture

1. **Code Reuse**: Shared components across all bots
2. **Testability**: Each component can be tested in isolation
3. **Flexibility**: Easy to add new strategies or DEXs
4. **Maintainability**: Clear separation of concerns
5. **Performance**: Optimized hot paths, parallel execution
6. **Observability**: Built-in metrics and logging
7. **Safety**: Comprehensive validation before execution
8. **Scalability**: Can run multiple strategies concurrently

## Next Steps

1. **Review and Approve**: Get team consensus on architecture
2. **Create Workspace**: Set up new Cargo workspace structure
3. **Define Interfaces**: Create trait definitions for core components
4. **Start Migration**: Begin with core libraries
5. **Document APIs**: Generate comprehensive documentation
6. **Set Up CI/CD**: Automated testing and deployment

This modular architecture will make your arbitrage system more maintainable, testable, and scalable while reducing code duplication and improving overall quality.
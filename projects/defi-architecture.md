# DeFi Infrastructure Architecture
*Professional, modular DeFi components for the AlphaPulse Trading System*

## Overview

The DeFi infrastructure provides modular components for decentralized finance operations, integrating seamlessly with the broader AlphaPulse trading system. This architecture emphasizes professionalism, reusability, and clear separation of concerns.

## System Context

```
AlphaPulse Trading System
├── backend/
│   ├── protocol/           # Binary message protocol
│   ├── services/           # Core services
│   │   ├── exchange_collector/
│   │   ├── relay_server/
│   │   └── data_writer/
│   │
│   └── defi/              # DeFi Infrastructure (NEW)
│       ├── core/          # Shared DeFi primitives
│       ├── strategies/    # Trading strategies
│       ├── execution/     # Execution engines
│       └── agents/        # Autonomous agents
│
├── contracts/             # Smart contracts
│   └── defi/             # DeFi contracts
│
└── frontend/             # Trading UI

```

## Proposed DeFi Architecture

```
backend/
└── defi/                          # DeFi infrastructure
    ├── Cargo.toml                 # Workspace configuration
    │
    ├── core/                      # Core DeFi primitives
    │   ├── Cargo.toml
    │   └── src/
    │       ├── lib.rs
    │       ├── types.rs           # Opportunity, ExecutionResult, etc.
    │       ├── traits.rs          # Core interfaces
    │       ├── metrics.rs         # Performance metrics
    │       ├── risk.rs            # Risk management
    │       └── gas.rs             # Gas optimization
    │
    ├── protocols/                 # DeFi protocol integrations
    │   ├── Cargo.toml
    │   └── src/
    │       ├── lib.rs
    │       ├── dex/              # DEX integrations
    │       │   ├── mod.rs
    │       │   ├── uniswap_v2.rs
    │       │   ├── uniswap_v3.rs
    │       │   ├── curve.rs
    │       │   └── balancer.rs
    │       ├── lending/          # Lending protocols
    │       │   ├── mod.rs
    │       │   ├── aave.rs
    │       │   ├── compound.rs
    │       │   └── maker.rs
    │       └── aggregators/      # DEX aggregators
    │           ├── mod.rs
    │           ├── oneinch.rs
    │           └── zerox.rs
    │
    ├── strategies/                # Trading strategies
    │   ├── Cargo.toml
    │   └── src/
    │       ├── lib.rs
    │       ├── arbitrage/
    │       │   ├── mod.rs
    │       │   ├── spatial.rs    # Cross-DEX arbitrage
    │       │   ├── triangular.rs # Three-asset cycles
    │       │   └── statistical.rs# Statistical arbitrage
    │       ├── market_making/
    │       │   ├── mod.rs
    │       │   └── grid.rs       # Grid trading
    │       ├── liquidation/
    │       │   ├── mod.rs
    │       │   └── aave.rs       # AAVE liquidations
    │       └── yield/
    │           ├── mod.rs
    │           └── farming.rs    # Yield optimization
    │
    ├── execution/                 # Execution engines
    │   ├── Cargo.toml
    │   └── src/
    │       ├── lib.rs
    │       ├── engines/
    │       │   ├── mod.rs
    │       │   ├── capital.rs    # Own capital execution
    │       │   ├── flash_loan.rs # Flash loan execution
    │       │   └── flash_swap.rs # Flash swap execution
    │       ├── simulation/
    │       │   ├── mod.rs
    │       │   └── tenderly.rs   # Tenderly integration
    │       └── mev/
    │           ├── mod.rs
    │           └── flashbots.rs  # Flashbots integration
    │
    ├── validation/                # Opportunity validation
    │   ├── Cargo.toml
    │   └── src/
    │       ├── lib.rs
    │       ├── profitability.rs
    │       ├── liquidity.rs
    │       ├── slippage.rs
    │       └── gas_estimation.rs
    │
    ├── agents/                    # Autonomous trading agents
    │   ├── arbitrage_agent/      # Arbitrage executor
    │   │   ├── Cargo.toml
    │   │   └── src/
    │   │       └── main.rs
    │   │
    │   ├── liquidation_agent/    # Liquidation monitor
    │   │   ├── Cargo.toml
    │   │   └── src/
    │   │       └── main.rs
    │   │
    │   └── market_maker/         # Market making service
    │       ├── Cargo.toml
    │       └── src/
    │           └── main.rs
    │
    └── analytics/                # DeFi analytics
        ├── Cargo.toml
        └── src/
            ├── lib.rs
            ├── pnl.rs             # P&L tracking
            ├── performance.rs     # Strategy performance
            └── optimization.rs    # Parameter optimization

contracts/
└── defi/                          # Smart contracts
    ├── core/
    │   ├── FlashLoanReceiver.sol # Base flash loan receiver
    │   ├── Multicall.sol         # Batched operations
    │   └── GasOptimized.sol      # Gas optimizations
    │
    ├── strategies/
    │   ├── FlashLoans.sol        # Generalized flash loan handler
    │   ├── Arbitrage.sol         # Arbitrage execution
    │   └── Liquidator.sol        # Liquidation execution
    │
    └── interfaces/
        ├── IDEX.sol              # DEX interface
        ├── ILending.sol          # Lending interface
        └── IFlashLoanProvider.sol
```

## Core Components

### 1. DeFi Core (`defi/core`)

Professional interfaces and shared types:

```rust
// traits.rs - Core interfaces
use async_trait::async_trait;
use ethers::types::{Address, U256};

#[async_trait]
pub trait Strategy {
    type Opportunity;
    type Config;
    
    async fn scan(&self) -> Result<Vec<Self::Opportunity>>;
    async fn validate(&self, opp: &Self::Opportunity) -> Result<bool>;
    async fn estimate_profit(&self, opp: &Self::Opportunity) -> Result<U256>;
}

#[async_trait]
pub trait ExecutionEngine {
    type Input;
    type Output;
    
    async fn execute(&self, input: Self::Input) -> Result<Self::Output>;
    async fn simulate(&self, input: Self::Input) -> Result<SimulationResult>;
    async fn estimate_gas(&self, input: Self::Input) -> Result<U256>;
}

#[async_trait]
pub trait RiskManager {
    async fn check_limits(&self, value: U256) -> Result<bool>;
    async fn calculate_var(&self, position: &Position) -> Result<f64>;
    async fn get_exposure(&self) -> Result<ExposureReport>;
}

// types.rs - Domain types
#[derive(Debug, Clone)]
pub struct Opportunity {
    pub id: Uuid,
    pub strategy_type: StrategyType,
    pub venue: Venue,
    pub estimated_profit: U256,
    pub confidence: f64,
    pub expiry: Instant,
    pub metadata: serde_json::Value,
}

#[derive(Debug)]
pub struct ExecutionResult {
    pub opportunity_id: Uuid,
    pub tx_hash: H256,
    pub actual_profit: I256,
    pub gas_used: U256,
    pub slippage_bps: u16,
    pub execution_time_ms: u64,
}
```

### 2. Protocol Integrations (`defi/protocols`)

Clean abstractions over DeFi protocols:

```rust
// dex/mod.rs
use async_trait::async_trait;

#[async_trait]
pub trait DEX {
    async fn get_quote(&self, params: QuoteParams) -> Result<Quote>;
    async fn swap(&self, params: SwapParams) -> Result<SwapResult>;
    async fn get_reserves(&self, pair: Address) -> Result<Reserves>;
    fn protocol_id(&self) -> ProtocolId;
    fn supports_flash_swap(&self) -> bool;
}

// Factory pattern for DEX creation
pub struct DEXFactory;

impl DEXFactory {
    pub fn create(protocol: Protocol, config: ProtocolConfig) -> Arc<dyn DEX> {
        match protocol {
            Protocol::UniswapV2 => Arc::new(UniswapV2::new(config)),
            Protocol::UniswapV3 => Arc::new(UniswapV3::new(config)),
            Protocol::Curve => Arc::new(CurvePool::new(config)),
            Protocol::Balancer => Arc::new(BalancerVault::new(config)),
        }
    }
}

// lending/aave.rs
pub struct AaveV3 {
    pool: IPool,
    oracle: IPriceOracle,
    provider: Arc<Provider<Http>>,
}

impl AaveV3 {
    pub async fn get_user_data(&self, user: Address) -> Result<UserAccountData> {
        self.pool.get_user_account_data(user).call().await
    }
    
    pub async fn flash_loan(&self, params: FlashLoanParams) -> Result<H256> {
        let tx = self.pool.flash_loan_simple(
            params.receiver,
            params.asset,
            params.amount,
            params.data,
            params.referral_code,
        );
        
        let receipt = tx.send().await?.await?;
        Ok(receipt.transaction_hash)
    }
}
```

### 3. Strategy Implementations (`defi/strategies`)

Modular, testable strategies:

```rust
// arbitrage/spatial.rs
pub struct SpatialArbitrage {
    dex_a: Arc<dyn DEX>,
    dex_b: Arc<dyn DEX>,
    validator: Arc<OpportunityValidator>,
    config: ArbitrageConfig,
}

impl SpatialArbitrage {
    pub async fn find_opportunities(&self, pairs: &[TradingPair]) -> Vec<Opportunity> {
        let mut opportunities = Vec::new();
        
        for pair in pairs {
            // Get quotes from both venues
            let quote_a = self.dex_a.get_quote(pair.as_quote_params()).await?;
            let quote_b = self.dex_b.get_quote(pair.as_quote_params()).await?;
            
            // Calculate arbitrage opportunity
            if let Some(arb) = self.calculate_arbitrage(&quote_a, &quote_b) {
                if self.validator.validate(&arb).await? {
                    opportunities.push(arb.into());
                }
            }
        }
        
        opportunities
    }
    
    fn calculate_arbitrage(&self, quote_a: &Quote, quote_b: &Quote) -> Option<ArbitrageOpp> {
        let spread = quote_b.price.saturating_sub(quote_a.price);
        let spread_bps = (spread * 10000) / quote_a.price;
        
        if spread_bps > self.config.min_spread_bps {
            Some(ArbitrageOpp {
                buy_venue: quote_a.venue.clone(),
                sell_venue: quote_b.venue.clone(),
                spread_bps,
                max_size: self.calculate_max_size(quote_a, quote_b),
            })
        } else {
            None
        }
    }
}
```

### 4. Execution Engines (`defi/execution`)

Professional execution with multiple strategies:

```rust
// engines/flash_loan.rs
pub struct FlashLoanEngine {
    contract: FlashLoans,  // Generalized flash loan contract
    provider: Arc<SignerMiddleware<Provider<Http>, Wallet>>,
    gas_oracle: Arc<GasOracle>,
    nonce_manager: Arc<NonceManager>,
}

impl FlashLoanEngine {
    pub async fn new(config: EngineConfig) -> Result<Self> {
        let provider = Provider::<Http>::try_from(&config.rpc_url)?;
        let wallet = config.private_key.parse::<Wallet>()?.with_chain_id(config.chain_id);
        let client = Arc::new(SignerMiddleware::new(provider, wallet));
        
        let contract = FlashLoans::new(config.contract_address, client.clone());
        
        Ok(Self {
            contract,
            provider: client,
            gas_oracle: Arc::new(GasOracle::new()),
            nonce_manager: Arc::new(NonceManager::new()),
        })
    }
}

#[async_trait]
impl ExecutionEngine for FlashLoanEngine {
    type Input = FlashLoanRequest;
    type Output = ExecutionResult;
    
    async fn execute(&self, request: Self::Input) -> Result<Self::Output> {
        // Get optimal gas price
        let gas_config = self.gas_oracle.get_optimal_config().await?;
        
        // Get nonce
        let nonce = self.nonce_manager.get_next().await?;
        
        // Build transaction
        let tx = self.contract
            .execute_strategy(
                request.strategy_data,
                request.flash_loan_params,
            )
            .nonce(nonce)
            .gas_price(gas_config.price)
            .gas(gas_config.limit);
        
        // Execute with retry logic
        let receipt = self.execute_with_retry(tx).await?;
        
        // Parse results
        self.parse_execution_result(receipt).await
    }
    
    async fn simulate(&self, request: Self::Input) -> Result<SimulationResult> {
        // Use Tenderly or local fork for simulation
        let sim = TenderlySimulator::new();
        sim.simulate_transaction(request).await
    }
}
```

### 5. Trading Agents (`defi/agents/*/`)

Thin orchestration layers (no "bot" terminology):

```rust
// agents/arbitrage_agent/src/main.rs
use alphapulse_defi::{
    core::{Strategy, ExecutionEngine},
    strategies::arbitrage::SpatialArbitrage,
    execution::engines::FlashLoanEngine,
    protocols::DEXFactory,
};

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_env()?;
    
    // Initialize components
    let dex_factory = DEXFactory::new();
    let strategy = SpatialArbitrage::builder()
        .dex_a(dex_factory.create(Protocol::UniswapV2, config.dex_a))
        .dex_b(dex_factory.create(Protocol::Sushiswap, config.dex_b))
        .validator(Arc::new(OpportunityValidator::new(config.validation)))
        .build();
    
    let executor = FlashLoanEngine::new(config.execution).await?;
    
    // Create agent
    let agent = ArbitrageAgent::builder()
        .strategy(strategy)
        .executor(executor)
        .relay_connection(config.relay_socket)
        .metrics(MetricsCollector::new())
        .build();
    
    // Run agent
    info!("Starting Arbitrage Agent");
    agent.run().await
}

struct ArbitrageAgent<S: Strategy, E: ExecutionEngine> {
    strategy: S,
    executor: E,
    relay: RelayConnection,
    metrics: MetricsCollector,
}

impl<S: Strategy, E: ExecutionEngine> ArbitrageAgent<S, E> {
    async fn run(self) -> Result<()> {
        // Connect to relay for real-time opportunities
        let mut opportunity_stream = self.relay.subscribe_opportunities().await?;
        
        while let Some(opp) = opportunity_stream.next().await {
            // Validate opportunity
            if self.strategy.validate(&opp).await? {
                // Execute via chosen engine
                match self.executor.execute(opp.into()).await {
                    Ok(result) => {
                        info!("Execution successful: {:?}", result);
                        self.metrics.record_success(result);
                    }
                    Err(e) => {
                        warn!("Execution failed: {}", e);
                        self.metrics.record_failure(e);
                    }
                }
            }
        }
        
        Ok(())
    }
}
```

### 6. Smart Contracts (`contracts/defi/`)

Generalized, reusable contracts:

```solidity
// contracts/defi/strategies/FlashLoans.sol
pragma solidity ^0.8.19;

import "./interfaces/IFlashLoanReceiver.sol";
import "./interfaces/IStrategy.sol";

/**
 * @title FlashLoans
 * @notice Generalized flash loan handler for multiple strategies
 * @dev Supports Aave, Uniswap V3, and Balancer flash loans
 */
contract FlashLoans is IFlashLoanReceiver {
    mapping(bytes32 => IStrategy) public strategies;
    address public immutable owner;
    
    modifier onlyOwner() {
        require(msg.sender == owner, "Unauthorized");
        _;
    }
    
    constructor() {
        owner = msg.sender;
    }
    
    /**
     * @notice Execute strategy with flash loan
     * @param strategyId Strategy identifier
     * @param params Encoded strategy parameters
     */
    function executeStrategy(
        bytes32 strategyId,
        bytes calldata params
    ) external onlyOwner {
        IStrategy strategy = strategies[strategyId];
        require(address(strategy) != address(0), "Unknown strategy");
        
        // Decode flash loan parameters from strategy
        FlashLoanParams memory loanParams = strategy.getFlashLoanParams(params);
        
        // Initiate flash loan
        _initiateFlashLoan(loanParams);
    }
    
    /**
     * @notice Callback from flash loan provider
     */
    function onFlashLoan(
        address initiator,
        address token,
        uint256 amount,
        uint256 fee,
        bytes calldata data
    ) external override returns (bytes32) {
        // Decode and execute strategy
        (bytes32 strategyId, bytes memory strategyData) = abi.decode(data, (bytes32, bytes));
        
        IStrategy strategy = strategies[strategyId];
        uint256 profit = strategy.execute(token, amount, strategyData);
        
        // Ensure profitable after fees
        require(profit > fee, "Unprofitable");
        
        // Repay flash loan
        IERC20(token).approve(msg.sender, amount + fee);
        
        return keccak256("ERC3156FlashBorrower.onFlashLoan");
    }
    
    /**
     * @notice Register new strategy
     */
    function registerStrategy(
        bytes32 strategyId,
        address strategyContract
    ) external onlyOwner {
        strategies[strategyId] = IStrategy(strategyContract);
    }
}
```

## Integration with AlphaPulse System

### 1. Message Protocol Integration

```rust
// Extend existing protocol for DeFi messages
impl From<Opportunity> for ArbitrageOpportunityMessage {
    fn from(opp: Opportunity) -> Self {
        ArbitrageOpportunityMessage {
            timestamp_ns: opp.created_at.as_nanos() as u64,
            pair: opp.pair_name(),
            // ... mapping
        }
    }
}
```

### 2. Relay Server Integration

```rust
// DeFi agents subscribe to relay for opportunities
let relay_socket = "/tmp/alphapulse/relay.sock";
let mut stream = RelayClient::connect(relay_socket).await?;
stream.subscribe(MessageType::ArbitrageOpportunity).await?;
```

### 3. Data Writer Integration

```rust
// Store DeFi execution results
let execution_record = ExecutionRecord {
    timestamp: result.timestamp,
    strategy: "spatial_arbitrage",
    venue_a: result.venue_a,
    venue_b: result.venue_b,
    profit_usd: result.profit_usd,
    gas_cost_usd: result.gas_cost_usd,
};

data_writer.write_execution(execution_record).await?;
```

## Configuration

```yaml
# config/defi.yaml
defi:
  strategies:
    arbitrage:
      min_profit_usd: 50.0
      min_spread_bps: 10
      max_position_usd: 100000
      
  execution:
    flash_loan:
      contract_address: "0x..."
      max_gas_price_gwei: 100
      
    capital:
      max_allocation_pct: 0.5
      
  risk:
    max_daily_loss_usd: 10000
    max_position_count: 10
    var_confidence: 0.95
    
  protocols:
    uniswap_v2:
      router: "0x7a250d5630B4cF..."
      factory: "0x5C69bEe701ef814..."
      
    aave_v3:
      pool: "0x794a61358D6845594..."
      oracle: "0xb023e699F5a33916..."
```

## Benefits of This Architecture

1. **Professional Terminology**: No "bot" references, using industry-standard terms
2. **Modular Design**: Each component has a single responsibility
3. **Integration Ready**: Seamlessly integrates with existing AlphaPulse infrastructure
4. **Protocol Agnostic**: Easy to add new DeFi protocols
5. **Strategy Flexibility**: Simple to implement new trading strategies
6. **Testable**: Each component can be unit tested in isolation
7. **Observable**: Built-in metrics and monitoring
8. **Scalable**: Can run multiple strategies and agents concurrently

## Migration Plan

### Phase 1: Core Infrastructure (Week 1)
- [ ] Set up `defi/` workspace
- [ ] Implement core traits and types
- [ ] Create protocol abstractions

### Phase 2: Strategy Migration (Week 2)
- [ ] Port existing arbitrage logic to new framework
- [ ] Generalize flash loan handling
- [ ] Add validation framework

### Phase 3: Execution Engines (Week 3)
- [ ] Implement flash loan engine
- [ ] Add capital-based execution
- [ ] Integrate simulation

### Phase 4: Agent Deployment (Week 4)
- [ ] Deploy arbitrage agent
- [ ] Add liquidation monitoring
- [ ] Production testing

This architecture positions the DeFi components as a professional, integral part of the larger AlphaPulse trading system, with clear interfaces for expansion into other DeFi protocols and strategies.
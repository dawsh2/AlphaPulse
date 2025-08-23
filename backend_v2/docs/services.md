# AlphaPulse System Architecture

## Executive Summary

A high-performance arbitrage trading system built on the AlphaPulse message protocol, designed for multi-venue, multi-chain DeFi and traditional market opportunities. The architecture emphasizes deterministic data flow, atomic execution, and comprehensive observability while maintaining microsecond-level latency for critical trading paths.

## Core Design Principles

1. **Event-Driven Architecture**: All system state derived from immutable event streams
2. **Domain Isolation**: Clean separation between market data, signal generation, and execution
3. **Atomic Execution**: Flash loan-based arbitrage with all-or-nothing transaction semantics
4. **Zero-Capital Trading**: No inventory risk through capital-efficient flash loan strategies
5. **Universal Venue Support**: Protocol-agnostic design supporting CEX, DEX, and traditional markets

---

# Part I: System Overview

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           PRESENTATION LAYER                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
│  │ Dashboard   │  │ Risk        │  │ Analytics   │  │ Strategy    │        │
│  │ UI          │  │ Monitor     │  │ Engine      │  │ Manager     │        │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘        │
│         │                 │                 │                 │            │
└─────────┼─────────────────┼─────────────────┼─────────────────┼────────────┘
          │                 │                 │                 │
┌─────────┼─────────────────┼─────────────────┼─────────────────┼────────────┐
│         │        ALPHAPULSE MESSAGE PROTOCOL LAYER           │            │
├─────────┼─────────────────┼─────────────────┼─────────────────┼────────────┤
│         │                 │                 │                 │            │
│  ┌──────▼──────┐   ┌──────▼──────┐   ┌──────▼──────┐          │            │
│  │MarketData   │   │SignalRelay  │   │Execution    │          │            │
│  │Relay        │   │             │   │Relay        │          │            │
│  │(Types 1-19) │   │(Types 20-39)│   │(Types 40-59)│          │            │
│  └──────┬──────┘   └──────┬──────┘   └──────┬──────┘          │            │
└─────────┼─────────────────┼─────────────────┼─────────────────┼────────────┘
          │                 │                 │                 │
┌─────────┼─────────────────┼─────────────────┼─────────────────┼────────────┐
│         │           PROCESSING LAYER        │                 │            │
├─────────┼─────────────────┼─────────────────┼─────────────────┼────────────┤
│         ▼                 ▼                 ▼                 ▼            │
│  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐    │
│  │Market Data  │   │Arbitrage    │   │Execution    │   │Portfolio    │    │
│  │Collectors   │   │Strategy     │   │Engine       │   │Manager      │    │
│  │             │   │Engine       │   │             │   │             │    │
│  └─────────────┘   └─────────────┘   └─────────────┘   └─────────────┘    │
│         │                 │                 │                 │            │
└─────────┼─────────────────┼─────────────────┼─────────────────┼────────────┘
          │                 │                 │                 │
┌─────────┼─────────────────┼─────────────────┼─────────────────┼────────────┐
│         │          INTEGRATION LAYER       │                 │            │
├─────────┼─────────────────┼─────────────────┼─────────────────┼────────────┤
│         ▼                 │                 ▼                 │            │
│  ┌─────────────┐          │          ┌─────────────┐          │            │
│  │Venue        │          │          │Smart        │          │            │
│  │Connectors   │          │          │Contracts    │          │            │
│  │             │          │          │             │          │            │
│  │• Polygon    │          │          │• Flash Loan │          │            │
│  │• Ethereum   │          │          │• Arbitrage  │          │            │
│  │• Binance    │          │          │• MEV Bundle │          │            │
│  │• Coinbase   │          │          │             │          │            │
│  └─────────────┘          │          └─────────────┘          │            │
└────────────────────────────┼──────────────────────────────────┼────────────┘
                             │                                  │
                             ▼                                  ▼
                    ┌─────────────┐                    ┌─────────────┐
                    │Event        │                    │State        │
                    │Archive      │                    │Store        │
                    │             │                    │             │
                    │• Parquet    │                    │• Positions  │
                    │• Time Series│                    │• Orders     │
                    │• Recovery   │                    │• P&L        │
                    └─────────────┘                    └─────────────┘
```

## Core Components

### Market Data Layer
- **Venue Connectors**: Protocol-specific data collection from exchanges and DEXs
- **Market Data Relay**: Central hub for all market events (trades, quotes, liquidity changes)
- **Event Normalization**: Convert venue-specific data to universal TLV format

### Signal Generation Layer
- **Arbitrage Strategy Engine**: Real-time opportunity detection and self-contained execution
- **Signal Relay**: Route coordination signals for risk-managed strategies
- **Opportunity Scoring**: Economic viability analysis including gas costs and slippage

### Execution Layer
- **Execution Coordinator**: Coordinate multi-step execution for risk-managed strategies
- **Execution Modules**: Importable transaction building and blockchain interaction components
- **Smart Contracts**: Huff-optimized flash loan arbitrage with atomic execution
- **Execution Relay**: Track coordinated execution lifecycle and results

### State Management
- **In-Memory State**: Hot trading state for sub-millisecond decision making
- **State Store**: Persistent positions, orders, and portfolio state
- **Event Archive**: Complete system audit trail for replay and analysis

---

# Part II: Domain-Specific Architectures

## Market Data Collection Architecture

### Multi-Venue Data Ingestion

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        VENUE CONNECTOR LAYER                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐             │
│ │Polygon      │ │Ethereum     │ │Binance      │ │Coinbase     │             │
│ │Connector    │ │Connector    │ │Connector    │ │Connector    │             │
│ │             │ │             │ │             │ │             │             │
│ │• WebSocket  │ │• WebSocket  │ │• WebSocket  │ │• WebSocket  │             │
│ │• UniV2 Logs │ │• UniV2 Logs │ │• Order Book │ │• Order Book │             │
│ │• UniV3 Logs │ │• UniV3 Logs │ │• Trades     │ │• Trades     │             │
│ │• Mempool    │ │• Mempool    │ │• Tickers    │ │• Tickers    │             │
│ └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘             │
│        │               │               │               │                   │
└────────┼───────────────┼───────────────┼───────────────┼───────────────────┘
         │               │               │               │
         ▼               ▼               ▼               ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                     EVENT NORMALIZATION LAYER                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                   Universal Event Transformer                       │    │
│  │                                                                     │    │
│  │  Venue Events → InstrumentId → TLV Messages                        │    │
│  │                                                                     │    │
│  │  • Polygon Swap      → TradeTLV (Type 1)                          │    │
│  │  • Polygon Mint/Burn → LiquidityTLV (Type 5)                      │    │
│  │  • Binance Trade     → TradeTLV (Type 1)                          │    │
│  │  • Coinbase L2       → QuoteTLV (Type 2)                          │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                   │                                         │
└───────────────────────────────────┼─────────────────────────────────────────┘
                                    ▼
                            ┌───────────────┐
                            │ MarketData    │
                            │ Relay         │
                            │ (Domain 1)    │
                            └───────────────┘
```

### Event Processing Semantics

**State Invalidation Protocol:**
```rust
enum ConnectionEvent {
    Connected(VenueId),
    Disconnected(VenueId),
    Reconnecting(VenueId),
}

// On disconnection: immediate state wipe
match event {
    ConnectionEvent::Disconnected(venue_id) => {
        // Send invalidation for all instruments from this venue
        for instrument in instruments_by_venue(venue_id) {
            send_tlv(StateInvalidationTLV {
                instrument_id: instrument,
                action: StateAction::Reset,
            });
        }
        // Remove from active venue set
        active_venues.remove(venue_id);
    }
}
```

**Bijective Instrument ID Generation:**
```rust
// Each venue connector creates deterministic IDs
impl PolygonConnector {
    fn create_pool_id(&self, token0: Address, token1: Address) -> InstrumentId {
        let usdc_id = InstrumentId::ethereum_token(token0);
        let weth_id = InstrumentId::ethereum_token(token1);
        InstrumentId::pool(VenueId::UniswapV3, usdc_id, weth_id)
    }
}

// IDs are venue-independent for cross-venue arbitrage
assert_eq!(
    polygon_usdc_weth_pool.base_tokens(),
    ethereum_usdc_weth_pool.base_tokens()
); // Same token pair = same base IDs
```

## Arbitrage Strategy Architecture (Self-Contained)

### Real-Time Opportunity Detection and Direct Execution

**Key Principle**: Flash loan arbitrage strategies are **self-contained** - they detect opportunities AND execute them atomically without external coordination through execution engines.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                      ARBITRAGE STRATEGY ENGINE                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                    Event Processing Pipeline                        │    │
│  │                                                                     │    │
│  │  Market Event → Pool State Update → Cross-Pool Analysis → Signal   │    │
│  │                                                                     │    │
│  │  • Every swap updates affected pools                               │    │
│  │  • Every liquidity change triggers recomputation                   │    │
│  │  • Calculate exact AMM curves for all pool pairs                   │    │
│  │  • Generate signal only if profit > gas + fees + slippage          │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                      Pool State Manager                             │    │
│  │                                                                     │    │
│  │  pools: HashMap<InstrumentId, PoolState>                           │    │
│  │                                                                     │    │
│  │  struct PoolState {                                                 │    │
│  │      reserves: (u128, u128),                                       │    │
│  │      fee_tier: u32,         // 500, 3000, 10000 bps               │    │
│  │      last_update_block: u64,                                       │    │
│  │      tick_current: i32,     // UniV3 only                         │    │
│  │      liquidity: u128,       // UniV3 only                         │    │
│  │  }                                                                  │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                     Profitability Calculator                       │    │
│  │                                                                     │    │
│  │  • Exact Uniswap V2/V3 math using verified crates                 │    │
│  │  • Gas estimation from Huff contract bytecode                     │    │
│  │  • Real-time gas price feeds (base fee + priority fee)            │    │
│  │  • Slippage calculation for arbitrary trade sizes                 │    │
│  │  • Multi-hop arbitrage path optimization                          │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                   │                                         │
└───────────────────────────────────┼─────────────────────────────────────────┘
                                    ▼
                            ┌───────────────┐
                            │ Signal        │
                            │ Relay         │
                            │ (Domain 2)    │
                            └───────────────┘
```

### Self-Contained Execution Flow

**Cross-Pool Arbitrage Detection and Direct Execution:**
```rust
impl ArbitrageStrategy {
    fn process_swap_event(&mut self, trade: &TradeTLV) {
        // Update pool state
        let pool_id = trade.instrument_id;
        self.update_pool_state(pool_id, trade);
        
        // Check arbitrage opportunities against all other pools
        for (other_pool_id, other_pool) in &self.pools {
            if pools_can_arbitrage(pool_id, *other_pool_id) {
                if let Some(opportunity) = self.calculate_arbitrage(
                    pool_id, *other_pool_id
                ) {
                    // Execute directly - no external coordination needed
                    self.execute_arbitrage_atomically(opportunity).await;
                }
            }
        }
    }
    
    fn calculate_arbitrage(&self, pool_a: InstrumentId, pool_b: InstrumentId) 
        -> Option<ArbitrageOpportunity> {
        
        let state_a = &self.pools[&pool_a];
        let state_b = &self.pools[&pool_b];
        
        // Find optimal trade size that maximizes profit
        let optimal_size = self.optimize_trade_size(state_a, state_b)?;
        
        // Calculate exact execution path
        let path = TradePath::new(pool_a, pool_b, optimal_size);
        let gross_profit = path.calculate_gross_profit();
        let total_costs = path.gas_cost + path.pool_fees + path.slippage_cost;
        
        if gross_profit > total_costs {
            Some(ArbitrageOpportunity {
                path,
                net_profit: gross_profit - total_costs,
                confidence: self.calculate_confidence(&path),
            })
        } else {
            None
        }
    }
}
```

**Direct Execution Implementation:**
```rust
// Flash loan arbitrage executes atomically within the strategy
async fn execute_arbitrage_atomically(&mut self, opportunity: ArbitrageOpportunity) -> Result<()> {
    // Build transaction directly
    let tx = self.build_flash_loan_transaction(&opportunity)?;
    
    // Submit to blockchain directly
    let tx_hash = match self.submission_strategy {
        SubmissionStrategy::PublicMempool => {
            self.rpc_client.send_transaction(tx).await?
        },
        SubmissionStrategy::Flashbots => {
            self.flashbots_client.send_bundle(tx).await?
        },
    };
    
    // Monitor execution and report results
    let result = self.monitor_transaction(tx_hash).await?;
    self.report_execution_result(result).await;
    
    Ok(())
}

fn build_flash_loan_transaction(&self, opportunity: &ArbitrageOpportunity) -> Result<Transaction> {
    // Build atomic flash loan transaction
    let call_data = encode_call("flashloanArbitrage", &[
        opportunity.path.pool_a.address,
        opportunity.path.pool_b.address,
        opportunity.path.trade_size,
        opportunity.net_profit * 95 / 100, // 5% slippage tolerance
    ]);
    
    Transaction {
        to: self.flash_arbitrage_contract,
        data: call_data,
        gas_limit: opportunity.path.gas_cost * 120 / 100, // 20% buffer
        gas_price: self.current_gas_price(),
    }
}
```

## Execution Engine Architecture (Risk-Managed Strategies Only)

**Important**: The Execution Engine is NOT used for flash loan arbitrage strategies. Flash loan strategies are self-contained and execute directly. The Execution Engine is only for risk-managed strategies that require position management, capital allocation, and external coordination.

### Smart Contract Integration for Risk-Managed Strategies

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        EXECUTION COORDINATION                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                    Execution Coordinator                            │    │
│  │                                                                     │    │
│  │  • Receives signals from Signal Relay for coordinated execution    │    │
│  │  • Validates timing and risk parameters                            │    │
│  │  • Coordinates multi-step execution sequences                      │    │
│  │  • Manages position lifecycle for risk-managed strategies          │    │
│  │  • Provides execution modules for direct import                    │    │
│  │  • Reports coordinated execution results                           │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                   │                                         │
│                                   ▼                                         │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                 Execution Modules                                   │    │
│  │                                                                     │    │
│  │  // Importable modules for direct execution                        │    │
│  │  pub mod gas_estimator { ... }                                     │    │
│  │  pub mod contract_interfaces { ... }                               │    │
│  │  pub mod transaction_builder {                                     │    │
│  │      pub fn build_flash_arbitrage_tx(...) -> TransactionCall       │    │
│  │      pub fn build_position_tx(...) -> TransactionCall              │    │
│  │      pub fn estimate_gas(...) -> GasEstimate                       │    │
│  │  }                                                                  │    │
│  │  pub mod blockchain_client {                                       │    │
│  │      pub async fn submit_transaction(...) -> TxResult              │    │
│  │      pub async fn monitor_transaction(...) -> TxStatus             │    │
│  │  }                                                                  │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         SMART CONTRACT LAYER                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │              Huff Flash Arbitrage Contract                          │    │
│  │                                                                     │    │
│  │  // Single atomic transaction:                                      │    │
│  │  function flashloanArbitrage(                                       │    │
│  │      address[] pools,           // [poolA, poolB, ...]             │    │
│  │      uint256[] amounts,         // Trade amounts for each step      │    │
│  │      uint256 minProfit,         // Minimum profit or revert         │    │
│  │      address profitToken,       // USDC, WETH, etc.                │    │
│  │      address profitRecipient    // Where to send profit             │    │
│  │  ) external {                                                       │    │
│  │      // 1. Flash loan from AAVE                                     │    │
│  │      // 2. Execute swap sequence across pools                       │    │
│  │      // 3. Repay flash loan + 0.09% fee                            │    │
│  │      // 4. Send remaining profit to recipient                       │    │
│  │      // 5. Revert if profit < minProfit                            │    │
│  │  }                                                                  │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                    Execution Guarantees                             │    │
│  │                                                                     │    │
│  │  • Atomic: All swaps succeed or entire transaction reverts         │    │
│  │  • Capital Efficient: No upfront capital required                  │    │
│  │  • Gas Optimized: Huff bytecode for minimal gas usage              │    │
│  │  • MEV Protected: Can submit via Flashbots private mempool         │    │
│  │  • Profit Guaranteed: Contract reverts if profit < threshold       │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼ (execution results)
                            ┌───────────────┐
                            │ Execution     │
                            │ Relay         │
                            │ (Domain 3)    │
                            └───────────────┘
```

### Transaction Lifecycle Management

**Execution Patterns:**

**1. Coordinated Execution (for risk-managed strategies):**
```rust
impl ExecutionCoordinator {
    async fn process_position_signal(&mut self, signal: PositionSignal) -> Result<()> {
        // Coordinated execution for multi-step position management
        let execution_plan = self.build_execution_plan(&signal).await?;
        
        for step in execution_plan.steps {
            let tx = transaction_builder::build_position_tx(&step);
            let result = blockchain_client::submit_transaction(tx).await?;
            self.track_execution_step(signal.signal_id, step.id, result).await;
        }
        
        Ok(())
    }
}
```

**2. Direct Module Usage (for immediate execution strategies):**
```rust
// Example: ArbitrageStrategy directly importing execution modules
use execution::transaction_builder;
use execution::blockchain_client;
use execution::gas_estimator;

impl ArbitrageStrategy {
    async fn execute_opportunity(&self, opportunity: Opportunity) -> Result<()> {
        // Build transaction using execution modules
        let tx = transaction_builder::build_flash_arbitrage_tx(
            &opportunity.pools,
            opportunity.trade_size,
            opportunity.min_profit,
        )?;
        
        // Estimate and adjust gas
        let gas_estimate = gas_estimator::estimate_gas(&tx).await?;
        let tx = tx.with_gas_limit(gas_estimate.limit * 120 / 100); // 20% buffer
        
        // Submit directly to blockchain
        let result = blockchain_client::submit_transaction(tx).await?;
        
        // Report result for analytics
        self.report_execution_result(opportunity.id, result).await;
        
        Ok(())
    }
}
```

## State Management Architecture

### Dual-Path State Design

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            HOT PATH (In-Memory)                            │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                      Trading State Store                            │    │
│  │                                                                     │    │
│  │  • Pool States: HashMap<InstrumentId, PoolState>                   │    │
│  │  • Active Signals: HashMap<SignalId, ArbitrageSignal>              │    │
│  │  • Pending Orders: HashMap<TxHash, ExecutionOrder>                 │    │
│  │  • Gas Price Cache: RealTimeGasPrices                             │    │
│  │                                                                     │    │
│  │  Performance: <1ms state lookups, <100μs updates                   │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                   Lock-Free Update Pattern                          │    │
│  │                                                                     │    │
│  │  • Single writer per pool (market data collector)                  │    │
│  │  • Multiple readers (strategy engines, dashboard)                  │    │
│  │  • Atomic pointer swaps for state updates                          │    │
│  │  • No locks in critical trading path                               │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼ (async write-behind)
┌─────────────────────────────────────────────────────────────────────────────┐
│                           COLD PATH (Persistent)                           │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                      Event Archive                                  │    │
│  │                                                                     │    │
│  │  Storage Format: Parquet files with TLV message streams            │    │
│  │  Partitioning: By date and relay domain                           │    │
│  │  Compression: ~5x compression ratio on binary TLV data            │    │
│  │  Retention: Infinite retention for replay and compliance          │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                   Portfolio State Store                             │    │
│  │                                                                     │    │
│  │  • Current Positions: TokenBalances by chain                       │    │
│  │  • Order History: Complete execution records                       │    │
│  │  • P&L Tracking: Realized and unrealized gains                    │    │
│  │  • Risk Metrics: Exposure, volatility, drawdown                   │    │
│  │                                                                     │    │
│  │  Update Pattern: Eventual consistency from execution events        │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
```

### State Synchronization Protocol

**Critical State Updates:**
```rust
enum StateUpdate {
    PoolStateChange {
        instrument_id: InstrumentId,
        new_reserves: (u128, u128),
        block_number: u64,
    },
    SignalGenerated {
        signal: ArbitrageSignal,
        confidence: u8,
    },
    ExecutionStarted {
        signal_id: u64,
        tx_hash: H256,
        gas_price: u64,
    },
    ExecutionCompleted {
        signal_id: u64,
        result: ExecutionResult,
        actual_profit: i128,
    },
}

// State updates flow through TLV messages for consistency
impl StateManager {
    fn apply_update(&mut self, update: StateUpdate) {
        match update {
            StateUpdate::PoolStateChange { instrument_id, new_reserves, block_number } => {
                // Update hot state
                self.hot_state.pools.insert(instrument_id, PoolState {
                    reserves: new_reserves,
                    last_update_block: block_number,
                    status: PoolStatus::Fresh,
                });
                
                // Queue for cold storage
                self.archive_queue.push(TradeTLV::from_pool_update(update));
            }
            StateUpdate::ExecutionCompleted { signal_id, result, actual_profit } => {
                // Update portfolio state
                self.portfolio.record_trade_result(signal_id, actual_profit);
                
                // Archive execution result
                self.archive_queue.push(ExecutionResultTLV::from_result(result));
            }
            // ... other update types
        }
    }
}
```

## Cross-Chain Architecture

### Multi-Chain State Coordination

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         CHAIN-SPECIFIC COLLECTORS                          │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐             │
│ │Polygon      │ │Ethereum     │ │Arbitrum     │ │Base         │             │
│ │Collector    │ │Collector    │ │Collector    │ │Collector    │             │
│ │             │ │             │ │             │ │             │             │
│ │Chain ID: 137│ │Chain ID: 1  │ │Chain ID: 42161 │Chain ID: 8453  │             │
│ │Gas Token:   │ │Gas Token:   │ │Gas Token:   │ │Gas Token:   │             │
│ │MATIC        │ │ETH          │ │ETH          │ │ETH          │             │
│ └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘             │
│        │               │               │               │                   │
└────────┼───────────────┼───────────────┼───────────────┼───────────────────┘
         │               │               │               │
         ▼               ▼               ▼               ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                      UNIFIED MARKET DATA RELAY                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  All chains feed into single MarketData Relay with chain-aware routing     │
│                                                                             │
│  Message Routing:                                                           │
│  • TradeTLV.instrument_id.venue → chain identification                     │
│  • Cross-chain arbitrage requires multiple chain states                    │
│  • Bridge cost integration for cross-chain opportunities                   │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                    ARBITRAGE STRATEGY ROUTING                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                Intra-Chain Strategies                               │    │
│  │                                                                     │    │
│  │  • PolygonArbitrageStrategy: Polygon-only opportunities             │    │
│  │  • EthereumArbitrageStrategy: Ethereum-only opportunities           │    │
│  │  • ArbitrumArbitrageStrategy: Arbitrum-only opportunities           │    │
│  │                                                                     │    │
│  │  Advantages: Single gas token, consistent block times               │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │               Cross-Chain Strategy                                  │    │
│  │                                                                     │    │
│  │  • Monitors multiple chains simultaneously                          │    │
│  │  • Calculates bridge costs (LayerZero, native bridges)             │    │
│  │  • Timing coordination between chains                              │    │
│  │  • Higher complexity, higher potential profits                     │    │
│  │                                                                     │    │
│  │  Example: ETH/USDC cheaper on Arbitrum → buy → bridge → sell Ethereum │
│  └─────────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Cross-Chain Signal Coordination

**Cross-Chain Arbitrage Signal:**
```rust
#[repr(C, packed)]
pub struct CrossChainArbitrageSignalTLV {
    pub tlv_type: u8,               // TLVType::CrossChainArbitrage (31)
    pub tlv_length: u8,             // ~120 bytes
    
    // Signal identity
    pub signal_id: u64,
    pub strategy_id: u16,           // CROSS_CHAIN_ARBITRAGE
    pub confidence: u8,
    
    // Multi-chain execution path
    pub source_chain_id: u32,       // Where to start (e.g., Arbitrum)
    pub dest_chain_id: u32,         // Where to end (e.g., Ethereum)
    pub bridge_protocol: u16,       // LayerZero, Polygon Bridge, etc.
    
    // Economic analysis
    pub gross_profit: i128,         // Before bridge and gas costs
    pub bridge_cost: u128,          // Bridge fees
    pub source_gas_cost: u128,      // Gas on source chain
    pub dest_gas_cost: u128,        // Gas on destination chain
    pub timing_risk: u16,           // Blocks until opportunity expires
    
    // Execution addresses per chain
    pub source_pool: Address,       // Pool on source chain
    pub dest_pool: Address,         // Pool on destination chain
    pub bridge_address: Address,    // Bridge contract
    
    pub reserved: [u8; 4],
}
```

---

# Part III: Integration Patterns

## Dashboard Integration Architecture

### Multi-Domain Consumer Pattern

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              DASHBOARD UI                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                     Real-Time Data Views                            │    │
│  │                                                                     │    │
│  │  • Market Data Panel: Live pool states, recent swaps               │    │
│  │  • Signal Panel: Active arbitrage opportunities                    │    │
│  │  • Execution Panel: Transaction status, gas usage                  │    │
│  │  • P&L Panel: Portfolio performance, risk metrics                  │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                   Debugging & Analysis Tools                        │    │
│  │                                                                     │    │
│  │  • Signal Inspector: Complete profitability breakdown              │    │
│  │  • Execution Tracer: Step-by-step transaction analysis             │    │
│  │  • Performance Analytics: Signal accuracy, latency metrics         │    │
│  │  • Error Analysis: Failed transaction investigation                │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
                │               │               │               │
                │ (WebSocket)   │ (WebSocket)   │ (WebSocket)   │
                ▼               ▼               ▼               ▼
┌───────────────────┐ ┌───────────────────┐ ┌───────────────────┐ ┌─────────────┐
│ MarketData        │ │ Signal            │ │ Execution         │ │ Portfolio   │
│ Relay             │ │ Relay             │ │ Relay             │ │ Manager     │
│                   │ │                   │ │                   │ │             │
│ • Pool updates    │ │ • Arbitrage       │ │ • Order status    │ │ • Positions │
│ • Trade events    │ │   signals         │ │ • Execution       │ │ • P&L       │
│ • Quote updates   │ │ • Confidence      │ │   results         │ │ • Risk      │
│                   │ │   scores          │ │ • Gas metrics     │ │   metrics   │
└───────────────────┘ └───────────────────┘ └───────────────────┘ └─────────────┘
```

### Dashboard State Management

**Real-Time State Synchronization:**
```rust
impl Dashboard {
    async fn run(&mut self) -> Result<()> {
        loop {
            tokio::select! {
                // Market data updates pool visualization
                msg = self.market_data_stream.next() => {
                    match parse_message(msg?) {
                        TLVType::Trade => {
                            self.update_pool_chart(trade);
                            self.check_price_alerts(trade);
                        }
                        TLVType::Quote => {
                            self.update_order_book_display(quote);
                        }
                        _ => {}
                    }
                }
                
                // Signal updates show opportunities
                msg = self.signal_stream.next() => {
                    match parse_message(msg?) {
                        TLVType::ArbitrageSignal => {
                            self.display_arbitrage_opportunity(signal);
                            self.update_profitability_metrics(signal);
                        }
                        _ => {}
                    }
                }
                
                // Execution updates show trade results
                msg = self.execution_stream.next() => {
                    match parse_message(msg?) {
                        TLVType::ExecutionResult => {
                            self.update_execution_status(result);
                            self.update_performance_charts(result);
                        }
                        _ => {}
                    }
                }
                
                // User interactions trigger commands
                user_action = self.ui_events.next() => {
                    match user_action {
                        UIEvent::ExecuteSignal(signal_id) => {
                            self.send_execution_command(signal_id).await?;
                        }
                        UIEvent::CancelOrder(order_id) => {
                            self.send_cancel_command(order_id).await?;
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
```

## Risk Management Integration

### Real-Time Risk Monitoring

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            RISK MANAGEMENT LAYER                           │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                      Risk Monitor Service                           │    │
│  │                                                                     │    │
│  │  Subscribes to ALL relay domains for comprehensive risk view:       │    │
│  │                                                                     │    │
│  │  • Market Data: Unusual price movements, liquidity drops           │    │
│  │  • Signals: Signal frequency, profit distribution                  │    │
│  │  • Execution: Gas usage spikes, execution failures                 │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                      Circuit Breakers                              │    │
│  │                                                                     │    │
│  │  • Max Daily Loss: Stop trading if losses exceed threshold         │    │
│  │  • Max Trade Size: Reject signals above size limit                 │    │
│  │  • Gas Price Ceiling: Pause execution if gas > threshold           │    │
│  │  • Execution Failure Rate: Stop if failure rate > 10%             │    │
│  │  • Market Volatility: Pause during extreme market conditions       │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                    Risk Metrics Calculation                         │    │
│  │                                                                     │    │
│  │  • Real-time P&L tracking across all trades                        │    │
│  │  • Value at Risk (VaR) based on position sizes                     │    │
│  │  • Maximum drawdown monitoring                                     │    │
│  │  • Sharpe ratio calculation from trade history                     │    │
│  │  • Correlation analysis between different strategies               │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Risk Event Processing:**
```rust
impl RiskMonitor {
    fn process_execution_result(&mut self, result: &ExecutionResultTLV) {
        // Update real-time P&L
        self.current_pnl += result.actual_profit;
        self.daily_pnl += result.actual_profit;
        
        // Check circuit breakers
        if self.daily_pnl < self.max_daily_loss {
            self.trigger_circuit_breaker(CircuitBreakerType::DailyLoss);
        }
        
        // Update risk metrics
        self.trade_history.push(TradeResult {
            profit: result.actual_profit,
            timestamp: result.timestamp,
            gas_used: result.gas_used,
        });
        
        self.recalculate_risk_metrics();
    }
    
    fn trigger_circuit_breaker(&mut self, breaker_type: CircuitBreakerType) {
        // Send emergency stop signal to all execution engines
        let stop_signal = TLVMessageBuilder::new(EXECUTION_DOMAIN, RISK_MONITOR_ID)
            .add_tlv(TLVType::EmergencyStop, &EmergencyStopTLV {
                reason: breaker_type as u8,
                timestamp: current_timestamp(),
                affected_strategies: ALL_STRATEGIES,
            })
            .build();
            
        self.execution_relay.broadcast(stop_signal);
        
        // Alert operators
        self.alert_system.send_critical_alert(format!(
            "Circuit breaker triggered: {:?}", breaker_type
        ));
    }
}
```

## Performance and Monitoring

### System Observability

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            METRICS COLLECTION                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                    Performance Metrics                              │    │
│  │                                                                     │    │
│  │  • Message Throughput: Messages/second by relay domain             │    │
│  │  • Processing Latency: Time from market event to signal            │    │
│  │  • Execution Latency: Time from signal to transaction              │    │
│  │  • Memory Usage: Heap usage by service component                   │    │
│  │  • Connection Health: WebSocket uptime by venue                    │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                     Trading Metrics                                │    │
│  │                                                                     │    │
│  │  • Signal Generation Rate: Opportunities/minute by strategy        │    │
│  │  • Signal Accuracy: Predicted vs actual profit                     │    │
│  │  • Execution Success Rate: Successful transactions/total           │    │
│  │  • Gas Efficiency: Actual vs estimated gas usage                   │    │
│  │  • Profit Distribution: P&L histogram by trade size                │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                      Alert Conditions                              │    │
│  │                                                                     │    │
│  │  • WebSocket disconnections > 5/hour                               │    │
│  │  • Signal generation drops < 10/hour                               │    │
│  │  • Execution failure rate > 10%                                    │    │
│  │  • Gas price > 100 gwei sustained                                  │    │
│  │  • Daily P&L < -$1000                                              │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Distributed Tracing

**End-to-End Request Tracing:**
```rust
// Every TLV message carries trace context
#[repr(C, packed)]
pub struct TraceContextTLV {
    pub tlv_type: u8,               // TLVType::TraceContext (120)
    pub tlv_length: u8,             // 20
    pub trace_id: u128,             // Distributed trace ID
    pub span_id: u64,               // Current span ID
    pub flags: u8,                  // Sampling flags
    pub reserved: [u8; 3],
}

// Trace flow: Market Event → Self-Contained Execution → Result
impl ArbitrageStrategy {
    async fn process_trade_event(&mut self, trade: &TradeTLV, trace_ctx: &TraceContextTLV) {
        let _span = tracing::span!(
            tracing::Level::INFO, 
            "arbitrage_calculation",
            trace_id = %trace_ctx.trace_id,
            pool_id = %trade.instrument_id.to_u64()
        );
        
        if let Some(opportunity) = self.calculate_arbitrage(trade) {
            // Execute immediately - no signal relay needed
            if let Err(e) = self.execute_arbitrage_atomically(opportunity, trace_ctx).await {
                tracing::error!("Arbitrage execution failed: {}", e);
            }
        }
    }
    
    async fn execute_arbitrage_atomically(&self, opportunity: ArbitrageOpportunity, trace_ctx: &TraceContextTLV) -> Result<()> {
        // Build transaction using execution modules
        let tx = transaction_builder::build_flash_arbitrage_tx(&opportunity)?;
        
        // Submit directly to blockchain
        let result = blockchain_client::submit_transaction(tx).await?;
        
        // Report result for analytics with trace context
        self.report_execution_result(opportunity.id, result, trace_ctx).await;
        
        Ok(())
    }
}
```

---

# Part IV: Operational Characteristics

## Deployment Architecture

### Service Topology

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            PRODUCTION DEPLOYMENT                           │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                      Critical Path Services                         │    │
│  │                                                                     │    │
│  │  • Market Data Collectors (per chain)                              │    │
│  │  • Arbitrage Strategy Engine (self-contained execution)            │    │
│  │  • Execution Coordinator (for risk-managed strategies)             │    │
│  │  • Risk Monitor                                                    │    │
│  │                                                                     │    │
│  │  Deployment: Dedicated high-performance instances                  │    │
│  │  Redundancy: Active-passive failover                               │    │
│  │  Monitoring: Sub-second health checks                              │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                    Supporting Services                              │    │
│  │                                                                     │    │
│  │  • Dashboard UI                                                    │    │
│  │  • Portfolio Manager                                               │    │
│  │  • Analytics Engine                                                │    │
│  │  • Event Archiver                                                  │    │
│  │                                                                     │    │
│  │  Deployment: Standard compute instances                            │    │
│  │  Redundancy: Load balanced, stateless                              │    │
│  │  Monitoring: Standard health checks                                │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Infrastructure Requirements

**Critical Path Performance:**
- **CPU**: High-frequency processors for microsecond-level calculations
- **Memory**: 64GB+ RAM for in-memory state management
- **Network**: Sub-millisecond connectivity to blockchain RPC providers
- **Storage**: NVMe SSDs for event archiving

**Redundancy Strategy:**
- **Geographic Distribution**: Primary/secondary deployments across regions
- **RPC Provider Diversity**: Multiple blockchain RPC providers per chain
- **Smart Contract Deployment**: Identical contracts across multiple addresses

## Security Architecture

### Key Management

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                             SECURITY LAYERS                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                    Wallet Security                                  │    │
│  │                                                                     │    │
│  │  • Hardware Security Modules (HSM) for private key storage         │    │
│  │  • Multi-signature wallets for large fund management               │    │
│  │  • Separate wallets per chain to limit blast radius                │    │
│  │  • Time-locked withdrawals for emergency recovery                  │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                  Smart Contract Security                            │    │
│  │                                                                     │    │
│  │  • Formal verification of arbitrage contract logic                 │    │
│  │  • Multi-step testing on testnets before mainnet deployment        │    │
│  │  • Emergency pause mechanisms in contract code                     │    │
│  │  • Audit trail for all contract interactions                       │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                   Network Security                                  │    │
│  │                                                                     │    │
│  │  • Private VPC for all trading infrastructure                      │    │
│  │  • Encrypted connections to all external services                  │    │
│  │  • Rate limiting and DDoS protection                               │    │
│  │  • Intrusion detection and monitoring                              │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
```

### MEV Protection Strategy

**Transaction Privacy:**
```rust
enum SubmissionStrategy {
    PublicMempool {
        gas_price_strategy: GasPriceStrategy,
    },
    PrivateMempool {
        builder: MEVBuilder,           // Flashbots, Eden, etc.
        bundle_strategy: BundleStrategy,
    },
    Hybrid {
        fallback_timeout: Duration,    // Try private first, then public
    },
}

impl ExecutionEngine {
    async fn submit_arbitrage_transaction(&self, tx: Transaction) -> Result<H256> {
        match &self.submission_strategy {
            SubmissionStrategy::PrivateMempool { builder, .. } => {
                // Submit via MEV-protected private mempool
                let bundle = MEVBundle {
                    transactions: vec![tx],
                    block_number: self.get_next_block().await?,
                    min_timestamp: None,
                    max_timestamp: Some(current_time() + 12), // 1 block timeout
                };
                
                builder.submit_bundle(bundle).await
            }
            SubmissionStrategy::PublicMempool { gas_price_strategy } => {
                // Traditional public mempool submission
                let gas_price = gas_price_strategy.get_current_price().await?;
                self.rpc_client.send_transaction(tx.with_gas_price(gas_price)).await
            }
            SubmissionStrategy::Hybrid { fallback_timeout } => {
                // Try private first, fallback to public
                tokio::select! {
                    result = self.submit_private(tx.clone()) => result,
                    _ = tokio::time::sleep(*fallback_timeout) => {
                        self.submit_public(tx).await
                    }
                }
            }
        }
    }
}
```

## Scaling and Evolution

### Horizontal Scaling Patterns

**Strategy Engine Scaling:**
```rust
// Scale by token pair groupings
enum StrategySharding {
    ByMarketCap {
        high_volume_pairs: Vec<TokenPair>,    // Dedicated instances
        mid_volume_pairs: Vec<TokenPair>,     // Shared instances
        long_tail_pairs: Vec<TokenPair>,      // Batch processing
    },
    ByChain {
        polygon_strategy: ArbitrageEngine,
        ethereum_strategy: ArbitrageEngine,
        arbitrum_strategy: ArbitrageEngine,
    },
    ByVenue {
        uniswap_strategy: ArbitrageEngine,
        sushiswap_strategy: ArbitrageEngine,
        curve_strategy: ArbitrageEngine,
    },
}
```

**Relay Scaling:**
```rust
// Shard relays by message volume
struct RelaySharding {
    high_frequency_relay: Relay,     // Top 100 trading pairs
    medium_frequency_relay: Relay,   // Next 500 trading pairs  
    low_frequency_relay: Relay,      // Long tail pairs
}

// Route messages based on instrument ID
fn route_message(msg: &TLVMessage) -> RelayId {
    let instrument_id = extract_instrument_id(msg);
    match get_volume_tier(instrument_id) {
        VolumeTier::High => RelayId::HighFrequency,
        VolumeTier::Medium => RelayId::MediumFrequency,
        VolumeTier::Low => RelayId::LowFrequency,
    }
}
```

### Future Architecture Evolution

**Migration to Message Bus:**
```rust
// Current: Unix Domain Sockets
struct CurrentArchitecture {
    market_data_socket: UnixListener,
    signal_socket: UnixListener,
    execution_socket: UnixListener,
}

// Future: High-performance message bus
struct FutureArchitecture {
    message_bus: MessageBus<TLVMessage>,
    partitions: Vec<Partition>,
    consumer_groups: HashMap<ServiceType, ConsumerGroup>,
}

// Migration strategy: gradual service transition
enum TransportMode {
    UnixSockets,                    // Phase 1: Current implementation
    Mixed(Vec<ServiceId>),         // Phase 2: Migrate services one by one
    MessageBus,                    // Phase 3: Full message bus deployment
}
```

# Part V: Project Structure

## Directory Organization

```
alphapulse/
├── README.md
├── Cargo.toml                     # Workspace configuration
├── Cargo.lock
├── .gitignore
├── docker-compose.yml             # Development environment
│
├── core/                         # Shared protocol implementation
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── protocol/             # Core protocol types
│   │   │   ├── mod.rs
│   │   │   ├── header.rs         # MessageHeader implementation
│   │   │   ├── tlv.rs            # TLV parsing and building
│   │   │   ├── instrument_id.rs  # Bijective ID implementation
│   │   │   └── relay.rs          # Relay client/server
│   │   ├── types/                # Domain-specific TLV types
│   │   │   ├── mod.rs
│   │   │   ├── market_data.rs    # Types 1-19 (TradeTLV, QuoteTLV)
│   │   │   ├── signals.rs        # Types 20-39 (SignalIdentityTLV, EconomicsTLV)
│   │   │   ├── execution.rs      # Types 40-59 (OrderRequestTLV, ExecutionResultTLV)
│   │   │   └── system.rs         # Types 100-109 (HeartbeatTLV, SnapshotTLV)
│   │   └── utils/                # Common utilities
│   │       ├── mod.rs
│   │       ├── checksum.rs
│   │       ├── serialization.rs
│   │       └── time.rs
│   └── tests/                    # Protocol integration tests
│       ├── tlv_roundtrip.rs
│       ├── relay_communication.rs
│       └── performance_benchmarks.rs
│
├── services/                     # Trading system services
│   ├── market-data/              # Market data collection services
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── collectors/       # Venue-specific collectors
│   │   │   │   ├── mod.rs
│   │   │   │   ├── polygon.rs    # Polygon WebSocket collector
│   │   │   │   ├── ethereum.rs   # Ethereum WebSocket collector
│   │   │   │   ├── binance.rs    # Binance API collector
│   │   │   │   └── coinbase.rs   # Coinbase Pro collector
│   │   │   ├── normalizer.rs     # Event → TLV transformation
│   │   │   ├── connection_manager.rs # WebSocket lifecycle
│   │   │   └── config.rs         # Configuration management
│   │   └── config/
│   │       ├── polygon.toml
│   │       ├── ethereum.toml
│   │       └── venues.toml
│   │
│   ├── arbitrage-strategy/       # Strategy engine
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── engine.rs         # Core arbitrage logic
│   │   │   ├── pool_state.rs     # Pool state management
│   │   │   ├── amm_math/         # DEX math implementations (check existing crates first)
│   │   │   │   ├── mod.rs
│   │   │   │   ├── uniswap_v2.rs
│   │   │   │   ├── uniswap_v3.rs
│   │   │   │   └── curve.rs
│   │   │   ├── gas_estimator.rs  # Gas cost calculation
│   │   │   ├── profitability.rs  # Economic analysis
│   │   │   └── signal_builder.rs # Signal TLV construction
│   │   └── config/
│   │       └── strategy.toml
│   │
│   ├── execution-engine/         # Trade execution service
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── engine.rs         # Execution coordination
│   │   │   ├── transaction_builder.rs # Smart contract calls
│   │   │   ├── gas_price.rs      # Real-time gas pricing
│   │   │   ├── mev_protection.rs # Flashbots integration
│   │   │   ├── monitoring.rs     # Transaction tracking
│   │   │   └── wallet.rs         # Key management
│   │   └── config/
│   │       ├── execution.toml
│   │       └── wallets.toml
│   │
│   ├── dashboard/                # Web-based UI
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── server.rs         # Web server (Axum)
│   │   │   ├── websocket.rs      # Real-time data streaming
│   │   │   ├── api/              # REST API handlers
│   │   │   │   ├── mod.rs
│   │   │   │   ├── signals.rs    # Signal management API
│   │   │   │   ├── execution.rs  # Execution control API
│   │   │   │   └── analytics.rs  # Performance metrics API
│   │   │   ├── state.rs          # Multi-relay consumer
│   │   │   └── templates/        # HTML templates
│   │   ├── static/               # CSS, JS, images
│   │   │   ├── css/
│   │   │   ├── js/
│   │   │   └── images/
│   │   └── config/
│   │       └── dashboard.toml
│   │
│   ├── portfolio-manager/        # Portfolio and P&L tracking
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── portfolio.rs      # Position management
│   │   │   ├── pnl_tracker.rs    # Profit/loss calculation
│   │   │   ├── risk_metrics.rs   # Risk calculation
│   │   │   └── reporting.rs      # Performance reports
│   │   └── config/
│   │       └── portfolio.toml
│   │
│   ├── risk-monitor/             # Risk management service
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── monitor.rs        # Risk monitoring engine
│   │   │   ├── circuit_breakers.rs # Trading halts
│   │   │   ├── alerts.rs         # Alert system
│   │   │   └── metrics.rs        # Risk metrics calculation
│   │   └── config/
│   │       └── risk.toml
│   │
│   └── relays/                   # Message relay infrastructure
│       ├── Cargo.toml
│       ├── src/
│       │   ├── main.rs           # Multi-relay coordinator
│       │   ├── market_data_relay.rs # Domain 1 relay
│       │   ├── signal_relay.rs   # Domain 2 relay
│       │   ├── execution_relay.rs # Domain 3 relay
│       │   ├── consumer_manager.rs # Consumer connection handling
│       │   ├── archiver.rs       # Event archival service
│       │   └── recovery.rs       # Sequence gap recovery
│       └── config/
│           └── relays.toml
│
├── contracts/                    # Smart contract implementations
│   ├── src/
│   │   ├── FlashArbitrage.huff   # Main arbitrage contract
│   │   ├── interfaces/           # Contract interfaces
│   │   │   ├── IAAVE.huff
│   │   │   ├── IUniswapV2.huff
│   │   │   └── IUniswapV3.huff
│   │   └── libraries/            # Shared contract libraries
│   │       ├── Math.huff
│   │       └── SafeTransfer.huff
│   ├── script/                   # Deployment scripts
│   │   ├── Deploy.s.sol
│   │   └── Verify.s.sol
│   ├── test/                     # Contract tests
│   │   ├── FlashArbitrage.t.sol
│   │   └── integration/
│   │       └── FullSystem.t.sol
│   └── foundry.toml              # Foundry configuration
│
├── tools/                        # Development and operational tools
│   ├── Cargo.toml
│   ├── src/
│   │   ├── bin/                  # Command-line tools
│   │   │   ├── relay-cli.rs      # Relay management CLI
│   │   │   ├── signal-replay.rs  # Historical signal replay
│   │   │   ├── performance-test.rs # Load testing tool
│   │   │   └── config-validator.rs # Configuration validation
│   │   └── lib.rs                # Shared tool utilities
│   └── scripts/                  # Operational scripts
│       ├── deploy.sh             # Service deployment
│       ├── health-check.sh       # Service health monitoring
│       └── backup.sh             # Data backup procedures
│
├── tests/                        # System integration tests
│   ├── integration/              # End-to-end test scenarios
│   │   ├── arbitrage_flow.rs     # Complete arbitrage execution
│   │   ├── market_data_flow.rs   # Data collection to strategy
│   │   ├── failover_scenarios.rs # Connection failure handling
│   │   └── performance_tests.rs  # Throughput and latency tests
│   ├── fixtures/                 # Test data
│   │   ├── market_events.json
│   │   ├── pool_states.json
│   │   └── gas_prices.json
│   └── common/                   # Shared test utilities
│       ├── mod.rs
│       ├── mock_relays.rs
│       ├── test_venues.rs
│       └── assertions.rs
│
├── docs/                         # Additional documentation
│   ├── ARCHITECTURE.md           # This document
│   ├── DEPLOYMENT.md             # Production deployment guide
│   ├── DEVELOPMENT.md            # Development setup guide
│   ├── API.md                    # Dashboard API documentation
│   └── TROUBLESHOOTING.md        # Common issues and solutions
│
└── config/                       # Environment configurations
    ├── shared/                   # Cross-service configuration
    │   ├── venues.toml           # Venue definitions used by all collectors
    │   └── chains.toml           # Chain-specific settings (RPC endpoints, etc.)
    ├── development/              # Local development
    │   ├── services.toml
    │   └── contracts.toml
    ├── staging/                  # Staging environment
    │   ├── services.toml
    │   └── contracts.toml
    └── production/               # Production environment
        ├── services.toml
        └── contracts.toml
```

## Workspace Configuration

**Root Cargo.toml:**
```toml
[workspace]
members = [
    "core",
    "services/market-data",
    "services/arbitrage-strategy", 
    "services/execution-engine",
    "services/dashboard",
    "services/portfolio-manager",
    "services/risk-monitor",
    "services/relays",
    "services/observability",
    "tools",
]

[workspace.dependencies]
alphapulse-core = { path = "./core" }
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
zerocopy = { version = "0.7", features = ["derive"] }
tracing = "0.1"
anyhow = "1.0"

[workspace.metadata.release]
tag-prefix = "v"
pre-release-replacements = [
    { file = "README.md", search = "Version: [0-9.]+", replace = "Version: {{version}}" }
]
```

## Design Principles

**Type Organization:**
- **Core Protocol Types**: TLV message types that cross service boundaries via relays (e.g., `TradeTLV`, `SignalIdentityTLV`)
- **Service-Specific Types**: Business logic types that remain within service boundaries (e.g., `ArbitrageOpportunity`, `TransactionRequest`)

**Configuration Strategy:**
- **Shared Configuration**: Venue definitions, chain settings, and other cross-service constants
- **Environment-Specific**: Service parameters that vary between development, staging, and production

**Observability Evolution:**
- **Phase 1**: Built-in metrics and TraceContextTLV (Type 120) distributed tracing
- **Phase 2**: Cross-domain trace correlation and business logic attribution  
- **Phase 3**: Centralized observability with performance attribution and error correlation

**Core Library (`./core/`):**
- Houses the AlphaPulse protocol implementation
- Shared by all services for consistent TLV handling
- Contains bijective ID logic and relay client/server code

**Services Directory (`./services/`):**
- Each service is an independent binary with its own `Cargo.toml`
- Services communicate only through AlphaPulse TLV messages
- Clean separation allows independent deployment and scaling

**Protocols Directory (`./protocols/`):**
- Documentation-only directory for protocol specifications
- Maintained separately from implementation for clarity
- Referenced by core library implementation

**Smart Contracts (`./contracts/`):**
- Huff-optimized arbitrage contracts
- Uses Foundry for testing and deployment
- Separate from Rust services but integrated via execution engine

This structure follows Rust ecosystem conventions while supporting the distributed nature of the trading system. Each service can be developed, tested, and deployed independently while sharing the common protocol implementation.

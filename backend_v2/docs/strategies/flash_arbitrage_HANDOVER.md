# Flash Arbitrage Strategy Module - Handover Document

## Current Status (Context Handover)

### ✅ Completed Tasks

1. **Module Structure Created**
   - Created directory structure at `backend_v2/services_v2/strategies/flash_arbitrage/`
   - Set up Cargo workspace configuration
   - Added all necessary dependencies (rust_decimal, ethers, dashmap, etc.)

2. **AMM Math Modules**
   - ✅ `src/math/v2_math.rs` - Complete V2 AMM calculations with:
     - Exact output/input calculations
     - Optimal arbitrage amount (closed-form solution)
     - Price impact and slippage calculations
     - Full test coverage
   - ✅ `src/math/v3_math.rs` - V3 tick mathematics with:
     - Swap within tick calculations
     - Amount0/Amount1 delta calculations
     - Price impact for concentrated liquidity
     - Basic test coverage
   - ✅ `src/math/mod.rs` - Module index with unified pool interface
   - ✅ `src/math/optimal_size.rs` - Optimal position sizing with profit/gas/slippage calculations

3. **Pool State Management**
   - ✅ `src/pool_state.rs` - Complete pool state manager with:
     - Fast O(1) lookups using PoolInstrumentId.fast_hash
     - Token and pair indexing for arbitrage detection
     - Support for both V2 and V3 pools
     - Stale pool cleanup
     - Full test coverage

4. **Opportunity Detection**
   - ✅ `src/detector.rs` - Arbitrage opportunity detector with:
     - Multi-directional opportunity evaluation
     - Token price oracle integration
     - Strategy type classification (V2ToV2, V3ToV3, etc.)
     - Profit threshold filtering

5. **Module Foundation**
   - ✅ `src/lib.rs` - Public API exports
   - ✅ `src/executor.rs` - Executor skeleton (TODO: implementation)
   - ✅ `src/mev/mod.rs` and `src/mev/flashbots.rs` - MEV protection skeleton
   - ✅ `src/strategy_engine.rs` - Main engine skeleton
   - ✅ `src/main.rs` - Entry point skeleton

### 🚧 Next Steps (In Priority Order)

#### 1. Complete Executor Implementation (`src/executor.rs`)
```rust
impl Executor {
    pub async fn execute_flash_arbitrage(&self, opp: &Opportunity) -> Result<TxHash> {
        // 1. Build flash loan transaction
        // 2. Submit via Flashbots (MEV protection)
        // 3. Report execution result
    }
}
```

#### 2. Complete MEV Protection (`src/mev/flashbots.rs`)
- Copy logic from old `backend/services/defi/arbitrage/src/mev_protection/flashbots_client.rs`
- Update for new transaction format
- Keep fallback to public mempool

#### 3. Complete Strategy Engine (`src/strategy_engine.rs`)
```rust
pub struct StrategyEngine {
    message_bus: MessageBus,
    pool_manager: PoolStateManager,
    detector: OpportunityDetector,
    executor: Executor,
}

impl StrategyEngine {
    pub async fn run(&mut self) -> Result<()> {
        loop {
            // 1. Read from message bus
            // 2. Update pool state
            // 3. Detect opportunities
            // 4. Execute profitable trades
        }
    }
}
```

#### 4. Message Bus Integration
- Connect to message bus using protocol_v2 TLV messages
- Subscribe to market_data channel
- Process TradeTLV messages to update pool states

#### 5. Configuration Loading (`src/main.rs`)
- Load from TOML config file
- Support environment variable overrides
- Validate configuration parameters

## Key Design Decisions Made

1. **Exact AMM Math**: Using Decimal type for full precision (no floating point)
2. **Pool State**: Using PoolInstrumentId.fast_hash as DashMap key for O(1) lookups
3. **Self-Contained Execution**: Strategy executes directly, no coordination via message bus
4. **MEV Protection**: Flashbots integration from day one

## Configuration Template

Create `config/flash_arbitrage.toml`:
```toml
[strategy]
min_profit_usd = 0.50
max_position_pct = 0.05
slippage_tolerance_bps = 50

[pools]
track_all = false
min_liquidity_usd = 10000

[mev]
use_flashbots = true
fallback_to_public = true
max_gas_price_gwei = 100

[message_bus]
channel = "market_data"
consumer_id = 1
```

## Testing Requirements

1. **Unit Tests**:
   - ✅ AMM math calculations
   - ⏳ Pool state management
   - ⏳ Opportunity detection logic
   - ⏳ Position sizing

2. **Integration Tests**:
   - ⏳ Message bus consumption
   - ⏳ End-to-end arbitrage flow
   - ⏳ MEV protection

## Dependencies to Note

From workspace Cargo.toml:
- `alphapulse-protocol` - For PoolInstrumentId and TLV messages
- `dashmap` - Concurrent HashMap for pool states
- `rust_decimal` - Exact decimal math
- `ethers` - Ethereum interaction
- `tokio` - Async runtime

## Critical Files from Old System to Reference

1. **AMM Math**: ✅ Already ported
2. **MEV Protection**: `backend/services/defi/arbitrage/src/mev_protection/flashbots_client.rs`
3. **Pool Monitoring**: `backend/services/defi/scanner/src/pool_monitor.rs` (for patterns)
4. **Gas Estimation**: `backend/services/defi/scanner/src/gas_estimation.rs`

## Message Bus Integration

The strategy will:
1. Subscribe to `market_data` channel as consumer
2. Process TradeTLV messages to update pool states
3. Execute trades directly (self-contained)
4. Report results for monitoring (optional)

## Next Session Should Start With

1. Create `src/math/mod.rs` to export math modules
2. Implement `src/pool_state.rs` with PoolInstrumentId
3. Continue with detector and executor modules

This handover ensures the next session can continue exactly where we left off, with all context preserved.
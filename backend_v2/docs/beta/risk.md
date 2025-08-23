# Risk Manager Module

## Executive Summary

The Risk Manager implements a dual-path architecture that correctly handles two fundamentally different strategy types: risk-managed strategies that require capital allocation and position sizing, and direct-execution strategies that perform atomic, riskless arbitrage through flash loans.

## Core Architecture Principle

**Not all strategies need risk management.** The system recognizes that:
- **Risk-Managed Strategies**: Take directional positions, require capital, carry market risk
- **Direct-Execution Strategies**: Execute atomic arbitrage, use flash loans, carry no market risk

## Dual-Path Architecture

### Path 1: Risk-Managed Strategies
Strategies that carry directional risk and require capital allocation:
- ML Prediction Strategies
- Statistical Arbitrage  
- Market Making
- Momentum Trading

**Flow**: `Strategy Signal → Risk Assessment → Position Sizing → Execution`

**Risk Checks Applied**:
- Position sizing based on Kelly Criterion or fixed fractional
- Portfolio concentration limits
- Maximum position constraints
- Correlation analysis with existing positions
- Available capital verification

### Path 2: Self-Contained Flash Loan Strategies
Atomic, riskless arbitrage that executes entirely within the strategy:
- Flash Loan DEX Arbitrage
- Triangular Arbitrage  
- MEV Bundle Execution

**Flow**: `Market Data → Strategy (detects opportunity & executes atomically) → Post-Trade Reporting`

**Key Architecture Point**: These strategies DO NOT use our execution engine. They:
- Detect opportunity from market data
- Build and submit transactions directly to blockchain
- Execute atomically within a single block
- Report results for analytics only

**Why Self-Contained?**:
- No position risk (trades are atomic, net-zero position)
- No capital required (flash loans provide liquidity)
- Speed critical (sub-millisecond execution required)
- Execution is inseparable from opportunity detection

## Strategy Registration

Strategies declare their type at initialization:

```rust
pub enum StrategyType {
    RiskManaged {
        strategy_id: u16,
        max_position_usd: u64,
        max_portfolio_pct: f32,
        risk_limit: RiskLimit,
        capital_source: CapitalSource,
        uses_execution_engine: true,  // Always true for risk-managed
    },
    SelfContained {
        strategy_id: u16,
        strategy_type: SelfContainedType,
        max_gas_cost: u64,
        reports_results_only: true,   // Only reports, doesn't request execution
    }
}

pub enum SelfContainedType {
    FlashLoanArbitrage,   // DEX arbitrage with flash loans
    MEVBundle,            // Flashbots bundle execution
    AtomicArbitrage,      // Any atomic, self-executing strategy
}

pub enum CapitalSource {
    Portfolio,           // Uses allocated capital
    FlashLoan,          // Borrows for atomic execution
    Hybrid,             // Can use either
}

pub enum RiskLimit {
    MaxDrawdown(f32),   // Maximum allowed drawdown
    VaR(f32),          // Value at Risk limit
    Volatility(f32),   // Maximum volatility target
}
```

## Risk Assessment for Managed Strategies

### Signal Validation
```rust
pub struct RiskAssessment {
    pub signal_id: u64,
    pub strategy_id: u16,
    pub requested_size: i128,
    pub approved_size: i128,
    pub risk_score: f32,
    pub checks_passed: Vec<RiskCheck>,
    pub checks_failed: Vec<RiskCheck>,
    pub decision: RiskDecision,
}

pub enum RiskDecision {
    Approved { 
        size: i128,
        max_slippage_bps: u16,
        time_in_force_ms: u64,
    },
    Rejected {
        reason: RejectionReason,
        retry_after_ms: Option<u64>,
    },
    Deferred {
        waiting_for: DeferralReason,
        timeout_ms: u64,
    },
}
```

### Position Sizing Logic
```rust
impl RiskManager {
    fn calculate_position_size(&self, signal: &Signal, portfolio: &Portfolio) -> i128 {
        let available_capital = portfolio.get_available_capital(signal.strategy_id);
        let current_exposure = portfolio.get_strategy_exposure(signal.strategy_id);
        
        // Kelly Criterion or fixed fractional
        let kelly_size = self.kelly_criterion(
            signal.expected_return,
            signal.win_probability,
            available_capital
        );
        
        // Apply constraints
        let constrained_size = kelly_size
            .min(self.max_position_size)
            .min(available_capital * self.max_concentration)
            .min(self.volatility_adjusted_size(signal));
            
        constrained_size
    }
}
```

## Self-Contained Strategy Reporting

For self-contained flash loan strategies, the Risk Manager only receives post-trade reports:

```rust
pub struct PostTradeAnalytics {
    pub execution_id: u64,
    pub strategy_id: u16,
    pub profit_usd: i128,
    pub gas_cost_usd: u64,
    pub execution_time_ms: u64,
    pub slippage_bps: i16,
    pub success_rate_update: f32,
}

impl RiskManager {
    fn analyze_direct_execution(&mut self, result: ExecutionResult) {
        // Update strategy performance metrics
        self.update_success_rate(result.strategy_id, result.success);
        
        // Check if strategy should be paused
        if self.get_failure_rate(result.strategy_id) > 0.1 {
            self.pause_strategy(result.strategy_id);
        }
        
        // Record for analysis but don't block execution
        self.record_execution_metrics(result);
    }
}
```

## Circuit Breakers

Both paths respect emergency circuit breakers:

```rust
pub struct CircuitBreaker {
    pub global_kill_switch: bool,
    pub paused_strategies: HashSet<u16>,
    pub max_daily_loss: i128,
    pub current_daily_pnl: i128,
    pub max_gas_cost: u64,
}

impl CircuitBreaker {
    fn should_halt(&self, signal: &Signal) -> bool {
        self.global_kill_switch ||
        self.paused_strategies.contains(&signal.strategy_id) ||
        self.current_daily_pnl < -self.max_daily_loss
    }
}
```

## Configuration Example

```toml
[risk_manager]
dual_path_enabled = true
max_concurrent_assessments = 100

[risk_managed_strategies]
default_max_position_usd = 100_000
default_max_concentration = 0.20
position_sizing_method = "kelly_criterion"
max_correlation = 0.70

[self_contained_strategies]
# These execute independently - we only track results
track_performance = true
max_gas_cost_usd = 100
min_profit_usd = 10
failure_rate_threshold = 0.10
alert_on_failure = true

[circuit_breakers]
max_daily_loss_usd = 50_000
emergency_pause_on_failure_rate = 0.25
global_kill_switch = false
```

## Performance Targets

### Risk-Managed Path
- Signal assessment latency: <1ms
- Position sizing calculation: <500μs
- Portfolio query response: <300μs

### Direct Execution Path
- Signal pass-through: <10μs (minimal processing)
- Post-trade analytics: <5ms (non-blocking)

## Summary

The dual-path architecture respects the fundamental difference between:

1. **Risk-Managed Strategies**: Need position sizing, capital allocation, risk limits, and use our execution engine
2. **Self-Contained Flash Loan Strategies**: Execute atomically within the strategy itself, only report results

This design:
- Eliminates unnecessary complexity by keeping flash loan execution self-contained
- Simplifies the development roadmap (can build flash loan arb before Portfolio/Risk/Execution engines)
- Maintains proper risk controls for capital-intensive strategies
- Allows flash loan strategies to operate independently with just market data input

**Key Insight**: Flash loan arbitrage is fundamentally different - it's not about managing positions or routing orders, it's about detecting and atomically executing opportunities in a single transaction. Trying to separate detection from execution would add latency and complexity with no benefit.
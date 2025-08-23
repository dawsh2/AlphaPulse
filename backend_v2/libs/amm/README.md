# AlphaPulse AMM Library

Mathematics and pool interfaces for Automated Market Makers including Uniswap V2/V3, SushiSwap, and other DEX protocols.

## Components

### V2 Math (`v2_math.rs`)
- Uniswap V2 style constant product formula calculations
- Output amount calculation with fees
- Input amount calculation (reverse)
- Slippage and price impact calculations

### V3 Math (`v3_math.rs`)
- Uniswap V3 concentrated liquidity calculations
- Tick-based price ranges
- Fee tier support (0.05%, 0.3%, 1.0%)
- Complex liquidity position math

### Optimal Size Calculator (`optimal_size.rs`)
- Arbitrage opportunity sizing
- Risk-adjusted position sizing
- Capital efficiency optimization
- Gas cost consideration

### Pool Traits (`pool_traits.rs`)
- Unified `AmmPool` interface
- Pool type identification
- Generic arbitrage calculations

## Usage

```rust
use alphapulse_amm::{
    V2Math, V2PoolState, AmmPool, 
    OptimalSizeCalculator, Decimal, dec
};

// V2 calculation
let output = V2Math::calculate_output_amount(
    dec!(100),     // input amount
    dec!(10000),   // reserve in  
    dec!(5000),    // reserve out
    30             // 0.3% fee
)?;

// Using trait interface
let pool = V2PoolState { /* ... */ };
let amount_out = pool.get_amount_out(dec!(100))?;
```

## Migration from strategies/flash_arbitrage/math

Replace imports:
```rust
// Old
use crate::math::{V2Math, V2PoolState};

// New
use alphapulse_amm::{V2Math, V2PoolState};
```
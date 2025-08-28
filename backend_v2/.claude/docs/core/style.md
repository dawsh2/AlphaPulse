# Torq Code Style Guide

## Core Philosophy: Greenfield Advantage

**Breaking changes are encouraged to improve the system.**

- Break APIs freely to improve design
- Remove deprecated code immediately  
- Update ALL references in same commit
- Delete unused code - no "just in case"

### Migration Style: All-at-Once
```bash
# ✅ CORRECT: Complete migration in single commit
git commit -m "Replace DataHandler with MarketDataProcessor

- Rename DataHandler → MarketDataProcessor across entire codebase
- Update all 47 call sites in services/
- Remove deprecated DataHandler completely"
```

## Rust Formatting & Naming

### Naming Conventions
```rust
// Modules: snake_case
mod exchange_collector;

// Structs/Enums: PascalCase  
struct TradeMessage;
enum ExchangeType { Kraken, Coinbase }

// Functions: snake_case
fn process_trade_message() -> Result<()> {}

// Constants: SCREAMING_SNAKE_CASE
const MAX_RECONNECT_ATTEMPTS: u32 = 10;
```

### Variable Naming Style
```rust
// ❌ WRONG: Cryptic abbreviations
fn proc(d: &str, t: u8, usr: u64) -> Vec<u8> { }

// ✅ CORRECT: Clear, searchable names
fn process_trade_message(data: &str, message_type: u8, user_id: u64) -> Vec<TradeMessage> { }
```

### Function Signatures
```rust
// ✅ CORRECT: Descriptive types and names
fn execute_trade(
    user_id: UserId, 
    pool_address: PoolAddress, 
    amount: TokenAmount
) -> Result<TransactionHash, ExecutionError> {
    // Implementation
}
```

## File Organization

### Directory Structure Rules
```
// ✅ CORRECT: Clear hierarchy
services_v2/adapters/src/input/collectors/binance.rs
services_v2/strategies/flash_arbitrage/src/strategy_engine.rs
protocol_v2/src/tlv/market_data.rs

// ❌ WRONG: Flat or confusing structure  
binance_collector.rs
binance_collector_enhanced.rs
strategy.rs
```

### File Naming Conventions
```rust
// ✅ CORRECT: Single canonical files
pool_state.rs
trade_executor.rs
message_parser.rs

// ❌ WRONG: Multiple versions with adjectives
enhanced_pool_state.rs
new_pool_state.rs
pool_state_v2.rs
pool_state_fixed.rs
```

### Before Adding New Files
```bash
# Always check first to prevent duplication
rq check new_component_name
rq similar new_component_name

# Update README.md before creating files
echo "## ComponentName - Purpose and scope" >> README.md
```

## Code Organization Style

### Import Organization
```rust
// ✅ CORRECT: Organized imports
use std::collections::HashMap;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::types::{TradeMessage, InstrumentId};
use super::validation::validate_message;
```

### Error Handling Style
```rust
// ✅ CORRECT: Consistent error context pattern
pub fn process_message(raw: &str) -> Result<TradeMessage, CollectorError> {
    let parsed = serde_json::from_str(raw)
        .map_err(|e| CollectorError::ParseFailed { 
            source: e, 
            input_length: raw.len() 
        })?;
        
    validate_message(&parsed)
        .map_err(CollectorError::ValidationFailed)?;
        
    Ok(parsed.into())
}
```

### Configuration Style
```rust
// ✅ CORRECT: Structured configuration
#[derive(Debug, Clone)]
pub struct CollectorConfig {
    pub websocket_url: String,
    pub reconnect_attempts: u32,
    pub heartbeat_interval: Duration,
}

// Use in constructor
impl Collector {
    pub fn new(config: CollectorConfig) -> Self {
        // Never hardcode values in implementation
    }
}
```

## Documentation Style

### Module Documentation
```rust
//! # ExchangeCollector - Market Data Collection Service
//!
//! ## Purpose
//! Connects to exchange WebSocket feeds and converts to Protocol V2 TLV messages.
//!
//! ## Integration Points  
//! - **Input**: Exchange WebSocket (JSON messages)
//! - **Output**: MarketDataRelay (TLV messages)
//!
//! ## Performance
//! - Target: <35μs message processing latency
//! - Throughput: >10K messages/second per connection
```

### Function Documentation
```rust
/// Convert exchange trade data to Protocol V2 TradeTLV
/// 
/// # Arguments
/// * `trade_data` - Raw exchange trade in JSON format
/// * `instrument_id` - Bijective instrument identifier
/// 
/// # Errors
/// Returns `CollectorError` if JSON parsing fails or validation errors occur
pub fn convert_trade(trade_data: &str, instrument_id: InstrumentId) -> Result<TradeTLV> {
    // Implementation
}
```

## Testing Style

### Test Organization
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    // Real data fixtures - NO MOCKS
    fn create_real_trade_data() -> &'static str {
        // Actual exchange message captured from production
        r#"{"id": 12345, "price": "45000.00", "quantity": "0.1"}"#
    }
    
    #[test]
    fn test_trade_conversion_with_real_data() {
        let trade_data = create_real_trade_data();
        let result = convert_trade(trade_data, instrument_id);
        assert!(result.is_ok());
    }
}
```

### Benchmark Style
```rust
#[cfg(test)]
mod benchmarks {
    use criterion::{black_box, criterion_group, criterion_main, Criterion};
    
    fn bench_message_parsing(c: &mut Criterion) {
        let data = create_real_message_data();
        c.bench_function("parse_trade_message", |b| {
            b.iter(|| parse_message(black_box(&data)))
        });
    }
}
```

## Anti-Patterns We Eliminate

### 1. Adjective File Names
```rust
// ❌ WRONG: Proliferation of adjectives
enhanced_scanner.rs
improved_scanner.rs  
optimized_scanner.rs
fast_scanner.rs

// ✅ CORRECT: Improve the original
scanner.rs  // Continuously improve this one file
```

### 2. Boolean Parameter Blindness  
```rust
// ❌ WRONG: Mystery booleans
connect_to_exchange(true, false, true);

// ✅ CORRECT: Named configuration
connect_to_exchange(ConnectionConfig {
    use_ssl: true,
    verify_certificate: false,
    enable_compression: true,
});
```

### 3. Silent Error Handling
```rust
// ❌ WRONG: Swallow errors silently
match send_message(msg) {
    Ok(_) => {},
    Err(_) => {},  // Silent failure!
}

// ✅ CORRECT: Explicit error handling with context
match send_message(msg) {
    Ok(_) => {},
    Err(e) => {
        error!("Failed to send message to relay: {}", e);
        metrics.increment_send_failures();
        return Err(Error::RelaySendFailed(e));
    }
}
```

### 4. Magic Numbers
```rust
// ❌ WRONG: Hardcoded magic numbers
if spread > 0.005 { execute_trade(); }  // What's 0.005?

// ✅ CORRECT: Named constants with context
const MIN_PROFITABLE_SPREAD_PERCENTAGE: f64 = 0.005; // 0.5%

if spread > MIN_PROFITABLE_SPREAD_PERCENTAGE {
    execute_trade();
}
```

## Code Review Checklist

### Style Consistency
- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] No hardcoded magic numbers
- [ ] Consistent error handling patterns
- [ ] Clear, searchable variable names
- [ ] Proper module documentation

### File Organization
- [ ] Files in correct directory per project structure
- [ ] No duplicate functionality across files
- [ ] README.md updated for new components
- [ ] Single canonical implementation per concept

### Torq Conventions
- [ ] Real data in tests (no mocks)
- [ ] Configuration-driven behavior (no hardcoded thresholds)
- [ ] Performance benchmarks for critical paths
- [ ] Proper TLV domain separation (MarketData/Signal/Execution)

## Summary

This style guide focuses on **how** to write and organize code within Torq:
- Naming and formatting conventions
- File organization and structure  
- Documentation patterns
- Testing and benchmarking style
- Project-specific anti-patterns

For **what patterns to use and when**, see `.claude/principles.md`.
For **Torq-specific technical requirements**, see `.claude/practices.md`.
# STYLE.md - AlphaPulse Code Style Guide

## Core Philosophy
1. **Data Integrity Above All** - Never compromise precision for convenience
2. **No Deception** - Complete transparency in system behavior, no hiding failures or faking success
3. **Quality Over Speed** - Building robust/safe/validated systems takes priority over quick task completion
4. **Production-Ready Code** - ALWAYS write code as if it's going straight into production with real money. Never use fake/mock/dummy variables, services, or data. Every function must be production-quality from the start.
5. **Performance Where It Matters** - Optimize hot paths, keep warm paths maintainable  
6. **Explicit Over Implicit** - Clear intent, no magic
7. **Dynamic Over Hardcoded** - Avoid hardcoded values where dynamic configuration is more appropriate
8. **One Canonical Source** - One file per concept, no duplicates with adjective prefixes/suffixes
9. **Respect Project Structure** - Maintain established file hierarchy and service boundaries
10. **Test What Matters** - Focus on precision, edge cases, and performance
11. **NO MOCKS EVER** - Never use mock data, mock services, or any form of mocked testing under any circumstances

## Rust Style Guide

### General Rules
```rust
// Use rustfmt defaults
// Run before every commit:
cargo fmt --all

// Use clippy for linting
cargo clippy --workspace -- -D warnings
```

### Naming Conventions
```rust
// Modules: snake_case
mod exchange_collector;
mod binary_protocol;

// Structs/Enums: PascalCase
struct TradeMessage;
enum ExchangeType { Kraken, Coinbase }

// Functions/Methods: snake_case
fn process_trade_message() -> Result<()> {}

// Constants: SCREAMING_SNAKE_CASE
const BINARY_MESSAGE_SIZE: usize = 48;
const MAX_RECONNECT_ATTEMPTS: u32 = 10;

// Statics: SCREAMING_SNAKE_CASE
static DECIMAL_PRECISION: u32 = 8;
```

### File Organization - One Canonical Source
```rust
// ❌ WRONG - Multiple versions with adjectives
enhanced_pool_state.rs
fixed_pool_state.rs
new_pool_state.rs
pool_state_v2.rs
pool_state_old.rs

// ✅ CORRECT - Single canonical file
pool_state.rs

// ❌ WRONG - Duplicate implementations
fast_scanner.rs
enhanced_scanner.rs
optimized_scanner.rs

// ✅ CORRECT - Single implementation, improved in place
scanner.rs

// When refactoring: update the existing file, don't create duplicates
// Use version control (git) for history, not filename suffixes
```

### Project Structure & Hierarchy
```
// ❌ WRONG - Files scattered in wrong locations
backend/arbitrage_bot.py         // Should be in services/defi/
backend/scanner.rs                // Should be in services/
backend/scripts/pool_state.rs    // Core logic doesn't belong in scripts/

// ✅ CORRECT - Respect established hierarchy
backend/
├── services/              // All microservices
│   ├── exchange_collector/ // Exchange data collection service
│   ├── defi/              // All DeFi-related services
│   │   ├── scanner/       // Arbitrage scanner
│   │   └── arbitrage_bot/ // Arbitrage executor
│   └── ws_bridge/         // WebSocket bridge
├── scripts/               // Utility scripts only
│   └── defi/              // DeFi-specific utilities
└── api/                   // API endpoints

// Service boundaries must be respected:
// - exchange_collector handles exchange connections
// - defi services handle DeFi logic
// - Don't mix responsibilities across service boundaries
```

### Binary Protocol Handling (CRITICAL)
```rust
// ALWAYS use fixed-point arithmetic for prices/volumes
// NEVER use f32/f64 for financial data

// ✅ CORRECT - Preserves precision
#[derive(Debug, Clone, Copy)]
pub struct TradeMessage {
    pub price: i64,        // Fixed-point, 8 decimals (cents * 10^6)
    pub volume: i64,       // Fixed-point, 8 decimals
    pub timestamp_ns: u64, // Nanoseconds since epoch
    pub side: Side,
    pub symbol_hash: u32,
}

impl TradeMessage {
    pub fn from_decimal(price: Decimal, volume: Decimal) -> Self {
        Self {
            price: (price * Decimal::from(100_000_000)).to_i64().unwrap(),
            volume: (volume * Decimal::from(100_000_000)).to_i64().unwrap(),
            // ...
        }
    }
}

// ❌ WRONG - Loses precision
struct BadTrade {
    price: f64,  // NO! Floating point loses precision
    volume: f64, // NO! Will cause reconciliation errors
}
```

### Error Handling
```rust
// Use thiserror for error types
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CollectorError {
    #[error("WebSocket disconnected: {reason}")]
    Disconnected { reason: String },
    
    #[error("Failed to parse price: {input}")]
    PriceParseError { input: String },
    
    #[error("Precision loss detected: expected {expected}, got {actual}")]
    PrecisionLoss { expected: i64, actual: i64 },
}

// Always propagate errors up, don't hide them
pub fn process_message(raw: &str) -> Result<TradeMessage, CollectorError> {
    let parsed = parse_json(raw)?;  // Use ? operator
    let normalized = normalize_fields(parsed)?;
    validate_precision(&normalized)?;  // Always validate
    Ok(normalized)
}

// NEVER use unwrap() in production code
// ❌ WRONG
let price = data["price"].as_f64().unwrap();

// ✅ CORRECT
let price = data.get("price")
    .and_then(|v| v.as_str())
    .ok_or(CollectorError::MissingField("price"))?;
```

### Performance Patterns
```rust
// Pre-allocate buffers for hot paths
pub struct MessageProcessor {
    buffer: Vec<u8>,  // Reuse allocation
}

impl MessageProcessor {
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(1024),  // Pre-allocate
        }
    }
    
    // Mark hot path functions for inlining
    #[inline(always)]
    pub fn process_hot_path(&mut self, data: &[u8]) {
        self.buffer.clear();  // Reuse buffer
        self.buffer.extend_from_slice(data);
        // Process without allocating
    }
}

// Use zero-copy operations
pub fn zero_copy_parse(data: &[u8]) -> Result<&str> {
    std::str::from_utf8(data)  // No allocation
        .map_err(|e| ParseError::InvalidUtf8(e))
}

// Avoid allocations in loops
// ❌ WRONG
for msg in messages {
    let formatted = format!("Processing: {}", msg);  // Allocates every iteration
}

// ✅ CORRECT  
let mut buffer = String::with_capacity(100);
for msg in messages {
    buffer.clear();
    write!(&mut buffer, "Processing: {}", msg)?;
}
```

### Testing Standards - NO MOCKS, NO CONTRIVED RESULTS
```rust
// ABSOLUTE PROHIBITION: Never use mocks, stubs, fake data, or contrived test results
// ❌ FORBIDDEN - Mock services/data
// let mock_exchange = MockExchange::new();
// let fake_data = create_fake_trade_data();

// ❌ FORBIDDEN - Contrived/fabricated test results
// This is fraudulent - just hardcoding the "improvement"
let huff_gas = solidity_gas * 0.35;  // NEVER DO THIS!
println!("65% gas reduction!");      // Complete fabrication

// ✅ REQUIRED - Real connections, real data, real measurements only
#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    
    // Test precision preservation FIRST
    #[test]
    fn test_decimal_precision_preserved() {
        let prices = vec![
            dec!(0.00000001),
            dec!(12345.67890123),
            dec!(999999.99999999),
        ];
        
        for original in prices {
            let msg = TradeMessage::from_decimal(original, dec!(1));
            let restored = Decimal::from(msg.price) / dec!(100_000_000);
            
            assert_eq!(
                original, restored,
                "Precision lost: {} != {}", original, restored
            );
        }
    }
    
    // Use property-based testing for edge cases
    use proptest::prelude::*;
    
    proptest! {
        #[test]
        fn test_roundtrip_conversion(
            price in 1i64..=999_999_999_999i64,
            volume in 1i64..=999_999_999_999i64,
        ) {
            let msg = TradeMessage { price, volume, ..Default::default() };
            let bytes = msg.to_bytes();
            let restored = TradeMessage::from_bytes(&bytes).unwrap();
            
            prop_assert_eq!(msg.price, restored.price);
            prop_assert_eq!(msg.volume, restored.volume);
        }
    }
}
```

## Python Style Guide

### General Rules
```python
# Use black for formatting (line length 88)
black backend/ --check

# Use ruff for linting
ruff check backend/

# Use mypy for type checking
mypy backend/ --strict
```

### Type Hints Required
```python
from typing import Dict, List, Optional, Union, Tuple
from decimal import Decimal
import numpy as np

# ✅ CORRECT - Full type hints
def process_trade(
    price: Decimal,
    volume: Decimal, 
    timestamp_ns: int,
    side: str
) -> Dict[str, Union[Decimal, int, str]]:
    """Process trade maintaining decimal precision."""
    if price <= 0 or volume <= 0:
        raise ValueError("Price and volume must be positive")
        
    return {
        "price": price,
        "volume": volume,
        "timestamp_ns": timestamp_ns,
        "side": side.lower()
    }

# ❌ WRONG - Missing type hints
def process_trade(price, volume, timestamp, side):  # No type hints!
    return {"price": price, "volume": volume}
```

### Decimal Precision Handling
```python
from decimal import Decimal, getcontext

# Set precision for financial calculations
getcontext().prec = 28  # Higher than needed to catch errors

# ✅ CORRECT - Use Decimal for all financial data
def calculate_value(price: str, volume: str) -> Decimal:
    """Calculate value preserving precision."""
    price_decimal = Decimal(price)
    volume_decimal = Decimal(volume)
    
    # Decimal arithmetic preserves precision
    return price_decimal * volume_decimal

# ❌ WRONG - Float loses precision
def bad_calculate(price: str, volume: str) -> float:
    return float(price) * float(volume)  # Precision loss!

# Converting to fixed-point for binary protocol
def to_fixed_point(value: Decimal) -> int:
    """Convert Decimal to fixed-point integer (8 decimals)."""
    return int(value * Decimal('100000000'))

def from_fixed_point(value: int) -> Decimal:
    """Convert fixed-point integer to Decimal."""
    return Decimal(value) / Decimal('100000000')
```

### Async/Await Patterns
```python
import asyncio
import aiohttp
from typing import AsyncGenerator

# Use async context managers
async def fetch_data(session: aiohttp.ClientSession, url: str) -> Dict:
    """Fetch data with proper resource management."""
    async with session.get(url) as response:
        response.raise_for_status()  # Check status
        return await response.json()

# Async generators for streaming
async def stream_trades() -> AsyncGenerator[Dict, None]:
    """Stream trades from WebSocket."""
    async with aiohttp.ClientSession() as session:
        async with session.ws_connect('wss://exchange.com') as ws:
            async for msg in ws:
                if msg.type == aiohttp.WSMsgType.TEXT:
                    yield json.loads(msg.data)
                elif msg.type == aiohttp.WSMsgType.ERROR:
                    raise ConnectionError(f"WebSocket error: {ws.exception()}")

# Proper concurrent execution
async def process_multiple_exchanges():
    """Process multiple exchanges concurrently."""
    tasks = [
        fetch_kraken_data(),
        fetch_coinbase_data(),
        fetch_uniswap_data(),
    ]
    
    # Use gather with return_exceptions to handle partial failures
    results = await asyncio.gather(*tasks, return_exceptions=True)
    
    for i, result in enumerate(results):
        if isinstance(result, Exception):
            logger.error(f"Exchange {i} failed: {result}")
        else:
            process_result(result)
```

### Production-Ready Code & Transparency
```python
# CRITICAL: Every function must be production-ready from day one
# Treat every line of code as if real money depends on it (because it does!)

# ❌ WRONG - Hiding failures
def get_price():
    try:
        return fetch_from_exchange()
    except:
        return 0.0  # Silently returning fake data!

# ❌ WRONG - Faking success
def execute_trade():
    if not connected:
        return {"status": "success", "simulated": True}  # Deceptive!

# ❌ WRONG - Using dummy/placeholder variables
def process_arbitrage():
    dummy_profit = 100.0  # NEVER use fake values!
    test_gas_cost = 50.0  # Even for "testing"
    return dummy_profit - test_gas_cost

# ❌ WRONG - Rushing to complete task with shortcuts
def quick_arbitrage_scan():
    # Skipping validation to finish faster
    opportunities = find_opportunities()
    return opportunities  # No validation!

# ✅ CORRECT - Complete transparency, production-ready
def get_price():
    try:
        return fetch_from_exchange()
    except ExchangeError as e:
        logger.error(f"Failed to fetch price: {e}")
        raise  # Propagate the failure

# ✅ CORRECT - Honest status reporting, real execution only
def execute_trade():
    if not connected:
        raise ConnectionError("Cannot execute trade: Not connected to exchange")
    # Real execution only - no simulation modes

# ✅ CORRECT - Production values only, comprehensive validation
def process_arbitrage():
    actual_profit = calculate_real_profit_from_pools()
    real_gas_cost = estimate_gas_cost_from_network()
    validated_profit = validate_profit_calculations(actual_profit, real_gas_cost)
    return validated_profit  # Only real, validated data

# ✅ CORRECT - Quality over speed, fully validated
def robust_arbitrage_scan():
    # Take time to validate properly - no shortcuts
    opportunities = find_opportunities()
    validated = validate_liquidity(opportunities)
    confirmed = verify_profitability(validated)
    double_checked = cross_validate_with_multiple_sources(confirmed)
    return double_checked  # Production-ready results only
```

### Error Handling
```python
# Define specific exceptions
class PrecisionError(ValueError):
    """Raised when precision would be lost."""
    pass

class ExchangeError(Exception):
    """Base class for exchange-specific errors."""
    pass

class KrakenError(ExchangeError):
    """Kraken-specific error."""
    pass

# Use context managers for cleanup
from contextlib import asynccontextmanager

@asynccontextmanager
async def websocket_connection(url: str):
    """Manage WebSocket lifecycle."""
    ws = None
    try:
        ws = await connect_websocket(url)
        yield ws
    except Exception as e:
        logger.error(f"WebSocket error: {e}")
        raise
    finally:
        if ws:
            await ws.close()

# Log errors with context
import structlog
logger = structlog.get_logger()

def process_message(msg: Dict) -> Optional[Dict]:
    """Process message with comprehensive error handling."""
    try:
        validated = validate_message(msg)
        normalized = normalize_fields(validated)
        return normalized
        
    except PrecisionError as e:
        logger.error(
            "precision_loss_detected",
            message=msg,
            error=str(e),
            exc_info=True
        )
        raise  # Re-raise precision errors
        
    except Exception as e:
        logger.warning(
            "message_processing_failed", 
            message=msg,
            error=str(e)
        )
        return None  # Graceful degradation for non-critical errors
```

### Testing Standards - NO MOCKS
```python
# ABSOLUTE PROHIBITION: Never use mocks or fake data
# ❌ FORBIDDEN - Mock responses/services
# from unittest.mock import Mock, patch
# mock_exchange = Mock()
# with patch('requests.get') as mock_get:

# ✅ REQUIRED - Real services and data only
import pytest
from decimal import Decimal
from hypothesis import given, strategies as st

class TestPrecision:
    """Test decimal precision preservation."""
    
    @pytest.mark.parametrize("price_str,expected", [
        ("0.00000001", Decimal("0.00000001")),
        ("12345.67890123", Decimal("12345.67890123")),
        ("999999.99999999", Decimal("999999.99999999")),
    ])
    def test_precision_preserved(self, price_str: str, expected: Decimal):
        """Test that precision is preserved through conversions."""
        # Parse
        parsed = Decimal(price_str)
        assert parsed == expected
        
        # Convert to fixed-point and back
        fixed = to_fixed_point(parsed)
        restored = from_fixed_point(fixed)
        assert restored == expected
        
    @given(
        price=st.decimals(
            min_value=Decimal("0.00000001"),
            max_value=Decimal("999999.99999999"),
            places=8
        )
    )
    def test_property_roundtrip(self, price: Decimal):
        """Property test: roundtrip conversion preserves value."""
        fixed = to_fixed_point(price)
        restored = from_fixed_point(fixed)
        
        # Quantize to 8 decimal places for comparison
        expected = price.quantize(Decimal("0.00000001"))
        actual = restored.quantize(Decimal("0.00000001"))
        
        assert expected == actual
```

## TypeScript/React Style Guide

### Component Structure
```typescript
// Functional components with proper typing
interface TradeData {
  price: string;  // Keep as string to preserve precision
  volume: string;
  timestamp: number;
  side: 'buy' | 'sell';
}

interface DashboardProps {
  trades: TradeData[];
  onRefresh: () => Promise<void>;
}

// Component with proper error boundaries
export const TradeDashboard: React.FC<DashboardProps> = ({ 
  trades, 
  onRefresh 
}) => {
  // State management
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  
  // Memoized calculations
  const totalVolume = useMemo(() => {
    // Use decimal.js for precision
    return trades.reduce((sum, trade) => {
      return sum.plus(new Decimal(trade.volume));
    }, new Decimal(0)).toString();
  }, [trades]);
  
  // Error handling in effects
  useEffect(() => {
    const fetchData = async () => {
      try {
        setLoading(true);
        setError(null);
        await onRefresh();
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Unknown error');
      } finally {
        setLoading(false);
      }
    };
    
    fetchData();
  }, [onRefresh]);
  
  // Render with error boundary
  if (error) {
    return <ErrorDisplay message={error} onRetry={onRefresh} />;
  }
  
  return (
    <div className="trade-dashboard">
      {loading && <LoadingSpinner />}
      <TradeList trades={trades} />
      <div>Total Volume: {totalVolume}</div>
    </div>
  );
};
```

### WebSocket Handling
```typescript
// Proper WebSocket management with reconnection
class WebSocketManager {
  private ws: WebSocket | null = null;
  private reconnectAttempts = 0;
  private readonly maxReconnectAttempts = 10;
  
  async connect(url: string): Promise<void> {
    return new Promise((resolve, reject) => {
      this.ws = new WebSocket(url);
      
      this.ws.onopen = () => {
        console.log('WebSocket connected');
        this.reconnectAttempts = 0;
        resolve();
      };
      
      this.ws.onerror = (error) => {
        console.error('WebSocket error:', error);
        reject(error);
      };
      
      this.ws.onclose = () => {
        console.log('WebSocket closed');
        this.attemptReconnect(url);
      };
      
      this.ws.onmessage = (event) => {
        this.handleMessage(event.data);
      };
    });
  }
  
  private attemptReconnect(url: string): void {
    if (this.reconnectAttempts < this.maxReconnectAttempts) {
      this.reconnectAttempts++;
      const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts), 30000);
      
      setTimeout(() => {
        console.log(`Reconnecting... Attempt ${this.reconnectAttempts}`);
        this.connect(url);
      }, delay);
    }
  }
  
  private handleMessage(data: string): void {
    try {
      const message = JSON.parse(data);
      // Validate message structure
      if (this.isValidMessage(message)) {
        this.onMessage?.(message);
      }
    } catch (error) {
      console.error('Failed to parse message:', error);
    }
  }
}
```

## Git Commit Standards

### Format
```
<type>(<scope>): <subject>

<body>

<footer>
```

### Types
- `feat`: New feature
- `fix`: Bug fix
- `perf`: Performance improvement
- `refactor`: Code refactoring
- `test`: Test addition/modification
- `docs`: Documentation changes
- `style`: Code style changes (formatting)
- `chore`: Maintenance tasks
- `precision`: Decimal precision fixes (CRITICAL)

### Examples
```bash
# Feature
feat(collector): add Kraken exchange support

- Implement WebSocket client for Kraken
- Add field normalizer for array format
- Include comprehensive test suite
- Performance: 28μs average processing time

Closes #234

# Critical Fix
precision(protocol): fix decimal rounding in binary conversion

Previous implementation could lose precision beyond 6 decimal
places when converting from string to fixed-point. Now using
Decimal type throughout conversion pipeline.

BREAKING CHANGE: Binary protocol version incremented to v2

# Performance
perf(relay): optimize message routing with zero-copy

Replace Vec<u8> cloning with borrowed slices in hot path.
Reduces latency from 45μs to 31μs (31% improvement).

Benchmark results:
- Before: 45μs p99
- After: 31μs p99
```

## Code Review Checklist

### Before Submitting PR
- [ ] Precision tests passing (`cargo test precision`)
- [ ] No floating point for financial data
- [ ] Type hints on all Python functions
- [ ] Error handling comprehensive
- [ ] Performance impact measured
- [ ] Documentation updated
- [ ] CLAUDE.md updated if needed
- [ ] **NO MOCKS** - Confirmed no mock data, services, or responses used

### Review Focus Areas
1. **Data Integrity**: Check decimal handling
2. **Performance**: Review hot path changes
3. **Error Handling**: Ensure graceful degradation
4. **Testing**: Verify edge cases covered
5. **Documentation**: Confirm clarity

## IDE Configuration

### VS Code Settings
```json
{
  "editor.formatOnSave": true,
  "rust-analyzer.checkOnSave.command": "clippy",
  "python.formatting.provider": "black",
  "python.linting.enabled": true,
  "python.linting.ruffEnabled": true,
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  },
  "[python]": {
    "editor.defaultFormatter": "ms-python.black-formatter"
  }
}
```

### Pre-commit Hooks
```yaml
# .pre-commit-config.yaml
repos:
  - repo: local
    hooks:
      - id: rust-fmt
        name: Rust Format
        entry: cargo fmt --all -- --check
        language: system
        files: \.rs$
        
      - id: rust-clippy
        name: Rust Clippy
        entry: cargo clippy --workspace -- -D warnings
        language: system
        files: \.rs$
        
      - id: python-black
        name: Python Black
        entry: black --check backend/
        language: system
        types: [python]
        
      - id: precision-tests
        name: Precision Tests
        entry: cargo test precision
        language: system
        pass_filenames: false
```

This style guide ensures consistent, high-quality code that maintains data integrity and performance.
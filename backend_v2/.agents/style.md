# STYLE.md - AlphaPulse Code Style Guide

## Core Philosophy: Greenfield Advantage

**This is a greenfield codebase - we embrace breaking changes to build the best possible system.**

### Breaking Changes Are Welcome
- **No backward compatibility concerns** - break APIs freely to improve design
- **Remove deprecated code immediately** - zero tolerance for legacy cruft
- **Clean up after yourself** - remove old patterns when introducing new ones
- **Refactor aggressively** - improve naming, structure, and patterns without hesitation
- **Update all references** - when changing interfaces, update ALL callers in same commit
- **Delete unused code** - don't keep "just in case" code around

### Examples of Encouraged Breaking Changes
```rust
// ❌ OLD: Poor naming that confuses purpose
pub struct DataHandler {
    pub fn handle(&self, data: String) -> String { ... }
}

// ✅ NEW: Clear naming + breaking change (encouraged!)
pub struct MarketDataProcessor {
    pub fn process_trade_event(&self, event: TradeEvent) -> Result<ProcessedTrade, ProcessingError> { ... }
}
// DELETE the old struct entirely, update ALL 47+ references
```

```rust
// ❌ OLD: Hardcoded configuration
impl Scanner {
    fn scan(&self) {
        if profit > 100.0 { ... }  // Hardcoded threshold
    }
}

// ✅ NEW: Configurable + breaking change (encouraged!)
impl Scanner {
    fn scan(&self, config: &ScanConfig) -> Result<Vec<Opportunity>, ScanError> {
        if profit > config.min_profit_threshold { ... }
    }
}
// Update ALL call sites to pass config, remove hardcoded version
```

### Migration Philosophy: All-at-Once
```bash
# ❌ WRONG: Gradual migration leaving inconsistent state
git commit -m "Add new API (old still works)"
git commit -m "Migrate some files..."
git commit -m "Migrate more files..."
# Codebase has mixed old/new patterns

# ✅ CORRECT: Complete migration in single atomic commit
git commit -m "Replace all DataHandler with MarketDataProcessor

- Rename DataHandler -> MarketDataProcessor across entire codebase
- Update all 47 call sites in services/
- Update all 23 test files
- Remove deprecated DataHandler struct completely
- Update documentation to reflect new API"
```

## Development Principles
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
12. **README-First Development** - Before creating new files or directories, update the containing directory's README.md to document purpose, avoid duplication, and maintain clear hierarchy

## Rust Style Guide

### General Rules
Follow rustfmt defaults and clippy suggestions. See [tools.md](tools.md) for specific commands.

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

### Self-Documenting Code Principles

#### Use Types to Express Intent
```rust
// ❌ WRONG - Primitive obsession loses meaning
fn execute_trade(user: String, pool: u64, amount: i64) -> Result<String> { }

// ✅ CORRECT - Domain types provide semantic meaning
#[derive(Debug, Clone)]
struct UserId(pub String);

#[derive(Debug, Clone)]
struct PoolAddress(pub [u8; 20]);

#[derive(Debug, Clone)]
struct TokenAmount {
    pub value: u128,
    pub decimals: u8,
}

fn execute_trade(user: UserId, pool: PoolAddress, amount: TokenAmount) -> Result<TransactionHash> { }
```

#### Write Comprehensive Documentation
```rust
/// Detects arbitrage opportunities between DEX pools.
///
/// This detector evaluates pool pairs to find profitable arbitrage paths,
/// accounting for gas costs, slippage, and minimum profit thresholds.
///
/// # Examples
/// ```
/// let detector = OpportunityDetector::new(pool_manager, config);
/// let opportunities = detector.find_arbitrage(&updated_pool)?;
/// assert!(opportunities.iter().all(|o| o.expected_profit_usd > config.min_profit));
/// ```
///
/// # Errors
/// Returns `DetectorError::PoolNotFound` if the specified pool doesn't exist.
/// Returns `DetectorError::ZeroLiquidity` if pools have insufficient liquidity.
///
/// # Panics
/// Panics if the pool manager is not initialized (this is a bug).
pub fn find_arbitrage(&self, pool_id: &PoolInstrumentId) -> Result<Vec<ArbitrageOpportunity>, DetectorError> {
    // Implementation
}
```

#### Leverage Traits for Shared Behavior
```rust
// ❌ WRONG - Duplicated logic across types
impl V2Pool {
    fn calculate_fee(&self) -> Decimal { self.amount * self.fee_tier / 10000 }
}
impl V3Pool {
    fn calculate_fee(&self) -> Decimal { self.amount * self.fee_tier / 10000 }
}

// ✅ CORRECT - Trait defines shared contract
trait FeeCalculator {
    fn fee_tier(&self) -> u32;
    fn amount(&self) -> Decimal;
    
    fn calculate_fee(&self) -> Decimal {
        self.amount() * Decimal::from(self.fee_tier()) / dec!(10000)
    }
}

impl FeeCalculator for V2Pool {
    fn fee_tier(&self) -> u32 { self.fee_tier }
    fn amount(&self) -> Decimal { self.amount }
}
```

#### Use Iterator Chains Instead of Loops
```rust
// ❌ WRONG - Manual iteration with mutation
let mut profitable = Vec::new();
for opportunity in opportunities {
    if opportunity.profit > min_profit {
        if opportunity.gas_cost < max_gas {
            profitable.push(opportunity);
        }
    }
}

// ✅ CORRECT - Functional, self-documenting chain
let profitable: Vec<_> = opportunities
    .into_iter()
    .filter(|o| o.profit > min_profit)
    .filter(|o| o.gas_cost < max_gas)
    .collect();
```

#### Return Meaningful Types
```rust
// ❌ WRONG - Tuple loses meaning
fn analyze_pool() -> (Decimal, Decimal, bool) {
    (price, liquidity, is_valid)
}

// ✅ CORRECT - Struct provides context
#[derive(Debug)]
struct PoolAnalysis {
    pub price: Decimal,
    pub liquidity: Decimal,
    pub is_valid: bool,
}

fn analyze_pool() -> PoolAnalysis {
    PoolAnalysis { price, liquidity, is_valid }
}
```

#### Module Organization by Feature
```rust
// ❌ WRONG - Technical grouping
src/
  models.rs      // All structs together
  traits.rs      // All traits together  
  impls.rs       // All implementations
  utils.rs       // Grab bag of helpers

// ✅ CORRECT - Domain-driven organization
src/
  arbitrage/
    mod.rs       // Public API re-exports
    detector.rs  // Detection logic
    types.rs     // ArbitrageOpportunity, DetectorConfig
    executor.rs  // Execution logic
  pool/
    mod.rs       // Public API re-exports
    state.rs     // Pool state management
    math.rs      // AMM calculations
```

#### Use Builder Pattern for Complex Configuration
```rust
// ❌ WRONG - Constructor with many parameters
let detector = OpportunityDetector::new(
    pool_manager,
    dec!(10),     // What is this?
    dec!(0.05),   // And this?
    50,           // Magic number
    dec!(5),      // Another mystery
);

// ✅ CORRECT - Self-documenting builder
let detector = OpportunityDetector::builder()
    .pool_manager(pool_manager)
    .min_profit_usd(dec!(10))
    .max_position_pct(dec!(0.05))
    .slippage_tolerance_bps(50)
    .gas_cost_usd(dec!(5))
    .build()?;
```

#### Avoid Premature Abstraction
```rust
// ❌ WRONG - Over-engineered for single use case
trait MessageProcessor<T, U, V> 
where 
    T: Serialize + DeserializeOwned,
    U: Transport,
    V: Validator,
{
    fn process(&self, msg: T, transport: U, validator: V) -> Result<()>;
}

// ✅ CORRECT - Simple and direct until patterns emerge
struct TradeMesageProcessor {
    transport: UnixSocketTransport,
}

impl TradeMessageProcessor {
    fn process(&self, msg: TradeMessage) -> Result<()> {
        // Clear, straightforward implementation
    }
}
// Extract traits only when you have 2+ implementations
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

### README-First Development
```markdown
# CRITICAL: Before creating any new file or directory:
# 1. Check the containing directory's README.md
# 2. Ensure your addition doesn't duplicate existing functionality
# 3. Update README.md to document the new component
# 4. Verify it fits within the established architecture

# ❌ WRONG - Creating files without documentation
touch backend/services/defi/new_scanner.rs
mkdir backend/services/defi/strategies/

# ✅ CORRECT - Document first, then create
# 1. Read backend/services/defi/README.md
# 2. Update README.md with new component description
# 3. Ensure no duplication with existing scanner
# 4. Create file with clear purpose

# Example README.md structure:
## Purpose
This directory contains...

## Files
- `scanner.rs` - Core arbitrage scanning logic
- `pool_monitor.rs` - Pool data monitoring
- `strategies/` - Strategy-specific implementations

## Adding New Components
Before adding files here:
1. Check if functionality already exists
2. Ensure it belongs in this service (not exchange_collector or relay)
3. Update this README with your addition
4. Follow naming conventions (no adjective prefixes)
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

### Documentation Standards - TECHNICAL ACCURACY ONLY
```markdown
# CRITICAL: No marketing language in documentation
# ❌ WRONG: "Revolutionary high-performance trading system"
# ✅ CORRECT: "Trading system with <35μs message processing"

# Be honest about limitations
# ❌ WRONG: "Supports any protocol dynamically"  
# ✅ CORRECT: "Supports UniswapV2, V3, and Curve protocols via configuration"

# Write for engineers and AI agents
# ❌ WRONG: "Blazingly fast with incredible flexibility"
# ✅ CORRECT: "Processes 10,000 messages/second with configurable parameters"
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
Use black, ruff, and mypy. See [TOOLS.md](TOOLS.md) for specific commands.

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
- [ ] **Breaking changes complete** - No deprecated code left behind
- [ ] **All references updated** - Breaking changes propagated throughout codebase

### Review Focus Areas
1. **Data Integrity**: Check decimal handling
2. **Performance**: Review hot path changes
3. **Error Handling**: Ensure graceful degradation
4. **Testing**: Verify edge cases covered
5. **Documentation**: Confirm clarity


## Configuration-Driven Architecture

### Runtime Flexibility Without Code Branches
```rust
// ✅ Abstract transport layer - no if/else needed in business logic
pub trait MessageTransport: Send + Sync {
    fn send(&self, data: &MarketData) -> Result<(), TransportError>;
    fn receive(&self) -> Result<MarketData, TransportError>;
}

// Multiple implementations without code duplication
pub struct SharedMemoryTransport { /* Ultra-fast local IPC */ }
pub struct UnixSocketTransport { /* Process isolation */ }
pub struct NetworkTransport { /* Remote services */ }

// Configuration determines implementation - zero spaghetti code
pub struct TransportFactory;
impl TransportFactory {
    pub fn create(config: &Config) -> Box<dyn MessageTransport> {
        match config.transport_type {
            TransportType::Auto => {
                // Auto-detect best option
                if can_use_shared_memory() {
                    Box::new(SharedMemoryTransport::new())
                } else {
                    Box::new(UnixSocketTransport::new())
                }
            }
            TransportType::SharedMemory => Box::new(SharedMemoryTransport::new()),
            TransportType::UnixSocket => Box::new(UnixSocketTransport::new()),
        }
    }
}

// Clean usage - no transport-specific code in business logic
pub struct Relay {
    transport: Box<dyn MessageTransport>,  // Abstracted away
}

impl Relay {
    pub fn process(&self, data: MarketData) {
        // Same code works with any transport implementation
        self.transport.send(&data).unwrap();
    }
}
```

### Configuration Schema
```yaml
# config.yml - Drives architecture without code changes
alphapulse:
  transport:
    type: auto  # shared_memory, unix_socket, network
    fallback_chain: [shared_memory, unix_socket]
    
    shared_memory:
      path: /dev/shm/alphapulse
      ring_buffer_size: 65536
      
    unix_socket:
      path: /tmp/alphapulse.sock
      buffer_size: 8192
      
  performance:
    target_latency_us: 35
    enable_zero_copy: true
    
  # Breaking changes become config changes
  compatibility:
    symbol_migration_complete: true  # Remove old symbol references
    binary_protocol_version: 2       # Use latest protocol
```

### Deployment-Time Decisions
```bash
# Development - debugging friendly
ALPHAPULSE_TRANSPORT=unix_socket cargo run

# Production - maximum performance  
ALPHAPULSE_TRANSPORT=shared_memory cargo run --release

# Testing - isolated processes
ALPHAPULSE_TRANSPORT=network cargo test
```

## Anti-Patterns We Eliminate

### 1. Duplicate Code with Modifier Names
```rust
// ❌ DON'T: Create enhanced/improved/fixed versions
mod exchange_collector;
mod enhanced_exchange_collector;  // NO!
mod improved_exchange_collector;  // NO!
mod fixed_exchange_collector;     // NO!
mod exchange_collector_v2;        // NO!

// ✅ DO: Improve the original, delete old patterns
mod exchange_collector; // Continuously improve this one file
```

### 2. Compatibility Layers
```rust
// ❌ DON'T: Keep old APIs "for compatibility"
#[deprecated]
pub fn old_function() { /* ... */ }
pub fn new_function() { /* ... */ }

// ✅ DO: Just break and update everything
pub fn process_data() { /* Single, best implementation */ }
```

### 3. Gradual Migrations with Mixed States
```rust
// ❌ DON'T: Leave codebase in mixed state
enum TransportType {
    UnixSocket,       // Old way
    SharedMemory,     // New way - but both exist!
    Enhanced,         // Even newer way - now 3 patterns!
}

// ✅ DO: Atomic replacement
enum TransportType {
    UnixSocket,
    SharedMemory,
    // Only current, best patterns exist
}
```

This style guide ensures consistent, high-quality code that maintains data integrity and performance while embracing the greenfield advantage to continuously improve the codebase through breaking changes.
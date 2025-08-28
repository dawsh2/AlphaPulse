# Torq-Specific Best Practices

## Zero-Copy Message Architecture

### Core Requirement
**All Protocol V2 TLV messages MUST support zero-copy serialization/deserialization at >1M msg/s.**

### Struct Alignment Requirements

#### Memory Alignment Rules
```rust
// ❌ WRONG: Unaligned struct causes copy on serialization
#[derive(Debug)]
pub struct PoolSwapTLV {
    pub pool_address: [u8; 20],      // 20 bytes - NOT aligned!
    pub amount_in: i64,               // Misaligned due to previous field
    pub token_in: [u8; 20],          // Another unaligned field
}

// ✅ CORRECT: Properly aligned with padding
#[repr(C)]
#[derive(Debug, AsBytes, FromBytes)]
pub struct PoolSwapTLV {
    pub pool_address: [u8; 32],      // 32 bytes - naturally aligned
    pub amount_in: i64,               // 8 bytes - aligned on 8-byte boundary
    pub amount_out: i64,              // 8 bytes - maintains alignment
    pub token_in: [u8; 32],          // 32 bytes - aligned
    pub token_out: [u8; 32],         // 32 bytes - aligned
}
```

#### Padding Strategy
```rust
// For Ethereum addresses (20 bytes -> 32 bytes)
#[repr(transparent)]
#[derive(AsBytes, FromBytes)]
pub struct PaddedAddress([u8; 32]);

impl PaddedAddress {
    /// Convert 20-byte Ethereum address to 32-byte padded
    #[inline(always)]
    pub fn from_eth(addr: [u8; 20]) -> Self {
        let mut padded = [0u8; 32];
        padded[..20].copy_from_slice(&addr);
        Self(padded)
    }
    
    /// Extract 20-byte Ethereum address
    #[inline(always)]
    pub fn to_eth(&self) -> [u8; 20] {
        let mut addr = [0u8; 20];
        addr.copy_from_slice(&self.0[..20]);
        addr
    }
    
    /// Validate padding bytes are zero (for deserialization)
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.0[20..].iter().any(|&b| b != 0) {
            return Err(ValidationError::InvalidPadding);
        }
        Ok(())
    }
}
```

### Zero-Copy Checklist
- [ ] Struct has `#[repr(C)]` or `#[repr(packed)]`
- [ ] All fields are aligned to their natural boundaries
- [ ] Derives `AsBytes` and `FromBytes` from zerocopy
- [ ] No heap allocations in hot path (no Vec, String, Box)
- [ ] Uses fixed-size arrays with proper alignment
- [ ] Padding bytes validated on deserialization

## Precision Preservation

### Asset-Specific Precision Rules

```rust
// Traditional Exchanges (CEX): 8-decimal fixed-point for USD prices
pub const CEX_PRICE_SCALE: i64 = 100_000_000;  // 10^8

// DEX Token Amounts: Native precision per token
pub const WETH_DECIMALS: u8 = 18;   // 10^18 wei per ETH
pub const USDC_DECIMALS: u8 = 6;    // 10^6 per USDC
pub const USDT_DECIMALS: u8 = 6;    // 10^6 per USDT
pub const DAI_DECIMALS: u8 = 18;    // 10^18 per DAI

// ❌ WRONG: Normalizing everything to same precision
pub fn normalize_amount(amount: i64, decimals: u8) -> f64 {
    amount as f64 / 10_f64.powi(decimals as i32)  // LOSES PRECISION!
}

// ✅ CORRECT: Preserve native precision
#[derive(Debug, Clone, Copy)]
pub struct TokenAmount {
    pub raw: i128,      // Large enough for 10^38
    pub decimals: u8,   // Token's decimal places
}

impl TokenAmount {
    pub fn from_wei(wei: i128) -> Self {
        Self { raw: wei, decimals: 18 }
    }
    
    pub fn from_usdc_units(units: i64) -> Self {
        Self { raw: units as i128, decimals: 6 }
    }
}
```

### Decimal Arithmetic Rules
```rust
// ❌ WRONG: Float arithmetic for money
let profit = sell_price - buy_price * 0.997;  // Trading fee

// ✅ CORRECT: Integer arithmetic with explicit scaling
const FEE_BPS: i64 = 30;  // 0.3% = 30 basis points
let fee = (amount * FEE_BPS) / 10_000;
let amount_after_fee = amount - fee;
```

## TLV Message Protocol V2

### TLV Type Registry Management
```rust
// protocol_v2/src/tlv/types.rs

// Domain ranges - NEVER violate these boundaries
pub const MARKET_DATA_RANGE: RangeInclusive<u8> = 1..=19;
pub const SIGNAL_RANGE: RangeInclusive<u8> = 20..=39;  
pub const EXECUTION_RANGE: RangeInclusive<u8> = 40..=79;
pub const RESERVED_RANGE: RangeInclusive<u8> = 80..=255;

// When adding new TLV type:
// 1. Pick next available number in correct domain
// 2. NEVER reuse a previously used number
// 3. Update expected_payload_size() method
// 4. Add to TLV registry documentation
```

### Message Construction Pattern
```rust
// ✅ CORRECT: Use TLVMessageBuilder for all messages
pub fn create_trade_message(trade: &Trade) -> Result<Vec<u8>, Error> {
    let mut builder = TLVMessageBuilder::new(RelayDomain::MarketData, SOURCE_ID);
    
    let trade_tlv = TradeTLV {
        instrument_id: trade.instrument_id.to_bytes(),
        price: trade.price,
        quantity: trade.quantity,
        timestamp_ns: trade.timestamp,
        trade_id: trade.id,
    };
    
    builder.add_tlv(TLVType::Trade, &trade_tlv)?;
    builder.set_sequence(get_next_sequence());
    
    Ok(builder.build())
}

// ❌ WRONG: Manual byte manipulation
pub fn create_trade_message_bad(trade: &Trade) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&MAGIC);  // Error-prone
    bytes.extend_from_slice(&trade.instrument_id);
    // ... manual construction
}
```

## Performance Critical Paths

### Hot Path Rules (<35μs latency requirement)
```rust
// Hot path: Exchange WebSocket → Parser → Relay

// ✅ CORRECT: Pre-allocated buffers, zero-copy
pub struct MessageParser {
    buffer: [u8; 65536],  // Stack-allocated
}

impl MessageParser {
    #[inline(always)]
    pub fn parse(&mut self, data: &[u8]) -> Result<Message, Error> {
        // Zero-copy parsing
        let header = MessageHeader::ref_from_prefix(data)
            .ok_or(Error::InvalidHeader)?;
        
        // No allocations
        let tlv = TradeTLV::ref_from_bytes(&data[32..])
            .ok_or(Error::InvalidTLV)?;
        
        Ok(Message { header, tlv })
    }
}

// ❌ WRONG: Allocations in hot path
pub fn parse_bad(data: &[u8]) -> Result<Message, Error> {
    let header_bytes = data[0..32].to_vec();  // ALLOCATION!
    let tlv_bytes = data[32..].to_vec();      // ALLOCATION!
    // ...
}
```

### Warm Path Rules (<100μs latency)
```rust
// Warm path: Relay → Strategy → Signal generation

// Acceptable: Some allocations for business logic
pub fn analyze_arbitrage(pools: &[Pool]) -> Vec<Opportunity> {
    pools.iter()
        .filter_map(|pool| calculate_opportunity(pool))
        .filter(|opp| opp.profit_usd > MIN_PROFIT)
        .collect()  // Allocation acceptable in warm path
}
```

### Cold Path Rules (>1ms acceptable)
```rust
// Cold path: Configuration, monitoring, logging

// OK to prioritize clarity over performance
pub fn load_config() -> Result<Config, Error> {
    let contents = fs::read_to_string("config.toml")?;
    let config: Config = toml::from_str(&contents)?;
    validate_config(&config)?;
    Ok(config)
}
```

## InstrumentId Bijection Requirements

### Bijective Mapping Rules
```rust
// Every InstrumentId MUST be:
// 1. Deterministic: Same inputs always produce same ID
// 2. Reversible: Can extract venue, asset type, and identifiers
// 3. Unique: No collisions possible

// ✅ CORRECT: Bijective construction
pub fn create_pool_id(chain: u8, dex: u8, pool_address: [u8; 20]) -> InstrumentId {
    let mut bytes = [0u8; 32];
    bytes[0] = 3;  // AssetType::Pool
    bytes[1] = chain;
    bytes[2] = dex;
    bytes[3..23].copy_from_slice(&pool_address);
    InstrumentId::from_bytes(bytes)
}

// ❌ WRONG: Non-deterministic or lossy
pub fn create_pool_id_bad(pool_name: &str) -> InstrumentId {
    let hash = hash_string(pool_name);  // Lossy! Can't reverse
    InstrumentId::from_hash(hash)
}
```

## Service Boundary Rules

### Relay Domain Separation
```rust
// Each relay handles specific message types

// MarketDataRelay: Types 1-19
impl MarketDataRelay {
    pub fn accepts(&self, tlv_type: u8) -> bool {
        (1..=19).contains(&tlv_type)
    }
}

// SignalRelay: Types 20-39
impl SignalRelay {
    pub fn accepts(&self, tlv_type: u8) -> bool {
        (20..=39).contains(&tlv_type)
    }
}

// ExecutionRelay: Types 40-79
impl ExecutionRelay {
    pub fn accepts(&self, tlv_type: u8) -> bool {
        (40..=79).contains(&tlv_type)
    }
}

// ❌ WRONG: Mixing domains
impl BadRelay {
    pub fn handle_all(&self, msg: Message) {
        // DON'T mix market data, signals, and execution!
    }
}
```

### Service Communication Patterns
```rust
// ✅ CORRECT: Services communicate through relays
// Collector → MarketDataRelay → Strategy
// Strategy → SignalRelay → Executor
// Executor → ExecutionRelay → Monitor

// ❌ WRONG: Direct service-to-service communication
// Collector → Strategy (bypasses relay)
```

## Memory Management

### Pool Allocation Pattern
```rust
// For high-frequency allocations (>10K/sec)
pub struct BufferPool {
    buffers: Vec<Box<[u8; 4096]>>,
    available: VecDeque<Box<[u8; 4096]>>,
}

impl BufferPool {
    pub fn acquire(&mut self) -> Box<[u8; 4096]> {
        self.available.pop_front()
            .unwrap_or_else(|| Box::new([0u8; 4096]))
    }
    
    pub fn release(&mut self, buffer: Box<[u8; 4096]>) {
        self.available.push_back(buffer);
    }
}
```

### String Interning for Symbols
```rust
// Reuse string allocations for frequently seen symbols
use string_cache::DefaultAtom;

pub struct SymbolCache {
    cache: HashMap<DefaultAtom, InstrumentId>,
}

impl SymbolCache {
    pub fn get_or_create(&mut self, symbol: &str) -> InstrumentId {
        let atom = DefaultAtom::from(symbol);  // Interned string
        *self.cache.entry(atom.clone())
            .or_insert_with(|| create_instrument_id(&atom))
    }
}
```

## Error Handling

### Never Hide Failures
```rust
// ❌ WRONG: Silently ignoring errors
match send_message(msg) {
    Ok(_) => {},
    Err(_) => {},  // Silent failure!
}

// ✅ CORRECT: Propagate or log with context
match send_message(msg) {
    Ok(_) => {},
    Err(e) => {
        error!("Failed to send message: {}", e);
        metrics.increment_send_failures();
        return Err(Error::MessageSendFailed(e));
    }
}
```

### Error Context Pattern
```rust
use anyhow::{Context, Result};

// ✅ CORRECT: Rich error context
pub fn process_trade(data: &[u8]) -> Result<Trade> {
    let header = parse_header(data)
        .context("Failed to parse message header")?;
        
    let tlv = parse_tlv(&data[32..])
        .with_context(|| format!("Failed to parse TLV for sequence {}", header.sequence))?;
        
    validate_trade(&tlv)
        .context("Trade validation failed")?;
        
    Ok(tlv.into())
}
```

## Testing Requirements

### Real Data Only - No Mocks
```rust
// ❌ WRONG: Mock exchange connection
#[test]
fn test_with_mock() {
    let mock_exchange = MockExchange::new();
    mock_exchange.set_response(/* fake data */);
}

// ✅ CORRECT: Real exchange connection
#[test]
fn test_with_real_exchange() {
    let exchange = connect_to_kraken_sandbox().await?;
    let real_trade = exchange.get_latest_trade("BTC/USD").await?;
    assert!(real_trade.price > 0);
}
```

### Performance Benchmarks
```rust
// All performance-critical code must have benchmarks
#[bench]
fn bench_tlv_parsing(b: &mut Bencher) {
    let data = create_test_message();
    b.iter(|| {
        parse_tlv_message(&data).unwrap()
    });
    
    // Must maintain: <35μs for hot path
    assert!(b.ns_per_iter() < 35_000);
}
```

## Monitoring & Observability

### Metrics for Critical Paths
```rust
// Track performance at key points
pub fn process_message(msg: &[u8]) -> Result<()> {
    let start = Instant::now();
    
    let parsed = parse_message(msg)?;
    metrics.record_parse_time(start.elapsed());
    
    let validated = validate_message(parsed)?;
    metrics.record_validation_time(start.elapsed());
    
    relay.send(validated)?;
    metrics.record_total_latency(start.elapsed());
    
    Ok(())
}
```

### Health Checks
```rust
// Every service must implement health checks
impl HealthCheck for ExchangeCollector {
    fn is_healthy(&self) -> bool {
        self.websocket.is_connected() &&
        self.last_message_time.elapsed() < Duration::from_secs(30) &&
        self.error_rate() < 0.01
    }
}
```

## Migration Patterns

### Symbol to InstrumentId Migration
```rust
// During migration, support both patterns
pub enum Identifier {
    Legacy(String),      // Old: "BTC/USD"
    Modern(InstrumentId), // New: Bijective ID
}

impl From<String> for Identifier {
    fn from(s: String) -> Self {
        // Gradual migration support
        if let Ok(id) = InstrumentId::parse(&s) {
            Identifier::Modern(id)
        } else {
            Identifier::Legacy(s)
        }
    }
}
```

## Code Review Checklist

### Performance Review
- [ ] Zero-copy serialization for TLV messages
- [ ] No allocations in hot path (<35μs)
- [ ] Pre-allocated buffers for high-frequency operations
- [ ] Benchmarks for critical paths

### Safety Review  
- [ ] Precision preserved (no float arithmetic for money)
- [ ] Error handling with context
- [ ] No silent failures
- [ ] Resource cleanup on all paths

### Architecture Review
- [ ] Correct relay domain (MarketData/Signal/Execution)
- [ ] TLV type in correct range
- [ ] InstrumentId bijection maintained
- [ ] Service boundaries respected

### Testing Review
- [ ] Real data tests (no mocks)
- [ ] Performance benchmarks included
- [ ] Property-based tests for invariants
- [ ] Integration tests across services

## Summary

These Torq-specific practices ensure:
1. **Performance**: >1M msg/s with <35μs hot path latency
2. **Precision**: Zero loss in financial calculations
3. **Safety**: No silent failures, comprehensive validation
4. **Maintainability**: Clear service boundaries and migration paths
5. **Reliability**: Real data testing and continuous monitoring

Remember: We're handling real money at microsecond speeds. Every decision matters.
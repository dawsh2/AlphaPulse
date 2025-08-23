# Standard Operating Procedure for Adapter Development

## Overview
This document defines the standard workflow for developing new adapters or adding new data types to existing adapters in the AlphaPulse system.

## Core Principles
1. **Use real data as the source of truth. Never use mocks or simulated data.**
2. **Every data type from every adapter MUST complete the full validation pipeline before production use.**
3. **Validation is a development-time requirement, not a runtime process.**
4. **Correctness over performance - we validate everything during development, then optimize for production.**
5. **Forward ALL provider data - no artificial constraints or "reasonable bounds" filtering.**
6. **Validation detects corruption only - not business logic violations or data filtering.**

## Step-by-Step Workflow

### Step 1: Create Adapter Directory Structure
**Objective**: Set up proper directory structure following established patterns.

**Actions**:
1. **Create adapter subdirectory**: `mkdir -p src/input/collectors/[venue]/`  
2. **Create README.md template**: Use existing adapters (binance/, kraken/, polygon_dex/) as examples
3. **Include official documentation links**: Direct links to venue's API documentation
4. **Set up test fixture directory**: `mkdir -p tests/fixtures/[venue]/`
5. **Update mod.rs**: Add new collector module to `src/input/collectors/mod.rs`

**Example**:
```bash
mkdir -p src/input/collectors/coinbase/
cp src/input/collectors/binance/README.md src/input/collectors/coinbase/README.md
# Edit coinbase/README.md with venue-specific information
```

**Deliverable**: Directory structure with placeholder README.md referencing official documentation

### Step 2: Obtain Real Data Sample
**Objective**: Capture actual data from the provider for the specific object type you need to ingest.

**Implementation**: Create a quick data capture script (temporary, not production code).

**Actions**:
1. **Write a simple script** to connect to the data provider (WebSocket, REST API, RPC, etc.)
2. **Subscribe to target data streams** and capture multiple samples
3. **Save raw samples** in appropriate format:
   - **CEX WebSocket/REST**: JSON format for message samples
   - **Blockchain/DEX**: Raw transaction logs, ABI-decoded event structures, or binary data
   - **Traditional APIs**: Native format (FIX messages, binary protocols, etc.)
4. **Identify edge cases** (nulls, maximums, unusual values, malformed data)
5. **Document unusual behaviors** or provider-specific quirks

**Script Examples**:
- **Coinbase WebSocket**: `scripts/capture_coinbase_data.js` or `capture_coinbase_data.py`
- **Uniswap on Polygon**: `scripts/capture_polygon_swaps.rs` using Web3 RPC
- **Traditional API**: `scripts/capture_[venue]_data.py` using REST calls

**Sample Data Capture Script Pattern**:
```bash
# Create temporary capture script
mkdir -p scripts/temp
# Run capture for 5-10 minutes to get diverse samples
python scripts/temp/capture_coinbase_trades.py --duration 300 --output tests/fixtures/coinbase/
# Review and clean samples, remove any sensitive data
```

**Deliverable**: Raw data samples saved in `tests/fixtures/[venue]/[data_type]_samples.json`

**Note**: These capture scripts are temporary utilities - delete them after capturing sufficient samples.

### Step 2: Validate Against Official Documentation
**Objective**: Ensure your understanding matches the official specification and handle any undocumented behaviors.

**Actions**:
1. Locate official documentation for the data format
2. Map each field in your sample to the documentation
3. Identify any discrepancies or undocumented fields
4. Note precision/scale for numeric fields
5. Document any provider-specific quirks

**Documentation Sources**:
- **CEX APIs**: Official API documentation (REST and WebSocket)
- **DEX Events**: Ethereum ABI specifications, contract source code
- **Traditional Markets**: FIX protocol specs, vendor documentation

**Validation Checklist**:
- [ ] All fields in sample are documented
- [ ] Data types match specification
- [ ] Numeric precision is understood
- [ ] Null/optional fields identified
- [ ] Timestamp format confirmed
- [ ] Sequence/nonce fields identified

**Deliverable**: Annotated data structure in adapter code comments

### Step 3: Implement Parser and Semantic Mapping
**Objective**: Transform provider data into AlphaPulse TLV format while preserving semantic meaning.

**Parser Implementation**:
```rust
// 1. Define provider-specific structure matching their format exactly
#[derive(Deserialize)]
struct UniswapV3SwapEvent {
    sender: Address,
    recipient: Address,
    amount0: U256,  // Note: signed in actual event
    amount1: U256,  // Note: signed in actual event
    sqrtPriceX96: U256,
    liquidity: U128,
    tick: i32,
}

// 2. Implement semantic validation
impl UniswapV3SwapEvent {
    fn validate(&self) -> Result<()> {
        // Ensure exactly one amount is positive (in) and one negative (out)
        // Check tick is within valid range
        // Verify sqrtPriceX96 is reasonable
    }
}

// 3. Map to TLV with semantic preservation
impl From<UniswapV3SwapEvent> for PoolSwapTLV {
    fn from(event: UniswapV3SwapEvent) -> Self {
        // Semantic mapping:
        // - Negative amount → amount_out (make positive)
        // - Positive amount → amount_in
        // - Preserve full precision (u128)
        // - Map venue correctly (Polygon, not Uniswap)
    }
}
```

**Semantic Mapping Rules**:
1. **Preserve Precision**: Never truncate or round
2. **Maintain Semantics**: amount_in vs amount_out, bid vs ask
3. **Handle Signs**: DEX signed amounts → unsigned with direction flag
4. **Venue Mapping**: Map to correct VenueId enum value
5. **Timestamp Normalization**: Convert to nanoseconds since epoch
6. **Decimal Tracking**: Store decimal places separately from amounts

**Critical Validations**:
- Amount overflow checks (can value fit in target type?)
- Timestamp reasonableness (not in future, not too old)
- Required fields present
- Cross-field validation (e.g., token0 != token1)

**See**: `adapters/VALIDATION.md` for validation procedures

### Step 3.5: Implement Stateless Adapter Structure
**Objective**: Create adapter as stateless data transformer following clean architecture.

**CRITICAL**: Adapters should be pure data transformers, not state managers.

**Adapter Responsibilities (✅ DO)**:
- Parse raw provider data (JSON, binary, ABI)
- Convert to TLV messages with semantic preservation
- Handle connection lifecycle (connect, reconnect, disconnect)
- Monitor metrics (messages processed, errors, latency)
- Forward TLV messages to output channel

**NOT Adapter Responsibilities (❌ DON'T)**:
- Track which instruments are being monitored (→ Relay)
- Manage application state or caching (→ Consumer)
- Handle state invalidation logic (→ Consumer)
- Make business decisions about data validity (→ Consumer)

**Stateless Adapter Pattern**:
```rust
pub struct VenueCollector {
    // ✅ ALLOWED: Resource management
    connection: Arc<ConnectionManager>,
    metrics: Arc<AdapterMetrics>,
    rate_limiter: RateLimiter,
    
    // ✅ ALLOWED: Data transformation
    output_tx: mpsc::Sender<TLVMessage>,
    symbol_map: Arc<RwLock<HashMap<String, InstrumentId>>>,
    
    // ❌ NOT ALLOWED: State management  
    // state: Arc<StateManager>,              // → Belongs in Relay
    // invalidation_logic: InvalidationHandler, // → Belongs in Consumer
    // subscription_tracker: SubscriptionManager, // → Belongs in Relay
}

impl VenueCollector {
    // Simple constructor - no complex state initialization
    pub fn new(products: Vec<String>, output_tx: mpsc::Sender<TLVMessage>) -> Self {
        let metrics = Arc::new(AdapterMetrics::new());
        let config = ConnectionConfig { /* ... */ };
        
        Self {
            connection: Arc::new(ConnectionManager::new(venue, config, metrics.clone())),
            metrics,
            output_tx,
            // Clean: Focus on data transformation only
        }
    }
    
    // Pure function: Raw Data → TLV Message
    async fn process_message(&self, raw: &str) -> Result<Option<TLVMessage>> {
        // Parse, validate, convert - no state management
    }
}
```

**Architecture Flow**:
```
Provider → Adapter → Relay → Consumer
JSON     → TLV     → Route → Business Logic
```

### Step 4: Complete Validation Pipeline (MANDATORY)
**Objective**: Every data type MUST pass the full four-step validation pipeline before production use.

**CRITICAL**: This is not optional. No data type can be used in production without completing this validation.

**Validation Pipeline**:
1. **Raw Data Parsing Validation** - Ensure provider data is parsed correctly
2. **TLV Serialization Validation** - Ensure semantic mapping is correct  
3. **TLV Deserialization Validation** - Ensure no corruption in binary format
4. **Deep Equality Validation** - Ensure perfect roundtrip with zero data loss

**Implementation** (see `VALIDATION.md` for detailed procedures):
```rust
#[test]
fn test_[venue]_[data_type]_complete_validation() {
    // Load multiple real data samples
    let real_samples = load_fixtures("fixtures/[venue]/[data_type]_real.json");
    
    for sample in real_samples {
        // Run complete four-step validation pipeline
        let result = complete_validation_pipeline(&sample);
        assert!(result.is_ok(), "Validation pipeline failed for sample: {:?}", sample);
        
        // Additional semantic validation
        validate_semantic_correctness(&sample, &result.unwrap())?;
    }
}
```

**Validation Requirements**:
- [ ] **MANDATORY**: All four validation steps pass for every data type
- [ ] Normal case (typical values)
- [ ] Edge cases (maximum values, zeros, boundary conditions)
- [ ] Error cases (malformed data handled gracefully)
- [ ] Precision preservation (no data loss detected)
- [ ] Semantic correctness (fields mapped correctly)
- [ ] Deep equality (perfect serialization roundtrip)
- [ ] Cross-validation (multiple data sources agree when available)

**Performance Note**: Validation performance is irrelevant during development. Focus on correctness.

**CRITICAL VALIDATION PRINCIPLES**:

**What Validation DOES:**
- ✅ Detect JSON/data parsing corruption and errors
- ✅ Verify semantic correctness during parsing (parsed field == original field)  
- ✅ Ensure precision preservation through proper type usage
- ✅ Validate perfect serialization roundtrip with zero data loss
- ✅ Check structural integrity after deserialization
- ✅ Verify TLV format compliance and protocol adherence

**What Validation DOES NOT DO:**
- ❌ **Never apply "reasonable bounds"** (e.g., max $1M BTC price, max 10,000 BTC volume)
- ❌ **Never filter by recency** or time windows
- ❌ **Never apply market cap constraints** or volume limits
- ❌ **Never modify or normalize** provider data beyond format conversion
- ❌ **Never apply business logic** or trading constraints
- ❌ **Never reject data** based on unusual but valid values

**Collectors are data forwarders, not data filters. They must pass through ALL provider data unchanged.**

## System Quality Philosophy

**CRITICAL: Never bypass deeper architectural issues to complete local tasks.**

When compilation errors or architectural issues arise:
1. **Address the root cause** - Fix underlying system problems
2. **No workarounds or bypasses** - Don't create isolated tests to avoid compilation errors
3. **System-level thinking** - Consider impact on entire codebase, not just immediate task
4. **Quality over speed** - Take time to fix foundational issues properly

**The global goal is always producing high-quality, system-level code. Local task completion must never blind us to this objective.**

### Core API Modification Policy

**CRITICAL: DO NOT modify core code without explicit developer approval.**

When implementing adapters:
- **Use existing APIs as designed** - Learn correct usage patterns from other adapters
- **If APIs are insufficient** - HALT and request developer guidance before proceeding
- **No breaking changes** - Adapters must work with existing protocol_v2, error types, and shared interfaces
- **Study existing examples** - Look at binance.rs, polygon_dex.rs for correct API usage patterns

**If you encounter API issues: STOP and document the problem rather than modifying core code.**

### Step 5: Integration Testing
**Objective**: Ensure adapter works correctly in the full system pipeline.

**Integration Points**:
1. **Inbound Flow**: Provider → Parser → TLV → Relay → Consumer
2. **Outbound Flow**: Strategy → TLV → Transformer → Provider API

**End-to-End Test**:
```rust
#[test]
async fn test_e2e_polygon_swap_flow() {
    // 1. Start relay
    let relay = MarketDataRelay::start().await;
    
    // 2. Start adapter
    let adapter = PolygonDexAdapter::new(config);
    adapter.start_inbound(relay.clone()).await;
    
    // 3. Wait for real event
    let consumer = relay.subscribe();
    let tlv_msg = consumer.receive().await?;
    
    // 4. Validate received message
    let swap = PoolSwapTLV::from_bytes(&tlv_msg.payload)?;
    assert!(swap.amount_in > 0);
    
    // 5. Compare with direct query
    let direct = query_polygon_rpc(swap.block_number).await?;
    assert_eq!(swap, direct);
}
```

### Step 6: Production Performance Optimization
**Objective**: Optimize for production runtime performance after validation is complete.

**Important**: Performance optimization only begins AFTER validation pipeline is complete for all data types.

**Production Performance Targets**:
- Message processing latency: Achieve best possible performance for your system
- Memory usage: Minimize footprint
- Throughput: Maximize based on available resources
- Stability: 24-hour stability test required

**Performance Note**: Production code does NOT run validation pipeline - validation is development-time only.

### Step 7: Documentation
**Objective**: Document adapter for future maintenance.

**Required Documentation**:
1. **README.md in adapter directory** with:
   - Authoritative links to official data format documentation
   - Validation checklist for this venue
   - Test code locations and coverage
   - Performance characteristics and benchmarks
2. Data format specification (link to official docs)
3. Semantic mapping decisions
4. Known limitations or quirks
5. Test data location

**Adapter README Template**:
```markdown
# [Venue Name] Adapter

## Official Data Format Documentation
- **WebSocket API**: [Direct link to venue's WebSocket docs]
- **REST API**: [Direct link to venue's REST API docs]
- **ABI Specification**: [For DEX adapters - link to contract ABI]
- **Message Format**: [Link to message format specification]

## Validation Checklist
- [ ] Raw data parsing validation implemented
- [ ] TLV serialization validation implemented  
- [ ] TLV deserialization validation implemented
- [ ] Semantic & deep equality validation implemented
- [ ] Performance targets met (<10ms per validation)
- [ ] Real data fixtures created (no mocks)

## Test Coverage
- **Unit Tests**: `tests/validation/[venue].rs` 
- **Integration Tests**: `tests/integration/[venue].rs`
- **Real Data Fixtures**: `tests/fixtures/[venue]/`
- **Performance Tests**: Included in validation tests

## Performance Characteristics
- Validation Speed: [X]ms per event
- Throughput: [X] events/second
- Memory Usage: [X]MB baseline
```

## Common Patterns by Venue Type

### CEX (Centralized Exchange)
1. WebSocket for market data (trades, quotes, orderbook)
2. REST API for execution
3. HMAC authentication
4. Rate limiting critical
5. Heartbeat/ping handling required

### DEX (Decentralized Exchange)
1. Event log monitoring via Web3
2. ABI-based decoding
3. Handle blockchain reorgs
4. Native token precision (18, 6, etc. decimals)
5. Gas estimation for execution

### Traditional Markets (Stocks, Futures)
1. FIX protocol or vendor-specific
2. Session management
3. Market hours handling
4. Corporate actions affecting symbols

## Troubleshooting Guide

### Problem: Precision Loss
**Symptom**: Values don't match after roundtrip
**Solution**: Use larger types (u128), preserve decimal places separately

### Problem: Semantic Confusion
**Symptom**: Bid/ask swapped, amount_in/out confused
**Solution**: Add explicit semantic validation, document mapping

### Problem: Overflow
**Symptom**: Large values cause panic or wrap
**Solution**: Use u128 for blockchain amounts, check bounds before casting

### Problem: Missing Fields
**Symptom**: Parser fails on real data
**Solution**: Make fields optional, handle provider variations

## Quality Checklist

Before marking adapter complete:
- [ ] **MANDATORY**: Full validation pipeline completed for ALL data types
- [ ] Tested with >1000 real messages per data type
- [ ] No data loss (deep equality passes for all samples)
- [ ] Semantic correctness validated for all edge cases
- [ ] Production performance optimized (validation pipeline disabled)
- [ ] 24-hour stability test in production configuration
- [ ] Cross-validation with alternative source when available
- [ ] Documentation complete (references VALIDATION.md procedures)
- [ ] Code reviewed

**CRITICAL**: No data type can be used in production without completing the validation pipeline.

## References
- `VALIDATION.md` - Detailed validation procedures
- `STRUCTURE.md` - Adapter module organization
- Protocol V2 Specification - `protocol_v2/README.md`
- Real test data - `tests/fixtures/`
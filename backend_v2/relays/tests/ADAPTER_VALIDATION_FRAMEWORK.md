# Adapter Validation Framework - Standardized Protocol for Exchange Integration

## Problem Statement

When adding new exchange adapters, we face critical risks:
1. **Semantic Misinterpretation**: Parsing 'fees' as 'profit', 'volume' as 'price', etc.
2. **Field Mapping Errors**: Wrong JSON fields mapped to protocol fields
3. **Data Type Confusion**: Strings parsed as numbers, decimals as integers
4. **Deep Equality False Positives**: Binary equality passes while semantic meaning is wrong
5. **Human Validation Dependency**: Manual verification doesn't scale

## Solution: Multi-Layer Validation Framework

### Layer 1: Exchange Schema Definition (AUTOMATED)

**For each exchange/event type, create a schema definition:**

```rust
struct ExchangeSchema {
    exchange_name: &'static str,
    event_type: &'static str,
    api_documentation_url: &'static str,
    
    // JSON structure validation
    required_fields: Vec<FieldDefinition>,
    optional_fields: Vec<FieldDefinition>,
    
    // Semantic validation rules
    business_logic: Vec<BusinessRule>,
    
    // Cross-field dependencies
    field_relationships: Vec<FieldRelationship>,
}

struct FieldDefinition {
    json_path: &'static str,          // "data.amount0" or "topics[1]"
    field_name: &'static str,         // "amount0"
    description: &'static str,        // "Change in token0 balance"
    data_type: DataType,              // Int256, Uint128, Address, etc.
    semantic_meaning: SemanticType,   // TokenAmount, Price, Fee, Volume, etc.
    validation_rules: Vec<ValidationRule>,
}

enum SemanticType {
    TokenAmount,          // Amount of tokens
    Price,               // Price/rate between tokens
    Fee,                 // Transaction fee
    Volume,              // Trading volume  
    Timestamp,           // Time data
    Address,             // Wallet/contract address
    BlockNumber,         // Blockchain block number
    TransactionHash,     // Transaction identifier
    PoolIdentifier,      // Liquidity pool ID
    Tick,               // Price tick (Uniswap V3)
    Liquidity,          // Available liquidity
}
```

### Layer 2: Protocol Mapping Validation (SEMI-AUTOMATED)

**Define explicit mappings from exchange fields to protocol fields:**

```rust
struct ProtocolMapping {
    source_exchange: &'static str,
    source_event: &'static str,
    
    mappings: Vec<FieldMapping>,
    
    // Validation logic
    mapping_validator: fn(&ExchangeData, &ProtocolData) -> ValidationResult,
}

struct FieldMapping {
    source_field: &'static str,       // "amount0" from exchange
    protocol_field: &'static str,     // "amount_in" in protocol
    transformation: TransformationType,
    
    // Semantic validation
    semantic_check: fn(&ExchangeFieldValue, &ProtocolFieldValue) -> bool,
    
    // Documentation of mapping logic
    mapping_rationale: &'static str,
}

enum TransformationType {
    Direct,                           // 1:1 mapping
    AbsoluteValue,                   // Take absolute value
    ConditionalSelection(Condition), // If amount0 < 0 then use amount0, else use amount1
    Mathematical(MathOp),            // Apply mathematical transformation
    Lookup(LookupTable),             // Translate using lookup table
}
```

### Layer 3: Automated Test Suite Generation (AUTOMATED)

```rust
// Generate comprehensive test cases from schema
fn generate_test_suite(schema: &ExchangeSchema, mapping: &ProtocolMapping) -> TestSuite {
    TestSuite {
        // Valid data tests
        happy_path_tests: generate_valid_cases(schema),
        
        // Edge case tests  
        boundary_tests: generate_boundary_cases(schema),
        
        // Error condition tests
        invalid_data_tests: generate_error_cases(schema),
        
        // Semantic validation tests
        semantic_tests: generate_semantic_tests(schema, mapping),
        
        // Roundtrip tests
        roundtrip_tests: generate_roundtrip_tests(mapping),
        
        // Regression tests from real data
        live_data_tests: generate_from_live_data(schema),
    }
}
```

### Layer 4: Human Validation Gateway (NON-AUTOMATABLE)

**This is the critical manual step that cannot be automated:**

```markdown
## Human Validation Checklist for New Exchange Adapters

### Pre-Implementation Validation (REQUIRED)

1. **Exchange API Documentation Review**
   - [ ] Read complete API documentation for target exchange
   - [ ] Document all relevant event types and their JSON structure
   - [ ] Identify semantic meaning of each field
   - [ ] Note any unusual conventions or edge cases
   - [ ] Cross-reference with exchange's open source implementations

2. **Sample Data Collection**
   - [ ] Collect 50+ real examples of each event type from exchange
   - [ ] Include edge cases: large numbers, negative values, zero values
   - [ ] Document the business context of each sample
   - [ ] Verify samples represent different market conditions

3. **Semantic Field Analysis**
   - [ ] For each JSON field, document:
     - [ ] Exact semantic meaning
     - [ ] Data type and format
     - [ ] Possible value ranges
     - [ ] Units (wei, USD, percentage, etc.)
     - [ ] Sign conventions (negative = sell?)
   
4. **Protocol Mapping Design**
   - [ ] Map each exchange field to protocol field
   - [ ] Document transformation logic
   - [ ] Identify conditional mappings
   - [ ] Validate no semantic confusion (fees ≠ profit)

5. **Cross-Validation with Exchange Team**
   - [ ] If possible, verify interpretation with exchange developers
   - [ ] Check against exchange's own documentation/examples
   - [ ] Validate edge case handling

### Implementation Validation (REQUIRED)

6. **Schema Definition Accuracy**
   - [ ] Schema matches collected samples exactly
   - [ ] All required fields identified
   - [ ] Optional fields properly handled
   - [ ] Validation rules catch known edge cases

7. **Mapping Logic Verification**
   - [ ] Each mapping preserves semantic meaning
   - [ ] No field confusion (amount vs fee vs price)
   - [ ] Transformation logic mathematically correct
   - [ ] Edge cases handled properly

8. **Test Data Validation**
   - [ ] Generated tests cover all collected samples
   - [ ] Manual verification of test case correctness
   - [ ] Edge cases properly represented
   - [ ] Error conditions properly tested

### Post-Implementation Validation (REQUIRED)

9. **Live Data Verification**
   - [ ] Run adapter against live exchange data
   - [ ] Manually verify first 100 conversions
   - [ ] Check output makes business sense
   - [ ] Validate against known market events

10. **Regression Testing**
    - [ ] All generated tests pass
    - [ ] No regressions in existing adapters
    - [ ] Performance within acceptable bounds
    - [ ] Memory usage reasonable
```

## Implementation Guide for Other Agents

### Step 1: Create Exchange Schema (AUTOMATED)

```bash
# Agent should analyze exchange API documentation and create:
backend_v2/adapters/schemas/[exchange_name]_[event_type].rs

# Example:
backend_v2/adapters/schemas/binance_trade.rs
backend_v2/adapters/schemas/coinbase_orderbook.rs
backend_v2/adapters/schemas/uniswap_v3_swap.rs
```

### Step 2: Collect Live Sample Data (AUTOMATED)

```bash
# Agent should collect real examples:
backend_v2/adapters/test_data/[exchange_name]/
  ├── live_samples/
  │   ├── trade_events_100_samples.json
  │   ├── orderbook_updates_50_samples.json
  │   └── metadata.json  # Business context for each sample
  ├── edge_cases/
  │   ├── zero_amounts.json
  │   ├── large_numbers.json
  │   └── error_conditions.json
  └── validation_notes.md  # Human analysis notes
```

### Step 3: Generate Test Suite (AUTOMATED)

```rust
// Agent should generate comprehensive tests:
#[test]
fn test_binance_trade_happy_path() {
    let sample_data = load_sample("binance_trade_sample_1.json");
    let result = parse_binance_trade(&sample_data);
    
    // Validate schema compliance
    assert_schema_valid(&result, &BINANCE_TRADE_SCHEMA);
    
    // Validate semantic correctness
    assert_semantic_valid(&result);
    
    // Validate protocol mapping
    let protocol_data = convert_to_protocol(&result);
    assert_mapping_correct(&result, &protocol_data);
}
```

### Step 4: Human Validation Required (NON-AUTOMATABLE)

**Critical: This step cannot be skipped or automated**

- Human expert must review all schemas, mappings, and test cases
- Verify semantic correctness against exchange documentation
- Check for field confusion (fees vs profit vs volume)
- Validate business logic makes sense
- Sign off on adapter before production deployment

### Step 5: Continuous Validation (AUTOMATED)

```rust
// Ongoing validation against live data
fn validate_adapter_continuously(adapter: &ExchangeAdapter) {
    // Daily validation against live exchange data
    let live_samples = collect_live_data(adapter.exchange_name, 100);
    
    for sample in live_samples {
        let result = adapter.parse(&sample);
        
        // Validate against schema
        assert_schema_compliance(&result, &adapter.schema);
        
        // Validate semantic reasonableness
        assert_business_logic_valid(&result);
        
        // Check for anomalies
        detect_anomalies(&result, &historical_patterns);
    }
}
```

## File Structure for Agent Implementation

```
backend_v2/adapters/
├── framework/
│   ├── schema.rs              # Schema definition types
│   ├── validation.rs          # Validation framework
│   ├── testing.rs             # Test generation
│   └── mapping.rs             # Protocol mapping types
├── schemas/
│   ├── kraken_trade.rs        # Kraken-specific schemas
│   ├── polygon_swap.rs        # Polygon/Uniswap schemas
│   └── ...                    # One file per exchange/event type
├── mappings/
│   ├── kraken_to_protocol.rs  # Kraken → Protocol mappings
│   ├── polygon_to_protocol.rs # Polygon → Protocol mappings
│   └── ...                    # One file per exchange
├── test_data/
│   ├── kraken/                # Live samples from Kraken
│   ├── polygon/               # Live samples from Polygon
│   └── ...                    # One directory per exchange
└── validation/
    ├── human_checklist.md     # Manual validation requirements
    ├── test_generator.rs      # Automated test generation
    └── continuous_validator.rs # Live validation system
```

## Agent Handoff Instructions

### For Schema Creation Agent:
1. Read exchange API documentation
2. Collect 100+ live data samples
3. Create schema definitions with semantic types
4. Generate basic validation rules
5. **STOP**: Hand off to human validator for semantic review

### For Mapping Creation Agent:
1. Take validated schemas from previous step
2. Create explicit field mappings to protocol
3. Document transformation logic
4. Generate mapping validation tests
5. **STOP**: Hand off to human validator for semantic review

### For Test Generation Agent:
1. Take validated schemas and mappings
2. Generate comprehensive test suites
3. Include edge cases and error conditions
4. Create continuous validation framework
5. **STOP**: Hand off to human validator for test case review

### For Integration Agent:
1. Take all validated components
2. Integrate into main codebase
3. Run full test suite
4. Deploy with monitoring
5. Set up continuous validation

## Success Criteria

- **Zero semantic confusion**: No field misinterpretation
- **100% test coverage**: All paths tested automatically
- **Human validation**: Expert sign-off on semantic correctness
- **Live data validation**: Continuous verification against real data
- **Documentation**: Complete rationale for all mapping decisions
- **Maintainability**: Clear process for adding new exchanges

This framework ensures that adapter creation is both rigorous and scalable, with clear separation between automatable and human-required validation steps.
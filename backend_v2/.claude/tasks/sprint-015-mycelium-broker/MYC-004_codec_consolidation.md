# MYC-004: Codec Consolidation

## Status
- **Status**: pending
- **Assignee**: TBD
- **Estimated Effort**: 2 days
- **Priority**: High (enables clean broker integration)

## Description
Consolidate all TLV parsing, building, and validation logic from the relay infrastructure into the alphapulse_codec library. This eliminates duplicate code, centralizes protocol handling, and ensures the broker can work with clean, validated messages without domain-specific logic.

## Objectives
1. Move all TLV validation logic from relays/ to alphapulse_codec
2. Remove duplicate parsing/building implementations
3. Consolidate message header handling and validation
4. Ensure zero-copy operations are preserved in consolidated code
5. Create unified API for producer and consumer services

## Technical Approach

### Current State Analysis
```bash
# Identify duplicate TLV code across codebase
libs/alphapulse_codec/src/
├── parser.rs              # Basic TLV parsing
├── message_builder.rs     # Message construction
└── tlv_types.rs          # Type definitions

relays/src/
├── market_data/parser.rs  # Domain-specific validation (DUPLICATE)
├── signal/parser.rs       # Domain-specific validation (DUPLICATE)
├── execution/parser.rs    # Domain-specific validation (DUPLICATE)
└── shared/validation.rs   # Shared validation logic (SHOULD BE IN CODEC)
```

### Consolidation Strategy
```rust
// libs/alphapulse_codec/src/validator.rs - NEW FILE
pub struct TLVValidator {
    domain_rules: HashMap<RelayDomain, DomainValidationRules>,
    type_registry: TLVTypeRegistry,
}

impl TLVValidator {
    pub fn validate_message(&self, header: &MessageHeader, payload: &[u8]) -> Result<ValidatedMessage, ValidationError> {
        // Consolidate validation logic from all relays
        self.validate_header(header)?;
        self.validate_payload_structure(payload)?;
        self.validate_domain_rules(header.relay_domain, payload)?;
        
        Ok(ValidatedMessage {
            header: *header,
            tlv_extensions: self.parse_and_validate_tlvs(payload)?,
        })
    }

    fn validate_domain_rules(&self, domain: RelayDomain, payload: &[u8]) -> Result<(), ValidationError> {
        let rules = self.domain_rules.get(&domain)
            .ok_or(ValidationError::UnsupportedDomain(domain))?;
            
        // Apply domain-specific validation rules
        match domain {
            RelayDomain::MarketData => {
                self.validate_market_data_rules(payload, rules)?;
            }
            RelayDomain::Signal => {
                self.validate_signal_rules(payload, rules)?;
            }
            RelayDomain::Execution => {
                self.validate_execution_rules(payload, rules)?;
            }
        }
        
        Ok(())
    }
}
```

### Enhanced Message Builder
```rust
// libs/alphapulse_codec/src/message_builder.rs - ENHANCED
pub struct TLVMessageBuilder {
    domain: RelayDomain,
    source: MessageSource,
    sequence: u64,
    timestamp: u64,
    tlv_buffer: Vec<u8>,
    validator: TLVValidator,
}

impl TLVMessageBuilder {
    pub fn new(domain: RelayDomain, source: MessageSource) -> Self {
        Self {
            domain,
            source,
            sequence: 0,
            timestamp: SystemClock::now_nanos(),
            tlv_buffer: Vec::with_capacity(DEFAULT_TLV_CAPACITY),
            validator: TLVValidator::for_domain(domain),
        }
    }

    pub fn add_tlv<T>(&mut self, tlv_type: TLVType, data: &T) -> Result<&mut Self, BuilderError> 
    where
        T: Serialize + TLVValidate,
    {
        // Validate TLV data before adding
        data.validate_for_domain(self.domain)?;
        
        // Serialize with zero-copy optimization
        let serialized = self.serialize_tlv(tlv_type, data)?;
        
        // Validate TLV structure
        self.validator.validate_tlv(tlv_type, &serialized)?;
        
        self.tlv_buffer.extend_from_slice(&serialized);
        Ok(self)
    }

    pub fn build(mut self) -> Result<Vec<u8>, BuilderError> {
        // Final validation before building message
        self.validator.validate_message_structure(&self.tlv_buffer)?;
        
        let header = MessageHeader {
            magic: MESSAGE_MAGIC,
            version: PROTOCOL_VERSION,
            relay_domain: self.domain,
            source: self.source,
            sequence: self.sequence,
            timestamp: self.timestamp,
            payload_size: self.tlv_buffer.len() as u32,
            checksum: self.calculate_checksum(&self.tlv_buffer),
            reserved: [0; 8],
        };

        let mut message = Vec::with_capacity(HEADER_SIZE + self.tlv_buffer.len());
        message.extend_from_slice(&header.as_bytes());
        message.extend_from_slice(&self.tlv_buffer);

        Ok(message)
    }
}
```

### Unified Parser Interface
```rust
// libs/alphapulse_codec/src/parser.rs - ENHANCED
pub struct TLVParser {
    validator: TLVValidator,
    zero_copy_enabled: bool,
}

impl TLVParser {
    pub fn new() -> Self {
        Self {
            validator: TLVValidator::default(),
            zero_copy_enabled: true,
        }
    }

    pub fn parse_message(&self, data: &[u8]) -> Result<ValidatedMessage, ParseError> {
        // Parse header
        if data.len() < HEADER_SIZE {
            return Err(ParseError::MessageTooShort);
        }

        let header = MessageHeader::from_bytes(&data[..HEADER_SIZE])?;
        let payload = &data[HEADER_SIZE..HEADER_SIZE + header.payload_size as usize];

        // Validate and parse using consolidated logic
        self.validator.validate_message(&header, payload)
    }

    pub fn parse_tlv_payload(&self, payload: &[u8]) -> Result<Vec<TLVExtension>, ParseError> {
        let mut tlvs = Vec::new();
        let mut offset = 0;

        while offset < payload.len() {
            let tlv_header = TLVHeader::from_bytes(&payload[offset..offset + 4])?;
            offset += 4;

            if offset + tlv_header.length as usize > payload.len() {
                return Err(ParseError::TruncatedTLV);
            }

            let tlv_data = if self.zero_copy_enabled {
                // Zero-copy reference to original buffer
                &payload[offset..offset + tlv_header.length as usize]
            } else {
                // Copy data if zero-copy not available
                payload[offset..offset + tlv_header.length as usize].to_vec().as_slice()
            };

            tlvs.push(TLVExtension {
                header: tlv_header,
                data: tlv_data.to_vec(), // TODO: Support zero-copy with lifetimes
            });

            offset += tlv_header.length as usize;
        }

        Ok(tlvs)
    }
}
```

### Domain-Specific Validation Consolidation
```rust
// libs/alphapulse_codec/src/validation/mod.rs - NEW MODULE
pub mod market_data;
pub mod signal;
pub mod execution;

pub trait DomainValidator {
    fn validate_tlv(&self, tlv_type: TLVType, data: &[u8]) -> Result<(), ValidationError>;
    fn validate_message_structure(&self, tlvs: &[TLVExtension]) -> Result<(), ValidationError>;
    fn get_allowed_types(&self) -> &[TLVType];
}

// libs/alphapulse_codec/src/validation/market_data.rs
pub struct MarketDataValidator;

impl DomainValidator for MarketDataValidator {
    fn validate_tlv(&self, tlv_type: TLVType, data: &[u8]) -> Result<(), ValidationError> {
        // Consolidated from relays/src/market_data/parser.rs
        match tlv_type {
            TLVType::Trade => self.validate_trade_tlv(data),
            TLVType::Quote => self.validate_quote_tlv(data),
            TLVType::OrderBook => self.validate_orderbook_tlv(data),
            _ => Err(ValidationError::UnsupportedTLVType(tlv_type)),
        }
    }

    fn validate_message_structure(&self, tlvs: &[TLVExtension]) -> Result<(), ValidationError> {
        // Ensure market data specific structure rules
        for tlv in tlvs {
            if !self.is_market_data_type(tlv.header.tlv_type) {
                return Err(ValidationError::WrongDomain {
                    expected: RelayDomain::MarketData,
                    found_type: tlv.header.tlv_type,
                });
            }
        }
        Ok(())
    }

    fn get_allowed_types(&self) -> &[TLVType] {
        &[TLVType::Trade, TLVType::Quote, TLVType::OrderBook, TLVType::MarketStatus]
    }
}
```

### Migration Path
```rust
// libs/alphapulse_codec/src/migration.rs - TEMPORARY COMPATIBILITY
/// Temporary compatibility layer for services migrating from relay-specific parsing
#[deprecated(note = "Use TLVParser directly instead")]
pub mod compat {
    pub use super::TLVParser as MarketDataParser;
    pub use super::TLVParser as SignalParser;
    pub use super::TLVParser as ExecutionParser;
    
    // Bridge functions for existing relay code
    pub fn parse_market_data_message(data: &[u8]) -> Result<ValidatedMessage, ParseError> {
        TLVParser::new().parse_message(data)
    }
}
```

## Acceptance Criteria

### Code Consolidation
- [ ] All TLV parsing logic moved from relays/ to alphapulse_codec
- [ ] No duplicate validation functions across codebase
- [ ] Single TLVValidator handles all domain-specific rules
- [ ] Message builder supports all TLV types with validation

### Performance Preservation
- [ ] Zero-copy operations maintained where possible
- [ ] Parsing performance ≥ current relay parsers (>1.6M msg/s)
- [ ] Building performance ≥ current builders (>1M msg/s)
- [ ] Memory usage not increased by consolidation

### API Consistency
- [ ] Unified parser interface for all domains
- [ ] Consistent error types across all validation
- [ ] Builder API supports all current relay functionality
- [ ] Backwards compatibility during migration period

### Integration Points
- [ ] Codec integrates cleanly with Mycelium broker
- [ ] Producer services can use unified message builder
- [ ] Consumer services can use unified parser
- [ ] Validation rules are configurable and extensible

## Dependencies
- **Upstream**: MYC-001 (Platform Foundation) - for integration patterns
- **Downstream**: MYC-005 (Producer Migration), MYC-006 (Consumer Migration)
- **External**: None (consolidating existing code)

## Testing Requirements

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validator_consolidates_all_domains() {
        let validator = TLVValidator::default();
        
        // Test market data validation
        let trade_data = create_test_trade_tlv();
        assert!(validator.validate_tlv(TLVType::Trade, &trade_data).is_ok());
        
        // Test signal validation
        let signal_data = create_test_signal_tlv();
        assert!(validator.validate_tlv(TLVType::SignalIdentity, &signal_data).is_ok());
        
        // Test execution validation
        let execution_data = create_test_execution_tlv();
        assert!(validator.validate_tlv(TLVType::ExecutionReport, &execution_data).is_ok());
    }

    #[test]
    fn parser_handles_all_message_types() {
        let parser = TLVParser::new();
        
        // Test messages from each domain
        for domain in [RelayDomain::MarketData, RelayDomain::Signal, RelayDomain::Execution] {
            let test_message = create_test_message_for_domain(domain);
            let parsed = parser.parse_message(&test_message).unwrap();
            assert_eq!(parsed.header.relay_domain, domain);
        }
    }

    #[test]
    fn builder_validates_before_adding_tlv() {
        let mut builder = TLVMessageBuilder::new(RelayDomain::MarketData, MessageSource::PolygonAdapter);
        
        // Valid TLV for domain should succeed
        let trade_tlv = TradeTLV::default();
        assert!(builder.add_tlv(TLVType::Trade, &trade_tlv).is_ok());
        
        // Invalid TLV for domain should fail
        let execution_tlv = ExecutionReportTLV::default();
        assert!(builder.add_tlv(TLVType::ExecutionReport, &execution_tlv).is_err());
    }
}
```

### Performance Tests
```rust
#[cfg(test)]
mod perf_tests {
    use super::*;

    #[test]
    #[ignore]
    fn consolidated_parser_performance() {
        let parser = TLVParser::new();
        let test_message = create_1kb_test_message();
        let num_iterations = 1_000_000;
        
        let start = std::time::Instant::now();
        for _ in 0..num_iterations {
            parser.parse_message(&test_message).unwrap();
        }
        let elapsed = start.elapsed();
        
        let parse_rate = num_iterations as f64 / elapsed.as_secs_f64();
        println!("Consolidated parser: {:.0} msg/s", parse_rate);
        assert!(parse_rate > 1_600_000.0); // Maintain >1.6M msg/s
    }

    #[test]
    #[ignore]
    fn consolidated_builder_performance() {
        let num_iterations = 1_000_000;
        
        let start = std::time::Instant::now();
        for _ in 0..num_iterations {
            let mut builder = TLVMessageBuilder::new(RelayDomain::MarketData, MessageSource::Test);
            builder.add_tlv(TLVType::Trade, &TradeTLV::default()).unwrap();
            builder.build().unwrap();
        }
        let elapsed = start.elapsed();
        
        let build_rate = num_iterations as f64 / elapsed.as_secs_f64();
        println!("Consolidated builder: {:.0} msg/s", build_rate);
        assert!(build_rate > 1_000_000.0); // Maintain >1M msg/s
    }
}
```

### Migration Tests
```rust
#[cfg(test)]
mod migration_tests {
    use super::*;

    #[test]
    fn backwards_compatibility_maintained() {
        // Test deprecated compatibility functions still work
        let test_message = create_market_data_message();
        
        #[allow(deprecated)]
        let result = crate::migration::compat::parse_market_data_message(&test_message);
        assert!(result.is_ok());
    }

    #[test]
    fn relay_validation_logic_preserved() {
        // Ensure consolidated validation matches original relay behavior
        let validator = TLVValidator::default();
        
        // Test cases that should pass (from original relay tests)
        for test_case in get_relay_validation_test_cases() {
            let result = validator.validate_tlv(test_case.tlv_type, &test_case.data);
            assert_eq!(result.is_ok(), test_case.should_pass, 
                      "Validation changed for {:?}", test_case.tlv_type);
        }
    }
}
```

## Rollback Plan

### If Performance Regression
1. Revert to separate parsers for each domain
2. Move only common validation logic to codec
3. Keep domain-specific optimizations in relays

### If Integration Issues
1. Keep compatibility layer permanently instead of removing it
2. Gradual migration service-by-service
3. Parallel implementation during transition period

### If Validation Logic Issues
1. Copy validation rules exactly from relay implementations
2. Add comprehensive test coverage for edge cases
3. Implement feature flags for new vs. old validation

## Technical Notes

### Design Decisions
- **Single Validator**: Centralizes all domain knowledge in one place
- **Zero-Copy Preservation**: Maintains performance-critical optimizations
- **Backwards Compatibility**: Eases migration with deprecated interfaces
- **Domain-Specific Rules**: Preserves existing validation behavior

### Performance Considerations
- **Validation Caching**: Cache validation rules to avoid repeated lookups
- **Memory Pool**: Reuse validation objects to reduce allocations
- **Branch Prediction**: Order validation checks by frequency
- **SIMD Operations**: Use vectorized operations where possible

### Migration Strategy
- **Gradual Migration**: Move services one at a time to consolidated codec
- **Compatibility Layer**: Provides time for thorough testing
- **Comprehensive Tests**: Ensure no behavior changes during migration
- **Rollback Capability**: Can revert to old parsers if issues arise

## Validation Steps

1. **Code Consolidation**:
   ```bash
   # Verify no duplicate parsing logic remains
   rq find "parse_tlv" --type function
   rq find "validate_" --type function
   ```

2. **Performance Validation**:
   ```bash
   cargo test --package alphapulse_codec --release -- --ignored
   ```

3. **Integration Testing**:
   ```bash
   # Test with existing relay infrastructure
   cargo test --package relays integration_with_consolidated_codec
   ```

4. **Backwards Compatibility**:
   ```bash
   # Verify compatibility layer works
   cargo test --package alphapulse_codec migration_tests
   ```

This consolidation creates a clean, unified codec that eliminates duplicate code while preserving all existing functionality and performance characteristics needed for the broker migration.
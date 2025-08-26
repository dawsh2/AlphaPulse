# Task OPT-003: Enhanced Error Reporting with Context

**Branch**: `feat/enhanced-error-context`  
**Priority**: 🟡 HIGH  
**Estimated Hours**: 3  
**Performance Impact**: NONE - Debugging improvement only  
**Risk Level**: LOW - Error handling enhancement

**NEVER WORK ON MAIN BRANCH**

## Git Branch Enforcement
```bash
# Verify you're on the correct branch
if [ "$(git branch --show-current)" != "feat/enhanced-error-context" ]; then
    echo "❌ WRONG BRANCH! You must work on feat/enhanced-error-context"
    echo "Current branch: $(git branch --show-current)"
    echo "Run: git checkout -b feat/enhanced-error-context"
    exit 1
fi

# Verify we're not on main
if [ "$(git branch --show-current)" = "main" ]; then
    echo "❌ NEVER WORK ON MAIN! Switch to feat/enhanced-error-context"
    echo "Run: git checkout -b feat/enhanced-error-context"
    exit 1
fi
```

## Context & Motivation

Protocol V2 error handling currently provides minimal context for debugging, making it difficult to diagnose issues in production environments. Enhanced error reporting with diagnostic information will significantly improve debugging and troubleshooting capabilities.

**Current Generic Errors**:
```rust
// Minimal context - hard to debug
return Err(ProtocolError::ChecksumMismatch);
return Err(ProtocolError::TruncatedTLV);
return Err(ProtocolError::InvalidTLVType);
```

**Target Rich Errors**:
```rust
// Rich context - actionable debugging information
return Err(ProtocolError::ChecksumMismatch {
    expected: calculated_checksum,
    actual: header.checksum,
    message_size: payload_size,
    tlv_count: extensions.len(),
});

return Err(ProtocolError::TruncatedTLV {
    buffer_size: buffer.len(),
    required_bytes: tlv_length + offset,
    tlv_type: tlv_header.tlv_type,
    offset: offset,
});
```

## Acceptance Criteria

### Functional Requirements (MANDATORY)
- [ ] All Protocol V2 error types enhanced with diagnostic context
- [ ] Error messages include actionable information for debugging
- [ ] Debug formatting provides human-readable error descriptions
- [ ] Display formatting suitable for user-facing error messages
- [ ] Backward compatibility maintained for error matching patterns

### Error Enhancement Requirements
- [ ] **ChecksumMismatch**: Include expected/actual checksums, message metadata
- [ ] **TruncatedTLV**: Include buffer size, required bytes, TLV type, offset position
- [ ] **InvalidTLVType**: Include unknown type number and valid type ranges
- [ ] **ParseError**: Include byte offset, context description, buffer state
- [ ] **PayloadError**: Include payload size, capacity limits, operation context

### Code Quality Requirements
- [ ] Error structures implement Debug, Display, Error traits correctly
- [ ] Error formatting tests validate human-readable output
- [ ] Documentation examples show proper error handling patterns
- [ ] No performance impact on happy path (error construction only)
- [ ] Consistent error formatting patterns across all error types

## Implementation Strategy

### Phase 1: Error Audit & Design (1 hour)
1. **Current Error Analysis**:
   ```bash
   # Find all error creation sites in Protocol V2
   rg "ProtocolError::" --type rust protocol_v2/src/ -A 2 -B 2
   rg "return Err" --type rust protocol_v2/src/ -A 1 -B 1
   rg "map_err|with_context" --type rust protocol_v2/src/
   ```

2. **Error Context Design**:
   ```rust
   // Design enhanced error structures
   #[derive(Debug, thiserror::Error)]
   pub enum ProtocolError {
       #[error("Checksum mismatch: expected {expected:#x}, got {actual:#x} (message: {message_size} bytes, {tlv_count} TLVs)")]
       ChecksumMismatch {
           expected: u32,
           actual: u32,  
           message_size: usize,
           tlv_count: usize,
       },
       
       #[error("TLV truncated: need {required_bytes} bytes, buffer has {buffer_size} (TLV type {tlv_type} at offset {offset})")]
       TruncatedTLV {
           buffer_size: usize,
           required_bytes: usize,
           tlv_type: u16,
           offset: usize,
       },
       
       #[error("Invalid TLV type {tlv_type}: valid ranges are 1-19 (MarketData), 20-39 (Signals), 40-79 (Execution)")]
       InvalidTLVType {
           tlv_type: u16,
       },
       
       #[error("Parse error at byte {offset}: {description} (buffer state: {buffer_size} bytes, {context})")]
       ParseError {
           offset: usize,
           description: String,
           buffer_size: usize,
           context: String,
       },
       
       #[error("Payload error: {operation} failed, size {actual_size}/{capacity_limit} (context: {context})")]
       PayloadError {
           operation: String,
           actual_size: usize,
           capacity_limit: usize,
           context: String,
       },
   }
   ```

### Phase 2: Core Error Structure Implementation (1 hour)

1. **Update ProtocolError Enum**:
   ```rust
   // In protocol_v2/src/lib.rs or dedicated errors module
   use thiserror::Error;
   
   #[derive(Debug, Error)]
   pub enum ProtocolError {
       // Enhanced with diagnostic context...
   }
   
   // Implement additional helper methods
   impl ProtocolError {
       pub fn checksum_mismatch(expected: u32, actual: u32, message_size: usize, tlv_count: usize) -> Self {
           Self::ChecksumMismatch { expected, actual, message_size, tlv_count }
       }
       
       pub fn truncated_tlv(buffer_size: usize, required_bytes: usize, tlv_type: u16, offset: usize) -> Self {
           Self::TruncatedTLV { buffer_size, required_bytes, tlv_type, offset }
       }
       
       pub fn invalid_tlv_type(tlv_type: u16) -> Self {
           Self::InvalidTLVType { tlv_type }
       }
       
       pub fn parse_error(offset: usize, description: impl Into<String>, buffer_size: usize, context: impl Into<String>) -> Self {
           Self::ParseError { 
               offset, 
               description: description.into(), 
               buffer_size, 
               context: context.into() 
           }
       }
       
       pub fn payload_error(operation: impl Into<String>, actual_size: usize, capacity_limit: usize, context: impl Into<String>) -> Self {
           Self::PayloadError {
               operation: operation.into(),
               actual_size,
               capacity_limit, 
               context: context.into()
           }
       }
   }
   ```

2. **Custom Display Implementation** (if needed beyond thiserror):
   ```rust
   impl std::fmt::Display for ProtocolError {
       fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
           match self {
               Self::ChecksumMismatch { expected, actual, message_size, tlv_count } => {
                   write!(f, "Checksum validation failed: calculated {:#010x}, message header contains {:#010x}. Message details: {} bytes with {} TLV extensions. This indicates data corruption or transmission error.", expected, actual, message_size, tlv_count)
               },
               Self::TruncatedTLV { buffer_size, required_bytes, tlv_type, offset } => {
                   write!(f, "TLV message truncated: attempting to read {} bytes from buffer with {} bytes available. TLV type {} at byte offset {}. Check message framing and buffer management.", required_bytes, buffer_size, tlv_type, offset)
               },
               // ... other variants
           }
       }
   }
   ```

### Phase 3: Update Error Creation Sites (1 hour)

1. **Systematic Error Site Updates**:
   ```bash
   # Find and update all error creation patterns
   
   # ChecksumMismatch updates
   rg "ChecksumMismatch" --type rust protocol_v2/src/ -l | xargs -I {} \
       sed -i 's/ProtocolError::ChecksumMismatch/ProtocolError::checksum_mismatch(expected_checksum, header.checksum, payload_size, tlv_count)/g' {}
   
   # TruncatedTLV updates  
   rg "TruncatedTLV" --type rust protocol_v2/src/ -l | xargs -I {} \
       sed -i 's/ProtocolError::TruncatedTLV/ProtocolError::truncated_tlv(buffer.len(), required_bytes, tlv_type, offset)/g' {}
   ```

2. **Manual Updates for Complex Cases**:
   ```rust
   // Example: In TLV parsing code
   // BEFORE:
   if offset + tlv_length > payload.len() {
       return Err(ParseError::TruncatedTLV);
   }
   
   // AFTER:
   if offset + tlv_length > payload.len() {
       return Err(ProtocolError::truncated_tlv(
           payload.len(),
           offset + tlv_length, 
           tlv.header.tlv_type,
           offset
       ));
   }
   
   // Example: In checksum validation
   // BEFORE:
   if calculated_checksum != header.checksum {
       return Err(ProtocolError::ChecksumMismatch);
   }
   
   // AFTER:
   if calculated_checksum != header.checksum {
       return Err(ProtocolError::checksum_mismatch(
           calculated_checksum,
           header.checksum,
           payload.len(),
           parsed_tlvs.len()
       ));
   }
   ```

3. **Update Error Handling in Calling Code**:
   ```rust
   // Example: Better error context propagation
   match parse_tlv_message(&bytes) {
       Ok(message) => message,
       Err(e) => {
           error!("TLV message parsing failed: {}", e);
           // Enhanced error provides actionable context automatically
           return Err(e.into());
       }
   }
   ```

## Files to Modify

### Core Error Definitions
- `/Users/daws/alphapulse/backend_v2/protocol_v2/src/lib.rs`
  - Update ProtocolError enum with enhanced variants
  - Add constructor helper methods

### Parser & Validation Code
- `/Users/daws/alphapulse/backend_v2/protocol_v2/src/tlv/parser.rs`
  - Update TLV parsing error creation
  - Add context information to parse failures

- `/Users/daws/alphapulse/backend_v2/protocol_v2/src/validation/checksum.rs`
  - Update checksum validation errors
  - Include calculated vs expected values

- `/Users/daws/alphapulse/backend_v2/protocol_v2/src/validation/bounds.rs`  
  - Update bounds checking errors
  - Include buffer state information

### TLV-Specific Code
- `/Users/daws/alphapulse/backend_v2/protocol_v2/src/tlv/dynamic_payload.rs`
  - Update FixedVec capacity errors
  - Include operation context

- `/Users/daws/alphapulse/backend_v2/protocol_v2/src/tlv/builder.rs`
  - Update message building errors
  - Include construction state context

### Test Files (New)
- `/Users/daws/alphapulse/backend_v2/protocol_v2/tests/error_formatting.rs`
  - Comprehensive error message testing
  - Debug vs Display formatting validation

## Testing & Validation Strategy

### Error Formatting Tests
```rust
#[cfg(test)]
mod error_tests {
    use super::*;
    
    #[test]
    fn test_checksum_mismatch_formatting() {
        let error = ProtocolError::checksum_mismatch(0x12345678, 0x87654321, 1024, 5);
        
        let debug_output = format!("{:?}", error);
        assert!(debug_output.contains("0x12345678"));
        assert!(debug_output.contains("0x87654321"));
        assert!(debug_output.contains("1024"));
        assert!(debug_output.contains("5"));
        
        let display_output = format!("{}", error);
        assert!(display_output.contains("calculated"));
        assert!(display_output.contains("message header contains"));
        assert!(display_output.contains("data corruption"));
    }
    
    #[test]
    fn test_truncated_tlv_formatting() {
        let error = ProtocolError::truncated_tlv(100, 150, 42, 75);
        
        let display_output = format!("{}", error);
        assert!(display_output.contains("attempting to read 150 bytes"));
        assert!(display_output.contains("buffer with 100 bytes"));
        assert!(display_output.contains("TLV type 42"));
        assert!(display_output.contains("byte offset 75"));
    }
    
    #[test]
    fn test_invalid_tlv_type_formatting() {
        let error = ProtocolError::invalid_tlv_type(99);
        
        let display_output = format!("{}", error);
        assert!(display_output.contains("type 99"));
        assert!(display_output.contains("1-19 (MarketData)"));
        assert!(display_output.contains("20-39 (Signals)"));
        assert!(display_output.contains("40-79 (Execution)"));
    }
}
```

### Integration Error Testing
```rust
#[test]  
fn test_enhanced_errors_in_real_parsing() {
    // Create intentionally malformed message
    let mut malformed_bytes = vec![0u8; 32]; // Valid header size
    malformed_bytes[0..4].copy_from_slice(&0xDEADBEEFu32.to_le_bytes()); // Magic
    malformed_bytes[4..8].copy_from_slice(&100u32.to_le_bytes()); // Claim 100 bytes payload
    // But only provide 32 bytes total -> truncation error
    
    match parse_tlv_message(&malformed_bytes) {
        Err(ProtocolError::TruncatedTLV { buffer_size, required_bytes, .. }) => {
            assert_eq!(buffer_size, 32);
            assert_eq!(required_bytes, 132); // 32 + 100
        },
        other => panic!("Expected TruncatedTLV error, got {:?}", other),
    }
}

#[test]
fn test_enhanced_errors_in_checksum_validation() {
    let message = create_valid_message_with_wrong_checksum();
    
    match parse_tlv_message(&message.bytes) {
        Err(ProtocolError::ChecksumMismatch { expected, actual, message_size, tlv_count }) => {
            assert_ne!(expected, actual); // Different checksums
            assert!(message_size > 0);    // Non-zero message  
            assert!(tlv_count >= 0);      // Valid TLV count
        },
        other => panic!("Expected ChecksumMismatch error, got {:?}", other),
    }
}
```

## Validation Commands

### Error Message Testing
```bash
# Run error formatting tests
cargo test --package protocol_v2 error_formatting -- --nocapture

# Test error propagation in integration scenarios
cargo test --package protocol_v2 enhanced_errors_in_real -- --nocapture

# Verify backward compatibility 
cargo test --package protocol_v2 error_handling
```

### Performance Impact Analysis
```bash
# Verify no performance regression on happy path
cargo bench --package protocol_v2 --bench message_construction
cargo bench --package protocol_v2 --bench message_parsing

# The enhanced errors should only impact error cases, not success cases
```

### Documentation Testing
```bash
# Generate and review error documentation  
cargo doc --package protocol_v2 --open --document-private-items

# Check error examples in documentation
cargo test --doc --package protocol_v2
```

## Success Definition

This task is successful when:

1. **Comprehensive Context**: All Protocol V2 errors provide actionable debugging information
2. **Human-Readable**: Error messages are clear and help identify root causes
3. **Backward Compatible**: Existing error matching patterns continue to work
4. **Well Tested**: Error formatting and integration scenarios have comprehensive tests
5. **Zero Performance Impact**: Happy path performance unchanged (error construction only)

**Key Measure**: Debug logs become significantly more useful for troubleshooting Protocol V2 issues in production environments.

## Example Enhanced Error Output

**Before**: 
```
ERROR: Protocol parsing failed: ChecksumMismatch
```

**After**:
```  
ERROR: Protocol parsing failed: Checksum validation failed: calculated 0x12345678, message header contains 0x87654321. Message details: 1024 bytes with 3 TLV extensions. This indicates data corruption or transmission error.

DEBUG Context:
  - Buffer size: 1024 bytes
  - Expected checksum: 0x12345678
  - Actual checksum: 0x87654321  
  - TLV count: 3
  - Likely cause: Network transmission error or memory corruption
  - Suggested action: Retry message transmission or check network integrity
```

This level of diagnostic information transforms debugging from guesswork into systematic troubleshooting.
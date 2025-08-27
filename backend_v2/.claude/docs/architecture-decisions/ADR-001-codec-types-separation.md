# ADR-001: Codec and Types Package Separation

**Status**: Accepted  
**Date**: 2025-08-27  
**Context**: Sprint 012 Critical Gaps Resolution  

## Summary

Separate `alphapulse_codec` and `alphapulse-types` packages to eliminate circular dependencies while maintaining clean architecture boundaries between type definitions and encoding/decoding logic.

## Context

During Sprint 012 Critical Gaps resolution, a circular dependency was identified between:
- `libs/types` (alphapulse-types): Core TLV type definitions and protocol structures
- `libs/alphapulse_codec`: TLV encoding, decoding, and message construction utilities

### The Circular Dependency Problem

**Before GAP-002 Fix:**
```
alphapulse-types
    ↓ depends on (for TLVMessageBuilder)
alphapulse_codec  
    ↓ depends on (for TLV struct definitions)
alphapulse-types
```

This circular dependency caused:
- Compilation errors: "cyclic package dependency: package `alphapulse-types` depends on itself"
- Import resolution failures across multiple services
- Inability to build critical production services

### Root Cause Analysis

The circular dependency emerged because:
1. **Type definitions** (QuoteTLV, StateInvalidationTLV, etc.) were in `alphapulse-types`
2. **Message construction utilities** (TLVMessageBuilder) were in `alphapulse_codec`
3. **Types package** tried to import TLVMessageBuilder for convenience methods
4. **Codec package** needed the type definitions to implement encoding/decoding

## Decision

**Resolve the circular dependency by establishing a clear architectural hierarchy:**

### New Dependency Architecture
```
Services (dashboard, adapters, strategies)
    ↓ import from both
├── alphapulse_codec (encoding/decoding logic)
│   ↓ depends on
└── alphapulse-types (core type definitions)
    ↓ depends on
    alphapulse-transport (timestamp utilities)
```

### Package Responsibilities

#### alphapulse-types (`libs/types`)
- **Core TLV type definitions**: QuoteTLV, StateInvalidationTLV, PoolSwapTLV, etc.
- **Protocol structures**: MessageHeader, TLV enums, validation logic
- **Common types**: InstrumentId, VenueId, InvalidationReason
- **Protocol constants**: TLV type numbers, domain ranges
- **NO dependency on alphapulse_codec**

#### alphapulse_codec (`libs/alphapulse_codec`)  
- **Message construction**: TLVMessageBuilder, message formatting
- **Parsing utilities**: parse_header, parse_tlv_extensions
- **Encoding/Decoding**: Serialization and deserialization logic
- **Message validation**: Protocol compliance checking
- **Depends on alphapulse-types for type definitions**

### Import Patterns for Services

Services should import based on their needs:

```rust
// For services that only need type definitions:
use alphapulse_types::{QuoteTLV, InstrumentId, VenueId};

// For services that need message construction:
use alphapulse_codec::{TLVMessageBuilder, parse_header, parse_tlv_extensions};
use alphapulse_types::{QuoteTLV, TLVType, RelayDomain};

// Example: Dashboard websocket server
use alphapulse_codec::{parse_header, parse_tlv_extensions, ParseError};
use alphapulse_types::{QuoteTLV, InvalidationReason, StateInvalidationTLV};
```

## Rationale

### Why This Architecture is Correct

1. **Clear Separation of Concerns**
   - **Types**: What the data structures are
   - **Codec**: How to encode/decode those structures
   - This follows standard library design patterns

2. **Dependency Flow Aligns with Abstraction Levels**
   - Codec (higher-level operations) depends on Types (lower-level definitions)
   - Services (application-level) depend on both as needed
   - No circular references

3. **Follows AlphaPulse Principles**
   - **Breaking Changes Welcome**: We freely broke the circular dependency
   - **One Canonical Source**: Single definition for each TLV type
   - **Clean Architecture**: Proper separation of data and logic

4. **Industry Standard Pattern**
   - Similar to `serde` (traits) + `serde_json` (implementation)
   - Similar to `tokio` (runtime) + `tokio-util` (utilities)
   - Rust ecosystem commonly separates type definitions from implementations

### Performance Considerations

- **Zero-Copy Preserved**: All zero-copy operations maintained across package boundary
- **Compilation Time**: Reduced because services can import only what they need
- **Binary Size**: Services that only need types don't pull in codec machinery

### Alternative Approaches Considered

#### Option 1: Merge Everything into One Package
**Rejected because:**
- Creates unnecessary coupling between data definitions and processing logic
- Services that only need types would pull in heavy codec dependencies
- Violates single responsibility principle

#### Option 2: Move TLVMessageBuilder to types package
**Rejected because:**
- Would require types package to have encoding/decoding knowledge
- Violates separation of concerns
- Still creates coupling between data and processing

#### Option 3: Create third package for shared utilities
**Rejected because:**
- Adds unnecessary complexity for this specific case
- The codec functionality naturally belongs with message construction
- Would create three packages instead of a clean two-package solution

## Implementation

### Changes Made in GAP-002

1. **Removed circular dependency**:
   - Removed `alphapulse_codec` from `libs/types/Cargo.toml` dependencies
   - Added comment explaining the architectural decision

2. **Updated import patterns across services**:
   - Dashboard websocket: Import codec parsing functions directly
   - Flash arbitrage strategy: Import TLVMessageBuilder from codec
   - Relay services: Import both types and codec as needed

3. **Maintained all functionality**:
   - No loss of features or performance
   - All TLV types remain accessible
   - All encoding/decoding functionality preserved

### Validation

- ✅ All critical services compile successfully
- ✅ No performance regression in TLV processing
- ✅ All GAP-001 exported types accessible
- ✅ Message construction and parsing work correctly
- ✅ Zero circular dependencies verified

## Consequences

### Positive
- **Clean architecture**: Clear separation between data definitions and processing logic
- **No circular dependencies**: Eliminates compilation issues
- **Better performance**: Services can import only needed functionality
- **Easier maintenance**: Changes to encoding logic don't require rebuilding type definitions
- **Industry standard**: Follows common Rust ecosystem patterns

### Negative  
- **Slightly more imports**: Services need to import from both packages when using both
- **Learning curve**: Developers need to understand which package provides what functionality

### Migration Impact
- **Breaking change**: Services must update their import statements
- **One-time cost**: Migration completed as part of GAP-002
- **Future proof**: Architecture prevents similar issues going forward

## Monitoring

To prevent regression of this architectural decision:

1. **Automated checks**: Added to `scripts/validate-dependencies.sh`
2. **Documentation**: This ADR documents the rationale for future reference  
3. **Code review**: Reviewers should check for import patterns that might reintroduce circular dependencies

## References

- **Sprint 012 Documentation**: `.claude/tasks/sprint-012-critical-gaps/`
- **GAP-002 Task**: `GAP-002_compilation_import_errors.md`
- **Dependency Patterns**: `.claude/docs/dependency-patterns.md`
- **Development Guide**: `.claude/docs/development.md`

## Future Considerations

### When This Decision Might Be Revisited

1. **Major Protocol Changes**: If Protocol V3 requires fundamental restructuring
2. **Performance Issues**: If the separation creates measurable overhead (unlikely)
3. **Ecosystem Changes**: If Rust ecosystem patterns evolve significantly

### Success Metrics

- No circular dependency errors in CI/CD
- Clean import patterns across all services
- No performance regression in TLV processing
- Developer productivity maintained or improved

---

**Decision Status**: ✅ **Accepted and Implemented**  
**Next Review**: No scheduled review - architecture is stable
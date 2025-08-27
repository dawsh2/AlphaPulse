# Architecture Validation Tests

Comprehensive architecture validation tests for the AlphaPulse backend_v2 codebase that validate adherence to all architectural constraints defined in [CLAUDE.md](../../CLAUDE.md).

> **Implementation Status**: ✅ **COMPLETE** - All validation modules implemented and functional

## Overview

These tests validate that the codebase follows the critical architectural principles of the AlphaPulse trading system:

1. **Codec Usage** - Services use codec library consistently, no protocol duplication
2. **Plugin Compliance** - Adapters implement proper trait interfaces and structure
3. **Typed ID Usage** - Use InstrumentId/PoolId instead of raw primitives, bijective design
4. **Code Quality** - No mocks in production, proper precision handling, configuration usage
5. **Dependency Compliance** - Correct import patterns and service boundaries

## Running the Tests

### Run All Tests
```bash
# Run all architecture validation tests (recommended)
cargo run --bin architecture_validation

# Or run unit tests
cargo test
```

### From Project Root
```bash
# Through manage.sh (integrated with validation pipeline)
./scripts/manage.sh validate

# Direct execution
cargo run --manifest-path tests/architecture_validation/Cargo.toml
```

### Available Validation Categories

- **Dependency Validation** - Codec usage, no protocol duplication, correct imports
- **Plugin Compliance** - Adapter trait implementation, directory structure
- **Typed ID Usage** - InstrumentId usage, correct imports, bijective patterns
- **Code Quality** - No mocks, no floats for finance, error handling patterns

## Test Structure

### Protocol V2 Compliance (`protocol_v2_compliance.rs`)
- TLV message header format (32-byte header + variable payload)
- Magic number validation (0xDEADBEEF)
- Domain separation: Market Data (1-19), Signals (20-39), Execution (40-79)
- Nanosecond timestamp preservation
- TLV type registry uniqueness
- Sequence integrity validation

### Precision Validation (`precision_validation.rs`)
- Detects floating point usage in financial contexts
- Validates proper rust_decimal usage
- Ensures native token precision preservation
- Checks fixed-point arithmetic patterns
- AST-based type analysis for comprehensive coverage

### Mock Detection (`mock_detection.rs`)
- Detects mock services and data usage
- Validates real exchange connections
- Ensures no simulation modes
- Checks for stubbed WebSocket connections
- Validates live data requirements

### File Organization (`file_organization.rs`)
- Project structure compliance (libs/, services_v2/, relays/, etc.)
- Service boundary respect
- Proper library usage patterns  
- README-first development validation
- Code scatter detection

### Duplicate Detection (`duplicate_detection.rs`)
- Detects prefixed duplicates ("Enhanced", "New", "V2", etc.)
- Function name uniqueness analysis
- Struct concept canonicalization
- Redundant utility detection
- Code reuse through libs validation

### Performance Validation (`performance_validation.rs`)
- Hot path performance constraints (<35μs latency)
- Memory allocation pattern analysis
- Protocol V2 benchmark requirements
- Zero-copy compliance validation
- Async and networking performance patterns

### Breaking Changes (`breaking_changes.rs`)
- Deprecated code detection (should be removed)
- Backward compatibility avoidance
- Interface consistency validation
- Clean refactoring pattern enforcement
- Legacy pattern detection

### Documentation Standards (`documentation_standards.rs`)
- Marketing language detection and removal
- Precise capability statement requirements
- Limitation documentation enforcement
- Context-aware writing validation
- Technical precision requirements

## Integration with CI/CD

The validation tests are designed for CI/CD integration:

```yaml
# Example GitHub Actions workflow
- name: Architecture Validation
  run: |
    cd backend_v2/tests/architecture_validation
    cargo test --verbose
    cargo run --bin architecture_validation
```

Exit codes:
- `0` - All tests passed
- `1` - Architecture violations detected

## Adding New Validations

To add new architectural constraints:

1. Create validation function in appropriate module
2. Add comprehensive test coverage
3. Update test runner to include new validation
4. Document the constraint in this README

## Performance

These validation tests are designed to be fast and can be run frequently during development. They use efficient parsing and pattern matching to validate large codebases quickly.

## Maintenance

- Update TLV type ranges when protocol changes
- Add new financial keywords as needed
- Maintain whitelist patterns for legitimate exceptions
- Review and update performance thresholds regularly

## Dependencies

- `syn` - Rust code parsing and AST analysis
- `regex` - Pattern matching for code validation
- `walkdir` - Recursive directory traversal
- `cargo_metadata` - Workspace dependency analysis
- `colored` - Formatted test output
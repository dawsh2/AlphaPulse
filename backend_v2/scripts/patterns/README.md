# AlphaPulse Pattern Enforcement System

This directory contains automated scripts for detecting architectural pattern violations in the AlphaPulse codebase. These tools help maintain code quality and architectural consistency.

## Overview

The pattern enforcement system automatically detects:

1. **Direct Transport Usage** - Inappropriate `UnixSocketTransport::new` usage outside factory locations
2. **Precision Violations** - Float/double usage in financial calculations (critical for trading systems)
3. **TLV Pattern Issues** - Improper TLV macro usage (planned)
4. **Redundant Implementations** - Code duplicating shared library functionality (planned)

## Quick Start

```bash
# Run all pattern checks
./scripts/patterns/run-all-pattern-checks.sh

# Check specific patterns
./scripts/patterns/detect-transport-violations.sh src/
python3 scripts/patterns/detect-precision-violations.py src/

# Check single file
./scripts/patterns/detect-transport-violations.sh src/main.rs
python3 scripts/patterns/detect-precision-violations.py src/trading.rs
```

## Pattern Detection Scripts

### 1. Transport Usage Violations (`detect-transport-violations.sh`)

**Purpose**: Prevents direct `UnixSocketTransport::new()` usage outside approved factory locations.

**Why This Matters**: Direct transport usage bypasses the factory pattern, making connection management inconsistent and harder to test.

**Example Violation**:
```rust
// ❌ VIOLATION - Direct usage outside factory
let transport = UnixSocketTransport::new("/tmp/socket");

// ✅ CORRECT - Use factory
let transport = TransportFactory::create("/tmp/socket")?;
```

**Configuration**:
- Whitelist file: `scripts/patterns/transport-whitelist.txt`
- Add approved files/patterns to whitelist

**Usage**:
```bash
# Check single file
./scripts/patterns/detect-transport-violations.sh src/main.rs

# Check directory
./scripts/patterns/detect-transport-violations.sh src/

# Check with verbose output
VERBOSE=true ./scripts/patterns/detect-transport-violations.sh src/
```

### 2. Precision Violations (`detect-precision-violations.py`)

**Purpose**: Detects float/double usage in financial calculations where precision loss could cause trading errors.

**Why This Matters**: In trading systems, precision loss from floating-point arithmetic can result in:
- Rounding errors in price calculations
- Cumulative errors in position sizing  
- Compliance issues with financial regulations
- Lost profit opportunities

**Example Violations**:
```rust
// ❌ VIOLATIONS - Float usage for financial data
pub struct Trade {
    pub price: f64,      // Should use fixed-point
    pub quantity: f32,   // Should use native precision
    pub commission: f64, // Should use fixed-point
}

pub fn calculate_profit(buy: f64, sell: f64) -> f64 {  // VIOLATION
    sell - buy
}

// ✅ CORRECT - Fixed-point arithmetic
pub struct Trade {
    pub price: i64,      // 8-decimal fixed-point (* 100_000_000)
    pub quantity: i64,   // Native token precision (18 decimals WETH)
    pub commission: i64, // 8-decimal fixed-point
}

pub fn calculate_profit(buy: i64, sell: i64) -> i64 {
    sell - buy  // No precision loss
}
```

**Financial Context Detection**:
The script detects financial context through:
- Keywords: `price`, `amount`, `value`, `fee`, `profit`, `trade`, `swap`, etc.
- Variable names: `*_price`, `*_amount`, `*_value`, etc.
- File locations: `trading/`, `finance/`, `dex/`

**Precision Guidelines**:
- **DEX Tokens**: Use native precision (18 decimals WETH, 6 USDC, etc.)
- **USD Prices**: Use 8-decimal fixed-point (`* 100_000_000`)
- **Quantities**: Preserve exchange native precision
- **Non-Financial**: Float usage OK for graphics, geometry, etc.

**Usage**:
```bash
# Check single file
python3 scripts/patterns/detect-precision-violations.py src/trading.rs

# Check directory
python3 scripts/patterns/detect-precision-violations.py src/

# Quiet mode (for CI)
python3 scripts/patterns/detect-precision-violations.py --quiet src/
```

## Running All Checks

The `run-all-pattern-checks.sh` script executes all pattern detection tools:

```bash
# Run all checks (continue on errors)
./scripts/patterns/run-all-pattern-checks.sh

# Run all checks (fail fast)
./scripts/patterns/run-all-pattern-checks.sh . true

# Check specific directory
./scripts/patterns/run-all-pattern-checks.sh services_v2/
```

## CI/CD Integration

### GitHub Actions Integration

Add to `.github/workflows/ci.yml`:

```yaml
- name: Architectural Pattern Checks
  run: |
    cd backend_v2
    ./scripts/patterns/run-all-pattern-checks.sh . true
```

See `scripts/patterns/ci-integration.yml` for complete configuration.

### Pre-commit Hook

Add to `.pre-commit-config.yaml`:

```yaml
- repo: local
  hooks:
    - id: pattern-enforcement
      name: AlphaPulse Pattern Enforcement
      entry: scripts/patterns/run-all-pattern-checks.sh
      language: script
      pass_filenames: false
      always_run: true
```

## Whitelisting Legitimate Usage

### Transport Usage Whitelist

Edit `scripts/patterns/transport-whitelist.txt`:

```
# Transport factory implementations (approved)
src/transport/factory.rs
libs/transport/src/factory.rs
network/transport/src/unix.rs

# Service adapters (direct usage OK)
services_v2/adapters/src/transport/
```

### Precision Usage Whitelist

The precision detector automatically whitelists:
- `*graphics*`, `*ui/*`, `*display*` - UI/graphics code
- `*render*`, `*geometry*`, `*physics*` - Non-financial math
- `*test*` - Test files (more lenient)

Add custom patterns by modifying `WHITELIST_PATTERNS` in the script.

## Error Messages and Fixes

### Transport Violation
```
VIOLATION: Direct UnixSocketTransport usage in src/service.rs:42
  Found: let transport = UnixSocketTransport::new("/tmp/socket");
  Suggestion: Use TransportFactory::create() instead
  Example: let transport = TransportFactory::create("/tmp/socket")?;
```

**Fix**: Replace direct usage with factory pattern.

### Precision Violation  
```
VIOLATION: Float usage in financial context
  File: src/trading.rs:15
  Found: pub price: f64,
  Context: Financial context: price, trade
  Suggestion: Use 8-decimal fixed-point for USD values
  Example: let price_fixed: i64 = 4500000000000; // $45,000.00
```

**Fix**: Convert to appropriate fixed-point arithmetic based on context.

## Performance

Pattern checks are designed for CI performance:
- **Transport Detection**: ~2 seconds for full codebase
- **Precision Detection**: ~3 seconds for full codebase  
- **Combined**: <10 seconds total CI overhead

## Development Workflow

### Adding New Pattern Checks

1. **Create Test File**: `tests/patterns/test_detect_new_pattern.py`
2. **Write Failing Tests** (TDD Red phase)
3. **Implement Script**: `scripts/patterns/detect-new-pattern.py`
4. **Make Tests Pass** (TDD Green phase)
5. **Add to CI Runner**: Update `run-all-pattern-checks.sh`
6. **Document**: Update this README

### Debugging Pattern Detection

```bash
# Enable verbose output
VERBOSE=true ./scripts/patterns/detect-transport-violations.sh src/

# Test single file with debug
python3 -c "
from scripts.patterns.detect_precision_violations import PrecisionDetector
detector = PrecisionDetector()
violations = detector.detect_in_file('src/test.rs')
print(f'Found {len(violations)} violations')
"
```

## False Positive Handling

If pattern detection reports false positives:

1. **Review Context**: Ensure the usage is actually non-problematic
2. **Add to Whitelist**: Add file/pattern to appropriate whitelist
3. **Improve Detection**: Refine detection logic to reduce false positives
4. **Document Exception**: Add comments explaining why usage is acceptable

## Integration with Development Tools

### VS Code

Add to `.vscode/tasks.json`:

```json
{
  "label": "Pattern Check",
  "type": "shell", 
  "command": "./scripts/patterns/run-all-pattern-checks.sh",
  "args": ["${workspaceFolder}"],
  "group": "test",
  "presentation": {
    "echo": true,
    "reveal": "always"
  }
}
```

### Git Hooks

```bash
#!/bin/bash
# .git/hooks/pre-push
./scripts/patterns/run-all-pattern-checks.sh . true
```

## Troubleshooting

### Common Issues

**Script not executable**:
```bash
chmod +x scripts/patterns/*.sh
```

**Python module not found**:
```bash
# Run from project root
cd backend_v2
python3 scripts/patterns/detect-precision-violations.py src/
```

**Whitelist not working**:
- Check file paths are relative to project root
- Verify pattern syntax (supports regex)
- Test with single file first

**Performance issues**:
- Check file count: `find src/ -name "*.rs" | wc -l`
- Use directory targeting: `./script src/specific/directory/`
- Consider parallelization for very large codebases

## Future Enhancements

Planned pattern detection additions:

1. **TLV Pattern Validation** - Detect improper `define_tlv!` usage
2. **Redundant Implementation Detection** - Find code duplicating shared libraries  
3. **Error Handling Patterns** - Ensure proper error propagation
4. **Performance Patterns** - Detect known performance anti-patterns
5. **Security Patterns** - Find potential security vulnerabilities

## Contributing

To add new pattern detection:

1. Follow TDD (Test-Driven Development)
2. Write comprehensive tests first
3. Implement minimal viable detection
4. Add CI integration
5. Update documentation
6. Consider false positive impact

## License

Part of the AlphaPulse trading system. See project root for license information.
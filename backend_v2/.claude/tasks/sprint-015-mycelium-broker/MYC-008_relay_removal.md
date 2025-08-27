# MYC-008: Relay Removal

## Status
- **Status**: pending
- **Assignee**: TBD
- **Estimated Effort**: 1 day
- **Priority**: Medium (cleanup after successful migration)

## Description
Clean up legacy relay infrastructure after successful migration to Mycelium broker. This involves removing the relays/ directory, updating build configurations, cleaning up documentation, and ensuring no references to the old relay system remain in the codebase.

## Objectives
1. Remove relays/ directory and all relay implementation code
2. Update root Cargo.toml to remove relay workspace members
3. Clean up service configurations and remove relay dependencies
4. Update documentation and architecture diagrams
5. Ensure no broken references or imports remain

## Technical Approach

### Code Removal Audit
```bash
# Identify all relay-related code before removal
find . -name "*.rs" -exec grep -l "RelayClient\|relay::" {} \;
find . -name "*.toml" -exec grep -l "relay" {} \;
find . -name "*.md" -exec grep -l "relay" {} \;

# Identify relay dependencies
rq find "relay" --type struct
rq find "RelayDomain" --type enum
rq find "relay_client" --type function
```

### Directory Structure Cleanup
```bash
# Current structure (to be removed)
relays/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── market_data/
│   │   ├── mod.rs
│   │   ├── parser.rs
│   │   └── validator.rs
│   ├── signal/
│   │   ├── mod.rs
│   │   ├── parser.rs
│   │   └── validator.rs
│   ├── execution/
│   │   ├── mod.rs
│   │   ├── parser.rs
│   │   └── validator.rs
│   └── shared/
│       ├── client.rs
│       ├── config.rs
│       └── validation.rs
├── tests/
└── examples/

# Files to remove
rm -rf relays/
rm -f scripts/start_relays.sh
rm -f scripts/monitor_relays.sh
```

### Cargo.toml Updates
```toml
# Root Cargo.toml - REMOVE relay entries
[workspace]
members = [
    "libs/alphapulse_codec",
    "libs/types", 
    "libs/message_sink",
    # REMOVE: "relays",
    "services_v2/adapters",
    "services_v2/strategies",
    "services_v2/dashboard",
    # ... other members
]

# Remove relay-specific dependencies
[workspace.dependencies]
# REMOVE: relay-client = { path = "relays" }
# REMOVE: relay-config = "0.1"
```

### Service Configuration Cleanup
```rust
// services_v2/adapters/Cargo.toml - REMOVE relay dependencies
[dependencies]
alphapulse_codec = { path = "../../libs/alphapulse_codec" }
mycelium-transport = { path = "../../../mycelium/crates/mycelium-transport" }
mycelium-broker = { path = "../../../mycelium/crates/mycelium-broker" }
# REMOVE: relay-client = { workspace = true }

# services_v2/strategies/flash_arbitrage/Cargo.toml - REMOVE relay dependencies
[dependencies]
alphapulse_codec = { path = "../../../libs/alphapulse_codec" }
mycelium-transport = { path = "../../../../mycelium/crates/mycelium-transport" }
# REMOVE: relay-client = { workspace = true }
```

### Documentation Updates
```markdown
<!-- docs/ARCHITECTURE.md - UPDATE architecture diagrams -->
# AlphaPulse Architecture

## Message Flow (Updated)
```
Exchange APIs → Collectors → Mycelium Broker → Strategies
             WebSocket      Unix Socket     Topic-based
                           Subscription
```

<!-- REMOVE old relay architecture -->
<!--
## Legacy Architecture (REMOVED)
```
Exchange APIs → Collectors → Domain Relays → Strategies
             WebSocket     ├── MarketDataRelay
                          ├── SignalRelay  
                          └── ExecutionRelay
```
-->

<!-- README.md - UPDATE getting started guide -->
# Getting Started

## Start the System
```bash
# Start Mycelium broker
cd mycelium && cargo run --bin mycelium-broker

# Start services
cd services_v2
cargo run --bin polygon_adapter
cargo run --bin flash_arbitrage
```

<!-- REMOVE old relay instructions -->
<!-- ./scripts/start_relays.sh -->
<!-- ./scripts/start_services.sh -->
```

### Code Reference Updates
```rust
// Find and fix remaining references
// services_v2/common/src/config.rs - REMOVE relay configurations
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SystemConfig {
    pub adapters: AdapterConfig,
    pub strategies: StrategyConfig,
    pub broker: BrokerConfig, // UPDATED: was relay_config
    // REMOVE: pub relays: RelayConfig,
}

// libs/types/src/common/mod.rs - REMOVE relay-specific types
pub use crate::protocol::message::header::MessageHeader;
pub use crate::protocol::tlv::*;
// REMOVE: pub use crate::relay::*;

// Update import statements across codebase
// OLD: use relay_client::{RelayClient, RelayDomain};
// NEW: use mycelium_broker::{BrokerClient, Topic};
```

### Script and Tool Updates
```bash
# scripts/system_health_check.sh - UPDATE health checks
#!/bin/bash

echo "=== AlphaPulse System Health Check ==="

# Check Mycelium broker
echo "Checking Mycelium broker..."
if pgrep -f "mycelium-broker" > /dev/null; then
    echo "✓ Broker running"
else
    echo "✗ Broker not running"
    exit 1
fi

# REMOVE relay health checks
# echo "Checking relays..."
# for relay in market_data signal execution; do
#     if pgrep -f "${relay}_relay" > /dev/null; then
#         echo "✓ ${relay} relay running"
#     else
#         echo "✗ ${relay} relay not running"
#     fi
# done

# Check services
echo "Checking services..."
for service in polygon_adapter flash_arbitrage portfolio_service dashboard; do
    if pgrep -f "$service" > /dev/null; then
        echo "✓ $service running"
    else
        echo "✗ $service not running"
    fi
done
```

### Test Cleanup
```rust
// Remove relay-specific tests
rm -f tests/integration/relay_*.rs
rm -f tests/unit/relay_*.rs

// Update integration tests that referenced relays
// tests/integration/end_to_end.rs - UPDATE to use broker
#[tokio::test]
async fn complete_system_integration() {
    // Start broker instead of relays
    let broker = start_test_broker().await;
    
    // OLD: let relay_clients = start_test_relays().await;
    // NEW: Services connect directly to broker
    
    let polygon_adapter = start_polygon_adapter(&broker.socket_path()).await;
    let flash_arbitrage = start_flash_arbitrage(&broker.socket_path()).await;
    
    // Test message flow through broker
    test_message_flow(&polygon_adapter, &flash_arbitrage).await;
}
```

## Acceptance Criteria

### Code Cleanup
- [ ] relays/ directory completely removed
- [ ] No relay-related dependencies in any Cargo.toml files
- [ ] All import statements updated to use broker clients
- [ ] No broken references or compilation errors

### Documentation Updates
- [ ] Architecture diagrams updated to show broker-only design
- [ ] README and setup instructions updated
- [ ] Legacy relay documentation removed or marked as deprecated
- [ ] API documentation reflects current broker architecture

### Script and Tool Updates
- [ ] System health check scripts updated for broker
- [ ] Start/stop scripts updated to use broker
- [ ] Monitoring scripts updated for new architecture
- [ ] Development workflow documentation updated

### Validation
- [ ] Full system builds successfully without relay components
- [ ] All tests pass with relay code removed
- [ ] Services start and run normally
- [ ] No references to removed relay code in logs or error messages

## Dependencies
- **Upstream**: MYC-007 (Integration Testing) - must be successful before removal
- **Downstream**: None (final cleanup task)
- **External**: None (internal cleanup)

## Testing Requirements

### Build Validation
```bash
# Verify clean build after removal
cargo clean
cargo build --workspace

# Verify no broken dependencies
cargo check --workspace

# Verify tests still pass
cargo test --workspace
```

### Runtime Validation
```bash
# Start system with relay code removed
./scripts/start_system.sh

# Verify all services connect to broker
./scripts/system_health_check.sh

# Run integration tests
cargo test --package integration --test end_to_end
```

### Reference Validation
```bash
# Search for any remaining relay references
grep -r "relay" --include="*.rs" --include="*.toml" --include="*.md" .
grep -r "RelayClient" --include="*.rs" .
grep -r "RelayDomain" --include="*.rs" .

# Should return only acceptable references (like "reliable", "related", etc.)
```

## Rollback Plan

### If Broken References Found
1. **Identify Missing Components**: Catalog what still depends on relay code
2. **Temporary Restoration**: Restore minimal relay components needed for compilation
3. **Complete Migration**: Finish migrating any remaining relay-dependent code
4. **Retry Removal**: Attempt cleanup again after dependencies resolved

### If System Stability Issues
1. **Restore Relay Directory**: Git restore the relay code from previous commit
2. **Parallel Operation**: Run relay and broker systems in parallel temporarily
3. **Gradual Transition**: Phase out relay usage service by service
4. **Final Removal**: Attempt removal again after stability confirmed

### If Performance Regression
1. **Performance Comparison**: Compare broker-only vs relay+broker performance
2. **Optimization**: Tune broker configuration for optimal performance  
3. **Selective Restoration**: Restore specific relay optimizations if needed
4. **Alternative Approach**: Consider keeping relay code as backup implementation

## Technical Notes

### Removal Strategy
- **Automated Detection**: Use grep and rq tools to find all relay references
- **Incremental Removal**: Remove directory first, then fix compilation errors
- **Validation at Each Step**: Ensure system still builds after each change
- **Documentation Last**: Update docs after code changes are stable

### Safety Considerations
- **Git History**: Relay code remains in git history for reference
- **Branch Backup**: Create backup branch before major removal
- **Rollback Capability**: Ensure quick rollback if critical issues found
- **Testing Coverage**: Extensive testing before and after removal

### Performance Impact
- **Reduced Memory**: Remove relay process memory overhead
- **Simplified Monitoring**: Fewer processes to monitor and manage
- **Cleaner Architecture**: Single message routing mechanism
- **Maintenance Reduction**: Less code to maintain and debug

## Validation Steps

1. **Pre-removal Audit**:
   ```bash
   # Document current relay usage
   find . -name "*.rs" -exec grep -l "relay" {} \; > relay_references_before.txt
   cargo tree | grep relay > relay_dependencies_before.txt
   ```

2. **Remove Relay Code**:
   ```bash
   git rm -rf relays/
   git commit -m "Remove legacy relay infrastructure"
   ```

3. **Fix Compilation**:
   ```bash
   cargo check --workspace 2>&1 | tee compilation_errors.txt
   # Fix each error systematically
   ```

4. **Update Dependencies**:
   ```bash
   # Remove relay entries from all Cargo.toml files
   find . -name "Cargo.toml" -exec sed -i '/relay/d' {} \;
   ```

5. **Final Validation**:
   ```bash
   cargo build --workspace
   cargo test --workspace  
   ./scripts/start_system.sh
   ./scripts/system_health_check.sh
   ```

This cleanup task completes the migration by removing all legacy relay infrastructure, leaving a clean, maintainable broker-based architecture that's simpler to understand, deploy, and maintain.
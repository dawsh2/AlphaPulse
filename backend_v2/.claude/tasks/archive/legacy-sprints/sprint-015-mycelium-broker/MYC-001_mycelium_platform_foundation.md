# MYC-001: Mycelium Platform Foundation

## Status
- **Status**: pending
- **Assignee**: TBD
- **Estimated Effort**: 1 day
- **Priority**: Critical (blocking all other tasks)

## Description
Set up the foundational Mycelium platform repository structure with Cargo workspace configuration and basic trait definitions. This creates the minimal infrastructure needed for parallel development of transport and broker layers.

## Objectives
1. Create clean repository structure for Mycelium platform
2. Configure Cargo workspace for multiple crates
3. Define core trait signatures for Transport and Broker
4. Establish basic project documentation
5. Set up development tooling and CI configuration

## Technical Approach

### Repository Structure
```
mycelium/
├── Cargo.toml                   # Workspace configuration
├── README.md                    # Platform overview
├── ARCHITECTURE.md              # Technical architecture
├── crates/
│   ├── mycelium-transport/      # Transport abstraction layer
│   │   ├── src/lib.rs
│   │   └── Cargo.toml
│   ├── mycelium-broker/         # Broker implementation
│   │   ├── src/lib.rs
│   │   └── Cargo.toml
│   └── mycelium-config/         # Configuration utilities
│       ├── src/lib.rs
│       └── Cargo.toml
├── examples/                    # Usage examples
│   └── basic_broker.rs
└── tests/                       # Integration tests
    └── end_to_end.rs
```

### Core Trait Definitions

#### Transport Trait
```rust
// mycelium-transport/src/lib.rs
pub trait Transport: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;
    
    async fn send(&self, data: &[u8]) -> Result<(), Self::Error>;
    async fn recv(&self) -> Result<Vec<u8>, Self::Error>;
    async fn close(&self) -> Result<(), Self::Error>;
}

pub trait Listener: Send + Sync {
    type Transport: Transport;
    type Error: std::error::Error + Send + Sync + 'static;
    
    async fn accept(&self) -> Result<Self::Transport, Self::Error>;
    async fn bind(addr: &str) -> Result<Self, Self::Error>;
}
```

#### Broker Trait
```rust
// mycelium-broker/src/lib.rs
pub trait Broker: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;
    
    async fn publish(&self, topic: &str, data: &[u8]) -> Result<(), Self::Error>;
    async fn subscribe(&self, topic: &str) -> Result<(), Self::Error>;
    async fn unsubscribe(&self, topic: &str) -> Result<(), Self::Error>;
}

#[derive(Debug, Clone)]
pub enum Topic {
    MarketData,
    Signals,
    Execution,
    Custom(String),
}
```

### Workspace Configuration
```toml
# Cargo.toml
[workspace]
members = [
    "crates/mycelium-transport",
    "crates/mycelium-broker", 
    "crates/mycelium-config"
]
resolver = "2"

[workspace.dependencies]
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"
anyhow = "1.0"
thiserror = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"

[workspace.package]
version = "0.1.0"
edition = "2021"
authors = ["Torq Team"]
license = "MIT"
repository = "https://github.com/torq/mycelium"
```

## Acceptance Criteria

### Repository Setup
- [ ] Repository created with proper directory structure
- [ ] Cargo workspace builds successfully (`cargo build`)
- [ ] All crates have proper Cargo.toml with dependencies
- [ ] Basic trait definitions compile without errors

### Documentation
- [ ] README.md with platform overview and quick start
- [ ] ARCHITECTURE.md with technical design decisions
- [ ] API documentation builds with `cargo doc`
- [ ] Examples directory with basic usage patterns

### Development Environment
- [ ] CI configuration (GitHub Actions or similar)
- [ ] Rustfmt and clippy configurations
- [ ] Basic integration test structure
- [ ] Development scripts for common tasks

### Integration Points
- [ ] Transport trait supports unix socket implementation
- [ ] Broker trait supports topic-based routing
- [ ] Configuration system supports TOML files
- [ ] Error types integrate with anyhow/thiserror

## Dependencies
- **Upstream**: None (foundational task)
- **Downstream**: All other MYC tasks depend on this
- **External**: Git repository creation, development environment setup

## Testing Requirements

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_trait_is_object_safe() {
        // Verify trait can be used as Box<dyn Transport>
    }

    #[test]
    fn broker_trait_supports_async() {
        // Verify async methods compile correctly
    }
}
```

### Integration Tests
```rust
// tests/workspace_integration.rs
#[tokio::test]
async fn workspace_builds_and_links() {
    // Verify all crates can be imported together
    use mycelium_transport::Transport;
    use mycelium_broker::Broker;
    use mycelium_config::Config;
}
```

### Documentation Tests
```bash
# Verify documentation builds
cargo doc --workspace --no-deps

# Verify examples compile
cargo build --examples
```

## Rollback Plan

### If Repository Setup Fails
1. Delete repository and restart with simpler structure
2. Use single crate instead of workspace if workspace issues persist
3. Manually create directory structure without cargo workspace

### If Trait Design Issues
1. Simplify traits to minimal required methods
2. Remove generic parameters if they cause compilation issues
3. Use concrete types instead of associated types if needed

## Technical Notes

### Design Decisions
- **Workspace over Single Crate**: Enables parallel development and clear separation
- **Trait-Based Design**: Allows multiple transport/broker implementations
- **Async by Default**: Modern Rust networking requires async
- **Error Trait Bounds**: Ensures proper error propagation and debugging

### Performance Considerations
- Trait methods use `&self` for zero-cost abstractions
- Associated types avoid runtime type erasure overhead
- Send + Sync bounds enable multi-threaded usage

### Future Extensibility
- Transport trait supports multiple protocols (unix socket, TCP, WebSocket)
- Broker trait can be extended with more complex routing patterns
- Topic enum supports both predefined and custom topics

## Validation Steps

1. **Repository Creation**:
   ```bash
   git clone <mycelium-repo>
   cd mycelium
   cargo build --workspace
   ```

2. **Documentation Validation**:
   ```bash
   cargo doc --workspace --open
   cargo test --doc
   ```

3. **Integration Testing**:
   ```bash
   cargo test --workspace
   cargo build --examples
   ```

4. **Development Workflow**:
   ```bash
   cargo fmt --all
   cargo clippy --workspace
   ```

This foundation enables parallel development of transport (MYC-002) and broker (MYC-003) layers while ensuring consistent interfaces and development practices.
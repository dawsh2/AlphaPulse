---
task_id: SINK-003
status: TODO
priority: CRITICAL
estimated_hours: 4
assigned_branch: feat/sinkfactory-stage1
assignee: TBD
created: 2025-08-26
completed: null
depends_on:
  - SINK-001  # Need MessageSink trait
  - SINK-002  # Need lazy wrapper implementation
blocks: []
scope:
  - "network/transport/src/factory/"  # SinkFactory implementation
  - "network/transport/src/config.rs"  # Configuration support
  - "services_v2/*/Cargo.toml"  # Update service dependencies
---

# SINK-003: Build SinkFactory with Configuration Support

## 🔴 CRITICAL: Bridge Between Stage 1 (Config) and Stage 2 (Mycelium)

### Git Worktree Setup
```bash
# Create worktree for this task
git worktree add -b feat/sinkfactory-stage1 ../messagesink-003
cd ../messagesink-003
```

## Status
**Status**: TODO  
**Priority**: CRITICAL - Central factory for all sink creation
**Branch**: `feat/sinkfactory-stage1`
**Estimated**: 4 hours
**Depends On**: SINK-001, SINK-002

## Problem Statement
We need a factory that:
- Creates MessageSinks based on configuration (Stage 1)
- Has a stable API that won't change when we add Mycelium (Stage 2)
- Supports different sink types (relay, direct, composite)
- Handles lazy wrapping automatically
- Manages sink lifecycle and caching

## Acceptance Criteria
- [ ] SinkFactory with stable public API
- [ ] Stage 1: Read from services.toml configuration
- [ ] Support relay, direct, and composite sink types
- [ ] Automatic lazy wrapping of all sinks
- [ ] Sink caching to avoid duplicate connections
- [ ] Clear migration path to Stage 2 (Mycelium)
- [ ] Comprehensive tests for all sink types

## Technical Design

### SinkFactory API (Stable Across Stage 1 & 2)
```rust
// libs/message_sink/src/factory.rs

/// Factory for creating MessageSinks
/// This API remains stable between Stage 1 (config) and Stage 2 (Mycelium)
pub struct SinkFactory {
    /// Stage 1: Config-based registry
    registry: Arc<ServiceRegistry>,
    
    /// Sink cache to avoid duplicates
    cache: Arc<RwLock<HashMap<String, Arc<dyn MessageSink>>>>,
    
    /// Default lazy configuration
    default_lazy_config: LazyConfig,
    
    // Stage 2 will add:
    // mycelium: Option<Arc<MyceliumApi>>,
}

impl SinkFactory {
    /// Create factory for Stage 1 (config-based)
    pub fn from_config(config_path: &Path) -> Result<Self, SinkError> {
        let registry = ServiceRegistry::from_file(config_path)?;
        Ok(Self {
            registry: Arc::new(registry),
            cache: Arc::new(RwLock::new(HashMap::new())),
            default_lazy_config: LazyConfig::default(),
        })
    }
    
    // Stage 2 will add:
    // pub fn from_mycelium(api: MyceliumApi) -> Self { ... }
    
    /// Main API: Create or retrieve a sink by service name
    /// THIS METHOD SIGNATURE NEVER CHANGES
    pub async fn create_sink(&self, service_name: &str) -> Result<Arc<dyn MessageSink>, SinkError> {
        // Check cache first
        if let Some(cached) = self.get_cached(service_name).await {
            return Ok(cached);
        }
        
        // Stage 1: Look up in config
        let config = self.registry.lookup(service_name)
            .ok_or_else(|| SinkError::InvalidConfig(
                format!("Service '{}' not found", service_name)
            ))?;
        
        // Stage 2 will replace above with:
        // let connection = self.mycelium.provision_connection(service_name).await?;
        
        // Create appropriate sink type
        let sink = self.create_from_config(config).await?;
        
        // Wrap in lazy wrapper
        let lazy_sink = self.wrap_lazy(sink);
        
        // Cache and return
        self.cache_sink(service_name, lazy_sink.clone()).await;
        Ok(lazy_sink)
    }
    
    /// Create sink from configuration
    async fn create_from_config(&self, config: ServiceConfig) -> Result<Box<dyn MessageSink>, SinkError> {
        match config.sink_type {
            SinkType::Relay => self.create_relay_sink(config).await,
            SinkType::Direct => self.create_direct_sink(config).await,
            SinkType::Composite => self.create_composite_sink(config).await,
        }
    }
}
```

### Stage 1 Configuration Structure
```rust
// libs/message_sink/src/config.rs

#[derive(Debug, Clone, Deserialize)]
pub struct ServicesConfig {
    pub services: HashMap<String, ServiceConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServiceConfig {
    /// Type of sink
    #[serde(rename = "type")]
    pub sink_type: SinkType,
    
    /// Connection endpoint
    pub endpoint: Option<String>,
    
    /// For composite sinks
    pub pattern: Option<CompositePattern>,
    pub targets: Option<Vec<String>>,
    
    /// Buffer configuration
    pub buffer_size: Option<usize>,
    
    /// Retry configuration
    pub max_retries: Option<u32>,
    pub retry_delay_ms: Option<u64>,
    
    /// Custom lazy config
    pub lazy: Option<LazyConfigToml>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SinkType {
    Relay,
    Direct,
    Composite,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompositePattern {
    Fanout,      // Send to all targets
    RoundRobin,  // Rotate between targets
    Failover,    // Primary with fallbacks
}
```

### Example services.toml
```toml
# Stage 1 configuration file
[services.market_data_relay]
type = "relay"
endpoint = "unix:///tmp/market_data_relay.sock"
buffer_size = 10000
max_retries = 3
retry_delay_ms = 100

[services.polygon_strategy]
type = "direct"
endpoint = "tcp://127.0.0.1:9001"
buffer_size = 5000

[services.dashboard]
type = "direct"
endpoint = "ws://127.0.0.1:8080/ws"

[services.broadcast_all]
type = "composite"
pattern = "fanout"
targets = ["market_data_relay", "dashboard"]

[services.failover_group]
type = "composite"
pattern = "failover"
targets = ["polygon_strategy", "backup_strategy"]

# Lazy connection config (optional)
[services.slow_service]
type = "direct"
endpoint = "tcp://remote:9999"
lazy.max_retries = 5
lazy.retry_delay_ms = 500
lazy.auto_reconnect = true
lazy.connect_timeout_secs = 10
```

### Sink Type Implementations
```rust
impl SinkFactory {
    /// Create relay-based sink
    async fn create_relay_sink(&self, config: ServiceConfig) -> Result<Box<dyn MessageSink>, SinkError> {
        let endpoint = config.endpoint
            .ok_or_else(|| SinkError::InvalidConfig("Relay requires endpoint".into()))?;
        
        let sink = RelaySink::new(
            &endpoint,
            config.buffer_size.unwrap_or(10000)
        )?;
        
        Ok(Box::new(sink))
    }
    
    /// Create direct connection sink
    async fn create_direct_sink(&self, config: ServiceConfig) -> Result<Box<dyn MessageSink>, SinkError> {
        let endpoint = config.endpoint
            .ok_or_else(|| SinkError::InvalidConfig("Direct requires endpoint".into()))?;
        
        let sink = match endpoint {
            e if e.starts_with("unix://") => {
                DirectSink::unix(&e[7..]).await?
            }
            e if e.starts_with("tcp://") => {
                DirectSink::tcp(&e[6..]).await?
            }
            e if e.starts_with("ws://") | e.starts_with("wss://") => {
                DirectSink::websocket(&e).await?
            }
            _ => return Err(SinkError::InvalidConfig(
                format!("Unknown endpoint type: {}", endpoint)
            ))
        };
        
        Ok(Box::new(sink))
    }
    
    /// Create composite sink
    async fn create_composite_sink(&self, config: ServiceConfig) -> Result<Box<dyn MessageSink>, SinkError> {
        let pattern = config.pattern
            .ok_or_else(|| SinkError::InvalidConfig("Composite requires pattern".into()))?;
        
        let target_names = config.targets
            .ok_or_else(|| SinkError::InvalidConfig("Composite requires targets".into()))?;
        
        // Recursively create target sinks
        let mut targets = Vec::new();
        for target in target_names {
            let sink = self.create_sink(&target).await?;
            targets.push(sink);
        }
        
        let sink = match pattern {
            CompositePattern::Fanout => {
                CompositeSink::fanout(targets)
            }
            CompositePattern::RoundRobin => {
                CompositeSink::round_robin(targets)
            }
            CompositePattern::Failover => {
                CompositeSink::failover(targets)
            }
        };
        
        Ok(Box::new(sink))
    }
}
```

### Migration Path to Stage 2
```rust
// Future Stage 2 implementation (DO NOT IMPLEMENT NOW)
impl SinkFactory {
    /// Stage 2: Create from Mycelium API
    pub fn from_mycelium(api: MyceliumApi) -> Self {
        Self {
            registry: None, // No longer needed
            mycelium: Some(Arc::new(api)),
            cache: Arc::new(RwLock::new(HashMap::new())),
            default_lazy_config: LazyConfig::default(),
        }
    }
    
    /// Same public API, different internals
    pub async fn create_sink(&self, service_name: &str) -> Result<Arc<dyn MessageSink>, SinkError> {
        // Check cache (same as Stage 1)
        if let Some(cached) = self.get_cached(service_name).await {
            return Ok(cached);
        }
        
        // Stage 2: Ask Mycelium for connection
        let connection = self.mycelium
            .as_ref()
            .unwrap()
            .provision_connection(service_name)
            .await?;
        
        // Wrap Mycelium connection in MessageSink
        let sink = MyceliumSink::new(connection);
        
        // Rest is identical to Stage 1
        let lazy_sink = self.wrap_lazy(Box::new(sink));
        self.cache_sink(service_name, lazy_sink.clone()).await;
        Ok(lazy_sink)
    }
}
```

## Testing Requirements

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_factory_creates_relay_sink() {
        let config = r#"
            [services.test_relay]
            type = "relay"
            endpoint = "unix:///tmp/test.sock"
        "#;
        
        let factory = SinkFactory::from_config_str(config).unwrap();
        let sink = factory.create_sink("test_relay").await.unwrap();
        
        assert!(sink.metadata().sink_type == "relay");
    }
    
    #[tokio::test]
    async fn test_factory_caches_sinks() {
        let factory = SinkFactory::from_config(/* ... */);
        
        let sink1 = factory.create_sink("service_a").await.unwrap();
        let sink2 = factory.create_sink("service_a").await.unwrap();
        
        // Should be same instance (Arc pointers equal)
        assert!(Arc::ptr_eq(&sink1, &sink2));
    }
    
    #[tokio::test]
    async fn test_composite_fanout() {
        // Test that fanout sends to all targets
    }
}
```

## Files to Create/Modify
- `libs/message_sink/src/factory.rs` - Main factory implementation
- `libs/message_sink/src/config.rs` - Configuration structures
- `libs/message_sink/src/registry.rs` - Service registry
- `libs/message_sink/src/sinks/` - Concrete sink implementations
- `services.toml` - Example configuration

## Completion Checklist
- [ ] Worktree created
- [ ] SinkFactory with stable API
- [ ] ServiceRegistry for config reading
- [ ] All sink types implemented
- [ ] Lazy wrapping automatic
- [ ] Sink caching working
- [ ] services.toml example created
- [ ] Tests for all sink types
- [ ] Clear Stage 2 migration path documented
- [ ] PR created
- [ ] Status updated to COMPLETE

## Why This Matters
The SinkFactory is the critical abstraction that:
- Allows Stage 1 implementation today with config files
- Enables seamless Stage 2 migration to Mycelium
- Services NEVER need to change when migrating
- Provides consistent sink creation across the system
- Makes testing easy with config-based setup
# AlphaPulse Declarative Topology Implementation Plan

## Overview

This directory implements a declarative topology system that separates logical service contracts (actors) from physical deployment (nodes), enabling optimal transport selection and NUMA-aware placement.

## Architecture Layers

### 1. Actor Layer (Logical)
- **Purpose**: Define service contracts independent of deployment
- **Components**: Actor definitions, input/output channels, TLV type requirements
- **Files**: `src/actors.rs`, `config/actors.yaml`

### 2. Node Layer (Physical) 
- **Purpose**: Hardware-specific deployment with transport optimization
- **Components**: NUMA topology, CPU affinity, local channels, inter-node routing
- **Files**: `src/nodes.rs`, `config/nodes.yaml`

### 3. Resolution Layer
- **Purpose**: Bridge logical and physical, resolve optimal transports
- **Components**: Transport resolution, deployment generation, validation
- **Files**: `src/resolution.rs`, `src/deployment.rs`

## Implementation Phases

### Phase 1: Foundation (Current)
- [x] Create basic project structure
- [ ] Define core types and traits
- [ ] Implement YAML configuration loading
- [ ] Basic actor/node definitions
- **Goal**: Load and validate topology configurations

### Phase 2: Protocol Integration
- [ ] Integrate with protocol_v2 TLV types
- [ ] RelayDomain-aware routing
- [ ] Message bus configuration generation
- [ ] Transport resolution engine
- **Goal**: Generate deployment configs from topology

### Phase 3: Runtime System
- [ ] Actor factory and lifecycle management
- [ ] NUMA-aware process placement
- [ ] Shared memory channel creation
- [ ] Network transport setup
- **Goal**: Actually deploy and run actors

### Phase 4: Advanced Features
- [ ] Dynamic actor migration
- [ ] Load balancing and auto-scaling
- [ ] Health monitoring and recovery
- [ ] Performance metrics collection
- **Goal**: Production-ready topology management

## Directory Structure

```
topology/
├── PLAN.md                    # This file
├── Cargo.toml                 # Project dependencies
├── src/
│   ├── lib.rs                # Public API
│   ├── actors.rs             # Actor definitions and types
│   ├── nodes.rs              # Node configuration and placement
│   ├── resolution.rs         # Transport resolution logic
│   ├── deployment.rs         # Deployment engine
│   ├── runtime.rs            # Actor runtime system
│   ├── config.rs             # YAML configuration loading
│   └── error.rs              # Error types
├── examples/
│   ├── single_node.yaml      # Simple single-node deployment
│   ├── multi_node.yaml       # Multi-node with network transport
│   └── flash_arbitrage.yaml  # Flash arbitrage strategy example
├── tests/
│   ├── config_validation.rs  # Configuration validation tests
│   ├── transport_resolution.rs # Transport selection tests
│   └── deployment_tests.rs   # End-to-end deployment tests
└── benches/
    └── resolution.rs          # Performance benchmarks
```

## Key Design Decisions

### 1. Actor Types
```rust
pub enum ActorType {
    Producer,    // Data sources (collectors, feeds)
    Transformer, // Processing services (strategies, analyzers)
    Consumer,    // Sinks (executors, databases)
}
```

### 2. Transport Selection
- **Same Node**: Shared memory with NUMA optimization
- **Different Nodes**: TCP with compression and routing
- **Automatic**: Based on actor placement in topology

### 3. Configuration Format
- **YAML**: Human-readable, version controllable
- **Validation**: Schema validation with detailed error messages
- **Templating**: Support for environment variable substitution

### 4. Integration Points
- **Protocol_v2**: TLV message routing via RelayDomains
- **Services_v2**: Actor implementations from adapters/strategies
- **Message Bus**: Transport-agnostic message delivery

## Development Workflow

### Adding New Actor Types
1. Define actor in `actors.yaml`
2. Implement `ActorRuntime` trait
3. Register in `ActorFactory`
4. Add to example configurations

### Adding New Transport Types
1. Extend `Transport` enum
2. Implement resolution logic
3. Add deployment configuration
4. Update examples and tests

### Testing Strategy
- **Unit Tests**: Individual component logic
- **Integration Tests**: Actor deployment and communication
- **Property Tests**: Configuration validation invariants
- **Performance Tests**: Transport resolution benchmarks

## Example Configurations

### Single-Node Development
```yaml
# For development and testing
actors:
  polygon_collector: {type: producer, outputs: [market_data]}
  flash_arbitrage: {type: transformer, inputs: [market_data], outputs: [signals]}

nodes:
  dev_node:
    hostname: localhost
    actors: {polygon_collector: {cpu: [0]}, flash_arbitrage: {cpu: [1]}}
    local_channels:
      market_data: {type: SPMC, buffer_size: "64MB"}
```

### Production Multi-Node
```yaml
# For production deployment
nodes:
  data_node:
    hostname: "data-01.prod"
    numa_topology: [0, 1]
    actors:
      polygon_collector: {numa: 0, cpu: [0,1]}
      binance_collector: {numa: 0, cpu: [2,3]}
      
  strategy_node:
    hostname: "strategy-01.prod"
    actors:
      flash_arbitrage: {numa: 0, cpu: [0,1,2,3]}
      risk_monitor: {numa: 1, cpu: [4,5]}

inter_node:
  market_data_feed:
    source: data_node.market_data
    targets: [strategy_node]
    transport: tcp
    compression: lz4
```

## Integration Timeline

### Immediate (Phase 1)
- **No dependencies**: Can be developed standalone
- **Configuration only**: Focus on YAML loading and validation
- **Testing**: Validate topology configurations work correctly

### Short-term (Phase 2)
- **Protocol_v2 integration**: Once TLV system is stable
- **Message bus**: When transport layer is mature
- **Basic deployment**: Generate configs for manual deployment

### Medium-term (Phase 3)
- **Services_v2 integration**: When actor implementations exist
- **Runtime deployment**: Actually spawn and manage processes
- **NUMA optimization**: Hardware-specific optimizations

### Long-term (Phase 4)
- **Production deployment**: When system is mature enough
- **Dynamic management**: Live topology changes
- **Monitoring integration**: Health and performance metrics

## Success Metrics

### Phase 1 Success
- Load complex topology YAML without errors
- Validate actor dependencies and node resources
- Generate meaningful error messages for invalid configs

### Phase 2 Success
- Resolve optimal transport for any actor pair
- Generate protocol_v2 compatible message bus configs
- Route TLV messages correctly through RelayDomains

### Phase 3 Success
- Deploy flash arbitrage strategy across multiple nodes
- Achieve <35μs latency for intra-node communication
- Maintain <5ms latency for inter-node communication

### Phase 4 Success
- Migrate actors between nodes without data loss
- Auto-scale based on load and performance metrics
- Maintain 99.99% uptime during topology changes

## Risk Mitigation

### Configuration Complexity
- **Risk**: YAML becomes too complex for operators
- **Mitigation**: Provide templates, validation, and clear documentation

### Performance Overhead
- **Risk**: Topology abstraction adds latency
- **Mitigation**: Zero-cost abstractions, compile-time optimization

### Protocol Changes
- **Risk**: Breaking changes in protocol_v2 affect topology
- **Mitigation**: Version compatibility, migration tools

### Hardware Dependencies
- **Risk**: NUMA/CPU features not available on all systems
- **Mitigation**: Graceful fallbacks, feature detection

## Future Extensions

### Kubernetes Integration
- Generate Kubernetes manifests from topology
- Pod affinity and resource allocation
- Service mesh integration

### Cloud Deployment
- AWS placement groups and enhanced networking
- GCP regional deployment optimization
- Azure proximity placement groups

### Observability
- Distributed tracing across actor boundaries
- Performance metrics per transport type
- Topology visualization and debugging tools

This plan provides a roadmap for implementing the declarative topology system while maintaining compatibility with our existing protocol_v2 and services_v2 architecture.
# Actor-Node Deployment Architecture

## Overview

AlphaPulse uses a two-layer deployment model: **logical actors** define service contracts and data flow, while **physical nodes** specify hardware placement and transport optimization.

## Actor Specification (Logical Layer)

Defines service contracts independent of deployment topology:

```yaml
# actors.yaml
actors:
  polygon_collector:
    type: producer
    outputs: [market_data]
    source_id: 1
    
  arbitrage_strategy:
    type: transformer
    inputs: [market_data]
    outputs: [signals]
    source_id: 20
    
  execution_coordinator:
    type: consumer
    inputs: [signals]
    source_id: 40
```

**Purpose**: Development contracts, testing, service discovery

## Node Graph (Physical Layer)

Maps actors to hardware with transport-specific optimizations:

```yaml
# nodes.yaml
nodes:
  trading_primary:
    hostname: "trade-01"
    numa_topology: [0, 1]
    
    # Intra-node: shared memory
    local_channels:
      market_data:
        type: SPMC
        buffer_size: "1GB"
        numa_node: 0
        huge_pages: true
        
    # Actor placement
    actors:
      polygon_collector: {numa: 0, cpu: [0,1]}
      arbitrage_strategy: {numa: 0, cpu: [2,3]}
      execution_coordinator: {numa: 1, cpu: [8,9]}
      
  analytics_cluster:
    hostname: "analytics-01"
    actors:
      risk_monitor: {cpu: [0,1]}
      
# Inter-node: network transport
inter_node:
  market_data_feed:
    source: trading_primary.market_data
    targets: [analytics_cluster]
    transport: tcp
    compression: lz4
```

**Purpose**: Hardware optimization, NUMA placement, transport selection

## Transport Resolution

The deployment engine resolves transport based on actor placement:

```rust
fn resolve_transport(source_actor: &Actor, target_actor: &Actor) -> Transport {
    match (source_actor.node_id, target_actor.node_id) {
        (a, b) if a == b => Transport::SharedMemory,  // Same node
        (a, b) => Transport::Network(tcp_config),     // Different nodes
    }
}
```

## Migration Strategy

**Phase 1**: Single-node deployment, all shared memory
**Phase 2**: Multi-node with explicit inter-node channels  
**Phase 3**: Dynamic actor migration and load balancing

## Benefits

- **Logical**: Clean service contracts, testable in isolation
- **Physical**: NUMA-aware, transport-optimized, hardware-specific
- **Deployment**: Infrastructure-as-code, reproducible environments
- **Performance**: Microsecond IPC where possible, efficient network where required

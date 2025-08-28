# Network Module Organizational Analysis

## Current State: Organizational Disaster

The `network/src` directory suffers from severe organizational issues resulting from merging multiple crates (`torq-transport`, `torq-topology`, `torq-network`) without proper consolidation.

## Current Module Structure & Issues

### 1. `hybrid/` - Hybrid Transport Module
- **Purpose**: Claims to combine direct network and message queue transport
- **Problems**:
  - Contains a `HybridTransport` struct
  - Duplicates transport selection logic found elsewhere
  - Originally from `torq-transport` crate (consolidated but not cleaned up)
  - Unclear relationship with root-level `transport.rs`

### 2. `mycelium/` - Actor Runtime
- **Purpose**: Zero-cost actor runtime with adaptive transport selection
- **Problems**:
  - Has its own `ActorTransport` and transport selection logic
  - Contains proof_of_concept.rs file (experimental code in production?)
  - Overlaps conceptually with topology's actor placement
  - Includes message types that should be in protocol/codec layers
  - Competing with topology module's actor system

### 3. `network/` - Basic Network Transports
- **Purpose**: TCP, UDP, Unix socket implementations
- **Problems**:
  - Contains actual transport implementations (good)
  - Has its own `NetworkTransport` wrapper (redundant)
  - Includes compression, security, envelope - unclear if these should be shared
  - Name collision with parent module

### 4. `topology/` - Service Placement and Discovery
- **Purpose**: Declarative topology system for service placement
- **Problems**:
  - Originally from `torq-topology` crate
  - Contains actors, nodes, deployment concepts
  - Has its own transport selection logic in transport.rs
  - Overlaps with mycelium's actor concepts
  - Unclear separation of concerns

### 5. `topology_integration/` - Bridge Module
- **Purpose**: Integrate transport with topology system
- **Problems**:
  - Yet another layer of transport selection/resolution
  - `TransportFactory` and `TopologyTransportResolver`
  - Unclear why this needs its own module vs being part of topology
  - Adds complexity without clear value

### 6. `transport.rs` (root level) - Transport Abstractions
- **Problems**:
  - Re-exports from network/ module
  - Has ANOTHER `HybridTransport` implementation (different from hybrid/)
  - Creates confusion about which transport abstraction to use

## Key Problems Identified

1. **Multiple HybridTransport Implementations**
   - One in `hybrid/mod.rs`
   - One in `transport.rs`
   - Unclear which is canonical

2. **Three Competing Actor Systems**
   - mycelium actors (with ActorSystem, ActorBehavior)
   - topology actors (with Actor, Node definitions)
   - Unclear separation of responsibilities

3. **Four Transport Selection Mechanisms**
   - `hybrid/router.rs` - RouteDecision logic
   - `mycelium/transport.rs` - ActorTransport selection
   - `topology/transport.rs` - Topology-based selection
   - `topology_integration/factory.rs` - TransportFactory resolution

4. **Scattered Responsibilities**
   - Transport logic spread across 6 modules
   - No clear ownership of functionality
   - Duplicate code and concepts

5. **Incomplete Consolidation**
   - Comments indicate merging from multiple crates
   - APIs preserved for "backward compatibility"
   - No actual cleanup performed

## Proposed Clean Organization

```
network/src/
├── transports/           # All transport implementations
│   ├── mod.rs           # Transport trait & common types
│   ├── tcp.rs           # TCP transport
│   ├── udp.rs           # UDP transport  
│   ├── unix.rs          # Unix socket transport
│   └── quic.rs          # QUIC transport (future)
│
├── routing/             # Transport selection & routing
│   ├── mod.rs          # Router trait
│   ├── latency.rs      # Latency-based routing
│   ├── topology.rs     # Topology-aware routing
│   └── hybrid.rs       # Hybrid selection logic
│
├── actors/             # Actor system (pick ONE)
│   ├── mod.rs         # Actor traits & runtime
│   ├── behavior.rs    # Actor behaviors
│   ├── supervision.rs # Supervision trees
│   └── registry.rs    # Actor registry
│
├── discovery/          # Service discovery & topology
│   ├── mod.rs         # Discovery traits
│   ├── static.rs      # Static topology config
│   ├── dynamic.rs     # Dynamic discovery
│   └── placement.rs   # NUMA-aware placement
│
├── protocol/          # Wire protocol & serialization
│   ├── mod.rs        # Protocol traits
│   ├── envelope.rs   # Message envelopes
│   ├── compression.rs # Compression engines
│   └── security.rs   # Encryption/auth
│
├── metrics.rs        # Performance metrics
├── error.rs         # Error types
└── lib.rs          # Public API
```

## Refactoring Plan

### Phase 1: Consolidate Transports
1. Move all actual transport implementations to `transports/`
2. Create single Transport trait
3. Remove duplicate HybridTransport implementations

### Phase 2: Unify Routing
1. Merge all transport selection logic into `routing/`
2. Create clear routing strategy interface
3. Remove redundant selection mechanisms

### Phase 3: Choose Actor System
1. Evaluate mycelium vs topology actors
2. Pick ONE system and remove the other
3. Consolidate actor-related code

### Phase 4: Clarify Service Discovery
1. Separate discovery from transport selection
2. Move topology concepts to discovery/
3. Remove topology_integration module

### Phase 5: Infrastructure Consolidation
1. Move wire format, compression, security to protocol/
2. Create clear protocol abstraction
3. Remove scattered protocol logic

### Phase 6: Cleanup
1. Delete all duplicate code
2. Update all imports and dependencies
3. Write comprehensive tests for new structure

## Impact Assessment

### Breaking Changes Required
- All imports will change
- Transport initialization will be different
- Actor system APIs will change

### Benefits
- Clear module boundaries
- No duplicate code
- Single source of truth for each concept
- Easier to understand and maintain
- Better performance (less indirection)

## Recommendation

This reorganization is critical for long-term maintainability. The current structure is unsustainable and will lead to bugs and performance issues. Since CLAUDE.md states "Breaking changes are welcome" for this greenfield codebase, we should proceed with the full refactoring.

## Next Steps

1. Get buy-in on proposed structure
2. Create feature branch for refactoring
3. Execute phase-by-phase with tests
4. Update all dependent services
5. Remove old code completely
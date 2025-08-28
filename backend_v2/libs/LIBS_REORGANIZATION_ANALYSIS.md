# Libraries Directory Reorganization Analysis

## Current State

The `libs/` directory is intended for shared libraries but has accumulated various components that don't belong here, creating confusion about what constitutes a "shared library" vs service-specific code.

## Library Analysis

### 1. `amm/` - AMM Mathematics
- **Purpose**: AMM math for V2/V3 pools, optimal sizing calculations
- **Current Location**: libs/amm/
- **Should Be**: Keep in libs/ for now, but move to future `execution/` module when created
- **Reason**: Core math used across multiple services (strategies, validation, execution)

### 2. `codec/` - Protocol Codec
- **Purpose**: TLV message encoding/decoding, Protocol V2 implementation
- **Current Location**: libs/codec/
- **Should Be**: Keep in libs/
- **Reason**: Core protocol layer used by ALL services

### 3. `config/` - Centralized Configuration
- **Purpose**: Constants for blockchain, protocol, financial, service configs
- **Current Location**: libs/config/
- **Should Be**: Keep in libs/ but needs cleanup
- **Reason**: Truly shared configuration across all services

### 4. `dex/` - DEX ABIs and Event Decoding
- **Purpose**: DEX event signatures, ABI definitions, event decoders
- **Current Location**: libs/dex/
- **Should Be**: Move to `services_v2/adapters/dex_utils/`
- **Reason**: Only used by polygon adapter for external data format parsing

### 5. `health_check/` - Health Check System
- **Purpose**: Service health monitoring, HTTP endpoints, metrics collection
- **Current Location**: libs/health_check/
- **Should Be**: Move to `services_v2/observability/health_check/`
- **Reason**: Observability concern, belongs with monitoring and tracing

### 6. `message_sink/` - Message Routing
- **Purpose**: Message routing to relays, lazy connections, circuit breaking
- **Current Location**: libs/message_sink/
- **Should Be**: Keep in libs/ for now
- **Reason**: Used by multiple adapters to send messages to relays

### 7. `mev/` - MEV Protection
- **Purpose**: Bundle creation, flashbots integration, MEV protection
- **Current Location**: libs/mev/
- **Should Be**: Move to `services_v2/strategies/mev/` or future `execution/` module
- **Reason**: Strategy/execution specific, not shared infrastructure

### 8. `service_discovery/` - Service Discovery
- **Purpose**: Dynamic service endpoint resolution, health checking
- **Current Location**: libs/service_discovery/
- **Should Be**: Delete - network layer handles this now
- **Reason**: Duplicate functionality with network/topology, comment says "TODO: MOVE TO MYCELIUM"

### 9. `state/` - State Management
- **Purpose**: Market state, pool cache, pool state tracking
- **Current Location**: libs/state/
- **Should Be**: Move `market/` to `services_v2/strategies/state/`, delete `core/`
- **Reason**: Failed experiment per your comment, market state is strategy-specific

### 10. `time/` - Cached Clock System
- **Purpose**: High-performance cached timestamps to avoid syscalls
- **Current Location**: libs/time/
- **Should Be**: Move to `network/time/` or keep in libs/
- **Reason**: Could be infrastructure (network) or truly shared (libs)

### 11. `types/` - Core Protocol Types
- **Purpose**: Protocol types, TLV definitions, identifiers, precision handling
- **Current Location**: libs/types/
- **Should Be**: Keep in libs/ but needs major cleanup
- **Reason**: Core types used everywhere, but has accumulated too much

## Problems Identified

1. **Service-Specific Code in libs/**:
   - `dex/` is only for polygon adapter
   - `mev/` is strategy/execution specific
   - `state/market/` is strategy specific

2. **Infrastructure Mixed with Libraries**:
   - `health_check/` is infrastructure
   - `service_discovery/` duplicates network functionality
   - `time/` could be infrastructure

3. **Failed Experiments**:
   - `state/core/` is empty (failed idea)
   - `service_discovery/` marked for removal

4. **Unclear Boundaries**:
   - When does math belong in libs vs services?
   - When does state belong in libs vs services?

## Proposed Organization

### Keep in `libs/` (True Shared Libraries)
```
libs/
├── amm/           # AMM math (until execution module exists)
├── codec/         # Protocol V2 codec
├── config/        # Centralized configuration
├── message_sink/  # Message routing to relays
└── types/         # Core protocol types (needs cleanup)
```

### Move to Services
```
services_v2/
├── adapters/
│   └── dex_utils/     # Move dex/ here (DEX ABIs, events)
└── strategies/
    ├── mev/          # Move mev/ here
    └── state/        # Move state/market/ here
```

### Move to Network or Observability
```
network/
└── time/            # Move time/ here (or keep in libs/)

services_v2/observability/
└── health_check/    # Move health_check/ here
```

### Delete
- `libs/service_discovery/` - Duplicate of network/topology
- `libs/state/core/` - Empty, failed experiment

## Migration Plan

### Phase 1: Quick Wins (Delete/Move obvious items)
1. Delete `service_discovery/` (marked TODO: MOVE TO MYCELIUM)
2. Delete `state/core/` (empty)
3. Move `dex/` → `services_v2/adapters/dex_utils/`

### Phase 2: Service-Specific Moves
1. Move `mev/` → `services_v2/strategies/mev/`
2. Move `state/market/` → `services_v2/strategies/state/`

### Phase 3: Observability Consolidation
1. Move `health_check/` → `services_v2/observability/health_check/`
2. Integrate with existing trace_collector service
3. Decide on `time/` location (network/ or keep in libs/)

### Phase 4: Cleanup Remaining
1. Clean up `types/` (too much accumulated)
2. Clean up `config/` (organize better)
3. Document clear criteria for libs/ vs services/

## Decision Criteria

### What belongs in `libs/`?
- Used by 3+ services
- Core protocol functionality
- Pure utility functions (no business logic)
- No service-specific dependencies

### What belongs in `services/`?
- Used by 1-2 services only
- Business logic specific to a domain
- External data format handling
- Strategy-specific calculations

### What belongs in `network/`?
- Network transport and routing
- Service discovery and topology
- Performance optimization utilities
- Time synchronization (possibly)

### What belongs in `observability/`?
- Health checking and monitoring
- Metrics collection
- Tracing and telemetry
- Service status endpoints

## Impact Assessment

### Breaking Changes
- All imports will need updates
- Service dependencies will change
- Some libraries may need interface changes

### Benefits
- Clear separation of concerns
- Easier to understand codebase structure
- Reduced coupling between services
- Better testability

## Next Steps

1. Get agreement on proposed structure
2. Create migration branches for each phase
3. Update imports systematically
4. Update documentation
5. Remove all legacy code
# Relay Architecture - Clean Organization

## Overview

The relay infrastructure has been reorganized from a mixed monolithic/micro-service pattern into a clean, layered architecture with proper separation of concerns.

## Before: Mixed Architecture Issues

### Problems Identified
- **Dual Implementation Pattern**: Both `signal.rs` and `signal_relay.rs` with unclear ownership
- **Infrastructure Mixed with Business Logic**: Transport, validation, and topics mixed with domain logic
- **Configuration Scattered**: Domain configs at root level with no environment separation
- **Test Organization Disaster**: Executable files mixed with .rs tests, no categorization
- **Unclear Dependencies**: No clear API boundaries between library and binaries

## After: Clean Layered Architecture

```
relays/
├── core/                           # Infrastructure Layer
│   ├── src/
│   │   ├── transport/              # Transport adapters
│   │   ├── validation/             # Message validation policies
│   │   ├── topics/                 # Topic-based routing
│   │   ├── common/                 # Shared utilities
│   │   ├── config.rs               # Configuration management  
│   │   ├── message_construction.rs # TLV message building
│   │   └── types.rs                # Core type definitions
│   └── Cargo.toml
│
├── domains/                        # Domain Layer
│   ├── market_data/                # Market Data domain (TLV 1-19)
│   │   ├── src/
│   │   │   └── lib.rs              # Domain logic implementation
│   │   └── Cargo.toml
│   ├── signal/                     # Signal domain (TLV 20-39)  
│   │   ├── src/
│   │   │   ├── lib.rs              # Domain logic
│   │   │   └── relay.rs            # Full service implementation
│   │   └── Cargo.toml
│   └── execution/                  # Execution domain (TLV 40-79)
│       ├── src/
│       │   └── lib.rs              # Domain logic implementation
│       └── Cargo.toml
│
├── config/                         # Configuration Layer
│   ├── environments/
│   │   ├── development/            # Dev-specific configs
│   │   ├── staging/                # Staging configs
│   │   └── production/             # Production-tuned configs
│   └── templates/                  # Config templates
│
├── testing/                        # Testing Layer
│   ├── unit/                       # Unit tests
│   ├── integration/
│   │   ├── local/                  # Local integration tests
│   │   └── live/                   # Live external service tests
│   ├── benchmarks/                 # Performance tests
│   ├── fixtures/                   # Test data
│   └── performance/                # Performance results
│
├── docs/                           # Documentation Layer
│   ├── architecture/               # Architecture docs
│   ├── performance/                # Performance analysis
│   └── protocols/                  # Protocol documentation
│
└── deployment/                     # Deployment Layer
    ├── docker/                     # Container definitions
    ├── k8s/                        # Kubernetes manifests
    └── scripts/                    # Deployment automation
```

## Benefits of New Architecture

### 1. Clear Separation of Concerns
- **Core Infrastructure**: Reusable components shared across all domains
- **Domain Logic**: Specific implementations isolated by message type ranges
- **Configuration**: Environment-specific settings properly organized
- **Testing**: Clear categorization by test type and scope

### 2. No Duplicate Implementations
- Eliminated `signal.rs` vs `signal_relay.rs` confusion
- Single canonical implementation per concept
- Clear API boundaries between layers

### 3. Environment-Specific Configuration
- Development: Relaxed settings for debugging
- Production: Performance-tuned with security hardening
- Staging: Production-like with additional logging

### 4. Proper Testing Organization
- **Unit Tests**: Fast, isolated component testing
- **Local Integration**: Component interaction testing
- **Live Integration**: External service validation (optional)
- **Benchmarks**: Performance regression detection

### 5. Better Performance Monitoring
- Centralized benchmark organization
- Performance results properly documented
- Clear performance regression detection

### 6. Cleaner Deployment Strategy
- Environment-specific deployment configurations
- Container and orchestration support
- Automated deployment scripts

## Domain-Specific Optimizations

### Market Data Domain
- **Validation Policy**: Performance (no checksums)
- **Buffer Sizes**: Large (128KB) for high throughput
- **Target**: >1M messages/second
- **Use Case**: High-frequency market events

### Signal Domain  
- **Validation Policy**: Reliability (CRC32 checksums)
- **Buffer Sizes**: Medium (32KB) for balanced performance
- **Target**: >100K messages/second  
- **Use Case**: Trading signals and strategy outputs

### Execution Domain
- **Validation Policy**: Security (full audit trail)
- **Buffer Sizes**: Small (16KB) for security focus
- **Target**: >50K messages/second
- **Use Case**: Order execution with compliance requirements

## Dependency Flow

```
Binary Applications
        ↓
    Domain Crates (market_data, signal, execution)  
        ↓
    Core Infrastructure (torq-relay-core)
        ↓
    Protocol Libraries (torq-codec, torq-types)
        ↓
    Network Transport (torq-network)
```

## Migration Benefits

1. **Maintainability**: Clear structure makes adding new relay domains straightforward
2. **Testing**: Organized testing framework supports reliable CI/CD
3. **Performance**: Domain-specific optimizations improve throughput
4. **Deployment**: Environment-specific configurations enable proper staging
5. **Documentation**: Structured docs support team knowledge sharing

## Breaking Changes

The reorganization introduces breaking changes to:
- **Import paths**: All relay components have new locations
- **Configuration locations**: Environment-specific config files
- **Binary locations**: Domain binaries moved to respective crates
- **Test execution**: New test organization affects CI/CD scripts

These breaking changes are acceptable per CLAUDE.md's greenfield guidance and significantly improve the system's maintainability and performance characteristics.
# Network Transport Implementation Plan

## Overview
Implement a comprehensive network transport system under `backend_v2/transport/` that integrates with the existing topology system and provides optional message queue capabilities.

## Project Structure
```
backend_v2/
├── transport/                    # New network transport module
│   ├── PLAN.md                  # This implementation plan
│   ├── Cargo.toml               # Transport dependencies  
│   ├── src/
│   │   ├── lib.rs               # Public API and re-exports
│   │   ├── error.rs             # Transport-specific errors
│   │   ├── network/             # Core network transport
│   │   │   ├── mod.rs           # Network module exports
│   │   │   ├── tcp.rs           # TCP transport implementation
│   │   │   ├── udp.rs           # UDP transport for ultra-low latency
│   │   │   ├── quic.rs          # QUIC transport for modern networking
│   │   │   ├── connection.rs    # Connection management and pooling
│   │   │   ├── envelope.rs      # Wire protocol message envelope
│   │   │   ├── compression.rs   # LZ4/Zstd/Snappy compression
│   │   │   └── security.rs      # TLS/ChaCha20Poly1305 encryption
│   │   ├── mq/                  # Message queue integration
│   │   │   ├── mod.rs           # MQ module exports
│   │   │   ├── rabbitmq.rs      # RabbitMQ integration
│   │   │   ├── kafka.rs         # Kafka integration
│   │   │   ├── redis.rs         # Redis Streams integration
│   │   │   └── traits.rs        # MQ trait abstractions
│   │   ├── hybrid/              # Hybrid direct/MQ system
│   │   │   ├── mod.rs           # Hybrid transport orchestration
│   │   │   ├── router.rs        # Routing between direct/MQ
│   │   │   ├── config.rs        # Transport mode configuration
│   │   │   └── bridge.rs        # Direct-to-MQ bridging
│   │   ├── topology_integration/ # Integration with topology system
│   │   │   ├── mod.rs           # Integration module
│   │   │   ├── resolver.rs      # Enhanced topology resolver
│   │   │   └── factory.rs       # Transport factory
│   │   └── monitoring/          # Transport monitoring and metrics
│   │       ├── mod.rs           # Monitoring exports
│   │       ├── metrics.rs       # Performance metrics
│   │       ├── health.rs        # Health checks and circuit breakers
│   │       └── tracing.rs       # Distributed tracing support
│   ├── examples/
│   │   ├── direct_tcp.yaml      # Direct TCP configuration
│   │   ├── hybrid_mq.yaml       # Hybrid direct/MQ configuration
│   │   ├── multi_protocol.yaml  # Multi-protocol example
│   │   └── performance_tuned.yaml # Performance-optimized config
│   ├── tests/
│   │   ├── integration/         # Integration tests
│   │   ├── network/             # Network transport tests
│   │   ├── mq/                  # Message queue tests
│   │   └── topology/            # Topology integration tests
│   └── benches/                 # Performance benchmarks
│       ├── latency.rs           # Latency measurements
│       ├── throughput.rs        # Throughput benchmarks
│       └── comparison.rs        # Direct vs MQ comparisons
├── topology/                    # Existing topology system (enhance)
│   └── src/
│       ├── transport.rs         # Update to integrate with new system
│       └── resolution.rs        # Enhance with transport selection
```

## Implementation Phases

### Phase 1: Core Network Transport (Week 1) ✅ IN PROGRESS
**Goal**: Direct peer-to-peer transport with topology integration

#### Core Network Infrastructure
- **TCP Transport**: Connection pooling, low-latency configuration
- **UDP Transport**: Unreliable ultra-low latency for trading signals  
- **Wire Protocol**: NetworkEnvelope with compression/encryption
- **Connection Management**: Pool, heartbeats, failure detection
- **Security Layer**: TLS and ChaCha20Poly1305 encryption
- **Compression Engine**: LZ4, Zstd, Snappy support

#### Topology Integration
- **Enhanced Transport Resolution**: Select optimal transport per actor pair
- **Configuration Integration**: YAML transport config in topology
- **Factory Pattern**: Create transports from topology configuration

### Phase 2: Message Queue Integration (Week 2)  
**Goal**: Add optional MQ backends for reliability-critical channels

#### MQ Implementations
- **RabbitMQ**: High-reliability message broker
- **Kafka**: High-throughput streaming platform
- **Redis Streams**: In-memory message queues
- **Abstract Traits**: Common interface for all MQ backends

#### Hybrid System
- **Transport Router**: Route messages via direct or MQ based on config
- **Bridge Components**: Convert between direct and MQ when needed
- **Configuration**: Per-channel transport mode selection

### Phase 3: Advanced Features (Week 3)
**Goal**: Production-ready features and optimization

#### Performance & Reliability
- **Circuit Breakers**: Automatic failure detection and recovery
- **Load Balancing**: Distribute connections across multiple nodes
- **Connection Multiplexing**: Share connections between actors
- **Adaptive Compression**: Dynamic compression based on network conditions

#### Monitoring & Observability  
- **Metrics Collection**: Latency, throughput, error rates
- **Health Monitoring**: Transport health checks and alerting
- **Distributed Tracing**: Request tracing across transport boundaries
- **Performance Dashboards**: Real-time transport performance

### Phase 4: Protocol Integration (Week 4)
**Goal**: Deep integration with protocol_v2 and services_v2

#### Protocol_v2 Integration
- **TLV Message Routing**: Direct integration with TLV types
- **RelayDomain Awareness**: Route based on message domains
- **Message Bus Compatibility**: Work with existing message bus

#### Services_v2 Integration
- **Adapter Integration**: Seamless transport for data adapters
- **Strategy Integration**: High-performance transport for strategies
- **Service Discovery**: Automatic service endpoint resolution

## Key Design Decisions

### 1. Hybrid Transport Architecture
```yaml
# Configuration example - per-channel transport selection
transport_config:
  arbitrage_signals:
    mode: direct        # Ultra-low latency for trading
    protocol: udp
    compression: none
    encryption: none
  
  market_data:
    mode: direct        # High throughput for market feeds  
    protocol: tcp
    compression: lz4
    encryption: none
    
  audit_trail:
    mode: message_queue # Reliability for compliance
    backend: rabbitmq
    durability: persistent
    routing_key: "audit.trades"
```

### 2. Transport Selection Logic
```rust
// Automatic transport selection based on:
// - Actor placement (same-node vs inter-node)
// - Channel criticality (latency vs reliability)
// - Network topology (same DC vs cross-region)
// - Security requirements (encrypted vs plain)

impl TransportResolver {
    fn resolve_transport(&self, from: &Actor, to: &Actor, channel: &Channel) -> Transport {
        if self.same_node(from, to) {
            Transport::SharedMemory { /* optimized for NUMA */ }
        } else if channel.criticality == Criticality::UltraLowLatency {
            Transport::Direct(DirectConfig::udp_optimized())
        } else if channel.reliability == Reliability::GuaranteedDelivery {
            Transport::MessageQueue(MqConfig::rabbitmq_persistent())
        } else {
            Transport::Direct(DirectConfig::tcp_balanced())
        }
    }
}
```

### 3. Performance Optimization Strategy
- **Critical Path**: Direct UDP with no compression/encryption (<1ms)
- **High Throughput**: Direct TCP with LZ4 compression (<5ms)  
- **Reliable Delivery**: RabbitMQ with persistence (>20ms, guaranteed)
- **Audit/Compliance**: Kafka with retention (>50ms, durable)

### 4. Integration Points
- **Topology System**: Transport selection and configuration
- **Protocol_v2**: TLV message routing and RelayDomain awareness
- **Services_v2**: Adapter and strategy integration
- **Monitoring**: Metrics collection and health monitoring

## Success Metrics

### Phase 1 Success
- [ ] TCP transport achieves <5ms latency between nodes
- [ ] UDP transport achieves <1ms latency for signals
- [ ] Topology integration automatically selects optimal transport
- [ ] All transport tests pass with 100% reliability

### Phase 2 Success  
- [ ] RabbitMQ integration provides guaranteed message delivery
- [ ] Hybrid routing correctly selects direct vs MQ per configuration
- [ ] Bridge components maintain message ordering and consistency
- [ ] MQ integration tests demonstrate reliability under failure

### Phase 3 Success
- [ ] Circuit breakers prevent cascade failures
- [ ] Monitoring provides real-time transport performance visibility
- [ ] Load balancing distributes traffic evenly across connections
- [ ] Performance benchmarks meet AlphaPulse latency requirements

### Phase 4 Success
- [ ] Protocol_v2 messages route correctly through transport layer
- [ ] Services_v2 adapters and strategies work seamlessly
- [ ] End-to-end integration tests pass with realistic workloads
- [ ] Production deployment successfully handles live traffic

## Risk Mitigation

### Network Reliability
- **Risk**: Network partitions cause message loss
- **Mitigation**: Circuit breakers, automatic failover, MQ for critical data

### Performance Regression  
- **Risk**: Transport layer adds unacceptable latency
- **Mitigation**: Extensive benchmarking, zero-copy optimizations, direct UDP option

### Configuration Complexity
- **Risk**: Transport configuration becomes too complex
- **Mitigation**: Sensible defaults, validation, configuration templates

### Integration Breaking Changes
- **Risk**: Transport changes break existing protocol_v2/services_v2
- **Mitigation**: Backward compatibility, gradual migration, comprehensive testing

## Dependencies and Integration

### New Dependencies
```toml
# Core networking
tokio = { version = "1.40", features = ["net", "rt-multi-thread"] }
quinn = "0.10"              # QUIC implementation
mio = "0.8"                 # Low-level networking

# Compression
lz4 = "1.24"               # Fast compression
zstd = "0.13"              # Better compression ratio  
snap = "1.1"               # Google Snappy

# Encryption  
rustls = "0.21"            # TLS implementation
chacha20poly1305 = "0.10"  # Fast authenticated encryption

# Message Queues
lapin = "2.3"              # RabbitMQ client
rdkafka = "0.33"           # Kafka client  
redis = "0.23"             # Redis client

# Monitoring
prometheus = "0.13"         # Metrics collection
tracing = "0.1"            # Distributed tracing
```

### Integration Points
- **topology/src/transport.rs**: Enhance with new transport types
- **topology/src/resolution.rs**: Add transport selection logic
- **protocol_v2/src/transport/**: Extend for network transport support
- **services_v2/**: Update adapters/strategies to use new transport

## Migration Strategy

### Immediate (Phase 1)
- Implement alongside existing transport without breaking changes
- Add feature flags for gradual rollout
- Extensive testing with current workloads

### Short-term (Phase 2-3)  
- Begin using direct transport for non-critical channels
- Add MQ integration for reliability-sensitive data
- Monitor performance impact and adjust

### Long-term (Phase 4)
- Full production deployment with hybrid transport
- Migrate all channels to optimal transport selection
- Decommission old transport implementations

This plan provides a comprehensive network transport system that maintains AlphaPulse's low-latency requirements while adding the reliability benefits of message queues where appropriate.
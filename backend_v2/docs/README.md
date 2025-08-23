# AlphaPulse Backend V2 - Documentation

This directory contains comprehensive documentation for the AlphaPulse backend V2 architecture and implementation.

## Core Protocol Documentation

### **[protocol.md](protocol.md)** 📋
Complete technical specification for the AlphaPulse Protocol V2:
- Universal TLV message format
- Bijective InstrumentId system  
- Domain-specific relay architecture
- Performance characteristics (>1M msg/s achieved)
- Implementation examples and usage patterns

### **[message-types.md](message-types.md)** 📖
Comprehensive TLV message type reference:
- Complete type registry (1-255) with status indicators
- Domain-organized listings (Market Data, Signals, Execution)
- Size specifications and routing behavior
- Usage examples for each message domain

### **[PERFORMANCE_ANALYSIS.md](PERFORMANCE_ANALYSIS.md)** ⚡
Performance benchmarks and analysis:
- Measured throughput results
- Memory usage characteristics
- Latency analysis and optimization notes

### **[POOL_MESSAGES_DESIGN.md](POOL_MESSAGES_DESIGN.md)** 🏊
DEX pool message design rationale:
- Pool liquidity tracking
- Swap event handling
- V2 vs V3 protocol differences

## System Architecture Documentation

### **[services.md](services.md)** 🏗️
Service architecture and component design

### **[services_review.md](services_review.md)** 🔍  
Service architecture review and recommendations

### **[message_bus.md](message_bus.md)** 📨
Message bus architecture and transport layer design

### **[bus.md](bus.md)** 🚌
Additional bus architecture documentation

## Beta Architecture (Legacy/Planning)

The `beta/` directory contains earlier architectural planning documents:
- `adapters.md` - Adapter pattern design
- `architecture_clarification.md` - System clarifications
- `connectors.md` - External system connections
- `execution.md` - Execution engine design
- `portfolio.md` - Portfolio management
- `risk.md` - Risk management system

## Documentation Organization

```
docs/
├── README.md                     # This file - documentation index
├── protocol.md                   # ⭐ Core protocol specification
├── message-types.md              # ⭐ TLV message reference  
├── PERFORMANCE_ANALYSIS.md       # Performance benchmarks
├── POOL_MESSAGES_DESIGN.md       # DEX pool messages
├── services.md                   # Service architecture
├── services_review.md            # Architecture review
├── message_bus.md                # Message bus design
├── bus.md                        # Bus architecture
└── beta/                         # Legacy planning docs
    ├── adapters.md
    ├── architecture_clarification.md
    ├── connectors.md
    ├── execution.md
    ├── portfolio.md
    └── risk.md
```

## Quick Start

1. **Understanding the Protocol**: Start with [protocol.md](protocol.md)
2. **Message Reference**: Use [message-types.md](message-types.md) for TLV types
3. **Performance Data**: Check [PERFORMANCE_ANALYSIS.md](PERFORMANCE_ANALYSIS.md) for benchmarks
4. **Implementation**: See `../protocol_v2/README.md` for code examples

## Status

**✅ Production Ready**: The Protocol V2 implementation is complete and tested:
- All relay servers implemented
- >1M msg/s performance achieved  
- Comprehensive test coverage
- Zero-copy parsing with robust error handling

The documentation accurately reflects the working implementation.
# Sprint 014: MessageSink Architecture & Lazy Connections

Implement MessageSink trait architecture with lazy connection patterns for flexible, composable communication pathways that wake up as data flows.

## 🎯 Sprint Goal

Create a decoupled message routing system where services don't know or care how their messages reach destinations. Connections establish themselves lazily when data flows ("wake on data" pattern), eliminating startup order dependencies.

## 🔑 Key Innovation: Two-Stage Implementation

### Stage 1: Config-Based (This Sprint)
```
Adapter → SinkFactory → ServiceRegistry → network_primitives
                        (reads services.toml)
```
- Build and test TODAY without waiting for Mycelium
- Simple TOML configuration for service discovery
- Full functionality with config files

### Stage 2: Mycelium-Powered (Future)
```
Adapter → SinkFactory → Mycelium API
         (same API!)    (handles everything)
```
- **Zero changes to adapter code**
- SinkFactory internally switches to Mycelium
- Advanced connection provisioning and management

## Quick Start

1. **Review sprint plan**: 
   ```bash
   cat SPRINT_PLAN.md
   ```

2. **Check current status**:
   ```bash
   ../../scrum/task-manager.sh sprint-014
   ```

3. **Start with SINK-001**:
   ```bash
   # Read the first task
   cat SINK-001_define_messagesink_trait.md
   
   # Create worktree (NEW workflow!)
   git worktree add -b feat/messagesink-trait-core ../messagesink-001
   cd ../messagesink-001
   ```

4. **Test the implementation**:
   ```bash
   cargo test -p torq-message-sink
   cargo bench -p torq-message-sink
   ```

## Task Overview

### 🔴 Critical Path (Must Complete First)
- **SINK-001**: Define MessageSink trait - Foundation for everything
- **SINK-002**: Lazy connection wrapper - "Wake on data" pattern
- **SINK-003**: SinkFactory with config - Stage 1 implementation
- **SINK-004**: ServiceRegistry - Read services.toml

### 🟡 Core Implementation
- **SINK-005**: Relay-based sinks - Connect to domain relays
- **SINK-006**: Direct sinks - Point-to-point connections
- **SINK-007**: Composite patterns - Fanout, round-robin, failover
- **SINK-008**: Buffering/backpressure - Handle load gracefully

### 🟢 Integration
- **SINK-009**: Migrate first adapter - Prove the pattern works
- **SINK-010**: Update relay consumers - Use lazy connections
- **SINK-011**: Monitoring/metrics - Observe connection behavior
- **SINK-012**: Comprehensive tests - Edge cases and chaos

### 🔵 Documentation
- **SINK-013**: Architecture docs - How it all fits together
- **SINK-014**: Usage examples - Show developers how to use
- **SINK-015**: Performance benchmarks - Prove <1% overhead

## Important Rules

- **Use git worktree**, NOT git checkout
- **Always update task status** (TODO → IN_PROGRESS → COMPLETE)
- **Test everything** - This is foundational architecture
- **Keep Stage 2 in mind** - Don't break future migration path
- **Document clearly** - Others will build on this
- **Create TEST_RESULTS.md** when tests pass
- **Use PR for all merges**

## Directory Structure
```
.
├── README.md                              # This file
├── SPRINT_PLAN.md                        # Detailed sprint specification
├── SINK-001_define_messagesink_trait.md  # Foundation trait
├── SINK-002_lazy_connection_wrapper.md   # Lazy connections
├── SINK-003_sinkfactory_configuration.md # Stage 1 factory
├── TASK-001_rename_me.md                 # Template for additional tasks
├── check-status.sh                       # Quick status check
└── TEST_RESULTS.md                       # Created when tests complete
```

## Why This Sprint Matters

This sprint creates the abstraction layer that:
1. **Decouples services completely** - No knowledge of connections
2. **Enables gradual migration** - Config today, Mycelium tomorrow
3. **Improves developer experience** - No startup order issues
4. **Reduces resource usage** - Only connect when needed
5. **Future-proofs the architecture** - Ready for advanced routing

**Start with SINK-001 - it's the foundation for everything!**

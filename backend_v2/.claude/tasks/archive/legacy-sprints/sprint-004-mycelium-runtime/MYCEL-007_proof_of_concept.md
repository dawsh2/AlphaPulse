---
task_id: MYCEL-007
status: COMPLETED
priority: CRITICAL
estimated_hours: 12
actual_hours: 3
assigned_branch: feat/mycelium-poc-migration
assignee: Claude
created: 2025-08-26
completed: 2025-08-28
depends_on:
  - MYCEL-001  # Need transport layer
  - MYCEL-002  # Need message types
  - MYCEL-003  # Need actor system
blocks: []
scope:
  - "services_v2/adapters/src/"  # MarketDataProcessor modifications
  - "services_v2/strategies/src/"  # SignalGenerator modifications
  - "tests/integration/"  # Performance validation tests
---

# MYCEL-007: Proof-of-Concept Migration

## Task Overview
**Sprint**: 004-mycelium-runtime
**Priority**: CRITICAL
**Estimate**: 12 hours
**Status**: TODO
**Dependencies**: MYCEL-001 through MYCEL-006
**Goal**: Migrate market_data → signal_generator pair to prove 50% latency reduction

## Problem
Need to validate that Mycelium delivers promised performance improvements by migrating a real high-frequency communication path from relay-based to actor-based architecture.

## Target Services
**MarketDataProcessor** → **SignalGenerator**
- Currently: Serialize every message to TLV, send through relay
- After: Pass Arc<MarketMessage> directly when bundled
- Expected: 350x speedup (35μs → 100ns)

## Migration Plan

### Phase 1: Wrap Existing Services
```rust
// Wrapper for existing MarketDataProcessor
pub struct MarketDataActor {
    inner: MarketDataProcessor,  // Existing service
    signal_actor: ActorRef<SignalMessage>,
}

#[async_trait]
impl ActorBehavior for MarketDataActor {
    type Message = MarketMessage;
    
    async fn handle(&mut self, msg: MarketMessage) -> Result<()> {
        match msg {
            MarketMessage::Swap(event) => {
                // Process swap event
                let pools = self.inner.process_swap(event.as_ref())?;
                
                // OLD WAY (commented out):
                // let tlv = create_pool_update_tlv(&pools);
                // self.relay.send(tlv).await?;
                
                // NEW WAY: Direct actor message
                if let Some(signal) = self.detect_opportunity(&pools) {
                    self.signal_actor.send(
                        SignalMessage::Arbitrage(Arc::new(signal))
                    ).await?;
                }
            }
            MarketMessage::Quote(quote) => {
                self.inner.process_quote(quote.as_ref())?;
            }
            // ... other message types
        }
        Ok(())
    }
    
    async fn on_start(&mut self) -> Result<()> {
        info!("MarketDataActor starting");
        self.inner.initialize().await
    }
}
```

### Phase 2: Wrap Signal Generator
```rust
pub struct SignalGeneratorActor {
    inner: SignalGenerator,
    execution_actor: Option<ActorRef<ExecutionMessage>>,
}

#[async_trait]
impl ActorBehavior for SignalGeneratorActor {
    type Message = SignalMessage;
    
    async fn handle(&mut self, msg: SignalMessage) -> Result<()> {
        match msg {
            SignalMessage::Arbitrage(signal) => {
                // Validate and enhance signal
                if let Some(enhanced) = self.inner.enhance_signal(signal.as_ref())? {
                    // Send to execution if profitable
                    if enhanced.net_profit_usd > self.inner.config.min_profit {
                        if let Some(exec) = &self.execution_actor {
                            exec.send(
                                ExecutionMessage::SubmitOrder(
                                    Arc::new(enhanced.to_order_request())
                                )
                            ).await?;
                        }
                    }
                }
            }
            // ... other signal types
        }
        Ok(())
    }
}
```

### Phase 3: Bundle Configuration
```toml
# config/bundles.toml

[[bundles]]
name = "trading_core"
mode = "shared_memory"

[bundles.actors.market_data]
type = "MarketDataActor"
config = { pools_to_monitor = 1000, update_frequency_ms = 100 }

[bundles.actors.signal_generator]  
type = "SignalGeneratorActor"
config = { min_profit_usd = 10.0, max_gas_cost = 50.0 }

# These actors will use Arc<T> for communication
```

### Phase 4: Launch Bundled Actors
```rust
pub async fn launch_bundled_trading_core(system: &ActorSystem) -> Result<()> {
    // Create shared memory channels for bundle
    let (market_tx, market_rx) = mpsc::channel(10_000);
    let (signal_tx, signal_rx) = mpsc::channel(1_000);
    
    // Create transport for bundled actors
    let market_transport = ActorTransport {
        local: Some(signal_tx.clone()),  // Can send to signal generator
        remote: None,  // Not used in bundle
        metrics: TransportMetrics::new(),
    };
    
    let signal_transport = ActorTransport {
        local: Some(market_tx.clone()),  // Can receive from market data
        remote: None,
        metrics: TransportMetrics::new(),
    };
    
    // Spawn market data actor
    let market_actor = MarketDataActor {
        inner: MarketDataProcessor::new(config.market_data),
        signal_actor: /* reference to signal actor */,
    };
    let market_ref = system.spawn(market_actor).await?;
    
    // Spawn signal generator actor
    let signal_actor = SignalGeneratorActor {
        inner: SignalGenerator::new(config.signals),
        execution_actor: None,  // Not bundled yet
    };
    let signal_ref = system.spawn(signal_actor).await?;
    
    info!("Trading core bundle launched with zero-serialization between actors");
    Ok(())
}
```

## Performance Validation

### Benchmark Setup
```rust
#[bench]
fn bench_relay_vs_actor_latency(b: &mut Bencher) {
    let rt = Runtime::new().unwrap();
    
    // Setup: Create test message
    let swap_event = PoolSwapEvent {
        pool: [0x45; 20],
        amount0_in: 1000000,
        amount1_out: 2000000,
        timestamp_ns: 123456789,
    };
    
    b.iter(|| {
        rt.block_on(async {
            // BEFORE: Relay-based (serialization required)
            let before_start = Instant::now();
            let tlv = create_swap_tlv(&swap_event);
            relay.send(tlv).await.unwrap();
            let before_time = before_start.elapsed();
            
            // AFTER: Actor-based (Arc passing only)
            let after_start = Instant::now();
            actor_ref.send(MarketMessage::Swap(Arc::new(swap_event.clone()))).await.unwrap();
            let after_time = after_start.elapsed();
            
            println!("Relay: {:?}, Actor: {:?}, Speedup: {}x",
                before_time, after_time,
                before_time.as_nanos() / after_time.as_nanos());
        });
    });
}
```

### Metrics Collection
```rust
pub struct MigrationMetrics {
    pub messages_processed: u64,
    pub avg_latency_before_ns: u64,
    pub avg_latency_after_ns: u64,
    pub serialization_eliminated_bytes: u64,
    pub cpu_usage_before: f64,
    pub cpu_usage_after: f64,
}

impl MigrationMetrics {
    pub fn calculate_improvement(&self) -> MigrationReport {
        MigrationReport {
            latency_reduction: (self.avg_latency_before_ns - self.avg_latency_after_ns) as f64 
                / self.avg_latency_before_ns as f64 * 100.0,
            throughput_increase: self.messages_processed as f64 / self.avg_latency_after_ns as f64
                / (self.messages_processed as f64 / self.avg_latency_before_ns as f64),
            serialization_saved_mb: self.serialization_eliminated_bytes as f64 / 1_048_576.0,
            cpu_reduction: (self.cpu_usage_before - self.cpu_usage_after) / self.cpu_usage_before * 100.0,
        }
    }
}
```

### Load Test
```rust
pub async fn load_test_bundled_actors(system: &ActorSystem) -> Result<MigrationMetrics> {
    let market_ref = /* ... */;
    let duration = Duration::from_secs(60);
    let start = Instant::now();
    let mut message_count = 0;
    
    // Generate realistic load
    while start.elapsed() < duration {
        // Simulate market events at realistic rate
        for _ in 0..100 {
            let event = generate_random_swap_event();
            market_ref.send(MarketMessage::Swap(Arc::new(event))).await?;
            message_count += 1;
        }
        
        // ~10ms between batches
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    // Collect metrics
    let metrics = system.get_transport_metrics().await;
    Ok(MigrationMetrics {
        messages_processed: message_count,
        avg_latency_after_ns: metrics.avg_local_latency_ns,
        // ... other metrics
    })
}
```

## Migration Checklist

### Pre-Migration
- [ ] Backup existing configuration
- [ ] Document current performance baseline
- [ ] Set up monitoring for comparison
- [ ] Create rollback plan

### Migration Steps
- [ ] Wrap MarketDataProcessor as actor
- [ ] Wrap SignalGenerator as actor
- [ ] Configure bundle with shared memory
- [ ] Update configuration files
- [ ] Deploy bundled version
- [ ] Run side-by-side comparison

### Validation
- [ ] Functional correctness (same signals generated)
- [ ] Latency reduction achieved (target: 50%)
- [ ] No message loss
- [ ] Memory usage acceptable
- [ ] CPU usage reduced
- [ ] Metrics collected and documented

## Success Criteria
1. **Latency**: <100ns for bundled messages (from ~35μs)
2. **Throughput**: >50% increase in messages/second
3. **CPU**: >30% reduction in CPU usage
4. **Memory**: <10% increase (acceptable for channels)
5. **Correctness**: Identical trading signals generated

## Rollback Plan
```bash
# If issues arise, immediately rollback
./scripts/rollback_to_relay.sh

# Monitor for 5 minutes
./scripts/monitor_health.sh

# If stable, investigate issue offline
```

## Documentation
Document the migration process for other service pairs:
1. Identify high-frequency communication paths
2. Wrap services as actors
3. Configure bundles
4. Deploy and measure
5. Iterate on configuration

## Definition of Done
- Market data and signal generator running as bundled actors
- Zero serialization verified between them
- 50% latency reduction measured and documented
- Load test showing sustained performance
- Migration guide written for other services
- Metrics dashboard showing improvements
# Event System and Subscriptions

High-performance, subscription-based event system for reactive trading with pattern matching and selective broadcasting.

## Event Definitions

```rust
use tokio::sync::broadcast;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubscriberId(pub u64);

#[derive(Debug, Clone)]
pub enum RegistryEvent {
    InstrumentAdded {
        id: InstrumentId,
        instrument_type: InstrumentType,
        venue: Option<VenueId>,
        timestamp: SystemTime,
    },
    
    PriceUpdate {
        id: InstrumentId,
        old_price: f64,
        new_price: f64,
        timestamp: SystemTime,
    },
    
    ArbitrageDetected {
        opportunity_id: u64,
        instruments: Vec<InstrumentId>,
        venues: Vec<VenueId>,
        spread_bps: f64,
        confidence: f64,
        expires_at: SystemTime,
    },
    
    SyntheticEvaluated {
        synthetic_id: InstrumentId,
        value: f64,
        components: HashMap<InstrumentId, f64>,
        evaluation_time_ns: u64,
    },
    
    VenueStatusChange {
        venue_id: VenueId,
        old_status: VenueStatus,
        new_status: VenueStatus,
        affected_instruments: Vec<InstrumentId>,
    },
    
    PoolDiscovered {
        pool_id: PoolId,
        token0: InstrumentId,
        token1: InstrumentId,
        dex: String,
        blockchain: Blockchain,
        initial_liquidity: f64,
    },
    
    HashCollisionDetected {
        id: InstrumentId,
        existing_instrument: String,
        new_instrument: String,
        severity: CollisionSeverity,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum CollisionSeverity {
    Critical,   // Different instruments with same ID
    Warning,    // Same instrument registered twice
    Info,       // Handled collision with fallback
}
```

## Subscription Patterns

```rust
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum SubscriptionPattern {
    // Asset type subscriptions
    AllStocks,
    AllTokens { blockchain: Blockchain },
    AllFutures { underlying: Option<String> },
    AllOptions { underlying_id: Option<InstrumentId> },
    
    // Venue-based subscriptions
    VenueInstruments { venue_id: VenueId },
    AllDEXPools { blockchain: Blockchain },
    
    // ISIN-based subscriptions
    ISINPattern { isin_prefix: String },     // e.g., "US" for all US securities
    
    // Arbitrage subscriptions
    ArbitrageOpportunities { min_spread_bps: u32 },
    CrossAssetArbitrage { asset_types: Vec<InstrumentType> },
    
    // Synthetic subscriptions
    SyntheticUpdates { synthetic_id: InstrumentId },
    AllSynthetics,
    
    // System events
    NewListings,
    RegistryErrors,
    PerformanceAlerts { threshold_ms: u64 },
}
```

## Subscription Manager

```rust
pub struct RegistrySubscriptionManager {
    // Instrument-specific subscriptions
    instrument_subs: Arc<DashMap<InstrumentId, HashSet<SubscriberId>>>,
    
    // Pattern-based subscriptions
    pattern_subs: Arc<DashMap<SubscriptionPattern, HashSet<SubscriberId>>>,
    
    // Event channels
    event_bus: broadcast::Sender<RegistryEvent>,
    
    // Subscriber info
    subscribers: Arc<DashMap<SubscriberId, SubscriberInfo>>,
    
    // Metrics
    metrics: Arc<SubscriptionMetrics>,
}

#[derive(Debug, Clone)]
pub struct SubscriberInfo {
    pub id: SubscriberId,
    pub patterns: Vec<SubscriptionPattern>,
    pub created_at: SystemTime,
    pub last_event: Option<SystemTime>,
    pub events_received: u64,
    pub events_filtered: u64,
}

#[derive(Debug, Default)]
pub struct SubscriptionMetrics {
    pub total_subscribers: AtomicU64,
    pub total_events_published: AtomicU64,
    pub total_events_delivered: AtomicU64,
    pub avg_delivery_time_ns: AtomicU64,
    pub failed_deliveries: AtomicU64,
}
```

## Subscription Operations

```rust
impl RegistrySubscriptionManager {
    pub async fn subscribe(
        &self,
        subscriber_id: SubscriberId,
        pattern: SubscriptionPattern,
    ) -> broadcast::Receiver<RegistryEvent> {
        // Register subscription
        self.pattern_subs
            .entry(pattern.clone())
            .or_insert_with(HashSet::new)
            .insert(subscriber_id);
        
        // Update subscriber info
        self.subscribers
            .entry(subscriber_id)
            .and_modify(|info| info.patterns.push(pattern.clone()))
            .or_insert_with(|| SubscriberInfo {
                id: subscriber_id,
                patterns: vec![pattern],
                created_at: SystemTime::now(),
                last_event: None,
                events_received: 0,
                events_filtered: 0,
            });
        
        // Update metrics
        self.metrics.total_subscribers.fetch_add(1, Ordering::Relaxed);
        
        // Return event receiver
        self.event_bus.subscribe()
    }
    
    pub async fn unsubscribe(
        &self,
        subscriber_id: SubscriberId,
        pattern: Option<SubscriptionPattern>,
    ) -> Result<(), SubscriptionError> {
        match pattern {
            Some(p) => {
                // Remove specific pattern subscription
                if let Some(mut subs) = self.pattern_subs.get_mut(&p) {
                    subs.remove(&subscriber_id);
                }
                
                // Update subscriber info
                if let Some(mut info) = self.subscribers.get_mut(&subscriber_id) {
                    info.patterns.retain(|pat| pat != &p);
                }
            }
            None => {
                // Remove all subscriptions for this subscriber
                for mut entry in self.pattern_subs.iter_mut() {
                    entry.value_mut().remove(&subscriber_id);
                }
                
                self.subscribers.remove(&subscriber_id);
                self.metrics.total_subscribers.fetch_sub(1, Ordering::Relaxed);
            }
        }
        
        Ok(())
    }
}
```

## Event Publishing

```rust
impl RegistrySubscriptionManager {
    pub async fn publish(&self, event: RegistryEvent) -> Result<usize, PublishError> {
        let start = std::time::Instant::now();
        
        // Match event to subscriptions
        let interested_subscribers = self.match_subscribers(&event);
        
        // Send to interested parties only
        let sent = self.event_bus.send(event.clone())
            .map_err(|_| PublishError::NoSubscribers)?;
        
        // Update metrics
        let delivery_time = start.elapsed().as_nanos() as u64;
        self.metrics.total_events_published.fetch_add(1, Ordering::Relaxed);
        self.metrics.total_events_delivered.fetch_add(sent as u64, Ordering::Relaxed);
        self.metrics.avg_delivery_time_ns.store(delivery_time, Ordering::Relaxed);
        
        // Update subscriber stats
        for subscriber_id in interested_subscribers {
            if let Some(mut info) = self.subscribers.get_mut(&subscriber_id) {
                info.last_event = Some(SystemTime::now());
                info.events_received += 1;
            }
        }
        
        Ok(sent)
    }
    
    fn match_subscribers(&self, event: &RegistryEvent) -> HashSet<SubscriberId> {
        let mut subscribers = HashSet::new();
        
        match event {
            RegistryEvent::InstrumentAdded { instrument_type, .. } => {
                // Match type-based patterns
                match instrument_type {
                    InstrumentType::Stock { .. } => {
                        if let Some(subs) = self.pattern_subs.get(&SubscriptionPattern::AllStocks) {
                            subscribers.extend(subs.iter());
                        }
                    }
                    InstrumentType::Token { blockchain, .. } => {
                        let pattern = SubscriptionPattern::AllTokens { 
                            blockchain: blockchain.clone() 
                        };
                        if let Some(subs) = self.pattern_subs.get(&pattern) {
                            subscribers.extend(subs.iter());
                        }
                    }
                    InstrumentType::Future { underlying, .. } => {
                        // Match all futures
                        if let Some(subs) = self.pattern_subs.get(&SubscriptionPattern::AllFutures { underlying: None }) {
                            subscribers.extend(subs.iter());
                        }
                        // Match specific underlying
                        let pattern = SubscriptionPattern::AllFutures { 
                            underlying: Some(underlying.clone()) 
                        };
                        if let Some(subs) = self.pattern_subs.get(&pattern) {
                            subscribers.extend(subs.iter());
                        }
                    }
                    _ => {}
                }
                
                // Always notify new listing subscribers
                if let Some(subs) = self.pattern_subs.get(&SubscriptionPattern::NewListings) {
                    subscribers.extend(subs.iter());
                }
            }
            
            RegistryEvent::ArbitrageDetected { spread_bps, .. } => {
                // Find all arbitrage pattern subscribers with matching criteria
                for entry in self.pattern_subs.iter() {
                    if let SubscriptionPattern::ArbitrageOpportunities { min_spread_bps } = entry.key() {
                        if *spread_bps >= *min_spread_bps as f64 {
                            subscribers.extend(entry.value().iter());
                        }
                    }
                }
            }
            
            RegistryEvent::HashCollisionDetected { severity, .. } => {
                // Always notify error subscribers for critical collisions
                if *severity == CollisionSeverity::Critical {
                    if let Some(subs) = self.pattern_subs.get(&SubscriptionPattern::RegistryErrors) {
                        subscribers.extend(subs.iter());
                    }
                }
            }
            
            RegistryEvent::SyntheticEvaluated { synthetic_id, .. } => {
                // Specific synthetic subscribers
                let pattern = SubscriptionPattern::SyntheticUpdates { 
                    synthetic_id: *synthetic_id 
                };
                if let Some(subs) = self.pattern_subs.get(&pattern) {
                    subscribers.extend(subs.iter());
                }
                
                // All synthetics subscribers
                if let Some(subs) = self.pattern_subs.get(&SubscriptionPattern::AllSynthetics) {
                    subscribers.extend(subs.iter());
                }
            }
            
            _ => {}
        }
        
        subscribers
    }
}
```

## Event Filters

```rust
pub struct EventFilter {
    pub min_price_change_pct: Option<f64>,
    pub instruments: Option<HashSet<InstrumentId>>,
    pub venues: Option<HashSet<VenueId>>,
    pub event_types: Option<HashSet<EventType>>,
    pub time_window: Option<Duration>,
}

impl EventFilter {
    pub fn matches(&self, event: &RegistryEvent) -> bool {
        // Check event type filter
        if let Some(ref types) = self.event_types {
            if !types.contains(&event.event_type()) {
                return false;
            }
        }
        
        // Check price change filter
        if let Some(min_change) = self.min_price_change_pct {
            if let RegistryEvent::PriceUpdate { old_price, new_price, .. } = event {
                let change_pct = ((new_price - old_price) / old_price * 100.0).abs();
                if change_pct < min_change {
                    return false;
                }
            }
        }
        
        // Check instrument filter
        if let Some(ref instruments) = self.instruments {
            if let Some(instrument_id) = event.get_instrument_id() {
                if !instruments.contains(&instrument_id) {
                    return false;
                }
            }
        }
        
        true
    }
}
```

## Event Handlers

```rust
pub trait EventHandler: Send + Sync {
    fn handle_event(&mut self, event: &RegistryEvent) -> Result<(), EventHandlingError>;
    fn event_types(&self) -> Vec<EventType>;
    fn priority(&self) -> EventPriority;
}

// Example: Arbitrage opportunity handler
pub struct ArbitrageOpportunityHandler {
    arbitrage_engine: Arc<Mutex<ArbitrageEngine>>,
    min_profit_threshold: f64,
}

impl EventHandler for ArbitrageOpportunityHandler {
    fn handle_event(&mut self, event: &RegistryEvent) -> Result<(), EventHandlingError> {
        if let RegistryEvent::ArbitrageDetected { 
            opportunity_id, 
            spread_bps, 
            confidence, 
            expires_at, 
            .. 
        } = event {
            // Check if opportunity meets threshold
            let estimated_profit = spread_bps * confidence / 10000.0;
            if estimated_profit >= self.min_profit_threshold {
                // Execute arbitrage
                let mut engine = self.arbitrage_engine.lock().unwrap();
                engine.execute_opportunity(*opportunity_id, *expires_at)?;
            }
        }
        
        Ok(())
    }
    
    fn event_types(&self) -> Vec<EventType> {
        vec![EventType::ArbitrageDetected]
    }
    
    fn priority(&self) -> EventPriority {
        EventPriority::Critical // Execute immediately
    }
}
```

## Event Priority Queue

```rust
use std::collections::BinaryHeap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventPriority {
    Critical = 0,   // Immediate execution
    High = 1,       // Within 1ms
    Normal = 2,     // Within 10ms
    Low = 3,        // Within 100ms
    Background = 4, // When convenient
}

#[derive(Debug)]
pub struct PrioritizedEvent {
    pub event: RegistryEvent,
    pub priority: EventPriority,
    pub timestamp: SystemTime,
}

impl Ord for PrioritizedEvent {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Higher priority (lower value) comes first
        other.priority.cmp(&self.priority)
            .then_with(|| other.timestamp.cmp(&self.timestamp))
    }
}

impl PartialOrd for PrioritizedEvent {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub struct EventProcessor {
    priority_queue: Arc<Mutex<BinaryHeap<PrioritizedEvent>>>,
    handlers: Arc<DashMap<EventType, Vec<Box<dyn EventHandler>>>>,
    processing_tasks: Arc<DashMap<EventType, JoinHandle<()>>>,
}

impl EventProcessor {
    pub async fn process_events(&self) {
        loop {
            let event = {
                let mut queue = self.priority_queue.lock().unwrap();
                queue.pop()
            };
            
            if let Some(prioritized_event) = event {
                // Process based on priority
                match prioritized_event.priority {
                    EventPriority::Critical => {
                        // Process immediately in current thread
                        self.process_event_immediate(prioritized_event.event).await;
                    }
                    _ => {
                        // Spawn task for lower priority
                        let handlers = Arc::clone(&self.handlers);
                        tokio::spawn(async move {
                            Self::process_event_async(prioritized_event.event, handlers).await;
                        });
                    }
                }
            } else {
                // No events, sleep briefly
                tokio::time::sleep(Duration::from_micros(100)).await;
            }
        }
    }
}
```

## Event Replay and Recovery

```rust
pub struct EventLog {
    events: Arc<DashMap<u64, StoredEvent>>,
    sequence: AtomicU64,
    persistence: Box<dyn EventPersistence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredEvent {
    pub sequence: u64,
    pub event: RegistryEvent,
    pub timestamp: SystemTime,
    pub subscribers_notified: Vec<SubscriberId>,
}

impl EventLog {
    pub async fn replay_from(&self, start_sequence: u64) -> Result<Vec<RegistryEvent>, ReplayError> {
        let mut events = Vec::new();
        let current_sequence = self.sequence.load(Ordering::Relaxed);
        
        for seq in start_sequence..=current_sequence {
            if let Some(stored) = self.events.get(&seq) {
                events.push(stored.event.clone());
            }
        }
        
        Ok(events)
    }
    
    pub async fn replay_time_range(
        &self, 
        start: SystemTime, 
        end: SystemTime
    ) -> Result<Vec<RegistryEvent>, ReplayError> {
        let events: Vec<_> = self.events.iter()
            .filter(|entry| {
                entry.timestamp >= start && entry.timestamp <= end
            })
            .map(|entry| entry.event.clone())
            .collect();
        
        Ok(events)
    }
}
```
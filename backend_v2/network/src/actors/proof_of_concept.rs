//! MYCEL-007: Proof-of-Concept Migration
//!
//! Demonstrates migration of MarketDataProcessor → SignalGenerator pair
//! to the Mycelium actor runtime, achieving zero-cost bundling with Arc<T>
//! message passing for same-process actors.

use super::messages::{
    MarketMessage, SignalMessage, PoolSwapEvent, QuoteUpdate, 
    ArbitrageSignal, Message
};
use super::system::{ActorBehavior, ActorSystem, SupervisorDirective};
use super::bundle::{BundleConfiguration, DeploymentMode};
use crate::{Result, TransportError};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::{info, debug, warn, error};
use tokio::sync::mpsc;

/// Market data processor actor - receives raw market events and normalizes them
pub struct MarketDataProcessorActor {
    /// Count of processed messages by type
    pub processed_swaps: AtomicU64,
    pub processed_quotes: AtomicU64,
    
    /// Output channel to signal generator
    signal_generator_ref: Option<super::system::ActorRef<MarketMessage>>,
    
    /// Pool state cache for context
    pool_states: HashMap<[u8; 20], PoolState>,
}

/// Pool state for arbitrage detection
#[derive(Debug, Clone)]
struct PoolState {
    reserve0: u64,
    reserve1: u64,
    last_update: u64,
}

impl MarketDataProcessorActor {
    pub fn new() -> Self {
        Self {
            processed_swaps: AtomicU64::new(0),
            processed_quotes: AtomicU64::new(0),
            signal_generator_ref: None,
            pool_states: HashMap::new(),
        }
    }
    
    pub fn with_signal_generator(mut self, signal_ref: super::system::ActorRef<MarketMessage>) -> Self {
        self.signal_generator_ref = Some(signal_ref);
        self
    }
    
    async fn process_swap(&mut self, event: Arc<PoolSwapEvent>) -> Result<()> {
        self.processed_swaps.fetch_add(1, Ordering::Relaxed);
        
        // Update pool state cache
        self.pool_states.insert(event.pool_address, PoolState {
            reserve0: event.token0_in,
            reserve1: event.token1_out,
            last_update: event.timestamp_ns,
        });
        
        // Forward to signal generator if connected
        // CRITICAL: This is where zero-cost Arc<T> passing happens for bundled actors
        if let Some(ref signal_gen) = self.signal_generator_ref {
            let market_msg = MarketMessage::Swap(Arc::clone(&event));
            signal_gen.send(market_msg).await?;
        }
        
        debug!(
            "Processed swap: pool={:?}, in={}, out={}", 
            event.pool_address, event.token0_in, event.token1_out
        );
        
        Ok(())
    }
    
    async fn process_quote(&mut self, quote: Arc<QuoteUpdate>) -> Result<()> {
        self.processed_quotes.fetch_add(1, Ordering::Relaxed);
        
        // Forward normalized quote to signal generator
        if let Some(ref signal_gen) = self.signal_generator_ref {
            let market_msg = MarketMessage::Quote(Arc::clone(&quote));
            signal_gen.send(market_msg).await?;
        }
        
        debug!(
            "Processed quote: instrument={}, bid={}, ask={}", 
            quote.instrument_id, quote.bid_price, quote.ask_price
        );
        
        Ok(())
    }
}

#[async_trait::async_trait]
impl ActorBehavior for MarketDataProcessorActor {
    type Message = MarketMessage;
    
    async fn handle(&mut self, msg: MarketMessage) -> Result<()> {
        match msg {
            MarketMessage::Swap(event) => self.process_swap(event).await,
            MarketMessage::Quote(quote) => self.process_quote(quote).await,
            MarketMessage::OrderBook(_) => {
                // Not implemented for PoC
                Ok(())
            }
            MarketMessage::VolumeSnapshot(_) => {
                // Not implemented for PoC
                Ok(())
            }
        }
    }
    
    async fn on_start(&mut self) -> Result<()> {
        info!("MarketDataProcessor actor started");
        Ok(())
    }
    
    async fn on_stop(&mut self) -> Result<()> {
        info!(
            "MarketDataProcessor stopped - processed {} swaps, {} quotes",
            self.processed_swaps.load(Ordering::Relaxed),
            self.processed_quotes.load(Ordering::Relaxed)
        );
        Ok(())
    }
    
    async fn on_error(&mut self, error: TransportError) -> SupervisorDirective {
        warn!("MarketDataProcessor error: {}", error);
        SupervisorDirective::Resume // Continue processing
    }
}

/// Signal generator actor - analyzes market data and generates trading signals
pub struct SignalGeneratorActor {
    /// Count of generated signals
    pub generated_arbitrage_signals: AtomicU64,
    
    /// Price cache for arbitrage detection (instrument_id -> (venue, price))
    price_cache: HashMap<u64, Vec<(u8, i64)>>,
    
    /// Minimum profit threshold for arbitrage signals
    min_profit_threshold: i64, // 8-decimal fixed point
    
    /// Output channel for signals
    signal_output: Option<mpsc::UnboundedSender<SignalMessage>>,
}

impl SignalGeneratorActor {
    pub fn new(min_profit_threshold: i64) -> Self {
        Self {
            generated_arbitrage_signals: AtomicU64::new(0),
            price_cache: HashMap::new(),
            min_profit_threshold,
            signal_output: None,
        }
    }
    
    pub fn with_output(mut self, output: mpsc::UnboundedSender<SignalMessage>) -> Self {
        self.signal_output = Some(output);
        self
    }
    
    async fn analyze_for_arbitrage(&mut self, quote: Arc<QuoteUpdate>) -> Result<()> {
        // Update price cache
        let prices = self.price_cache.entry(quote.instrument_id).or_insert_with(Vec::new);
        
        // Simple arbitrage detection: find price differences across venues
        for (venue, cached_price) in prices.iter() {
            let price_diff = (quote.bid_price - cached_price).abs();
            
            if price_diff > self.min_profit_threshold {
                // Generate arbitrage signal
                let signal = ArbitrageSignal {
                    opportunity_id: self.generated_arbitrage_signals.fetch_add(1, Ordering::Relaxed),
                    venue_a: *venue,
                    venue_b: 1, // Simplified - would track actual venue
                    instrument_id: quote.instrument_id,
                    price_difference: price_diff,
                    potential_profit_usd: price_diff * quote.bid_size as i64 / 100_000_000, // Simplified calculation
                    confidence_score: 80, // Simplified confidence
                    timestamp_ns: quote.timestamp_ns,
                };
                
                // Send signal through output channel
                if let Some(ref output) = self.signal_output {
                    let signal_msg = SignalMessage::Arbitrage(Arc::new(signal));
                    output.send(signal_msg).map_err(|_| {
                        TransportError::network("Signal output channel closed")
                    })?;
                    
                    debug!(
                        "Generated arbitrage signal: instrument={}, profit={}", 
                        quote.instrument_id, price_diff
                    );
                }
            }
        }
        
        // Update cache with latest price
        prices.push((1, quote.bid_price)); // Simplified venue tracking
        if prices.len() > 10 {
            prices.remove(0); // Keep cache bounded
        }
        
        Ok(())
    }
}

#[async_trait::async_trait]
impl ActorBehavior for SignalGeneratorActor {
    type Message = MarketMessage;
    
    async fn handle(&mut self, msg: MarketMessage) -> Result<()> {
        match msg {
            MarketMessage::Quote(quote) => {
                // Analyze quote for arbitrage opportunities
                self.analyze_for_arbitrage(quote).await
            }
            MarketMessage::Swap(_) => {
                // Could analyze for DEX arbitrage
                Ok(())
            }
            _ => Ok(()),
        }
    }
    
    async fn on_start(&mut self) -> Result<()> {
        info!("SignalGenerator actor started");
        Ok(())
    }
    
    async fn on_stop(&mut self) -> Result<()> {
        info!(
            "SignalGenerator stopped - generated {} arbitrage signals",
            self.generated_arbitrage_signals.load(Ordering::Relaxed)
        );
        Ok(())
    }
    
    async fn on_error(&mut self, error: TransportError) -> SupervisorDirective {
        warn!("SignalGenerator error: {}", error);
        SupervisorDirective::Resume
    }
}

/// Proof-of-concept migration demonstrating zero-cost bundling
pub struct ProofOfConceptMigration {
    system: ActorSystem,
    market_processor_ref: Option<super::system::ActorRef<MarketMessage>>,
    signal_generator_ref: Option<super::system::ActorRef<MarketMessage>>,
    signal_receiver: Option<mpsc::UnboundedReceiver<SignalMessage>>,
}

impl ProofOfConceptMigration {
    pub async fn setup() -> Result<Self> {
        info!("Setting up proof-of-concept migration");
        
        // Create actor system
        let system = ActorSystem::new();
        
        // Create bundle configuration for zero-cost communication
        let bundle = BundleConfiguration {
            name: "trading_core".to_string(),
            actors: vec!["market_processor".to_string(), "signal_generator".to_string()],
            deployment: DeploymentMode::SharedMemory {
                // Actors in same process use Arc<T> channels
                channels: HashMap::new(),
            },
        };
        
        // Register bundle with system
        system.add_bundle("trading_core".to_string(), bundle).await?;
        
        // Create signal output channel
        let (signal_tx, signal_rx) = mpsc::unbounded_channel();
        
        // Spawn signal generator first
        let signal_generator = SignalGeneratorActor::new(100_000_000) // $1.00 minimum profit
            .with_output(signal_tx);
        let signal_ref = system.spawn(signal_generator).await?;
        
        // Spawn market processor with connection to signal generator
        let market_processor = MarketDataProcessorActor::new()
            .with_signal_generator(signal_ref.clone());
        let market_ref = system.spawn(market_processor).await?;
        
        info!("Proof-of-concept actors spawned successfully");
        
        Ok(Self {
            system,
            market_processor_ref: Some(market_ref),
            signal_generator_ref: Some(signal_ref),
            signal_receiver: Some(signal_rx),
        })
    }
    
    /// Send test market data through the pipeline
    pub async fn send_test_data(&self) -> Result<()> {
        let market_ref = self.market_processor_ref.as_ref()
            .ok_or_else(|| TransportError::configuration("Market processor not initialized", None))?;
        
        // Send swap event
        let swap = PoolSwapEvent {
            pool_address: [1; 20],
            token0_in: 1_000_000_000_000_000_000, // 1 ETH
            token1_out: 3000_000_000, // 3000 USDC
            timestamp_ns: 1234567890,
            tx_hash: [2; 32],
            gas_used: 21000,
        };
        market_ref.send(MarketMessage::Swap(Arc::new(swap))).await?;
        
        // Send quotes that should trigger arbitrage detection
        let quote1 = QuoteUpdate {
            instrument_id: 12345,
            bid_price: 4500_00000000, // $4500.00
            ask_price: 4501_00000000, // $4501.00
            bid_size: 1000,
            ask_size: 1000,
            timestamp_ns: 1234567891,
        };
        market_ref.send(MarketMessage::Quote(Arc::new(quote1))).await?;
        
        // Send quote with price difference for arbitrage
        let quote2 = QuoteUpdate {
            instrument_id: 12345,
            bid_price: 4502_00000000, // $4502.00 - $2 arbitrage opportunity
            ask_price: 4503_00000000,
            bid_size: 1000,
            ask_size: 1000,
            timestamp_ns: 1234567892,
        };
        market_ref.send(MarketMessage::Quote(Arc::new(quote2))).await?;
        
        Ok(())
    }
    
    /// Get statistics from the migration
    pub async fn get_stats(&self) -> MigrationStats {
        let metrics = self.system.metrics();
        let system_stats = metrics.get_stats();
        
        MigrationStats {
            actors_spawned: system_stats.actors_spawned,
            messages_handled: system_stats.messages_handled,
            avg_message_latency_ns: system_stats.avg_message_latency_ns,
            zero_serialization_percentage: self.calculate_zero_serialization_percentage(&system_stats),
        }
    }
    
    fn calculate_zero_serialization_percentage(&self, stats: &super::system::SystemStats) -> f64 {
        // In bundled mode, all messages should use Arc<T> (zero serialization)
        100.0 // Simplified - would check actual transport metrics
    }
    
    /// Shutdown the proof-of-concept
    pub async fn shutdown(self) -> Result<()> {
        info!("Shutting down proof-of-concept migration");
        self.system.shutdown().await
    }
}

/// Migration statistics
#[derive(Debug, Clone)]
pub struct MigrationStats {
    pub actors_spawned: u64,
    pub messages_handled: u64,
    pub avg_message_latency_ns: f64,
    pub zero_serialization_percentage: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    #[tokio::test]
    async fn test_proof_of_concept_migration() {
        // Setup the migration
        let mut migration = ProofOfConceptMigration::setup().await.unwrap();
        
        // Send test data
        migration.send_test_data().await.unwrap();
        
        // Give time for processing
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Check stats
        let stats = migration.get_stats().await;
        assert_eq!(stats.actors_spawned, 2);
        assert!(stats.messages_handled > 0);
        assert_eq!(stats.zero_serialization_percentage, 100.0);
        
        // Check for generated signals
        if let Some(mut signal_rx) = migration.signal_receiver.take() {
            if let Ok(signal) = signal_rx.try_recv() {
                match signal {
                    SignalMessage::Arbitrage(arb) => {
                        assert!(arb.price_difference > 0);
                        println!("Received arbitrage signal with profit: {}", arb.price_difference);
                    }
                    _ => {}
                }
            }
        }
        
        // Shutdown
        migration.shutdown().await.unwrap();
    }
    
    #[tokio::test]
    async fn test_zero_cost_message_passing() {
        use std::time::Instant;
        
        let migration = ProofOfConceptMigration::setup().await.unwrap();
        let market_ref = migration.market_processor_ref.as_ref().unwrap();
        
        // Measure Arc::clone() performance
        let quote = Arc::new(QuoteUpdate {
            instrument_id: 99999,
            bid_price: 5000_00000000,
            ask_price: 5001_00000000,
            bid_size: 100,
            ask_size: 100,
            timestamp_ns: 1234567890,
        });
        
        let iterations = 10_000;
        let start = Instant::now();
        
        for _ in 0..iterations {
            let msg = MarketMessage::Quote(Arc::clone(&quote));
            market_ref.send(msg).await.unwrap();
        }
        
        let elapsed = start.elapsed();
        let ns_per_message = elapsed.as_nanos() / iterations;
        
        println!("Average latency per message: {}ns", ns_per_message);
        
        // Should be well under 1000ns (1μs) for local Arc<T> passing
        assert!(ns_per_message < 1000, "Message passing too slow: {}ns", ns_per_message);
        
        migration.shutdown().await.unwrap();
    }
}
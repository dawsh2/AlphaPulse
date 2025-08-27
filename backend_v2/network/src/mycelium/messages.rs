//! Message Type System (MYCEL-002)
//!
//! Domain-specific message enums that group related messages while maintaining
//! type safety and enabling efficient dispatch. Uses Arc<T> for zero-copy
//! sharing between actors.

#[cfg(feature = "protocol-integration")]
use alphapulse_types::protocol::{
    tlv::{TLVType, TradeTLV, QuoteTLV, PoolSwapTLV},
    message::TLVMessageBuilder,
};

use anyhow::Result;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::warn;

/// Core trait for all messages in the actor system
pub trait Message: Send + Sync + 'static {
    /// Convert to TLV for cross-process communication
    fn to_tlv(&self) -> Result<Vec<u8>>;
    
    /// Reconstruct from TLV bytes
    fn from_tlv(bytes: &[u8]) -> Result<Self>
    where 
        Self: Sized;
    
    /// Type ID for runtime checking and downcasting
    fn message_type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }
    
    /// Convert to Any for local passing
    fn as_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> where Self: Sized {
        self as Arc<dyn Any + Send + Sync>
    }
    
    /// Estimated message size for metrics
    fn estimated_size(&self) -> usize where Self: Sized {
        std::mem::size_of::<Self>()
    }
}

/// Market data domain messages
#[derive(Debug, Clone)]
pub enum MarketMessage {
    /// Pool swap event from DEX
    Swap(Arc<PoolSwapEvent>),
    /// Price quote update
    Quote(Arc<QuoteUpdate>),
    /// Order book update
    OrderBook(Arc<OrderBookUpdate>),
    /// Volume snapshot
    VolumeSnapshot(Arc<VolumeData>),
}

/// Signal generation domain messages
#[derive(Debug, Clone)]
pub enum SignalMessage {
    /// Arbitrage opportunity signal
    Arbitrage(Arc<ArbitrageSignal>),
    /// Momentum trading signal
    Momentum(Arc<MomentumSignal>),
    /// Liquidation opportunity signal
    Liquidation(Arc<LiquidationSignal>),
    /// Risk threshold breach
    RiskAlert(Arc<RiskAlert>),
}

/// Execution domain messages
#[derive(Debug, Clone)]
pub enum ExecutionMessage {
    /// Submit new order
    SubmitOrder(Arc<OrderRequest>),
    /// Cancel existing order
    CancelOrder(Arc<CancelRequest>),
    /// Execution result report
    ExecutionReport(Arc<ExecutionResult>),
    /// Position update
    PositionUpdate(Arc<PositionUpdate>),
}

/// Type-safe receiver for actor message channels
pub struct TypedReceiver<M: Message> {
    rx: mpsc::Receiver<Arc<dyn Any + Send + Sync>>,
    _phantom: PhantomData<M>,
}

impl<M: Message> TypedReceiver<M> {
    /// Create new typed receiver from channel
    pub fn new(rx: mpsc::Receiver<Arc<dyn Any + Send + Sync>>) -> Self {
        Self {
            rx,
            _phantom: PhantomData,
        }
    }
    
    /// Receive next message of expected type
    pub async fn recv(&mut self) -> Option<Arc<M>> {
        while let Some(any_msg) = self.rx.recv().await {
            // Try to downcast to expected type
            if let Ok(typed) = any_msg.downcast::<M>() {
                return Some(typed);
            } else {
                // Log unexpected message type and continue waiting
                warn!(
                    expected_type = std::any::type_name::<M>(),
                    "Received unexpected message type in TypedReceiver"
                );
            }
        }
        None
    }
    
    /// Try to receive without blocking
    pub fn try_recv(&mut self) -> Result<Arc<M>, tokio::sync::mpsc::error::TryRecvError> {
        match self.rx.try_recv() {
            Ok(any_msg) => {
                if let Ok(typed) = any_msg.downcast::<M>() {
                    Ok(typed)
                } else {
                    warn!(
                        expected_type = std::any::type_name::<M>(),
                        "Received unexpected message type in TypedReceiver"
                    );
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty)
                }
            },
            Err(e) => Err(e),
        }
    }
}

/// Message handler trait for efficient actor dispatch
pub trait MessageHandler: Send + Sync {
    type Message: Message;
    
    /// Handle incoming message
    async fn handle(&mut self, msg: Self::Message) -> Result<()>;
}

/// Example market data processor actor
pub struct MarketDataProcessor {
    pub processed_swaps: u64,
    pub processed_quotes: u64,
    pub processed_books: u64,
    pub processed_volume: u64,
}

impl MarketDataProcessor {
    pub fn new() -> Self {
        Self {
            processed_swaps: 0,
            processed_quotes: 0,
            processed_books: 0,
            processed_volume: 0,
        }
    }
    
    async fn handle_swap(&mut self, _event: Arc<PoolSwapEvent>) -> Result<()> {
        self.processed_swaps += 1;
        Ok(())
    }
    
    async fn handle_quote(&mut self, _quote: Arc<QuoteUpdate>) -> Result<()> {
        self.processed_quotes += 1;
        Ok(())
    }
    
    async fn handle_orderbook(&mut self, _book: Arc<OrderBookUpdate>) -> Result<()> {
        self.processed_books += 1;
        Ok(())
    }
    
    async fn handle_volume(&mut self, _vol: Arc<VolumeData>) -> Result<()> {
        self.processed_volume += 1;
        Ok(())
    }
}

impl MessageHandler for MarketDataProcessor {
    type Message = MarketMessage;
    
    async fn handle(&mut self, msg: MarketMessage) -> Result<()> {
        // Efficient enum dispatch - compiles to jump table
        match msg {
            MarketMessage::Swap(event) => self.handle_swap(event).await,
            MarketMessage::Quote(quote) => self.handle_quote(quote).await,
            MarketMessage::OrderBook(book) => self.handle_orderbook(book).await,
            MarketMessage::VolumeSnapshot(vol) => self.handle_volume(vol).await,
        }
    }
}

// Individual message types

/// Pool swap event from DEX monitoring
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PoolSwapEvent {
    pub pool_address: [u8; 20],
    pub token0_in: u64,
    pub token1_out: u64,
    pub timestamp_ns: u64,
    pub tx_hash: [u8; 32],
    pub gas_used: u64,
}

/// Price quote update from exchange
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuoteUpdate {
    pub instrument_id: u64,
    pub bid_price: i64,  // 8-decimal fixed point
    pub ask_price: i64,  // 8-decimal fixed point
    pub bid_size: u64,
    pub ask_size: u64,
    pub timestamp_ns: u64,
}

/// Order book update
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderBookUpdate {
    pub instrument_id: u64,
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
    pub timestamp_ns: u64,
    pub sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PriceLevel {
    pub price: i64,  // 8-decimal fixed point
    pub size: u64,
}

/// Volume data snapshot
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VolumeData {
    pub instrument_id: u64,
    pub volume_24h: u64,
    pub volume_1h: u64,
    pub timestamp_ns: u64,
}

/// Arbitrage opportunity signal
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArbitrageSignal {
    pub opportunity_id: u64,
    pub venue_a: u8,
    pub venue_b: u8,
    pub instrument_id: u64,
    pub price_difference: i64,  // 8-decimal fixed point
    pub potential_profit_usd: i64,  // 8-decimal fixed point
    pub confidence_score: u8,  // 0-100
    pub timestamp_ns: u64,
}

/// Momentum trading signal
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MomentumSignal {
    pub signal_id: u64,
    pub instrument_id: u64,
    pub direction: TradeDirection,
    pub strength: u8,  // 0-100
    pub duration_estimate_seconds: u32,
    pub timestamp_ns: u64,
}

/// Liquidation opportunity
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiquidationSignal {
    pub position_id: u64,
    pub instrument_id: u64,
    pub liquidation_price: i64,  // 8-decimal fixed point
    pub position_size: u64,
    pub timestamp_ns: u64,
}

/// Risk alert
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiskAlert {
    pub alert_id: u64,
    pub alert_type: RiskAlertType,
    pub severity: AlertSeverity,
    pub description: String,
    pub timestamp_ns: u64,
}

/// Order execution request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderRequest {
    pub order_id: u64,
    pub instrument_id: u64,
    pub side: TradeSide,
    pub order_type: OrderType,
    pub quantity: u64,
    pub price: Option<i64>,  // 8-decimal fixed point, None for market orders
    pub timestamp_ns: u64,
}

/// Cancel order request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CancelRequest {
    pub order_id: u64,
    pub timestamp_ns: u64,
}

/// Execution result
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub execution_id: u64,
    pub order_id: u64,
    pub status: ExecutionStatus,
    pub filled_quantity: u64,
    pub average_price: i64,  // 8-decimal fixed point
    pub fees: i64,  // 8-decimal fixed point
    pub timestamp_ns: u64,
}

/// Position update
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PositionUpdate {
    pub position_id: u64,
    pub instrument_id: u64,
    pub quantity: i64,  // Signed for long/short
    pub average_price: i64,  // 8-decimal fixed point
    pub unrealized_pnl: i64,  // 8-decimal fixed point
    pub timestamp_ns: u64,
}

// Enums for message fields

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeDirection {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    Market,
    Limit,
    StopLoss,
    TakeProfit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Pending,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskAlertType {
    PositionLimit,
    VolumeLimit,
    DrawdownLimit,
    VolatilitySpike,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
    Emergency,
}

// Message trait implementations

impl Message for MarketMessage {
    fn to_tlv(&self) -> Result<Vec<u8>> {
        match self {
            MarketMessage::Swap(event) => event.to_tlv(),
            MarketMessage::Quote(quote) => quote.to_tlv(),
            MarketMessage::OrderBook(book) => book.to_tlv(),
            MarketMessage::VolumeSnapshot(volume) => volume.to_tlv(),
        }
    }
    
    fn from_tlv(_bytes: &[u8]) -> Result<Self>
    where 
        Self: Sized 
    {
        // TODO: Implement TLV parsing - requires TLV header analysis
        todo!("TLV parsing implementation needed")
    }
    
    fn estimated_size(&self) -> usize {
        match self {
            MarketMessage::Swap(event) => std::mem::size_of_val(event.as_ref()),
            MarketMessage::Quote(quote) => std::mem::size_of_val(quote.as_ref()),
            MarketMessage::OrderBook(book) => {
                std::mem::size_of_val(book.as_ref()) + 
                book.bids.len() * std::mem::size_of::<PriceLevel>() +
                book.asks.len() * std::mem::size_of::<PriceLevel>()
            },
            MarketMessage::VolumeSnapshot(volume) => std::mem::size_of_val(volume.as_ref()),
        }
    }
}

impl Message for SignalMessage {
    fn to_tlv(&self) -> Result<Vec<u8>> {
        match self {
            SignalMessage::Arbitrage(signal) => signal.to_tlv(),
            SignalMessage::Momentum(signal) => signal.to_tlv(),
            SignalMessage::Liquidation(signal) => signal.to_tlv(),
            SignalMessage::RiskAlert(alert) => alert.to_tlv(),
        }
    }
    
    fn from_tlv(_bytes: &[u8]) -> Result<Self>
    where 
        Self: Sized 
    {
        // TODO: Implement TLV parsing
        todo!("TLV parsing implementation needed")
    }
}

impl Message for ExecutionMessage {
    fn to_tlv(&self) -> Result<Vec<u8>> {
        match self {
            ExecutionMessage::SubmitOrder(request) => request.to_tlv(),
            ExecutionMessage::CancelOrder(request) => request.to_tlv(),
            ExecutionMessage::ExecutionReport(report) => report.to_tlv(),
            ExecutionMessage::PositionUpdate(update) => update.to_tlv(),
        }
    }
    
    fn from_tlv(_bytes: &[u8]) -> Result<Self>
    where 
        Self: Sized 
    {
        // TODO: Implement TLV parsing
        todo!("TLV parsing implementation needed")
    }
}

// Individual message TLV implementations using Protocol V2

#[cfg(feature = "protocol-integration")]
macro_rules! impl_message_tlv {
    ($type:ty, $tlv_type:expr, $domain:expr) => {
        impl Message for $type {
            fn to_tlv(&self) -> Result<Vec<u8>> {
                use alphapulse_types::protocol::{
                    message::{TLVMessageBuilder, SourceType, RelayDomain},
                };
                
                let mut builder = TLVMessageBuilder::new($domain, SourceType::Internal);
                builder.add_tlv($tlv_type, self);
                Ok(builder.build())
            }
            
            fn from_tlv(bytes: &[u8]) -> Result<Self>
            where 
                Self: Sized 
            {
                // Parse TLV message and extract payload for this type
                use alphapulse_types::protocol::message::parse_tlv_message;
                let parsed = parse_tlv_message(bytes)?;
                
                // Find the TLV matching our type
                for tlv in parsed.tlvs {
                    if tlv.tlv_type == $tlv_type as u16 {
                        // Deserialize the payload (would use proper Protocol V2 deserializer)
                        return Ok(bincode::deserialize(&tlv.payload)?);
                    }
                }
                
                anyhow::bail!("TLV type {} not found in message", $tlv_type as u16)
            }
        }
    };
}

#[cfg(not(feature = "protocol-integration"))]
macro_rules! impl_message_tlv {
    ($type:ty, $tlv_type:expr, $domain:expr) => {
        impl Message for $type {
            fn to_tlv(&self) -> Result<Vec<u8>> {
                // Fallback: Use bincode when Protocol V2 not available
                // This creates a minimal TLV-like structure
                let payload = bincode::serialize(self)?;
                let mut message = Vec::new();
                
                // Simple TLV header: [type:2][length:2][payload...]
                message.extend_from_slice(&($tlv_type as u16).to_le_bytes());
                message.extend_from_slice(&(payload.len() as u16).to_le_bytes());
                message.extend_from_slice(&payload);
                
                Ok(message)
            }
            
            fn from_tlv(bytes: &[u8]) -> Result<Self>
            where 
                Self: Sized 
            {
                if bytes.len() < 4 {
                    anyhow::bail!("Message too short for TLV header");
                }
                
                let payload = &bytes[4..];
                Ok(bincode::deserialize(payload)?)
            }
        }
    };
}

// Add serde derives for fallback serialization
use serde::{Serialize, Deserialize};

// Market Data domain messages (TLV types 1-19)
// CRITICAL: TLV type numbers MUST match central registry in libs/types/src/protocol/tlv/types.rs
impl_message_tlv!(PoolSwapEvent, 11, 1);    // Market Data domain - matches PoolSwap = 11
impl_message_tlv!(QuoteUpdate, 17, 1);      // Market Data domain - matches QuoteUpdate = 17
impl_message_tlv!(OrderBookUpdate, 3, 1);   // Market Data domain - matches OrderBook = 3
impl_message_tlv!(VolumeData, 9, 1);        // Market Data domain - matches VolumeUpdate = 9

// Signal domain messages (TLV types 20-39) 
// CRITICAL: Signal domain TLV numbers MUST match registry
impl_message_tlv!(ArbitrageSignal, 32, 2);  // Signal domain - matches ArbitrageSignal = 32
impl_message_tlv!(MomentumSignal, 21, 2);   // Signal domain - matches AssetCorrelation = 21
impl_message_tlv!(LiquidationSignal, 62, 2); // Signal domain - matches RiskAlert = 62
impl_message_tlv!(RiskAlert, 62, 2);        // Signal domain - matches RiskAlert = 62

// Execution domain messages (TLV types 40-79)
// CRITICAL: Execution domain TLV numbers MUST match registry  
impl_message_tlv!(OrderRequest, 40, 3);     // Execution domain - matches OrderRequest = 40
impl_message_tlv!(CancelRequest, 43, 3);    // Execution domain - matches OrderCancel = 43
impl_message_tlv!(ExecutionResult, 42, 3);  // Execution domain - matches Fill = 42
impl_message_tlv!(PositionUpdate, 61, 3);   // Execution domain - matches PositionUpdate = 61

/// Message registry for debugging and monitoring
#[derive(Debug, Default)]
pub struct MessageRegistry {
    types: HashMap<TypeId, &'static str>,
    counts: HashMap<TypeId, AtomicU64>,
}

impl MessageRegistry {
    pub fn new() -> Self {
        let mut registry = Self::default();
        
        // Register known message types
        registry.register::<MarketMessage>("MarketMessage");
        registry.register::<SignalMessage>("SignalMessage");
        registry.register::<ExecutionMessage>("ExecutionMessage");
        registry.register::<PoolSwapEvent>("PoolSwapEvent");
        registry.register::<QuoteUpdate>("QuoteUpdate");
        registry.register::<ArbitrageSignal>("ArbitrageSignal");
        
        registry
    }
    
    /// Register a message type
    pub fn register<M: Message>(&mut self, name: &'static str) {
        let type_id = TypeId::of::<M>();
        self.types.insert(type_id, name);
        self.counts.insert(type_id, AtomicU64::new(0));
    }
    
    /// Record message processed
    pub fn record_message<M: Message>(&self) {
        if let Some(counter) = self.counts.get(&TypeId::of::<M>()) {
            counter.fetch_add(1, Ordering::Relaxed);
        }
    }
    
    /// Get message statistics
    pub fn get_stats(&self) -> MessageStats {
        let mut message_counts = HashMap::new();
        
        for (type_id, counter) in &self.counts {
            if let Some(&name) = self.types.get(type_id) {
                let count = counter.load(Ordering::Relaxed);
                message_counts.insert(name.to_string(), count);
            }
        }
        
        MessageStats { message_counts }
    }
}

/// Message statistics
#[derive(Debug, Clone)]
pub struct MessageStats {
    pub message_counts: HashMap<String, u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_enum_sizes() {
        // All message enums should be 16 bytes (8 byte Arc + 8 byte discriminant)
        assert_eq!(std::mem::size_of::<MarketMessage>(), 16);
        assert_eq!(std::mem::size_of::<SignalMessage>(), 16);
        assert_eq!(std::mem::size_of::<ExecutionMessage>(), 16);
    }

    #[test]
    fn test_message_registry() {
        let mut registry = MessageRegistry::new();
        registry.register::<PoolSwapEvent>("PoolSwap");
        
        // Record some messages
        registry.record_message::<PoolSwapEvent>();
        registry.record_message::<PoolSwapEvent>();
        
        let stats = registry.get_stats();
        assert_eq!(stats.message_counts["PoolSwap"], 2);
    }

    #[test]
    fn test_arc_sharing() {
        let event = Arc::new(PoolSwapEvent {
            pool_address: [1; 20],
            token0_in: 1000,
            token1_out: 2000,
            timestamp_ns: 12345,
            tx_hash: [2; 32],
            gas_used: 21000,
        });
        
        let msg1 = MarketMessage::Swap(Arc::clone(&event));
        let msg2 = MarketMessage::Swap(Arc::clone(&event));
        
        // Both messages point to same allocation
        assert_eq!(Arc::strong_count(&event), 3);
    }

    #[test]
    fn test_message_type_ids() {
        let swap = PoolSwapEvent {
            pool_address: [1; 20],
            token0_in: 1000,
            token1_out: 2000,
            timestamp_ns: 12345,
            tx_hash: [2; 32],
            gas_used: 21000,
        };
        
        let quote = QuoteUpdate {
            instrument_id: 123,
            bid_price: 4500000000000,  // $45,000.00
            ask_price: 4500100000000,  // $45,001.00
            bid_size: 1000,
            ask_size: 1000,
            timestamp_ns: 12345,
        };
        
        assert_ne!(swap.message_type_id(), quote.message_type_id());
    }

    #[tokio::test]
    async fn test_typed_receiver() {
        let (tx, rx) = mpsc::channel(10);
        let mut typed_rx = TypedReceiver::<PoolSwapEvent>::new(rx);
        
        // Send correct type
        let swap = Arc::new(PoolSwapEvent {
            pool_address: [1; 20],
            token0_in: 1000,
            token1_out: 2000,
            timestamp_ns: 12345,
            tx_hash: [2; 32],
            gas_used: 21000,
        });
        
        tx.send(swap.clone() as Arc<dyn Any + Send + Sync>).await.unwrap();
        
        // Receive and verify
        let received = typed_rx.recv().await.unwrap();
        assert_eq!(*received, *swap);
    }

    #[tokio::test]
    async fn test_message_handler() {
        let mut processor = MarketDataProcessor::new();
        
        let swap_event = Arc::new(PoolSwapEvent {
            pool_address: [1; 20],
            token0_in: 1000,
            token1_out: 2000,
            timestamp_ns: 12345,
            tx_hash: [2; 32],
            gas_used: 21000,
        });
        
        let market_msg = MarketMessage::Swap(swap_event);
        
        // Handle message
        processor.handle(market_msg).await.unwrap();
        
        // Verify processing
        assert_eq!(processor.processed_swaps, 1);
        assert_eq!(processor.processed_quotes, 0);
    }

    #[test]
    fn test_enum_dispatch_performance() {
        use std::time::Instant;
        
        let event = Arc::new(PoolSwapEvent {
            pool_address: [1; 20],
            token0_in: 1000,
            token1_out: 2000,
            timestamp_ns: 12345,
            tx_hash: [2; 32],
            gas_used: 21000,
        });
        let msg = MarketMessage::Swap(event);
        
        // Enum dispatch should be a simple jump table
        let start = Instant::now();
        for _ in 0..1_000_000 {
            match &msg {
                MarketMessage::Swap(_) => {},
                MarketMessage::Quote(_) => {},
                MarketMessage::OrderBook(_) => {},
                MarketMessage::VolumeSnapshot(_) => {},
            }
        }
        let elapsed = start.elapsed();
        
        // Should be <10ns per dispatch (very generous threshold)
        let ns_per_dispatch = elapsed.as_nanos() / 1_000_000;
        assert!(ns_per_dispatch < 100, "Dispatch took {}ns, expected <100ns", ns_per_dispatch);
    }
}
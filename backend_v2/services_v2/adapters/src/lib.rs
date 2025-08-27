//! # AlphaPulse Adapters - Protocol V2 Data Transformation Layer
//!
//! ## Purpose
//!
//! High-performance stateless adapters that transform raw exchange data into Protocol V2 TLV
//! messages for the AlphaPulse trading system. Provides unified data collection from centralized
//! exchanges (CEX) and decentralized exchanges (DEX) with comprehensive validation, circuit breaker
//! protection, and sub-millisecond conversion latency optimized for high-frequency trading workloads.
//!
//! ## Integration Points
//!
//! - **Input Sources**: WebSocket feeds, RPC endpoints, event streams from 15+ exchanges
//! - **Output Destinations**: Domain-specific relays (MarketData, Signal, Execution)
//! - **Validation Pipeline**: Four-step validation ensuring zero data loss during transformation
//! - **Monitoring**: Circuit breakers, rate limiting, metrics collection for operational health
//! - **Configuration**: Dynamic venue-specific settings and credential management
//! - **Error Handling**: Comprehensive error classification and recovery strategies
//!
//! ## Architecture Role
//!
//! Adapters serve as the critical boundary between external exchange protocols and the
//! unified Protocol V2 message system, ensuring data integrity and performance isolation.
//!
//! See [`architecture_diagram()`] for visual representation of the data flow.
//!
//! ## Performance Profile
//!
//! - **Conversion Latency**: <1ms event-to-TLV transformation
//! - **Throughput**: 10,000+ events/second per venue adapter
//! - **Memory Usage**: <256MB per collector with bounded buffers
//! - **Connection Recovery**: <5 seconds automatic reconnection
//! - **Validation Speed**: <2ms for complete four-step pipeline
//! - **Error Recovery**: <100ms circuit breaker response time
//!
//! ### Zero-Copy Architecture with Minimal Allocation
//!
//! **AlphaPulse adapters achieve near-zero-copy performance with a single required allocation:**
//!
//! ```rust
//! // Pattern: Zero-copy construction + one allocation for async ownership
//! with_hot_path_buffer(|buffer| {
//!     // ✅ ZERO allocations: Direct buffer write (~15ns)
//!     let size = TrueZeroCopyBuilder::new(domain, source)
//!         .build_into_buffer(buffer, tlv_type, &tlv_data)?;
//!
//!     // ✅ ONE allocation: Required for Rust async + cross-thread send (~5ns)
//!     let message = buffer[..size].to_vec();
//!     relay_output.send_bytes(message).await
//! })
//! // Total: ~25ns per message construction
//! ```
//!
//! **Why the Single Allocation Cannot Be Eliminated:**
//! - **Rust Ownership**: Async functions require owned data across await points
//! - **Thread Safety**: Message must survive async socket operations
//! - **Architecture**: Even direct RelayOutput patterns require `Vec<u8>` ownership
//!
//! **Architecture Comparison:**
//! - **Channel-based** (legacy): WebSocket → TLV(Vec) → Channel → Relay Thread → Socket
//! - **Direct** (optimized): WebSocket → TLV(Vec) → RelayOutput → Socket
//!
//! Both require the same single Vec allocation due to Rust's async ownership model.
//! The optimization eliminates thread boundaries and channel overhead, not the core allocation.
//!
//! **Measured Performance:** 1M messages/second = 1M allocations (~25ns each) = minimal GC pressure
//!
//! ## Stateless Transformation Principles
//!
//! ### ✅ Adapters ARE:
//! - **Stateless transformers**: Raw Data → Protocol V2 TLV Messages
//! - **Format converters**: JSON/Binary/WebSocket → Typed TLV structs
//! - **Validators**: Four-step validation ensuring zero data loss
//! - **Forwarders**: Route to domain-specific relays (MarketData, Signal, Execution)
//! - **Connection managers**: WebSocket/RPC connection handling with recovery
//! - **Error handlers**: Circuit breakers, rate limiting, retry logic
//!
//! ### ❌ Adapters are NOT:
//! - **State managers** (no StateManager, order books, or data aggregation)
//! - **Business logic** (no trading decisions or signal generation)
//! - **Historical storage** (no databases or persistent state)
//! - **Cross-exchange logic** (no arbitrage detection or multi-venue analysis)
//!
//! ## Common Mistakes
//!
//! ### Wrong: StateManager in Adapter
//! ```rust
//! // ❌ WRONG - Adapters should NOT have state
//! struct BadAdapter {
//!     state: Arc<StateManager>,  // ❌ No!
//!     order_book: OrderBook,     // ❌ No!
//! }
//! ```
//!
//! ### Right: Stateless Transformation
//! ```rust
//! // ✅ CORRECT - Adapters are pure transformers
//! struct GoodAdapter {
//!     connection: Arc<ConnectionManager>,  // ✅ Connection only
//!     output_tx: Sender<Vec<u8>>,         // ✅ Output channel (Protocol V2 binary)
//!     metrics: Arc<AdapterMetrics>,       // ✅ Monitoring
//!     // No state management!
//! }
//!
//! impl GoodAdapter {
//!     async fn process(&self, json: &str) -> Result<()> {
//!         let parsed = parse_message(json)?;     // Parse
//!         let tlv = TradeTLV::try_from(parsed)?; // Convert
//!
//!         // Build Protocol V2 message
//!         let message = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::CoinbaseCollector)
//!             .add_tlv(TLVType::Trade, &tlv)
//!             .build();
//!
//!         self.output_tx.send(message).await?; // Forward
//!         Ok(()) // Simple!
//!     }
//! }
//! ```
//!
//! ## Production Adapters
//!
//! | Exchange | Type | Protocol | Status | Template For |
//! |----------|------|----------|--------|--------------|
//! | [`CoinbaseCollector`] | CEX | WebSocket | ✅ Production | **CEX adapters** |
//! | bin/polygon/polygon.rs | DEX | WebSocket | ✅ Production | **Unified DEX collector** |
//! | [`KrakenCollector`] | CEX | WebSocket | ⚠️ Legacy | - |
//! | [`BinanceCollector`] | CEX | WebSocket | ⚠️ Legacy | - |
//!
//! ## Examples
//!
//! ### Complete CEX Adapter Implementation
//! ```rust
//! use alphapulse_adapters::{CoinbaseCollector, RelayDomain, SourceType};
//! use alphapulse_alphapulse_types::protocol::{TLVMessageBuilder, TLVType};
//! use tokio::sync::mpsc;
//!
//! // Production-ready CEX adapter usage
//! let (message_tx, message_rx) = mpsc::channel(10000);
//! let collector = CoinbaseCollector::new(
//!     vec!["BTC-USD".to_string(), "ETH-USD".to_string()],
//!     message_tx.clone()
//! );
//!
//! // Start data collection with automatic reconnection
//! tokio::spawn(async move {
//!     collector.start().await.expect("Collector failed");
//! });
//!
//! // Process incoming TLV messages
//! while let Some(message_bytes) = message_rx.recv().await {
//!     relay_client.send_to_market_data_relay(message_bytes).await?;
//! }
//! ```
//!
//! ### DEX Adapter with Pool Monitoring
//! ```rust
//! // Use the unified polygon binary instead: bin/polygon/polygon.rs
//!
//! // Monitor specific pools for arbitrage opportunities
//! let pools = vec![
//!     "0x45dDa9cb7c25131DF268515131f647d726f50608".to_string(), // USDC/WETH
//!     "0xA374094527e1673A86dE625aa59517c5dE346d32".to_string(), // USDC/WMATIC
//! ];
//!
//! // See bin/polygon/polygon.rs for direct relay integration
//! tokio::spawn(async move {
//!     dex_collector.collect_pool_events().await.expect("DEX collector failed");
//! });
//! ```
//!
//! ### Four-Step Validation Pipeline
//! ```rust
//! use alphapulse_adapters::validation::complete_validation_pipeline;
//!
//! #[test]
//! fn test_adapter_zero_data_loss() -> Result<()> {
//!     // Load real exchange data (never use mocks)
//!     let raw_json = load_real_fixture("coinbase_trades.json");
//!     let parsed_event = CoinbaseTradeEvent::from_json(&raw_json)?;
//!
//!     // Comprehensive validation ensuring bijective transformation
//!     complete_validation_pipeline(raw_json.as_bytes(), parsed_event)?;
//!
//!     println!("✅ Zero data loss validated");
//!     Ok(())
//! }
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]

// Common adapter infrastructure and shared utilities
pub mod common;

// Core utilities (formerly in libs/adapters)
pub mod circuit_breaker;
pub mod rate_limit;

// Plugin architecture adapter implementations
pub mod adapters;

// Legacy adapter implementations
pub mod config;
pub mod error;
pub mod input;
pub mod latency_instrumentation;
pub mod output;
pub mod polygon;
pub mod validation;

// Re-export common adapter infrastructure
pub use common::{
    Adapter, AdapterFactory, AdapterHealth, AdapterOutput, BaseAdapterConfig, ChannelOutput,
    ConfigurableAdapter, ConnectionStatus, InstrumentType, SafeAdapter,
};

// Re-export core utilities for adapter developers
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
pub use common::auth::{ApiCredentials, AuthManager};
pub use common::metrics::{AdapterMetrics, ErrorType};
pub use rate_limit::{RateLimitConfig, RateLimiter};

// Re-export existing adapter types
pub use error::{AdapterError, Result};
pub use input::InputAdapter;
pub use latency_instrumentation::{
    global_instrument, LatencyInstrument, LatencyMetrics, LatencyStats, MessageType,
    ProcessingToken,
};
pub use output::OutputAdapter;
pub use validation::{
    complete_validation_pipeline, validate_equality, validate_raw_parsing,
    validate_tlv_deserialization, validate_tlv_serialization, RawDataValidator, SemanticValidator,
    ValidationConfig, ValidationError, ValidationResult,
};

#[cfg(feature = "strict-validation")]
pub use validation::ValidatedAdapter;

// Re-export plugin adapters
pub use adapters::{CoinbaseAdapterConfig, CoinbasePluginAdapter};

// Re-export legacy collectors for external use
pub use input::collectors::{
    BinanceCollector,
    CoinbaseCollector, // GeminiCollector,
    KrakenCollector,
    // Polygon collector implemented as standalone binary: bin/polygon/polygon.rs
};

// Re-export protocol types for convenience
pub use alphapulse_types::{
    InstrumentId, InvalidationReason, QuoteTLV, TLVType, TradeTLV, VenueId,
};

// Re-export codec functionality
pub use codec::TLVMessageBuilder;

/// Architecture diagram showing adapter service data flow and component relationships
#[cfg_attr(doc, aquamarine::aquamarine)]
/// ```mermaid
/// graph LR
///     subgraph Exchanges["🌐 External Exchanges"]
///         direction TB
///         WS[WebSocket/RPC]
///         JB[JSON/Binary]
///         RL[Rate Limited]
///         CB[Circuit Breaker]
///     end
///
///     subgraph Adapters["⚡ Adapter Layer"]
///         direction TB
///         ST[Stateless Transformation]
///         VA[Validation Pipeline]
///         ER[Error Recovery]
///         CO[Connection Management]
///     end
///
///     subgraph Relays["📡 Domain Relays"]
///         direction TB
///         TM[TLV Messages]
///         P2[Protocol V2]
///         BF[Binary Format]
///         RR[Relay Routing]
///     end
///
///     subgraph Services["🎯 Strategy Services"]
///         direction TB
///         BL[Business Logic]
///         TD[Trading Decisions]
///         PU[Position Updates]
///         AD[Arbitrage Detection]
///     end
///
///     WS --> ST
///     JB --> VA
///     RL --> ER
///     CB --> CO
///
///     ST --> TM
///     VA --> P2
///     ER --> BF
///     CO --> RR
///
///     TM --> BL
///     P2 --> TD
///     BF --> PU
///     RR --> AD
///
///     style Exchanges fill:#ffebee
///     style Adapters fill:#fff3e0
///     style Relays fill:#e8f5e9
///     style Services fill:#e3f2fd
/// ```
pub fn architecture_diagram() {
    // This function exists solely for documentation purposes
    // The diagram is rendered by aquamarine in rustdoc
}

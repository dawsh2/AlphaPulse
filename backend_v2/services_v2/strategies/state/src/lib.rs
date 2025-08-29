//! # Market State Management - Real-Time DEX Pool Tracking
//!
//! ## Purpose
//!
//! High-performance market state management system providing real-time tracking of
//! decentralized exchange pool states, order book data, and cross-venue price aggregation.
//! Implements microsecond-latency embedded state access and scalable service-based
//! architecture for sophisticated trading strategy support with comprehensive validation.
//!
//! ## Integration Points
//!
//! - **Input Sources**: Pool swap events, mint/burn events, tick updates from DEX protocols
//! - **Output Destinations**: Strategy engines, arbitrage detectors, portfolio managers
//! - **State Persistence**: Background cache management with atomic snapshot operations
//! - **Validation**: Pool state integrity checking and cross-reference validation
//! - **Performance Modes**: Embedded (μs latency) and service-based (IPC) architectures
//! - **Protocol Support**: Uniswap V2/V3, SushiSwap, QuickSwap, Curve, Balancer pools
//!
//! ## Architecture Role
//!
//! ```text
//! DEX Pool Events → [State Management] → [Pool Cache] → [Strategy Access]
//!       ↓                  ↓                  ↓               ↓
//! Swap Events         Real-time Updates   Background Sync   μs Latency Reads
//! Mint/Burn Events    Reserve Tracking    Atomic Snapshots  Arbitrage Detection
//! Tick Updates        Price Calculation   Persistence       Portfolio Updates
//! Liquidity Changes   Validation Pipeline Cache Management  Strategy Decisions
//!
//! Mode Selection:
//! ┌─ Embedded Mode: Strategy → Direct Memory Access → Pool State (μs latency)
//! └─ Service Mode:   Strategy → IPC Channel → State Service → Pool State (ms latency)
//! ```
//!
//! Market state system serves as the foundational data layer enabling sophisticated
//! trading strategies through reliable, high-performance access to DEX market conditions.
//!
//! ## Performance Profile
//!
//! - **State Update Speed**: <5μs per pool state modification from swap events
//! - **Embedded Access**: <1μs pool state lookup via direct memory access
//! - **Service Mode Latency**: <2ms IPC round-trip for cross-process state access
//! - **Cache Persistence**: <50μs background snapshot write with zero hot-path blocking
//! - **Memory Usage**: <32MB for tracking 1000+ active pools with full history
//! - **Validation Speed**: <10μs per pool state integrity check with cross-referencing
//!
//! ## Architecture Modes
//!
//! ### Embedded Mode (Recommended for Strategies)
//! Strategy embeds PoolStateManager directly for microsecond-latency pool access.
//! Ideal for arbitrage detection, real-time trading, and high-frequency operations.
//!
//! ### Service Mode (Multi-Consumer Scenarios)
//! PoolStateManager runs as separate service, providing state via IPC channels.
//! Suitable for dashboard displays, analytics, and non-latency-critical consumers.

pub mod pool_cache;
pub mod pool_state;
pub mod pool_validator;

pub use pool_state::{
    ArbitragePair,
    PoolEvent,
    PoolState,
    PoolStateError,
    PoolStateManager,
    StrategyArbitragePair,
    // Strategy compatibility exports
    StrategyPoolState,
    V2PoolState,
    V3PoolState,
};

pub use pool_cache::{
    PoolCache, PoolCacheConfig, PoolCacheError, PoolCacheEvent, PoolCacheStats, PoolInfo,
};

pub use pool_validator::{PoolValidator, ValidatedSwap};

// Re-export core traits for convenience
pub use state_core::{SequencedStateful, StateError, Stateful};

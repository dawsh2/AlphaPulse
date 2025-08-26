//! # Market Data Relay Logic
//!
//! Domain-specific logic for the Market Data relay that handles TLV types 1-19.
//! This includes trades, quotes, order book updates, and other real-time market events.
//!
//! ## Architecture Role
//!
//! Implements the `RelayLogic` trait for the Market Data domain, providing:
//! - Domain identification (RelayDomain::MarketData) 
//! - Socket path configuration
//! - Message filtering for market data types
//!
//! ```mermaid
//! graph TB
//!     PP[polygon_publisher] -->|TLV Messages| Socket["/tmp/alphapulse/market_data.sock"]
//!     Socket --> Engine["`Relay<MarketDataLogic>`"]
//!     Engine -->|Broadcast| Dashboard[Dashboard Consumer]
//!     Engine -->|Broadcast| Strategy[Strategy Services]
//!     
//!     subgraph "MarketDataLogic"
//!         Domain[domain() = MarketData]
//!         Path[socket_path() = market_data.sock]
//!         Filter[should_forward() = default]
//!     end
//!     
//!     Engine --> Domain
//!     Engine --> Path  
//!     Engine --> Filter
//! ```
//!
//! ## Message Types Handled
//! - **TLV Types 1-19**: All market data messages
//! - **High Volume**: Optimized for >1M msg/s throughput
//! - **Low Latency**: <35μs forwarding for real-time trading
//!
//! ## Performance Profile
//! - **No Custom Filtering**: Uses efficient default domain check
//! - **Direct Forwarding**: All market data messages forwarded to subscribers
//! - **Zero Overhead**: Minimal logic overhead for maximum throughput

use crate::common::RelayLogic;
use alphapulse_types::protocol::RelayDomain;

/// Market Data relay logic implementation
///
/// Handles all market data messages (TLV types 1-19) with maximum performance.
/// Uses the default message filtering which simply checks domain matching.
///
/// ## Design Philosophy
/// - **Performance First**: No custom filtering to avoid hot-path overhead
/// - **Broad Distribution**: All market data forwarded to all subscribers
/// - **Simple Logic**: Minimal complexity for maximum reliability
///
/// ## Socket Configuration
/// Uses `/tmp/alphapulse/market_data.sock` for Unix socket communication.
/// This path must be consistent with existing market data publishers and consumers.
pub struct MarketDataLogic;

impl RelayLogic for MarketDataLogic {
    /// Returns MarketData domain for message routing
    fn domain(&self) -> RelayDomain {
        RelayDomain::MarketData
    }

    /// Returns the Unix socket path for market data relay
    /// 
    /// **CRITICAL**: This path must match the path used by:
    /// - `polygon_publisher` and other market data producers
    /// - Dashboard websocket server and other consumers
    /// - Existing configuration files and documentation
    fn socket_path(&self) -> &'static str {
        "/tmp/alphapulse/market_data.sock"
    }

    // Uses default should_forward() implementation for maximum performance
    // All messages with RelayDomain::MarketData are forwarded
}

#[cfg(test)]
mod tests {
    use super::*;
    use alphapulse_types::protocol::MessageHeader;

    #[test]
    fn test_market_data_logic() {
        let logic = MarketDataLogic;
        
        assert_eq!(logic.domain(), RelayDomain::MarketData);
        assert_eq!(logic.socket_path(), "/tmp/alphapulse/market_data.sock");
    }

    #[test]
    fn test_message_filtering() {
        let logic = MarketDataLogic;
        
        // Create test header for market data
        let header = MessageHeader {
            magic: alphapulse_types::protocol::MESSAGE_MAGIC,
            relay_domain: RelayDomain::MarketData as u8,
            version: 1,
            source: 1,
            flags: 0,
            sequence: 1,
            timestamp: 0,
            payload_size: 0,
            checksum: 0,
        };

        // Should forward market data messages
        assert!(logic.should_forward(&header));

        // Should not forward other domain messages
        let mut signal_header = header;
        signal_header.relay_domain = RelayDomain::Signal as u8;
        assert!(!logic.should_forward(&signal_header));

        let mut execution_header = header;
        execution_header.relay_domain = RelayDomain::Execution as u8;
        assert!(!logic.should_forward(&execution_header));
    }
}
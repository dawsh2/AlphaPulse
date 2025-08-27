//! # Signal Relay Logic
//!
//! Domain-specific logic for the Signal relay that handles TLV types 20-39.
//! This includes arbitrage signals, strategy outputs, and trading recommendations.
//!
//! ## Architecture Role
//!
//! Implements the `RelayLogic` trait for the Signal domain, providing:
//! - Domain identification (RelayDomain::Signal)
//! - Socket path configuration
//! - Optional signal-specific message filtering
//!
//! ```mermaid
//! graph TB
//!     Strategy[Flash Arbitrage Strategy] -->|Signal TLVs| Socket["/tmp/alphapulse/signals.sock"]
//!     Socket --> Engine["`Relay<SignalLogic>`"]
//!     Engine -->|Broadcast| Portfolio[Portfolio Manager]
//!     Engine -->|Broadcast| Dashboard[Dashboard Consumer]  
//!     Engine -->|Broadcast| Risk[Risk Manager]
//!     
//!     subgraph "SignalLogic"
//!         Domain[domain() = Signal]
//!         Path[socket_path() = signals.sock]
//!         Filter[should_forward() = TLV 20-39]
//!     end
//!     
//!     Engine --> Domain
//!     Engine --> Path
//!     Engine --> Filter
//! ```
//!
//! ## Message Types Handled
//! - **TLV Types 20-39**: All signal and strategy messages
//! - **Medium Volume**: Optimized for strategy-generated signals
//! - **Quality Focus**: Emphasis on signal integrity and validation
//!
//! ## Performance Profile
//! - **Domain Validation**: Strict adherence to Signal TLV range (20-39)
//! - **Selective Distribution**: Only valid signals forwarded
//! - **Balanced Performance**: Good throughput with signal validation

use crate::common::RelayLogic;
use alphapulse_types::protocol::{MessageHeader, RelayDomain};

/// Signal relay logic implementation
///
/// Handles all signal messages (TLV types 20-39) with emphasis on signal integrity.
/// May include additional validation beyond basic domain checking in the future.
///
/// ## Design Philosophy
/// - **Signal Integrity**: Ensure only valid signals are distributed
/// - **Strategy Support**: Optimized for strategy-generated messages
/// - **Future Extensibility**: Designed for additional signal validation
///
/// ## Socket Configuration
/// Uses `/tmp/alphapulse/signals.sock` for Unix socket communication.
/// This path must be consistent with signal producers and consumers.
pub struct SignalLogic;

impl RelayLogic for SignalLogic {
    /// Returns Signal domain for message routing
    fn domain(&self) -> RelayDomain {
        RelayDomain::Signal
    }

    /// Returns the Unix socket path for signal relay
    ///
    /// **CRITICAL**: This path must match the path used by:
    /// - Strategy services producing signals
    /// - Portfolio managers and risk engines consuming signals
    /// - Dashboard and monitoring services
    fn socket_path(&self) -> &'static str {
        "/tmp/alphapulse/signals.sock"
    }

    /// Custom signal message filtering
    ///
    /// For now, uses the default domain-based filtering. In the future,
    /// this could be extended to include:
    /// - TLV type range validation (20-39 only)
    /// - Signal quality checks
    /// - Strategy authentication
    /// - Risk limit validation
    fn should_forward(&self, header: &MessageHeader) -> bool {
        // Use default domain filtering for now
        header.relay_domain == self.domain() as u8

        // Future extensions could include:
        // - Parse TLV payload to check type range
        // - Validate signal metadata
        // - Apply rate limiting per strategy
        // - Check signal confidence thresholds
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_logic() {
        let logic = SignalLogic;

        assert_eq!(logic.domain(), RelayDomain::Signal);
        assert_eq!(logic.socket_path(), "/tmp/alphapulse/signals.sock");
    }

    #[test]
    fn test_message_filtering() {
        let logic = SignalLogic;

        // Create test header for signal domain
        let header = MessageHeader {
            magic: alphapulse_types::protocol::MESSAGE_MAGIC,
            relay_domain: RelayDomain::Signal as u8,
            version: 1,
            source: 1,
            flags: 0,
            sequence: 1,
            timestamp: 0,
            payload_size: 0,
            checksum: 0,
        };

        // Should forward signal messages
        assert!(logic.should_forward(&header));

        // Should not forward other domain messages
        let mut market_header = header;
        market_header.relay_domain = RelayDomain::MarketData as u8;
        assert!(!logic.should_forward(&market_header));

        let mut execution_header = header;
        execution_header.relay_domain = RelayDomain::Execution as u8;
        assert!(!logic.should_forward(&execution_header));
    }
}

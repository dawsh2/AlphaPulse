//! # Signal Relay Binary - Generic Engine Implementation
//!
//! Unix socket server for Protocol V2 signal messages using the new
//! generic relay engine architecture.
//!
//! ## Architecture Changes
//!
//! **Before**: Custom signal relay implementation with duplicated connection handling
//! **After**: Uses generic `Relay<SignalLogic>` engine with signal-specific logic
//!
//! ## Performance Profile
//! - **Throughput**: Optimized for strategy-generated signals
//! - **Latency**: <35μs forwarding maintained
//! - **Reliability**: Shared engine reduces bug surface area
//! - **Maintainability**: Single codebase for all relay types
//!
//! ## Usage
//! ```bash
//! # Start the signal relay
//! cargo run --release --bin signal_relay
//!
//! # Or using the relays package
//! cargo run --release -p alphapulse-relays --bin signal_relay
//! ```

use alphapulse_relays::common::{Relay, RelayLogic};
use alphapulse_relays::signal::SignalLogic;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("🚀 Starting Generic Signal Relay");
    info!("📋 Using new generic engine architecture");

    // Create signal logic
    let logic = SignalLogic;
    info!("✅ Signal Logic: domain={:?}, socket={}", 
          logic.domain(), logic.socket_path());

    // Create and start relay
    let mut relay = Relay::new(logic);
    
    match relay.run().await {
        Ok(()) => {
            info!("✅ Signal Relay completed successfully");
            Ok(())
        }
        Err(e) => {
            error!("❌ Signal Relay failed: {}", e);
            Err(Box::new(e))
        }
    }
}
//! # Execution Relay Binary - Generic Engine Implementation
//!
//! Unix socket server for Protocol V2 execution messages using the new
//! generic relay engine architecture with security-focused features.
//!
//! ## Architecture Changes
//!
//! **Before**: Custom execution relay implementation with duplicated connection handling
//! **After**: Uses generic `Relay<ExecutionLogic>` engine with execution-specific logic
//!
//! ## Security Features
//! - **Message Validation**: Enhanced validation for execution messages
//! - **Audit Logging**: Comprehensive logging for compliance
//! - **Future Extensions**: Ready for authentication and authorization
//!
//! ## Performance Profile
//! - **Security First**: Validation may add minimal latency
//! - **Execution Integrity**: Correctness over pure speed
//! - **Compliance Ready**: Designed for regulatory requirements
//!
//! ## Usage
//! ```bash
//! # Start the execution relay
//! cargo run --release --bin execution_relay
//!
//! # Or using the relays package  
//! cargo run --release -p alphapulse-relays --bin execution_relay
//! ```

use alphapulse_relays::common::{Relay, RelayLogic};
use alphapulse_relays::execution::ExecutionLogic;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("🚀 Starting Generic Execution Relay");
    info!("📋 Using new generic engine architecture");
    info!("🔒 Security-focused execution message handling");

    // Create execution logic
    let logic = ExecutionLogic;
    info!(
        "✅ Execution Logic: domain={:?}, socket={}",
        logic.domain(),
        logic.socket_path()
    );

    // Create and start relay
    let mut relay = Relay::new(logic);

    match relay.run().await {
        Ok(()) => {
            info!("✅ Execution Relay completed successfully");
            Ok(())
        }
        Err(e) => {
            error!("❌ Execution Relay failed: {}", e);
            Err(e.into())
        }
    }
}

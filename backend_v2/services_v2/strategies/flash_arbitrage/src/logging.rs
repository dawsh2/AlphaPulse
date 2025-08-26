//! Standardized emoji logging for flash arbitrage modules
//!
//! Provides consistent emoji usage across all flash arbitrage components
//! to improve log readability and maintain professional standards.

/// Standard emoji set for flash arbitrage logging
pub struct LogEmoji;

impl LogEmoji {
    // Status indicators
    pub const SUCCESS: &'static str = "✅"; // Operation succeeded
    pub const ERROR: &'static str = "❌"; // Operation failed
    pub const WARNING: &'static str = "⚠️"; // Warning or caution
    pub const INFO: &'static str = "ℹ️"; // Information

    // Module-specific
    pub const SEARCH: &'static str = "🔍"; // Searching/detecting/analyzing
    pub const CHART: &'static str = "📊"; // Data/statistics/metrics
    pub const EXECUTE: &'static str = "⚡"; // Execution/action
    pub const MONEY: &'static str = "💰"; // Profit/financial
    pub const NETWORK: &'static str = "🌐"; // Network/connection
    pub const POOL: &'static str = "🏊"; // Pool events (swap/mint/burn/sync)
    pub const GAS: &'static str = "⛽"; // Gas price/costs
    pub const CLOCK: &'static str = "⏱️"; // Timing/latency

    // Event types
    pub const SWAP: &'static str = "🔄"; // Swap event
    pub const MINT: &'static str = "➕"; // Mint/liquidity add
    pub const BURN: &'static str = "➖"; // Burn/liquidity remove
    pub const SYNC: &'static str = "🔄"; // Sync event (same as swap)
}

// Convenience macros for standardized logging
#[macro_export]
macro_rules! log_success {
    ($($arg:tt)*) => {
        tracing::info!("{} {}", $crate::logging::LogEmoji::SUCCESS, format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        tracing::error!("{} {}", $crate::logging::LogEmoji::ERROR, format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_search {
    ($($arg:tt)*) => {
        tracing::info!("{} {}", $crate::logging::LogEmoji::SEARCH, format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_metrics {
    ($($arg:tt)*) => {
        tracing::info!("{} {}", $crate::logging::LogEmoji::CHART, format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_execution {
    ($($arg:tt)*) => {
        tracing::info!("{} {}", $crate::logging::LogEmoji::EXECUTE, format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_profit {
    ($($arg:tt)*) => {
        tracing::info!("{} {}", $crate::logging::LogEmoji::MONEY, format!($($arg)*))
    };
}

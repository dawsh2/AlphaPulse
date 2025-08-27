//! End-to-end test scenarios

// pub mod basic_connectivity; // Commented out - file not found
pub mod kraken_to_dashboard;
pub mod polygon_arbitrage;
// pub mod precision_validation; // Commented out - file not found
// pub mod latency_benchmark; // Commented out - file not found
// pub mod strategy_execution; // Commented out - file not found

// pub use basic_connectivity::BasicConnectivityTest; // Commented out
pub use kraken_to_dashboard::KrakenToDashboardTest;
pub use polygon_arbitrage::PolygonArbitrageTest;
// pub use precision_validation::PrecisionValidationTest; // Commented out
// pub use latency_benchmark::LatencyBenchmarkTest; // Commented out
// pub use strategy_execution::StrategyExecutionTest; // Commented out

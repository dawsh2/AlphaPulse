//! End-to-end test scenarios

pub mod basic_connectivity;
pub mod kraken_to_dashboard;
pub mod polygon_arbitrage;
pub mod precision_validation;
pub mod latency_benchmark;
pub mod strategy_execution;

pub use basic_connectivity::BasicConnectivityTest;
pub use kraken_to_dashboard::KrakenToDashboardTest;
pub use polygon_arbitrage::PolygonArbitrageTest;
pub use precision_validation::PrecisionValidationTest;
pub use latency_benchmark::LatencyBenchmarkTest;
pub use strategy_execution::StrategyExecutionTest;
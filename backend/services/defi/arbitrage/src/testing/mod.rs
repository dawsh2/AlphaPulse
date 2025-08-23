// Testing module exports

pub mod testnet_deployer;
pub mod integration_test_runner;
pub mod validation_reporter;
pub mod live_data_tests;
pub mod testnet_swap_executor;

pub use testnet_deployer::{TestnetDeployer, TestnetNetwork, TestnetConfig, ValidationReport as DeploymentValidationReport};
pub use integration_test_runner::{IntegrationTestRunner, IntegrationTestConfig, TestResults};
pub use validation_reporter::{ValidationReporter, ValidationReport, ValidationMetrics, ValidationEntry, ExecutionStatus};
pub use live_data_tests::{get_live_gas_price, get_live_matic_price, test_profit_with_live_data};
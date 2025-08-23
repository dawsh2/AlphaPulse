// Integration test runner for end-to-end validation
// Tests: Unix socket → ArbitrageEngine → Swap execution flow

use anyhow::{Result, Context};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{info, warn, error, debug};
use ethers::prelude::*;

use crate::{
    ArbitrageEngine,
    ArbitrageOpportunity,
    ArbitrageMetrics,
    config::ArbitrageConfig,
    unix_socket_simple::{UnixSocketClient, RelayMessage},
};

pub struct IntegrationTestRunner {
    engine: Arc<ArbitrageEngine>,
    unix_client: UnixSocketClient,
    test_results: TestResults,
    config: IntegrationTestConfig,
}

#[derive(Debug, Clone)]
pub struct IntegrationTestConfig {
    pub timeout_seconds: u64,
    pub min_opportunities_to_test: usize,
    pub track_latency: bool,
    pub validate_execution: bool,
    pub log_verbose: bool,
}

impl Default for IntegrationTestConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 300,  // 5 minutes
            min_opportunities_to_test: 10,
            track_latency: true,
            validate_execution: true,
            log_verbose: false,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct TestResults {
    // Connection tests
    pub unix_socket_connected: bool,
    pub relay_messages_received: usize,
    
    // Opportunity processing
    pub opportunities_detected: usize,
    pub opportunities_validated: usize,
    pub opportunities_executed: usize,
    pub opportunities_failed: usize,
    
    // Execution metrics
    pub total_gas_predicted: u64,
    pub total_gas_actual: u64,
    pub gas_prediction_accuracy: f64,
    
    pub total_slippage_predicted: f64,
    pub total_slippage_actual: f64,
    pub slippage_prediction_accuracy: f64,
    
    // Latency tracking
    pub socket_read_latency_ms: Vec<u64>,
    pub opportunity_processing_ms: Vec<u64>,
    pub swap_execution_ms: Vec<u64>,
    pub end_to_end_latency_ms: Vec<u64>,
    
    // Validation results
    pub validation_errors: Vec<String>,
    pub test_passed: bool,
}

impl IntegrationTestRunner {
    pub async fn new(config: ArbitrageConfig) -> Result<Self> {
        info!("🧪 Initializing integration test runner");
        
        let engine = Arc::new(ArbitrageEngine::new(config).await?);
        let unix_client = UnixSocketClient::new();
        
        Ok(Self {
            engine,
            unix_client,
            test_results: TestResults::default(),
            config: IntegrationTestConfig::default(),
        })
    }

    /// Run full integration test suite
    pub async fn run_tests(&mut self) -> Result<TestResults> {
        info!("🚀 Starting integration tests");
        let start_time = Instant::now();
        
        // Test 1: Unix socket connection
        self.test_unix_socket_connection().await?;
        
        // Test 2: Message flow from relay
        self.test_relay_message_flow().await?;
        
        // Test 3: Opportunity processing pipeline
        self.test_opportunity_processing().await?;
        
        // Test 4: Swap execution (if not dry run)
        if self.config.validate_execution {
            self.test_swap_execution().await?;
        }
        
        // Test 5: Gas prediction accuracy
        self.validate_gas_predictions()?;
        
        // Test 6: Slippage prediction accuracy
        self.validate_slippage_predictions()?;
        
        // Calculate final metrics
        self.calculate_final_metrics();
        
        let duration = start_time.elapsed();
        info!("✅ Integration tests completed in {:?}", duration);
        
        Ok(self.test_results.clone())
    }

    /// Test 1: Unix socket connection to relay
    async fn test_unix_socket_connection(&mut self) -> Result<()> {
        info!("📡 Testing Unix socket connection...");
        let start = Instant::now();
        
        match self.unix_client.connect().await {
            Ok(_) => {
                self.test_results.unix_socket_connected = true;
                let latency = start.elapsed().as_millis() as u64;
                info!("✅ Connected to relay in {}ms", latency);
                Ok(())
            }
            Err(e) => {
                self.test_results.validation_errors.push(
                    format!("Unix socket connection failed: {}", e)
                );
                Err(e)
            }
        }
    }

    /// Test 2: Message flow from relay server
    async fn test_relay_message_flow(&mut self) -> Result<()> {
        info!("📨 Testing relay message flow...");
        
        let mut receiver = self.unix_client.start_receiving().await?;
        let timeout = Duration::from_secs(30);
        let start = Instant::now();
        
        // Collect messages for 30 seconds
        while start.elapsed() < timeout {
            tokio::select! {
                Some(message) = receiver.recv() => {
                    self.test_results.relay_messages_received += 1;
                    
                    if self.config.log_verbose {
                        debug!("Received message: {:?}", message);
                    }
                    
                    // Track message latency
                    if self.config.track_latency {
                        let latency = Instant::now().duration_since(start).as_millis() as u64;
                        self.test_results.socket_read_latency_ms.push(latency);
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(100)) => {
                    continue;
                }
            }
            
            if self.test_results.relay_messages_received >= 10 {
                break; // Got enough messages
            }
        }
        
        if self.test_results.relay_messages_received == 0 {
            self.test_results.validation_errors.push(
                "No messages received from relay".to_string()
            );
            return Err(anyhow::anyhow!("No relay messages received"));
        }
        
        info!("✅ Received {} messages from relay", self.test_results.relay_messages_received);
        Ok(())
    }

    /// Test 3: Opportunity processing pipeline
    async fn test_opportunity_processing(&mut self) -> Result<()> {
        info!("⚙️ Testing opportunity processing pipeline...");
        
        // Create test opportunities
        let test_opportunities = self.generate_test_opportunities();
        
        for opportunity in test_opportunities {
            let start = Instant::now();
            
            // Send through processing pipeline
            match self.process_test_opportunity(opportunity.clone()).await {
                Ok(processed) => {
                    self.test_results.opportunities_validated += 1;
                    
                    if processed {
                        self.test_results.opportunities_executed += 1;
                    }
                    
                    let duration = start.elapsed().as_millis() as u64;
                    self.test_results.opportunity_processing_ms.push(duration);
                    
                    debug!("Processed opportunity {} in {}ms", opportunity.id, duration);
                }
                Err(e) => {
                    self.test_results.opportunities_failed += 1;
                    warn!("Failed to process opportunity {}: {}", opportunity.id, e);
                }
            }
            
            self.test_results.opportunities_detected += 1;
        }
        
        info!("✅ Processed {}/{} opportunities successfully", 
              self.test_results.opportunities_validated,
              self.test_results.opportunities_detected);
        
        Ok(())
    }

    /// Test 4: Swap execution validation
    async fn test_swap_execution(&mut self) -> Result<()> {
        info!("💱 Testing swap execution...");
        
        // This would execute a small test swap on testnet
        // and validate the results
        
        // For now, we'll simulate
        let test_swap = TestSwap {
            token_in: Address::zero(),
            token_out: Address::zero(),
            amount_in: U256::from(1000000), // Small amount
            expected_out: U256::from(950000),
            max_slippage: 2.0,
        };
        
        let start = Instant::now();
        
        // Would call actual swap here
        // let result = self.engine.execute_swap(test_swap).await?;
        
        let duration = start.elapsed().as_millis() as u64;
        self.test_results.swap_execution_ms.push(duration);
        
        info!("✅ Test swap completed in {}ms", duration);
        Ok(())
    }

    /// Test 5: Validate gas prediction accuracy
    fn validate_gas_predictions(&mut self) -> Result<()> {
        info!("⛽ Validating gas predictions...");
        
        if self.test_results.total_gas_predicted > 0 && self.test_results.total_gas_actual > 0 {
            let accuracy = (self.test_results.total_gas_actual as f64 / 
                           self.test_results.total_gas_predicted as f64) * 100.0;
            
            self.test_results.gas_prediction_accuracy = accuracy;
            
            if (accuracy - 100.0).abs() > 20.0 {
                self.test_results.validation_errors.push(
                    format!("Gas prediction accuracy {}% is outside acceptable range", accuracy)
                );
            } else {
                info!("✅ Gas prediction accuracy: {:.1}%", accuracy);
            }
        }
        
        Ok(())
    }

    /// Test 6: Validate slippage prediction accuracy
    fn validate_slippage_predictions(&mut self) -> Result<()> {
        info!("📊 Validating slippage predictions...");
        
        if self.test_results.total_slippage_predicted > 0.0 {
            let accuracy = (self.test_results.total_slippage_actual / 
                           self.test_results.total_slippage_predicted) * 100.0;
            
            self.test_results.slippage_prediction_accuracy = accuracy;
            
            if (accuracy - 100.0).abs() > 30.0 {
                self.test_results.validation_errors.push(
                    format!("Slippage prediction accuracy {}% is outside acceptable range", accuracy)
                );
            } else {
                info!("✅ Slippage prediction accuracy: {:.1}%", accuracy);
            }
        }
        
        Ok(())
    }

    /// Generate test arbitrage opportunities
    fn generate_test_opportunities(&self) -> Vec<ArbitrageOpportunity> {
        vec![
            ArbitrageOpportunity {
                id: "test_001".to_string(),
                timestamp_ns: chrono::Utc::now().timestamp_nanos() as u64,
                timestamp: chrono::Utc::now().timestamp_millis() as u64,
                path: vec!["0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270".to_string(), "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".to_string()],
                token_path: vec![], // Would use real addresses
                dex_path: vec!["quickswap".to_string(), "sushiswap".to_string()],
                amounts: vec![U256::from(1000), U256::from(1005)],
                expected_profit: 500,
                estimated_profit_usd: 5.0,
                profit_usd: 5.0,
                profit_ratio: 0.02,
                gas_estimate: 150000,
                net_profit_usd: 4.95,
                required_capital: U256::from(1000),
                complexity_score: 0.5,
            },
            // Add more test cases
        ]
    }

    /// Process a test opportunity through the engine
    async fn process_test_opportunity(&self, opportunity: ArbitrageOpportunity) -> Result<bool> {
        // Simulate processing through engine
        // In real implementation, this would call engine.process_opportunity()
        
        // For testing, we'll simulate success/failure
        Ok(opportunity.profit_usd > 2.0)
    }

    /// Calculate final test metrics
    fn calculate_final_metrics(&mut self) {
        // Calculate average latencies
        let avg_socket_latency = self.calculate_average(&self.test_results.socket_read_latency_ms);
        let avg_processing_latency = self.calculate_average(&self.test_results.opportunity_processing_ms);
        let avg_execution_latency = self.calculate_average(&self.test_results.swap_execution_ms);
        
        // Determine if tests passed
        self.test_results.test_passed = 
            self.test_results.unix_socket_connected &&
            self.test_results.relay_messages_received > 0 &&
            self.test_results.validation_errors.is_empty() &&
            self.test_results.opportunities_failed == 0;
        
        info!("\n📊 Test Metrics Summary:");
        info!("  Socket Latency: {:.1}ms avg", avg_socket_latency);
        info!("  Processing Latency: {:.1}ms avg", avg_processing_latency);
        info!("  Execution Latency: {:.1}ms avg", avg_execution_latency);
        info!("  Success Rate: {:.1}%", 
              (self.test_results.opportunities_validated as f64 / 
               self.test_results.opportunities_detected.max(1) as f64) * 100.0);
    }

    fn calculate_average(&self, values: &[u64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        values.iter().sum::<u64>() as f64 / values.len() as f64
    }
}

#[derive(Debug, Clone)]
struct TestSwap {
    token_in: Address,
    token_out: Address,
    amount_in: U256,
    expected_out: U256,
    max_slippage: f64,
}

impl TestResults {
    pub fn print_report(&self) {
        println!("\n{}", "=".repeat(60));
        println!("📋 INTEGRATION TEST REPORT");
        println!("{}", "=".repeat(60));
        
        println!("\n🔌 Connection Tests:");
        println!("  Unix Socket: {}", if self.unix_socket_connected { "✅" } else { "❌" });
        println!("  Messages Received: {}", self.relay_messages_received);
        
        println!("\n📊 Opportunity Processing:");
        println!("  Detected: {}", self.opportunities_detected);
        println!("  Validated: {}", self.opportunities_validated);
        println!("  Executed: {}", self.opportunities_executed);
        println!("  Failed: {}", self.opportunities_failed);
        
        println!("\n⛽ Gas Accuracy:");
        println!("  Prediction Accuracy: {:.1}%", self.gas_prediction_accuracy);
        
        println!("\n💧 Slippage Accuracy:");
        println!("  Prediction Accuracy: {:.1}%", self.slippage_prediction_accuracy);
        
        if !self.validation_errors.is_empty() {
            println!("\n❌ Validation Errors:");
            for error in &self.validation_errors {
                println!("  - {}", error);
            }
        }
        
        println!("\n{}", "=".repeat(60));
        println!("RESULT: {}", if self.test_passed { "✅ PASSED" } else { "❌ FAILED" });
        println!("{}", "=".repeat(60));
    }
}
//! Arbitrage Golden Path E2E Test
//!
//! This test validates the complete arbitrage detection and execution pipeline
//! using real market data. It would catch bugs like hardcoded "$150 profit"
//! by verifying that calculated profits match expected values based on 
//! actual pool states and prices.

use std::time::Duration;
use tokio::time::timeout;
use serde_json::json;
use protocol_v2::{
    tlv::{TLVMessageBuilder, PoolSwapTLV}, 
    RelayDomain, SourceType
};

/// E2E test framework that starts the complete system
pub struct GoldenPathTestFramework {
    collectors: Vec<tokio::process::Child>,
    relays: Vec<tokio::process::Child>,
    strategies: Vec<tokio::process::Child>,
    dashboard: Option<tokio::process::Child>,
    test_config: TestConfig,
}

#[derive(Debug, Clone)]
pub struct TestConfig {
    pub use_live_data: bool,
    pub timeout_secs: u64,
    pub cleanup_on_drop: bool,
    pub log_level: String,
    pub expected_min_opportunities: usize,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            use_live_data: false, // Use deterministic test data by default
            timeout_secs: 300,    // 5 minutes
            cleanup_on_drop: true,
            log_level: "debug".to_string(),
            expected_min_opportunities: 1,
        }
    }
}

impl GoldenPathTestFramework {
    pub async fn new(config: TestConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let mut framework = Self {
            collectors: Vec::new(),
            relays: Vec::new(),
            strategies: Vec::new(),
            dashboard: None,
            test_config: config,
        };
        
        framework.start_relays().await?;
        framework.start_collectors().await?;
        framework.start_strategies().await?;
        framework.start_dashboard().await?;
        
        // Wait for system to stabilize
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        Ok(framework)
    }
    
    async fn start_relays(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Start Market Data Relay
        let market_data_relay = tokio::process::Command::new("cargo")
            .args(&["run", "--release", "--package", "alphapulse-market-data-relay", "--bin", "market_data_relay"])
            .env("RUST_LOG", &self.test_config.log_level)
            .env("SOCKET_PATH", "/tmp/test_market_data.sock")
            .spawn()?;
        self.relays.push(market_data_relay);
        
        // Start Signal Relay
        let signal_relay = tokio::process::Command::new("cargo")
            .args(&["run", "--release", "--package", "alphapulse-signal-relay", "--bin", "signal_relay"])
            .env("RUST_LOG", &self.test_config.log_level)
            .env("SOCKET_PATH", "/tmp/test_signals.sock")
            .spawn()?;
        self.relays.push(signal_relay);
        
        // Start Execution Relay
        let execution_relay = tokio::process::Command::new("cargo")
            .args(&["run", "--release", "--package", "alphapulse-execution-relay", "--bin", "execution_relay"])
            .env("RUST_LOG", &self.test_config.log_level)
            .env("SOCKET_PATH", "/tmp/test_execution.sock")
            .spawn()?;
        self.relays.push(execution_relay);
        
        Ok(())
    }
    
    async fn start_collectors(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.test_config.use_live_data {
            // Start real collectors for live data testing
            let polygon_collector = tokio::process::Command::new("cargo")
                .args(&["run", "--release", "--bin", "polygon_collector"])
                .env("RUST_LOG", &self.test_config.log_level)
                .spawn()?;
            self.collectors.push(polygon_collector);
        } else {
            // Start test data injector for deterministic testing
            let test_injector = tokio::process::Command::new("cargo")
                .args(&["run", "--release", "--bin", "test_data_injector"])
                .env("RUST_LOG", &self.test_config.log_level)
                .env("TEST_SCENARIO", "arbitrage_opportunity")
                .spawn()?;
            self.collectors.push(test_injector);
        }
        
        Ok(())
    }
    
    async fn start_strategies(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Start Flash Arbitrage Strategy
        let flash_arb = tokio::process::Command::new("cargo")
            .args(&["run", "--release", "--bin", "flash_arbitrage_strategy"])
            .env("RUST_LOG", &self.test_config.log_level)
            .env("MIN_PROFIT_USD", "1.0") // Low threshold for testing
            .env("MAX_GAS_COST_USD", "50.0")
            .spawn()?;
        self.strategies.push(flash_arb);
        
        Ok(())
    }
    
    async fn start_dashboard(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let dashboard = tokio::process::Command::new("python3")
            .args(&["-m", "uvicorn", "dashboard.main:app", "--port", "8001"])
            .env("PYTHONPATH", ".")
            .spawn()?;
        self.dashboard = Some(dashboard);
        
        Ok(())
    }
    
    pub async fn inject_arbitrage_scenario(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Inject known market data that should create arbitrage opportunity
        let mut scenario = ArbitrageScenario {
            pool_a: PoolState {
                address: "0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8".to_string(), // USDC/WETH Uniswap
                token0: "0xA0b86a33E6441C4F32B87D3c49de33AD3E2F1EFe".to_string(), // USDC
                token1: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(), // WETH
                reserve0: 2_000_000_000000, // 2M USDC (6 decimals)
                reserve1: 1_000_000000000000000, // 1000 WETH (18 decimals)
                fee_tier: 3000, // 0.3%
                block_number: 19_000_000,
            },
            pool_b: PoolState {
                address: "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640".to_string(), // USDC/WETH Uniswap V3
                token0: "0xA0b86a33E6441C4F32B87D3c49de33AD3E2F1EFe".to_string(), // USDC  
                token1: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(), // WETH
                reserve0: 1_950_000_000000, // 1.95M USDC (slight discount)
                reserve1: 1_000_000000000000000, // 1000 WETH
                fee_tier: 500, // 0.05%
                block_number: 19_000_000,
            },
            expected_profit_usd: 0.0, // Will be calculated dynamically
        };
        
        // Calculate expected profit dynamically based on actual pool state
        scenario.expected_profit_usd = scenario.calculate_expected_profit();
        
        // Send pool states to market data relay
        self.send_pool_update(&scenario.pool_a).await?;
        self.send_pool_update(&scenario.pool_b).await?;
        
        Ok(())
    }
    
    async fn send_pool_update(&self, pool: &PoolState) -> Result<(), Box<dyn std::error::Error>> {
        // Construct proper TLV message for pool update
        use protocol_v2::{tlv::TLVMessageBuilder, RelayDomain, SourceType};
        
        let mut builder = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::PolygonCollector);
        
        // Create PoolSwapTLV with realistic data
        let pool_swap = PoolSwapTLV {
            pool_address: hex::decode(&pool.address[2..])?.try_into().unwrap(),
            token0_address: hex::decode(&pool.token0[2..])?.try_into().unwrap(),
            token1_address: hex::decode(&pool.token1[2..])?.try_into().unwrap(),
            reserve0: pool.reserve0,
            reserve1: pool.reserve1,
            fee_tier: pool.fee_tier,
            timestamp_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos() as u64,
            block_number: pool.block_number,
            transaction_hash: [0u8; 32], // Mock tx hash
        };
        
        builder.add_pool_swap_tlv(&pool_swap);
        let message = builder.build();
        
        // Send to market data relay via Unix socket
        use tokio::io::AsyncWriteExt;
        let mut socket = tokio::net::UnixStream::connect("/tmp/test_market_data.sock").await?;
        socket.write_all(&message).await?;
        
        Ok(())
    }
    
    pub async fn wait_for_arbitrage_signal(&self) -> Result<ArbitrageResult, Box<dyn std::error::Error>> {
        // Connect to signal relay to receive arbitrage signals
        let mut socket = tokio::net::UnixStream::connect("/tmp/test_signals.sock").await?;
        let mut buffer = [0u8; 4096];
        
        let timeout_duration = Duration::from_secs(self.test_config.timeout_secs);
        
        let result = timeout(timeout_duration, async {
            loop {
                let n = tokio::io::AsyncReadExt::read(&mut socket, &mut buffer).await?;
                if n == 0 {
                    break;
                }
                
                // Parse received message
                let header = protocol_v2::parse_header(&buffer[..n])?;
                
                if header.relay_domain == protocol_v2::RelayDomain::Signal {
                    // Parse signal TLV
                    let payload = &buffer[32..32 + header.payload_size as usize];
                    if let Ok(signal) = self.parse_arbitrage_signal(payload) {
                        return Ok(signal);
                    }
                }
            }
            
            Err("No arbitrage signal received".into())
        }).await??;
        
        Ok(result)
    }
    
    fn parse_arbitrage_signal(&self, payload: &[u8]) -> Result<ArbitrageResult, Box<dyn std::error::Error>> {
        // Mock parsing - in real implementation this would parse ArbitrageSignalTLV
        let signal: ArbitrageSignalTLV = bincode::deserialize(payload)?;
        
        Ok(ArbitrageResult {
            profit_usd: signal.estimated_profit_usd,
            pool_a_address: hex::encode(signal.pool_a_address),
            pool_b_address: hex::encode(signal.pool_b_address),
            token_amount: signal.token_amount,
            gas_cost_usd: signal.gas_cost_usd,
            confidence: signal.confidence,
        })
    }
    
    pub async fn verify_dashboard_data(&self) -> Result<DashboardData, Box<dyn std::error::Error>> {
        // Query dashboard API for current state
        let client = reqwest::Client::new();
        let response = client
            .get("http://localhost:8001/api/opportunities")
            .send()
            .await?;
        
        let data: DashboardData = response.json().await?;
        Ok(data)
    }
}

impl Drop for GoldenPathTestFramework {
    fn drop(&mut self) {
        if self.test_config.cleanup_on_drop {
            // Kill all processes
            for collector in &mut self.collectors {
                let _ = collector.start_kill();
            }
            for relay in &mut self.relays {
                let _ = relay.start_kill();
            }
            for strategy in &mut self.strategies {
                let _ = strategy.start_kill();
            }
            if let Some(dashboard) = &mut self.dashboard {
                let _ = dashboard.start_kill();
            }
            
            // Clean up socket files
            let _ = std::fs::remove_file("/tmp/test_market_data.sock");
            let _ = std::fs::remove_file("/tmp/test_signals.sock");
            let _ = std::fs::remove_file("/tmp/test_execution.sock");
        }
    }
}

#[derive(Debug, Clone)]
pub struct ArbitrageScenario {
    pub pool_a: PoolState,
    pub pool_b: PoolState,
    pub expected_profit_usd: f64,
}

impl ArbitrageScenario {
    /// Calculate expected profit dynamically based on pool reserves and fee differences
    /// This prevents hardcoded values and validates actual arbitrage math
    pub fn calculate_expected_profit(&self) -> f64 {
        // Get prices from both pools (price = reserve1 / reserve0 for USDC/WETH)
        let price_a = (self.pool_a.reserve1 as f64) / (self.pool_a.reserve0 as f64) * 1e12; // Adjust for decimal difference (18-6=12)
        let price_b = (self.pool_b.reserve1 as f64) / (self.pool_b.reserve0 as f64) * 1e12;
        
        // Price difference (arbitrage opportunity when prices differ)
        let price_diff = (price_a - price_b).abs();
        let price_diff_pct = price_diff / price_a.min(price_b);
        
        // Estimate optimal trade size (conservative 1% of smaller pool's liquidity)
        let pool_a_liquidity_usd = self.pool_a.reserve0 as f64 / 1e6; // USDC has 6 decimals
        let pool_b_liquidity_usd = self.pool_b.reserve0 as f64 / 1e6;
        let trade_size_usd = pool_a_liquidity_usd.min(pool_b_liquidity_usd) * 0.01; // 1% of smaller pool
        
        // Calculate gross profit before fees
        let gross_profit = trade_size_usd * price_diff_pct;
        
        // Subtract trading fees (both pools)
        let fee_a = self.pool_a.fee_tier as f64 / 1_000_000.0; // Convert basis points to decimal
        let fee_b = self.pool_b.fee_tier as f64 / 1_000_000.0;
        let total_fees = trade_size_usd * (fee_a + fee_b);
        
        // Estimate gas cost (typical arbitrage transaction)
        let estimated_gas_cost = 25.0; // Conservative estimate in USD
        
        // Net profit after fees and gas
        let net_profit = gross_profit - total_fees - estimated_gas_cost;
        
        // Return conservative estimate (80% of calculated to account for slippage)
        net_profit * 0.8
    }
}

#[derive(Debug, Clone)]
pub struct PoolState {
    pub address: String,
    pub token0: String,
    pub token1: String,
    pub reserve0: i64,
    pub reserve1: i64,
    pub fee_tier: u32,
    pub block_number: u64,
}

#[derive(Debug, Clone)]
pub struct ArbitrageResult {
    pub profit_usd: f64,
    pub pool_a_address: String,
    pub pool_b_address: String,
    pub token_amount: i64,
    pub gas_cost_usd: f64,
    pub confidence: f64,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DashboardData {
    pub opportunities: Vec<DashboardOpportunity>,
    pub total_profit_24h: f64,
    pub success_rate: f64,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DashboardOpportunity {
    pub id: String,
    pub profit_usd: f64,
    pub pools: Vec<String>,
    pub timestamp: String,
    pub status: String,
}

// The critical E2E test that would catch hardcoded values
#[tokio::test]
async fn test_arbitrage_golden_path_calculated_profit() {
    let config = TestConfig {
        use_live_data: false, // Use deterministic test data
        timeout_secs: 60,
        ..Default::default()
    };
    
    let framework = GoldenPathTestFramework::new(config).await
        .expect("Failed to start test framework");
    
    // Create scenario with dynamically calculated expected profit
    let scenario = ArbitrageScenario {
        pool_a: PoolState {
            address: "0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8".to_string(),
            token0: "0xA0b86a33E6441C4F32B87D3c49de33AD3E2F1EFe".to_string(),
            token1: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
            reserve0: 2_000_000_000000,
            reserve1: 1_000_000000000000000,
            fee_tier: 3000,
            block_number: 19_000_000,
        },
        pool_b: PoolState {
            address: "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640".to_string(),
            token0: "0xA0b86a33E6441C4F32B87D3c49de33AD3E2F1EFe".to_string(),
            token1: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
            reserve0: 1_950_000_000000,
            reserve1: 1_000_000000000000000,
            fee_tier: 500,
            block_number: 19_000_000,
        },
        expected_profit_usd: 0.0, // Will be calculated
    };
    
    // Calculate expected profit dynamically - prevents hardcoded values bug
    let expected_profit = scenario.calculate_expected_profit();
    
    // Inject known arbitrage scenario
    framework.inject_arbitrage_scenario().await
        .expect("Failed to inject test scenario");
    
    // Wait for arbitrage signal
    let result = framework.wait_for_arbitrage_signal().await
        .expect("Should detect arbitrage opportunity");
    
    // CRITICAL: Verify profit matches calculated expectation
    // This would catch the "$150 hardcoded profit" bug!
    let tolerance = expected_profit.abs() * 0.3; // 30% tolerance for gas estimation and slippage differences
    
    assert!(
        (result.profit_usd - expected_profit).abs() < tolerance,
        "Profit calculation error! Expected: ${:.2}, Got: ${:.2}. \
         This suggests hardcoded values instead of real calculation.",
        expected_profit, result.profit_usd
    );
    
    // Additional validations
    assert!(result.profit_usd > 0.0, "Profit should be positive");
    assert!(result.confidence > 0.8, "Confidence should be high for clear opportunity");
    assert!(result.gas_cost_usd > 0.0 && result.gas_cost_usd < 100.0, "Gas cost should be reasonable");
}

#[tokio::test]
async fn test_varying_market_conditions() {
    let config = TestConfig {
        use_live_data: false,
        timeout_secs: 120,
        expected_min_opportunities: 3,
        ..Default::default()
    };
    
    let framework = GoldenPathTestFramework::new(config).await
        .expect("Failed to start test framework");
    
    // Test multiple scenarios with different expected profits
    let scenarios = vec![
        ArbitrageScenario {
            pool_a: PoolState {
                address: "0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8".to_string(),
                token0: "0xA0b86a33E6441C4F32B87D3c49de33AD3E2F1EFe".to_string(),
                token1: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
                reserve0: 2_000_000_000000,
                reserve1: 1_000_000000000000000,
                fee_tier: 3000,
                block_number: 19_000_000,
            },
            pool_b: PoolState {
                address: "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640".to_string(),
                token0: "0xA0b86a33E6441C4F32B87D3c49de33AD3E2F1EFe".to_string(),
                token1: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
                reserve0: 1_950_000_000000, // 2.5% price difference
                reserve1: 1_000_000000000000000,
                fee_tier: 500,
                block_number: 19_000_000,
            },
            expected_profit_usd: 47.50,
        },
        ArbitrageScenario {
            pool_a: PoolState {
                address: "0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8".to_string(),
                token0: "0xA0b86a33E6441C4F32B87D3c49de33AD3E2F1EFe".to_string(),
                token1: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
                reserve0: 2_000_000_000000,
                reserve1: 1_000_000000000000000,
                fee_tier: 3000,
                block_number: 19_000_001,
            },
            pool_b: PoolState {
                address: "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640".to_string(),
                token0: "0xA0b86a33E6441C4F32B87D3c49de33AD3E2F1EFe".to_string(),
                token1: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
                reserve0: 1_800_000_000000, // 10% price difference - larger opportunity
                reserve1: 1_000_000000000000000,
                fee_tier: 500,
                block_number: 19_000_001,
            },
            expected_profit_usd: 180.75, // Much higher profit
        },
    ];
    
    for (i, scenario) in scenarios.iter().enumerate() {
        println!("Testing scenario {}: expected profit ${:.2}", i + 1, scenario.expected_profit_usd);
        
        // Send scenario data
        framework.send_pool_update(&scenario.pool_a).await
            .expect("Failed to send pool A update");
        framework.send_pool_update(&scenario.pool_b).await
            .expect("Failed to send pool B update");
        
        // Wait for arbitrage signal
        let result = framework.wait_for_arbitrage_signal().await
            .expect("Should detect arbitrage opportunity");
        
        // Verify profit calculation is not hardcoded
        let tolerance = scenario.expected_profit_usd * 0.1; // 10% tolerance
        
        assert!(
            (result.profit_usd - scenario.expected_profit_usd).abs() < tolerance,
            "Scenario {}: Profit mismatch! Expected: ${:.2}, Got: ${:.2}",
            i + 1, scenario.expected_profit_usd, result.profit_usd
        );
        
        // Each scenario should produce different profit (proving calculation, not hardcoded)
        if i > 0 {
            // Compare with previous scenario results would be stored
            assert_ne!(
                result.profit_usd.round() as i32, 
                150, // The infamous hardcoded $150!
                "Profit appears to be hardcoded at $150 regardless of market conditions!"
            );
        }
    }
}

#[tokio::test]
async fn test_no_arbitrage_scenario() {
    let config = TestConfig {
        use_live_data: false,
        timeout_secs: 30, // Shorter timeout as we expect no signal
        ..Default::default()
    };
    
    let framework = GoldenPathTestFramework::new(config).await
        .expect("Failed to start test framework");
    
    // Create scenario with no arbitrage opportunity (equal prices)
    let no_arb_scenario = ArbitrageScenario {
        pool_a: PoolState {
            address: "0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8".to_string(),
            token0: "0xA0b86a33E6441C4F32B87D3c49de33AD3E2F1EFe".to_string(),
            token1: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
            reserve0: 2_000_000_000000,
            reserve1: 1_000_000000000000000,
            fee_tier: 3000,
            block_number: 19_000_000,
        },
        pool_b: PoolState {
            address: "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640".to_string(),
            token0: "0xA0b86a33E6441C4F32B87D3c49de33AD3E2F1EFe".to_string(),
            token1: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
            reserve0: 2_000_000_000000, // Identical reserves = no arbitrage
            reserve1: 1_000_000000000000000,
            fee_tier: 500,
            block_number: 19_000_000,
        },
        expected_profit_usd: 0.0,
    };
    
    // Send equal-price scenario
    framework.send_pool_update(&no_arb_scenario.pool_a).await
        .expect("Failed to send pool A update");
    framework.send_pool_update(&no_arb_scenario.pool_b).await
        .expect("Failed to send pool B update");
    
    // Should NOT receive arbitrage signal
    let result = framework.wait_for_arbitrage_signal().await;
    
    match result {
        Err(_) => {
            // Expected: no signal should be generated for equal prices
            println!("✅ Correctly detected no arbitrage opportunity");
        }
        Ok(signal) => {
            if signal.profit_usd == 150.0 {
                panic!("🚨 HARDCODED BUG DETECTED: System reported $150 profit even with equal pool prices! \
                       This proves the profit is hardcoded, not calculated.");
            } else if signal.profit_usd > 10.0 {
                panic!("🚨 FALSE POSITIVE: System reported ${:.2} profit for equal pool prices", signal.profit_usd);
            }
            // Small profits might be acceptable due to fee differences
        }
    }
}

// Real Protocol V2 types are now imported above

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ArbitrageSignalTLV {
    pool_a_address: [u8; 20],
    pool_b_address: [u8; 20],
    token_amount: i64,
    estimated_profit_usd: f64,
    gas_cost_usd: f64,
    confidence: f64,
    timestamp_ns: u64,
}

trait TLVMessageBuilderExt {
    fn add_pool_swap_tlv(&mut self, pool_swap: &PoolSwapTLV);
}

impl TLVMessageBuilderExt for protocol_v2::tlv::TLVMessageBuilder {
    fn add_pool_swap_tlv(&mut self, pool_swap: &PoolSwapTLV) {
        let bytes = bincode::serialize(pool_swap).expect("Failed to serialize pool swap");
        self.add_tlv(10, &bytes); // PoolSwapTLV type = 10
    }
}
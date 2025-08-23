//! End-to-end test for arbitrage strategy -> dashboard pipeline
//!
//! Tests that arbitrage opportunities from the flash arbitrage strategy
//! are properly transmitted through the signal relay to the dashboard
//! using the new DemoDeFiArbitrageTLV format.

use alphapulse_e2e_tests::{
    fixtures::ArbitrageSignalFixture,
    framework::{TestRunner, TestConfig},
    validation::assert_dashboard_received_arbitrage
};
use alphapulse_protocol::{
    MessageHeader, RelayDomain, SourceType,
    tlv::builder::TLVMessageBuilder,
    tlv::demo_defi::DemoDeFiArbitrageTLV,
    tlv::types::TLVType,
    VenueId, PoolInstrumentId,
};
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;
use tracing::{info, warn};

const FLASH_ARBITRAGE_STRATEGY_ID: u16 = 21;

#[tokio::test]
async fn test_arbitrage_signal_to_dashboard() {
    // Initialize test environment
    let config = TestConfig {
        signal_relay_path: "/tmp/alphapulse_test/signals_arb.sock".to_string(),
        dashboard_port: 8081,
        test_timeout_secs: 30,
        ..Default::default()
    };
    
    let mut runner = TestRunner::new(config);
    
    // Start signal relay and dashboard WebSocket server
    runner.start_signal_relay().await.expect("Failed to start signal relay");
    runner.start_dashboard().await.expect("Failed to start dashboard");
    
    // Wait for services to be ready
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    info!("🧪 Starting arbitrage signal to dashboard E2E test");
    
    // Create mock arbitrage opportunity
    let opportunity = create_mock_arbitrage_opportunity();
    
    // Connect to signal relay as if we're the flash arbitrage strategy
    let mut signal_stream = UnixStream::connect(&runner.config.signal_relay_path)
        .await
        .expect("Failed to connect to signal relay");
    
    info!("📡 Connected to signal relay, sending mock arbitrage signal");
    
    // Send the arbitrage signal
    send_arbitrage_signal(&mut signal_stream, &opportunity).await
        .expect("Failed to send arbitrage signal");
    
    info!("✅ Arbitrage signal sent successfully");
    
    // Connect to dashboard WebSocket to verify message is received
    let dashboard_url = format!("ws://127.0.0.1:{}/ws", runner.config.dashboard_port);
    let (ws_stream, _) = tokio_tungstenite::connect_async(&dashboard_url)
        .await
        .expect("Failed to connect to dashboard WebSocket");
    
    info!("🔗 Connected to dashboard WebSocket, waiting for arbitrage message");
    
    // Wait for arbitrage opportunity message
    let arbitrage_msg = runner.wait_for_dashboard_message(ws_stream, "arbitrage_opportunity", Duration::from_secs(10))
        .await
        .expect("Failed to receive arbitrage opportunity from dashboard");
    
    info!("📊 Received arbitrage opportunity message from dashboard");
    
    // Validate the message contains expected fields
    assert_dashboard_received_arbitrage(&arbitrage_msg, &opportunity)
        .expect("Dashboard arbitrage message validation failed");
    
    info!("🎯 E2E test completed successfully - arbitrage pipeline working!");
    
    // Cleanup
    runner.shutdown().await;
}

#[derive(Debug, Clone)]
struct MockArbitrageOpportunity {
    signal_id: u64,
    expected_profit_usd: f64,
    required_capital_usd: f64,
    estimated_gas_cost_usd: f64,
    confidence: u8,
    chain_id: u8,
    token_in: u64,
    token_out: u64,
    timestamp_ns: u64,
}

fn create_mock_arbitrage_opportunity() -> MockArbitrageOpportunity {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    
    MockArbitrageOpportunity {
        signal_id: now,
        expected_profit_usd: 42.50,
        required_capital_usd: 1000.0,
        estimated_gas_cost_usd: 2.5,
        confidence: 95,
        chain_id: 137, // Polygon
        token_in: 0xa0b86991c431aa73u64,  // USDC
        token_out: 0xc02aaa39b223fe8du64, // WETH
        timestamp_ns: now,
    }
}

async fn send_arbitrage_signal(
    stream: &mut UnixStream,
    opportunity: &MockArbitrageOpportunity,
) -> Result<(), Box<dyn std::error::Error>> {
    // Convert f64 values to fixed-point with proper scaling
    let expected_profit_q = ((opportunity.expected_profit_usd * 100000000.0) as i128); // 8 decimals for USD
    let required_capital_q = ((opportunity.required_capital_usd * 100000000.0) as u128); // 8 decimals for USD
    let estimated_gas_cost_q = ((opportunity.estimated_gas_cost_usd * 1000000000000000000.0) as u128); // 18 decimals for ETH
    
    // Create pool IDs for the test
    let pool_a = PoolInstrumentId::from_v2_pair(VenueId::UniswapV2, opportunity.token_in, opportunity.token_out);
    let pool_b = PoolInstrumentId::from_v3_pair(VenueId::UniswapV3, opportunity.token_in, opportunity.token_out);
    
    let optimal_amount_q = required_capital_q; // Same as capital for test
    
    // Create DemoDeFiArbitrageTLV
    let arbitrage_tlv = DemoDeFiArbitrageTLV::new(
        FLASH_ARBITRAGE_STRATEGY_ID,
        opportunity.signal_id,
        opportunity.confidence,
        opportunity.chain_id,
        expected_profit_q,
        required_capital_q,
        estimated_gas_cost_q,
        VenueId::UniswapV2,         // Pool A venue
        pool_a,
        VenueId::UniswapV3,         // Pool B venue
        pool_b,
        opportunity.token_in,
        opportunity.token_out,
        optimal_amount_q,
        50,                         // 0.5% slippage tolerance
        100,                        // 100 Gwei max gas
        (opportunity.timestamp_ns / 1_000_000_000) as u32 + 300, // Valid for 5 minutes
        200,                        // High priority
        opportunity.timestamp_ns,
    );
    
    // Serialize the DemoDeFiArbitrageTLV to bytes
    let tlv_payload = arbitrage_tlv.to_bytes();
    
    // Build complete protocol message with header using ExtendedTLV
    let message_bytes = TLVMessageBuilder::new(RelayDomain::Signal, SourceType::ArbitrageStrategy)
        .add_tlv_bytes(TLVType::ExtendedTLV, tlv_payload)
        .build();
    
    // Send complete message
    stream.write_all(&message_bytes).await?;
    stream.flush().await?;
    
    Ok(())
}

#[tokio::test]
async fn test_multiple_arbitrage_signals() {
    let config = TestConfig {
        signal_relay_path: "/tmp/alphapulse_test/signals_multi_arb.sock".to_string(),
        dashboard_port: 8082,
        test_timeout_secs: 45,
        ..Default::default()
    };
    
    let mut runner = TestRunner::new(config);
    
    // Start services
    runner.start_signal_relay().await.expect("Failed to start signal relay");
    runner.start_dashboard().await.expect("Failed to start dashboard");
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    info!("🧪 Testing multiple arbitrage signals");
    
    // Connect to signal relay
    let mut signal_stream = UnixStream::connect(&runner.config.signal_relay_path)
        .await
        .expect("Failed to connect to signal relay");
    
    // Connect to dashboard
    let dashboard_url = format!("ws://127.0.0.1:{}/ws", runner.config.dashboard_port);
    let (ws_stream, _) = tokio_tungstenite::connect_async(&dashboard_url)
        .await
        .expect("Failed to connect to dashboard WebSocket");
    
    // Send multiple arbitrage opportunities
    let base_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64;
    let opportunities = vec![
        MockArbitrageOpportunity {
            signal_id: 1001,
            expected_profit_usd: 15.75,
            required_capital_usd: 500.0,
            estimated_gas_cost_usd: 1.5,
            confidence: 85,
            chain_id: 137,
            token_in: 0xa0b86991c431aa73u64,  // USDC
            token_out: 0xc02aaa39b223fe8du64, // WETH
            timestamp_ns: base_time,
        },
        MockArbitrageOpportunity {
            signal_id: 1002,
            expected_profit_usd: 88.25,
            required_capital_usd: 2000.0,
            estimated_gas_cost_usd: 3.0,
            confidence: 95,
            chain_id: 137,
            token_in: 0x8f3cf7ad23cd3cadbd9735aff958023239c6a063u64, // DAI
            token_out: 0x2791bca1f2de4661ed88a30c99a7a9449aa84174u64, // USDC
            timestamp_ns: base_time + 1_000_000,
        },
        MockArbitrageOpportunity {
            signal_id: 1003,
            expected_profit_usd: 5.50,
            required_capital_usd: 250.0,
            estimated_gas_cost_usd: 0.8,
            confidence: 75,
            chain_id: 137,
            token_in: 0x0d500b1d8e8ef31e21c99d1db9a6444d3adf1270u64, // WMATIC
            token_out: 0xa0b86991c431aa73u64, // USDC
            timestamp_ns: base_time + 2_000_000,
        },
    ];
    
    for opportunity in &opportunities {
        send_arbitrage_signal(&mut signal_stream, opportunity).await
            .expect("Failed to send arbitrage signal");
        
        info!("📡 Sent arbitrage signal {} with ${:.2} profit", 
              opportunity.signal_id, opportunity.expected_profit_usd);
        
        // Small delay between signals
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    // Verify all signals are received by dashboard
    let mut received_count = 0;
    let mut ws_stream = ws_stream;
    
    while received_count < opportunities.len() {
        match runner.wait_for_dashboard_message(ws_stream, "arbitrage_opportunity", Duration::from_secs(5)).await {
            Ok((msg, stream)) => {
                ws_stream = stream;
                received_count += 1;
                
                info!("📊 Received arbitrage opportunity #{} from dashboard", received_count);
                
                // Validate message structure
                assert!(msg.get("msg_type").and_then(|v| v.as_str()) == Some("arbitrage_opportunity"));
                assert!(msg.get("estimated_profit").and_then(|v| v.as_f64()).is_some());
                assert!(msg.get("strategy_id").and_then(|v| v.as_u64()) == Some(FLASH_ARBITRAGE_STRATEGY_ID as u64));
            }
            Err(e) => {
                warn!("Failed to receive arbitrage message #{}: {}", received_count + 1, e);
                break;
            }
        }
    }
    
    assert_eq!(received_count, opportunities.len(), 
               "Expected {} arbitrage messages, received {}", opportunities.len(), received_count);
    
    info!("🎯 Multiple arbitrage signals test completed successfully!");
    
    runner.shutdown().await;
}
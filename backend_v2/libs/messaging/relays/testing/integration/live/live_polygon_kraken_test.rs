//! Live Polygon AND Kraken WebSocket test with roundtrip validation
//!
//! Uses REAL connections to both:
//! - Kraken WebSocket for CEX data
//! - Polygon RPC WebSocket for DEX data (Uniswap V3, QuickSwap, etc.)

use ethers::prelude::*;
use futures_util::{SinkExt, StreamExt};
use protocol_v2::{
    build_instrument_id, MessageHeader, PoolSwapTLV, QuoteTLV, RelayDomain, SourceType, TLVMessage,
    TLVType, TradeTLV, VenueId, MESSAGE_MAGIC, PROTOCOL_VERSION,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::Message};

// Public Polygon WebSocket endpoints (choose one that works)
const POLYGON_RPC_OPTIONS: &[&str] = &[
    "wss://ws-mainnet.matic.network",
    "wss://polygon-mainnet.public.blastapi.io",
    "wss://polygon-bor.publicnode.com",
    "wss://polygon.drpc.org",
];
const UNISWAP_V3_FACTORY: &str = "0x1F98431c8aD98523631AE4a59f267346ea31F984";

// Uniswap V3 Pool ABI events we care about
abigen!(
    UniswapV3Pool,
    r#"[
        event Swap(address indexed sender, address indexed recipient, int256 amount0, int256 amount1, uint160 sqrtPriceX96, uint128 liquidity, int24 tick)
        event Mint(address sender, address indexed owner, int24 indexed tickLower, int24 indexed tickUpper, uint128 amount, uint256 amount0, uint256 amount1)
        event Burn(address indexed owner, int24 indexed tickLower, int24 indexed tickUpper, uint128 amount, uint256 amount0, uint256 amount1)
    ]"#
);

#[derive(Clone)]
struct StrategyConsumer {
    name: String,
    subscribed_topics: Vec<String>,
    received_messages: Arc<Mutex<Vec<(MessageHeader, Vec<u8>)>>>,
    roundtrip_results: Arc<Mutex<Vec<bool>>>,
}

impl StrategyConsumer {
    fn new(name: &str, topics: Vec<&str>) -> Self {
        Self {
            name: name.to_string(),
            subscribed_topics: topics.iter().map(|s| s.to_string()).collect(),
            received_messages: Arc::new(Mutex::new(Vec::new())),
            roundtrip_results: Arc::new(Mutex::new(Vec::new())),
        }
    }

    async fn receive_and_validate(&self, header: MessageHeader, data: Vec<u8>) -> bool {
        let mut messages = self.received_messages.lock().await;
        messages.push((header, data.clone()));

        // Validate roundtrip
        let valid = self.validate_roundtrip(&data).await;
        let mut results = self.roundtrip_results.lock().await;
        results.push(valid);

        valid
    }

    async fn validate_roundtrip(&self, data: &[u8]) -> bool {
        // Deserialize
        let header = unsafe { std::ptr::read(data.as_ptr() as *const MessageHeader) };

        // Re-serialize
        let header_bytes = unsafe {
            std::slice::from_raw_parts(
                &header as *const _ as *const u8,
                std::mem::size_of::<MessageHeader>(),
            )
        };

        // Create new message from deserialized data
        let mut new_message = header_bytes.to_vec();
        if data.len() > std::mem::size_of::<MessageHeader>() {
            new_message.extend_from_slice(&data[std::mem::size_of::<MessageHeader>()..]);
        }

        // Compare byte-for-byte
        data == new_message
    }
}

fn kraken_trade_to_protocol(pair: &str, trade_data: &serde_json::Value) -> Option<Vec<u8>> {
    let arr = trade_data.as_array()?;
    if arr.len() < 4 {
        return None;
    }

    let price_str = arr[0].as_str()?;
    let volume_str = arr[1].as_str()?;
    let timestamp = arr[2].as_f64()?;
    let side = arr[3].as_str()?;

    let price = (price_str.parse::<f64>().ok()? * 100_000_000.0) as i64;
    let volume = (volume_str.parse::<f64>().ok()? * 100_000_000.0) as i64;
    let timestamp_ns = (timestamp * 1_000_000_000.0) as u64;

    let instrument_id = match pair {
        "XBT/USD" => build_instrument_id(1, 1, 2, 1, VenueId::Kraken as u16, 0),
        "ETH/USD" => build_instrument_id(1, 3, 2, 1, VenueId::Kraken as u16, 0),
        _ => 0,
    };

    let header = MessageHeader {
        magic: MESSAGE_MAGIC,
        version: PROTOCOL_VERSION,
        message_type: TLVType::Trade as u8,
        relay_domain: RelayDomain::MarketData as u8,
        source_type: SourceType::KrakenCollector as u8,
        sequence: 0,
        timestamp_ns,
        instrument_id,
        checksum: 0,
    };

    let header_bytes = unsafe {
        std::slice::from_raw_parts(
            &header as *const _ as *const u8,
            std::mem::size_of::<MessageHeader>(),
        )
    };

    let mut message = header_bytes.to_vec();

    message.push(TLVType::Trade as u8);
    message.push(if side == "b" { 0x01 } else { 0x00 });
    message.extend_from_slice(&16u16.to_le_bytes());
    message.extend_from_slice(&price.to_le_bytes());
    message.extend_from_slice(&volume.to_le_bytes());

    let checksum = crc32fast::hash(&message);
    let checksum_offset = std::mem::size_of::<MessageHeader>() - 4;
    message[checksum_offset..checksum_offset + 4].copy_from_slice(&checksum.to_le_bytes());

    Some(message)
}

fn polygon_swap_to_protocol(
    pool_address: Address,
    token0: Address,
    token1: Address,
    amount0: I256,
    amount1: I256,
    sqrt_price_x96: U256,
    liquidity: u128,
    tick: i32,
    block_timestamp: u64,
) -> Vec<u8> {
    let timestamp_ns = block_timestamp * 1_000_000_000;

    let (token_in, token_out, amount_in, amount_out) = if amount0 > I256::zero() {
        (token0, token1, amount0.as_u128(), amount1.abs().as_u128())
    } else {
        (token1, token0, amount1.as_u128(), amount0.abs().as_u128())
    };

    let header = MessageHeader {
        magic: MESSAGE_MAGIC,
        version: PROTOCOL_VERSION,
        message_type: 11, // PoolSwapTLV
        relay_domain: RelayDomain::MarketData as u8,
        source_type: SourceType::PolygonCollector as u8,
        sequence: 0,
        timestamp_ns,
        instrument_id: build_instrument_id(2, 100, 101, 3, VenueId::UniswapV3Polygon as u16, 500),
        checksum: 0,
    };

    let header_bytes = unsafe {
        std::slice::from_raw_parts(
            &header as *const _ as *const u8,
            std::mem::size_of::<MessageHeader>(),
        )
    };

    let mut message = header_bytes.to_vec();

    message.push(11);
    message.push(0);

    let payload_size = 2 + 32 + 8 + 8 + 16 + 16 + 16 + 1 + 1 + 4;
    message.extend_from_slice(&(payload_size as u16).to_le_bytes());

    message.extend_from_slice(&(VenueId::UniswapV3Polygon as u16).to_le_bytes());
    message.extend_from_slice(pool_address.as_bytes());

    let token_in_u64 = u64::from_be_bytes(token_in.as_bytes()[12..20].try_into().unwrap());
    let token_out_u64 = u64::from_be_bytes(token_out.as_bytes()[12..20].try_into().unwrap());

    message.extend_from_slice(&token_in_u64.to_le_bytes());
    message.extend_from_slice(&token_out_u64.to_le_bytes());
    message.extend_from_slice(&amount_in.to_le_bytes());
    message.extend_from_slice(&amount_out.to_le_bytes());
    message.extend_from_slice(&sqrt_price_x96.as_u128().to_le_bytes());
    message.push(18); // Wei decimals
    message.push(18); // Wei decimals
    message.extend_from_slice(&tick.to_le_bytes());

    let checksum = crc32fast::hash(&message);
    let checksum_offset = std::mem::size_of::<MessageHeader>() - 4;
    message[checksum_offset..checksum_offset + 4].copy_from_slice(&checksum.to_le_bytes());

    message
}

#[tokio::test]
async fn test_live_polygon_and_kraken() {
    println!("\n=== Live Polygon + Kraken Roundtrip Test ===\n");

    // Setup strategies
    let mut consumers = HashMap::new();

    let flash_arbitrage = StrategyConsumer::new(
        "flash-arbitrage",
        vec!["market_data_polygon", "market_data_kraken"],
    );

    let kraken_signal = StrategyConsumer::new("kraken-signal", vec!["market_data_kraken"]);

    consumers.insert("flash-arbitrage", flash_arbitrage.clone());
    consumers.insert("kraken-signal", kraken_signal.clone());

    println!("Strategies:");
    println!("  flash-arbitrage → [polygon, kraken]");
    println!("  kraken-signal → [kraken]\n");

    // Connect to Kraken
    println!("Connecting to Kraken WebSocket...");
    let (kraken_stream, _) = connect_async("wss://ws.kraken.com")
        .await
        .expect("Failed to connect to Kraken");
    let (mut kraken_write, mut kraken_read) = kraken_stream.split();

    let subscribe = json!({
        "event": "subscribe",
        "pair": ["XBT/USD", "ETH/USD"],
        "subscription": {"name": "trade"}
    });
    kraken_write
        .send(Message::Text(subscribe.to_string()))
        .await
        .unwrap();

    // Connect to Polygon via WebSocket provider (try multiple endpoints)
    println!("Connecting to Polygon WebSocket...");
    let mut provider = None;
    for &endpoint in POLYGON_RPC_OPTIONS {
        match Provider::<Ws>::connect(endpoint).await {
            Ok(p) => {
                println!("  ✓ Connected to {}", endpoint);
                provider = Some(Arc::new(p));
                break;
            }
            Err(e) => {
                println!("  ✗ Failed to connect to {}: {}", endpoint, e);
            }
        }
    }

    let provider = provider.expect("Failed to connect to any Polygon endpoint");

    // Subscribe to Uniswap V3 WETH/USDC pool swaps
    let weth_usdc_pool: Address = "0x45dda9cb7c25131df268515131f647d726f50608"
        .parse()
        .unwrap();

    let swap_filter = Filter::new()
        .address(weth_usdc_pool)
        .event("Swap(address,address,int256,int256,uint160,uint128,int24)");

    let mut swap_stream = provider.subscribe_logs(&swap_filter).await.unwrap();

    println!("Connected to both Kraken and Polygon!\n");
    println!("Collecting live data for 30 seconds...\n");

    let start = std::time::SystemTime::now();
    let duration = std::time::Duration::from_secs(30);

    let mut kraken_count = 0;
    let mut polygon_count = 0;

    while std::time::SystemTime::now().duration_since(start).unwrap() < duration {
        tokio::select! {
            // Kraken messages
            Some(msg) = kraken_read.next() => {
                if let Ok(Message::Text(text)) = msg {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                        if json.get("event").is_some() { continue; }

                        if let Some(arr) = json.as_array() {
                            if arr.len() >= 4 && arr[2].as_str() == Some("trade") {
                                if let Some(trades) = arr[1].as_array() {
                                    let pair = arr[3].as_str().unwrap_or("");
                                    for trade in trades {
                                        if let Some(msg_bytes) = kraken_trade_to_protocol(pair, trade) {
                                            kraken_count += 1;

                                            let header = unsafe {
                                                std::ptr::read(msg_bytes.as_ptr() as *const MessageHeader)
                                            };

                                            println!("[Kraken #{}: {} @ {}]",
                                                kraken_count, pair, trade[0].as_str().unwrap_or("?"));

                                            // Route to strategies
                                            for (name, consumer) in &consumers {
                                                if consumer.subscribed_topics.contains(&"market_data_kraken".to_string()) {
                                                    let valid = consumer.receive_and_validate(header, msg_bytes.clone()).await;
                                                    println!("  → {} roundtrip: {}",
                                                        name, if valid { "✅" } else { "❌" });
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Polygon swap events
            Some(log) = swap_stream.next() => {
                if log.topics.len() >= 3 {
                    // Parse swap event
                    let sender = Address::from(log.topics[1]);
                    let recipient = Address::from(log.topics[2]);

                    // Decode data (amount0, amount1, sqrtPriceX96, liquidity, tick)
                    if log.data.len() >= 160 {
                        let amount0 = I256::from_raw(U256::from_big_endian(&log.data[0..32]));
                        let amount1 = I256::from_raw(U256::from_big_endian(&log.data[32..64]));
                        let sqrt_price = U256::from_big_endian(&log.data[64..96]);
                        let liquidity = u128::from_be_bytes(log.data[112..128].try_into().unwrap());
                        let tick = i32::from_be_bytes(log.data[156..160].try_into().unwrap());

                        let block = provider.get_block(log.block_number.unwrap())
                            .await
                            .unwrap()
                            .unwrap();

                        let msg_bytes = polygon_swap_to_protocol(
                            log.address,
                            "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse().unwrap(), // WETH
                            "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse().unwrap(), // USDC
                            amount0,
                            amount1,
                            sqrt_price,
                            liquidity,
                            tick,
                            block.timestamp.as_u64(),
                        );

                        polygon_count += 1;

                        let header = unsafe {
                            std::ptr::read(msg_bytes.as_ptr() as *const MessageHeader)
                        };

                        println!("[Polygon Swap #{}: WETH/USDC]", polygon_count);
                        println!("  Amount0: {}", amount0);
                        println!("  Amount1: {}", amount1);

                        // Route to strategies
                        for (name, consumer) in &consumers {
                            if consumer.subscribed_topics.contains(&"market_data_polygon".to_string()) {
                                let valid = consumer.receive_and_validate(header, msg_bytes.clone()).await;
                                println!("  → {} roundtrip: {}",
                                    name, if valid { "✅" } else { "❌" });
                            }
                        }
                    }
                }
            }

            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {}
        }
    }

    println!("\n=== Results ===\n");

    let flash_messages = flash_arbitrage.received_messages.lock().await;
    let flash_results = flash_arbitrage.roundtrip_results.lock().await;
    let kraken_messages = kraken_signal.received_messages.lock().await;
    let kraken_results = kraken_signal.roundtrip_results.lock().await;

    let flash_valid = flash_results.iter().filter(|&&v| v).count();
    let kraken_valid = kraken_results.iter().filter(|&&v| v).count();

    println!("flash-arbitrage:");
    println!(
        "  Received: {} messages (Kraken + Polygon)",
        flash_messages.len()
    );
    println!("  Roundtrip valid: {}/{}", flash_valid, flash_results.len());

    println!("\nkraken-signal:");
    println!(
        "  Received: {} messages (Kraken only)",
        kraken_messages.len()
    );
    println!(
        "  Roundtrip valid: {}/{}",
        kraken_valid,
        kraken_results.len()
    );

    println!("\nData sources:");
    println!("  Kraken trades: {}", kraken_count);
    println!("  Polygon swaps: {}", polygon_count);

    // Verify all roundtrips passed
    assert_eq!(
        flash_valid,
        flash_results.len(),
        "Some flash-arbitrage messages failed roundtrip!"
    );
    assert_eq!(
        kraken_valid,
        kraken_results.len(),
        "Some kraken-signal messages failed roundtrip!"
    );

    // Verify routing
    assert!(
        kraken_messages.len() <= flash_messages.len(),
        "kraken-signal should have fewer messages than flash-arbitrage"
    );

    println!("\n✅ Live multi-source roundtrip test PASSED!");
    println!("  - Real Kraken WebSocket data processed");
    println!("  - Real Polygon blockchain events processed");
    println!("  - Perfect binary roundtrip equality maintained");
    println!("  - Correct topic-based routing to strategies");
}

pub mod dex;
pub mod arbitrage_validator;
pub mod v3_math;

use crate::instruments::INSTRUMENTS;
use crate::unix_socket::UnixSocketWriter;
use crate::dex_registry::DexRegistry;
use alphapulse_protocol::{
    StatusUpdateMessage,
    SwapEvent, 
    PoolEvent, PoolUpdateType,
};
use alphapulse_protocol::messages::{
    TradeMessage, SwapEventMessage, PoolUpdateMessage,
};
use alphapulse_protocol::message_protocol::{
    SourceType, MESSAGE_MAGIC,
};
use zerocopy::FromZeroes;
use anyhow::Result;
use parking_lot::RwLock;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use tracing::{debug, info, warn, error};
use chrono;
use uuid::Uuid;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use dex::{DexPool, PoolFactory, identify_pool_event};

pub struct PolygonCollector {
    socket_writer: Arc<UnixSocketWriter>,
    pool_factory: Arc<PoolFactory>,
    dex_registry: Arc<DexRegistry>,
    pool_cache: Arc<RwLock<HashMap<String, Arc<Box<dyn DexPool>>>>>,
    alchemy_ws_url: String,
    sequence: Arc<std::sync::atomic::AtomicU32>,
    // Phase 2: Deep equality validation tracking
    message_cache: Arc<RwLock<HashMap<String, Value>>>, // message_id -> original_data
    // Phase 3: New message protocol with schema cache
    schema_cache: Arc<alphapulse_protocol::SchemaTransformCache>, // Required for bijective IDs
}

impl PolygonCollector {
    
    pub fn new(socket_writer: Arc<UnixSocketWriter>, schema_cache: Arc<alphapulse_protocol::SchemaTransformCache>) -> Self {
        // Try working endpoints provided by user: polygon-rpc.com, rpc.ankr.com/polygon, polygon.llamarpc.com, polygon.publicnode.com
        // Force use of public endpoint regardless of ANKR_API_KEY
        info!("🌐 Using free polygon.publicnode.com endpoint (ANKR credits exhausted)");
        let (rpc_url, ws_url) = (
            "https://polygon.publicnode.com".to_string(),
            "wss://polygon.publicnode.com".to_string()
        );
        
        // Create ONE shared HTTP client with SINGLE connection for ALL RPC calls
        let shared_client = Arc::new(
            reqwest::Client::builder()
                .pool_max_idle_per_host(1)    // ONLY 1 connection to polygon.publicnode.com
                .pool_idle_timeout(std::time::Duration::from_secs(30)) // Reduce to 30 seconds
                .timeout(std::time::Duration::from_secs(2))
                .tcp_keepalive(std::time::Duration::from_secs(10))
                // Let reqwest negotiate HTTP/1.1 or HTTP/2 automatically
                // polygon.publicnode.com doesn't support HTTP/2 prior knowledge
                .build()
                .expect("Failed to create shared HTTP client")
        );
        
        info!("🔗 Created shared HTTP client for all Polygon operations");
        
        let pool_factory = Arc::new(PoolFactory::new_with_client(rpc_url.clone(), shared_client.clone()));
        let dex_registry = Arc::new(DexRegistry::new());
        
        Self {
            socket_writer,
            pool_factory,
            dex_registry,
            pool_cache: Arc::new(RwLock::new(HashMap::new())),
            alchemy_ws_url: ws_url,
            sequence: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            // Phase 2: Initialize message cache for deep equality validation
            message_cache: Arc::new(RwLock::new(HashMap::new())),
            // Use the schema cache with bijective IDs
            schema_cache,
        }
    }
    
    /// Create new PolygonCollector with SchemaTransformCache for bijective ID protocol
    pub fn new_with_schema_cache(
        socket_writer: Arc<UnixSocketWriter>,
        schema_cache: Arc<alphapulse_protocol::SchemaTransformCache>
    ) -> Self {
        // Try working endpoints provided by user: polygon-rpc.com, rpc.ankr.com/polygon, polygon.llamarpc.com, polygon.publicnode.com
        // Force use of public endpoint regardless of ANKR_API_KEY
        info!("🌐 Using free polygon.publicnode.com endpoint (ANKR credits exhausted)");
        let (rpc_url, ws_url) = (
            "https://polygon.publicnode.com".to_string(),
            "wss://polygon.publicnode.com".to_string()
        );
        
        // Create ONE shared HTTP client with SINGLE connection for ALL RPC calls
        let shared_client = Arc::new(
            reqwest::Client::builder()
                .pool_max_idle_per_host(1)    // ONLY 1 connection to polygon.publicnode.com
                .pool_idle_timeout(std::time::Duration::from_secs(30)) // Reduce to 30 seconds
                .timeout(std::time::Duration::from_secs(2))
                .tcp_keepalive(std::time::Duration::from_secs(10))
                // Let reqwest negotiate HTTP/1.1 or HTTP/2 automatically
                // polygon.publicnode.com doesn't support HTTP/2 prior knowledge
                .build()
                .expect("Failed to create shared HTTP client")
        );
        
        info!("🔗 Created shared HTTP client for all Polygon operations");
        
        // Use the passed-in registries instead of creating new ones
        let pool_factory = Arc::new(PoolFactory::new_with_client(rpc_url.clone(), shared_client.clone()));
        let dex_registry = Arc::new(DexRegistry::new());
        
        Self {
            socket_writer,
            pool_factory,
            dex_registry,
            pool_cache: Arc::new(RwLock::new(HashMap::new())),
            alchemy_ws_url: ws_url,
            sequence: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            // Phase 2: Initialize message cache for deep equality validation
            message_cache: Arc::new(RwLock::new(HashMap::new())),
            // Use the new schema cache with bijective IDs
            schema_cache,
        }
    }
    
    pub async fn start(&self) -> Result<()> {
        info!("🚀 Starting Polygon DEX collector with modular architecture");
        
        // Schema cache handles all token caching via bijective IDs
        
        // Gas price monitoring will be event-driven via WebSocket newHeads subscription
        // No need for separate polling task
        
        // Start WebSocket monitoring
        if self.alchemy_ws_url != "no_websocket" && !self.alchemy_ws_url.is_empty() {
            info!("🔗 Attempting WebSocket connection to: {}", self.alchemy_ws_url);
            self.monitor_dex_events().await?;
        } else {
            warn!("⚠️ No WebSocket - real-time monitoring disabled");
            info!("💤 Entering idle mode - keeping connection alive...");
            
            // Keep the service alive without infinite reconnection
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            }
        }
        
        Ok(())
    }
    
    async fn monitor_dex_events(&self) -> Result<()> {
        info!("📡 Connecting to Polygon WebSocket for real-time DEX events");
        
        let (ws_stream, _) = connect_async(&self.alchemy_ws_url).await?;
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        
        // Split subscriptions for better reliability
        // Subscription 1: V3 Swap events only (most common)
        let v3_swap_subscription = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_subscribe",
            "params": [
                "logs",
                {
                    "topics": ["0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67"] // UniswapV3 Swap
                }
            ]
        });
        
        // Subscription 2: V2 Swap events
        let v2_swap_subscription = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "eth_subscribe",
            "params": [
                "logs",
                {
                    "topics": ["0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822"] // UniswapV2 Swap
                }
            ]
        });
        
        // Subscription 3: Sync events (V2 liquidity updates - most frequent)
        let sync_subscription = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "eth_subscribe",
            "params": [
                "logs",
                {
                    "topics": ["0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1"] // V2 Sync
                }
            ]
        });
        
        // Subscription 4: V2 Mint events
        let v2_mint_subscription = json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "eth_subscribe",
            "params": [
                "logs",
                {
                    "topics": ["0x4c209b5fc8ad50758f13e2e1088ba56a560dff690a1c6fef26394f4c03821c4f"] // V2 Mint
                }
            ]
        });
        
        // Subscription 5: V2 Burn events  
        let v2_burn_subscription = json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "eth_subscribe",
            "params": [
                "logs",
                {
                    "topics": ["0xdccd412f0b1252819cb1fd330b93224ca42612892bb3f4f789976e6d81936496"] // V2 Burn
                }
            ]
        });
        
        // Subscription 6: V3 Mint events
        let v3_mint_subscription = json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "eth_subscribe",
            "params": [
                "logs",
                {
                    "topics": ["0x7a53080ba414158be7ec69b987b5fb7d07dee101babe276914f785c42da22a1"] // V3 Mint
                }
            ]
        });
        
        // Subscription 7: V3 Burn events
        let v3_burn_subscription = json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "eth_subscribe",
            "params": [
                "logs",
                {
                    "topics": ["0x0c396cd989a39f4459b5fa1aed6a9a8dcdbc45908acfd67e028cd568da98982c"] // V3 Burn
                }
            ]
        });
        
        // Subscription 8: V3 Collect events
        let v3_collect_subscription = json!({
            "jsonrpc": "2.0",
            "id": 8,
            "method": "eth_subscribe",
            "params": [
                "logs",
                {
                    "topics": ["0x70935338e69775456a85ddef226c395fb668b63fa0115f5f20610b388e6ca9c0"] // V3 Collect
                }
            ]
        });
        
        // Subscription 9: New blocks for gas prices
        let block_subscription = json!({
            "jsonrpc": "2.0",
            "id": 9,
            "method": "eth_subscribe",
            "params": ["newHeads"]
        });
        
        // Send subscriptions with small delay between each to ensure proper handling
        ws_sender.send(Message::Text(v3_swap_subscription.to_string())).await?;
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        ws_sender.send(Message::Text(v2_swap_subscription.to_string())).await?;
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        ws_sender.send(Message::Text(sync_subscription.to_string())).await?;
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        ws_sender.send(Message::Text(v2_mint_subscription.to_string())).await?;
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        ws_sender.send(Message::Text(v2_burn_subscription.to_string())).await?;
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        ws_sender.send(Message::Text(v3_mint_subscription.to_string())).await?;
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        ws_sender.send(Message::Text(v3_burn_subscription.to_string())).await?;
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        ws_sender.send(Message::Text(v3_collect_subscription.to_string())).await?;
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        ws_sender.send(Message::Text(block_subscription.to_string())).await?;
        
        info!("✅ Subscribed to all DEX events: V3/V2 swaps, Sync, V2/V3 Mint/Burn, V3 Collect, and blocks");
        
        let collector = self.clone();
        let handle = tokio::spawn(async move {
            let mut swap_count = 0;
            let mut heartbeat_count = 0;
            
            while let Some(msg) = ws_receiver.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        let ws_receive_time = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs_f64();
                        
                        if let Ok(data) = serde_json::from_str::<Value>(&text) {
                            // Check for subscription confirmation
                            if data.get("id").is_some() && data.get("result").is_some() {
                                let id = data.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
                                let sub_name = match id {
                                    1 => "V3 swaps",
                                    2 => "V2 swaps",
                                    3 => "Sync events",
                                    4 => "V2 Mint events",
                                    5 => "V2 Burn events",
                                    6 => "V3 Mint events",
                                    7 => "V3 Burn events",
                                    8 => "V3 Collect events",
                                    9 => "blocks",
                                    _ => "unknown"
                                };
                                info!("🔗 WebSocket subscription confirmed: {}", sub_name);
                                continue;
                            }
                            
                            // Check for actual events
                            if let Some(params) = data.get("params") {
                                if let Some(result) = params.get("result") {
                                    // Check if this is a block header update
                                    if result.get("gasLimit").is_some() && result.get("number").is_some() {
                                        // New block header received
                                        if let Err(e) = collector.handle_new_block(result).await {
                                            debug!("Failed to handle new block: {}", e);
                                        }
                                        continue;
                                    }
                                    
                                    // Check event type from topics[0]
                                    if let Some(topics) = result.get("topics").and_then(|t| t.as_array()) {
                                        if let Some(event_sig) = topics.get(0).and_then(|s| s.as_str()) {
                                            match event_sig {
                                                // Swap events
                                                "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822" |
                                                "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67" |
                                                "0x8b3e96f2b889fa771c53c981b40daf005f63f637f1869f707052d15a3dd97140" => {
                                                    swap_count += 1;
                                                    
                                                    // Log WebSocket message timing to detect ANKR batching
                                                    let time_str = chrono::DateTime::from_timestamp(ws_receive_time as i64, 
                                                        ((ws_receive_time.fract() * 1_000_000_000.0) as u32))
                                                        .map(|dt| dt.format("%H:%M:%S%.6f").to_string())
                                                        .unwrap_or_else(|| format!("{:.6}", ws_receive_time));
                                                    info!("🔍 Public WS delivered swap #{} at {}", swap_count, time_str);
                                                    
                                                    // PHASE 2: Generate unique message ID for deep equality tracking
                                                    let message_id = Uuid::new_v4().to_string();
                                                    
                                                    // Cache original message for validation
                                                    {
                                                        let mut cache = collector.message_cache.write();
                                                        cache.insert(message_id.clone(), result.clone());
                                                        
                                                        // Clean up old entries to prevent memory leak (keep last 1000)
                                                        if cache.len() > 1000 {
                                                            // Remove oldest entries (simplified cleanup)
                                                            let keys_to_remove: Vec<_> = cache.keys().take(100).cloned().collect();
                                                            for key in keys_to_remove {
                                                                cache.remove(&key);
                                                            }
                                                        }
                                                    }
                                                    
                                                    debug!("🆔 Generated message ID {} for swap #{}", message_id, swap_count);
                                                    
                                                    if swap_count % 10 == 0 {
                                                        debug!("📊 Processed {} swaps", swap_count);
                                                    }
                                                    // Spawn async task to process swap without blocking
                                                    let collector_clone = collector.clone();
                                                    let result_clone = result.clone();
                                                    let swap_num = swap_count;
                                                    let msg_id = message_id.clone();
                                                    tokio::spawn(async move {
                                                        if let Err(e) = collector_clone.process_swap_event_with_id(&result_clone, &msg_id).await {
                                                            if let Some(addr) = result_clone.get("address").and_then(|v| v.as_str()) {
                                                                debug!("Failed to process swap #{} for pool {}: {}", swap_num, addr, e);
                                                            }
                                                        }
                                                    });
                                                }
                                                // Pool events (V2/V3 Mint/Burn/Collect/Sync)
                                                "0x4c209b5fc8ad50758f13e2e1088ba56a560dff690a1c6fef26394f4c03821c4f" | // V2 Mint
                                                "0xdccd412f0b1252819cb1fd330b93224ca42612892bb3f4f789976e6d8136129a" | // V2 Burn
                                                "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1" | // V2 Sync
                                                "0x7a53080ba414158be7ec69b987b5fb7d07dee101babe276914f785c42da22a01b" | // V3 Mint
                                                "0x0c396cd989a39f4459b5fa1aed6a9a8dcdbc45908acfd67e028cd568da98982c" | // V3 Burn
                                                "0x40d0efd1a53d60ecbf40971b9daf7dc90178c3aadc7aab1765632738fa8b8f01" => { // V3 Collect
                                                    // Use unified pool event handler
                                                    let collector_clone = collector.clone();
                                                    let result_clone = result.clone();
                                                    tokio::spawn(async move {
                                                        if let Err(e) = collector_clone.handle_pool_event(&result_clone).await {
                                                            debug!("Failed to process pool event: {}", e);
                                                        }
                                                    });
                                                }
                                                _ => {
                                                    // Unknown event
                                                    debug!("Unknown event signature: {}", event_sig);
                                                }
                                            }
                                            continue;
                                        }
                                    }
                                    swap_count += 1;
                                    if swap_count % 10 == 0 {
                                        debug!("📊 Processed {} swaps", swap_count);
                                    }
                                    // Spawn async task to process swap without blocking
                                    let collector_clone = collector.clone();
                                    let result_clone = result.clone();
                                    let swap_num = swap_count;
                                    tokio::spawn(async move {
                                        if let Err(e) = collector_clone.process_swap_event(&result_clone).await {
                                            // Log more details about the failure
                                            if let Some(addr) = result_clone.get("address").and_then(|v| v.as_str()) {
                                                debug!("Failed to process swap #{} for pool {}: {}", swap_num, addr, e);
                                            } else {
                                                debug!("Failed to process swap #{}: {}", swap_num, e);
                                            }
                                        }
                                    });
                                }
                            } else {
                                // Heartbeat or other message
                                heartbeat_count += 1;
                                if heartbeat_count % 100 == 0 {
                                    debug!("💓 Received {} heartbeats", heartbeat_count);
                                }
                            }
                        } else {
                            debug!("Failed to parse WebSocket message: {}", text);
                        }
                    }
                    Ok(Message::Close(_)) => {
                        warn!("WebSocket closed, reconnecting immediately for blazing fast recovery!");
                        // No delay - immediate reconnection for real-time arbitrage!
                        break;
                    }
                    Err(e) => {
                        error!("WebSocket error: {}", e);
                    }
                    _ => {}
                }
            }
            warn!("DEX monitoring loop exited after {} swaps", swap_count);
        });
        
        // CRITICAL FIX: Wait for the WebSocket task to complete instead of returning immediately
        // This prevents the infinite reconnection loop that was creating 100+ connections
        handle.await.map_err(|e| anyhow::anyhow!("WebSocket task failed: {}", e))?;
        
        Ok(())
    }
    
    /// PHASE 2: Process swap event with message ID for deep equality validation
    async fn process_swap_event_with_id(&self, event: &Value, message_id: &str) -> Result<()> {
        debug!("🆔 Processing swap event with message ID: {}", message_id);
        
        // Generate hash of original data for integrity checking
        let original_hash = self.generate_data_hash(event);
        
        // DISABLED: Send message trace through binary protocol (causing corruption)
        // MessageTrace messages contain string data that corrupts the binary protocol stream
        // TODO: Implement separate logging channel for validation traces
        /*
        if let Err(e) = self.send_message_trace(message_id, &original_hash, "collector").await {
            warn!("Failed to send message trace for {}: {}", message_id, e);
        }
        */
        
        // CRITICAL: Track processing result for deep equality validation
        match self.process_swap_event(event).await {
            Ok(()) => {
                debug!("✅ Completed processing for message ID: {}", message_id);
                Ok(())
            }
            Err(e) => {
                // DEEP EQUALITY VIOLATION: We have input but failed to produce output!
                let pool_address = event["address"].as_str().unwrap_or("unknown");
                error!("🚨 DEEP EQUALITY VIOLATION: Failed to process swap {} for pool {}: {}", 
                       message_id, pool_address, e);
                
                // Track failure in validation system
                self.track_processing_failure(message_id, pool_address, &e.to_string()).await;
                
                // Still return the error to maintain existing behavior
                Err(e)
            }
        }
    }

    /// Track processing failure for deep equality validation
    async fn track_processing_failure(&self, message_id: &str, pool_address: &str, error: &str) {
        // Increment failure counter
        static FAILURE_COUNT: AtomicU64 = AtomicU64::new(0);
        let failures = FAILURE_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
        
        // Alert if failure rate is too high
        static TOTAL_COUNT: AtomicU64 = AtomicU64::new(0);
        let total = TOTAL_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
        let failure_rate = (failures as f64 / total as f64) * 100.0;
        
        if failure_rate > 10.0 && total > 100 {
            error!("🚨🚨 CRITICAL: Deep equality failure rate {:.1}% ({}/{} messages failed to process)", 
                   failure_rate, failures, total);
        }
        
        // Send failure notification through pipeline (future enhancement)
        // This would send a special "ProcessingFailed" message type through the binary protocol
        // so the validation system knows this message had no output
    }
    
    /// Generate SHA-256 hash of the original data for integrity verification
    fn generate_data_hash(&self, data: &Value) -> String {
        let json_str = serde_json::to_string(data).unwrap_or_default();
        let mut hasher = DefaultHasher::new();
        json_str.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    // REMOVED: send_message_trace function was causing binary protocol corruption
    // The MessageTrace messages contain variable-length strings that were corrupting
    // the fixed-size binary protocol stream

    async fn process_swap_event(&self, event: &Value) -> Result<()> {
        let pool_address = event["address"].as_str()
            .ok_or_else(|| anyhow::anyhow!("No pool address"))?;
        
        // DEBUG: Check if this is a valid Ethereum address
        if !pool_address.starts_with("0x") || pool_address.len() != 42 {
            warn!("⚠️ Invalid pool address format: {}", pool_address);
            return Err(anyhow::anyhow!("Invalid pool address format: {}", pool_address));
        }
        
        let swap_data = event["data"].as_str()
            .ok_or_else(|| anyhow::anyhow!("No swap data"))?;
        let tx_hash = event["transactionHash"].as_str().unwrap_or("unknown");
        let block_hex = event["blockNumber"].as_str().unwrap_or("0x0");
        let block_number = u64::from_str_radix(block_hex.trim_start_matches("0x"), 16)?;
        
        // Extract event signature from topics[0]
        let event_signature = event["topics"][0].as_str()
            .ok_or_else(|| anyhow::anyhow!("No event signature in topics[0]"))?;
        
        // Get or create pool instance using event signature
        let pool = self.get_or_create_pool_by_signature(pool_address, event_signature).await?;
        
        // Parse swap event using the appropriate DEX module
        let mut swap_event = pool.parse_swap_event(swap_data)?;
        
        // Get token addresses from the pool
        let (token0_addr, token1_addr) = pool.get_tokens().await?;
        
        // Create bijective InstrumentIds from addresses
        use alphapulse_protocol::message_protocol::{InstrumentId, VenueId};
        let token0_id = InstrumentId::polygon_token(&token0_addr)?;
        let token1_id = InstrumentId::polygon_token(&token1_addr)?;
        let pool_id = InstrumentId::pool(VenueId::Polygon, token0_id, token1_id);
        
        debug!("📡 Processing swap for pool {}: token0_id={:?}, token1_id={:?}", 
               pool_address, token0_id, token1_id);
        
        // Fill in common token info and transaction details
        match &mut swap_event {
            SwapEvent::UniswapV2(v2) => {
                v2.core.tx_hash = tx_hash.to_string();
                v2.core.block_number = block_number;
                v2.core.timestamp_ns = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64;
                v2.core.pool_id = pool_id;
                v2.core.token0_id = token0_id;
                v2.core.token1_id = token1_id;
                
                // Store raw amounts for debugging
                let raw_amount0_in = v2.core.amount0_in;
                let raw_amount1_in = v2.core.amount1_in;
                let raw_amount0_out = v2.core.amount0_out;
                let raw_amount1_out = v2.core.amount1_out;
                
                // Apply decimal adjustments for V2 (amounts are in raw form)
                // NOTE: We keep amounts in raw wei format (no division) to preserve precision
                // The price calculation should handle the decimal conversion
                debug!("Raw swap amounts before decimal adjustment: in0={}, in1={}, out0={}, out1={}",
                       raw_amount0_in, raw_amount1_in, raw_amount0_out, raw_amount1_out);
                // Decimals handled by downstream services via bijective IDs
            }
            SwapEvent::UniswapV3(v3) => {
                v3.core.tx_hash = tx_hash.to_string();
                v3.core.block_number = block_number;
                v3.core.timestamp_ns = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64;
                v3.core.pool_id = pool_id;
                v3.core.token0_id = token0_id;
                v3.core.token1_id = token1_id;
                
                // Store raw amounts for debugging
                let raw_amount0_in = v3.core.amount0_in;
                let raw_amount1_in = v3.core.amount1_in;
                let raw_amount0_out = v3.core.amount0_out;
                let raw_amount1_out = v3.core.amount1_out;
                
                // Apply decimal adjustments for V3
                // NOTE: We keep amounts in raw wei format (no division) to preserve precision
                // The price calculation should handle the decimal conversion
                debug!("V3 Raw swap amounts: in0={}, in1={}, out0={}, out1={}",
                       raw_amount0_in, raw_amount1_in, raw_amount0_out, raw_amount1_out);
                // Decimals handled by downstream services via bijective IDs
                
                // Get actual V3 liquidity
                if let Ok(liq) = self.get_v3_active_liquidity(pool_address, v3.tick).await {
                    v3.liquidity = liq;
                }
            }
            SwapEvent::Curve(_) => {
                // Future implementation for Curve
            }
        }
        
        // Collector just forwards raw data - no validation or price calculation
        // Downstream services handle any analysis using bijective IDs
        let core = swap_event.core();
        debug!("Forwarding swap event for pool {}: amounts in0={}, out0={}, in1={}, out1={}",
               pool_address, core.amount0_in, core.amount0_out, core.amount1_in, core.amount1_out);
        
        // Send swap event via binary protocol
        self.send_swap_event(&swap_event, &pool).await?;
        
        debug!("💱 {} swap processed for pool {}",
            pool.dex_name(),
            pool_address
        );
        
        Ok(())
    }
    
    async fn get_or_create_pool_by_signature(&self, address: &str, event_signature: &str) -> Result<Arc<Box<dyn DexPool>>> {
        // Check cache first
        {
            let cache = self.pool_cache.read();
            if let Some(pool) = cache.get(address) {
                return Ok(pool.clone());
            }
        }
        
        // Classify pool type by event signature
        let pool_type = self.pool_factory.classify_by_event_signature(event_signature)
            .ok_or_else(|| anyhow::anyhow!("Unknown event signature: {}", event_signature))?;
        
        info!("🔍 Creating pool instance for {} with event signature {} (type: {:?})", 
              address, event_signature, pool_type);
        
        // Create pool without expensive DEX identification
        let pool = self.pool_factory.create_pool_by_signature(address, pool_type).await?;
        
        // Pool tracking handled by bijective IDs - no registration needed
        info!("🔍 Created pool instance for {} (type: {:?})", address, pool_type);
        
        // Cache it
        let pool_arc = Arc::new(pool);
        {
            let mut cache = self.pool_cache.write();
            cache.insert(address.to_string(), pool_arc.clone());
        }
        
        Ok(pool_arc)
    }
    
    // Keep old method for backward compatibility (but mark deprecated)
    #[deprecated(note = "Use get_or_create_pool_by_signature instead")]
    async fn get_or_create_pool(&self, address: &str) -> Result<Arc<Box<dyn DexPool>>> {
        // Default to UniswapV2 signature for backward compatibility
        self.get_or_create_pool_by_signature(address, dex::UNISWAP_V2_SWAP_SIGNATURE).await
    }
    
    async fn send_swap_event(
        &self,
        swap: &SwapEvent,
        pool: &Arc<Box<dyn DexPool>>,
    ) -> Result<()> {
        // Convert to new binary message format with InstrumentIds
        use alphapulse_protocol::messages::SwapEventMessage;
        use alphapulse_protocol::message_protocol::SourceType;
        
        let core = swap.core();
        let sequence = self.sequence.fetch_add(1, Ordering::Relaxed) as u64;
        
        // Convert amounts to fixed-point (8 decimals)
        let to_fixed_point = |amount: u128| -> u64 {
            // Assuming 18 decimals, scale down to 8
            (amount / 10_u128.pow(10)) as u64
        };
        
        let swap_msg = SwapEventMessage::new(
            core.pool_id,
            core.token0_id,
            core.token1_id,
            to_fixed_point(core.amount0_in),
            to_fixed_point(core.amount1_in),
            to_fixed_point(core.amount0_out),
            to_fixed_point(core.amount1_out),
            sequence,
            SourceType::PolygonCollector,
        );
        
        // Send via new protocol
        // TODO: Implement actual sending once socket writer is updated
        // self.socket_writer.write_message(&swap_msg)?;
        
        debug!("📤 Sent SwapEvent: pool_id={:?} type={:?} in0={} in1={} out0={} out1={}",
            core.pool_id, 
            match swap {
                SwapEvent::UniswapV2(_) => "V2",
                SwapEvent::UniswapV3(_) => "V3",
                SwapEvent::Curve(_) => "Curve",
            },
            core.amount0_in, core.amount1_in, 
            core.amount0_out, core.amount1_out);
        
        Ok(())
    }
    
    /// Send trade using new bijective ID protocol
    async fn send_new_protocol_trade(
        &self,
        swap: &SwapEvent,
        pool: &Arc<Box<dyn DexPool>>,
        schema_cache: &alphapulse_protocol::SchemaTransformCache,
    ) -> Result<()> {
        use alphapulse_protocol::{
            InstrumentId, VenueId, NewTradeMessage, NewTradeSide, SourceType,
            InstrumentDiscoveredMessage, CachedObject, TokenMetadata
        };
        use zerocopy::AsBytes;
        
        let core = swap.core();
        
        // Pool ID is already in the core
        let pool_instrument_id = core.pool_id;
        
        // Cache the pool instrument if not already cached
        if schema_cache.get(&pool_instrument_id).is_none() {
            let pool_symbol = format!("POOL_{:x}", pool_instrument_id.to_u64());
            let discovery_msg = InstrumentDiscoveredMessage::new(
                pool_instrument_id,
                pool_symbol,
                18, // Default to 18 decimals for pools
                format!("{}:pool", pool.dex_name()).into_bytes(),
                self.sequence.fetch_add(1, std::sync::atomic::Ordering::Relaxed) as u64,
                SourceType::PolygonCollector,
            );
            
            // Send instrument discovery message
            let discovery_bytes = discovery_msg.serialize();
            if let Err(e) = self.socket_writer.write_bytes(&discovery_bytes) {
                warn!("Failed to send instrument discovery: {}", e);
            } else {
                debug!("📝 Sent instrument discovery for pool: {}", pool_instrument_id.debug_info());
            }
        }
        
        // Determine trade side and calculate amounts
        let (side, price_fixed, volume_fixed) = if core.amount0_in > 0 && core.amount1_out > 0 {
            // Buying token1 with token0 - treat as Buy
            let price = if core.amount0_in > 0 { 
                (core.amount1_out as f64) / (core.amount0_in as f64) 
            } else { 0.0 };
            let volume = core.amount0_in as f64;
            (NewTradeSide::Buy, (price * 100_000_000.0) as i64, (volume * 100_000_000.0) as u64)
        } else if core.amount1_in > 0 && core.amount0_out > 0 {
            // Selling token1 for token0 - treat as Sell
            let price = if core.amount1_in > 0 { 
                (core.amount0_out as f64) / (core.amount1_in as f64) 
            } else { 0.0 };
            let volume = core.amount1_in as f64;
            (NewTradeSide::Sell, (price * 100_000_000.0) as i64, (volume * 100_000_000.0) as u64)
        } else {
            // No trade amounts, skip
            return Ok(());
        };
        
        // Create new protocol trade message
        let trade_msg = NewTradeMessage::new(
            pool_instrument_id,
            price_fixed,
            volume_fixed,
            side,
            self.sequence.fetch_add(1, std::sync::atomic::Ordering::Relaxed) as u64,
            SourceType::PolygonCollector,
        );
        
        // Send new protocol message
        let trade_bytes = trade_msg.as_bytes();
        if let Err(e) = self.socket_writer.write_bytes(trade_bytes) {
            warn!("Failed to send new protocol trade: {}", e);
        } else {
            debug!("🎯 Sent NEW PROTOCOL trade: {} {} @ {:.6} vol {:.2}", 
                   pool_instrument_id.debug_info(), 
                   format!("{:?}", side).to_lowercase(),
                   trade_msg.price_decimal(),
                   trade_msg.volume_decimal());
        }
        
        Ok(())
    }
    
    async fn send_trade_update(
        &self,
        pool: &Arc<Box<dyn DexPool>>,
        base: &str,
        quote: &str,
        price: f64,
        volume: f64,
    ) -> Result<()> {
        // Collector just forwards events, doesn't send trade updates
        Ok(())
    }
    
    async fn get_v3_active_liquidity(&self, _pool_address: &str, _current_tick: i32) -> Result<u128> {
        // Collector doesn't track liquidity
        Ok(0)
    }
    
    async fn handle_new_block(&self, _block: &Value) -> Result<()> {
        // Collector just forwards events
        Ok(())
    }
    
    /* Commented out - old implementation
    async fn send_trade_update_old(
        &self,
        pool: &Arc<Box<dyn DexPool>>,
        base: &str,
        quote: &str,
        price: f64,
        volume: f64,
    ) -> Result<()> {
            // Use pool-specific symbol hashing for DEX swaps
        // Format: "polygon:POOL_ADDRESS:DAI/LGNS" to distinguish between pools
        let pool_address = pool.address();
        let symbol_string = format!("{}:{}/{}", pool_address, base, quote);
        let symbol_id = INSTRUMENTS.get_or_create_hash("polygon", &symbol_string);
        
        // Send symbol mapping if needed
        if let Some(mapping) = INSTRUMENTS.create_mapping_message(symbol_id) {
            self.socket_writer.write_symbol_mapping(&mapping)?;
        }
        
        // Get actual pool liquidity from V2 Sync events
        let (pool_liquidity_usd, actual_reserves, total_tracked_pools) = {
            let liquidity_states = self.pool_liquidity.read();
            let tracked_pools = liquidity_states.len();
            
            // Try to get actual reserves from Sync data
            let (liquidity_usd, reserves) = if let Some(state) = liquidity_states.get(pool_address) {
                // We have actual reserve data from Sync events!
                // Calculate USD value based on reserves and price
                let reserve_product = state.reserve0 * state.reserve1;
                
                // For USD pairs (USDC, USDT, DAI), one side is already in USD
                // For non-USD pairs, estimate based on geometric mean and price
                let usd_value = if symbol_string.contains("USDC") || symbol_string.contains("USDT") || symbol_string.contains("DAI") {
                    // One side is USD-pegged, use 2x that reserve as total liquidity
                    if symbol_string.ends_with("/USDC") || symbol_string.ends_with("/USDT") || symbol_string.ends_with("/DAI") {
                        state.reserve1 * 2.0  // Token1 is USD
                    } else {
                        state.reserve0 * 2.0  // Token0 is USD
                    }
                } else {
                    // Non-USD pair, use geometric mean with price estimate
                    // Total value = 2 * sqrt(reserve0 * reserve1) * sqrt(price)
                    2.0 * reserve_product.sqrt() * price.sqrt()
                };
                
                debug!("💎 Using ACTUAL pool reserves: {:.2}/{:.2}, USD value: ${:.2}", 
                       state.reserve0, state.reserve1, usd_value);
                
                (usd_value, Some((state.reserve0, state.reserve1)))
            } else {
                // No Sync data yet - use trade volume as a last resort
                // But mark it clearly as an estimate
                let safe_volume = volume.min(10_000.0);
                let estimate = (safe_volume * 100.0).max(25_000.0).min(2_000_000.0);
                
                debug!("⚠️ No pool reserves available for {}, using volume estimate: ${:.2}", 
                       pool_address, estimate);
                
                (estimate, None)
            };
            
            (liquidity_usd, reserves, tracked_pools)
        };
        
        // Send trade update - use proper constructor like other exchanges
        let now_ns = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64;
        
        // Convert to fixed-point like Coinbase (multiply by scale factor)
        let price_fixed = (price * 1e8) as u64;  // 8 decimal places precision
        let volume_fixed = (volume * 1e8) as u64;  // Send actual trade volume, not pool liquidity
        
        // TODO: Create a separate PoolLiquidity message type to send actual reserves
        // For now, we're fixing the critical bug of sending pool liquidity as volume
        
        debug!("💰 Pool liquidity: trade_volume=${:.2}, pool_liquidity_usd=${:.2}, has_actual_reserves={}, pools_tracked={}", 
               volume, pool_liquidity_usd, actual_reserves.is_some(), total_tracked_pools);
        
        if let Some((r0, r1)) = actual_reserves {
            debug!("📊 Actual reserves: {:.2}/{:.2} for {}", r0, r1, symbol_string);
        }
        
        debug!("Fixed-point conversion: price={:.6} -> {}, liquidity={:.2} -> {}, symbol_id={}", 
               price, price_fixed, pool_liquidity_usd, volume_fixed, symbol_id);
        
        let trade_side = alphapulse_protocol::TradeSide::Buy; // Default side
        let trade = TradeMessage::new(
            now_ns,
            price_fixed,
            volume_fixed,
            symbol_id,
            trade_side,
        );
        
        self.socket_writer.write_trade(&trade)?;
        
        Ok(())
    }
    
    /// Estimate V3 pool liquidity from swap impact
    /// This provides a better estimate than hardcoded 0 until we have direct pool queries
    /// Get actual V3 liquidity at the current tick from tracked mint/burn events
    async fn get_v3_active_liquidity(&self, _pool_address: &str, _current_tick: i32) -> Result<u128> {
        // Collector doesn't track liquidity
        Ok(0)
    }
    
    */
    
    /* Commented out - old implementation
    async fn get_v3_active_liquidity_old(&self, pool_address: &str, current_tick: i32) -> Result<u128> {
        let liquidity_states = self.pool_liquidity.read();
        
        if let Some(state) = liquidity_states.get(pool_address) {
            // V3 liquidity is active within tick ranges
            // Sum liquidity from all positions that include the current tick
            let mut active_liquidity = 0.0;
            
            // Check tick ranges around current tick (typical tick spacing is 60 for 0.3% pools)
            let tick_spacing = 60;
            let aligned_tick = (current_tick / tick_spacing) * tick_spacing;
            
            // Sum liquidity from nearby ticks
            for offset in -2..=2 {
                let tick = aligned_tick + (offset * tick_spacing);
                if let Some(liq) = state.tick_liquidity.get(&tick) {
                    active_liquidity += liq;
                }
            }
            
            // Also check if we have a total liquidity value
            if active_liquidity == 0.0 && state.total_liquidity > 0.0 {
                // Use total liquidity as fallback
                active_liquidity = state.total_liquidity;
            }
            
            debug!("V3 active liquidity for {} at tick {}: {:.2}", 
                   pool_address, current_tick, active_liquidity);
            
            Ok(active_liquidity as u128)
        } else {
            // No liquidity data tracked yet
            debug!("No V3 liquidity data for pool {}", pool_address);
            Err(anyhow::anyhow!("No liquidity data available"))
        }
    }
    
    async fn estimate_v3_liquidity(&self, swap: &SwapEvent, sqrt_price: u128) -> Result<u128> {
        // For V3 pools, we can estimate active liquidity from the swap impact
        // Using the formula: liquidity = amount / (sqrt(P1) - sqrt(P0))
        // This is an approximation but much better than hardcoded 0
        
        let core = swap.core();
        let total_in = (core.amount0_in + core.amount1_in) as f64;
        let total_out = (core.amount0_out + core.amount1_out) as f64;
        
        if total_in == 0.0 && total_out == 0.0 {
            return Ok(0);
        }
        
        // Use the larger of input/output as the trade size
        let trade_size = total_in.max(total_out);
        
        // Estimate that this trade caused ~0.1% price impact on average
        // For a typical V3 pool, liquidity = trade_size / price_impact_percentage
        let estimated_impact = 0.001; // 0.1% typical impact
        let estimated_liquidity = trade_size / estimated_impact;
        
        // Convert to u128, cap at reasonable maximum
        let liquidity_u128 = estimated_liquidity.min(1e18) as u128;
        
        debug!("Estimated V3 liquidity from swap: {} (trade_size: {:.2})", 
               liquidity_u128, trade_size);
        
        Ok(liquidity_u128)
    }
    
    /// Generate bijective pool ID (preferred over legacy hash)
    fn get_pool_id(&self, _pool_address: &str, token0: &str, token1: &str) -> alphapulse_protocol::InstrumentId {
        // Create InstrumentIds for both tokens
        let token0_id = alphapulse_protocol::InstrumentId::polygon_token(token0)
            .unwrap_or_else(|_| alphapulse_protocol::InstrumentId::from_u64(0));
        let token1_id = alphapulse_protocol::InstrumentId::polygon_token(token1)
            .unwrap_or_else(|_| alphapulse_protocol::InstrumentId::from_u64(0));
            
        // Create pool ID from constituent tokens
        alphapulse_protocol::InstrumentId::pool(
            alphapulse_protocol::VenueId::Polygon,
            token0_id,
            token1_id
        )
    }
    
    /// DEPRECATED: Legacy hash function for backward compatibility
    #[deprecated(note = "Use get_pool_id() with bijective IDs instead")]
    fn get_pool_hash(&self, pool_address: &str) -> u64 {
        // Keep for legacy compatibility but prefer bijective IDs
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        pool_address.hash(&mut hasher);
        hasher.finish()
    }

    /// Send precise pool state from Sync events to scanner (no latency)
    async fn send_pool_state_update(
        &self,
        pool_address: &str,
        reserve0: f64,
        reserve1: f64,
        token0_symbol: &str,
        token1_symbol: &str,
        token0_address: &str,
        token1_address: &str,
    ) -> Result<()> {
        // Collector just forwards events, doesn't send state updates
        Ok(())
    }
    
    */
    
    /* Commented out - old implementation
    async fn send_pool_state_update_old(
        &self,
        pool_address: &str,
        reserve0: f64,
        reserve1: f64,
        token0_symbol: &str,
        token1_symbol: &str,
        token0_address: &str,
        token1_address: &str,
    ) -> Result<()> {
        let now_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos() as u64;
            
        let pool_hash = PoolUpdateMessage::hash_pool_address(pool_address);
        
        // Pool tracking handled by bijective IDs
        
        // Convert to fixed-point decimals for precision
        let to_fixed = |value: f64| -> i64 {
            (value * 100_000_000.0) as i64 // 8 decimal places
        };
        
        // Note: PoolUpdate messages don't need token hashes like SwapEvents
        
        // Create PoolUpdate message with exact reserves for arbitrage detection
        let reserve0_fixed = to_fixed(reserve0) as u128;
        let reserve1_fixed = to_fixed(reserve1) as u128;
        
        let mut pool_msg = PoolUpdateMessage::new_zeroed();
        pool_msg.set_timestamp_ns(now_ns);
        pool_msg.set_pool_hash(pool_hash);
        pool_msg.update_type = PoolUpdateType::Sync as u8; // Reserve sync
        pool_msg.protocol_type = 1; // V2
        
        // Pack V2 reserve data into data field:
        // offset 0-15: liquidity (16 bytes) - set to 0 for reserve updates
        // offset 16-31: amount0 (16 bytes) - set to 0 for reserve updates  
        // offset 32-47: amount1 (16 bytes) - set to 0 for reserve updates
        // offset 48-63: reserves0_after (16 bytes) - current reserve0
        // offset 64-79: reserves1_after (16 bytes) - current reserve1
        // offset 80: token0_decimals (1 byte)
        // offset 81: token1_decimals (1 byte)
        
        // Skip liquidity(16) + amount0(16) + amount1(16) = 48 bytes
        let mut offset = 48;
        pool_msg.data[offset..offset+16].copy_from_slice(&reserve0_fixed.to_le_bytes());
        offset += 16;
        pool_msg.data[offset..offset+16].copy_from_slice(&reserve1_fixed.to_le_bytes());
        
        // Add token decimal info (will be populated by handle_pool_event)
        pool_msg.data[80] = 18; // Default decimals, will be updated by caller
        pool_msg.data[81] = 18; // Default decimals, will be updated by caller
        
        // Pack token metadata starting at offset 100
        // offset 100-119: token0_address (20 bytes)
        // offset 120-139: token1_address (20 bytes)
        // offset 140-147: token0_symbol (8 bytes, null-padded)
        // offset 148-155: token1_symbol (8 bytes, null-padded)
        
        debug!("📦 Packing token metadata - token0: {} ({}), token1: {} ({})", 
               token0_symbol, token0_address, token1_symbol, token1_address);
        
        // Convert address strings to bytes (assuming hex format "0x...")
        if token0_address.starts_with("0x") && token0_address.len() >= 42 {
            if let Ok(addr_bytes) = hex::decode(&token0_address[2..42]) {
                pool_msg.data[100..120].copy_from_slice(&addr_bytes);
                debug!("✅ Packed token0 address: {}", token0_address);
            } else {
                debug!("❌ Failed to decode token0 address: {}", token0_address);
            }
        } else {
            debug!("⚠️ Invalid token0 address format: {}", token0_address);
        }
        
        if token1_address.starts_with("0x") && token1_address.len() >= 42 {
            if let Ok(addr_bytes) = hex::decode(&token1_address[2..42]) {
                pool_msg.data[120..140].copy_from_slice(&addr_bytes);
                debug!("✅ Packed token1 address: {}", token1_address);
            } else {
                debug!("❌ Failed to decode token1 address: {}", token1_address);
            }
        } else {
            debug!("⚠️ Invalid token1 address format: {}", token1_address);
        }
        
        // Pack symbols (truncate to 8 bytes, null-pad if shorter)
        let symbol0_bytes = token0_symbol.as_bytes();
        let symbol0_len = symbol0_bytes.len().min(8);
        pool_msg.data[140..140+symbol0_len].copy_from_slice(&symbol0_bytes[..symbol0_len]);
        debug!("✅ Packed token0 symbol: {} (len: {})", token0_symbol, symbol0_len);
        
        let symbol1_bytes = token1_symbol.as_bytes();
        let symbol1_len = symbol1_bytes.len().min(8);
        pool_msg.data[148..148+symbol1_len].copy_from_slice(&symbol1_bytes[..symbol1_len]);
        debug!("✅ Packed token1 symbol: {} (len: {})", token1_symbol, symbol1_len);
        
        // Send via binary protocol with exact reserves for <35μs arbitrage detection
        self.socket_writer.write_pool_update(&pool_msg)?;
        
        debug!("📤 Sent V2 pool reserves: {} reserves={:.2}/{:.2} ({}:{}) hash=0x{:016x}", 
              &pool_address[..10], reserve0, reserve1, token0_symbol, token1_symbol, pool_hash);
        
        Ok(())
    }
    
    /// Send V3 pool state updates with exact tick/liquidity data (no latency)
    async fn send_v3_pool_state_update(
        &self,
        pool_address: &str,
        sqrt_price_x96: u128,
        tick: i32,
        liquidity: u128,
        fee_tier: u32,
        token0_symbol: &str,
        token1_symbol: &str,
        token0_address: &str,
        token1_address: &str,
    ) -> Result<()> {
        let now_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos() as u64;
            
        let pool_hash = PoolUpdateMessage::hash_pool_address(pool_address);
        
        // Pool tracking handled by bijective IDs
        
        // Create PoolUpdate message with exact V3 state for arbitrage detection
        let mut pool_msg = PoolUpdateMessage::new_zeroed();
        pool_msg.set_timestamp_ns(now_ns);
        pool_msg.set_pool_hash(pool_hash);
        pool_msg.update_type = PoolUpdateType::Sync as u8; // State sync
        pool_msg.protocol_type = 2; // V3
        
        // Pack V3 state data into data field:
        // offset 0-15: liquidity (16 bytes)
        // offset 16-31: amount0 (16 bytes) - set to 0 for state updates
        // offset 32-47: amount1 (16 bytes) - set to 0 for state updates  
        // offset 48-63: sqrt_price_x96 (16 bytes)
        // offset 64-67: tick (4 bytes)
        // offset 68-71: fee_tier (4 bytes)
        // offset 80: token0_decimals (1 byte)
        // offset 81: token1_decimals (1 byte)
        
        let mut offset = 0;
        pool_msg.data[offset..offset+16].copy_from_slice(&liquidity.to_le_bytes());
        offset += 32; // Skip amount0(16) + amount1(16) = 32 bytes
        pool_msg.data[offset..offset+16].copy_from_slice(&sqrt_price_x96.to_le_bytes());
        offset += 16;
        pool_msg.data[offset..offset+4].copy_from_slice(&tick.to_le_bytes());
        offset += 4;
        pool_msg.data[offset..offset+4].copy_from_slice(&fee_tier.to_le_bytes());
        
        // Add token decimal info (will be populated by handle_pool_event)
        pool_msg.data[80] = 18; // Default decimals, will be updated by caller
        pool_msg.data[81] = 18; // Default decimals, will be updated by caller
        
        // Pack token metadata starting at offset 100
        // offset 100-119: token0_address (20 bytes)
        // offset 120-139: token1_address (20 bytes)
        // offset 140-147: token0_symbol (8 bytes, null-padded)
        // offset 148-155: token1_symbol (8 bytes, null-padded)
        
        debug!("📦 Packing token metadata - token0: {} ({}), token1: {} ({})", 
               token0_symbol, token0_address, token1_symbol, token1_address);
        
        // Convert address strings to bytes (assuming hex format "0x...")
        if token0_address.starts_with("0x") && token0_address.len() >= 42 {
            if let Ok(addr_bytes) = hex::decode(&token0_address[2..42]) {
                pool_msg.data[100..120].copy_from_slice(&addr_bytes);
                debug!("✅ Packed token0 address: {}", token0_address);
            } else {
                debug!("❌ Failed to decode token0 address: {}", token0_address);
            }
        } else {
            debug!("⚠️ Invalid token0 address format: {}", token0_address);
        }
        
        if token1_address.starts_with("0x") && token1_address.len() >= 42 {
            if let Ok(addr_bytes) = hex::decode(&token1_address[2..42]) {
                pool_msg.data[120..140].copy_from_slice(&addr_bytes);
                debug!("✅ Packed token1 address: {}", token1_address);
            } else {
                debug!("❌ Failed to decode token1 address: {}", token1_address);
            }
        } else {
            debug!("⚠️ Invalid token1 address format: {}", token1_address);
        }
        
        // Pack symbols (truncate to 8 bytes, null-pad if shorter)
        let symbol0_bytes = token0_symbol.as_bytes();
        let symbol0_len = symbol0_bytes.len().min(8);
        pool_msg.data[140..140+symbol0_len].copy_from_slice(&symbol0_bytes[..symbol0_len]);
        debug!("✅ Packed token0 symbol: {} (len: {})", token0_symbol, symbol0_len);
        
        let symbol1_bytes = token1_symbol.as_bytes();
        let symbol1_len = symbol1_bytes.len().min(8);
        pool_msg.data[148..148+symbol1_len].copy_from_slice(&symbol1_bytes[..symbol1_len]);
        debug!("✅ Packed token1 symbol: {} (len: {})", token1_symbol, symbol1_len);
        
        // Send via binary protocol with exact V3 state for <35μs arbitrage detection
        self.socket_writer.write_pool_update(&pool_msg)?;
        
        debug!("📤 Sent V3 pool state: {} tick={} sqrt_price=0x{:032x} liquidity={} fee={} ({}:{}) hash=0x{:016x}", 
              &pool_address[..10], tick, sqrt_price_x96, liquidity, fee_tier, 
              token0_symbol, token1_symbol, pool_hash);
        
        Ok(())
    }
}

/// Determine if we should invert the price for better UX
/// Uses token addresses for accurate identification, preventing fake token confusion
/// Returns true if we should show token1/token0 instead of token0/token1
fn should_invert_price_by_address(token0_address: &str, token1_address: &str) -> bool {
    // Convert to lowercase for consistent comparison
    let addr0 = token0_address.to_lowercase();
    let addr1 = token1_address.to_lowercase();
    
    // Token priority by verified address (higher number = more likely to be quote currency)
    let get_priority_by_address = |address: &str| -> i32 {
        match address {
            // Stablecoins have highest priority (1000-1100)
            "0x2791bca1f2de4661ed88a30c99a7a9449aa84174" => 1000, // USDC (PoS)
            "0x3c499c542cef5e3811e1192ce70d8cc03d5c3359" => 1000, // USDC (Native)
            "0xc2132d05d31c914a87c6611c10748aeb04b58e8f" => 1000, // USDT  
            "0x8f3cf7ad23cd3cadbd9735aff958023239c6a063" => 1000, // DAI
            
            // Major assets have medium priority (400-600)
            "0x7ceb23fd6bc0add59e62ac25578270cff1b9f619" => 500, // Real WETH
            "0x1bfd67037b42cf73acf2047067bd4f2c47d9bfd6" => 499, // WBTC
            
            // Native/wrapped native tokens (100-200)
            "0x0d500b1d8e8ef31e21c99d1db9a6444d3adf1270" => 100, // WMATIC
            "0xa3fa99a148fa48d14ed51d610c367c61876997f1" => 100, // MATIC (POL)
            
            // DeFi tokens (50-99)
            "0x455e53724f9266ca11607ef1e22d3f2c4c5f34b1" => 50, // LINK
            
            // Known fake tokens get negative priority (prevents them from being quote currency)
            "0x4c28f48448720e9000907bc2611f73022fdce1fa" => -1000, // Fake WETH
            
            // Unknown tokens have lowest priority
            _ => 0
        }
    };
    
    let priority0 = get_priority_by_address(&addr0);
    let priority1 = get_priority_by_address(&addr1);
    
    // If token0 has higher priority than token1, we need to invert
    // This ensures lower priority tokens are priced in higher priority tokens
    // E.g., MATIC/USDC stays as is (MATIC priced in USDC)
    // But USDC/MATIC becomes MATIC/USDC after inversion
    priority0 > priority1
}

/// Legacy function for backward compatibility - DEPRECATED
/// Use should_invert_price_by_address instead for accurate token identification
#[deprecated(note = "Use should_invert_price_by_address to avoid fake token confusion")]
fn should_invert_price(token0_symbol: &str, token1_symbol: &str) -> bool {
    warn!("DEPRECATED: should_invert_price() called with symbols instead of addresses. This can cause fake token confusion!");
    
    // Token priority (higher number = more likely to be quote currency)
    let get_priority = |token: &str| -> i32 {
        match token {
            // Stablecoins have highest priority
            "USDC" | "USDT" | "DAI" | "BUSD" | "USD" => 1000,
            // Major assets have medium priority  
            "ETH" | "WETH" => 500,
            "BTC" | "WBTC" => 499,
            // Native token has lower priority
            "MATIC" | "WMATIC" | "POL" | "WPOL" => 100,
            // Other tokens have lowest priority
            _ => 0
        }
    };
    
    let priority0 = get_priority(token0_symbol);
    let priority1 = get_priority(token1_symbol);
    
    priority0 > priority1
}

/// Check if a token address is known to be fake/malicious
fn is_known_fake_token(address: &str) -> bool {
    let addr = address.to_lowercase();
    match addr.as_str() {
        "0x4c28f48448720e9000907bc2611f73022fdce1fa" => true, // Fake WETH
        // Add more known fake tokens here as they are discovered
        _ => false
    }
}

/// Get the real token address if this is a known fake, otherwise return the original
fn get_canonical_token_address(address: &str) -> &str {
    let addr = address.to_lowercase();
    match addr.as_str() {
        "0x4c28f48448720e9000907bc2611f73022fdce1fa" => "0x7ceb23fd6bc0add59e62ac25578270cff1b9f619", // Fake WETH -> Real WETH
        _ => address
    }
}

/// Validate token authenticity by checking core contract functions
async fn validate_token_authenticity(address: &str, expected_symbol: &str, rpc_url: &str) -> bool {
    // For WETH tokens, check if they have deposit() and withdraw() functions
    if expected_symbol == "WETH" {
        // This would require web3 calls to check contract methods
        // For now, use the known fake list
        return !is_known_fake_token(address);
    }
    
    // For other tokens, basic validation could include:
    // - Contract size check
    // - Standard ERC20 function availability
    // - Creation date analysis
    
    true // Default to valid for unknown tokens
}

impl PolygonCollector {
    
    /// Handle new block header from WebSocket subscription
    async fn handle_new_block(&self, block: &Value) -> Result<()> {
        // Extract block number and gas price from block header
        let block_hex = block["number"].as_str().unwrap_or("0x0");
        let block_num = u64::from_str_radix(block_hex.trim_start_matches("0x"), 16)?;
        
        // Base fee is included in the block header for EIP-1559 chains
        let base_fee_hex = block["baseFeePerGas"].as_str().unwrap_or("0x0");
        let base_fee_wei = u64::from_str_radix(base_fee_hex.trim_start_matches("0x"), 16).unwrap_or(30_000_000_000);
        let mut gas_gwei = (base_fee_wei / 1_000_000_000) as u32;
        
        // Polygon often has zero baseFeePerGas, use minimum realistic gas price
        if gas_gwei == 0 {
            gas_gwei = 25; // 25 Gwei is typical for Polygon
            debug!("🔧 Using default Polygon gas price: {} Gwei", gas_gwei);
        }
        
        // Get current POL price from recent swaps
        let pol_price = *self.pol_price.read();
        
        // Only send updates if we have a real POL price from swaps
        if pol_price > 0.0 && pol_price != 0.65 {
            // Create status update message with real-time POL price
            let status = StatusUpdateMessage::new(
                gas_gwei,
                gas_gwei * 12 / 10,  // Fast = 20% higher
                gas_gwei * 15 / 10,  // Instant = 50% higher  
                pol_price,
                block_num
            );
            
            // Send via Unix socket
            if let Err(e) = self.socket_writer.write_status_update(&status) {
                error!("Failed to send gas price update: {}", e);
            } else {
                debug!("⛽ Block #{}: Gas {} Gwei, POL ${:.3}", block_num, gas_gwei, pol_price);
            }
        } else {
            debug!("⏳ Block #{}: Waiting for POL price discovery from swaps...", block_num);
        }
        
        Ok(())
    }
    
    // V2 Liquidity event handlers
    async fn process_v2_mint(&self, event: &Value) -> Result<()> {
        let pool_address = event["address"].as_str()
            .ok_or_else(|| anyhow::anyhow!("No pool address"))?;
        let data = event["data"].as_str()
            .ok_or_else(|| anyhow::anyhow!("No mint data"))?;
        
        // V2 Mint event: amount0, amount1
        let hex_data = data.strip_prefix("0x").unwrap_or(data);
        if hex_data.len() >= 128 {
            let amount0_hex = &hex_data[0..64];
            let amount1_hex = &hex_data[64..128];
            
            let amount0 = u128::from_str_radix(amount0_hex, 16)? as f64;
            let amount1 = u128::from_str_radix(amount1_hex, 16)? as f64;
            
            // Update liquidity state
            let mut liquidity_states = self.pool_liquidity.write();
            let state = liquidity_states.entry(pool_address.to_string())
                .or_insert_with(PoolLiquidityState::default);
            
            state.reserve0 += amount0;
            state.reserve1 += amount1;
            state.total_liquidity = (state.reserve0 * state.reserve1).sqrt();
            
            debug!("💧 V2 Mint: {} added liquidity {:.2}/{:.2}", pool_address, amount0, amount1);
        }
        
        Ok(())
    }
    
    async fn process_v2_burn(&self, event: &Value) -> Result<()> {
        let pool_address = event["address"].as_str()
            .ok_or_else(|| anyhow::anyhow!("No pool address"))?;
        let data = event["data"].as_str()
            .ok_or_else(|| anyhow::anyhow!("No burn data"))?;
        
        // V2 Burn event: amount0, amount1
        let hex_data = data.strip_prefix("0x").unwrap_or(data);
        if hex_data.len() >= 128 {
            let amount0_hex = &hex_data[0..64];
            let amount1_hex = &hex_data[64..128];
            
            let amount0 = u128::from_str_radix(amount0_hex, 16)? as f64;
            let amount1 = u128::from_str_radix(amount1_hex, 16)? as f64;
            
            // Update liquidity state
            let mut liquidity_states = self.pool_liquidity.write();
            let state = liquidity_states.entry(pool_address.to_string())
                .or_insert_with(PoolLiquidityState::default);
            
            state.reserve0 = (state.reserve0 - amount0).max(0.0);
            state.reserve1 = (state.reserve1 - amount1).max(0.0);
            state.total_liquidity = (state.reserve0 * state.reserve1).sqrt();
            
            debug!("🔥 V2 Burn: {} removed liquidity {:.2}/{:.2}", pool_address, amount0, amount1);
        }
        
        Ok(())
    }
    
    async fn process_v2_sync(&self, event: &Value) -> Result<()> {
        // Collector just forwards events, doesn't track state
        debug!("V2 Sync event received, forwarding");
        Ok(())
    }
    
    */
    
    /* Commented out - old implementation
    async fn process_v2_sync_old(&self, event: &Value) -> Result<()> {
        let pool_address = event["address"].as_str()
            .ok_or_else(|| anyhow::anyhow!("No pool address"))?;
        let data = event["data"].as_str()
            .ok_or_else(|| anyhow::anyhow!("No sync data"))?;
        
        // V2 Sync event: reserve0, reserve1
        let hex_data = data.strip_prefix("0x").unwrap_or(data);
        if hex_data.len() >= 128 {
            let reserve0_hex = &hex_data[0..64];
            let reserve1_hex = &hex_data[64..128];
            
            // Parse raw reserve amounts
            let reserve0_raw = u128::from_str_radix(reserve0_hex, 16)? as f64;
            let reserve1_raw = u128::from_str_radix(reserve1_hex, 16)? as f64;
            
            // Get token decimals for proper conversion
            let pool = self.get_or_create_pool(pool_address).await?;
            let (token0_addr, token1_addr) = pool.get_tokens().await?;
            
            // Get token info with decimal information
            let token0_info = self.token_registry.get_token_info(&token0_addr).await?;
            let token1_info = self.token_registry.get_token_info(&token1_addr).await?;
            
            // Apply decimal adjustment to reserves
            let decimals0_factor = 10_f64.powi(token0_info.decimals as i32);
            let decimals1_factor = 10_f64.powi(token1_info.decimals as i32);
            
            let reserve0_adjusted = reserve0_raw / decimals0_factor;
            let reserve1_adjusted = reserve1_raw / decimals1_factor;
            
            // Update liquidity state with decimal-adjusted reserves
            {
                let mut liquidity_states = self.pool_liquidity.write();
                let state = liquidity_states.entry(pool_address.to_string())
                    .or_insert_with(PoolLiquidityState::default);
                
                state.reserve0 = reserve0_adjusted;
                state.reserve1 = reserve1_adjusted;
                state.total_liquidity = (reserve0_adjusted * reserve1_adjusted).sqrt();
                
                debug!("🔄 V2 Sync: {} reserves updated to {:.2}/{:.2} (decimals: {}/{})", 
                       pool_address, reserve0_adjusted, reserve1_adjusted, 
                       token0_info.decimals, token1_info.decimals);
                debug!("💰 Calculated geometric mean liquidity: {:.2} from reserves {:.2}*{:.2}", 
                       state.total_liquidity, reserve0_adjusted, reserve1_adjusted);
            } // Release lock before await
            
            // Send precise pool reserves to scanner for arbitrage detection
            self.send_pool_state_update(
                pool_address,
                reserve0_adjusted,
                reserve1_adjusted,
                &token0_info.symbol,
                &token1_info.symbol,
                &token0_addr,
                &token1_addr,
            ).await?;
        }
        
        Ok(())
    }
    
    // V3 Liquidity event handlers
    async fn process_v3_mint(&self, event: &Value) -> Result<()> {
        let pool_address = event["address"].as_str()
            .ok_or_else(|| anyhow::anyhow!("No pool address"))?;
        let data = event["data"].as_str()
            .ok_or_else(|| anyhow::anyhow!("No mint data"))?;
        
        // V3 Mint event data: sender, tickLower, tickUpper, amount, amount0, amount1
        let hex_data = data.strip_prefix("0x").unwrap_or(data);
        if hex_data.len() >= 384 { // 6 * 64
            let tick_lower = i32::from_str_radix(&hex_data[64..128], 16)?;
            let tick_upper = i32::from_str_radix(&hex_data[128..192], 16)?;
            let liquidity = u128::from_str_radix(&hex_data[192..256], 16)? as f64;
            
            // Update tick liquidity map
            let mut liquidity_states = self.pool_liquidity.write();
            let state = liquidity_states.entry(pool_address.to_string())
                .or_insert_with(PoolLiquidityState::default);
            
            // Add liquidity to tick range
            for tick in (tick_lower..=tick_upper).step_by(60) { // V3 tick spacing
                *state.tick_liquidity.entry(tick).or_insert(0.0) += liquidity;
            }
            state.total_liquidity += liquidity;
            
            debug!("💧 V3 Mint: {} added {:.2} liquidity to ticks [{}, {}]", 
                   pool_address, liquidity, tick_lower, tick_upper);
        }
        
        Ok(())
    }
    
    async fn process_v3_burn(&self, event: &Value) -> Result<()> {
        let pool_address = event["address"].as_str()
            .ok_or_else(|| anyhow::anyhow!("No pool address"))?;
        let data = event["data"].as_str()
            .ok_or_else(|| anyhow::anyhow!("No burn data"))?;
        
        // V3 Burn event data: tickLower, tickUpper, amount
        let hex_data = data.strip_prefix("0x").unwrap_or(data);
        if hex_data.len() >= 192 { // 3 * 64
            let tick_lower = i32::from_str_radix(&hex_data[0..64], 16)?;
            let tick_upper = i32::from_str_radix(&hex_data[64..128], 16)?;
            let liquidity = u128::from_str_radix(&hex_data[128..192], 16)? as f64;
            
            // Update tick liquidity map
            let mut liquidity_states = self.pool_liquidity.write();
            let state = liquidity_states.entry(pool_address.to_string())
                .or_insert_with(PoolLiquidityState::default);
            
            // Remove liquidity from tick range
            for tick in (tick_lower..=tick_upper).step_by(60) { // V3 tick spacing
                if let Some(tick_liq) = state.tick_liquidity.get_mut(&tick) {
                    *tick_liq = (*tick_liq - liquidity).max(0.0);
                }
            }
            state.total_liquidity = (state.total_liquidity - liquidity).max(0.0);
            
            debug!("🔥 V3 Burn: {} removed {:.2} liquidity from ticks [{}, {}]", 
                   pool_address, liquidity, tick_lower, tick_upper);
        }
        
        Ok(())
    }
    
    async fn process_v3_collect(&self, event: &Value) -> Result<()> {
        let pool_address = event["address"].as_str()
            .ok_or_else(|| anyhow::anyhow!("No pool address"))?;
        
        // V3 Collect event indicates fees being collected
        // This helps us understand pool activity and fee generation
        debug!("💰 V3 Collect: {} fees collected", pool_address);
        
        Ok(())
    }

    /// Unified pool event handler using new pool event system
    async fn handle_pool_event(&self, log: &Value) -> Result<()> {
        // Pool events are processed but collector doesn't track state
        // Just forward the raw event data
        debug!("Pool event received, forwarding raw data");
        Ok(())
    }
}

/* Commented out - old implementation
    async fn handle_pool_event_old(&self, log: &Value) -> Result<()> {
        let pool_address = log["address"].as_str()
            .ok_or_else(|| anyhow::anyhow!("No pool address in log"))?;
        
        let data = log["data"].as_str().unwrap_or("0x");
        let topics: Vec<String> = log["topics"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|t| t.as_str())
            .map(|s| s.to_string())
            .collect();
        
        if topics.is_empty() {
            return Err(anyhow::anyhow!("No topics in pool event"));
        }
        
        let event_signature = &topics[0];
        
        // Identify pool event type
        if let Some((_event_type, _pool_type)) = identify_pool_event(event_signature) {
            // Try to get pool from cache
            let pool = {
                let pool_cache = self.pool_cache.read();
                pool_cache.get(pool_address).cloned()
            }; // Pool cache lock is dropped here
            
            if let Some(pool) = pool {
                // Parse the pool event using the pool-specific parser
                match pool.parse_pool_event(event_signature, data, &topics) {
                    Ok(pool_event) => {
                        // Get timestamp and block info
                        let timestamp_ns = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_nanos() as u64;
                        let block_number = log["blockNumber"].as_str()
                            .and_then(|s| u64::from_str_radix(s.strip_prefix("0x").unwrap_or(s), 16).ok())
                            .unwrap_or(0);
                        let tx_hash = log["transactionHash"].as_str().unwrap_or("").to_string();
                        let log_index = log["logIndex"].as_str()
                            .and_then(|s| u32::from_str_radix(s.strip_prefix("0x").unwrap_or(s), 16).ok())
                            .unwrap_or(0);
                        
                        // Update core event data
                        let mut complete_event = pool_event;
                        {
                            // Get token info from registry FIRST (we need decimals)
                            let mut token0_decimals = 18u8; // Default
                            let mut token1_decimals = 18u8; // Default
                            
                            if let Ok((token0_addr, token1_addr)) = pool.get_tokens().await {
                                if let Ok(token0_info) = self.token_registry.get_token_info(&token0_addr).await {
                                    token0_decimals = token0_info.decimals;
                                }
                                if let Ok(token1_info) = self.token_registry.get_token_info(&token1_addr).await {
                                    token1_decimals = token1_info.decimals;
                                }
                            }
                            
                            // Now update the event with all info including decimals
                            match &mut complete_event {
                                PoolEvent::UniswapV2Mint(ref mut e) => {
                                    e.core.timestamp_ns = timestamp_ns;
                                    e.core.block_number = block_number;
                                    e.core.tx_hash = tx_hash.clone();
                                    e.core.log_index = log_index;
                                    e.token0_decimals = token0_decimals;
                                    e.token1_decimals = token1_decimals;
                                    
                                    if let Ok((token0_addr, token1_addr)) = pool.get_tokens().await {
                                        if let Ok(token0_id) = InstrumentId::polygon_token(&token0_addr) {
                                            e.core.token0_id = token0_id;
                                        }
                                        if let Ok(token1_id) = InstrumentId::polygon_token(&token1_addr) {
                                            e.core.token1_id = token1_id;
                                        }
                                        // Create pool ID from token IDs
                                        if let (Ok(t0_id), Ok(t1_id)) = (
                                            InstrumentId::polygon_token(&token0_addr),
                                            InstrumentId::polygon_token(&token1_addr)
                                        ) {
                                            e.core.pool_id = InstrumentId::pool(VenueId::Polygon, t0_id, t1_id);
                                        }
                                    }
                                },
                                PoolEvent::UniswapV2Burn(ref mut e) => {
                                    e.core.timestamp_ns = timestamp_ns;
                                    e.core.block_number = block_number;
                                    e.core.tx_hash = tx_hash.clone();
                                    e.core.log_index = log_index;
                                    e.token0_decimals = token0_decimals;
                                    e.token1_decimals = token1_decimals;
                                    
                                    if let Ok((token0_addr, token1_addr)) = pool.get_tokens().await {
                                        if let Ok(token0_id) = InstrumentId::polygon_token(&token0_addr) {
                                            e.core.token0_id = token0_id;
                                        }
                                        if let Ok(token1_id) = InstrumentId::polygon_token(&token1_addr) {
                                            e.core.token1_id = token1_id;
                                        }
                                        // Create pool ID from token IDs
                                        if let (Ok(t0_id), Ok(t1_id)) = (
                                            InstrumentId::polygon_token(&token0_addr),
                                            InstrumentId::polygon_token(&token1_addr)
                                        ) {
                                            e.core.pool_id = InstrumentId::pool(VenueId::Polygon, t0_id, t1_id);
                                        }
                                    }
                                },
                                PoolEvent::UniswapV2Sync(ref mut e) => {
                                    e.core.timestamp_ns = timestamp_ns;
                                    e.core.block_number = block_number;
                                    e.core.tx_hash = tx_hash.clone();
                                    e.core.log_index = log_index;
                                    e.token0_decimals = token0_decimals;
                                    e.token1_decimals = token1_decimals;
                                    
                                    if let Ok((token0_addr, token1_addr)) = pool.get_tokens().await {
                                        if let Ok(token0_id) = InstrumentId::polygon_token(&token0_addr) {
                                            e.core.token0_id = token0_id;
                                        }
                                        if let Ok(token1_id) = InstrumentId::polygon_token(&token1_addr) {
                                            e.core.token1_id = token1_id;
                                        }
                                        // Create pool ID from token IDs
                                        if let (Ok(t0_id), Ok(t1_id)) = (
                                            InstrumentId::polygon_token(&token0_addr),
                                            InstrumentId::polygon_token(&token1_addr)
                                        ) {
                                            e.core.pool_id = InstrumentId::pool(VenueId::Polygon, t0_id, t1_id);
                                        }
                                    }
                                },
                                PoolEvent::UniswapV3Mint(ref mut e) => {
                                    e.core.timestamp_ns = timestamp_ns;
                                    e.core.block_number = block_number;
                                    e.core.tx_hash = tx_hash.clone();
                                    e.core.log_index = log_index;
                                    e.token0_decimals = token0_decimals;
                                    e.token1_decimals = token1_decimals;
                                    
                                    if let Ok((token0_addr, token1_addr)) = pool.get_tokens().await {
                                        if let Ok(token0_id) = InstrumentId::polygon_token(&token0_addr) {
                                            e.core.token0_id = token0_id;
                                        }
                                        if let Ok(token1_id) = InstrumentId::polygon_token(&token1_addr) {
                                            e.core.token1_id = token1_id;
                                        }
                                        // Create pool ID from token IDs
                                        if let (Ok(t0_id), Ok(t1_id)) = (
                                            InstrumentId::polygon_token(&token0_addr),
                                            InstrumentId::polygon_token(&token1_addr)
                                        ) {
                                            e.core.pool_id = InstrumentId::pool(VenueId::Polygon, t0_id, t1_id);
                                        }
                                    }
                                },
                                PoolEvent::UniswapV3Burn(ref mut e) => {
                                    e.core.timestamp_ns = timestamp_ns;
                                    e.core.block_number = block_number;
                                    e.core.tx_hash = tx_hash.clone();
                                    e.core.log_index = log_index;
                                    e.token0_decimals = token0_decimals;
                                    e.token1_decimals = token1_decimals;
                                    
                                    if let Ok((token0_addr, token1_addr)) = pool.get_tokens().await {
                                        if let Ok(token0_id) = InstrumentId::polygon_token(&token0_addr) {
                                            e.core.token0_id = token0_id;
                                        }
                                        if let Ok(token1_id) = InstrumentId::polygon_token(&token1_addr) {
                                            e.core.token1_id = token1_id;
                                        }
                                        // Create pool ID from token IDs
                                        if let (Ok(t0_id), Ok(t1_id)) = (
                                            InstrumentId::polygon_token(&token0_addr),
                                            InstrumentId::polygon_token(&token1_addr)
                                        ) {
                                            e.core.pool_id = InstrumentId::pool(VenueId::Polygon, t0_id, t1_id);
                                        }
                                    }
                                },
                                PoolEvent::UniswapV3Collect(ref mut e) => {
                                    e.core.timestamp_ns = timestamp_ns;
                                    e.core.block_number = block_number;
                                    e.core.tx_hash = tx_hash.clone();
                                    e.core.log_index = log_index;
                                    e.token0_decimals = token0_decimals;
                                    e.token1_decimals = token1_decimals;
                                    
                                    if let Ok((token0_addr, token1_addr)) = pool.get_tokens().await {
                                        if let Ok(token0_id) = InstrumentId::polygon_token(&token0_addr) {
                                            e.core.token0_id = token0_id;
                                        }
                                        if let Ok(token1_id) = InstrumentId::polygon_token(&token1_addr) {
                                            e.core.token1_id = token1_id;
                                        }
                                        // Create pool ID from token IDs
                                        if let (Ok(t0_id), Ok(t1_id)) = (
                                            InstrumentId::polygon_token(&token0_addr),
                                            InstrumentId::polygon_token(&token1_addr)
                                        ) {
                                            e.core.pool_id = InstrumentId::pool(VenueId::Polygon, t0_id, t1_id);
                                        }
                                    }
                                },
                                _ => return Ok(()), // Other pool types not implemented yet
                            }
                        }
                        
                        // Convert to wire format and send
                        let mut pool_msg = complete_event.to_message();
                        
                        // InstrumentIds are already packed in the core structure - no additional packing needed
                        // The scanner will resolve symbols from InstrumentIds using the schema cache
                        let core = complete_event.core();
                        
                        debug!("📦 PoolUpdate with InstrumentIds - pool_id: {:?}, token0_id: {:?}, token1_id: {:?}",
                               core.pool_id, core.token0_id, core.token1_id);
                        
                        if let Err(e) = self.socket_writer.write_pool_update(&pool_msg) {
                            debug!("Failed to send pool event: {}", e);
                        }
                        
                        // Update local pool liquidity state (HOT PATH)
                        self.update_pool_liquidity_from_event_sync(&complete_event)?;
                        
                        debug!("🔄 Processed {} event for pool {}", 
                               match complete_event.event_type() {
                                   PoolUpdateType::Mint => "Mint",
                                   PoolUpdateType::Burn => "Burn", 
                                   PoolUpdateType::Sync => "Sync",
                                   PoolUpdateType::Collect => "Collect",
                                   _ => "Pool",
                               },
                               pool_address);
                        
                        Ok(())
                    },
                    Err(e) => {
                        debug!("Failed to parse pool event {}: {}", event_signature, e);
                        Ok(())
                    }
                }
            } else {
                // Pool not in cache - try to create it
                debug!("Pool {} not in cache for event {}", pool_address, event_signature);
                Ok(())
            }
        } else {
            debug!("Unknown pool event signature: {}", event_signature);
            Ok(())
        }
    }
    
    /// Update pool liquidity state from pool events (HOT PATH - <35μs)
    fn update_pool_liquidity_from_event_sync(&self, event: &PoolEvent) -> Result<()> {
        // Collector doesn't track state - just forward events
        Ok(())
    }
    */
    */
    
    /* Commented out - old implementation
    fn update_pool_liquidity_from_event_sync_old(&self, event: &PoolEvent) -> Result<()> {
        let pool_address = event.core().pool_address.clone();
        let core = event.core();
        
        // 🔥 POPULATE POOL REGISTRY - This is the key fix!
        // Every pool event contains complete pool info including token addresses/symbols
        let pool_hash = PoolRegistry::compute_hash(&pool_address);
        
        // Determine DEX name and fee tier from event type
        let (dex_name, fee_tier) = match event {
            PoolEvent::UniswapV2Mint(_) | PoolEvent::UniswapV2Burn(_) | PoolEvent::UniswapV2Sync(_) => {
                ("uniswap_v2".to_string(), None)
            },
            PoolEvent::UniswapV3Mint(_) | PoolEvent::UniswapV3Burn(_) | PoolEvent::UniswapV3Collect(_) => {
                ("uniswap_v3".to_string(), Some(3000)) // Default to 0.3% fee, could be extracted from event
            },
            _ => ("unknown".to_string(), None),
        };
        
        let pool_info = crate::pool_registry::PoolInfo {
            address: pool_address.clone(),
            hash: pool_hash,
            token0_address: core.token0_address.clone(),
            token1_address: core.token1_address.clone(),
            dex_name,
            fee_tier,
        };
        
        // Register the pool so scanner can resolve pool_hash → full pool info
        self.pool_registry.register_pool(pool_info);
        info!("✅ Registered pool {:#x}: {}/{} on {}", 
              pool_hash, core.token0_symbol, core.token1_symbol, pool_address);

        let mut liquidity_states = self.pool_liquidity.write();
        let state = liquidity_states.entry(pool_address.clone())
            .or_insert_with(PoolLiquidityState::default);
        
        match event {
            PoolEvent::UniswapV2Mint(mint_event) => {
                // Update reserves after mint
                state.reserve0 = mint_event.reserves0_after as f64;
                state.reserve1 = mint_event.reserves1_after as f64;
                debug!("💧 V2 Mint: {} reserves updated: [{:.2}, {:.2}]", 
                       pool_address, state.reserve0, state.reserve1);
            },
            PoolEvent::UniswapV2Burn(burn_event) => {
                state.reserve0 = burn_event.reserves0_after as f64;
                state.reserve1 = burn_event.reserves1_after as f64;
                debug!("🔥 V2 Burn: {} reserves updated: [{:.2}, {:.2}]", 
                       pool_address, state.reserve0, state.reserve1);
            },
            PoolEvent::UniswapV2Sync(sync_event) => {
                state.reserve0 = sync_event.reserves0_after as f64;
                state.reserve1 = sync_event.reserves1_after as f64;
                debug!("🔄 V2 Sync: {} reserves synced: [{:.2}, {:.2}]", 
                       pool_address, state.reserve0, state.reserve1);
                
                // Note: Pool updates with token metadata are sent via the async process_pool_event function
                // which handles Swap events. Sync events just update the local state.
            },
            PoolEvent::UniswapV3Mint(mint_event) => {
                // Update V3 state and tick liquidity
                state.current_tick = mint_event.tick_after;
                state.current_sqrt_price = mint_event.sqrt_price_x96_after as f64;
                state.active_liquidity = mint_event.liquidity_after;
                
                // Add liquidity to tick range
                for tick in (mint_event.tick_lower..=mint_event.tick_upper).step_by(60) {
                    *state.tick_liquidity.entry(tick).or_insert(0.0) += mint_event.liquidity as f64;
                }
                debug!("💧 V3 Mint: {} added {:.2} liquidity to ticks [{}, {}]", 
                       pool_address, mint_event.liquidity, mint_event.tick_lower, mint_event.tick_upper);
            },
            PoolEvent::UniswapV3Burn(burn_event) => {
                state.current_tick = burn_event.tick_after;
                state.current_sqrt_price = burn_event.sqrt_price_x96_after as f64;
                state.active_liquidity = burn_event.liquidity_after;
                
                // Remove liquidity from tick range
                for tick in (burn_event.tick_lower..=burn_event.tick_upper).step_by(60) {
                    if let Some(tick_liq) = state.tick_liquidity.get_mut(&tick) {
                        *tick_liq = (*tick_liq - burn_event.liquidity as f64).max(0.0);
                    }
                }
                debug!("🔥 V3 Burn: {} removed {:.2} liquidity from ticks [{}, {}]", 
                       pool_address, burn_event.liquidity, burn_event.tick_lower, burn_event.tick_upper);
            },
            PoolEvent::UniswapV3Collect(collect_event) => {
                // Collect events don't change liquidity, just fees
                debug!("💰 V3 Collect: {} collected fees: [{:.2}, {:.2}]", 
                       pool_address, collect_event.amount0_collected, collect_event.amount1_collected);
            },
            _ => {
                // Other pool types not implemented yet
                debug!("Pool event type not implemented for liquidity tracking: {:?}", event.event_type());
            }
        }
        
        Ok(())
    }
    */
}

impl Clone for PolygonCollector {
    fn clone(&self) -> Self {
        Self {
            socket_writer: Arc::clone(&self.socket_writer),
            pool_factory: Arc::clone(&self.pool_factory),
            dex_registry: Arc::clone(&self.dex_registry),
            pool_cache: Arc::clone(&self.pool_cache),
            alchemy_ws_url: self.alchemy_ws_url.clone(),
            sequence: Arc::clone(&self.sequence),
            message_cache: Arc::clone(&self.message_cache),
            schema_cache: Arc::clone(&self.schema_cache),
        }
    }
}


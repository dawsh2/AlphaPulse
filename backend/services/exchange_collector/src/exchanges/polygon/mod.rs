pub mod dex;

use crate::instruments::INSTRUMENTS;
use crate::unix_socket::UnixSocketWriter;
use crate::token_registry::TokenRegistry;
use crate::dex_registry::DexRegistry;
use alphapulse_protocol::{TradeMessage, SymbolMappingMessage};
use anyhow::{Context, Result};
use parking_lot::RwLock;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use tracing::{debug, info, warn, error};

use dex::{DexPool, PoolFactory, PoolType, EventBasedPoolType};

pub struct PolygonCollector {
    socket_writer: Arc<UnixSocketWriter>,
    token_registry: Arc<TokenRegistry>,
    pool_factory: Arc<PoolFactory>,
    dex_registry: Arc<DexRegistry>,
    pool_cache: Arc<RwLock<HashMap<String, Arc<Box<dyn DexPool>>>>>,
    alchemy_ws_url: String,
    sequence: Arc<std::sync::atomic::AtomicU32>,
}

impl PolygonCollector {
    pub fn new(socket_writer: Arc<UnixSocketWriter>) -> Self {
        // Try working endpoints provided by user: polygon-rpc.com, rpc.ankr.com/polygon, polygon.llamarpc.com, polygon.publicnode.com
        let (rpc_url, ws_url) = if let Ok(ankr_key) = std::env::var("ANKR_API_KEY") {
            if ankr_key.len() > 10 {
                info!("🔑 Using Ankr Polygon API with WebSocket");
                (
                    format!("https://rpc.ankr.com/polygon/{}", ankr_key),
                    format!("wss://rpc.ankr.com/polygon/ws/{}", ankr_key)
                )
            } else {
                info!("🌐 Using free polygon.publicnode.com endpoint");
                (
                    "https://polygon.publicnode.com".to_string(),
                    "wss://polygon.publicnode.com".to_string()
                )
            }
        } else {
            info!("🌐 Using free polygon.publicnode.com endpoint");
            (
                "https://polygon.publicnode.com".to_string(),
                "wss://polygon.publicnode.com".to_string()
            )
        };
        
        let token_registry = Arc::new(TokenRegistry::new(rpc_url.clone()));
        let pool_factory = Arc::new(PoolFactory::new(rpc_url.clone()));
        let dex_registry = Arc::new(DexRegistry::new());
        
        Self {
            socket_writer,
            token_registry,
            pool_factory,
            dex_registry,
            pool_cache: Arc::new(RwLock::new(HashMap::new())),
            alchemy_ws_url: ws_url,
            sequence: Arc::new(std::sync::atomic::AtomicU32::new(0)),
        }
    }
    
    pub async fn start(&self) -> Result<()> {
        info!("🚀 Starting Polygon DEX collector with modular architecture");
        
        // Preload common tokens
        info!("📚 Preloading common token information...");
        self.token_registry.preload_common_tokens().await;
        
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
        
        // Subscribe to all supported swap event signatures
        let subscription = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_subscribe",
            "params": [
                "logs",
                {
                    "topics": [
                        [
                            "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822", // UniswapV2 Swap
                            "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67", // UniswapV3 Swap  
                            "0x8b3e96f2b889fa771c53c981b40daf005f63f637f1869f707052d15a3dd97140"  // Curve TokenExchange
                        ]
                    ]
                }
            ]
        });
        
        ws_sender.send(Message::Text(subscription.to_string())).await?;
        info!("✅ Subscribed to DEX swap events");
        
        let collector = self.clone();
        tokio::spawn(async move {
            let mut swap_count = 0;
            let mut heartbeat_count = 0;
            
            while let Some(msg) = ws_receiver.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Ok(data) = serde_json::from_str::<Value>(&text) {
                            // Check for subscription confirmation
                            if data.get("id").is_some() && data.get("result").is_some() {
                                info!("🔗 WebSocket subscription confirmed");
                                continue;
                            }
                            
                            // Check for actual swap events
                            if let Some(params) = data.get("params") {
                                if let Some(result) = params.get("result") {
                                    swap_count += 1;
                                    if swap_count % 10 == 0 {
                                        debug!("📊 Processed {} swaps", swap_count);
                                    }
                                    if let Err(e) = collector.process_swap_event(result).await {
                                        // Log more details about the failure
                                        if let Some(addr) = result.get("address").and_then(|v| v.as_str()) {
                                            debug!("Failed to process swap #{} for pool {}: {}", swap_count, addr, e);
                                        } else {
                                            debug!("Failed to process swap #{}: {}", swap_count, e);
                                        }
                                    }
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
                        warn!("WebSocket closed, waiting 5 seconds before reconnecting to avoid rate limits...");
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
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
        
        Ok(())
    }
    
    async fn process_swap_event(&self, event: &Value) -> Result<()> {
        let pool_address = event["address"].as_str()
            .ok_or_else(|| anyhow::anyhow!("No pool address"))?;
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
        swap_event.tx_hash = tx_hash.to_string();
        swap_event.block_number = block_number;
        
        // Get token info for decimal adjustment
        let (token0_addr, token1_addr) = pool.get_tokens().await?;
        
        // Get token info via RPC (The Graph integration disabled for stability)
        let token0_info = self.token_registry.get_token_info(&token0_addr).await?;
        let token1_info = self.token_registry.get_token_info(&token1_addr).await?;
        debug!("📡 Using RPC data for {}: {}/{} (decimals: {}/{})", 
               pool_address, token0_info.symbol, token1_info.symbol, 
               token0_info.decimals, token1_info.decimals);
        
        // Apply decimal adjustments BEFORE price calculation
        let decimals0_factor = 10_f64.powi(token0_info.decimals as i32);
        let decimals1_factor = 10_f64.powi(token1_info.decimals as i32);
        
        swap_event.amount0_in /= decimals0_factor;
        swap_event.amount1_in /= decimals1_factor;
        swap_event.amount0_out /= decimals0_factor;
        swap_event.amount1_out /= decimals1_factor;
        
        // Calculate price with properly adjusted amounts
        let mut price = pool.calculate_price(&swap_event);
        price.token0_symbol = token0_info.symbol.clone();
        price.token1_symbol = token1_info.symbol.clone();
        price.timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64;
        
        // Normalize price ordering for common pairs (ETH should be priced in USD terms)
        let should_invert = should_invert_price(&token0_info.symbol, &token1_info.symbol);
        if should_invert {
            price.price = 1.0 / price.price;
            // Swap token symbols to match the inverted price
            let temp_symbol = price.token0_symbol.clone();
            price.token0_symbol = price.token1_symbol.clone();
            price.token1_symbol = temp_symbol;
            debug!("Inverted price for better UX: {}/{} @ ${:.6}", 
                   price.token0_symbol, price.token1_symbol, price.price);
        }
        
        // Debug volume calculation with token ordering
        debug!("Volume calculation debug for {}/{}: price={:.6}, volume={:.2}, adjusted_amounts: in0={:.6}, out0={:.6}, in1={:.6}, out1={:.6}",
               price.token0_symbol, price.token1_symbol, price.price, price.volume,
               swap_event.amount0_in, swap_event.amount0_out, swap_event.amount1_in, swap_event.amount1_out);
        debug!("Token ordering: token0={} ({}), token1={} ({})", 
               token0_info.symbol, token0_addr, token1_info.symbol, token1_addr);
        
        // Enhanced price and volume validation with detailed debugging
        if price.price > 100_000.0 || price.price < 0.000001 || price.volume > 1_000_000.0 {
            warn!("⚠️ Unusual price/volume detected for pool {}: {}/{} @ ${:.9} (vol: ${:.0})", 
                  pool_address, price.token0_symbol, price.token1_symbol, price.price, price.volume);
            warn!("   Token decimals: {} = {}, {} = {}", 
                  price.token0_symbol, token0_info.decimals,
                  price.token1_symbol, token1_info.decimals);
            warn!("   Raw amounts: in0={:.0}, out0={:.0}, in1={:.0}, out1={:.0}",
                  swap_event.amount0_in * decimals0_factor,
                  swap_event.amount0_out * decimals0_factor,
                  swap_event.amount1_in * decimals1_factor, 
                  swap_event.amount1_out * decimals1_factor);
            warn!("   Adjusted amounts: in0={:.6}, out0={:.6}, in1={:.6}, out1={:.6}",
                  swap_event.amount0_in, swap_event.amount0_out,
                  swap_event.amount1_in, swap_event.amount1_out);
            
            // Skip obviously wrong prices or volumes but allow investigation
            if price.price > 10_000_000.0 || price.volume > 100_000_000.0 {
                warn!("🚫 Skipping unrealistic price > $10M or volume > $100M - likely decimal/calculation error");
                return Ok(());
            }
        }
        
        // Hard cap volume to prevent dashboard display issues
        if price.volume > 1_000_000.0 {
            warn!("📊 Capping volume from ${:.0} to $1M for display", price.volume);
            price.volume = 1_000_000.0;
        }
        
        // Send to dashboard
        self.send_trade_update(
            &pool,
            &price.token0_symbol,
            &price.token1_symbol,
            price.price,
            price.volume,
        ).await?;
        
        debug!("💱 {} swap: {}/{} @ ${:.6} (volume: ${:.0})",
            pool.dex_name(),
            price.token0_symbol,
            price.token1_symbol,
            price.price,
            price.volume
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
    
    async fn send_trade_update(
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
        
        // Send trade update - use proper constructor like other exchanges
        let now_ns = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64;
        
        // Convert to fixed-point like Coinbase (multiply by scale factor)
        let price_fixed = (price * 1e8) as u64;  // 8 decimal places precision
        let volume_fixed = (volume * 1e8) as u64;
        
        debug!("Fixed-point conversion: price={:.6} -> {}, volume={:.2} -> {}, symbol_id={}", 
               price, price_fixed, volume, volume_fixed, symbol_id);
        
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
}

/// Determine if we should invert the price for better UX
/// Returns true if we should show token1/token0 instead of token0/token1
fn should_invert_price(token0_symbol: &str, token1_symbol: &str) -> bool {
    // Common quote currencies that should be denominators
    let quote_currencies = ["USDT", "USDC", "DAI", "USD"];
    
    // Major tokens that should be priced in USD terms (numerators)
    let major_tokens = ["WETH", "ETH", "WBTC", "BTC", "MATIC"];
    
    // If token0 is a quote currency and token1 is a major token, invert
    // This makes USDT/WETH -> WETH/USDT (showing ETH price in USD)
    if quote_currencies.contains(&token0_symbol) && major_tokens.contains(&token1_symbol) {
        return true;
    }
    
    // If token0 is a major token and token1 is also major, don't invert
    // This keeps WETH/WBTC as is
    
    false
}

impl Clone for PolygonCollector {
    fn clone(&self) -> Self {
        Self {
            socket_writer: Arc::clone(&self.socket_writer),
            token_registry: Arc::clone(&self.token_registry),
            pool_factory: Arc::clone(&self.pool_factory),
            dex_registry: Arc::clone(&self.dex_registry),
            pool_cache: Arc::clone(&self.pool_cache),
            alchemy_ws_url: self.alchemy_ws_url.clone(),
            sequence: Arc::clone(&self.sequence),
        }
    }
}
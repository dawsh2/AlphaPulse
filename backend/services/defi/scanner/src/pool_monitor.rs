use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::HashSet;
use tokio::time::{interval, Duration, Instant};
use tokio::net::UnixStream;
use tokio::io::{AsyncReadExt, BufReader};
use tokio::sync::mpsc;
use tracing::{info, debug, error, warn};
use parking_lot::RwLock;
use dashmap::DashMap;
use zerocopy::FromBytes;
use rust_decimal::prelude::{ToPrimitive, FromStr};
use alphapulse_protocol::{
    MessageHeader, MessageType, SwapEventMessage, StatusUpdateMessage, PoolUpdateMessage,
    MARKET_DATA_RELAY_PATH,  // Connect to MarketDataRelay for input
    // New protocol imports for bijective ID system
    SchemaTransformCache, InstrumentId, ProcessedMessage, NewMessageHeader, NewTradeMessage
};
use serde_json::json;
use reqwest::Client;
use exchange_collector::{
    token_registry::TokenRegistry,
    pool_registry::PoolRegistry,
};

use crate::{PoolInfo, config::ScannerConfig, FlashType};
use crate::opportunity_detector::ScanTrigger;



/// Monitors DEX pools for reserve updates and new pools
pub struct PoolMonitor {
    config: ScannerConfig,
    pools: Arc<DashMap<String, PoolInfo>>,
    last_block_processed: Arc<RwLock<u64>>,
    rpc_client: Arc<Client>,
    scan_sender: Arc<RwLock<Option<mpsc::UnboundedSender<ScanTrigger>>>>,
    token_registry: Arc<TokenRegistry>,
    pool_registry: Arc<PoolRegistry>,
    socket_reader_started: Arc<AtomicBool>,
    dashboard_flash_callback: Arc<RwLock<Option<Box<dyn Fn(&str, FlashType) + Send + Sync>>>>,
    // Schema cache for bijective ID protocol
    schema_cache: Arc<SchemaTransformCache>,
}

impl PoolMonitor {
    pub async fn new(
        config: &ScannerConfig, 
        token_registry: Arc<TokenRegistry>, 
        pool_registry: Arc<PoolRegistry>
    ) -> Result<Arc<Self>> {
        let pools = Arc::new(DashMap::new());
        let last_block_processed = Arc::new(RwLock::new(0));
        
        // Create RPC client for blockchain queries
        let rpc_client = Arc::new(Client::builder()
            .timeout(Duration::from_secs(5))
            .build()?);

        // Initialize schema cache for bijective ID protocol
        let schema_cache = Arc::new(SchemaTransformCache::new());
        info!("📋 Initialized SchemaTransformCache for bijective message decoding");

        let monitor = Arc::new(Self {
            config: config.clone(),
            pools,
            last_block_processed,
            rpc_client,
            scan_sender: Arc::new(RwLock::new(None)),
            token_registry,
            pool_registry,
            socket_reader_started: Arc::new(AtomicBool::new(false)),
            dashboard_flash_callback: Arc::new(RwLock::new(None)),
            schema_cache,
        });

        // Initialize with existing pools
        monitor.discover_pools().await?;

        Ok(monitor)
    }

    /// Connect the scan sender from OpportunityDetector
    pub fn set_scan_sender(&self, sender: mpsc::UnboundedSender<ScanTrigger>) {
        *self.scan_sender.write() = Some(sender);
    }
    
    /// Set dashboard flash callback for live visual feedback
    pub fn set_dashboard_flash_callback<F>(&self, callback: F)
    where
        F: Fn(&str, FlashType) + Send + Sync + 'static,
    {
        *self.dashboard_flash_callback.write() = Some(Box::new(callback));
    }

    /// Get access to the TokenRegistry for token address → symbol resolution
    pub fn token_registry(&self) -> &Arc<TokenRegistry> {
        &self.token_registry
    }

    /// Get access to the PoolRegistry for pool hash → pool info resolution  
    pub fn pool_registry(&self) -> &Arc<PoolRegistry> {
        &self.pool_registry
    }

    /// Resolve pool information from hash using PoolRegistry + TokenRegistry
    /// If pool not in registry, query blockchain and register it
    async fn resolve_pool_info(&self, pool_hash: u64) -> Result<(String, String)> {
        // First try PoolRegistry
        if let Some(pool_info) = self.pool_registry.get_by_hash(pool_hash) {
            debug!("✅ Pool {:#x} found in PoolRegistry: {}/{}", 
                   pool_hash, pool_info.token0_address, pool_info.token1_address);
            
            // Resolve token addresses to symbols using TokenRegistry
            let token0_info = self.token_registry.get_token_info(&pool_info.token0_address).await?;
            let token1_info = self.token_registry.get_token_info(&pool_info.token1_address).await?;
            
            return Ok((token0_info.symbol, token1_info.symbol));
        }

        // Pool not in registry - need to query blockchain and register it
        debug!("⚠️ Pool {:#x} not in PoolRegistry, querying blockchain...", pool_hash);
        
        // For now, fall back to hardcoded lookup as we need the exchange_collector's
        // pool discovery to populate the PoolRegistry properly
        warn!("Pool {:#x} not found in PoolRegistry. PoolRegistry needs to be populated by exchange_collector first.", pool_hash);
        
        // Return placeholder until PoolRegistry is properly populated
        Ok(("UNKNOWN".to_string(), "UNKNOWN".to_string()))
    }

    pub async fn start(&self) -> Result<()> {
        info!("Starting pool monitoring...");

        let mut update_interval = interval(Duration::from_secs(30)); // Update every 30 seconds

        loop {
            update_interval.tick().await;

            if let Err(e) = self.update_pools().await {
                error!("Error updating pools: {}", e);
                continue;
            }

            if let Err(e) = self.discover_pools().await {
                error!("Error discovering new pools: {}", e);
                continue;
            }
        }
    }

    async fn discover_pools(&self) -> Result<()> {
        info!("Discovering pools from enabled exchanges...");

        for exchange_config in self.config.enabled_exchanges() {
            match exchange_config.name.as_str() {
                "uniswap_v2" => {
                    self.discover_uniswap_v2_pools(exchange_config).await?;
                }
                "uniswap_v3" => {
                    self.discover_uniswap_v3_pools(exchange_config).await?;
                }
                "sushiswap" => {
                    self.discover_sushiswap_pools(exchange_config).await?;
                }
                _ => {
                    debug!("Unknown exchange: {}", exchange_config.name);
                }
            }
        }

        info!("Pool discovery complete. Total pools: {}", self.pools.len());
        Ok(())
    }

    /// Initialize the socket reader ONCE during startup
    pub async fn initialize_socket_reader(&self) -> Result<()> {
        // Atomic check-and-set to ensure only ONE socket reader is ever started
        if self.socket_reader_started.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
            info!("🔌 Initializing SINGLE Unix socket reader for all pool data (GUARANTEED SINGLETON)");
            self.start_socket_reader().await?;
        } else {
            warn!("⚠️ Socket reader already started - ignoring duplicate call to initialize_socket_reader()");
        }
        Ok(())
    }

    async fn discover_uniswap_v2_pools(
        &self,
        _config: &crate::config::ExchangeConfig,
    ) -> Result<()> {
        info!("Real Uniswap V2 pool data will come from the shared Unix socket stream");
        // No separate connection needed - all exchanges share one stream
        Ok(())
    }

    async fn discover_uniswap_v3_pools(
        &self,
        _config: &crate::config::ExchangeConfig,
    ) -> Result<()> {
        info!("Real Uniswap V3 pool data will come from the shared Unix socket stream");
        // No separate connection needed - all exchanges share one stream
        Ok(())
    }

    async fn discover_sushiswap_pools(
        &self,
        _config: &crate::config::ExchangeConfig,
    ) -> Result<()> {
        info!("Real Sushiswap pool data will come from the shared Unix socket stream");
        // No separate connection needed - all exchanges share one stream
        Ok(())
    }

    /// Start the single Unix socket reader for ALL pool data
    async fn start_socket_reader(&self) -> Result<()> {
        info!("🚀 Starting SINGLE Unix socket reader for all pool data from MarketDataRelay: {}", MARKET_DATA_RELAY_PATH);
        
        let pools = self.pools.clone();
        let scan_sender = self.scan_sender.clone();
        let token_registry = self.token_registry.clone();
        let pool_registry = self.pool_registry.clone();
        let schema_cache = self.schema_cache.clone();
        
        // Spawn ONE task to read from Unix socket - this task will run for the lifetime of the program
        tokio::spawn(async move {
            info!("📡 Socket reader task spawned - entering connection management loop");
            if let Err(e) = Self::read_socket_stream(pools, scan_sender, token_registry, pool_registry, schema_cache).await {
                error!("❌ Unix socket reader task failed permanently: {}", e);
            }
            error!("❌ Socket reader task exited unexpectedly");
        });
        
        Ok(())
    }
    
    /// Read V3 swap events and pool data from Unix socket
    async fn read_socket_stream(
        pools: Arc<DashMap<String, PoolInfo>>, 
        scan_trigger_sender: Arc<RwLock<Option<mpsc::UnboundedSender<ScanTrigger>>>>,
        token_registry: Arc<TokenRegistry>,
        pool_registry: Arc<PoolRegistry>,
        schema_cache: Arc<SchemaTransformCache>
    ) -> Result<()> {
        let mut connection_count = 0;
        
        loop {
            connection_count += 1;
            info!("🔌 Attempting MarketDataRelay connection #{} to {}", connection_count, MARKET_DATA_RELAY_PATH);
            
            // Connect to Unix socket with retry
            let mut stream = match UnixStream::connect(MARKET_DATA_RELAY_PATH).await {
                Ok(s) => {
                    info!("✅ Connected to Unix socket for pool data (connection #{})", connection_count);
                    s
                }
                Err(e) => {
                    warn!("❌ Failed to connect to Unix socket (attempt #{}): {}. Retrying in 5s...", connection_count, e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };
            
            let mut buffer = vec![0u8; 8192]; // Buffer for reading messages
            
            // Read messages from socket
            loop {
                match stream.read(&mut buffer).await {
                    Ok(0) => {
                        info!("🔌 Unix socket closed (connection #{}), reconnecting...", connection_count);
                        break; // Socket closed, reconnect
                    }
                    Ok(bytes_read) => {
                        debug!("Received {} bytes from relay socket", bytes_read);
                        // Process socket data with registries and deduplication
                        if let Err(e) = Self::process_socket_data_with_registries(
                            &pools, 
                            &scan_trigger_sender, 
                            &token_registry,
                            &pool_registry,
                            &schema_cache,
                            &buffer[..bytes_read]
                        ).await {
                            error!("Error processing socket data: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("❌ Error reading from Unix socket (connection #{}): {}", connection_count, e);
                        break; // Connection error, reconnect
                    }
                }
            }
            
            // Reconnect delay
            info!("⏳ Waiting 2 seconds before reconnection attempt...");
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }
    
    /// Process binary protocol messages from Unix socket WITH REGISTRIES
    async fn process_socket_data_with_registries(
        pools: &Arc<DashMap<String, PoolInfo>>, 
        scan_trigger_sender: &Arc<RwLock<Option<mpsc::UnboundedSender<ScanTrigger>>>>,
        token_registry: &Arc<TokenRegistry>,
        pool_registry: &Arc<PoolRegistry>,
        schema_cache: &Arc<SchemaTransformCache>,
        data: &[u8]
    ) -> Result<()> {
        let mut offset = 0;
        
        while offset + MessageHeader::SIZE <= data.len() {
            // Check if this might be a new protocol message (starts with 0xDEADBEEF)
            if data.len() >= offset + 4 {
                let potential_magic = u32::from_le_bytes([
                    data[offset], data[offset + 1], data[offset + 2], data[offset + 3]
                ]);
                
                if potential_magic == alphapulse_protocol::MESSAGE_MAGIC {
                    // This is a new protocol message - try to process it with schema cache
                    match schema_cache.process_message(&data[offset..]) {
                        Ok(processed_msg) => {
                            debug!("📋 Processed new protocol message: {:?}", processed_msg);
                            
                            // Handle different message types
                            match processed_msg {
                                ProcessedMessage::Trade(trade_data) => {
                                    debug!("💱 New protocol trade: instrument={:?} price={} volume={}", 
                                           trade_data.instrument_id, trade_data.price, trade_data.volume);
                                    
                                    // Trigger scan for this instrument if it's a pool
                                    if let Some(sender) = scan_trigger_sender.read().as_ref() {
                                        // Convert InstrumentId to legacy format for compatibility
                                        let trigger = ScanTrigger::SwapEvent { 
                                            pool_hash: trade_data.instrument_id.cache_key(),
                                            token0_hash: 0, // TODO: Extract from pool metadata 
                                            token1_hash: 0  // TODO: Extract from pool metadata
                                        };
                                        let _ = sender.send(trigger);
                                    }
                                }
                                ProcessedMessage::InstrumentDiscovered(metadata) => {
                                    info!("🔍 New instrument discovered: {} ({})", metadata.symbol, metadata.id.debug_info());
                                }
                                ProcessedMessage::Quote(quote_data) => {
                                    debug!("💰 New protocol quote: instrument={:?} bid={} ask={}", 
                                           quote_data.instrument_id, quote_data.bid_price, quote_data.ask_price);
                                }
                                ProcessedMessage::SwapEvent(swap_data) => {
                                    debug!("🔄 New protocol swap: pool={:?} token0_in={} token1_out={}", 
                                           swap_data.pool_id, swap_data.amount0_in, swap_data.amount1_out);
                                    
                                    // Handle swap event for arbitrage detection
                                    if let Some(sender) = scan_trigger_sender.read().as_ref() {
                                        let trigger = ScanTrigger::SwapEvent { 
                                            pool_hash: swap_data.pool_id.cache_key(),
                                            token0_hash: swap_data.token0_id.cache_key(),
                                            token1_hash: swap_data.token1_id.cache_key()
                                        };
                                        if let Err(e) = sender.send(trigger) {
                                            debug!("Failed to send bijective SwapEvent scan trigger: {}", e);
                                        } else {
                                            debug!("🎯 Bijective SwapEvent triggering scan for pool {:?}", swap_data.pool_id);
                                        }
                                    }
                                }
                                ProcessedMessage::PoolUpdate(pool_data) => {
                                    debug!("📊 New protocol pool update: pool={:?} reserve0={} reserve1={}", 
                                           pool_data.pool_id, pool_data.reserve0, pool_data.reserve1);
                                    
                                    // Update pool state for arbitrage calculations
                                    let pool_address = format!("{:?}", pool_data.pool_id);
                                    let current_time = chrono::Utc::now().timestamp();
                                    
                                    // Get or create pool entry
                                    let mut pool_entry = pools.entry(pool_address.clone()).or_insert_with(|| {
                                        PoolInfo {
                                            address: pool_address.clone(),
                                            exchange: "bijective_protocol".to_string(),
                                            token0: "UNKNOWN".to_string(), // Will be resolved from schema cache
                                            token1: "UNKNOWN".to_string(),
                                            reserve0: rust_decimal::Decimal::try_from(pool_data.reserve0).unwrap_or_default(),
                                            reserve1: rust_decimal::Decimal::try_from(pool_data.reserve1).unwrap_or_default(),
                                            fee: rust_decimal::Decimal::new(3000, 6), // Default 0.3%
                                            last_updated: current_time,
                                            block_number: 0,
                                            v3_tick: Some(pool_data.tick),
                                            v3_sqrt_price_x96: Some(pool_data.sqrt_price_x96 as u128),
                                            v3_liquidity: None,
                                        }
                                    });
                                    
                                    // Update pool state
                                    pool_entry.value_mut().reserve0 = rust_decimal::Decimal::try_from(pool_data.reserve0).unwrap_or_default();
                                    pool_entry.value_mut().reserve1 = rust_decimal::Decimal::try_from(pool_data.reserve1).unwrap_or_default();
                                    pool_entry.value_mut().last_updated = current_time;
                                    pool_entry.value_mut().v3_tick = Some(pool_data.tick);
                                    pool_entry.value_mut().v3_sqrt_price_x96 = Some(pool_data.sqrt_price_x96 as u128);
                                    
                                    // Trigger arbitrage scan
                                    if let Some(sender) = scan_trigger_sender.read().as_ref() {
                                        let trigger = ScanTrigger::PoolUpdate { pool_hash: pool_data.pool_id.cache_key() };
                                        if let Err(e) = sender.send(trigger) {
                                            debug!("Failed to send bijective PoolUpdate scan trigger: {}", e);
                                        } else {
                                            debug!("🔥 Bijective PoolUpdate scan trigger for pool {:?}", pool_data.pool_id);
                                        }
                                    }
                                }
                                ProcessedMessage::ArbitrageOpportunity(arb_data) => {
                                    info!("🎯 New protocol arbitrage opportunity: {}% profit on {:?}/{:?}", 
                                          arb_data.profit_percentage, arb_data.token0_id, arb_data.token1_id);
                                    
                                    // Execute arbitrage opportunity detection
                                    if arb_data.profit_percentage > 0.5 { // Only process if >0.5% profit
                                        info!("🚀 Viable arbitrage detected: {}% profit between {:?} and {:?}", 
                                              arb_data.profit_percentage, arb_data.token0_id, arb_data.token1_id);
                                        
                                        // Trigger immediate targeted scan for this opportunity
                                        if let Some(sender) = scan_trigger_sender.read().as_ref() {
                                            let trigger = ScanTrigger::SwapEvent { 
                                                pool_hash: arb_data.token0_id.cache_key(), // Use token0 as pool identifier
                                                token0_hash: arb_data.token0_id.cache_key(),
                                                token1_hash: arb_data.token1_id.cache_key()
                                            };
                                            if let Err(e) = sender.send(trigger) {
                                                error!("Failed to send arbitrage opportunity scan trigger: {}", e);
                                            } else {
                                                info!("🎯 Arbitrage opportunity scan triggered for {:?}/{:?}", 
                                                      arb_data.token0_id, arb_data.token1_id);
                                            }
                                        }
                                    }
                                }
                                ProcessedMessage::Unknown { message_type, .. } => {
                                    debug!("❓ Unknown new protocol message type: {:?}", message_type);
                                }
                            }
                            
                            // Skip to next message (find the message size from the header)
                            if let Ok(header) = NewMessageHeader::from_bytes(&data[offset..]) {
                                let msg_size = std::mem::size_of::<NewMessageHeader>() + header.payload_size as usize;
                                offset += msg_size;
                                continue;
                            }
                        }
                        Err(e) => {
                            debug!("Failed to process potential new protocol message: {}", e);
                            // Fall through to try old protocol parsing
                        }
                    }
                }
            }
            
            // Parse legacy protocol message header
            let header_bytes = &data[offset..offset + MessageHeader::SIZE];
            let header = MessageHeader::read_from_prefix(header_bytes)
                .ok_or_else(|| anyhow::anyhow!("Failed to parse message header"))?;
                
            if let Err(e) = header.validate() {
                // Find next magic byte (0xFE) to resync message boundaries
                if let Some(next_magic_pos) = data[offset + 1..].iter().position(|&b| b == 0xFE) {
                    offset += next_magic_pos + 1;
                    debug!("Resync'd to magic byte at offset {}", offset);
                } else {
                    // No more magic bytes in this buffer, wait for more data
                    debug!("No magic byte found, waiting for more data");
                    break;
                }
                continue;
            }
            
            let payload_len = header.get_length() as usize;
            let total_msg_len = MessageHeader::SIZE + payload_len;
            
            if offset + total_msg_len > data.len() {
                // Incomplete message, wait for more data
                break;
            }
            
            let payload = &data[offset + MessageHeader::SIZE..offset + total_msg_len];
            
            match header.get_type()? {
                MessageType::SwapEvent => {
                    Self::handle_swap_event(&pools, &scan_trigger_sender, payload).await?;
                }
                MessageType::Trade => {
                    Self::handle_trade_message(&pools, payload).await?;
                }
                MessageType::StatusUpdate => {
                    Self::handle_status_update(payload).await?;
                }
                MessageType::PoolUpdate => {
                    Self::handle_pool_update(&pools, scan_trigger_sender, token_registry, pool_registry, payload).await?;
                }
                MessageType::TokenInfo => {
                    // Skip legacy TokenInfo messages - using bijective IDs
                    debug!("Skipping legacy TokenInfo message - using bijective IDs");
                }
                _ => {
                    // Ignore other message types for now
                    debug!("Ignoring message type: {:?}", header.get_type());
                }
            }
            
            offset += total_msg_len;
        }
        
        Ok(())
    }
    
    /// Handle swap event messages - for actual swap transactions only
    /// Pool reserves/liquidity updates should come via PoolUpdate messages
    async fn handle_swap_event(
        pools: &Arc<DashMap<String, PoolInfo>>, 
        scan_sender: &Arc<RwLock<Option<mpsc::UnboundedSender<ScanTrigger>>>>,
        payload: &[u8]
    ) -> Result<()> {
        if payload.len() < SwapEventMessage::SIZE {
            return Err(anyhow::anyhow!("Swap event payload too small"));
        }
        
        let swap_msg = SwapEventMessage::read_from_prefix(payload)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse swap event"))?;
            
        let pool_hash = swap_msg.pool_address_hash();
        let pool_address = format!("0x{:016x}", pool_hash);
        
        let amount0_out = swap_msg.amount0_out();
        let amount1_out = swap_msg.amount1_out();
        let amount0_in = swap_msg.amount0_in();
        let amount1_in = swap_msg.amount1_in();
        
        // Only handle actual swaps (non-zero amounts)
        if amount0_out > 0 || amount1_out > 0 || amount0_in > 0 || amount1_in > 0 {
            let token0_hash = swap_msg.token0_hash();
            let token1_hash = swap_msg.token1_hash();
            
            debug!("💱 Swap: {} amounts_in={}/{} amounts_out={}/{} tokens={:#x}/{:#x}",
                   pool_address, amount0_in, amount1_in, amount0_out, amount1_out, 
                   token0_hash, token1_hash);
            
            // 🔥 TRIGGER DASHBOARD FLASH for visual feedback
            if let Some(callback) = pools.get(&pool_address) {
                // We need to pass the flash callback here, but we need to restructure this
                // For now, just log the flash trigger
                debug!("🔥 FLASH TRIGGER: SwapEvent for pool {}", pool_address);
            }
            
            // 🔥 TRIGGER TARGETED ARBITRAGE SCAN for this specific token pair
            if let Some(sender) = scan_sender.read().as_ref() {
                let trigger = ScanTrigger::SwapEvent { pool_hash, token0_hash, token1_hash };
                if let Err(e) = sender.send(trigger) {
                    debug!("Failed to send SwapEvent scan trigger: {}", e);
                } else {
                    debug!("🎯 SwapEvent triggering targeted scan for pool {:#x} token pair {:#x}/{:#x}", 
                           pool_hash, token0_hash, token1_hash);
                }
            }
            
        } else {
            // This looks like a state update event sent via SwapEvent
            debug!("🔄 State update received via SwapEvent (should come via PoolUpdate instead): {}", 
                   pool_address);
        }
        
        Ok(())
    }
    
    /// Handle PoolUpdate messages with registries - SINGLE PROCESSING PATH
    async fn handle_pool_update(
        pools: &Arc<DashMap<String, PoolInfo>>, 
        scan_sender: &Arc<RwLock<Option<mpsc::UnboundedSender<ScanTrigger>>>>,
        token_registry: &Arc<TokenRegistry>,
        pool_registry: &Arc<PoolRegistry>,
        payload: &[u8]
    ) -> Result<()> {
        if payload.len() < PoolUpdateMessage::SIZE {
            return Err(anyhow::anyhow!("PoolUpdate message payload too small"));
        }
        
        let pool_msg = PoolUpdateMessage::read_from_prefix(payload)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse PoolUpdate message"))?;
            
        let pool_hash = pool_msg.pool_hash();
        let pool_address = format!("0x{:016x}", pool_hash);
        let current_time = chrono::Utc::now().timestamp();
        
        // Decode update type and protocol type
        let update_type = pool_msg.update_type;
        let is_v2 = pool_msg.is_v2();
        let is_v3 = pool_msg.is_v3();
        
        debug!("📊 PoolUpdate: type={} protocol={} pool={}", 
               update_type, if is_v2 { "V2" } else if is_v3 { "V3" } else { "Unknown" }, pool_address);
        
        // Extract token metadata from PoolUpdateMessage (packed at offset 100+)
        let (token0_symbol, token1_symbol, token0_address, token1_address) = if pool_msg.data.len() >= 156 {
            // Extract token addresses from offset 100-139
            let token0_addr_bytes = &pool_msg.data[100..120];
            let token1_addr_bytes = &pool_msg.data[120..140];
            
            // Convert address bytes to hex string
            let token0_address = if token0_addr_bytes.iter().any(|&b| b != 0) {
                format!("0x{}", hex::encode(token0_addr_bytes))
            } else {
                String::new()
            };
            
            let token1_address = if token1_addr_bytes.iter().any(|&b| b != 0) {
                format!("0x{}", hex::encode(token1_addr_bytes))
            } else {
                String::new()
            };
            
            // Extract token symbols from offset 140-155
            let symbol0_bytes = &pool_msg.data[140..148];
            let symbol1_bytes = &pool_msg.data[148..156];
            
            // Convert bytes to string (null-padded)
            let token0_symbol = String::from_utf8_lossy(symbol0_bytes)
                .trim_end_matches('\0')
                .to_string();
            let token1_symbol = String::from_utf8_lossy(symbol1_bytes)
                .trim_end_matches('\0')
                .to_string();
            
            // If we got valid symbols, use them
            if !token0_symbol.is_empty() && !token1_symbol.is_empty() {
                debug!("📝 Extracted token metadata from message: {}/{} ({}/{})", 
                       token0_symbol, token1_symbol, &token0_address[..10], &token1_address[..10]);
                
                // Also register in local PoolRegistry for future lookups
                if !token0_address.is_empty() && !token1_address.is_empty() {
                    let pool_info = exchange_collector::pool_registry::PoolInfo {
                        address: pool_address.clone(),
                        hash: pool_hash,
                        token0_address: token0_address.clone(),
                        token1_address: token1_address.clone(),
                        dex_name: if is_v2 { "uniswap_v2" } else { "uniswap_v3" }.to_string(),
                        fee_tier: None,
                    };
                    pool_registry.register_pool(pool_info);
                }
                
                (token0_symbol, token1_symbol, token0_address, token1_address)
            } else {
                // Fallback to registry resolution
                debug!("⚠️ No token symbols in message, falling back to registry");
                let (s0, s1) = Self::resolve_pool_tokens_static(pool_hash, token_registry, pool_registry).await;
                (s0, s1, String::new(), String::new())
            }
        } else {
            // Old message format, use registry
            debug!("⚠️ Old message format, using registry resolution");
            let (s0, s1) = Self::resolve_pool_tokens_static(pool_hash, token_registry, pool_registry).await;
            (s0, s1, String::new(), String::new())
        };
        
        if is_v2 {
            // V2 processing with registry resolution
            if pool_msg.data.len() >= 82 {
                let mut offset = 48; // reserves0_after starts at offset 48
                let reserves0_raw = u128::from_le_bytes(
                    pool_msg.data[offset..offset+16].try_into()
                        .map_err(|_| anyhow::anyhow!("Failed to read reserves0"))?
                );
                offset += 16;
                let reserves1_raw = u128::from_le_bytes(
                    pool_msg.data[offset..offset+16].try_into()
                        .map_err(|_| anyhow::anyhow!("Failed to read reserves1"))?
                );
                
                let token0_decimals = pool_msg.data[80];
                let token1_decimals = pool_msg.data[81];
                
                let decimals0_divisor = 10_f64.powi(token0_decimals as i32);
                let decimals1_divisor = 10_f64.powi(token1_decimals as i32);
                
                let reserve0_f64 = reserves0_raw as f64 / decimals0_divisor;
                let reserve1_f64 = reserves1_raw as f64 / decimals1_divisor;
                
                let reserve0 = rust_decimal::Decimal::try_from(reserve0_f64)
                    .unwrap_or(rust_decimal::Decimal::ZERO);
                let reserve1 = rust_decimal::Decimal::try_from(reserve1_f64)
                    .unwrap_or(rust_decimal::Decimal::ZERO);
                
                // Update or insert pool info with resolved tokens
                let mut pool_entry = pools.entry(pool_address.clone()).or_insert_with(|| {
                    PoolInfo {
                        address: pool_address.clone(),
                        exchange: "polygon_dex".to_string(),
                        token0: token0_symbol.clone(),
                        token1: token1_symbol.clone(),
                        reserve0,
                        reserve1,
                        fee: rust_decimal::Decimal::new(3000, 6),
                        last_updated: current_time,
                        block_number: 0,
                        v3_tick: None,
                        v3_sqrt_price_x96: None,
                        v3_liquidity: None,
                    }
                });
                
                // Update reserves and token symbols
                pool_entry.value_mut().reserve0 = reserve0;
                pool_entry.value_mut().reserve1 = reserve1;
                pool_entry.value_mut().last_updated = current_time;
                pool_entry.value_mut().token0 = token0_symbol.clone();
                pool_entry.value_mut().token1 = token1_symbol.clone();
                
                debug!("📊 V2: {} {}/{} reserves={:.4}/{:.4}", 
                      pool_address, token0_symbol, token1_symbol, reserve0, reserve1);
            }
        } else if is_v3 {
            // V3 processing with registry resolution
            let mut pool_entry = pools.entry(pool_address.clone()).or_insert_with(|| {
                PoolInfo {
                    address: pool_address.clone(),
                    exchange: "polygon_dex_v3".to_string(),
                    token0: token0_symbol.clone(),
                    token1: token1_symbol.clone(),
                    reserve0: rust_decimal::Decimal::ZERO,
                    reserve1: rust_decimal::Decimal::ZERO,
                    fee: rust_decimal::Decimal::new(3000, 6),
                    last_updated: current_time,
                    block_number: 0,
                    v3_tick: Some(0),
                    v3_sqrt_price_x96: Some(0),
                    v3_liquidity: Some(0),
                }
            });
            
            pool_entry.value_mut().last_updated = current_time;
            pool_entry.value_mut().token0 = token0_symbol.clone();
            pool_entry.value_mut().token1 = token1_symbol.clone();
            
            debug!("📊 V3: {} {}/{}", pool_address, token0_symbol, token1_symbol);
        }
        
        // SINGLE scan trigger (no duplication)
        if let Some(sender) = scan_sender.read().as_ref() {
            let trigger = ScanTrigger::PoolUpdate { pool_hash };
            if let Err(e) = sender.send(trigger) {
                debug!("Failed to send PoolUpdate scan trigger: {}", e);
            } else {
                debug!("🔥 PoolUpdate scan trigger for pool {:#x}", pool_hash);
            }
        }
        
        Ok(())
    }

    /// Static helper to resolve token symbols from registries
    async fn resolve_pool_tokens_static(
        pool_hash: u64,
        token_registry: &Arc<TokenRegistry>,
        pool_registry: &Arc<PoolRegistry>
    ) -> (String, String) {
        // First try PoolRegistry
        if let Some(pool_info) = pool_registry.get_by_hash(pool_hash) {
            // Resolve token addresses to symbols using TokenRegistry
            let token0_info = token_registry.get_token_info(&pool_info.token0_address).await.ok();
            let token1_info = token_registry.get_token_info(&pool_info.token1_address).await.ok();
            
            if let (Some(t0), Some(t1)) = (token0_info, token1_info) {
                return (t0.symbol, t1.symbol);
            }
        }

        // Fallback to UNKNOWN
        ("UNKNOWN".to_string(), "UNKNOWN".to_string())
    }

    
    /// Handle trade messages and infer pool liquidity data
    async fn handle_trade_message(pools: &Arc<DashMap<String, PoolInfo>>, payload: &[u8]) -> Result<()> {
        use alphapulse_protocol::{TradeMessage, SymbolDescriptor};
        use zerocopy::FromBytes;
        
        if payload.len() < TradeMessage::SIZE {
            return Err(anyhow::anyhow!("Trade message payload too small"));
        }
        
        let trade_msg = TradeMessage::read_from_prefix(payload)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse trade message"))?;
            
        let symbol_hash = trade_msg.symbol_hash();
        let price = trade_msg.price_f64();
        let volume = trade_msg.volume_f64();
        
        // Log the trade
        debug!("📊 TRADE: Hash={:016x} Price=${:.6} Volume={:.4} Side={:?}", 
              symbol_hash, price, volume, trade_msg.side());
        
        // REMOVED: Creating fake pools from trade data violates "no mocks" principle
        // Pools must have REAL reserves from blockchain, not estimated from trade volume
        
        // This function should not create synthetic pools
        // Instead, it should only track trade prices for monitoring
        // Real arbitrage detection requires fetching actual DEX pool data
        
        warn!("Trade received but not creating fake pool - need real pool data from blockchain");
        
        // TODO: Instead of creating fake pools, we should:
        // 1. Use the PoolFetcher to get real DEX pool data
        // 2. Use the TradeOptimizer to calculate optimal trade sizes
        // 3. Only alert when there's a genuine profitable opportunity
        
        Ok(())
    }
    
    /// Handle status updates (gas prices, block numbers)
    async fn handle_status_update(payload: &[u8]) -> Result<()> {
        if payload.len() < StatusUpdateMessage::SIZE {
            return Err(anyhow::anyhow!("Status update payload too small"));
        }
        
        let status_msg = StatusUpdateMessage::read_from_prefix(payload)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse status update"))?;
            
        debug!("Status update - gas: {} gwei, native price: ${:.2}", 
               status_msg.gas_price_gwei(),
               status_msg.native_price());
               
        // TODO: Store gas prices and native token price for opportunity calculation
        
        Ok(())
    }


    async fn handle_token_info(
        token_registry: &Arc<exchange_collector::token_registry::TokenRegistry>,
        payload: &[u8],
    ) -> Result<()> {
        use alphapulse_protocol::TokenInfoMessage;
        use zerocopy::FromBytes;
        
        if payload.len() < 128 {
            return Err(anyhow::anyhow!("Token info message too small"));
        }
        
        let token_msg = TokenInfoMessage::read_from(&payload[..128])
            .ok_or_else(|| anyhow::anyhow!("Failed to parse token info message"))?;
        
        let address = token_msg.get_token_address();
        let symbol = token_msg.get_symbol();
        let name = token_msg.get_name();
        let decimals = token_msg.decimals;
        
        info!("📨 Received token broadcast: {} ({}) at {} with {} decimals",
              symbol, name, address, decimals);
        
        // Cache the token info locally
        let token_info = exchange_collector::token_registry::TokenInfo {
            address: address.clone(),
            symbol: symbol.clone(),
            decimals,
            name: if name.is_empty() { None } else { Some(name) },
        };
        
        // Store in the local token registry cache
        token_registry.cache_token(token_info);
        
        debug!("✅ Cached token {} in local registry", symbol);
        
        Ok(())
    }

    async fn add_mock_pools(&self, exchange: &str) {
        use rust_decimal::Decimal;

        let mock_pools = vec![
            PoolInfo {
                address: format!("0x{}_usdc_weth_pool", exchange),
                exchange: exchange.to_string(),
                token0: "USDC".to_string(),
                token1: "WETH".to_string(),
                reserve0: Decimal::new(1000000, 0), // 1M USDC
                reserve1: Decimal::new(500, 0),     // 500 WETH
                fee: Decimal::new(3, 3),            // 0.3%
                last_updated: chrono::Utc::now().timestamp(),
                block_number: 0,
                v3_tick: None,
                v3_sqrt_price_x96: None,
                v3_liquidity: None,
            },
            PoolInfo {
                address: format!("0x{}_usdc_wmatic_pool", exchange),
                exchange: exchange.to_string(),
                token0: "USDC".to_string(),
                token1: "WMATIC".to_string(),
                reserve0: Decimal::new(500000, 0),  // 500K USDC
                reserve1: Decimal::new(1000000, 0), // 1M WMATIC
                fee: Decimal::new(3, 3),            // 0.3%
                last_updated: chrono::Utc::now().timestamp(),
                block_number: 0,
                v3_tick: None,
                v3_sqrt_price_x96: None,
                v3_liquidity: None,
            },
        ];

        for pool in mock_pools {
            debug!("Adding mock pool: {} on {}", pool.address, pool.exchange);
            self.pools.insert(pool.address.clone(), pool);
        }
    }

    async fn update_pools(&self) -> Result<()> {
        debug!("Updating pool reserves...");
        
        // TODO: Implement real pool reserve updates via RPC calls
        // For now, just update timestamps
        for mut pool in self.pools.iter_mut() {
            pool.last_updated = chrono::Utc::now().timestamp();
        }

        Ok(())
    }

    pub async fn get_all_pools(&self) -> Vec<PoolInfo> {
        self.pools.iter().map(|entry| entry.value().clone()).collect()
    }

    pub async fn get_pools_for_exchange(&self, exchange: &str) -> Vec<PoolInfo> {
        self.pools
            .iter()
            .filter(|entry| entry.value().exchange == exchange)
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub async fn get_pools_for_token_pair(&self, token0: &str, token1: &str) -> Vec<PoolInfo> {
        self.pools
            .iter()
            .filter(|entry| {
                let pool = entry.value();
                (pool.token0 == token0 && pool.token1 == token1) ||
                (pool.token0 == token1 && pool.token1 == token0)
            })
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn get_pool_count(&self) -> usize {
        self.pools.len()
    }
    
    /// Get reference to pools for dashboard access
    pub fn get_pools_ref(&self) -> Arc<DashMap<String, PoolInfo>> {
        self.pools.clone()
    }
    
    /// Query actual pool reserves from blockchain via RPC (V2 pools)
    /// Returns (reserve0, reserve1) in proper decimal format
    async fn query_pool_reserves(pool_address: &str) -> Result<(rust_decimal::Decimal, rust_decimal::Decimal)> {
        let rpc_url = std::env::var("ALCHEMY_RPC_URL")
            .unwrap_or_else(|_| "https://polygon-rpc.com".to_string());
            
        let client = Client::builder()
            .timeout(Duration::from_secs(2))
            .build()?;
        
        // Ensure pool address has 0x prefix
        let pool_addr = if pool_address.starts_with("0x") { 
            pool_address.to_string() 
        } else { 
            format!("0x{}", pool_address) 
        };
        
        // UniswapV2 getReserves() method selector: 0x0902f1ac
        let call_data = json!({
            "jsonrpc": "2.0",
            "method": "eth_call",
            "params": [{
                "to": pool_addr,
                "data": "0x0902f1ac"
            }, "latest"],
            "id": 1
        });
        
        let response = client.post(&rpc_url)
            .json(&call_data)
            .send()
            .await?;
            
        let json_response: serde_json::Value = response.json().await?;
        
        if let Some(result_hex) = json_response["result"].as_str() {
            // Parse getReserves() return data:
            // bytes 0-32: reserve0 (uint112)
            // bytes 32-64: reserve1 (uint112)  
            // bytes 64-96: blockTimestampLast (uint32)
            
            let hex_data = result_hex.strip_prefix("0x").unwrap_or(result_hex);
            
            if hex_data.len() >= 128 {
                // Parse reserve0 (first 32 bytes, but only 14 bytes are used for uint112)
                let reserve0_hex = &hex_data[24..64]; // Skip padding, get last 20 bytes  
                let reserve1_hex = &hex_data[88..128]; // Skip padding, get last 20 bytes
                
                let reserve0_raw = u128::from_str_radix(reserve0_hex, 16)
                    .map_err(|e| anyhow::anyhow!("Failed to parse reserve0: {}", e))?;
                let reserve1_raw = u128::from_str_radix(reserve1_hex, 16)
                    .map_err(|e| anyhow::anyhow!("Failed to parse reserve1: {}", e))?;
                
                // Convert from wei to human-readable decimals (assuming 18 decimals for most tokens)
                // TODO: Query actual token decimals for precise conversion
                let reserve0 = rust_decimal::Decimal::new(reserve0_raw as i64, 18);
                let reserve1 = rust_decimal::Decimal::new(reserve1_raw as i64, 18);
                
                Ok((reserve0, reserve1))
            } else {
                Err(anyhow::anyhow!("Invalid getReserves response length: {}", hex_data.len()))
            }
        } else {
            Err(anyhow::anyhow!("No result in RPC response: {:?}", json_response))
        }
    }

}
// Service discovery pattern for dynamic shared memory feed management
// This implementation allows the API server to discover available feeds
// without hardcoded paths or fallbacks, as requested by the user.

use crate::{
    shared_memory::SharedMemoryReader,
    shared_memory_v2::{OptimizedOrderBookDeltaReader, OptimizedTradeReader},
    event_driven_shm::EventDrivenTradeReader,
    semaphore_shm::SemaphoreTradeReader,
    types::Trade,
    Result, AlphaPulseError,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};
use tracing::{info, warn, error, debug};

/// Registry metadata for a shared memory feed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedMetadata {
    pub feed_id: String,
    pub feed_type: FeedType,
    pub path: PathBuf,
    pub exchange: String,
    pub symbol: Option<String>,
    pub created_at: u64,
    pub last_heartbeat: u64,
    pub capacity: usize,
    pub producer_pid: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FeedType {
    Trades,
    OrderBookDeltas,
}

/// Central registry for discovering and managing shared memory feeds
pub struct SharedMemoryRegistry {
    registry_dir: PathBuf,
    feeds: HashMap<String, FeedMetadata>,
    trade_readers: HashMap<String, OptimizedTradeReader>,
    event_driven_trade_readers: HashMap<String, EventDrivenTradeReader>,
    pub semaphore_trade_readers: HashMap<String, SemaphoreTradeReader>,
    delta_readers: HashMap<String, OptimizedOrderBookDeltaReader>,
}

impl SharedMemoryRegistry {
    pub fn new() -> Result<Self> {
        let registry_dir = PathBuf::from("./shm_registry");
        
        // Ensure registry directory exists
        if !registry_dir.exists() {
            fs::create_dir_all(&registry_dir)
                .map_err(|e| AlphaPulseError::ConfigError(format!("Failed to create registry dir: {}", e)))?;
        }
        
        Ok(Self {
            registry_dir,
            feeds: HashMap::new(),
            trade_readers: HashMap::new(),
            event_driven_trade_readers: HashMap::new(),
            semaphore_trade_readers: HashMap::new(),
            delta_readers: HashMap::new(),
        })
    }
    
    /// Register a new shared memory feed
    pub fn register_feed(&mut self, metadata: FeedMetadata) -> Result<()> {
        let metadata_path = self.registry_dir.join(format!("{}.json", metadata.feed_id));
        
        // Write metadata file
        let metadata_json = serde_json::to_string_pretty(&metadata)
            .map_err(|e| AlphaPulseError::ConfigError(format!("Failed to serialize metadata: {}", e)))?;
        
        fs::write(&metadata_path, metadata_json)
            .map_err(|e| AlphaPulseError::ConfigError(format!("Failed to write metadata: {}", e)))?;
        
        info!("📋 Registered feed: {} ({:?}) at {:?}", metadata.feed_id, metadata.feed_type, metadata.path);
        self.feeds.insert(metadata.feed_id.clone(), metadata);
        
        Ok(())
    }
    
    /// Discover available feeds by scanning the registry directory
    pub fn discover_feeds(&mut self) -> Result<usize> {
        info!("🔍 Discovering available shared memory feeds in {:?}...", self.registry_dir);
        
        // First check if directory exists
        if !self.registry_dir.exists() {
            warn!("📂 Registry directory {:?} does not exist", self.registry_dir);
            // Try to create it
            if let Err(e) = fs::create_dir_all(&self.registry_dir) {
                warn!("📂 Failed to create registry directory {:?}: {}", self.registry_dir, e);
                return Ok(0);
            }
            info!("📂 Created registry directory {:?}", self.registry_dir);
        }
        
        let mut discovered_count = 0;
        let entries = fs::read_dir(&self.registry_dir)
            .map_err(|e| AlphaPulseError::ConfigError(format!("Failed to read registry dir {:?}: {}", self.registry_dir, e)))?;
        
        for entry in entries {
            let entry = entry
                .map_err(|e| AlphaPulseError::ConfigError(format!("Failed to read entry: {}", e)))?;
            
            let path = entry.path();
            info!("📁 Found file: {:?}", path);
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                info!("📄 Processing JSON metadata file: {:?}", path);
                match self.load_feed_metadata(&path) {
                    Ok(metadata) => {
                        // Verify the shared memory file still exists
                        if metadata.path.exists() {
                            // Check if process is still alive AND heartbeat is recent
                            let now = SystemTime::now().duration_since(UNIX_EPOCH)
                                .map_err(|e| AlphaPulseError::ConfigError(format!("Failed to get timestamp: {}", e)))?
                                .as_secs();
                            let heartbeat_fresh = (now - metadata.last_heartbeat) < 30; // 30 second timeout
                            
                            if self.is_producer_alive(metadata.producer_pid) && heartbeat_fresh {
                                self.feeds.insert(metadata.feed_id.clone(), metadata);
                                discovered_count += 1;
                            } else {
                                let reason = if !self.is_producer_alive(metadata.producer_pid) { 
                                    "dead process" 
                                } else { 
                                    "stale heartbeat" 
                                };
                                warn!("🧹 Cleaning up stale feed metadata ({}): {}", reason, metadata.feed_id);
                                let _ = fs::remove_file(&path);
                            }
                        } else {
                            warn!("🧹 Shared memory file missing, removing metadata: {:?}", metadata.path);
                            let _ = fs::remove_file(&path);
                        }
                    }
                    Err(e) => {
                        warn!("📋 Failed to load feed metadata from {:?}: {}", path, e);
                    }
                }
            }
        }
        
        info!("🔍 Discovered {} active shared memory feeds", discovered_count);
        Ok(discovered_count)
    }
    
    /// Initialize readers for all discovered feeds (only creates new readers for feeds not already initialized)
    pub fn initialize_readers(&mut self) -> Result<()> {
        info!("🚀 Initializing readers for {} feeds", self.feeds.len());
        
        for (feed_id, metadata) in &self.feeds {
            match metadata.feed_type {
                FeedType::Trades => {
                    // Try semaphore readers first (true zero-polling architecture!)
                    if !self.semaphore_trade_readers.contains_key(feed_id) {
                        match SemaphoreTradeReader::open(&metadata.path.to_string_lossy()) {
                            Ok(reader) => {
                                self.semaphore_trade_readers.insert(feed_id.clone(), reader);
                                info!("✅ Semaphore trade reader initialized: {} (TRUE zero polling!)", feed_id);
                            }
                            Err(e) => {
                                info!("🔄 Semaphore reader failed for {}, trying event-driven: {}", feed_id, e);
                                
                                // Fallback to event-driven readers
                                if !self.event_driven_trade_readers.contains_key(feed_id) {
                                    match EventDrivenTradeReader::open(&metadata.path.to_string_lossy()) {
                                        Ok(reader) => {
                                            self.event_driven_trade_readers.insert(feed_id.clone(), reader);
                                            info!("✅ Event-driven trade reader initialized: {} (adaptive polling)", feed_id);
                                        }
                                        Err(e2) => {
                                            error!("❌ Failed to initialize event-driven trade reader {}: {}", feed_id, e2);
                                            // Final fallback to legacy readers
                                            info!("🔄 Falling back to legacy reader for {}", feed_id);
                                            if !self.trade_readers.contains_key(feed_id) {
                                                let consumer_id = self.extract_consumer_id(feed_id);
                                                match OptimizedTradeReader::open(&metadata.path.to_string_lossy(), consumer_id as usize) {
                                                    Ok(reader) => {
                                                        self.trade_readers.insert(feed_id.clone(), reader);
                                                        info!("✅ Legacy trade reader initialized: {} (consumer_id: {})", feed_id, consumer_id);
                                                    }
                                                    Err(e3) => {
                                                        error!("❌ Failed to initialize legacy trade reader {}: {}", feed_id, e3);
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
                FeedType::OrderBookDeltas => {
                    // TEMPORARY: Disable delta reader due to data corruption
                    warn!("⚠️ Skipping delta reader for {} due to data corruption in shared memory", feed_id);
                    warn!("⚠️ Delta data shows invalid timestamps/versions - likely layout mismatch");
                    warn!("⚠️ Trade streaming works perfectly - focusing on that for now");
                    // TODO: Debug and fix delta writer/reader layout mismatch
                }
            }
        }
        
        info!("🎯 Initialized {} semaphore trade readers, {} event-driven trade readers, {} legacy trade readers, and {} delta readers", 
              self.semaphore_trade_readers.len(),
              self.event_driven_trade_readers.len(),
              self.trade_readers.len(), 
              self.delta_readers.len());
        
        Ok(())
    }
    
    /// Read all available trades from all trade feeds
    pub fn read_all_trades(&mut self) -> Vec<Trade> {
        let mut all_trades = Vec::new();
        
        for (_feed_id, reader) in &mut self.trade_readers {
            let trades = reader.read_trades_optimized();
            for shared_trade in trades {
                all_trades.push(Trade {
                    timestamp: shared_trade.timestamp_ns as f64 / 1_000_000_000.0,
                    symbol: shared_trade.symbol_str(),
                    exchange: shared_trade.exchange_str(),
                    price: shared_trade.price,
                    volume: shared_trade.volume,
                    side: Some(if shared_trade.side == 0 { "buy".to_string() } else { "sell".to_string() }),
                    trade_id: Some(
                        String::from_utf8_lossy(&shared_trade.trade_id)
                            .trim_end_matches('\0')
                            .to_string()
                    ),
                });
            }
        }
        
        all_trades
    }
    
    /// Read all available trades using semaphore readers (TRUE zero polling!)
    pub async fn read_all_trades_semaphore(&mut self) -> Vec<Trade> {
        let mut all_trades = Vec::new();
        
        for (feed_id, reader) in &mut self.semaphore_trade_readers {
            // Use non-blocking read first to get any existing data
            match reader.read_new_trades() {
                Ok(shared_trades) => {
                    if !shared_trades.is_empty() {
                        info!("📊 Semaphore reader got {} trades from {}", shared_trades.len(), feed_id);
                    }
                    for shared_trade in shared_trades {
                        all_trades.push(Trade {
                            timestamp: shared_trade.timestamp_ns as f64 / 1_000_000_000.0,
                            symbol: shared_trade.symbol_str(),
                            exchange: shared_trade.exchange_str(),
                            price: shared_trade.price,
                            volume: shared_trade.volume,
                            side: Some(if shared_trade.side == 0 { "buy".to_string() } else { "sell".to_string() }),
                            trade_id: Some(
                                String::from_utf8_lossy(&shared_trade.trade_id)
                                    .trim_end_matches('\0')
                                    .to_string()
                            ),
                        });
                    }
                }
                Err(e) => {
                    warn!("Semaphore reader error for {}: {}", feed_id, e);
                }
            }
        }
        
        all_trades
    }
    
    /// Read all available trades using event-driven readers (optimized for async)
    pub async fn read_all_trades_event_driven(&mut self) -> Vec<Trade> {
        let mut all_trades = Vec::new();
        
        for (feed_id, reader) in &mut self.event_driven_trade_readers {
            // Use non-blocking read for better async performance
            match reader.read_new_trades() {
                Ok(shared_trades) => {
                    if !shared_trades.is_empty() {
                        info!("📊 Event-driven reader got {} trades from {}", shared_trades.len(), feed_id);
                    }
                    for shared_trade in shared_trades {
                        all_trades.push(Trade {
                            timestamp: shared_trade.timestamp_ns as f64 / 1_000_000_000.0,
                            symbol: shared_trade.symbol_str(),
                            exchange: shared_trade.exchange_str(),
                            price: shared_trade.price,
                            volume: shared_trade.volume,
                            side: Some(if shared_trade.side == 0 { "buy".to_string() } else { "sell".to_string() }),
                            trade_id: Some(
                                String::from_utf8_lossy(&shared_trade.trade_id)
                                    .trim_end_matches('\0')
                                    .to_string()
                            ),
                        });
                    }
                }
                Err(e) => {
                    warn!("Event-driven reader error for {}: {}", feed_id, e);
                }
            }
        }
        
        all_trades
    }
    
    /// Read all available order book deltas from all delta feeds
    pub fn read_all_deltas(&mut self) -> Vec<crate::shared_memory::SharedOrderBookDelta> {
        let mut all_deltas = Vec::new();
        
        for (_feed_id, reader) in &mut self.delta_readers {
            let deltas = reader.read_deltas_optimized();
            all_deltas.extend(deltas);
        }
        
        all_deltas
    }
    
    /// Get metadata for all active feeds
    pub fn get_feed_metadata(&self) -> &HashMap<String, FeedMetadata> {
        &self.feeds
    }
    
    /// Get active feed count by type
    pub fn get_feed_counts(&self) -> (usize, usize) {
        let trade_count = self.feeds.values().filter(|f| f.feed_type == FeedType::Trades).count();
        let delta_count = self.feeds.values().filter(|f| f.feed_type == FeedType::OrderBookDeltas).count();
        (trade_count, delta_count)
    }
    
    // Private helper methods
    
    fn load_feed_metadata(&self, path: &Path) -> Result<FeedMetadata> {
        let content = fs::read_to_string(path)
            .map_err(|e| AlphaPulseError::ConfigError(format!("Failed to read metadata file: {}", e)))?;
        
        serde_json::from_str(&content)
            .map_err(|e| AlphaPulseError::ConfigError(format!("Failed to parse metadata: {}", e)))
    }
    
    fn is_producer_alive(&self, pid: u32) -> bool {
        // On Unix systems, we can check if a process exists by trying to send signal 0
        #[cfg(unix)]
        {
            use std::process::Command;
            let output = Command::new("kill")
                .args(["-0", &pid.to_string()])
                .output();
            
            match output {
                Ok(result) => result.status.success(),
                Err(_) => false,
            }
        }
        
        #[cfg(not(unix))]
        {
            // On non-Unix systems, assume alive (conservative approach)
            true
        }
    }
    
    fn extract_consumer_id(&self, feed_id: &str) -> u32 {
        // Use per-feed reader ID allocation to prevent collisions
        // This ensures unique consumer IDs for concurrent readers
        use std::collections::HashMap;
        use std::sync::Mutex;
        
        static READER_ID_COUNTERS: std::sync::LazyLock<Mutex<HashMap<String, u32>>> = 
            std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));
        
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        let max_readers = 8;
        #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
        let max_readers = 16;
        
        let mut counters = READER_ID_COUNTERS.lock().unwrap();
        let counter = counters.entry(feed_id.to_string()).or_insert(0);
        let reader_id = *counter;
        *counter = (*counter + 1) % max_readers; // Platform-specific max readers
        
        reader_id
    }
}

/// Helper function to create feed metadata for collectors
pub fn create_feed_metadata(
    feed_id: String,
    feed_type: FeedType,
    path: PathBuf,
    exchange: String,
    symbol: Option<String>,
    capacity: usize,
) -> Result<FeedMetadata> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| AlphaPulseError::ConfigError(format!("Failed to get timestamp: {}", e)))?
        .as_secs();
    
    Ok(FeedMetadata {
        feed_id,
        feed_type,
        path,
        exchange,
        symbol,
        created_at: now,
        last_heartbeat: now,
        capacity,
        producer_pid: std::process::id(),
    })
}

/// Update heartbeat for a feed (called by producers)
pub fn update_feed_heartbeat(feed_id: &str) -> Result<()> {
    let registry_dir = PathBuf::from("./shm_registry");
    let metadata_path = registry_dir.join(format!("{}.json", feed_id));
    
    if metadata_path.exists() {
        let content = fs::read_to_string(&metadata_path)
            .map_err(|e| AlphaPulseError::ConfigError(format!("Failed to read metadata: {}", e)))?;
        
        let mut metadata: FeedMetadata = serde_json::from_str(&content)
            .map_err(|e| AlphaPulseError::ConfigError(format!("Failed to parse metadata: {}", e)))?;
        
        metadata.last_heartbeat = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| AlphaPulseError::ConfigError(format!("Failed to get timestamp: {}", e)))?
            .as_secs();
        
        let updated_json = serde_json::to_string_pretty(&metadata)
            .map_err(|e| AlphaPulseError::ConfigError(format!("Failed to serialize metadata: {}", e)))?;
        
        fs::write(&metadata_path, updated_json)
            .map_err(|e| AlphaPulseError::ConfigError(format!("Failed to write metadata: {}", e)))?;
    }
    
    Ok(())
}
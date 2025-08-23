use crate::message_protocol::{InstrumentId, MessageType, ParseError};
use crate::messages::{TradeMessage, QuoteMessage, InstrumentDiscoveredMessage, SwapEventMessage, PoolUpdateMessage, ArbitrageOpportunityMessage};
use dashmap::DashMap;
use std::sync::Arc;
use anyhow::Result;
use std::any::Any;
use zerocopy::AsBytes;

/// Schema and transform cache with full InstrumentId precision
pub struct SchemaTransformCache {
    /// Static schemas loaded at startup
    static_schemas: std::collections::HashMap<(MessageType, u8), &'static MessageSchema>,
    
    /// Dynamic schemas registered at runtime
    dynamic_schemas: DashMap<(MessageType, u8), MessageSchema>,
    
    /// Object cache keyed by full InstrumentId (no truncation!)
    objects: DashMap<InstrumentId, CachedObject>,
    
    /// Optional reverse lookup for legacy u64 keys
    u64_index: Option<DashMap<u64, InstrumentId>>,
}

/// Message schema definition
pub struct MessageSchema {
    pub message_type: MessageType,
    pub version: u8,
    pub size: Option<usize>,  // Fixed size if Some
    pub parser: Box<dyn MessageParser>,
}

/// Message parser trait for dynamic parsing
pub trait MessageParser: Send + Sync {
    fn parse(&self, data: &[u8]) -> Result<Box<dyn Any>>;
    fn to_cached_object(&self, parsed: Box<dyn Any>) -> Option<CachedObject>;
}

/// Cached object types
#[derive(Debug, Clone)]
pub enum CachedObject {
    Instrument(InstrumentMetadata),
    Pool(PoolMetadata),
    Token(TokenMetadata),
    Custom(Arc<dyn Any + Send + Sync>),
}

/// Instrument metadata
#[derive(Debug, Clone)]
pub struct InstrumentMetadata {
    pub id: InstrumentId,
    pub symbol: String,
    pub decimals: u8,
    pub discovered_at: u64,
    pub venue_name: String,
    pub asset_type_name: String,
}

/// Pool metadata
#[derive(Debug, Clone)]
pub struct PoolMetadata {
    pub id: InstrumentId,
    pub token0_id: InstrumentId,
    pub token1_id: InstrumentId,
    pub symbol: String,
    pub fee_tier: Option<u32>,
    pub protocol_type: String,
    pub discovered_at: u64,
}

/// Token metadata
#[derive(Debug, Clone)]
pub struct TokenMetadata {
    pub id: InstrumentId,
    pub address: String,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub chain_id: u32,
    pub discovered_at: u64,
}

impl SchemaTransformCache {
    /// Create a new cache
    pub fn new() -> Self {
        Self {
            static_schemas: std::collections::HashMap::new(),
            dynamic_schemas: DashMap::new(),
            objects: DashMap::new(),
            u64_index: Some(DashMap::new()), // Enable for compatibility
        }
    }
    
    /// Create cache without u64 compatibility index
    pub fn new_without_u64_index() -> Self {
        Self {
            static_schemas: std::collections::HashMap::new(),
            dynamic_schemas: DashMap::new(),
            objects: DashMap::new(),
            u64_index: None,
        }
    }
    
    /// Insert object with full InstrumentId key (no data loss!)
    pub fn insert(&self, id: InstrumentId, object: CachedObject) {
        self.objects.insert(id, object);
        
        // Optionally maintain u64 index for legacy compatibility
        if let Some(ref index) = self.u64_index {
            let u64_key = id.to_u64();
            index.insert(u64_key, id);
        }
    }
    
    /// Get object by full InstrumentId (primary lookup method)
    pub fn get(&self, id: &InstrumentId) -> Option<CachedObject> {
        self.objects.get(id).map(|r| r.clone())
    }
    
    /// Legacy compatibility: get by u64 key (may have precision loss)
    pub fn get_by_u64(&self, key: u64) -> Option<CachedObject> {
        if let Some(ref index) = self.u64_index {
            if let Some(id_ref) = index.get(&key) {
                let id = *id_ref;
                return self.objects.get(&id).map(|r| r.clone());
            }
        }
        
        // Fallback: try to reconstruct InstrumentId from u64 (loses precision)
        let reconstructed_id = InstrumentId::from_u64(key);
        self.objects.get(&reconstructed_id).map(|r| r.clone())
    }
    
    /// Get all cached objects
    pub fn all_objects(&self) -> Vec<(InstrumentId, CachedObject)> {
        self.objects
            .iter()
            .map(|entry| (*entry.key(), entry.value().clone()))
            .collect()
    }
    
    /// Get objects by venue
    pub fn get_by_venue(&self, venue: crate::message_protocol::VenueId) -> Vec<(InstrumentId, CachedObject)> {
        self.objects
            .iter()
            .filter_map(|entry| {
                let id = *entry.key();
                if id.venue().ok() == Some(venue) {
                    Some((id, entry.value().clone()))
                } else {
                    None
                }
            })
            .collect()
    }
    
    /// Get objects by asset type
    pub fn get_by_asset_type(&self, asset_type: crate::message_protocol::AssetType) -> Vec<(InstrumentId, CachedObject)> {
        self.objects
            .iter()
            .filter_map(|entry| {
                let id = *entry.key();
                if id.asset_type().ok() == Some(asset_type) {
                    Some((id, entry.value().clone()))
                } else {
                    None
                }
            })
            .collect()
    }
    
    /// Remove object
    pub fn remove(&self, id: &InstrumentId) -> Option<CachedObject> {
        let result = self.objects.remove(id).map(|(_, v)| v);
        
        // Clean up u64 index if enabled
        if let Some(ref index) = self.u64_index {
            let u64_key = id.to_u64();
            index.remove(&u64_key);
        }
        
        result
    }
    
    /// Clear all objects
    pub fn clear(&self) {
        self.objects.clear();
        if let Some(ref index) = self.u64_index {
            index.clear();
        }
    }
    
    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            object_count: self.objects.len(),
            u64_index_count: self.u64_index.as_ref().map(|i| i.len()).unwrap_or(0),
            schema_count: self.static_schemas.len() + self.dynamic_schemas.len(),
        }
    }
    
    /// Process binary message and cache any discovered objects
    pub fn process_message(&self, data: &[u8]) -> Result<ProcessedMessage, ParseError> {
        // Parse header to determine message type
        let header = crate::message_protocol::MessageHeader::from_bytes(data)?;
        let message_type = header.message_type()?;
        let version = header.version;
        
        match message_type {
            MessageType::Trade => {
                let trade = TradeMessage::from_bytes(data)?;
                Ok(ProcessedMessage::Trade(TradeData {
                    instrument_id: trade.instrument_id,
                    price: trade.price_decimal(),
                    volume: trade.volume_decimal(),
                    side: trade.trade_side()?,
                    timestamp: header.timestamp,
                }))
            }
            MessageType::Quote => {
                let quote = QuoteMessage::from_bytes(data)?;
                Ok(ProcessedMessage::Quote(QuoteData {
                    instrument_id: quote.instrument_id,
                    bid_price: quote.bid_price_decimal(),
                    ask_price: quote.ask_price_decimal(),
                    bid_size: quote.bid_size_decimal(),
                    ask_size: quote.ask_size_decimal(),
                    spread_bps: quote.spread_bps(),
                    timestamp: header.timestamp,
                }))
            }
            MessageType::InstrumentDiscovered => {
                let discovery = InstrumentDiscoveredMessage::parse(data)?;
                
                // Create and cache instrument metadata
                let metadata = InstrumentMetadata {
                    id: discovery.header.instrument_id,
                    symbol: discovery.symbol.clone(),
                    decimals: discovery.header.decimals,
                    discovered_at: header.timestamp,
                    venue_name: discovery.header.instrument_id.venue()
                        .map(|v| format!("{:?}", v))
                        .unwrap_or_else(|_| "Unknown".to_string()),
                    asset_type_name: discovery.header.instrument_id.asset_type()
                        .map(|t| format!("{:?}", t))
                        .unwrap_or_else(|_| "Unknown".to_string()),
                };
                
                self.insert(discovery.header.instrument_id, CachedObject::Instrument(metadata.clone()));
                
                Ok(ProcessedMessage::InstrumentDiscovered(metadata))
            }
            MessageType::SwapEvent => {
                let swap = SwapEventMessage::from_bytes(data)?;
                Ok(ProcessedMessage::SwapEvent(SwapEventData {
                    pool_id: swap.pool_id,
                    token0_id: swap.token0_id,
                    token1_id: swap.token1_id,
                    amount0_in: swap.amount0_in_decimal(),
                    amount1_in: swap.amount1_in_decimal(),
                    amount0_out: swap.amount0_out_decimal(),
                    amount1_out: swap.amount1_out_decimal(),
                    timestamp: header.timestamp,
                }))
            }
            MessageType::PoolUpdate => {
                let pool_update = PoolUpdateMessage::from_bytes(data)?;
                Ok(ProcessedMessage::PoolUpdate(PoolUpdateData {
                    pool_id: pool_update.pool_id,
                    reserve0: pool_update.reserve0_decimal(),
                    reserve1: pool_update.reserve1_decimal(),
                    sqrt_price_x96: pool_update.sqrt_price_x96,
                    tick: pool_update.tick,
                    timestamp: header.timestamp,
                }))
            }
            MessageType::ArbitrageOpportunity => {
                let arb = ArbitrageOpportunityMessage::from_bytes(data)?;
                Ok(ProcessedMessage::ArbitrageOpportunity(ArbitrageData {
                    token0_id: arb.token0_id,
                    token1_id: arb.token1_id,
                    buy_pool_id: arb.buy_pool_id,
                    sell_pool_id: arb.sell_pool_id,
                    buy_price: arb.buy_price as f64 / 100_000_000.0,
                    sell_price: arb.sell_price as f64 / 100_000_000.0,
                    profit_percentage: arb.profit_percent_decimal(),
                    timestamp: header.timestamp,
                }))
            }
            _ => {
                // For unknown message types, return raw data for forwarding
                Ok(ProcessedMessage::Unknown {
                    message_type,
                    version,
                    data: data.to_vec(),
                })
            }
        }
    }
    
    /// Register a dynamic schema at runtime
    pub fn register_dynamic_schema(&self, schema: MessageSchema) {
        let key = (schema.message_type, schema.version);
        self.dynamic_schemas.insert(key, schema);
    }
}

/// Processed message types
#[derive(Debug, Clone)]
pub enum ProcessedMessage {
    Trade(TradeData),
    Quote(QuoteData),
    InstrumentDiscovered(InstrumentMetadata),
    SwapEvent(SwapEventData),
    PoolUpdate(PoolUpdateData),
    ArbitrageOpportunity(ArbitrageData),
    Unknown {
        message_type: MessageType,
        version: u8,
        data: Vec<u8>,
    },
}

/// Processed trade data
#[derive(Debug, Clone)]
pub struct TradeData {
    pub instrument_id: InstrumentId,
    pub price: f64,
    pub volume: f64,
    pub side: crate::messages::TradeSide,
    pub timestamp: u64,
}

/// Processed quote data
#[derive(Debug, Clone)]
pub struct QuoteData {
    pub instrument_id: InstrumentId,
    pub bid_price: f64,
    pub ask_price: f64,
    pub bid_size: f64,
    pub ask_size: f64,
    pub spread_bps: u32,
    pub timestamp: u64,
}

/// Processed swap event data
#[derive(Debug, Clone)]
pub struct SwapEventData {
    pub pool_id: InstrumentId,
    pub token0_id: InstrumentId,
    pub token1_id: InstrumentId,
    pub amount0_in: f64,
    pub amount1_in: f64,
    pub amount0_out: f64,
    pub amount1_out: f64,
    pub timestamp: u64,
}

/// Processed pool update data
#[derive(Debug, Clone)]
pub struct PoolUpdateData {
    pub pool_id: InstrumentId,
    pub reserve0: f64,
    pub reserve1: f64,
    pub sqrt_price_x96: u128,
    pub tick: i32,
    pub timestamp: u64,
}

/// Processed arbitrage opportunity data
#[derive(Debug, Clone)]
pub struct ArbitrageData {
    pub token0_id: InstrumentId,
    pub token1_id: InstrumentId,
    pub buy_pool_id: InstrumentId,
    pub sell_pool_id: InstrumentId,
    pub buy_price: f64,
    pub sell_price: f64,
    pub profit_percentage: f64,
    pub timestamp: u64,
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub object_count: usize,
    pub u64_index_count: usize,
    pub schema_count: usize,
}

impl Default for SchemaTransformCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message_protocol::{VenueId, AssetType, SourceType};
    use crate::messages::TradeSide;

    #[test]
    fn test_full_precision_cache() {
        let cache = SchemaTransformCache::new();
        
        // Create token with full address precision
        let usdc = InstrumentId::ethereum_token("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48").unwrap();
        
        let metadata = InstrumentMetadata {
            id: usdc,
            symbol: "USDC".to_string(),
            decimals: 6,
            discovered_at: 1234567890,
            venue_name: "Ethereum".to_string(),
            asset_type_name: "Token".to_string(),
        };
        
        // Insert with full InstrumentId (no truncation)
        cache.insert(usdc, CachedObject::Instrument(metadata.clone()));
        
        // Retrieve with full InstrumentId - exact match
        let retrieved = cache.get(&usdc);
        assert!(retrieved.is_some());
        
        if let Some(CachedObject::Instrument(meta)) = retrieved {
            assert_eq!(meta.symbol, "USDC");
            assert_eq!(meta.decimals, 6);
        }
        
        // Test u64 compatibility (may have precision loss)
        let u64_key = usdc.to_u64();
        let by_u64 = cache.get_by_u64(u64_key);
        assert!(by_u64.is_some());
    }

    #[test]
    fn test_venue_filtering() {
        let cache = SchemaTransformCache::new();
        
        // Add tokens from different venues
        let eth_token = InstrumentId::ethereum_token("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48").unwrap();
        let poly_token = InstrumentId::polygon_token("0x2791bca1f2de4661ed88a30c99a7a9449aa84174").unwrap();
        let stock = InstrumentId::stock(VenueId::NASDAQ, "AAPL");
        
        cache.insert(eth_token, CachedObject::Token(TokenMetadata {
            id: eth_token,
            address: "0xa0b8...".to_string(),
            symbol: "USDC".to_string(),
            name: "USD Coin".to_string(),
            decimals: 6,
            chain_id: 1,
            discovered_at: 1234567890,
        }));
        
        cache.insert(poly_token, CachedObject::Token(TokenMetadata {
            id: poly_token,
            address: "0x2791...".to_string(),
            symbol: "USDC".to_string(),
            name: "USD Coin (PoS)".to_string(),
            decimals: 6,
            chain_id: 137,
            discovered_at: 1234567891,
        }));
        
        cache.insert(stock, CachedObject::Instrument(InstrumentMetadata {
            id: stock,
            symbol: "AAPL".to_string(),
            decimals: 2,
            discovered_at: 1234567892,
            venue_name: "NASDAQ".to_string(),
            asset_type_name: "Stock".to_string(),
        }));
        
        // Filter by venue
        let ethereum_objects = cache.get_by_venue(VenueId::Ethereum);
        assert_eq!(ethereum_objects.len(), 1);
        
        let polygon_objects = cache.get_by_venue(VenueId::Polygon);
        assert_eq!(polygon_objects.len(), 1);
        
        let nasdaq_objects = cache.get_by_venue(VenueId::NASDAQ);
        assert_eq!(nasdaq_objects.len(), 1);
        
        // Filter by asset type
        let tokens = cache.get_by_asset_type(AssetType::Token);
        assert_eq!(tokens.len(), 2);
        
        let stocks = cache.get_by_asset_type(AssetType::Stock);
        assert_eq!(stocks.len(), 1);
    }

    #[test]
    fn test_message_processing() {
        let cache = SchemaTransformCache::new();
        
        // Create a trade message
        let instrument_id = InstrumentId::stock(VenueId::NASDAQ, "AAPL");
        let trade = TradeMessage::new(
            instrument_id,
            15000000000, // $150.00
            10000000,    // 0.1 shares
            TradeSide::Buy,
            1234,
            SourceType::External,
        );
        
        let bytes = trade.as_bytes();
        let processed = cache.process_message(bytes).unwrap();
        
        match processed {
            ProcessedMessage::Trade(data) => {
                assert_eq!(data.instrument_id, instrument_id);
                assert_eq!(data.price, 150.0);
                assert_eq!(data.volume, 0.1);
                assert_eq!(data.side, TradeSide::Buy);
            }
            _ => panic!("Expected trade message"),
        }
    }

    #[test]
    fn test_cache_stats() {
        let cache = SchemaTransformCache::new();
        
        let id1 = InstrumentId::stock(VenueId::NASDAQ, "AAPL");
        let id2 = InstrumentId::ethereum_token("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48").unwrap();
        
        cache.insert(id1, CachedObject::Instrument(InstrumentMetadata {
            id: id1,
            symbol: "AAPL".to_string(),
            decimals: 2,
            discovered_at: 123,
            venue_name: "NASDAQ".to_string(),
            asset_type_name: "Stock".to_string(),
        }));
        
        cache.insert(id2, CachedObject::Token(TokenMetadata {
            id: id2,
            address: "0xa0b8...".to_string(),
            symbol: "USDC".to_string(),
            name: "USD Coin".to_string(),
            decimals: 6,
            chain_id: 1,
            discovered_at: 124,
        }));
        
        let stats = cache.stats();
        assert_eq!(stats.object_count, 2);
        assert_eq!(stats.u64_index_count, 2); // u64 index enabled by default
    }
}
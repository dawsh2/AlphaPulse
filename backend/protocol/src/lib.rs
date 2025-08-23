use byteorder::{ByteOrder, LittleEndian};
use thiserror::Error;
use zerocopy::{AsBytes, FromBytes, FromZeroes};
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

// Export conversion and validation modules
pub mod conversion;
pub mod validation;
pub mod schema_cache;
pub mod schemas;
pub mod codec;
pub mod dex_config;
pub mod message_protocol;
pub mod messages;
pub mod schema_transform_cache;
// TokenRegistry is in exchange_collector crate (alphapulse_exchange_collector::token_registry)

pub const MAGIC_BYTE: u8 = 0xFE;
// Multi-relay architecture paths
pub const MARKET_DATA_RELAY_PATH: &str = "/tmp/alphapulse/market_data.sock";
pub const SIGNAL_RELAY_PATH: &str = "/tmp/alphapulse/signals.sock";

// Legacy paths (deprecated)
pub const UNIX_SOCKET_PATH: &str = "/tmp/alphapulse/market_data.sock";
pub const METRICS_SOCKET_PATH: &str = "/tmp/alphapulse/metrics.sock";
pub const RELAY_BIND_PATH: &str = "/tmp/alphapulse/market_data.sock"; // Now points to market_data.sock

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageType {
    Trade = 1,
    OrderBook = 2,  // Legacy full book
    Heartbeat = 3,
    Metrics = 4,
    L2Snapshot = 5,
    L2Delta = 6,
    L2Reset = 7,
    SymbolMapping = 8,  // Maps hash to human-readable symbol
    ArbitrageOpportunity = 9,  // DeFi arbitrage opportunity
    StatusUpdate = 10,  // Block numbers, gas prices, system status
    SwapEvent = 12,  // V2/V3 swap events with tick liquidity data
    PoolUpdate = 13,  // Pool liquidity updates (Mint/Burn/Collect events)
    // Phase 2: Deep equality validation
    MessageTrace = 11,  // Message ID tracing for deep equality validation
    TokenInfo = 14,  // Token metadata broadcast for newly discovered tokens
}

impl TryFrom<u8> for MessageType {
    type Error = ProtocolError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(MessageType::Trade),
            2 => Ok(MessageType::OrderBook),
            3 => Ok(MessageType::Heartbeat),
            4 => Ok(MessageType::Metrics),
            5 => Ok(MessageType::L2Snapshot),
            6 => Ok(MessageType::L2Delta),
            7 => Ok(MessageType::L2Reset),
            8 => Ok(MessageType::SymbolMapping),
            9 => Ok(MessageType::ArbitrageOpportunity),
            10 => Ok(MessageType::StatusUpdate),
            11 => Ok(MessageType::MessageTrace),
            12 => Ok(MessageType::SwapEvent),
            13 => Ok(MessageType::PoolUpdate),
            _ => Err(ProtocolError::InvalidMessageType(value)),
        }
    }
}

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("Invalid magic byte: {0:x}")]
    InvalidMagicByte(u8),
    #[error("Invalid message type: {0}")]
    InvalidMessageType(u8),
    #[error("Buffer too small: need {need}, got {got}")]
    BufferTooSmall { need: usize, got: usize },
    #[error("Invalid symbol hash: {0}")]
    InvalidSymbolHash(u64),
}

/// Represents a complete symbol with all necessary components for different asset classes
#[derive(Debug, Clone, PartialEq)]
pub struct SymbolDescriptor {
    pub exchange: String,
    pub base: String,           // BTC, AAPL, EUR
    pub quote: Option<String>,  // USD, EUR (for pairs)
    pub expiry: Option<u32>,    // YYYYMMDD format
    pub strike: Option<f64>,    
    pub option_type: Option<char>, // 'C' or 'P'
}

impl SymbolDescriptor {
    /// Create a spot/cash symbol (crypto or forex)
    pub fn spot(exchange: impl Into<String>, base: impl Into<String>, quote: impl Into<String>) -> Self {
        Self {
            exchange: exchange.into(),
            base: base.into(),
            quote: Some(quote.into()),
            expiry: None,
            strike: None,
            option_type: None,
        }
    }

    /// Create a stock symbol
    pub fn stock(exchange: impl Into<String>, ticker: impl Into<String>) -> Self {
        Self {
            exchange: exchange.into(),
            base: ticker.into(),
            quote: None,
            expiry: None,
            strike: None,
            option_type: None,
        }
    }

    /// Create a futures symbol
    pub fn future(exchange: impl Into<String>, base: impl Into<String>, expiry: u32) -> Self {
        Self {
            exchange: exchange.into(),
            base: base.into(),
            quote: None,
            expiry: Some(expiry),
            strike: None,
            option_type: None,
        }
    }

    /// Create an option symbol
    pub fn option(
        exchange: impl Into<String>, 
        base: impl Into<String>, 
        expiry: u32, 
        strike: f64, 
        option_type: char
    ) -> Self {
        Self {
            exchange: exchange.into(),
            base: base.into(),
            quote: None,
            expiry: Some(expiry),
            strike: Some(strike),
            option_type: Some(option_type),
        }
    }

    /// Generate canonical string representation
    pub fn to_string(&self) -> String {
        let mut parts = vec![self.exchange.clone()];
        
        // Add base-quote or just base
        if let Some(ref quote) = self.quote {
            parts.push(format!("{}-{}", self.base, quote));
        } else {
            parts.push(self.base.clone());
        }
        
        // Add expiry if present
        if let Some(expiry) = self.expiry {
            parts.push(expiry.to_string());
        }
        
        // Add strike if present
        if let Some(strike) = self.strike {
            parts.push(format!("{:.2}", strike));
        }
        
        // Add option type if present
        if let Some(opt_type) = self.option_type {
            parts.push(opt_type.to_string());
        }
        
        parts.join(":")
    }

    /// Generate deterministic 64-bit hash for zero-copy protocol
    pub fn hash(&self) -> u64 {
        let canonical = self.to_string();
        let mut hasher = DefaultHasher::new();
        canonical.hash(&mut hasher);
        hasher.finish()
    }
    
    /// Parse from canonical string format
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() < 2 {
            return None;
        }
        
        let exchange = parts[0].to_string();
        let symbol_part = parts[1];
        
        // Check if it's a pair (contains hyphen)
        let (base, quote) = if symbol_part.contains('-') {
            let pair_parts: Vec<&str> = symbol_part.split('-').collect();
            if pair_parts.len() != 2 {
                return None;
            }
            (pair_parts[0].to_string(), Some(pair_parts[1].to_string()))
        } else {
            (symbol_part.to_string(), None)
        };
        
        // Parse remaining parts for derivatives
        let expiry = parts.get(2).and_then(|s| s.parse::<u32>().ok());
        let strike = parts.get(3).and_then(|s| s.parse::<f64>().ok());
        let option_type = parts.get(4).and_then(|s| s.chars().next());
        
        Some(Self {
            exchange,
            base,
            quote,
            expiry,
            strike,
            option_type,
        })
    }
}

#[derive(Debug, Clone, Copy, AsBytes, FromBytes, FromZeroes)]
#[repr(C)]
pub struct MessageHeader {
    pub magic: u8,
    pub msg_type: u8,
    pub flags: u8,
    pub length: [u8; 2],
    pub sequence: [u8; 3],
}

impl MessageHeader {
    pub const SIZE: usize = 8;

    pub fn new(msg_type: MessageType, payload_len: u16, sequence: u32) -> Self {
        let mut header = Self::new_zeroed();
        header.magic = MAGIC_BYTE;
        header.msg_type = msg_type as u8;
        header.flags = 0;
        LittleEndian::write_u16(&mut header.length, payload_len);
        
        let seq_bytes = sequence.to_le_bytes();
        header.sequence[0] = seq_bytes[0];
        header.sequence[1] = seq_bytes[1];
        header.sequence[2] = seq_bytes[2];
        
        header
    }

    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.magic != MAGIC_BYTE {
            return Err(ProtocolError::InvalidMagicByte(self.magic));
        }
        MessageType::try_from(self.msg_type)?;
        Ok(())
    }

    pub fn get_length(&self) -> u16 {
        LittleEndian::read_u16(&self.length)
    }

    pub fn get_sequence(&self) -> u32 {
        let mut bytes = [0u8; 4];
        bytes[0] = self.sequence[0];
        bytes[1] = self.sequence[1];
        bytes[2] = self.sequence[2];
        u32::from_le_bytes(bytes)
    }

    pub fn get_type(&self) -> Result<MessageType, ProtocolError> {
        MessageType::try_from(self.msg_type)
    }
}

#[derive(Debug, Clone, Copy, AsBytes, FromBytes, FromZeroes)]
#[repr(C)]
pub struct TradeMessage {
    pub timestamp_ns: [u8; 8],          // Original exchange timestamp
    pub ingestion_ns: [u8; 8],          // When we received from exchange
    pub relay_ns: [u8; 8],              // When relay processed
    pub bridge_ns: [u8; 8],             // When websocket bridge processed
    pub price: [u8; 8],
    pub volume: [u8; 8],
    pub symbol_hash: [u8; 8],           // Deterministic hash of symbol
    pub side: u8,
    pub _padding: [u8; 7],              // Padding to make total size 64 bytes
}

impl TradeMessage {
    pub const SIZE: usize = 64;  // Updated size for latency tracking

    pub fn new(
        timestamp_ns: u64,
        price: u64,
        volume: u64,
        symbol_hash: u64,
        side: TradeSide,
    ) -> Self {
        let mut msg = Self::new_zeroed();
        let now_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
            
        LittleEndian::write_u64(&mut msg.timestamp_ns, timestamp_ns);
        LittleEndian::write_u64(&mut msg.ingestion_ns, now_ns);  // Mark ingestion time
        LittleEndian::write_u64(&mut msg.relay_ns, 0);          // Will be set by relay
        LittleEndian::write_u64(&mut msg.bridge_ns, 0);         // Will be set by bridge
        LittleEndian::write_u64(&mut msg.price, price);
        LittleEndian::write_u64(&mut msg.volume, volume);
        LittleEndian::write_u64(&mut msg.symbol_hash, symbol_hash);
        msg.side = side as u8;
        // Padding is already zeroed by new_zeroed()
        msg
    }

    pub fn timestamp_ns(&self) -> u64 {
        LittleEndian::read_u64(&self.timestamp_ns)
    }

    pub fn ingestion_ns(&self) -> u64 {
        LittleEndian::read_u64(&self.ingestion_ns)
    }

    pub fn relay_ns(&self) -> u64 {
        LittleEndian::read_u64(&self.relay_ns)
    }

    pub fn bridge_ns(&self) -> u64 {
        LittleEndian::read_u64(&self.bridge_ns)
    }

    pub fn set_relay_timestamp(&mut self) {
        let now_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        LittleEndian::write_u64(&mut self.relay_ns, now_ns);
    }

    pub fn set_bridge_timestamp(&mut self) {
        let now_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        LittleEndian::write_u64(&mut self.bridge_ns, now_ns);
    }

    pub fn price(&self) -> u64 {
        LittleEndian::read_u64(&self.price)
    }

    pub fn volume(&self) -> u64 {
        LittleEndian::read_u64(&self.volume)
    }

    pub fn symbol_hash(&self) -> u64 {
        LittleEndian::read_u64(&self.symbol_hash)
    }

    pub fn side(&self) -> TradeSide {
        TradeSide::try_from(self.side).unwrap_or(TradeSide::Unknown)
    }

    pub fn price_f64(&self) -> f64 {
        self.price() as f64 / 1e8
    }

    pub fn volume_f64(&self) -> f64 {
        self.volume() as f64 / 1e8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TradeSide {
    Buy = 1,
    Sell = 2,
    Unknown = 0,
}

impl TryFrom<u8> for TradeSide {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(TradeSide::Unknown),
            1 => Ok(TradeSide::Buy),
            2 => Ok(TradeSide::Sell),
            _ => Err(()),
        }
    }
}

/// Maps a symbol hash to its human-readable representation
#[derive(Debug, Clone)]
pub struct SymbolMappingMessage {
    pub symbol_hash: u64,
    pub symbol_string: String,  // e.g., "coinbase:BTC-USD"
}

impl SymbolMappingMessage {
    pub fn new(descriptor: &SymbolDescriptor) -> Self {
        Self {
            symbol_hash: descriptor.hash(),
            symbol_string: descriptor.to_string(),
        }
    }
    
    pub fn encode(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        buffer.extend_from_slice(&self.symbol_hash.to_le_bytes());
        buffer.extend_from_slice(&(self.symbol_string.len() as u16).to_le_bytes());
        buffer.extend_from_slice(self.symbol_string.as_bytes());
        buffer
    }
    
    pub fn decode(data: &[u8]) -> Result<Self, ProtocolError> {
        if data.len() < 10 {
            return Err(ProtocolError::BufferTooSmall { need: 10, got: data.len() });
        }
        
        let symbol_hash = LittleEndian::read_u64(&data[0..8]);
        let string_len = LittleEndian::read_u16(&data[8..10]) as usize;
        
        if data.len() < 10 + string_len {
            return Err(ProtocolError::BufferTooSmall { 
                need: 10 + string_len, 
                got: data.len() 
            });
        }
        
        let symbol_string = String::from_utf8_lossy(&data[10..10 + string_len]).to_string();
        
        Ok(Self {
            symbol_hash,
            symbol_string,
        })
    }
}

#[derive(Debug, Clone)]
pub struct OrderBookMessage {
    pub timestamp_ns: u64,
    pub symbol_hash: u64,
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
}

#[derive(Debug, Clone, Copy, AsBytes, FromBytes, FromZeroes)]
#[repr(C)]
pub struct PriceLevel {
    pub price: [u8; 8],
    pub volume: [u8; 8],
}

impl PriceLevel {
    pub fn new(price: u64, volume: u64) -> Self {
        let mut level = Self::new_zeroed();
        LittleEndian::write_u64(&mut level.price, price);
        LittleEndian::write_u64(&mut level.volume, volume);
        level
    }

    pub fn price(&self) -> u64 {
        LittleEndian::read_u64(&self.price)
    }

    pub fn volume(&self) -> u64 {
        LittleEndian::read_u64(&self.volume)
    }

    pub fn price_f64(&self) -> f64 {
        self.price() as f64 / 1e8
    }

    pub fn volume_f64(&self) -> f64 {
        self.volume() as f64 / 1e8
    }
}

impl OrderBookMessage {
    pub fn encode(&self, buffer: &mut Vec<u8>) {
        buffer.extend_from_slice(&self.timestamp_ns.to_le_bytes());
        buffer.extend_from_slice(&self.symbol_hash.to_le_bytes());
        buffer.extend_from_slice(&(self.bids.len() as u16).to_le_bytes());
        buffer.extend_from_slice(&(self.asks.len() as u16).to_le_bytes());
        
        for bid in &self.bids {
            buffer.extend_from_slice(bid.as_bytes());
        }
        
        for ask in &self.asks {
            buffer.extend_from_slice(ask.as_bytes());
        }
    }

    pub fn decode(data: &[u8]) -> Result<Self, ProtocolError> {
        if data.len() < 20 {
            return Err(ProtocolError::BufferTooSmall {
                need: 20,
                got: data.len(),
            });
        }

        let timestamp_ns = LittleEndian::read_u64(&data[0..8]);
        let symbol_hash = LittleEndian::read_u64(&data[8..16]);
        let bid_count = LittleEndian::read_u16(&data[16..18]) as usize;
        let ask_count = LittleEndian::read_u16(&data[18..20]) as usize;

        let total_size = 20 + (bid_count + ask_count) * 16;
        if data.len() < total_size {
            return Err(ProtocolError::BufferTooSmall {
                need: total_size,
                got: data.len(),
            });
        }

        let mut offset = 20;
        let mut bids = Vec::with_capacity(bid_count);
        for _ in 0..bid_count {
            let level = PriceLevel::read_from_prefix(&data[offset..offset + 16])
                .ok_or(ProtocolError::BufferTooSmall {
                    need: offset + 16,
                    got: data.len(),
                })?;
            bids.push(level);
            offset += 16;
        }

        let mut asks = Vec::with_capacity(ask_count);
        for _ in 0..ask_count {
            let level = PriceLevel::read_from_prefix(&data[offset..offset + 16])
                .ok_or(ProtocolError::BufferTooSmall {
                    need: offset + 16,
                    got: data.len(),
                })?;
            asks.push(level);
            offset += 16;
        }

        Ok(OrderBookMessage {
            timestamp_ns,
            symbol_hash,
            bids,
            asks,
        })
    }
}

#[derive(Debug, Clone, Copy, AsBytes, FromBytes, FromZeroes)]
#[repr(C)]
pub struct HeartbeatMessage {
    pub timestamp_ns: [u8; 8],
    pub sequence: [u8; 4],
    pub _padding: [u8; 4],
}

impl HeartbeatMessage {
    pub const SIZE: usize = 16;

    pub fn new(timestamp_ns: u64, sequence: u32) -> Self {
        let mut msg = Self::new_zeroed();
        LittleEndian::write_u64(&mut msg.timestamp_ns, timestamp_ns);
        LittleEndian::write_u32(&mut msg.sequence, sequence);
        msg
    }

    pub fn timestamp_ns(&self) -> u64 {
        LittleEndian::read_u64(&self.timestamp_ns)
    }

    pub fn sequence(&self) -> u32 {
        LittleEndian::read_u32(&self.sequence)
    }
}

// Serde imports removed - no longer needed after removing SymbolMapper

// REMOVED: Old SymbolMapper - replaced with deterministic hashing via SymbolDescriptor

// REMOVED: ExchangeId enum - exchange names are now part of SymbolDescriptor

// L2 Orderbook Extensions

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum L2Action {
    Delete = 0,
    Update = 1,
    Insert = 2,
}

impl TryFrom<u8> for L2Action {
    type Error = ();
    
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(L2Action::Delete),
            1 => Ok(L2Action::Update),
            2 => Ok(L2Action::Insert),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, AsBytes, FromBytes, FromZeroes)]
#[repr(C)]
pub struct L2Update {
    pub side: u8,           // 0=bid, 1=ask
    pub price: [u8; 8],     // Fixed-point price
    pub volume: [u8; 8],    // Fixed-point volume
    pub action: u8,         // L2Action
}

impl L2Update {
    pub const SIZE: usize = 18;
    
    pub fn new(side: u8, price: u64, volume: u64, action: L2Action) -> Self {
        let mut update = Self::new_zeroed();
        update.side = side;
        LittleEndian::write_u64(&mut update.price, price);
        LittleEndian::write_u64(&mut update.volume, volume);
        update.action = action as u8;
        update
    }
    
    pub fn price(&self) -> u64 {
        LittleEndian::read_u64(&self.price)
    }
    
    pub fn volume(&self) -> u64 {
        LittleEndian::read_u64(&self.volume)
    }
    
    pub fn action(&self) -> L2Action {
        L2Action::try_from(self.action).unwrap_or(L2Action::Update)
    }
}

#[derive(Debug, Clone)]
pub struct L2DeltaMessage {
    pub timestamp_ns: u64,
    pub symbol_hash: u64,    // Deterministic hash only
    pub sequence: u64,
    pub updates: Vec<L2Update>,
}

impl L2DeltaMessage {
    pub fn encode(&self, buffer: &mut Vec<u8>) {
        buffer.extend_from_slice(&self.timestamp_ns.to_le_bytes());
        buffer.extend_from_slice(&self.symbol_hash.to_le_bytes());
        buffer.extend_from_slice(&self.sequence.to_le_bytes());
        buffer.extend_from_slice(&(self.updates.len() as u16).to_le_bytes());
        
        for update in &self.updates {
            buffer.extend_from_slice(update.as_bytes());
        }
    }
    
    pub fn decode(data: &[u8]) -> Result<Self, ProtocolError> {
        if data.len() < 26 {
            return Err(ProtocolError::BufferTooSmall {
                need: 26,
                got: data.len(),
            });
        }
        
        let timestamp_ns = LittleEndian::read_u64(&data[0..8]);
        let symbol_hash = LittleEndian::read_u64(&data[8..16]);
        let sequence = LittleEndian::read_u64(&data[16..24]);
        let update_count = LittleEndian::read_u16(&data[24..26]) as usize;
        
        let total_size = 26 + update_count * L2Update::SIZE;
        if data.len() < total_size {
            return Err(ProtocolError::BufferTooSmall {
                need: total_size,
                got: data.len(),
            });
        }
        
        let mut updates = Vec::with_capacity(update_count);
        let mut offset = 26;
        
        for _ in 0..update_count {
            let update = L2Update::read_from_prefix(&data[offset..offset + L2Update::SIZE])
                .ok_or(ProtocolError::BufferTooSmall {
                    need: offset + L2Update::SIZE,
                    got: data.len(),
                })?;
            updates.push(update);
            offset += L2Update::SIZE;
        }
        
        Ok(L2DeltaMessage {
            timestamp_ns,
            symbol_hash,
            sequence,
            updates,
        })
    }
}

#[derive(Debug, Clone)]
pub struct L2SnapshotMessage {
    pub timestamp_ns: u64,
    pub symbol_hash: u64,    // Deterministic hash only
    pub sequence: u64,
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
}

impl L2SnapshotMessage {
    pub fn encode(&self, buffer: &mut Vec<u8>) {
        buffer.extend_from_slice(&self.timestamp_ns.to_le_bytes());
        buffer.extend_from_slice(&self.symbol_hash.to_le_bytes());
        buffer.extend_from_slice(&self.sequence.to_le_bytes());
        buffer.extend_from_slice(&(self.bids.len() as u16).to_le_bytes());
        buffer.extend_from_slice(&(self.asks.len() as u16).to_le_bytes());
        
        for bid in &self.bids {
            buffer.extend_from_slice(bid.as_bytes());
        }
        
        for ask in &self.asks {
            buffer.extend_from_slice(ask.as_bytes());
        }
    }
    
    pub fn decode(data: &[u8]) -> Result<Self, ProtocolError> {
        if data.len() < 28 {
            return Err(ProtocolError::BufferTooSmall {
                need: 28,
                got: data.len(),
            });
        }
        
        let timestamp_ns = LittleEndian::read_u64(&data[0..8]);
        let symbol_hash = LittleEndian::read_u64(&data[8..16]);
        let sequence = LittleEndian::read_u64(&data[16..24]);
        let bid_count = LittleEndian::read_u16(&data[24..26]) as usize;
        let ask_count = LittleEndian::read_u16(&data[26..28]) as usize;
        
        let total_size = 28 + (bid_count + ask_count) * 16;
        if data.len() < total_size {
            return Err(ProtocolError::BufferTooSmall {
                need: total_size,
                got: data.len(),
            });
        }
        
        let mut offset = 28;
        let mut bids = Vec::with_capacity(bid_count);
        
        for _ in 0..bid_count {
            let level = PriceLevel::read_from_prefix(&data[offset..offset + 16])
                .ok_or(ProtocolError::BufferTooSmall {
                    need: offset + 16,
                    got: data.len(),
                })?;
            bids.push(level);
            offset += 16;
        }
        
        let mut asks = Vec::with_capacity(ask_count);
        for _ in 0..ask_count {
            let level = PriceLevel::read_from_prefix(&data[offset..offset + 16])
                .ok_or(ProtocolError::BufferTooSmall {
                    need: offset + 16,
                    got: data.len(),
                })?;
            asks.push(level);
            offset += 16;
        }
        
        Ok(L2SnapshotMessage {
            timestamp_ns,
            symbol_hash,
            sequence,
            bids,
            asks,
        })
    }
}

// Per-symbol sequence tracking
#[derive(Debug, Clone, Default)]
pub struct SymbolSequenceTracker {
    sequences: std::collections::HashMap<u64, u64>, // symbol_hash -> sequence
}

impl SymbolSequenceTracker {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn check_sequence(&mut self, symbol_hash: u64, sequence: u64) -> SequenceCheck {
        let expected = self.sequences.get(&symbol_hash).copied().unwrap_or(0) + 1;
        
        if sequence == expected || (expected == 1 && sequence > 0) {
            self.sequences.insert(symbol_hash, sequence);
            SequenceCheck::Ok
        } else if sequence > expected {
            let gap = sequence - expected;
            self.sequences.insert(symbol_hash, sequence);
            SequenceCheck::Gap(gap)
        } else {
            SequenceCheck::OutOfOrder
        }
    }
    
    pub fn reset(&mut self, symbol_hash: u64) {
        self.sequences.remove(&symbol_hash);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SequenceCheck {
    Ok,
    Gap(u64),
    OutOfOrder,
}

/// V3 pool state information for tick-based liquidity calculations
#[derive(Debug, Clone, AsBytes, FromBytes, FromZeroes)]
#[repr(C)]
pub struct V3PoolState {
    pub current_tick: [u8; 4],          // Current tick (i32)
    pub sqrt_price_x96: [u8; 16],       // Current sqrtPriceX96 (u128) 
    pub active_liquidity: [u8; 16],     // Current active liquidity (u128)
    pub fee_tier: [u8; 4],              // Fee tier in pips (u32)
    pub _padding: [u8; 8],              // Padding to align to 48 bytes
}

impl V3PoolState {
    pub const SIZE: usize = 48;
    
    pub fn new(tick: i32, sqrt_price_x96: u128, liquidity: u128, fee: u32) -> Self {
        let mut state = Self::new_zeroed();
        LittleEndian::write_i32(&mut state.current_tick, tick);
        LittleEndian::write_u128(&mut state.sqrt_price_x96, sqrt_price_x96);
        LittleEndian::write_u128(&mut state.active_liquidity, liquidity);
        LittleEndian::write_u32(&mut state.fee_tier, fee);
        state
    }
    
    pub fn current_tick(&self) -> i32 {
        LittleEndian::read_i32(&self.current_tick)
    }
    
    pub fn sqrt_price_x96(&self) -> u128 {
        LittleEndian::read_u128(&self.sqrt_price_x96)
    }
    
    pub fn active_liquidity(&self) -> u128 {
        LittleEndian::read_u128(&self.active_liquidity)
    }
    
    pub fn fee_tier(&self) -> u32 {
        LittleEndian::read_u32(&self.fee_tier)
    }
    
    // Setter methods
    pub fn set_sqrt_price_x96(&mut self, value: u128) {
        LittleEndian::write_u128(&mut self.sqrt_price_x96, value);
    }
    
    pub fn set_current_tick(&mut self, value: i32) {
        LittleEndian::write_i32(&mut self.current_tick, value);
    }
    
    pub fn set_active_liquidity(&mut self, value: u128) {
        LittleEndian::write_u128(&mut self.active_liquidity, value);
    }
    
    pub fn set_fee_tier(&mut self, value: u32) {
        LittleEndian::write_u32(&mut self.fee_tier, value);
    }
    
    /// Convert sqrt price X96 to human-readable price
    pub fn price(&self) -> f64 {
        let sqrt_price_x96 = self.sqrt_price_x96() as f64;
        let q96 = 2_f64.powi(96);
        let sqrt_price = sqrt_price_x96 / q96;
        sqrt_price * sqrt_price
    }
}

/// Core swap event data common to all DEX protocols
#[derive(Debug, Clone)]
pub struct SwapEventCore {
    pub timestamp_ns: u64,
    pub pool_id: crate::message_protocol::InstrumentId,
    pub token0_id: crate::message_protocol::InstrumentId,
    pub token1_id: crate::message_protocol::InstrumentId,
    pub tx_hash: String,
    pub block_number: u64,
    pub amount0_in: u128,   // Raw amount in smallest unit
    pub amount1_in: u128,
    pub amount0_out: u128,
    pub amount1_out: u128,
    pub sender: String,
    pub recipient: String,
}

/// Uniswap V2 specific swap data
#[derive(Debug, Clone)]
pub struct UniswapV2SwapEvent {
    pub core: SwapEventCore,
    pub reserves_after: (u128, u128),  // Pool reserves after swap
    pub fee_bps: u32,                  // Fee in basis points
}

/// Uniswap V3 specific swap data
#[derive(Debug, Clone)]
pub struct UniswapV3SwapEvent {
    pub core: SwapEventCore,
    pub sqrt_price_x96: u128,
    pub tick: i32,
    pub liquidity: u128,
    pub fee_tier: u32,
}

/// Curve specific swap data (for future extension)
#[derive(Debug, Clone)]
pub struct CurveSwapEvent {
    pub core: SwapEventCore,
    pub virtual_price: u128,
    pub amplification: u64,
    pub n_coins: u8,
}

/// Unified swap event enum for type-safe handling
#[derive(Debug, Clone)]
pub enum SwapEvent {
    UniswapV2(UniswapV2SwapEvent),
    UniswapV3(UniswapV3SwapEvent),
    Curve(CurveSwapEvent),
    // Add more DEX types as needed
}

/// Common interface all swap events must implement
pub trait SwapEventTrait {
    /// Get the core swap data
    fn core(&self) -> &SwapEventCore;
    
    /// Get the protocol type
    fn protocol_type(&self) -> ProtocolType;
    
    /// Convert to wire format for transmission
    fn to_message(&self) -> SwapEventMessage;
    
    /// Calculate normalized amounts (with decimals applied)
    fn amount0_normalized(&self, decimals0: u8) -> f64 {
        let core = self.core();
        let amount_net = core.amount0_in as f64 - core.amount0_out as f64;
        amount_net / (10_f64.powi(decimals0 as i32))
    }
    
    fn amount1_normalized(&self, decimals1: u8) -> f64 {
        let core = self.core();
        let amount_net = core.amount1_in as f64 - core.amount1_out as f64;
        amount_net / (10_f64.powi(decimals1 as i32))
    }
}

/// Protocol types for different DEX implementations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolType {
    UniswapV2,
    UniswapV3,
    Curve,
    Balancer,
}

impl SwapEventTrait for UniswapV2SwapEvent {
    fn core(&self) -> &SwapEventCore { &self.core }
    fn protocol_type(&self) -> ProtocolType { ProtocolType::UniswapV2 }
    fn to_message(&self) -> SwapEventMessage {
        let mut msg = SwapEventMessage::new_zeroed();
        let core = &self.core;
        msg.set_timestamp_ns(core.timestamp_ns);
        // Use InstrumentId's u64 representation as hash
        msg.set_pool_address_hash(core.pool_id.to_u64());
        msg.set_token0_hash(core.token0_id.to_u64());
        msg.set_token1_hash(core.token1_id.to_u64());
        
        // Convert to fixed point (8 decimals) for wire format
        msg.set_amount0_in((core.amount0_in / 10_u128.pow(10)) as u64); // Assuming 18 decimals, scale to 8
        msg.set_amount1_in((core.amount1_in / 10_u128.pow(10)) as u64);
        msg.set_amount0_out((core.amount0_out / 10_u128.pow(10)) as u64);
        msg.set_amount1_out((core.amount1_out / 10_u128.pow(10)) as u64);
        msg.pool_type = 1; // V2
        msg
    }
}

impl SwapEventTrait for UniswapV3SwapEvent {
    fn core(&self) -> &SwapEventCore { &self.core }
    fn protocol_type(&self) -> ProtocolType { ProtocolType::UniswapV3 }
    fn to_message(&self) -> SwapEventMessage {
        let mut msg = SwapEventMessage::new_zeroed();
        let core = &self.core;
        msg.set_timestamp_ns(core.timestamp_ns);
        // Use InstrumentId's u64 representation as hash
        msg.set_pool_address_hash(core.pool_id.to_u64());
        msg.set_token0_hash(core.token0_id.to_u64());
        msg.set_token1_hash(core.token1_id.to_u64());
        
        // Convert to fixed point for wire format
        msg.set_amount0_in((core.amount0_in / 10_u128.pow(10)) as u64);
        msg.set_amount1_in((core.amount1_in / 10_u128.pow(10)) as u64);
        msg.set_amount0_out((core.amount0_out / 10_u128.pow(10)) as u64);
        msg.set_amount1_out((core.amount1_out / 10_u128.pow(10)) as u64);
        
        // Set V3-specific state
        msg.v3_state.set_sqrt_price_x96(self.sqrt_price_x96);
        msg.v3_state.set_current_tick(self.tick);
        msg.v3_state.set_active_liquidity(self.liquidity);
        msg.v3_state.set_fee_tier(self.fee_tier);
        msg.pool_type = 2; // V3
        msg
    }
}

impl SwapEventTrait for CurveSwapEvent {
    fn core(&self) -> &SwapEventCore { &self.core }
    fn protocol_type(&self) -> ProtocolType { ProtocolType::Curve }
    
    fn to_message(&self) -> SwapEventMessage {
        let mut msg = SwapEventMessage::new_zeroed();
        let core = &self.core;
        msg.set_timestamp_ns(core.timestamp_ns);
        // Use InstrumentId's u64 representation as hash
        msg.set_pool_address_hash(core.pool_id.to_u64());
        msg.set_token0_hash(core.token0_id.to_u64());
        msg.set_token1_hash(core.token1_id.to_u64());
        msg.set_amount0_in((core.amount0_in / 10_u128.pow(10)) as u64);
        msg.set_amount1_in((core.amount1_in / 10_u128.pow(10)) as u64);
        msg.set_amount0_out((core.amount0_out / 10_u128.pow(10)) as u64);
        msg.set_amount1_out((core.amount1_out / 10_u128.pow(10)) as u64);
        msg.pool_type = 3; // Curve
        msg
    }
}

// Helper for unified processing
impl SwapEvent {
    pub fn to_message(&self) -> SwapEventMessage {
        match self {
            SwapEvent::UniswapV2(e) => e.to_message(),
            SwapEvent::UniswapV3(e) => e.to_message(),
            SwapEvent::Curve(e) => e.to_message(),
        }
    }
    
    pub fn core(&self) -> &SwapEventCore {
        match self {
            SwapEvent::UniswapV2(e) => &e.core,
            SwapEvent::UniswapV3(e) => &e.core,
            SwapEvent::Curve(e) => &e.core,
        }
    }
}

#[derive(Debug, Clone, AsBytes, FromBytes, FromZeroes)]
#[repr(C)]
pub struct SwapEventMessage {
    pub timestamp_ns: [u8; 8],
    pub pool_address_hash: [u8; 8],     // Hash of pool address
    pub token0_hash: [u8; 8],           // Hash of token0 symbol
    pub token1_hash: [u8; 8],           // Hash of token1 symbol
    pub amount0_in: [u8; 8],            // Fixed point
    pub amount1_in: [u8; 8],            // Fixed point  
    pub amount0_out: [u8; 8],           // Fixed point
    pub amount1_out: [u8; 8],           // Fixed point
    pub v3_state: V3PoolState,          // V3-specific state (48 bytes)
    pub pool_type: u8,                  // V2=1, V3=2
    pub _padding: [u8; 15],             // Padding to align to 128 bytes total
}

impl SwapEventMessage {
    pub const SIZE: usize = 128; // 8*8 + 48 + 8 = 128 bytes
    
    /// DEPRECATED: Hash a pool address to a u64 for legacy compatibility only
    /// Use bijective InstrumentId system instead  
    #[deprecated(note = "Use InstrumentId system instead")]
    pub fn hash_pool_address(address: &str) -> u64 {
        // Simple hash for legacy compatibility - bijective IDs are preferred
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        address.hash(&mut hasher);
        hasher.finish()
    }
    
    /// DEPRECATED: Hash a token symbol to a u64 for legacy compatibility only
    /// Use bijective InstrumentId system instead
    #[deprecated(note = "Use InstrumentId system instead")]  
    pub fn hash_token_symbol(symbol: &str) -> u64 {
        // Simple hash for legacy compatibility - bijective IDs are preferred
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        symbol.hash(&mut hasher);
        hasher.finish()
    }
    
    // Setter methods for SwapEvent integration
    pub fn set_timestamp_ns(&mut self, value: u64) {
        LittleEndian::write_u64(&mut self.timestamp_ns, value);
    }
    
    pub fn set_pool_address_hash(&mut self, value: u64) {
        LittleEndian::write_u64(&mut self.pool_address_hash, value);
    }
    
    pub fn set_token0_hash(&mut self, value: u64) {
        LittleEndian::write_u64(&mut self.token0_hash, value);
    }
    
    pub fn set_token1_hash(&mut self, value: u64) {
        LittleEndian::write_u64(&mut self.token1_hash, value);
    }
    
    pub fn set_amount0_in(&mut self, value: u64) {
        LittleEndian::write_u64(&mut self.amount0_in, value);
    }
    
    pub fn set_amount1_in(&mut self, value: u64) {
        LittleEndian::write_u64(&mut self.amount1_in, value);
    }
    
    pub fn set_amount0_out(&mut self, value: u64) {
        LittleEndian::write_u64(&mut self.amount0_out, value);
    }
    
    pub fn set_amount1_out(&mut self, value: u64) {
        LittleEndian::write_u64(&mut self.amount1_out, value);
    }
    
    pub fn new_v2(timestamp_ns: u64, pool_hash: u64, token0_hash: u64, token1_hash: u64, 
                  amount0_in: u64, amount1_in: u64, amount0_out: u64, amount1_out: u64) -> Self {
        let mut msg = Self::new_zeroed();
        LittleEndian::write_u64(&mut msg.timestamp_ns, timestamp_ns);
        LittleEndian::write_u64(&mut msg.pool_address_hash, pool_hash);
        LittleEndian::write_u64(&mut msg.token0_hash, token0_hash);
        LittleEndian::write_u64(&mut msg.token1_hash, token1_hash);
        LittleEndian::write_u64(&mut msg.amount0_in, amount0_in);
        LittleEndian::write_u64(&mut msg.amount1_in, amount1_in);
        LittleEndian::write_u64(&mut msg.amount0_out, amount0_out);
        LittleEndian::write_u64(&mut msg.amount1_out, amount1_out);
        msg.pool_type = 1; // V2
        // v3_state remains zeroed for V2 pools
        msg
    }
    
    pub fn new_v3(timestamp_ns: u64, pool_hash: u64, token0_hash: u64, token1_hash: u64,
                  amount0_in: u64, amount1_in: u64, amount0_out: u64, amount1_out: u64, 
                  tick: i32, sqrt_price_x96: u128, liquidity: u128, fee: u32) -> Self {
        let mut msg = Self::new_zeroed();
        LittleEndian::write_u64(&mut msg.timestamp_ns, timestamp_ns);
        LittleEndian::write_u64(&mut msg.pool_address_hash, pool_hash);
        LittleEndian::write_u64(&mut msg.token0_hash, token0_hash);
        LittleEndian::write_u64(&mut msg.token1_hash, token1_hash);
        LittleEndian::write_u64(&mut msg.amount0_in, amount0_in);
        LittleEndian::write_u64(&mut msg.amount1_in, amount1_in);
        LittleEndian::write_u64(&mut msg.amount0_out, amount0_out);
        LittleEndian::write_u64(&mut msg.amount1_out, amount1_out);
        msg.v3_state = V3PoolState::new(tick, sqrt_price_x96, liquidity, fee);
        msg.pool_type = 2; // V3
        msg
    }
    
    pub fn timestamp_ns(&self) -> u64 {
        LittleEndian::read_u64(&self.timestamp_ns)
    }
    
    pub fn pool_address_hash(&self) -> u64 {
        LittleEndian::read_u64(&self.pool_address_hash)
    }
    
    pub fn token0_hash(&self) -> u64 {
        LittleEndian::read_u64(&self.token0_hash)
    }
    
    pub fn token1_hash(&self) -> u64 {
        LittleEndian::read_u64(&self.token1_hash)
    }
    
    pub fn amount0_in(&self) -> u64 {
        LittleEndian::read_u64(&self.amount0_in)
    }
    
    pub fn amount1_in(&self) -> u64 {
        LittleEndian::read_u64(&self.amount1_in)
    }
    
    pub fn amount0_out(&self) -> u64 {
        LittleEndian::read_u64(&self.amount0_out)
    }
    
    pub fn amount1_out(&self) -> u64 {
        LittleEndian::read_u64(&self.amount1_out)
    }
    
    pub fn is_v3(&self) -> bool {
        self.pool_type == 2
    }
    
    pub fn is_v2(&self) -> bool {
        self.pool_type == 1
    }
}

/// Core pool update data common to all DEX protocols
#[derive(Debug, Clone)]
pub struct PoolUpdateCore {
    pub timestamp_ns: u64,
    pub pool_id: crate::message_protocol::InstrumentId,
    pub token0_id: crate::message_protocol::InstrumentId,
    pub token1_id: crate::message_protocol::InstrumentId,
    pub tx_hash: String,
    pub block_number: u64,
    pub update_type: PoolUpdateType,
    pub reserves0_before: u128,  // Pool reserves before update
    pub reserves1_before: u128,
    pub reserves0_after: u128,   // Pool reserves after update  
    pub reserves1_after: u128,
    pub sender: String,          // Transaction sender
}

/// Types of pool updates/events
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PoolUpdateType {
    Mint = 1,           // Liquidity mint events
    Burn = 2,           // Liquidity burn events  
    Collect = 3,        // Fee collection events (V3)
    Flash = 4,          // Flash loan events
    Sync = 5,           // Reserve synchronization (V2)
    FeeChange = 6,      // Fee tier adjustments
    PriceUpdate = 7,    // Significant price movement
    ProtocolFeeChange = 8,
    Pause = 9,          // Pool paused/unpaused
}

// Legacy aliases for backward compatibility
pub const LIQUIDITY_ADD: PoolUpdateType = PoolUpdateType::Mint;
pub const LIQUIDITY_REMOVE: PoolUpdateType = PoolUpdateType::Burn;

/// Uniswap V2 specific pool update data
#[derive(Debug, Clone)]
pub struct V2PoolUpdate {
    pub core: PoolUpdateCore,
    pub liquidity_minted: u128,  // LP tokens minted (for Mint events)
    pub liquidity_burned: u128,  // LP tokens burned (for Burn events)
    pub fee_bps: u32,           // Fee in basis points
    pub k_value: u128,          // Constant product after update
}

/// Uniswap V3 specific pool update data
#[derive(Debug, Clone)]
pub struct V3PoolUpdate {
    pub core: PoolUpdateCore,
    pub sqrt_price_x96: u128,   // Price after update
    pub tick: i32,              // Current tick after update
    pub liquidity: u128,        // Active liquidity after update
    pub fee_tier: u32,          // Fee tier in basis points
    pub tick_lower: i32,        // Lower tick for position (Mint/Burn)
    pub tick_upper: i32,        // Upper tick for position (Mint/Burn)
    pub amount: u128,           // Amount of liquidity added/removed
}

/// Curve specific pool update data (for future extension)
#[derive(Debug, Clone)]
pub struct CurvePoolUpdate {
    pub core: PoolUpdateCore,
    pub virtual_price: u128,
    pub amplification: u64,
    pub admin_fee: u32,
    pub n_coins: u8,
    pub balances: Vec<u128>,    // All token balances after update
}

/// Balancer specific pool update data (for future extension)
#[derive(Debug, Clone)]
pub struct BalancerPoolUpdate {
    pub core: PoolUpdateCore,
    pub weights: Vec<u32>,      // Token weights
    pub swap_fee: u32,
    pub amp_factor: Option<u64>, // For stable pools
    pub tokens_added: Vec<u128>, // Amounts added per token
    pub tokens_removed: Vec<u128>, // Amounts removed per token
}

/// Unified pool update enum for type-safe handling
#[derive(Debug, Clone)]
pub enum PoolUpdate {
    UniswapV2(V2PoolUpdate),
    UniswapV3(V3PoolUpdate),
    Curve(CurvePoolUpdate),
    Balancer(BalancerPoolUpdate),
}

/// Common interface all pool updates must implement
pub trait PoolUpdateTrait {
    /// Get the core pool update data
    fn core(&self) -> &PoolUpdateCore;
    
    /// Get the protocol type
    fn protocol_type(&self) -> ProtocolType;
    
    /// Convert to wire format for transmission
    fn to_message(&self) -> PoolUpdateMessage;
    
    /// Calculate USD value of liquidity change
    fn liquidity_change_usd(&self, token0_price: f64, token1_price: f64, token0_decimals: u8, token1_decimals: u8) -> f64 {
        let core = self.core();
        let token0_change = (core.reserves0_after as f64 - core.reserves0_before as f64) / (10_f64.powi(token0_decimals as i32));
        let token1_change = (core.reserves1_after as f64 - core.reserves1_before as f64) / (10_f64.powi(token1_decimals as i32));
        (token0_change * token0_price) + (token1_change * token1_price)
    }
}

impl PoolUpdateTrait for V2PoolUpdate {
    fn core(&self) -> &PoolUpdateCore { &self.core }
    fn protocol_type(&self) -> ProtocolType { ProtocolType::UniswapV2 }
    fn to_message(&self) -> PoolUpdateMessage {
        let mut msg = PoolUpdateMessage::new_zeroed();
        let core = &self.core;
        msg.set_timestamp_ns(core.timestamp_ns);
        msg.set_pool_hash(core.pool_id.to_u64());
        msg.update_type = core.update_type as u8;
        msg.protocol_type = 1; // V2
        
        // Pack V2-specific data into data field
        let mut offset = 0;
        LittleEndian::write_u128(&mut msg.data[offset..], self.liquidity_minted);
        offset += 16;
        LittleEndian::write_u128(&mut msg.data[offset..], self.liquidity_burned);
        offset += 16;
        LittleEndian::write_u32(&mut msg.data[offset..], self.fee_bps);
        offset += 4;
        LittleEndian::write_u128(&mut msg.data[offset..], self.k_value);
        
        msg
    }
}

impl PoolUpdateTrait for V3PoolUpdate {
    fn core(&self) -> &PoolUpdateCore { &self.core }
    fn protocol_type(&self) -> ProtocolType { ProtocolType::UniswapV3 }
    fn to_message(&self) -> PoolUpdateMessage {
        let mut msg = PoolUpdateMessage::new_zeroed();
        let core = &self.core;
        msg.set_timestamp_ns(core.timestamp_ns);
        msg.set_pool_hash(core.pool_id.to_u64());
        msg.update_type = core.update_type as u8;
        msg.protocol_type = 2; // V3
        
        // Pack V3-specific data into data field
        let mut offset = 0;
        LittleEndian::write_u128(&mut msg.data[offset..], self.sqrt_price_x96);
        offset += 16;
        LittleEndian::write_i32(&mut msg.data[offset..], self.tick);
        offset += 4;
        LittleEndian::write_u128(&mut msg.data[offset..], self.liquidity);
        offset += 16;
        LittleEndian::write_u32(&mut msg.data[offset..], self.fee_tier);
        offset += 4;
        LittleEndian::write_i32(&mut msg.data[offset..], self.tick_lower);
        offset += 4;
        LittleEndian::write_i32(&mut msg.data[offset..], self.tick_upper);
        offset += 4;
        LittleEndian::write_u128(&mut msg.data[offset..], self.amount);
        
        msg
    }
}

impl PoolUpdateTrait for CurvePoolUpdate {
    fn core(&self) -> &PoolUpdateCore { &self.core }
    fn protocol_type(&self) -> ProtocolType { ProtocolType::Curve }
    fn to_message(&self) -> PoolUpdateMessage {
        let mut msg = PoolUpdateMessage::new_zeroed();
        let core = &self.core;
        msg.set_timestamp_ns(core.timestamp_ns);
        msg.set_pool_hash(core.pool_id.to_u64());
        msg.update_type = core.update_type as u8;
        msg.protocol_type = 3; // Curve
        
        // Pack Curve-specific data
        msg.data[0] = self.n_coins;
        let mut offset = 1;
        
        LittleEndian::write_u128(&mut msg.data[offset..], self.virtual_price);
        offset += 16;
        LittleEndian::write_u64(&mut msg.data[offset..], self.amplification);
        offset += 8;
        LittleEndian::write_u32(&mut msg.data[offset..], self.admin_fee);
        offset += 4;
        
        // Pack balances (up to 8 tokens max)
        for (i, &balance) in self.balances.iter().take(8).enumerate() {
            LittleEndian::write_u128(&mut msg.data[offset + i * 16..], balance);
        }
        
        msg
    }
}

impl PoolUpdateTrait for BalancerPoolUpdate {
    fn core(&self) -> &PoolUpdateCore { &self.core }
    fn protocol_type(&self) -> ProtocolType { ProtocolType::Balancer }
    fn to_message(&self) -> PoolUpdateMessage {
        let mut msg = PoolUpdateMessage::new_zeroed();
        let core = &self.core;
        msg.set_timestamp_ns(core.timestamp_ns);
        msg.set_pool_hash(core.pool_id.to_u64());
        msg.update_type = core.update_type as u8;
        msg.protocol_type = 4; // Balancer
        
        // Pack Balancer-specific data
        let mut offset = 0;
        LittleEndian::write_u32(&mut msg.data[offset..], self.swap_fee);
        offset += 4;
        
        if let Some(amp) = self.amp_factor {
            msg.data[offset] = 1; // Has amp factor
            offset += 1;
            LittleEndian::write_u64(&mut msg.data[offset..], amp);
            offset += 8;
        } else {
            msg.data[offset] = 0; // No amp factor
            offset += 9;
        }
        
        // Pack weights and amounts (limited by available space)
        let max_tokens = std::cmp::min(self.weights.len(), 4);
        msg.data[offset] = max_tokens as u8;
        offset += 1;
        
        for i in 0..max_tokens {
            LittleEndian::write_u32(&mut msg.data[offset..], self.weights[i]);
            offset += 4;
            if i < self.tokens_added.len() {
                LittleEndian::write_u128(&mut msg.data[offset..], self.tokens_added[i]);
            }
            offset += 16;
            if i < self.tokens_removed.len() {
                LittleEndian::write_u128(&mut msg.data[offset..], self.tokens_removed[i]);
            }
            offset += 16;
        }
        
        msg
    }
}

/// Helper for unified processing
impl PoolUpdate {
    pub fn to_message(&self) -> PoolUpdateMessage {
        match self {
            PoolUpdate::UniswapV2(e) => e.to_message(),
            PoolUpdate::UniswapV3(e) => e.to_message(),
            PoolUpdate::Curve(e) => e.to_message(),
            PoolUpdate::Balancer(e) => e.to_message(),
        }
    }
    
    pub fn core(&self) -> &PoolUpdateCore {
        match self {
            PoolUpdate::UniswapV2(e) => &e.core,
            PoolUpdate::UniswapV3(e) => &e.core,
            PoolUpdate::Curve(e) => &e.core,
            PoolUpdate::Balancer(e) => &e.core,
        }
    }
}

// =============================================================================
// POOL EVENT SYSTEM - Following SwapEvent architecture pattern
// =============================================================================

/// Core pool event data common to all pool event types
#[derive(Debug, Clone)]
pub struct PoolEventCore {
    pub timestamp_ns: u64,
    pub pool_id: crate::message_protocol::InstrumentId,
    pub token0_id: crate::message_protocol::InstrumentId,
    pub token1_id: crate::message_protocol::InstrumentId,
    pub tx_hash: String,
    pub block_number: u64,
    pub log_index: u32,
    pub event_type: PoolUpdateType,
    pub sender: String,           // Transaction sender
}

/// Uniswap V2 specific pool events (Mint/Burn/Sync)
#[derive(Debug, Clone)]
pub struct UniswapV2PoolEvent {
    pub core: PoolEventCore,
    pub liquidity: u128,         // LP tokens minted/burned
    pub amount0: u128,           // Token0 amount involved
    pub amount1: u128,           // Token1 amount involved
    pub to: String,              // Recipient address (for Mint events)
    pub reserves0_after: u128,   // Reserve0 after the event (for Sync events)
    pub reserves1_after: u128,   // Reserve1 after the event (for Sync events)
    pub token0_decimals: u8,     // Decimals for token0
    pub token1_decimals: u8,     // Decimals for token1
}

/// Uniswap V3 specific pool events (Mint/Burn/Collect)
#[derive(Debug, Clone)]
pub struct UniswapV3PoolEvent {
    pub core: PoolEventCore,
    pub owner: String,           // Position owner
    pub tick_lower: i32,         // Lower tick of position
    pub tick_upper: i32,         // Upper tick of position
    pub liquidity: u128,         // Liquidity amount minted/burned
    pub amount0: u128,           // Token0 amount
    pub amount1: u128,           // Token1 amount
    // Collect-specific fields
    pub amount0_collected: u128, // Only used for Collect events
    pub amount1_collected: u128, // Only used for Collect events
    // Pool state after event
    pub sqrt_price_x96_after: u128,
    pub tick_after: i32,
    pub liquidity_after: u128,
    pub token0_decimals: u8,     // Decimals for token0
    pub token1_decimals: u8,     // Decimals for token1
}

/// Curve specific pool events (future extension)
#[derive(Debug, Clone)]
pub struct CurvePoolEvent {
    pub core: PoolEventCore,
    pub provider: String,        // Liquidity provider
    pub token_amounts: Vec<u128>, // Amounts for each token in pool
    pub fees: Vec<u128>,         // Fees collected per token
    pub invariant: u128,         // Pool invariant after event
    pub token_supply: u128,      // Total LP token supply
    pub balances_after: Vec<u128>, // Token balances after event
}

/// Balancer specific pool events (future extension)
#[derive(Debug, Clone)]
pub struct BalancerPoolEvent {
    pub core: PoolEventCore,
    pub liquidity_provider: String,
    pub tokens_in: Vec<u128>,    // Amounts of each token added
    pub tokens_out: Vec<u128>,   // Amounts of each token removed
    pub pool_tokens_minted: u128, // BPT minted
    pub pool_tokens_burned: u128, // BPT burned
    pub pool_total_supply: u128, // Total BPT supply after event
    pub weights: Vec<u32>,       // Token weights (for weighted pools)
}

/// Unified pool event enum for type-safe handling
#[derive(Debug, Clone)]
pub enum PoolEvent {
    UniswapV2Mint(UniswapV2PoolEvent),
    UniswapV2Burn(UniswapV2PoolEvent),
    UniswapV2Sync(UniswapV2PoolEvent),
    UniswapV3Mint(UniswapV3PoolEvent),
    UniswapV3Burn(UniswapV3PoolEvent),
    UniswapV3Collect(UniswapV3PoolEvent),
    CurveMint(CurvePoolEvent),
    CurveBurn(CurvePoolEvent),
    BalancerMint(BalancerPoolEvent),
    BalancerBurn(BalancerPoolEvent),
}

/// Common interface all pool events must implement
pub trait PoolEventTrait {
    /// Get the core pool event data
    fn core(&self) -> &PoolEventCore;
    
    /// Get the protocol type
    fn protocol_type(&self) -> ProtocolType;
    
    /// Get the specific event type
    fn event_type(&self) -> PoolUpdateType;
    
    /// Convert to wire format for transmission
    fn to_message(&self) -> PoolUpdateMessage;
    
    /// Calculate USD value of liquidity change
    fn liquidity_change_usd(&self, token0_price: f64, token1_price: f64, token0_decimals: u8, token1_decimals: u8) -> f64;
    
    /// Get normalized token amounts (with decimals applied)
    fn amount0_normalized(&self, decimals0: u8) -> f64;
    fn amount1_normalized(&self, decimals1: u8) -> f64;
    
    /// Check if this is a liquidity-providing event (Mint)
    fn is_liquidity_add(&self) -> bool {
        matches!(self.event_type(), PoolUpdateType::Mint)
    }
    
    /// Check if this is a liquidity-removing event (Burn)
    fn is_liquidity_remove(&self) -> bool {
        matches!(self.event_type(), PoolUpdateType::Burn)
    }
}

impl PoolEventTrait for UniswapV2PoolEvent {
    fn core(&self) -> &PoolEventCore { &self.core }
    fn protocol_type(&self) -> ProtocolType { ProtocolType::UniswapV2 }
    fn event_type(&self) -> PoolUpdateType { self.core.event_type }
    
    fn to_message(&self) -> PoolUpdateMessage {
        let mut msg = PoolUpdateMessage::new_zeroed();
        let core = &self.core;
        msg.set_timestamp_ns(core.timestamp_ns);
        msg.set_pool_hash(core.pool_id.to_u64());
        msg.update_type = core.event_type as u8;
        msg.protocol_type = 1; // V2
        
        // Pack V2 event data into data field
        let mut offset = 0;
        LittleEndian::write_u128(&mut msg.data[offset..], self.liquidity);
        offset += 16;
        LittleEndian::write_u128(&mut msg.data[offset..], self.amount0);
        offset += 16;
        LittleEndian::write_u128(&mut msg.data[offset..], self.amount1);
        offset += 16;
        LittleEndian::write_u128(&mut msg.data[offset..], self.reserves0_after);
        offset += 16;
        LittleEndian::write_u128(&mut msg.data[offset..], self.reserves1_after);
        offset += 16;
        // Add token decimals at offset 80 and 81
        msg.data[80] = self.token0_decimals;
        msg.data[81] = self.token1_decimals;
        
        msg
    }
    
    fn liquidity_change_usd(&self, token0_price: f64, token1_price: f64, token0_decimals: u8, token1_decimals: u8) -> f64 {
        let amount0_norm = self.amount0_normalized(token0_decimals);
        let amount1_norm = self.amount1_normalized(token1_decimals);
        (amount0_norm * token0_price) + (amount1_norm * token1_price)
    }
    
    fn amount0_normalized(&self, decimals0: u8) -> f64 {
        self.amount0 as f64 / (10_f64.powi(decimals0 as i32))
    }
    
    fn amount1_normalized(&self, decimals1: u8) -> f64 {
        self.amount1 as f64 / (10_f64.powi(decimals1 as i32))
    }
}

impl PoolEventTrait for UniswapV3PoolEvent {
    fn core(&self) -> &PoolEventCore { &self.core }
    fn protocol_type(&self) -> ProtocolType { ProtocolType::UniswapV3 }
    fn event_type(&self) -> PoolUpdateType { self.core.event_type }
    
    fn to_message(&self) -> PoolUpdateMessage {
        let mut msg = PoolUpdateMessage::new_zeroed();
        let core = &self.core;
        msg.set_timestamp_ns(core.timestamp_ns);
        msg.set_pool_hash(core.pool_id.to_u64());
        msg.update_type = core.event_type as u8;
        msg.protocol_type = 2; // V3
        
        // Pack V3 event data into data field
        let mut offset = 0;
        LittleEndian::write_i32(&mut msg.data[offset..], self.tick_lower);
        offset += 4;
        LittleEndian::write_i32(&mut msg.data[offset..], self.tick_upper);
        offset += 4;
        LittleEndian::write_u128(&mut msg.data[offset..], self.liquidity);
        offset += 16;
        LittleEndian::write_u128(&mut msg.data[offset..], self.amount0);
        offset += 16;
        LittleEndian::write_u128(&mut msg.data[offset..], self.amount1);
        offset += 16;
        LittleEndian::write_u128(&mut msg.data[offset..], self.amount0_collected);
        offset += 16;
        LittleEndian::write_u128(&mut msg.data[offset..], self.amount1_collected);
        offset += 16;
        LittleEndian::write_u128(&mut msg.data[offset..], self.sqrt_price_x96_after);
        offset += 16;
        LittleEndian::write_i32(&mut msg.data[offset..], self.tick_after);
        offset += 4;
        LittleEndian::write_u128(&mut msg.data[offset..], self.liquidity_after);
        
        msg
    }
    
    fn liquidity_change_usd(&self, token0_price: f64, token1_price: f64, token0_decimals: u8, token1_decimals: u8) -> f64 {
        let amount0_norm = self.amount0_normalized(token0_decimals);
        let amount1_norm = self.amount1_normalized(token1_decimals);
        (amount0_norm * token0_price) + (amount1_norm * token1_price)
    }
    
    fn amount0_normalized(&self, decimals0: u8) -> f64 {
        self.amount0 as f64 / (10_f64.powi(decimals0 as i32))
    }
    
    fn amount1_normalized(&self, decimals1: u8) -> f64 {
        self.amount1 as f64 / (10_f64.powi(decimals1 as i32))
    }
}

impl PoolEventTrait for CurvePoolEvent {
    fn core(&self) -> &PoolEventCore { &self.core }
    fn protocol_type(&self) -> ProtocolType { ProtocolType::Curve }
    fn event_type(&self) -> PoolUpdateType { self.core.event_type }
    
    fn to_message(&self) -> PoolUpdateMessage {
        let mut msg = PoolUpdateMessage::new_zeroed();
        let core = &self.core;
        msg.set_timestamp_ns(core.timestamp_ns);
        msg.set_pool_hash(core.pool_id.to_u64());
        msg.update_type = core.event_type as u8;
        msg.protocol_type = 3; // Curve
        
        // Pack Curve event data (simplified for now)
        let mut offset = 0;
        LittleEndian::write_u128(&mut msg.data[offset..], self.invariant);
        offset += 16;
        LittleEndian::write_u128(&mut msg.data[offset..], self.token_supply);
        offset += 16;
        
        // Pack first few token amounts (up to 8 tokens)
        for (i, &amount) in self.token_amounts.iter().take(8).enumerate() {
            LittleEndian::write_u128(&mut msg.data[offset + i * 16..], amount);
        }
        
        msg
    }
    
    fn liquidity_change_usd(&self, token0_price: f64, token1_price: f64, token0_decimals: u8, token1_decimals: u8) -> f64 {
        // For Curve, calculate based on first two tokens
        if self.token_amounts.len() >= 2 {
            let amount0_norm = self.token_amounts[0] as f64 / (10_f64.powi(token0_decimals as i32));
            let amount1_norm = self.token_amounts[1] as f64 / (10_f64.powi(token1_decimals as i32));
            (amount0_norm * token0_price) + (amount1_norm * token1_price)
        } else {
            0.0
        }
    }
    
    fn amount0_normalized(&self, decimals0: u8) -> f64 {
        self.token_amounts.get(0).map_or(0.0, |&amount| {
            amount as f64 / (10_f64.powi(decimals0 as i32))
        })
    }
    
    fn amount1_normalized(&self, decimals1: u8) -> f64 {
        self.token_amounts.get(1).map_or(0.0, |&amount| {
            amount as f64 / (10_f64.powi(decimals1 as i32))
        })
    }
}

impl PoolEventTrait for BalancerPoolEvent {
    fn core(&self) -> &PoolEventCore { &self.core }
    fn protocol_type(&self) -> ProtocolType { ProtocolType::Balancer }
    fn event_type(&self) -> PoolUpdateType { self.core.event_type }
    
    fn to_message(&self) -> PoolUpdateMessage {
        let mut msg = PoolUpdateMessage::new_zeroed();
        let core = &self.core;
        msg.set_timestamp_ns(core.timestamp_ns);
        msg.set_pool_hash(core.pool_id.to_u64());
        msg.update_type = core.event_type as u8;
        msg.protocol_type = 4; // Balancer
        
        // Pack Balancer event data (simplified for now)
        let mut offset = 0;
        LittleEndian::write_u128(&mut msg.data[offset..], self.pool_tokens_minted);
        offset += 16;
        LittleEndian::write_u128(&mut msg.data[offset..], self.pool_tokens_burned);
        offset += 16;
        LittleEndian::write_u128(&mut msg.data[offset..], self.pool_total_supply);
        offset += 16;
        
        // Pack first few token amounts
        for (i, &amount) in self.tokens_in.iter().take(4).enumerate() {
            LittleEndian::write_u128(&mut msg.data[offset + i * 16..], amount);
        }
        
        msg
    }
    
    fn liquidity_change_usd(&self, token0_price: f64, token1_price: f64, token0_decimals: u8, token1_decimals: u8) -> f64 {
        // For Balancer, calculate based on first two tokens
        let amount0_norm = self.tokens_in.get(0).unwrap_or(&0) - self.tokens_out.get(0).unwrap_or(&0);
        let amount1_norm = self.tokens_in.get(1).unwrap_or(&0) - self.tokens_out.get(1).unwrap_or(&0);
        
        let amount0_f = amount0_norm as f64 / (10_f64.powi(token0_decimals as i32));
        let amount1_f = amount1_norm as f64 / (10_f64.powi(token1_decimals as i32));
        (amount0_f * token0_price) + (amount1_f * token1_price)
    }
    
    fn amount0_normalized(&self, decimals0: u8) -> f64 {
        let net_amount = self.tokens_in.get(0).unwrap_or(&0) - self.tokens_out.get(0).unwrap_or(&0);
        net_amount as f64 / (10_f64.powi(decimals0 as i32))
    }
    
    fn amount1_normalized(&self, decimals1: u8) -> f64 {
        let net_amount = self.tokens_in.get(1).unwrap_or(&0) - self.tokens_out.get(1).unwrap_or(&0);
        net_amount as f64 / (10_f64.powi(decimals1 as i32))
    }
}

// Helper for unified processing
impl PoolEvent {
    pub fn to_message(&self) -> PoolUpdateMessage {
        match self {
            PoolEvent::UniswapV2Mint(e) => e.to_message(),
            PoolEvent::UniswapV2Burn(e) => e.to_message(),
            PoolEvent::UniswapV2Sync(e) => e.to_message(),
            PoolEvent::UniswapV3Mint(e) => e.to_message(),
            PoolEvent::UniswapV3Burn(e) => e.to_message(),
            PoolEvent::UniswapV3Collect(e) => e.to_message(),
            PoolEvent::CurveMint(e) => e.to_message(),
            PoolEvent::CurveBurn(e) => e.to_message(),
            PoolEvent::BalancerMint(e) => e.to_message(),
            PoolEvent::BalancerBurn(e) => e.to_message(),
        }
    }
    
    pub fn core(&self) -> &PoolEventCore {
        match self {
            PoolEvent::UniswapV2Mint(e) => &e.core,
            PoolEvent::UniswapV2Burn(e) => &e.core,
            PoolEvent::UniswapV2Sync(e) => &e.core,
            PoolEvent::UniswapV3Mint(e) => &e.core,
            PoolEvent::UniswapV3Burn(e) => &e.core,
            PoolEvent::UniswapV3Collect(e) => &e.core,
            PoolEvent::CurveMint(e) => &e.core,
            PoolEvent::CurveBurn(e) => &e.core,
            PoolEvent::BalancerMint(e) => &e.core,
            PoolEvent::BalancerBurn(e) => &e.core,
        }
    }
    
    pub fn protocol_type(&self) -> ProtocolType {
        match self {
            PoolEvent::UniswapV2Mint(_) | PoolEvent::UniswapV2Burn(_) | PoolEvent::UniswapV2Sync(_) => ProtocolType::UniswapV2,
            PoolEvent::UniswapV3Mint(_) | PoolEvent::UniswapV3Burn(_) | PoolEvent::UniswapV3Collect(_) => ProtocolType::UniswapV3,
            PoolEvent::CurveMint(_) | PoolEvent::CurveBurn(_) => ProtocolType::Curve,
            PoolEvent::BalancerMint(_) | PoolEvent::BalancerBurn(_) => ProtocolType::Balancer,
        }
    }
    
    pub fn event_type(&self) -> PoolUpdateType {
        match self {
            PoolEvent::UniswapV2Mint(_) | PoolEvent::UniswapV3Mint(_) | PoolEvent::CurveMint(_) | PoolEvent::BalancerMint(_) => PoolUpdateType::Mint,
            PoolEvent::UniswapV2Burn(_) | PoolEvent::UniswapV3Burn(_) | PoolEvent::CurveBurn(_) | PoolEvent::BalancerBurn(_) => PoolUpdateType::Burn,
            PoolEvent::UniswapV2Sync(_) => PoolUpdateType::Sync,
            PoolEvent::UniswapV3Collect(_) => PoolUpdateType::Collect,
        }
    }
}

/// Wire format message for pool updates
#[derive(Debug, Clone, AsBytes, FromBytes, FromZeroes)]
#[repr(C)]
pub struct PoolUpdateMessage {
    pub timestamp_ns: [u8; 8],
    pub pool_hash: [u8; 8],         // Hash of pool address
    pub update_type: u8,            // PoolUpdateType as u8
    pub protocol_type: u8,          // Protocol identifier (V2=1, V3=2, etc.)
    pub data: [u8; 256],           // Protocol-specific serialized data
    pub _padding: [u8; 6],         // Padding to align to 280 bytes
}

impl PoolUpdateMessage {
    pub const SIZE: usize = 280;
    
    /// DEPRECATED: Hash a pool address to a u64 for legacy compatibility only  
    /// Use bijective InstrumentId system instead
    #[deprecated(note = "Use InstrumentId system instead")]
    pub fn hash_pool_address(address: &str) -> u64 {
        // Simple hash for legacy compatibility - bijective IDs are preferred
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        address.hash(&mut hasher);
        hasher.finish()
    }
    
    // Setter methods for pool update integration
    pub fn set_timestamp_ns(&mut self, value: u64) {
        LittleEndian::write_u64(&mut self.timestamp_ns, value);
    }
    
    pub fn set_pool_hash(&mut self, value: u64) {
        LittleEndian::write_u64(&mut self.pool_hash, value);
    }
    
    // Getter methods
    pub fn timestamp_ns(&self) -> u64 {
        LittleEndian::read_u64(&self.timestamp_ns)
    }
    
    pub fn pool_hash(&self) -> u64 {
        LittleEndian::read_u64(&self.pool_hash)
    }
    
    pub fn is_v2(&self) -> bool {
        self.protocol_type == 1
    }
    
    pub fn is_v3(&self) -> bool {
        self.protocol_type == 2
    }
    
    pub fn is_curve(&self) -> bool {
        self.protocol_type == 3
    }
    
    pub fn is_balancer(&self) -> bool {
        self.protocol_type == 4
    }
}

/// Represents a detected arbitrage opportunity between DEXs
#[derive(Debug, Clone)]
pub struct ArbitrageOpportunityMessage {
    pub timestamp_ns: u64,
    pub pair: String,           // e.g., "WMATIC-USDC"
    pub token_a: String,        // Contract address
    pub token_b: String,        // Contract address
    pub dex_buy: String,        // DEX name for buying
    pub dex_sell: String,       // DEX name for selling
    pub dex_buy_router: String, // Router contract address
    pub dex_sell_router: String,// Router contract address
    pub price_buy: u64,         // Fixed point price
    pub price_sell: u64,        // Fixed point price
    pub estimated_profit: u64,  // Fixed point USD
    pub profit_percent: u64,    // Fixed point percentage
    pub liquidity_buy: u64,     // Fixed point USD
    pub liquidity_sell: u64,    // Fixed point USD
    pub max_trade_size: u64,    // Fixed point USD
    pub gas_estimate: u32,      // Gas units
    pub v3_pool_state_buy: Option<V3PoolState>,  // V3 state for buy side
    pub v3_pool_state_sell: Option<V3PoolState>, // V3 state for sell side
}

impl ArbitrageOpportunityMessage {
    pub fn encode(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        
        // Fixed size fields first
        buffer.extend_from_slice(&self.timestamp_ns.to_le_bytes());
        buffer.extend_from_slice(&self.price_buy.to_le_bytes());
        buffer.extend_from_slice(&self.price_sell.to_le_bytes());
        buffer.extend_from_slice(&self.estimated_profit.to_le_bytes());
        buffer.extend_from_slice(&self.profit_percent.to_le_bytes());
        buffer.extend_from_slice(&self.liquidity_buy.to_le_bytes());
        buffer.extend_from_slice(&self.liquidity_sell.to_le_bytes());
        buffer.extend_from_slice(&self.max_trade_size.to_le_bytes());
        buffer.extend_from_slice(&self.gas_estimate.to_le_bytes());
        
        // V3 pool states (optional)
        if let Some(ref buy_state) = self.v3_pool_state_buy {
            buffer.push(1); // V3 buy state present
            buffer.extend_from_slice(buy_state.as_bytes());
        } else {
            buffer.push(0); // No V3 buy state
            buffer.extend_from_slice(&[0u8; V3PoolState::SIZE]); // Padding
        }
        
        if let Some(ref sell_state) = self.v3_pool_state_sell {
            buffer.push(1); // V3 sell state present  
            buffer.extend_from_slice(sell_state.as_bytes());
        } else {
            buffer.push(0); // No V3 sell state
            buffer.extend_from_slice(&[0u8; V3PoolState::SIZE]); // Padding
        }
        
        // Variable length strings with length prefix
        let strings = vec![
            &self.pair,
            &self.token_a,
            &self.token_b,
            &self.dex_buy,
            &self.dex_sell,
            &self.dex_buy_router,
            &self.dex_sell_router,
        ];
        
        for s in strings {
            buffer.extend_from_slice(&(s.len() as u16).to_le_bytes());
            buffer.extend_from_slice(s.as_bytes());
        }
        
        buffer
    }
    
    pub fn decode(data: &[u8]) -> Result<Self, ProtocolError> {
        let min_size = 68 + 2 + (V3PoolState::SIZE * 2); // Fixed fields + V3 state flags + V3 states
        if data.len() < min_size {
            return Err(ProtocolError::BufferTooSmall { need: min_size, got: data.len() });
        }
        
        let mut offset = 0;
        
        // Read fixed fields
        let timestamp_ns = LittleEndian::read_u64(&data[offset..offset+8]);
        offset += 8;
        let price_buy = LittleEndian::read_u64(&data[offset..offset+8]);
        offset += 8;
        let price_sell = LittleEndian::read_u64(&data[offset..offset+8]);
        offset += 8;
        let estimated_profit = LittleEndian::read_u64(&data[offset..offset+8]);
        offset += 8;
        let profit_percent = LittleEndian::read_u64(&data[offset..offset+8]);
        offset += 8;
        let liquidity_buy = LittleEndian::read_u64(&data[offset..offset+8]);
        offset += 8;
        let liquidity_sell = LittleEndian::read_u64(&data[offset..offset+8]);
        offset += 8;
        let max_trade_size = LittleEndian::read_u64(&data[offset..offset+8]);
        offset += 8;
        let gas_estimate = LittleEndian::read_u32(&data[offset..offset+4]);
        offset += 4;
        
        // Read V3 pool states
        let v3_pool_state_buy = if data[offset] == 1 {
            offset += 1;
            let state = V3PoolState::read_from_prefix(&data[offset..offset + V3PoolState::SIZE])
                .ok_or_else(|| ProtocolError::BufferTooSmall { 
                    need: offset + V3PoolState::SIZE, got: data.len() 
                })?;
            offset += V3PoolState::SIZE;
            Some(state)
        } else {
            offset += 1 + V3PoolState::SIZE; // Skip flag + padding
            None
        };
        
        let v3_pool_state_sell = if data[offset] == 1 {
            offset += 1;
            let state = V3PoolState::read_from_prefix(&data[offset..offset + V3PoolState::SIZE])
                .ok_or_else(|| ProtocolError::BufferTooSmall { 
                    need: offset + V3PoolState::SIZE, got: data.len() 
                })?;
            offset += V3PoolState::SIZE;
            Some(state)
        } else {
            offset += 1 + V3PoolState::SIZE; // Skip flag + padding
            None
        };
        
        // Read variable length strings
        let mut strings = Vec::new();
        for _ in 0..7 {
            if offset + 2 > data.len() {
                return Err(ProtocolError::BufferTooSmall { need: offset + 2, got: data.len() });
            }
            let len = LittleEndian::read_u16(&data[offset..offset+2]) as usize;
            offset += 2;
            
            if offset + len > data.len() {
                return Err(ProtocolError::BufferTooSmall { need: offset + len, got: data.len() });
            }
            
            let s = String::from_utf8_lossy(&data[offset..offset+len]).to_string();
            strings.push(s);
            offset += len;
        }
        
        Ok(Self {
            timestamp_ns,
            pair: strings[0].clone(),
            token_a: strings[1].clone(),
            token_b: strings[2].clone(),
            dex_buy: strings[3].clone(),
            dex_sell: strings[4].clone(),
            dex_buy_router: strings[5].clone(),
            dex_sell_router: strings[6].clone(),
            price_buy,
            price_sell,
            estimated_profit,
            profit_percent,
            liquidity_buy,
            liquidity_sell,
            max_trade_size,
            gas_estimate,
            v3_pool_state_buy,
            v3_pool_state_sell,
        })
    }
}

// StatusUpdateMessage for gas prices, native token price, and system status
#[derive(Debug, Clone, AsBytes, FromBytes, FromZeroes)]
#[repr(C)]
pub struct StatusUpdateMessage {
    pub timestamp_ns: [u8; 8],     // When this was fetched
    pub gas_price_gwei: [u8; 4],   // Current gas price in Gwei
    pub gas_price_fast: [u8; 4],   // Fast gas price in Gwei
    pub gas_price_instant: [u8; 4], // Instant gas price in Gwei
    pub native_price_usd: [u8; 4], // MATIC/ETH price in USD (fixed point * 100)
    pub block_number: [u8; 8],     // Current block number
    pub _padding: [u8; 4],
}

impl StatusUpdateMessage {
    pub const SIZE: usize = 36;
    
    pub fn new(gas_gwei: u32, gas_fast: u32, gas_instant: u32, native_price: f32, block: u64) -> Self {
        let mut msg = Self::new_zeroed();
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64;
        LittleEndian::write_u64(&mut msg.timestamp_ns, now);
        LittleEndian::write_u32(&mut msg.gas_price_gwei, gas_gwei);
        LittleEndian::write_u32(&mut msg.gas_price_fast, gas_fast);
        LittleEndian::write_u32(&mut msg.gas_price_instant, gas_instant);
        LittleEndian::write_u32(&mut msg.native_price_usd, (native_price * 100.0) as u32);
        LittleEndian::write_u64(&mut msg.block_number, block);
        msg
    }
    
    pub fn gas_price_gwei(&self) -> u32 {
        LittleEndian::read_u32(&self.gas_price_gwei)
    }
    
    pub fn native_price(&self) -> f32 {
        LittleEndian::read_u32(&self.native_price_usd) as f32 / 100.0
    }
}

/// PHASE 2: Message trace for deep equality validation
/// Maps message IDs to their associated trade data for end-to-end tracking
#[derive(Debug, Clone)]
pub struct MessageTraceMessage {
    pub message_id: String,         // UUID from Polygon collector
    pub symbol_hash: u64,          // Associated symbol for this message
    pub original_data_hash: String, // SHA-256 hash of original Polygon API data
    pub stage: String,             // Current pipeline stage (collector, relay, bridge)
    pub timestamp_ns: u64,         // When this trace was created
}

impl MessageTraceMessage {
    pub fn new(message_id: String, symbol_hash: u64, original_data_hash: String, stage: String) -> Self {
        let timestamp_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
            
        Self {
            message_id,
            symbol_hash,
            original_data_hash,
            stage,
            timestamp_ns,
        }
    }
    
    pub fn encode(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        
        // Fixed fields first
        buffer.extend_from_slice(&self.symbol_hash.to_le_bytes());
        buffer.extend_from_slice(&self.timestamp_ns.to_le_bytes());
        
        // Variable length strings with length prefix
        let strings = vec![&self.message_id, &self.original_data_hash, &self.stage];
        
        for s in strings {
            buffer.extend_from_slice(&(s.len() as u16).to_le_bytes());
            buffer.extend_from_slice(s.as_bytes());
        }
        
        buffer
    }
    
    pub fn decode(data: &[u8]) -> Result<Self, ProtocolError> {
        if data.len() < 16 { // Minimum size for fixed fields
            return Err(ProtocolError::BufferTooSmall { need: 16, got: data.len() });
        }
        
        let mut offset = 0;
        
        // Read fixed fields
        let symbol_hash = LittleEndian::read_u64(&data[offset..offset+8]);
        offset += 8;
        let timestamp_ns = LittleEndian::read_u64(&data[offset..offset+8]);
        offset += 8;
        
        // Read variable length strings
        let mut strings = Vec::new();
        for _ in 0..3 {
            if offset + 2 > data.len() {
                return Err(ProtocolError::BufferTooSmall { need: offset + 2, got: data.len() });
            }
            let len = LittleEndian::read_u16(&data[offset..offset+2]) as usize;
            offset += 2;
            
            if offset + len > data.len() {
                return Err(ProtocolError::BufferTooSmall { need: offset + len, got: data.len() });
            }
            
            let s = String::from_utf8_lossy(&data[offset..offset+len]).to_string();
            strings.push(s);
            offset += len;
        }
        
        Ok(Self {
            message_id: strings[0].clone(),
            original_data_hash: strings[1].clone(),
            stage: strings[2].clone(),
            symbol_hash,
            timestamp_ns,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_header() {
        let header = MessageHeader::new(MessageType::Trade, 40, 12345);
        assert_eq!(header.magic, MAGIC_BYTE);
        assert_eq!(header.get_length(), 40);
        assert_eq!(header.get_sequence(), 12345);
        assert!(header.validate().is_ok());
    }

    #[test]
    fn test_trade_message() {
        let symbol = SymbolDescriptor::spot("kraken", "BTC", "USD");
        let trade = TradeMessage::new(
            1234567890000000000,
            6500000000000,
            100000000,
            symbol.hash(),
            TradeSide::Buy,
        );
        
        assert_eq!(trade.timestamp_ns(), 1234567890000000000);
        assert_eq!(trade.price_f64(), 65000.0);
        assert_eq!(trade.volume_f64(), 1.0);
        assert_eq!(trade.symbol_hash(), symbol.hash());
        assert_eq!(trade.side(), TradeSide::Buy);
    }

    #[test]
    fn test_numerical_accuracy() {
        // Test price conversion accuracy
        let test_prices = vec![
            (0.00000001, 1u64),
            (0.001, 100000u64),
            (1.0, 100000000u64),
            (100.0, 10000000000u64),
            (65000.0, 6500000000000u64),
            (0.9986, 99860000u64),
        ];

        for (json_price, expected_fp) in test_prices {
            let fixed_point = (json_price * 1e8) as u64;
            assert_eq!(fixed_point, expected_fp);
            
            let trade = TradeMessage::new(
                1234567890000000000,
                fixed_point,
                100000000,
                12345678,
                TradeSide::Unknown,
            );
            
            let decoded_price = trade.price_f64();
            let diff = (decoded_price - json_price).abs();
            assert!(diff < 1e-9, "Price mismatch: {} vs {}", json_price, decoded_price);
        }
    }

    #[test]
    fn test_pool_event_types() {
        // Test UniswapV2PoolEvent creation and trait methods
        let v2_mint = UniswapV2PoolEvent {
            core: PoolEventCore {
                timestamp_ns: 1234567890000000000,
                pool_address: "0x1234567890abcdef".to_string(),
                tx_hash: "0xabcdef".to_string(),
                block_number: 12345,
                log_index: 1,
                token0_address: "0xA0b86a33E6417c39513dD5C05E02Ad8BF3c8E91c".to_string(),
                token1_address: "0xdAC17F958D2ee523a2206206994597C13D831ec7".to_string(),
                token0_symbol: "WETH".to_string(),
                token1_symbol: "USDT".to_string(),
                event_type: PoolUpdateType::Mint,
                sender: "0x456789".to_string(),
            },
            liquidity: 1000000000000000000u128, // 1 ETH worth
            amount0: 1000000000000000000u128,
            amount1: 3000000000u128, // 3000 USDT
            to: "0x789abc".to_string(),
            reserves0_after: 10000000000000000000u128,
            reserves1_after: 30000000000u128,
            token0_decimals: 18,
            token1_decimals: 6,
        };
        
        assert_eq!(v2_mint.core.event_type, PoolUpdateType::Mint);
        assert_eq!(v2_mint.amount0_normalized(18), 1.0);
        assert_eq!(v2_mint.amount1_normalized(6), 3000.0);
        assert!(v2_mint.is_liquidity_add());
        assert!(!v2_mint.is_liquidity_remove());
        
        // Test liquidity change calculation
        let usd_change = v2_mint.liquidity_change_usd(3000.0, 1.0, 18, 6);
        assert!((usd_change - 6000.0).abs() < 1.0); // 1 WETH * 3000 + 3000 USDT * 1 = 6000 USD
        
        // Test UniswapV3PoolEvent creation
        let v3_mint = UniswapV3PoolEvent {
            core: PoolEventCore {
                timestamp_ns: 1234567890000000000,
                pool_address: "0xabcdef1234567890".to_string(),
                tx_hash: "0x123456".to_string(),
                block_number: 12346,
                log_index: 2,
                token0_address: "0xA0b86a33E6417c39513dD5C05E02Ad8BF3c8E91c".to_string(),
                token1_address: "0xdAC17F958D2ee523a2206206994597C13D831ec7".to_string(),
                token0_symbol: "WETH".to_string(),
                token1_symbol: "USDT".to_string(),
                event_type: PoolUpdateType::Mint,
                sender: "0x987654".to_string(),
            },
            owner: "0xowner123".to_string(),
            tick_lower: -887272,
            tick_upper: 887272,
            liquidity: 1000000000000u128,
            amount0: 500000000000000000u128, // 0.5 WETH
            amount1: 1500000000u128, // 1500 USDT
            amount0_collected: 0,
            amount1_collected: 0,
            sqrt_price_x96_after: 2505414483750479251915322u128,
            tick_after: 0,
            liquidity_after: 5000000000000u128,
            token0_decimals: 18,
            token1_decimals: 6,
        };
        
        assert_eq!(v3_mint.core.event_type, PoolUpdateType::Mint);
        assert_eq!(v3_mint.amount0_normalized(18), 0.5);
        assert_eq!(v3_mint.amount1_normalized(6), 1500.0);
        assert!(v3_mint.is_liquidity_add());
        
        // Test PoolEvent enum
        let pool_event = PoolEvent::UniswapV2Mint(v2_mint);
        assert_eq!(pool_event.event_type(), PoolUpdateType::Mint);
        assert_eq!(pool_event.protocol_type(), ProtocolType::UniswapV2);
        
        let v3_event = PoolEvent::UniswapV3Mint(v3_mint);
        assert_eq!(v3_event.event_type(), PoolUpdateType::Mint);
        assert_eq!(v3_event.protocol_type(), ProtocolType::UniswapV3);
    }
    
    #[test]
    fn test_pool_update_message_serialization() {
        // Test that PoolUpdateMessage can be serialized/deserialized
        let v2_mint = UniswapV2PoolEvent {
            core: PoolEventCore {
                timestamp_ns: 1234567890000000000,
                pool_address: "0x1234567890abcdef".to_string(),
                tx_hash: "0xabcdef".to_string(),
                block_number: 12345,
                log_index: 1,
                token0_address: "0xA0b86a33E6417c39513dD5C05E02Ad8BF3c8E91c".to_string(),
                token1_address: "0xdAC17F958D2ee523a2206206994597C13D831ec7".to_string(),
                token0_symbol: "WETH".to_string(),
                token1_symbol: "USDT".to_string(),
                event_type: PoolUpdateType::Mint,
                sender: "0x456789".to_string(),
            },
            liquidity: 1000000000000000000u128,
            amount0: 1000000000000000000u128,
            amount1: 3000000000u128,
            to: "0x789abc".to_string(),
            reserves0_after: 10000000000000000000u128,
            reserves1_after: 30000000000u128,
            token0_decimals: 18,
            token1_decimals: 6,
        };
        
        let message = v2_mint.to_message();
        assert_eq!(message.update_type, PoolUpdateType::Mint as u8);
        assert_eq!(message.protocol_type, 1); // V2
        assert!(message.timestamp_ns() > 0);
    }

    #[test]
    fn test_extreme_dex_prices() {
        // These extreme prices are due to incorrect decimal handling in Polygon collector
        // USDC has 6 decimals, not 18, causing prices to be inflated by 10^12
        // The protocol can handle prices up to ~1.8e11 before u64 overflow
        
        // Test reasonable DEX prices that should work
        let reasonable_prices = vec![
            0.9999,         // Stablecoin pair
            4605.23,        // ETH/USDC
            118000.0,       // BTC/USD
            0.00001,        // Small altcoin
            1.0,            // Unity
        ];
        
        for price in reasonable_prices {
            let fp = (price * 1e8) as u64;
            let trade = TradeMessage::new(
                1234567890000000000,
                fp,
                100000000,
                12345678,
                TradeSide::Unknown,
            );
            
            let decoded = trade.price_f64();
            let relative_error = ((decoded - price) / price).abs();
            assert!(relative_error < 0.0001, "Price error: {} vs {}", price, decoded);
        }
        
        // Test maximum safe price (before u64 overflow)
        let max_safe_price = 1.8e11; // ~180 billion
        let fp = (max_safe_price * 1e8) as u64;
        assert!(fp < u64::MAX);
    }
}

/// Token information message for broadcasting newly discovered tokens
/// Binary format: 128 bytes fixed size
#[derive(Debug, Clone, FromZeroes, FromBytes, AsBytes)]
#[repr(C)]
pub struct TokenInfoMessage {
    pub timestamp_ns: [u8; 8],      // When token was discovered
    pub token_address: [u8; 20],    // Token contract address
    pub decimals: u8,                // Token decimals (e.g., 18 for WETH, 6 for USDC)
    pub _pad1: [u8; 7],              // Padding to align
    pub symbol: [u8; 16],            // Token symbol (null-padded)
    pub name: [u8; 32],              // Token name (null-padded)
    pub chain_id: [u8; 4],           // Chain ID (137 for Polygon)
    pub _pad2: [u8; 40],             // Reserved for future use
}

impl TokenInfoMessage {
    pub fn new(
        token_address: &str,
        symbol: &str,
        name: &str,
        decimals: u8,
        chain_id: u32,
    ) -> Self {
        let mut msg = Self::new_zeroed();
        
        // Set timestamp
        let now_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        msg.timestamp_ns.copy_from_slice(&now_ns.to_le_bytes());
        
        // Set token address (convert from hex string)
        if token_address.starts_with("0x") && token_address.len() >= 42 {
            if let Ok(addr_bytes) = hex::decode(&token_address[2..42]) {
                msg.token_address.copy_from_slice(&addr_bytes);
            }
        }
        
        // Set decimals
        msg.decimals = decimals;
        
        // Set symbol (truncate if necessary)
        let symbol_bytes = symbol.as_bytes();
        let symbol_len = symbol_bytes.len().min(16);
        msg.symbol[..symbol_len].copy_from_slice(&symbol_bytes[..symbol_len]);
        
        // Set name (truncate if necessary)
        let name_bytes = name.as_bytes();
        let name_len = name_bytes.len().min(32);
        msg.name[..name_len].copy_from_slice(&name_bytes[..name_len]);
        
        // Set chain ID
        msg.chain_id.copy_from_slice(&chain_id.to_le_bytes());
        
        msg
    }
    
    pub fn get_token_address(&self) -> String {
        format!("0x{}", hex::encode(&self.token_address))
    }
    
    pub fn get_symbol(&self) -> String {
        String::from_utf8_lossy(&self.symbol)
            .trim_end_matches('\0')
            .to_string()
    }
    
    pub fn get_name(&self) -> String {
        String::from_utf8_lossy(&self.name)
            .trim_end_matches('\0')
            .to_string()
    }
    
    pub fn get_chain_id(&self) -> u32 {
        LittleEndian::read_u32(&self.chain_id)
    }
    
    pub fn get_timestamp_ns(&self) -> u64 {
        LittleEndian::read_u64(&self.timestamp_ns)
    }
}

// Re-export new protocol types (aliased to avoid naming conflicts)
pub use message_protocol::{
    InstrumentId, MessageHeader as NewMessageHeader, VenueId, AssetType, SourceType, MessageType as NewMessageType,
    ParseError, MESSAGE_MAGIC
};
pub use messages::{
    TradeMessage as NewTradeMessage, QuoteMessage as NewQuoteMessage, 
    InstrumentDiscoveredMessage, TradeSide as NewTradeSide, InstrumentDiscoveredHeader,
    SwapEventMessage as NewSwapEventMessage, PoolUpdateMessage as NewPoolUpdateMessage,
    ArbitrageOpportunityMessage as NewArbitrageOpportunityMessage,
    DeFiSignalMessage, DeFiPoolSignal
};
pub use schema_transform_cache::{
    SchemaTransformCache, InstrumentMetadata, TokenMetadata, PoolMetadata,
    CachedObject, ProcessedMessage, TradeData, QuoteData, CacheStats,
    SwapEventData, PoolUpdateData, ArbitrageData
};
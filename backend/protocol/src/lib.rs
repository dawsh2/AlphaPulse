use byteorder::{ByteOrder, LittleEndian};
use thiserror::Error;
use zerocopy::{AsBytes, FromBytes, FromZeroes};
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

// Export conversion and validation modules
pub mod conversion;
pub mod validation;

pub const MAGIC_BYTE: u8 = 0xFE;
pub const UNIX_SOCKET_PATH: &str = "/tmp/alphapulse/market_data.sock";
pub const METRICS_SOCKET_PATH: &str = "/tmp/alphapulse/metrics.sock";
pub const RELAY_BIND_PATH: &str = "/tmp/alphapulse/relay.sock";

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
        if data.len() < 68 { // Minimum size for fixed fields
            return Err(ProtocolError::BufferTooSmall { need: 68, got: data.len() });
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
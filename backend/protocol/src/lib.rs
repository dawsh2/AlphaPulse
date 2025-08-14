use byteorder::{ByteOrder, LittleEndian};
use thiserror::Error;
use zerocopy::{AsBytes, FromBytes, FromZeroes};

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
    #[error("Invalid symbol ID: {0}")]
    InvalidSymbolId(u32),
    #[error("Invalid exchange ID: {0}")]
    InvalidExchangeId(u16),
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
    pub timestamp_ns: [u8; 8],
    pub price: [u8; 8],
    pub volume: [u8; 8],
    pub symbol_id: [u8; 4],
    pub exchange_id: [u8; 2],
    pub side: u8,
    pub _padding: [u8; 9],
}

impl TradeMessage {
    pub const SIZE: usize = 40;

    pub fn new(
        timestamp_ns: u64,
        price: u64,
        volume: u64,
        symbol_id: u32,
        exchange_id: u16,
        side: TradeSide,
    ) -> Self {
        let mut msg = Self::new_zeroed();
        LittleEndian::write_u64(&mut msg.timestamp_ns, timestamp_ns);
        LittleEndian::write_u64(&mut msg.price, price);
        LittleEndian::write_u64(&mut msg.volume, volume);
        LittleEndian::write_u32(&mut msg.symbol_id, symbol_id);
        LittleEndian::write_u16(&mut msg.exchange_id, exchange_id);
        msg.side = side as u8;
        msg
    }

    pub fn timestamp_ns(&self) -> u64 {
        LittleEndian::read_u64(&self.timestamp_ns)
    }

    pub fn price(&self) -> u64 {
        LittleEndian::read_u64(&self.price)
    }

    pub fn volume(&self) -> u64 {
        LittleEndian::read_u64(&self.volume)
    }

    pub fn symbol_id(&self) -> u32 {
        LittleEndian::read_u32(&self.symbol_id)
    }

    pub fn exchange_id(&self) -> u16 {
        LittleEndian::read_u16(&self.exchange_id)
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

#[derive(Debug, Clone)]
pub struct OrderBookMessage {
    pub timestamp_ns: u64,
    pub symbol_id: u32,
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
        buffer.extend_from_slice(&self.symbol_id.to_le_bytes());
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
        if data.len() < 16 {
            return Err(ProtocolError::BufferTooSmall {
                need: 16,
                got: data.len(),
            });
        }

        let timestamp_ns = LittleEndian::read_u64(&data[0..8]);
        let symbol_id = LittleEndian::read_u32(&data[8..12]);
        let bid_count = LittleEndian::read_u16(&data[12..14]) as usize;
        let ask_count = LittleEndian::read_u16(&data[14..16]) as usize;

        let total_size = 16 + (bid_count + ask_count) * 16;
        if data.len() < total_size {
            return Err(ProtocolError::BufferTooSmall {
                need: total_size,
                got: data.len(),
            });
        }

        let mut offset = 16;
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
            symbol_id,
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

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolMapper {
    symbols: Vec<String>,
    symbol_to_id: std::collections::HashMap<String, u32>,
}

impl SymbolMapper {
    pub fn new() -> Self {
        let symbols = vec![
            "BTC/USD".to_string(),
            "ETH/USD".to_string(),
            "BTC/USDT".to_string(),
            "ETH/USDT".to_string(),
            "SOL/USD".to_string(),
            "XRP/USD".to_string(),
        ];
        
        let mut symbol_to_id = std::collections::HashMap::new();
        for (id, symbol) in symbols.iter().enumerate() {
            symbol_to_id.insert(symbol.clone(), id as u32);
        }
        
        Self {
            symbols,
            symbol_to_id,
        }
    }
    
    // Symbol mapping is embedded in protocol messages - no shared memory needed

    pub fn get_id(&self, symbol: &str) -> Option<u32> {
        self.symbol_to_id.get(symbol).copied()
    }

    pub fn get_symbol(&self, id: u32) -> Option<&str> {
        self.symbols.get(id as usize).map(|s| s.as_str())
    }

    pub fn add_symbol(&mut self, symbol: String) -> u32 {
        if let Some(id) = self.symbol_to_id.get(&symbol) {
            return *id;
        }
        
        let id = self.symbols.len() as u32;
        self.symbols.push(symbol.clone());
        self.symbol_to_id.insert(symbol, id);
        id
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u16)]
pub enum ExchangeId {
    Kraken = 1,
    Coinbase = 2,
    Binance = 3,
}

impl ExchangeId {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "kraken" => Some(ExchangeId::Kraken),
            "coinbase" => Some(ExchangeId::Coinbase),
            "binance" => Some(ExchangeId::Binance),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            ExchangeId::Kraken => "kraken",
            ExchangeId::Coinbase => "coinbase",
            ExchangeId::Binance => "binance",
        }
    }
}

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
    pub symbol_id: u32,
    pub exchange_id: u16,
    pub sequence: u64,
    pub updates: Vec<L2Update>,
}

impl L2DeltaMessage {
    pub fn encode(&self, buffer: &mut Vec<u8>) {
        buffer.extend_from_slice(&self.timestamp_ns.to_le_bytes());
        buffer.extend_from_slice(&self.symbol_id.to_le_bytes());
        buffer.extend_from_slice(&self.exchange_id.to_le_bytes());
        buffer.extend_from_slice(&self.sequence.to_le_bytes());
        buffer.extend_from_slice(&(self.updates.len() as u16).to_le_bytes());
        
        for update in &self.updates {
            buffer.extend_from_slice(update.as_bytes());
        }
    }
    
    pub fn decode(data: &[u8]) -> Result<Self, ProtocolError> {
        if data.len() < 24 {
            return Err(ProtocolError::BufferTooSmall {
                need: 24,
                got: data.len(),
            });
        }
        
        let timestamp_ns = LittleEndian::read_u64(&data[0..8]);
        let symbol_id = LittleEndian::read_u32(&data[8..12]);
        let exchange_id = LittleEndian::read_u16(&data[12..14]);
        let sequence = LittleEndian::read_u64(&data[14..22]);
        let update_count = LittleEndian::read_u16(&data[22..24]) as usize;
        
        let total_size = 24 + update_count * L2Update::SIZE;
        if data.len() < total_size {
            return Err(ProtocolError::BufferTooSmall {
                need: total_size,
                got: data.len(),
            });
        }
        
        let mut updates = Vec::with_capacity(update_count);
        let mut offset = 24;
        
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
            symbol_id,
            exchange_id,
            sequence,
            updates,
        })
    }
}

#[derive(Debug, Clone)]
pub struct L2SnapshotMessage {
    pub timestamp_ns: u64,
    pub symbol_id: u32,
    pub exchange_id: u16,
    pub sequence: u64,
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
}

impl L2SnapshotMessage {
    pub fn encode(&self, buffer: &mut Vec<u8>) {
        buffer.extend_from_slice(&self.timestamp_ns.to_le_bytes());
        buffer.extend_from_slice(&self.symbol_id.to_le_bytes());
        buffer.extend_from_slice(&self.exchange_id.to_le_bytes());
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
        if data.len() < 26 {
            return Err(ProtocolError::BufferTooSmall {
                need: 26,
                got: data.len(),
            });
        }
        
        let timestamp_ns = LittleEndian::read_u64(&data[0..8]);
        let symbol_id = LittleEndian::read_u32(&data[8..12]);
        let exchange_id = LittleEndian::read_u16(&data[12..14]);
        let sequence = LittleEndian::read_u64(&data[14..22]);
        let bid_count = LittleEndian::read_u16(&data[22..24]) as usize;
        let ask_count = LittleEndian::read_u16(&data[24..26]) as usize;
        
        let total_size = 26 + (bid_count + ask_count) * 16;
        if data.len() < total_size {
            return Err(ProtocolError::BufferTooSmall {
                need: total_size,
                got: data.len(),
            });
        }
        
        let mut offset = 26;
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
            symbol_id,
            exchange_id,
            sequence,
            bids,
            asks,
        })
    }
}

// Per-symbol sequence tracking
#[derive(Debug, Clone, Default)]
pub struct SymbolSequenceTracker {
    sequences: std::collections::HashMap<(u16, u32), u64>, // (exchange_id, symbol_id) -> sequence
}

impl SymbolSequenceTracker {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn check_sequence(&mut self, exchange_id: u16, symbol_id: u32, sequence: u64) -> SequenceCheck {
        let key = (exchange_id, symbol_id);
        let expected = self.sequences.get(&key).copied().unwrap_or(0) + 1;
        
        if sequence == expected || (expected == 1 && sequence > 0) {
            self.sequences.insert(key, sequence);
            SequenceCheck::Ok
        } else if sequence > expected {
            let gap = sequence - expected;
            self.sequences.insert(key, sequence);
            SequenceCheck::Gap(gap)
        } else {
            SequenceCheck::OutOfOrder
        }
    }
    
    pub fn reset(&mut self, exchange_id: u16, symbol_id: u32) {
        self.sequences.remove(&(exchange_id, symbol_id));
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SequenceCheck {
    Ok,
    Gap(u64),
    OutOfOrder,
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
        let trade = TradeMessage::new(
            1234567890000000000,
            6500000000000,
            100000000,
            1,
            ExchangeId::Kraken as u16,
            TradeSide::Buy,
        );
        
        assert_eq!(trade.timestamp_ns(), 1234567890000000000);
        assert_eq!(trade.price_f64(), 65000.0);
        assert_eq!(trade.volume_f64(), 1.0);
        assert_eq!(trade.symbol_id(), 1);
        assert_eq!(trade.exchange_id(), 1);
        assert_eq!(trade.side(), TradeSide::Buy);
    }
}
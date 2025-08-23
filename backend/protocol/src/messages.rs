use crate::message_protocol::{MessageHeader, InstrumentId, ParseError, MESSAGE_MAGIC, MessageType, SourceType};
use zerocopy::{AsBytes, FromBytes, FromZeroes};
use std::mem::size_of;

/// Trade message (64 bytes) - zero-copy binary format
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, AsBytes, FromBytes, FromZeroes)]
pub struct TradeMessage {
    pub header: MessageHeader,      // 32 bytes
    pub instrument_id: InstrumentId, // 12 bytes
    pub price: i64,                 // 8 bytes (fixed-point with 8 decimal places)
    pub volume: u64,                // 8 bytes (fixed-point with 8 decimal places)
    pub side: u8,                   // 1 byte (Buy=1, Sell=2)
    pub flags: u8,                  // 1 byte (execution flags)
    pub _padding: [u8; 2],          // 2 bytes alignment
}

impl TradeMessage {
    /// Create a new trade message
    pub fn new(
        instrument_id: InstrumentId,
        price: i64,
        volume: u64,
        side: TradeSide,
        sequence: u64,
        source: crate::message_protocol::SourceType,
    ) -> Self {
        let header = MessageHeader::new(
            crate::message_protocol::MessageType::Trade,
            1, // version
            source,
            size_of::<Self>() as u32 - size_of::<MessageHeader>() as u32, // payload size
            sequence,
        );
        
        Self {
            header,
            instrument_id,
            price,
            volume,
            side: side as u8,
            flags: 0,
            _padding: [0; 2],
        }
    }
    
    /// Parse from bytes with validation
    pub fn from_bytes(data: &[u8]) -> Result<&Self, ParseError> {
        if data.len() < size_of::<Self>() {
            return Err(ParseError::TooSmall {
                need: size_of::<Self>(),
                got: data.len(),
            });
        }
        
        let msg = zerocopy::Ref::<_, Self>::new(data)
            .ok_or(ParseError::InvalidLayout)?
            .into_ref();
            
        // Validate header
        let magic = msg.header.magic;
        if magic != MESSAGE_MAGIC {
            return Err(ParseError::InvalidMagic {
                expected: MESSAGE_MAGIC,
                actual: magic,
            });
        }
        
        Ok(msg)
    }
    
    /// Get price as decimal (divide by 10^8)
    pub fn price_decimal(&self) -> f64 {
        self.price as f64 / 100_000_000.0
    }
    
    /// Get volume as decimal (divide by 10^8)
    pub fn volume_decimal(&self) -> f64 {
        self.volume as f64 / 100_000_000.0
    }
    
    /// Get trade side
    pub fn trade_side(&self) -> Result<TradeSide, ParseError> {
        TradeSide::try_from(self.side).map_err(|_| ParseError::InvalidLayout)
    }
}

/// Quote message (80 bytes) - zero-copy binary format  
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, AsBytes, FromBytes, FromZeroes)]
pub struct QuoteMessage {
    pub header: MessageHeader,      // 32 bytes
    pub instrument_id: InstrumentId, // 12 bytes
    pub bid_price: i64,             // 8 bytes (fixed-point)
    pub ask_price: i64,             // 8 bytes (fixed-point)
    pub bid_size: u64,              // 8 bytes (fixed-point)
    pub ask_size: u64,              // 8 bytes (fixed-point)
    pub _padding: [u8; 4],          // 4 bytes alignment
}

impl QuoteMessage {
    /// Create a new quote message
    pub fn new(
        instrument_id: InstrumentId,
        bid_price: i64,
        ask_price: i64,
        bid_size: u64,
        ask_size: u64,
        sequence: u64,
        source: crate::message_protocol::SourceType,
    ) -> Self {
        let header = MessageHeader::new(
            crate::message_protocol::MessageType::Quote,
            1, // version
            source,
            size_of::<Self>() as u32 - size_of::<MessageHeader>() as u32,
            sequence,
        );
        
        Self {
            header,
            instrument_id,
            bid_price,
            ask_price,
            bid_size,
            ask_size,
            _padding: [0; 4],
        }
    }
    
    /// Parse from bytes with validation
    pub fn from_bytes(data: &[u8]) -> Result<&Self, ParseError> {
        if data.len() < size_of::<Self>() {
            return Err(ParseError::TooSmall {
                need: size_of::<Self>(),
                got: data.len(),
            });
        }
        
        let msg = zerocopy::Ref::<_, Self>::new(data)
            .ok_or(ParseError::InvalidLayout)?
            .into_ref();
            
        // Validate header
        let magic = msg.header.magic;
        if magic != MESSAGE_MAGIC {
            return Err(ParseError::InvalidMagic {
                expected: MESSAGE_MAGIC,
                actual: magic,
            });
        }
        
        Ok(msg)
    }
    
    /// Get bid price as decimal
    pub fn bid_price_decimal(&self) -> f64 {
        self.bid_price as f64 / 100_000_000.0
    }
    
    /// Get ask price as decimal
    pub fn ask_price_decimal(&self) -> f64 {
        self.ask_price as f64 / 100_000_000.0
    }
    
    /// Get bid size as decimal
    pub fn bid_size_decimal(&self) -> f64 {
        self.bid_size as f64 / 100_000_000.0
    }
    
    /// Get ask size as decimal
    pub fn ask_size_decimal(&self) -> f64 {
        self.ask_size as f64 / 100_000_000.0
    }
    
    /// Calculate spread in basis points
    pub fn spread_bps(&self) -> u32 {
        if self.bid_price <= 0 || self.ask_price <= 0 || self.ask_price <= self.bid_price {
            return 0;
        }
        
        let spread = self.ask_price - self.bid_price;
        let mid_price = (self.ask_price + self.bid_price) / 2;
        
        ((spread * 10000) / mid_price) as u32
    }
}

/// Variable-size instrument discovery message
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, AsBytes, FromBytes, FromZeroes)]
pub struct InstrumentDiscoveredHeader {
    pub header: MessageHeader,      // 32 bytes
    pub instrument_id: InstrumentId, // 12 bytes
    pub decimals: u8,               // 1 byte
    pub symbol_len: u8,             // 1 byte (symbol string length)
    pub metadata_len: u16,          // 2 bytes (metadata blob length)
    // Variable data follows: symbol + metadata
}

/// Complete instrument discovery message with variable data
pub struct InstrumentDiscoveredMessage {
    pub header: InstrumentDiscoveredHeader,
    pub symbol: String,
    pub metadata: Vec<u8>,
}

impl InstrumentDiscoveredMessage {
    /// Create a new instrument discovery message
    pub fn new(
        instrument_id: InstrumentId,
        symbol: String,
        decimals: u8,
        metadata: Vec<u8>,
        sequence: u64,
        source: crate::message_protocol::SourceType,
    ) -> Self {
        let symbol_len = symbol.len().min(255) as u8;
        let metadata_len = metadata.len().min(65535) as u16;
        let payload_size = size_of::<InstrumentDiscoveredHeader>() as u32 
            - size_of::<MessageHeader>() as u32 
            + symbol_len as u32 
            + metadata_len as u32;
        
        let header_msg = MessageHeader::new(
            crate::message_protocol::MessageType::InstrumentDiscovered,
            1, // version
            source,
            payload_size,
            sequence,
        );
        
        let header = InstrumentDiscoveredHeader {
            header: header_msg,
            instrument_id,
            decimals,
            symbol_len,
            metadata_len,
        };
        
        Self {
            header,
            symbol,
            metadata,
        }
    }
    
    /// Parse from bytes
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        if data.len() < size_of::<InstrumentDiscoveredHeader>() {
            return Err(ParseError::TooSmall {
                need: size_of::<InstrumentDiscoveredHeader>(),
                got: data.len(),
            });
        }
        
        let header = zerocopy::Ref::<_, InstrumentDiscoveredHeader>::new(
            &data[..size_of::<InstrumentDiscoveredHeader>()]
        ).ok_or(ParseError::InvalidLayout)?.into_ref();
        
        // Validate header magic
        let magic = header.header.magic;
        if magic != MESSAGE_MAGIC {
            return Err(ParseError::InvalidMagic {
                expected: MESSAGE_MAGIC,
                actual: magic,
            });
        }
        
        let offset = size_of::<InstrumentDiscoveredHeader>();
        let symbol_end = offset + header.symbol_len as usize;
        let metadata_end = symbol_end + header.metadata_len as usize;
        
        if data.len() < metadata_end {
            return Err(ParseError::TooSmall {
                need: metadata_end,
                got: data.len(),
            });
        }
        
        Ok(Self {
            header: *header,
            symbol: String::from_utf8_lossy(&data[offset..symbol_end]).to_string(),
            metadata: data[symbol_end..metadata_end].to_vec(),
        })
    }
    
    /// Serialize to bytes
    pub fn serialize(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        
        // Serialize header
        buffer.extend_from_slice(self.header.as_bytes());
        
        // Serialize symbol (truncated to symbol_len)
        let symbol_bytes = self.symbol.as_bytes();
        let actual_len = symbol_bytes.len().min(self.header.symbol_len as usize);
        buffer.extend_from_slice(&symbol_bytes[..actual_len]);
        
        // Serialize metadata (truncated to metadata_len) 
        let actual_meta_len = self.metadata.len().min(self.header.metadata_len as usize);
        buffer.extend_from_slice(&self.metadata[..actual_meta_len]);
        
        buffer
    }
}

/// DEX swap event message (96 bytes) - zero-copy binary format
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, AsBytes, FromBytes, FromZeroes)]
pub struct SwapEventMessage {
    pub header: MessageHeader,        // 32 bytes
    pub pool_id: InstrumentId,        // 12 bytes
    pub token0_id: InstrumentId,      // 12 bytes  
    pub token1_id: InstrumentId,      // 12 bytes
    pub amount0_in: u64,              // 8 bytes (fixed-point with 8 decimals)
    pub amount1_in: u64,              // 8 bytes (fixed-point with 8 decimals)
    pub amount0_out: u64,             // 8 bytes (fixed-point with 8 decimals)
    pub amount1_out: u64,             // 8 bytes (fixed-point with 8 decimals)
}

impl SwapEventMessage {
    /// Create a new swap event message
    pub fn new(
        pool_id: InstrumentId,
        token0_id: InstrumentId,
        token1_id: InstrumentId,
        amount0_in: u64,
        amount1_in: u64,
        amount0_out: u64,
        amount1_out: u64,
        sequence: u64,
        source: crate::message_protocol::SourceType,
    ) -> Self {
        let header = MessageHeader::new(
            crate::message_protocol::MessageType::SwapEvent,
            1, // version
            source,
            size_of::<Self>() as u32 - size_of::<MessageHeader>() as u32,
            sequence,
        );
        
        Self {
            header,
            pool_id,
            token0_id,
            token1_id,
            amount0_in,
            amount1_in,
            amount0_out,
            amount1_out,
        }
    }
    
    /// Parse from bytes with validation
    pub fn from_bytes(data: &[u8]) -> Result<&Self, ParseError> {
        if data.len() < size_of::<Self>() {
            return Err(ParseError::TooSmall {
                need: size_of::<Self>(),
                got: data.len(),
            });
        }
        
        let msg = zerocopy::Ref::<_, Self>::new(data)
            .ok_or(ParseError::InvalidLayout)?
            .into_ref();
            
        // Validate header
        let magic = msg.header.magic;
        if magic != MESSAGE_MAGIC {
            return Err(ParseError::InvalidMagic {
                expected: MESSAGE_MAGIC,
                actual: magic,
            });
        }
        
        Ok(msg)
    }
    
    /// Get amount0_in as decimal
    pub fn amount0_in_decimal(&self) -> f64 {
        self.amount0_in as f64 / 100_000_000.0
    }
    
    /// Get amount1_in as decimal
    pub fn amount1_in_decimal(&self) -> f64 {
        self.amount1_in as f64 / 100_000_000.0
    }
    
    /// Get amount0_out as decimal
    pub fn amount0_out_decimal(&self) -> f64 {
        self.amount0_out as f64 / 100_000_000.0
    }
    
    /// Get amount1_out as decimal
    pub fn amount1_out_decimal(&self) -> f64 {
        self.amount1_out as f64 / 100_000_000.0
    }
}

/// Pool update message (80 bytes) - zero-copy binary format
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, AsBytes, FromBytes, FromZeroes)]
pub struct PoolUpdateMessage {
    pub header: MessageHeader,        // 32 bytes
    pub pool_id: InstrumentId,        // 12 bytes
    pub reserve0: u64,                // 8 bytes (fixed-point with 8 decimals)
    pub reserve1: u64,                // 8 bytes (fixed-point with 8 decimals)  
    pub sqrt_price_x96: u128,         // 16 bytes (V3 price)
    pub tick: i32,                    // 4 bytes (V3 tick)
}

impl PoolUpdateMessage {
    /// Create a new pool update message
    pub fn new(
        pool_id: InstrumentId,
        reserve0: u64,
        reserve1: u64,
        sqrt_price_x96: u128,
        tick: i32,
        sequence: u64,
        source: crate::message_protocol::SourceType,
    ) -> Self {
        let header = MessageHeader::new(
            crate::message_protocol::MessageType::PoolUpdate,
            1, // version
            source,
            size_of::<Self>() as u32 - size_of::<MessageHeader>() as u32,
            sequence,
        );
        
        Self {
            header,
            pool_id,
            reserve0,
            reserve1,
            sqrt_price_x96,
            tick,
        }
    }
    
    /// Parse from bytes with validation
    pub fn from_bytes(data: &[u8]) -> Result<&Self, ParseError> {
        if data.len() < size_of::<Self>() {
            return Err(ParseError::TooSmall {
                need: size_of::<Self>(),
                got: data.len(),
            });
        }
        
        let msg = zerocopy::Ref::<_, Self>::new(data)
            .ok_or(ParseError::InvalidLayout)?
            .into_ref();
            
        // Validate header
        let magic = msg.header.magic;
        if magic != MESSAGE_MAGIC {
            return Err(ParseError::InvalidMagic {
                expected: MESSAGE_MAGIC,
                actual: magic,
            });
        }
        
        Ok(msg)
    }
    
    /// Get reserve0 as decimal
    pub fn reserve0_decimal(&self) -> f64 {
        self.reserve0 as f64 / 100_000_000.0
    }
    
    /// Get reserve1 as decimal
    pub fn reserve1_decimal(&self) -> f64 {
        self.reserve1 as f64 / 100_000_000.0
    }
}

/// Arbitrage opportunity message (208 bytes) - zero-copy binary format with comprehensive fee data and token symbols
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, AsBytes, FromBytes, FromZeroes)]
pub struct ArbitrageOpportunityMessage {
    pub header: MessageHeader,        // 32 bytes
    pub token0_id: InstrumentId,      // 12 bytes
    pub token1_id: InstrumentId,      // 12 bytes
    pub buy_pool_id: InstrumentId,    // 12 bytes
    pub sell_pool_id: InstrumentId,   // 12 bytes
    pub buy_price: u64,               // 8 bytes (fixed-point 8 decimals)
    pub sell_price: u64,              // 8 bytes (fixed-point 8 decimals)
    pub trade_size_usd: u64,          // 8 bytes (fixed-point 8 decimals)
    pub gross_profit_usd: u64,        // 8 bytes (fixed-point 8 decimals)
    pub gas_fee_usd: u64,             // 8 bytes (fixed-point 8 decimals)
    pub dex_fees_usd: u64,            // 8 bytes (fixed-point 8 decimals)
    pub slippage_cost_usd: u64,       // 8 bytes (fixed-point 8 decimals)
    pub net_profit_usd: u64,          // 8 bytes (fixed-point 8 decimals)
    pub profit_percent: u32,          // 4 bytes (fixed-point 4 decimals, e.g., 1.5% = 15000)
    pub confidence_score: u16,        // 2 bytes (fixed-point 3 decimals, 0-1000 = 0.0-1.0)
    pub executable: u8,               // 1 byte (0=false, 1=true)
    pub _padding: u8,                 // 1 byte alignment
    // Token symbols (64 bytes total)
    pub token0_symbol: [u8; 16],      // 16 bytes (null-terminated string)
    pub token1_symbol: [u8; 16],      // 16 bytes (null-terminated string)
    pub buy_exchange: [u8; 16],       // 16 bytes (null-terminated string, e.g., "uniswap_v3")
    pub sell_exchange: [u8; 16],      // 16 bytes (null-terminated string, e.g., "sushiswap")
}

impl ArbitrageOpportunityMessage {
    /// Create a new arbitrage opportunity message with comprehensive fee data and token symbols
    pub fn new(
        token0_id: InstrumentId,
        token1_id: InstrumentId,
        buy_pool_id: InstrumentId,
        sell_pool_id: InstrumentId,
        buy_price: u64,
        sell_price: u64,
        trade_size_usd: u64,
        gross_profit_usd: u64,
        gas_fee_usd: u64,
        dex_fees_usd: u64,
        slippage_cost_usd: u64,
        net_profit_usd: u64,
        profit_percent: u32,
        confidence_score: u16,
        executable: bool,
        token0_symbol: &str,
        token1_symbol: &str,
        buy_exchange: &str,
        sell_exchange: &str,
        sequence: u64,
        source: crate::message_protocol::SourceType,
    ) -> Self {
        let header = MessageHeader::new(
            crate::message_protocol::MessageType::ArbitrageOpportunity,
            1, // version
            source,
            size_of::<Self>() as u32 - size_of::<MessageHeader>() as u32,
            sequence,
        );
        
        // Helper function to convert string to fixed-size null-terminated array
        fn str_to_fixed_array(s: &str, size: usize) -> Vec<u8> {
            let mut arr = vec![0u8; size];
            let bytes = s.as_bytes();
            let len = bytes.len().min(size - 1); // Leave space for null terminator
            arr[..len].copy_from_slice(&bytes[..len]);
            arr
        }
        
        let token0_symbol_arr = str_to_fixed_array(token0_symbol, 16);
        let token1_symbol_arr = str_to_fixed_array(token1_symbol, 16);
        let buy_exchange_arr = str_to_fixed_array(buy_exchange, 16);
        let sell_exchange_arr = str_to_fixed_array(sell_exchange, 16);
        
        Self {
            header,
            token0_id,
            token1_id,
            buy_pool_id,
            sell_pool_id,
            buy_price,
            sell_price,
            trade_size_usd,
            gross_profit_usd,
            gas_fee_usd,
            dex_fees_usd,
            slippage_cost_usd,
            net_profit_usd,
            profit_percent,
            confidence_score,
            executable: if executable { 1 } else { 0 },
            _padding: 0,
            token0_symbol: token0_symbol_arr.try_into().unwrap_or([0; 16]),
            token1_symbol: token1_symbol_arr.try_into().unwrap_or([0; 16]),
            buy_exchange: buy_exchange_arr.try_into().unwrap_or([0; 16]),
            sell_exchange: sell_exchange_arr.try_into().unwrap_or([0; 16]),
        }
    }
    
    /// Parse from bytes with validation
    pub fn from_bytes(data: &[u8]) -> Result<&Self, ParseError> {
        if data.len() < size_of::<Self>() {
            return Err(ParseError::TooSmall {
                need: size_of::<Self>(),
                got: data.len(),
            });
        }
        
        let msg = zerocopy::Ref::<_, Self>::new(data)
            .ok_or(ParseError::InvalidLayout)?
            .into_ref();
            
        // Validate header
        let magic = msg.header.magic;
        if magic != MESSAGE_MAGIC {
            return Err(ParseError::InvalidMagic {
                expected: MESSAGE_MAGIC,
                actual: magic,
            });
        }
        
        Ok(msg)
    }
    
    /// Get buy price as decimal (8 decimal places)
    pub fn buy_price_decimal(&self) -> f64 {
        self.buy_price as f64 / 100_000_000.0
    }
    
    /// Get sell price as decimal (8 decimal places)
    pub fn sell_price_decimal(&self) -> f64 {
        self.sell_price as f64 / 100_000_000.0
    }
    
    /// Get trade size in USD as decimal (8 decimal places)
    pub fn trade_size_usd_decimal(&self) -> f64 {
        self.trade_size_usd as f64 / 100_000_000.0
    }
    
    /// Get gross profit in USD as decimal (8 decimal places)
    pub fn gross_profit_usd_decimal(&self) -> f64 {
        self.gross_profit_usd as f64 / 100_000_000.0
    }
    
    /// Get gas fee in USD as decimal (8 decimal places)
    pub fn gas_fee_usd_decimal(&self) -> f64 {
        self.gas_fee_usd as f64 / 100_000_000.0
    }
    
    /// Get DEX fees in USD as decimal (8 decimal places)
    pub fn dex_fees_usd_decimal(&self) -> f64 {
        self.dex_fees_usd as f64 / 100_000_000.0
    }
    
    /// Get slippage cost in USD as decimal (8 decimal places)
    pub fn slippage_cost_usd_decimal(&self) -> f64 {
        self.slippage_cost_usd as f64 / 100_000_000.0
    }
    
    /// Get net profit in USD as decimal (8 decimal places)
    pub fn net_profit_usd_decimal(&self) -> f64 {
        self.net_profit_usd as f64 / 100_000_000.0
    }
    
    /// Get profit percentage as decimal (4 decimal places)
    pub fn profit_percent_decimal(&self) -> f64 {
        self.profit_percent as f64 / 10_000.0
    }
    
    /// Get confidence score as decimal (3 decimal places, 0.0-1.0)
    pub fn confidence_score_decimal(&self) -> f64 {
        self.confidence_score as f64 / 1_000.0
    }
    
    /// Check if opportunity is executable
    pub fn is_executable(&self) -> bool {
        self.executable != 0
    }
    
    /// Calculate total fees in USD
    pub fn total_fees_usd_decimal(&self) -> f64 {
        self.gas_fee_usd_decimal() + self.dex_fees_usd_decimal() + self.slippage_cost_usd_decimal()
    }
    
    /// Get token0 symbol as string
    pub fn token0_symbol_str(&self) -> String {
        Self::null_terminated_str(&self.token0_symbol)
    }
    
    /// Get token1 symbol as string
    pub fn token1_symbol_str(&self) -> String {
        Self::null_terminated_str(&self.token1_symbol)
    }
    
    /// Get buy exchange name as string
    pub fn buy_exchange_str(&self) -> String {
        Self::null_terminated_str(&self.buy_exchange)
    }
    
    /// Get sell exchange name as string
    pub fn sell_exchange_str(&self) -> String {
        Self::null_terminated_str(&self.sell_exchange)
    }
    
    /// Get trading pair symbol
    pub fn pair_symbol(&self) -> String {
        format!("{}/{}", self.token0_symbol_str(), self.token1_symbol_str())
    }
    
    /// Helper to convert null-terminated byte array to string
    fn null_terminated_str(bytes: &[u8]) -> String {
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        String::from_utf8_lossy(&bytes[..end]).to_string()
    }
}

/// Trade side enumeration
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeSide {
    Buy = 1,
    Sell = 2,
}

impl TryFrom<u8> for TradeSide {
    type Error = ();
    
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(TradeSide::Buy),
            2 => Ok(TradeSide::Sell),
            _ => Err(()),
        }
    }
}

/// Generic DeFi Pool Signal Message (128 bytes)
/// Based on PoolArbitrageSignal from MESSAGE_PROTOCOL.md
/// Can be used for arbitrage, liquidations, rebalancing, etc.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, AsBytes, FromBytes, FromZeroes)]
pub struct DeFiPoolSignal {
    pub header: MessageHeader,        // 32 bytes
    
    // Pool identifiers
    pub pool_a_id: InstrumentId,      // 12 bytes (source pool/protocol)
    pub pool_b_id: InstrumentId,      // 12 bytes (target pool/protocol)
    
    // Token pair
    pub token0_id: InstrumentId,      // 12 bytes
    pub token1_id: InstrumentId,      // 12 bytes
    
    // Liquidity data
    pub pool_a_reserve0: u64,         // 8 bytes (in wei/smallest unit)
    pub pool_a_reserve1: u64,         // 8 bytes
    pub pool_b_reserve0: u64,         // 8 bytes
    pub pool_b_reserve1: u64,         // 8 bytes
    
    // Opportunity metrics
    pub optimal_amount_in: u64,       // 8 bytes (calculated optimal swap size)
    pub expected_profit_wei: u64,     // 8 bytes (profit after gas)
    pub gas_cost_gwei: u32,           // 4 bytes
    pub confidence: u16,              // 2 bytes (0-10000)
    pub block_number: u32,            // 4 bytes
    pub expires_at_block: u32,        // 4 bytes (MEV protection)
    pub _padding: [u8; 2],            // 2 bytes alignment
}

impl DeFiPoolSignal {
    /// Create from ArbitrageOpportunityMessage for migration
    pub fn from_arbitrage(arb: &ArbitrageOpportunityMessage) -> Self {
        // Extract source type from the header's u8 field
        let source = crate::message_protocol::SourceType::try_from(arb.header.source)
            .unwrap_or(crate::message_protocol::SourceType::Scanner);
            
        let header = MessageHeader::new(
            crate::message_protocol::MessageType::ArbitrageOpportunity,
            2, // version 2 for new format
            source,
            size_of::<Self>() as u32 - size_of::<MessageHeader>() as u32,
            arb.header.sequence,
        );
        
        Self {
            header,
            pool_a_id: arb.buy_pool_id,
            pool_b_id: arb.sell_pool_id,
            token0_id: arb.token0_id,
            token1_id: arb.token1_id,
            pool_a_reserve0: 0, // Would need to be filled from pool state
            pool_a_reserve1: 0,
            pool_b_reserve0: 0,
            pool_b_reserve1: 0,
            optimal_amount_in: arb.trade_size_usd,
            expected_profit_wei: arb.net_profit_usd,
            gas_cost_gwei: (arb.gas_fee_usd / 1_000_000) as u32, // Convert USD to gwei estimate
            confidence: arb.confidence_score,
            block_number: 0, // Would need current block
            expires_at_block: 0, // Would need to calculate
            _padding: [0; 2],
        }
    }
    
    /// Parse from bytes with validation
    pub fn from_bytes(data: &[u8]) -> Result<&Self, ParseError> {
        if data.len() < size_of::<Self>() {
            return Err(ParseError::TooSmall {
                need: size_of::<Self>(),
                got: data.len(),
            });
        }
        
        let msg = zerocopy::Ref::<_, Self>::new(&data[..size_of::<Self>()])
            .ok_or(ParseError::InvalidLayout)?
            .into_ref();
        
        // Validate header magic
        if msg.header.magic != MESSAGE_MAGIC {
            return Err(ParseError::InvalidMagic {
                expected: MESSAGE_MAGIC,
                actual: msg.header.magic,
            });
        }
        
        Ok(msg)
    }
}

/// Generic DeFi Signal for various strategy outputs (208 bytes)
/// Supports arbitrage, liquidation, yield farming opportunities, etc.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, AsBytes, FromBytes, FromZeroes)]
pub struct DeFiSignalMessage {
    pub header: MessageHeader,        // 32 bytes
    
    // Signal metadata (16 bytes)
    pub signal_type: u8,              // 1 byte (Arbitrage=1, Liquidation=2, Yield=3, etc.)
    pub chain_id: u8,                 // 1 byte (137=Polygon, 1=Ethereum, etc.)
    pub urgency: u8,                  // 1 byte (0-255, higher = more urgent)
    pub flags: u8,                    // 1 byte (bitflags for properties)
    pub confidence: u16,              // 2 bytes (0-10000 = 0-100%)
    pub expires_at_block: u32,        // 4 bytes (block number)
    pub nonce: u32,                   // 4 bytes (for deduplication)
    pub signal_version: u16,          // 2 bytes (schema version for this signal type)
    
    // Asset identifiers (48 bytes)
    pub asset0_id: InstrumentId,      // 12 bytes (primary asset)
    pub asset1_id: InstrumentId,      // 12 bytes (secondary asset if pair)
    pub venue0_id: InstrumentId,      // 12 bytes (source venue/pool)
    pub venue1_id: InstrumentId,      // 12 bytes (target venue/pool if arb)
    
    // Financial metrics (64 bytes) - interpretation depends on signal_type
    pub metric0: u64,                 // 8 bytes (e.g., buy_price, collateral_value)
    pub metric1: u64,                 // 8 bytes (e.g., sell_price, debt_value)
    pub metric2: u64,                 // 8 bytes (e.g., input_amount, liquidation_bonus)
    pub metric3: u64,                 // 8 bytes (e.g., output_amount, health_factor)
    pub metric4: u64,                 // 8 bytes (e.g., gas_cost_wei, apy)
    pub metric5: u64,                 // 8 bytes (e.g., profit_wei, tvl)
    pub metric6: u64,                 // 8 bytes (e.g., slippage_bps, utilization)
    pub metric7: u64,                 // 8 bytes (e.g., reserves, timestamp)
    
    // Optional context (48 bytes) - human-readable hints
    pub context: [u8; 48],            // 48 bytes (signal-specific data or description)
}

impl DeFiSignalMessage {
    pub const SIZE: usize = 208;
    
    /// Signal types
    pub const SIGNAL_ARBITRAGE: u8 = 1;
    pub const SIGNAL_LIQUIDATION: u8 = 2;
    pub const SIGNAL_YIELD_FARM: u8 = 3;
    pub const SIGNAL_IMPERMANENT_LOSS: u8 = 4;
    pub const SIGNAL_FLASH_LOAN: u8 = 5;
    
    /// Create arbitrage signal (backward compatible with ArbitrageOpportunityMessage)
    pub fn arbitrage(
        token0: InstrumentId,
        token1: InstrumentId,
        buy_pool: InstrumentId,
        sell_pool: InstrumentId,
        buy_price: u64,
        sell_price: u64,
        optimal_amount: u64,
        expected_profit: u64,
        gas_cost: u64,
        confidence: u16,
        sequence: u64,
        source: crate::message_protocol::SourceType,
    ) -> Self {
        let mut msg = Self {
            header: MessageHeader::new(
                crate::message_protocol::MessageType::ArbitrageOpportunity, // Reuse type 20 for now
                1,
                source,
                Self::SIZE as u32 - size_of::<MessageHeader>() as u32,
                sequence,
            ),
            signal_type: Self::SIGNAL_ARBITRAGE,
            chain_id: 137, // Polygon
            urgency: ((confidence as u32 * 255) / 10000) as u8,
            flags: 0,
            confidence,
            expires_at_block: 0, // TODO: Set from block number
            nonce: sequence as u32,
            signal_version: 1,
            asset0_id: token0,
            asset1_id: token1,
            venue0_id: buy_pool,
            venue1_id: sell_pool,
            metric0: buy_price,
            metric1: sell_price,
            metric2: optimal_amount,
            metric3: expected_profit,
            metric4: gas_cost,
            metric5: 0, // Available for DEX fees
            metric6: 0, // Available for slippage
            metric7: 0, // Available for timestamp or reserves
            context: [0; 48],
        };
        
        // Calculate and set checksum
        let bytes = msg.as_bytes();
        let checksum = crc32fast::hash(&bytes[..bytes.len() - 4]);
        msg.header.checksum = checksum;
        
        msg
    }
    
    /// Parse from bytes
    pub fn from_bytes(data: &[u8]) -> Result<&Self, ParseError> {
        if data.len() != Self::SIZE {
            return Err(ParseError::TooSmall {
                need: Self::SIZE,
                got: data.len(),
            });
        }
        
        let msg = zerocopy::Ref::<_, Self>::new(data)
            .ok_or(ParseError::InvalidLayout)?
            .into_ref();
        
        // Validate header
        if msg.header.magic != MESSAGE_MAGIC {
            return Err(ParseError::InvalidMagic {
                expected: MESSAGE_MAGIC,
                actual: msg.header.magic,
            });
        }
        
        Ok(msg)
    }
    
    /// Convert to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
    
    /// Helper to get readable signal type
    pub fn signal_type_str(&self) -> &'static str {
        match self.signal_type {
            Self::SIGNAL_ARBITRAGE => "arbitrage",
            Self::SIGNAL_LIQUIDATION => "liquidation",
            Self::SIGNAL_YIELD_FARM => "yield_farm",
            Self::SIGNAL_IMPERMANENT_LOSS => "impermanent_loss",
            Self::SIGNAL_FLASH_LOAN => "flash_loan",
            _ => "unknown",
        }
    }
}

/// Production-ready DeFi Signal (256 bytes) - Optimized for execution
/// Self-contained with full execution context, no registry dependencies
/// Supports TLV extensions for optional pool addresses and tertiary venues
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, AsBytes, FromBytes, FromZeroes)]
pub struct DeFiSignal {
    // Standard header (32 bytes)
    pub header: MessageHeader,

    // Signal identity (16 bytes)
    pub strategy_id: u16,               // Strategy type (1=TriangularArb, 2=CrossDex, etc.)
    pub signal_id: u64,                 // Unique signal ID
    pub signal_nonce: u32,              // Monotonic per (source, strategy_id)
    pub chain_id: u32,                  // EVM chain ID (1, 137, 42161)
    pub version: u8,                    // Schema version
    pub adapter_id: u8,                 // Execution adapter selector

    // Asset correlation (24 bytes) - Links to market data
    pub base_instrument: InstrumentId,  // 12 bytes - market data correlation
    pub quote_instrument: InstrumentId, // 12 bytes - market data correlation

    // Execution addresses (80 bytes) - Self-contained execution
    pub base_token_addr: [u8; 20],      // Base token contract
    pub quote_token_addr: [u8; 20],     // Quote token contract
    pub venue_a_router: [u8; 20],       // Primary router (required)
    pub venue_b_router: [u8; 20],       // Secondary router (0s if unused)

    // Venue metadata (12 bytes)
    pub venue_a_type: u8,               // 1=UniV2, 2=UniV3, 3=Curve
    pub venue_b_type: u8,               // 0 if unused
    pub fee_a_ppm: u32,                 // Fee in parts per million
    pub fee_b_ppm: u32,                 // Fee in parts per million
    pub direction_flag: u8,             // 0=sell base, 1=buy base
    pub confidence: u8,                 // 0-100 confidence score

    // Economics (64 bytes) - Q64.64 fixed point
    pub expected_profit_q: i128,        // Expected profit in quote token
    pub required_capital_q: u128,       // Required capital in base token
    pub gas_estimate_q: u128,           // Gas cost estimate in quote token
    pub amount_in_q: u128,              // Suggested input amount

    // Execution parameters (32 bytes)
    pub min_out_q: u128,                // Minimum output (slippage protection)
    pub optimal_size_q: u128,           // Optimal size (full precision)

    // State reference (24 bytes) - Block-based validity
    pub observed_block: u64,            // Block used for simulation
    pub valid_through_block: u64,       // Last valid execution block
    pub state_hash: u64,                // Pool state hash (64-bit for size discipline)

    // Execution control (16 bytes) - Consolidated bitfields
    pub execution_flags: u32,           // Consolidated: tx_policy + priority + approvals
    pub slippage_bps: u16,              // Maximum slippage tolerance
    pub price_impact_bps: u16,          // Expected price impact
    pub replace_signal_id: u64,         // If >0, supersedes this signal

    // TLV extension (8 bytes)
    pub tlv_offset: u16,                // Bytes from struct start to TLV data
    pub tlv_length: u16,                // Bytes of TLV data
    pub created_at_sec: u32,            // Wall-clock for analytics only
}

impl DeFiSignal {
    pub const SIZE: usize = 256;

    /// Strategy ID constants
    pub const STRATEGY_TRIANGULAR_ARB: u16 = 1;
    pub const STRATEGY_CROSS_DEX_ARB: u16 = 2;
    pub const STRATEGY_FLASH_LOAN_ARB: u16 = 3;
    pub const STRATEGY_LIQUIDATION: u16 = 4;
    pub const STRATEGY_MEV_SANDWICH: u16 = 5;

    /// Venue type constants
    pub const VENUE_UNISWAP_V2: u8 = 1;
    pub const VENUE_UNISWAP_V3: u8 = 2;
    pub const VENUE_CURVE: u8 = 3;
    pub const VENUE_SUSHISWAP: u8 = 4;
    pub const VENUE_BALANCER: u8 = 5;

    /// Execution flags bitfield masks
    pub const TX_POLICY_MASK: u32 = 0x3;        // Bits 0-1: tx policy
    pub const PRIORITY_MASK: u32 = 0xC;         // Bits 2-3: priority
    pub const APPROVAL_MASK: u32 = 0xF0;        // Bits 4-7: approval needs
    pub const VENUE_COUNT_MASK: u32 = 0xF00;    // Bits 8-11: venue count

    /// Transaction policy flags
    pub const TX_POLICY_PUBLIC: u32 = 0;
    pub const TX_POLICY_PRIVATE: u32 = 1;
    pub const TX_POLICY_BUNDLE: u32 = 2;
    pub const TX_POLICY_SIMULATE: u32 = 3;

    /// Priority class flags
    pub const PRIORITY_NORMAL: u32 = 0;
    pub const PRIORITY_URGENT: u32 = 1;
    pub const PRIORITY_CRITICAL: u32 = 2;

    /// Create a new DeFi signal
    pub fn new(
        strategy_id: u16,
        signal_id: u64,
        signal_nonce: u32,
        chain_id: u32,
        base_instrument: InstrumentId,
        quote_instrument: InstrumentId,
        sequence: u64,
        source: crate::message_protocol::SourceType,
    ) -> Self {
        Self {
            header: MessageHeader::new(
                crate::message_protocol::MessageType::DeFiSignal,
                1,
                source,
                (Self::SIZE - size_of::<MessageHeader>()) as u32,
                sequence,
            ),
            strategy_id,
            signal_id,
            signal_nonce,
            chain_id,
            version: 1,
            adapter_id: 0,
            base_instrument,
            quote_instrument,
            base_token_addr: [0; 20],
            quote_token_addr: [0; 20],
            venue_a_router: [0; 20],
            venue_b_router: [0; 20],
            venue_a_type: 0,
            venue_b_type: 0,
            fee_a_ppm: 0,
            fee_b_ppm: 0,
            direction_flag: 0,
            confidence: 0,
            expected_profit_q: 0,
            required_capital_q: 0,
            gas_estimate_q: 0,
            amount_in_q: 0,
            min_out_q: 0,
            optimal_size_q: 0,
            observed_block: 0,
            valid_through_block: 0,
            state_hash: 0,
            execution_flags: 0,
            slippage_bps: 0,
            price_impact_bps: 0,
            replace_signal_id: 0,
            tlv_offset: Self::SIZE as u16,
            tlv_length: 0,
            created_at_sec: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as u32,
        }
    }

    /// Get transaction policy from execution flags
    pub fn tx_policy(&self) -> u32 {
        self.execution_flags & Self::TX_POLICY_MASK
    }

    /// Set transaction policy in execution flags
    pub fn set_tx_policy(&mut self, policy: u32) {
        self.execution_flags = (self.execution_flags & !Self::TX_POLICY_MASK) | (policy & Self::TX_POLICY_MASK);
    }

    /// Get priority class from execution flags
    pub fn priority(&self) -> u32 {
        (self.execution_flags & Self::PRIORITY_MASK) >> 2
    }

    /// Set priority class in execution flags
    pub fn set_priority(&mut self, priority: u32) {
        self.execution_flags = (self.execution_flags & !Self::PRIORITY_MASK) | ((priority << 2) & Self::PRIORITY_MASK);
    }

    /// Get approval requirements from execution flags
    pub fn approval_needs(&self) -> u8 {
        ((self.execution_flags & Self::APPROVAL_MASK) >> 4) as u8
    }

    /// Set approval requirements in execution flags
    pub fn set_approval_needs(&mut self, needs: u8) {
        let needs_bits = ((needs as u32) << 4) & Self::APPROVAL_MASK;
        self.execution_flags = (self.execution_flags & !Self::APPROVAL_MASK) | needs_bits;
    }

    /// Get venue count from execution flags
    pub fn venue_count(&self) -> u8 {
        ((self.execution_flags & Self::VENUE_COUNT_MASK) >> 8) as u8
    }

    /// Set venue count in execution flags
    pub fn set_venue_count(&mut self, count: u8) {
        let count_bits = ((count as u32) << 8) & Self::VENUE_COUNT_MASK;
        self.execution_flags = (self.execution_flags & !Self::VENUE_COUNT_MASK) | count_bits;
    }

    /// Check if signal is expired based on current block
    pub fn is_expired(&self, current_block: u64) -> bool {
        self.valid_through_block > 0 && current_block > self.valid_through_block
    }

    /// Get strategy name as string
    pub fn strategy_name(&self) -> &'static str {
        match self.strategy_id {
            Self::STRATEGY_TRIANGULAR_ARB => "triangular_arbitrage",
            Self::STRATEGY_CROSS_DEX_ARB => "cross_dex_arbitrage",
            Self::STRATEGY_FLASH_LOAN_ARB => "flash_loan_arbitrage",
            Self::STRATEGY_LIQUIDATION => "liquidation",
            Self::STRATEGY_MEV_SANDWICH => "mev_sandwich",
            _ => "unknown_strategy",
        }
    }

    /// Get venue type name as string
    pub fn venue_a_name(&self) -> &'static str {
        match self.venue_a_type {
            Self::VENUE_UNISWAP_V2 => "uniswap_v2",
            Self::VENUE_UNISWAP_V3 => "uniswap_v3",
            Self::VENUE_CURVE => "curve",
            Self::VENUE_SUSHISWAP => "sushiswap",
            Self::VENUE_BALANCER => "balancer",
            _ => "unknown_venue",
        }
    }

    /// Parse from bytes (zero-copy)
    pub fn from_bytes(data: &[u8]) -> Result<&Self, ParseError> {
        if data.len() < Self::SIZE {
            return Err(ParseError::TooSmall {
                need: Self::SIZE,
                got: data.len(),
            });
        }

        let signal = zerocopy::Ref::<_, Self>::new(&data[..Self::SIZE])
            .ok_or(ParseError::InvalidLayout)?
            .into_ref();

        signal.header.validate()?;
        Ok(signal)
    }

    /// Convert to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.as_bytes().to_vec();
        
        // Ensure exactly 256 bytes
        bytes.resize(Self::SIZE, 0);
        
        bytes
    }
}

/// TLV extension types for DeFi signals
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TLVType {
    PoolAddresses = 1,
    TertiaryVenue = 2,
    MEVBundle = 3,
    CustomParams = 4,
}

/// Pool addresses TLV for UniV3 quoter (44 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, AsBytes, FromBytes, FromZeroes)]
pub struct PoolAddressTLV {
    pub tlv_type: u8,           // 1
    pub tlv_length: u8,         // 42
    pub venue_a_pool: [u8; 20], // Pool for quoter calls
    pub venue_b_pool: [u8; 20], // Pool for quoter calls
    pub reserved: [u8; 2],      // Alignment
}

impl PoolAddressTLV {
    pub const SIZE: usize = 44;

    pub fn new(venue_a_pool: [u8; 20], venue_b_pool: [u8; 20]) -> Self {
        Self {
            tlv_type: TLVType::PoolAddresses as u8,
            tlv_length: 42, // Size excluding type and length fields
            venue_a_pool,
            venue_b_pool,
            reserved: [0; 2],
        }
    }
}

/// Tertiary venue TLV for triangular arbitrage (24 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, AsBytes, FromBytes, FromZeroes)]
pub struct TertiaryVenueTLV {
    pub tlv_type: u8,           // 2
    pub tlv_length: u8,         // 22
    pub venue_c_router: [u8; 20], // Third router
    pub venue_c_type: u8,       // Venue type
    pub reserved: u8,           // Alignment
}

impl TertiaryVenueTLV {
    pub const SIZE: usize = 24;

    pub fn new(venue_c_router: [u8; 20], venue_c_type: u8) -> Self {
        Self {
            tlv_type: TLVType::TertiaryVenue as u8,
            tlv_length: 22, // Size excluding type and length fields
            venue_c_router,
            venue_c_type,
            reserved: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message_protocol::{VenueId, SourceType};
    use zerocopy::AsBytes;

    #[test]
    fn test_trade_message_roundtrip() {
        let instrument_id = InstrumentId::stock(VenueId::NASDAQ, "AAPL");
        let trade = TradeMessage::new(
            instrument_id,
            15000000000, // $150.00 with 8 decimal places
            10000000,    // 0.1 shares
            TradeSide::Buy,
            1234,
            SourceType::EthereumCollector,
        );
        
        // Serialize and deserialize
        let bytes = trade.as_bytes();
        let restored = TradeMessage::from_bytes(bytes).unwrap();
        
        // Compare key fields (copy to avoid packed field issues)
        let original_price = trade.price;
        let restored_price = restored.price;
        let original_volume = trade.volume;
        let restored_volume = restored.volume;
        
        assert_eq!(original_price, restored_price);
        assert_eq!(original_volume, restored_volume);
        assert_eq!(trade.price_decimal(), 150.0);
        assert_eq!(trade.volume_decimal(), 0.1);
        assert_eq!(trade.trade_side().unwrap(), TradeSide::Buy);
    }

    #[test]
    fn test_quote_message_roundtrip() {
        let instrument_id = InstrumentId::ethereum_token("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48").unwrap();
        let quote = QuoteMessage::new(
            instrument_id,
            99950000,    // $0.9995 bid
            100050000,   // $1.0005 ask  
            100000000000, // 1000 bid size
            50000000000,  // 500 ask size
            5678,
            SourceType::BinanceCollector,
        );
        
        // Serialize and deserialize
        let bytes = quote.as_bytes();
        let _restored = QuoteMessage::from_bytes(bytes).unwrap();
        
        // Test decimal conversions
        assert!((quote.bid_price_decimal() - 0.9995).abs() < 0.0001);
        assert!((quote.ask_price_decimal() - 1.0005).abs() < 0.0001);
        assert_eq!(quote.bid_size_decimal(), 1000.0);
        assert_eq!(quote.ask_size_decimal(), 500.0);
        
        // Test spread calculation (should be 10 bps)
        assert_eq!(quote.spread_bps(), 10);
    }

    #[test]
    fn test_instrument_discovered_message() {
        let instrument_id = InstrumentId::pool(
            VenueId::UniswapV3,
            InstrumentId::ethereum_token("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48").unwrap(),
            InstrumentId::ethereum_token("0xc02aaa39b223fe8d0a0e5c4f27b010c5d91b298a").unwrap(),
        );
        
        let metadata = vec![0x01, 0x02, 0x03]; // Some binary metadata
        let message = InstrumentDiscoveredMessage::new(
            instrument_id,
            "USDC/WETH".to_string(),
            18,
            metadata.clone(),
            9999,
            SourceType::PolygonCollector,
        );
        
        // Serialize and parse
        let bytes = message.serialize();
        let parsed = InstrumentDiscoveredMessage::parse(&bytes).unwrap();
        
        assert_eq!(parsed.symbol, "USDC/WETH");
        assert_eq!(parsed.header.decimals, 18);
        assert_eq!(parsed.metadata, metadata);
    }

    #[test]
    fn test_swap_event_message() {
        let usdc = InstrumentId::ethereum_token("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48").unwrap();
        let weth = InstrumentId::ethereum_token("0xc02aaa39b223fe8d0a0e5c4f27b010c5d91b298a").unwrap();
        let pool_id = InstrumentId::pool(VenueId::UniswapV3, usdc, weth);
        
        let swap = SwapEventMessage::new(
            pool_id,
            usdc,
            weth,
            100000000,  // 1.0 USDC in (8 decimal places)
            0,          // 0 WETH in
            0,          // 0 USDC out
            50000000,   // 0.5 WETH out (8 decimal places)
            1234,
            SourceType::PolygonCollector,
        );
        
        // Test serialization roundtrip
        let bytes = swap.as_bytes();
        let restored = SwapEventMessage::from_bytes(bytes).unwrap();
        
        // Test decimal conversions
        assert_eq!(restored.amount0_in_decimal(), 1.0);
        assert_eq!(restored.amount1_in_decimal(), 0.0);
        assert_eq!(restored.amount0_out_decimal(), 0.0);
        assert_eq!(restored.amount1_out_decimal(), 0.5);
        
        // Test IDs are preserved
        let restored_pool_id = restored.pool_id;
        let restored_token0_id = restored.token0_id;
        let restored_token1_id = restored.token1_id;
        
        assert_eq!(pool_id.cache_key(), restored_pool_id.cache_key());
        assert_eq!(usdc.cache_key(), restored_token0_id.cache_key());
        assert_eq!(weth.cache_key(), restored_token1_id.cache_key());
    }

    #[test]
    fn test_pool_update_message() {
        let usdc = InstrumentId::ethereum_token("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48").unwrap();
        let weth = InstrumentId::ethereum_token("0xc02aaa39b223fe8d0a0e5c4f27b010c5d91b298a").unwrap();
        let pool_id = InstrumentId::pool(VenueId::UniswapV3, usdc, weth);
        
        let pool_update = PoolUpdateMessage::new(
            pool_id,
            100000000000,  // 1000.0 USDC reserve
            50000000000,   // 500.0 WETH reserve
            1234567890123456u128, // V3 sqrt price
            -887220,       // V3 tick
            5678,
            SourceType::PolygonCollector,
        );
        
        // Test serialization roundtrip
        let bytes = pool_update.as_bytes();
        let restored = PoolUpdateMessage::from_bytes(bytes).unwrap();
        
        // Test decimal conversions
        assert_eq!(restored.reserve0_decimal(), 1000.0);
        assert_eq!(restored.reserve1_decimal(), 500.0);
        
        // Test V3 data preserved
        let restored_sqrt_price = restored.sqrt_price_x96;
        let restored_tick = restored.tick;
        
        assert_eq!(restored_sqrt_price, 1234567890123456u128);
        assert_eq!(restored_tick, -887220);
        
        // Test pool ID preserved
        assert_eq!(pool_id.cache_key(), restored.pool_id.cache_key());
    }

    #[test]
    fn test_defi_signal_message() {
        let usdc = InstrumentId::ethereum_token("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48").unwrap();
        let weth = InstrumentId::ethereum_token("0xc02aaa39b223fe8d0a0e5c4f27b010c5d91b298a").unwrap();
        let pool1 = InstrumentId::pool(VenueId::UniswapV3, usdc, weth);
        let pool2 = InstrumentId::pool(VenueId::SushiSwap, usdc, weth);
        
        // Create arbitrage signal
        let signal = DeFiSignalMessage::arbitrage(
            usdc,
            weth,
            pool1,  // Buy from UniswapV3
            pool2,  // Sell on SushiSwap
            200000000000, // Buy at $2000.00
            202000000000, // Sell at $2020.00
            100000000000, // Optimal amount: $1000.00
            2000000000,   // Expected profit: $20.00
            50000000,     // Gas cost: 50 gwei
            9000,         // 90% confidence
            12345,        // Sequence
            SourceType::ArbitrageStrategy,
        );
        
        // Test serialization roundtrip
        let bytes = signal.to_bytes();
        assert_eq!(bytes.len(), DeFiSignalMessage::SIZE);
        
        let restored = DeFiSignalMessage::from_bytes(&bytes).unwrap();
        
        // Test signal type (copy values due to packed struct)
        let signal_type = restored.signal_type;
        assert_eq!(signal_type, DeFiSignalMessage::SIGNAL_ARBITRAGE);
        assert_eq!(restored.signal_type_str(), "arbitrage");
        
        // Test metrics preserved (copy values due to packed struct)
        let metric0 = restored.metric0;
        let metric1 = restored.metric1;
        let metric2 = restored.metric2;
        let metric3 = restored.metric3;
        let metric4 = restored.metric4;
        assert_eq!(metric0, 200000000000); // Buy price
        assert_eq!(metric1, 202000000000); // Sell price
        assert_eq!(metric2, 100000000000); // Optimal amount
        assert_eq!(metric3, 2000000000);   // Expected profit
        assert_eq!(metric4, 50000000);     // Gas cost
        
        // Test confidence and urgency (copy values due to packed struct)
        let confidence = restored.confidence;
        let urgency = restored.urgency;
        assert_eq!(confidence, 9000);
        assert_eq!(urgency, 229); // (9000 * 255) / 10000
        
        // Test IDs preserved (copy values due to packed struct)
        let asset0_id = restored.asset0_id;
        let asset1_id = restored.asset1_id;
        let venue0_id = restored.venue0_id;
        let venue1_id = restored.venue1_id;
        assert_eq!(asset0_id.cache_key(), usdc.cache_key());
        assert_eq!(asset1_id.cache_key(), weth.cache_key());
        assert_eq!(venue0_id.cache_key(), pool1.cache_key());
        assert_eq!(venue1_id.cache_key(), pool2.cache_key());
    }

    #[test]
    fn test_arbitrage_opportunity_message() {
        let usdc = InstrumentId::ethereum_token("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48").unwrap();
        let weth = InstrumentId::ethereum_token("0xc02aaa39b223fe8d0a0e5c4f27b010c5d91b298a").unwrap();
        let pool1 = InstrumentId::pool(VenueId::UniswapV3, usdc, weth);
        let pool2 = InstrumentId::pool(VenueId::SushiSwap, usdc, weth);
        
        let arb = ArbitrageOpportunityMessage::new(
            usdc,
            weth,
            pool1,     // Buy from UniswapV3
            pool2,     // Sell on SushiSwap
            200000000000, // Buy at $2000.00
            202000000000, // Sell at $2020.00 (1% profit)
            100000000000, // Trade size: $1000.00
            2000000000,   // Gross profit: $20.00 (1% of $2000)
            500000000,    // Gas fee: $5.00
            600000000,    // DEX fees: $6.00 (0.3% per swap)
            100000000,    // Slippage: $1.00
            1300000000,   // Net profit: $13.00 (20 - 5 - 6 - 1)
            10000,        // Profit percent: 1.00% (10000 = 1.0% in 4 decimal precision)
            900,          // Confidence score: 90% (900/1000)
            true,         // Executable
            "USDC",       // Token0 symbol
            "WETH",       // Token1 symbol
            "uniswap_v3", // Buy exchange
            "sushiswap",  // Sell exchange
            9999,         // Sequence number
            SourceType::Scanner,
        );
        
        // Test serialization roundtrip
        let bytes = arb.as_bytes();
        let restored = ArbitrageOpportunityMessage::from_bytes(bytes).unwrap();
        
        // Test enhanced profit calculations
        assert_eq!(restored.profit_percent_decimal(), 1.0); // 1.00% profit
        assert_eq!(restored.gross_profit_usd_decimal(), 20.0); // $20.00 gross profit
        assert_eq!(restored.gas_fee_usd_decimal(), 5.0); // $5.00 gas fee
        assert_eq!(restored.dex_fees_usd_decimal(), 6.0); // $6.00 DEX fees
        assert_eq!(restored.slippage_cost_usd_decimal(), 1.0); // $1.00 slippage
        assert_eq!(restored.net_profit_usd_decimal(), 13.0); // $13.00 net profit
        assert_eq!(restored.confidence_score_decimal(), 0.9); // 90% confidence
        assert_eq!(restored.is_executable(), true); // Executable
        assert_eq!(restored.total_fees_usd_decimal(), 12.0); // $12.00 total fees (5+6+1)
        
        // Test string fields
        assert_eq!(restored.token0_symbol_str(), "USDC");
        assert_eq!(restored.token1_symbol_str(), "WETH");
        assert_eq!(restored.buy_exchange_str(), "uniswap_v3");
        assert_eq!(restored.sell_exchange_str(), "sushiswap");
        assert_eq!(restored.pair_symbol(), "USDC/WETH");
        
        // Test IDs preserved
        assert_eq!(usdc.cache_key(), restored.token0_id.cache_key());
        assert_eq!(weth.cache_key(), restored.token1_id.cache_key());
        assert_eq!(pool1.cache_key(), restored.buy_pool_id.cache_key());
        assert_eq!(pool2.cache_key(), restored.sell_pool_id.cache_key());
    }
    
    #[test]
    fn test_defi_pool_signal() {
        // Verify size is exactly 128 bytes as per MESSAGE_PROTOCOL.md
        assert_eq!(size_of::<DeFiPoolSignal>(), 128);
        
        // Create test data
        let usdc = InstrumentId::polygon_token("0x2791bca1f2de4661ed88a30c99a7a9449aa84174").unwrap();
        let weth = InstrumentId::polygon_token("0x7ceb23fd6bc0add59e62ac25578270cff1b9f619").unwrap();
        let pool1 = InstrumentId::pool(VenueId::UniswapV3, usdc, weth);
        let pool2 = InstrumentId::pool(VenueId::SushiSwap, usdc, weth);
        
        // Create an arbitrage message
        let arb = ArbitrageOpportunityMessage::new(
            usdc,
            weth,
            pool1,
            pool2,
            200000000000, // Buy at $2000.00
            202000000000, // Sell at $2020.00
            100000000000, // Trade size: $1000.00
            2000000000,   // Gross profit: $20.00
            500000000,    // Gas fee: $5.00
            600000000,    // DEX fees: $6.00
            100000000,    // Slippage: $1.00
            1300000000,   // Net profit: $13.00
            10000,        // Profit percent: 1.00%
            900,          // Confidence: 90%
            true,
            "USDC",
            "WETH",
            "uniswap_v3",
            "sushiswap",
            1234,
            SourceType::Scanner,
        );
        
        // Convert to DeFiPoolSignal
        let signal = DeFiPoolSignal::from_arbitrage(&arb);
        
        // Verify conversion (copy packed fields to avoid alignment issues)
        assert_eq!(signal.token0_id, usdc);
        assert_eq!(signal.token1_id, weth);
        assert_eq!(signal.pool_a_id, pool1);
        assert_eq!(signal.pool_b_id, pool2);
        
        // Copy packed fields to local variables to avoid alignment issues
        let optimal_amount = signal.optimal_amount_in;
        let profit = signal.expected_profit_wei;
        let conf = signal.confidence;
        
        assert_eq!(optimal_amount, 100000000000);
        assert_eq!(profit, 1300000000);
        assert_eq!(conf, 900);
        
        // Test serialization
        let bytes = signal.as_bytes();
        let restored = DeFiPoolSignal::from_bytes(bytes).unwrap();
        assert_eq!(restored.token0_id, usdc);
        assert_eq!(restored.token1_id, weth);
    }

    #[test]
    fn test_defi_signal_struct_size() {
        // Verify size is exactly 256 bytes as designed
        assert_eq!(size_of::<DeFiSignal>(), 256);
        assert_eq!(DeFiSignal::SIZE, 256);
    }

    #[test]
    fn test_defi_signal_creation() {
        let usdc = InstrumentId::ethereum_token("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48").unwrap();
        let weth = InstrumentId::ethereum_token("0xc02aaa39b223fe8d0a0e5c4f27b010c5d91b298a").unwrap();

        let signal = DeFiSignal::new(
            DeFiSignal::STRATEGY_CROSS_DEX_ARB,
            12345,
            1001,
            1, // Ethereum mainnet
            usdc,
            weth,
            5678,
            SourceType::ArbitrageStrategy,
        );

        // Test identity fields
        assert_eq!(signal.strategy_id, DeFiSignal::STRATEGY_CROSS_DEX_ARB);
        assert_eq!(signal.signal_id, 12345);
        assert_eq!(signal.signal_nonce, 1001);
        assert_eq!(signal.chain_id, 1);
        assert_eq!(signal.version, 1);

        // Test instrument correlation
        assert_eq!(signal.base_instrument, usdc);
        assert_eq!(signal.quote_instrument, weth);

        // Test header
        assert_eq!(signal.header.message_type().unwrap(), MessageType::DeFiSignal);
        assert_eq!(signal.header.source().unwrap(), SourceType::ArbitrageStrategy);
        let header_sequence = signal.header.sequence;
        let header_payload_size = signal.header.payload_size;
        assert_eq!(header_sequence, 5678);
        assert_eq!(header_payload_size, (256 - size_of::<MessageHeader>()) as u32);

        // Test TLV defaults (copy packed fields to avoid alignment issues)
        let tlv_offset = signal.tlv_offset;
        let tlv_length = signal.tlv_length;
        assert_eq!(tlv_offset, 256);
        assert_eq!(tlv_length, 0);
    }

    #[test]
    fn test_defi_signal_execution_flags() {
        let usdc = InstrumentId::ethereum_token("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48").unwrap();
        let weth = InstrumentId::ethereum_token("0xc02aaa39b223fe8d0a0e5c4f27b010c5d91b298a").unwrap();

        let mut signal = DeFiSignal::new(
            DeFiSignal::STRATEGY_TRIANGULAR_ARB,
            99999,
            2001,
            137, // Polygon
            usdc,
            weth,
            9876,
            SourceType::ArbitrageStrategy,
        );

        // Test transaction policy flags
        signal.set_tx_policy(DeFiSignal::TX_POLICY_PRIVATE);
        assert_eq!(signal.tx_policy(), DeFiSignal::TX_POLICY_PRIVATE);

        signal.set_tx_policy(DeFiSignal::TX_POLICY_BUNDLE);
        assert_eq!(signal.tx_policy(), DeFiSignal::TX_POLICY_BUNDLE);

        // Test priority flags
        signal.set_priority(DeFiSignal::PRIORITY_URGENT);
        assert_eq!(signal.priority(), DeFiSignal::PRIORITY_URGENT);

        signal.set_priority(DeFiSignal::PRIORITY_CRITICAL);
        assert_eq!(signal.priority(), DeFiSignal::PRIORITY_CRITICAL);

        // Test approval flags (bitfield)
        signal.set_approval_needs(0b1101); // Need approval for base + quote + router
        assert_eq!(signal.approval_needs(), 0b1101);

        // Test venue count
        signal.set_venue_count(2); // Two-venue arbitrage
        assert_eq!(signal.venue_count(), 2);

        // Test combined flags don't interfere
        assert_eq!(signal.tx_policy(), DeFiSignal::TX_POLICY_BUNDLE);
        assert_eq!(signal.priority(), DeFiSignal::PRIORITY_CRITICAL);
        assert_eq!(signal.approval_needs(), 0b1101);
        assert_eq!(signal.venue_count(), 2);
    }

    #[test]
    fn test_defi_signal_economics() {
        let usdc = InstrumentId::ethereum_token("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48").unwrap();
        let weth = InstrumentId::ethereum_token("0xc02aaa39b223fe8d0a0e5c4f27b010c5d91b298a").unwrap();

        let mut signal = DeFiSignal::new(
            DeFiSignal::STRATEGY_FLASH_LOAN_ARB,
            54321,
            3001,
            42161, // Arbitrum
            usdc,
            weth,
            1111,
            SourceType::ArbitrageStrategy,
        );

        // Set economics (Q64.64 fixed point)
        signal.expected_profit_q = 2000000000i128; // $20.00 profit
        signal.required_capital_q = 100000000000u128; // $1000.00 capital
        signal.gas_estimate_q = 500000000u128; // $5.00 gas
        signal.amount_in_q = 50000000000u128; // $500.00 input
        signal.min_out_q = 51000000000u128; // $510.00 minimum output (2% slippage)
        signal.optimal_size_q = 75000000000u128; // $750.00 optimal size

        // Test economics preserved (copy packed fields to avoid alignment issues)
        let expected_profit = signal.expected_profit_q;
        let required_capital = signal.required_capital_q;
        let gas_estimate = signal.gas_estimate_q;
        let amount_in = signal.amount_in_q;
        let min_out = signal.min_out_q;
        let optimal_size = signal.optimal_size_q;
        
        assert_eq!(expected_profit, 2000000000i128);
        assert_eq!(required_capital, 100000000000u128);
        assert_eq!(gas_estimate, 500000000u128);
        assert_eq!(amount_in, 50000000000u128);
        assert_eq!(min_out, 51000000000u128);
        assert_eq!(optimal_size, 75000000000u128);

        // Test execution parameters
        signal.slippage_bps = 200; // 2% slippage
        signal.price_impact_bps = 150; // 1.5% price impact
        signal.confidence = 85; // 85% confidence

        let slippage_bps = signal.slippage_bps;
        let price_impact_bps = signal.price_impact_bps;
        let confidence = signal.confidence;
        assert_eq!(slippage_bps, 200);
        assert_eq!(price_impact_bps, 150);
        assert_eq!(confidence, 85);
    }

    #[test]
    fn test_defi_signal_venue_configuration() {
        let usdc = InstrumentId::polygon_token("0x2791bca1f2de4661ed88a30c99a7a9449aa84174").unwrap();
        let weth = InstrumentId::polygon_token("0x7ceb23fd6bc0add59e62ac25578270cff1b9f619").unwrap();

        let mut signal = DeFiSignal::new(
            DeFiSignal::STRATEGY_CROSS_DEX_ARB,
            77777,
            4001,
            137, // Polygon
            usdc,
            weth,
            2222,
            SourceType::ArbitrageStrategy,
        );

        // Configure execution addresses (example Uniswap V3 and SushiSwap on Polygon)
        signal.base_token_addr = hex::decode("2791bca1f2de4661ed88a30c99a7a9449aa84174")
            .unwrap()
            .try_into()
            .unwrap();
        signal.quote_token_addr = hex::decode("7ceb23fd6bc0add59e62ac25578270cff1b9f619")
            .unwrap()
            .try_into()
            .unwrap();

        // Example router addresses (not real addresses, just for testing)
        signal.venue_a_router = [0x01; 20]; // Mock UniV3 router
        signal.venue_b_router = [0x02; 20]; // Mock SushiSwap router

        // Configure venue metadata
        signal.venue_a_type = DeFiSignal::VENUE_UNISWAP_V3;
        signal.venue_b_type = DeFiSignal::VENUE_SUSHISWAP;
        signal.fee_a_ppm = 500; // 0.05% UniV3 fee tier
        signal.fee_b_ppm = 3000; // 0.3% SushiSwap fee
        signal.direction_flag = 1; // Buy base token

        // Test venue configuration preserved (copy packed fields to avoid alignment issues)
        let venue_a_type = signal.venue_a_type;
        let venue_b_type = signal.venue_b_type;
        let fee_a_ppm = signal.fee_a_ppm;
        let fee_b_ppm = signal.fee_b_ppm;
        let direction_flag = signal.direction_flag;
        
        assert_eq!(venue_a_type, DeFiSignal::VENUE_UNISWAP_V3);
        assert_eq!(venue_b_type, DeFiSignal::VENUE_SUSHISWAP);
        assert_eq!(fee_a_ppm, 500);
        assert_eq!(fee_b_ppm, 3000);
        assert_eq!(direction_flag, 1);

        // Test venue name strings
        assert_eq!(signal.venue_a_name(), "uniswap_v3");
    }

    #[test]
    fn test_defi_signal_block_validity() {
        let usdc = InstrumentId::ethereum_token("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48").unwrap();
        let weth = InstrumentId::ethereum_token("0xc02aaa39b223fe8d0a0e5c4f27b010c5d91b298a").unwrap();

        let mut signal = DeFiSignal::new(
            DeFiSignal::STRATEGY_LIQUIDATION,
            88888,
            5001,
            1, // Ethereum
            usdc,
            weth,
            3333,
            SourceType::ArbitrageStrategy,
        );

        // Set block-based validity
        signal.observed_block = 18500000; // Block used for simulation
        signal.valid_through_block = 18500010; // Valid for 10 blocks
        signal.state_hash = 0xDEADBEEFCAFEBABE; // Pool state hash

        // Test block validity logic
        assert!(!signal.is_expired(18500000)); // Same block - not expired
        assert!(!signal.is_expired(18500005)); // 5 blocks later - not expired
        assert!(!signal.is_expired(18500010)); // Exactly at valid_through_block - not expired
        assert!(signal.is_expired(18500011)); // 1 block past - expired
        assert!(signal.is_expired(18600000)); // Far in future - expired

        // Test state hash preserved (copy packed fields to avoid alignment issues)
        let state_hash = signal.state_hash;
        let observed_block = signal.observed_block;
        let valid_through_block = signal.valid_through_block;
        assert_eq!(state_hash, 0xDEADBEEFCAFEBABE);
        assert_eq!(observed_block, 18500000);
        assert_eq!(valid_through_block, 18500010);
    }

    #[test]
    fn test_defi_signal_serialization_roundtrip() {
        let usdc = InstrumentId::ethereum_token("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48").unwrap();
        let weth = InstrumentId::ethereum_token("0xc02aaa39b223fe8d0a0e5c4f27b010c5d91b298a").unwrap();

        let mut signal = DeFiSignal::new(
            DeFiSignal::STRATEGY_MEV_SANDWICH,
            99999999,
            6001,
            1, // Ethereum
            usdc,
            weth,
            4444,
            SourceType::ArbitrageStrategy,
        );

        // Set comprehensive data
        signal.base_token_addr = [0xAA; 20];
        signal.quote_token_addr = [0xBB; 20];
        signal.venue_a_router = [0xCC; 20];
        signal.venue_b_router = [0xDD; 20];
        signal.venue_a_type = DeFiSignal::VENUE_CURVE;
        signal.venue_b_type = DeFiSignal::VENUE_BALANCER;
        signal.fee_a_ppm = 1000;
        signal.fee_b_ppm = 2000;
        signal.direction_flag = 0;
        signal.confidence = 95;
        signal.expected_profit_q = 5000000000i128;
        signal.required_capital_q = 200000000000u128;
        signal.gas_estimate_q = 1000000000u128;
        signal.amount_in_q = 150000000000u128;
        signal.min_out_q = 155000000000u128;
        signal.optimal_size_q = 175000000000u128;
        signal.observed_block = 19000000;
        signal.valid_through_block = 19000020;
        signal.state_hash = 0xFEEDFACEDEADBEEF;
        signal.set_tx_policy(DeFiSignal::TX_POLICY_PRIVATE);
        signal.set_priority(DeFiSignal::PRIORITY_URGENT);
        signal.set_approval_needs(0b1111);
        signal.set_venue_count(3);
        signal.slippage_bps = 250;
        signal.price_impact_bps = 180;
        signal.replace_signal_id = 88888888;
        signal.adapter_id = 42;

        // Serialize to bytes
        let bytes = signal.to_bytes();
        assert_eq!(bytes.len(), 256);

        // Deserialize from bytes
        let restored = DeFiSignal::from_bytes(&bytes).unwrap();

        // Test all fields preserved (copy packed fields to avoid alignment issues)
        let r_strategy_id = restored.strategy_id;
        let r_signal_id = restored.signal_id;
        let r_signal_nonce = restored.signal_nonce;
        let r_chain_id = restored.chain_id;
        let r_version = restored.version;
        let r_adapter_id = restored.adapter_id;
        let r_base_instrument = restored.base_instrument;
        let r_quote_instrument = restored.quote_instrument;
        let r_base_token_addr = restored.base_token_addr;
        let r_quote_token_addr = restored.quote_token_addr;
        let r_venue_a_router = restored.venue_a_router;
        let r_venue_b_router = restored.venue_b_router;
        let r_venue_a_type = restored.venue_a_type;
        let r_venue_b_type = restored.venue_b_type;
        let r_fee_a_ppm = restored.fee_a_ppm;
        let r_fee_b_ppm = restored.fee_b_ppm;
        let r_direction_flag = restored.direction_flag;
        let r_confidence = restored.confidence;
        let r_expected_profit_q = restored.expected_profit_q;
        let r_required_capital_q = restored.required_capital_q;
        let r_gas_estimate_q = restored.gas_estimate_q;
        let r_amount_in_q = restored.amount_in_q;
        let r_min_out_q = restored.min_out_q;
        let r_optimal_size_q = restored.optimal_size_q;
        let r_observed_block = restored.observed_block;
        let r_valid_through_block = restored.valid_through_block;
        let r_state_hash = restored.state_hash;
        let r_slippage_bps = restored.slippage_bps;
        let r_price_impact_bps = restored.price_impact_bps;
        let r_replace_signal_id = restored.replace_signal_id;

        assert_eq!(r_strategy_id, DeFiSignal::STRATEGY_MEV_SANDWICH);
        assert_eq!(r_signal_id, 99999999);
        assert_eq!(r_signal_nonce, 6001);
        assert_eq!(r_chain_id, 1);
        assert_eq!(r_version, 1);
        assert_eq!(r_adapter_id, 42);
        assert_eq!(r_base_instrument, usdc);
        assert_eq!(r_quote_instrument, weth);
        assert_eq!(r_base_token_addr, [0xAA; 20]);
        assert_eq!(r_quote_token_addr, [0xBB; 20]);
        assert_eq!(r_venue_a_router, [0xCC; 20]);
        assert_eq!(r_venue_b_router, [0xDD; 20]);
        assert_eq!(r_venue_a_type, DeFiSignal::VENUE_CURVE);
        assert_eq!(r_venue_b_type, DeFiSignal::VENUE_BALANCER);
        assert_eq!(r_fee_a_ppm, 1000);
        assert_eq!(r_fee_b_ppm, 2000);
        assert_eq!(r_direction_flag, 0);
        assert_eq!(r_confidence, 95);
        assert_eq!(r_expected_profit_q, 5000000000i128);
        assert_eq!(r_required_capital_q, 200000000000u128);
        assert_eq!(r_gas_estimate_q, 1000000000u128);
        assert_eq!(r_amount_in_q, 150000000000u128);
        assert_eq!(r_min_out_q, 155000000000u128);
        assert_eq!(r_optimal_size_q, 175000000000u128);
        assert_eq!(r_observed_block, 19000000);
        assert_eq!(r_valid_through_block, 19000020);
        assert_eq!(r_state_hash, 0xFEEDFACEDEADBEEF);
        assert_eq!(restored.tx_policy(), DeFiSignal::TX_POLICY_PRIVATE);
        assert_eq!(restored.priority(), DeFiSignal::PRIORITY_URGENT);
        assert_eq!(restored.approval_needs(), 0b1111);
        assert_eq!(restored.venue_count(), 3);
        assert_eq!(r_slippage_bps, 250);
        assert_eq!(r_price_impact_bps, 180);
        assert_eq!(r_replace_signal_id, 88888888);

        // Test strategy name
        assert_eq!(restored.strategy_name(), "mev_sandwich");
    }

    #[test]
    fn test_pool_address_tlv() {
        let tlv = PoolAddressTLV::new(
            [0x11; 20], // Venue A pool
            [0x22; 20], // Venue B pool
        );

        assert_eq!(tlv.tlv_type, TLVType::PoolAddresses as u8);
        assert_eq!(tlv.tlv_length, 42);
        assert_eq!(tlv.venue_a_pool, [0x11; 20]);
        assert_eq!(tlv.venue_b_pool, [0x22; 20]);
        assert_eq!(PoolAddressTLV::SIZE, 44);

        // Test serialization
        let bytes = tlv.as_bytes();
        assert_eq!(bytes.len(), 44);

        let restored = zerocopy::Ref::<_, PoolAddressTLV>::new(bytes)
            .unwrap()
            .into_ref();
        assert_eq!(restored.venue_a_pool, [0x11; 20]);
        assert_eq!(restored.venue_b_pool, [0x22; 20]);
    }

    #[test]
    fn test_tertiary_venue_tlv() {
        let tlv = TertiaryVenueTLV::new(
            [0x33; 20], // Venue C router
            DeFiSignal::VENUE_BALANCER,
        );

        assert_eq!(tlv.tlv_type, TLVType::TertiaryVenue as u8);
        assert_eq!(tlv.tlv_length, 22);
        assert_eq!(tlv.venue_c_router, [0x33; 20]);
        assert_eq!(tlv.venue_c_type, DeFiSignal::VENUE_BALANCER);
        assert_eq!(TertiaryVenueTLV::SIZE, 24);

        // Test serialization
        let bytes = tlv.as_bytes();
        assert_eq!(bytes.len(), 24);

        let restored = zerocopy::Ref::<_, TertiaryVenueTLV>::new(bytes)
            .unwrap()
            .into_ref();
        assert_eq!(restored.venue_c_router, [0x33; 20]);
        assert_eq!(restored.venue_c_type, DeFiSignal::VENUE_BALANCER);
    }
}
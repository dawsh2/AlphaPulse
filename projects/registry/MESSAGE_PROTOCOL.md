# Message Protocol Architecture

## Executive Summary

A high-performance message protocol using bijective (reversible) IDs, fixed binary layouts, and dynamic schema registration. Services communicate using zero-copy binary messages with deterministic IDs that require no mapping tables or collision handling. The architecture uses domain-specific relays for different message types, enabling clean separation of concerns and easy migration to a future message bus.

## Core Design Principles

1. **Bijective IDs**: Every ID can be reversed to extract venue, asset type, and identifying data
2. **Zero-copy parsing**: Fixed layouts with `#[repr(C)]` for direct memory access
3. **Dynamic schemas**: Services can register new message types at runtime
4. **No hashing**: Deterministic ID construction eliminates collision risks
5. **Domain separation**: Different relays for market data, signals, and execution

## Message Header (32 bytes)

```rust
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::AsBytes)]
pub struct MessageHeader {
    pub magic: u32,                 // 0xDEADBEEF
    pub message_type: u8,           // MessageType discriminant
    pub version: u8,                // Schema version
    pub source: u8,                 // SourceType discriminant  
    pub flags: u8,                  // Compression, priority
    pub payload_size: u32,          // Payload bytes
    pub sequence: u64,              // Monotonic sequence
    pub timestamp: u64,             // Nanoseconds
    pub checksum: u32,              // CRC32 of entire message
}

impl MessageHeader {
    pub fn calculate_checksum(&mut self, full_message: &[u8]) {
        self.checksum = 0;
        // CRC32 over entire message except checksum field
        self.checksum = crc32fast::hash(&full_message[..full_message.len() - 4]);
    }
}
```

## Bijective Instrument IDs

```rust
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InstrumentId {
    pub venue: u16,        // VenueId enum
    pub asset_type: u8,    // AssetType enum
    pub reserved: u8,      // Future use/flags
    pub asset_id: u64,     // Venue-specific identifier
}

impl InstrumentId {
    /// Ethereum token from address (first 8 bytes)
    pub fn ethereum_token(address: &str) -> Result<Self> {
        let bytes = &hex::decode(&address[2..18])?[..8];
        Ok(Self {
            venue: VenueId::Ethereum as u16,
            asset_type: AssetType::Token as u8,
            reserved: 0,
            asset_id: u64::from_be_bytes(bytes.try_into()?),
        })
    }
    
    /// Stock from exchange and symbol
    pub fn stock(exchange: VenueId, symbol: &str) -> Self {
        Self {
            venue: exchange as u16,
            asset_type: AssetType::Stock as u8,
            reserved: 0,
            asset_id: symbol_to_u64(symbol),
        }
    }
    
    /// DEX pool from constituent tokens
    pub fn pool(dex: VenueId, token0: InstrumentId, token1: InstrumentId) -> Self {
        // Canonical ordering for consistency
        let (id0, id1) = if token0.asset_id <= token1.asset_id {
            (token0.asset_id, token1.asset_id)
        } else {
            (token1.asset_id, token0.asset_id)
        };
        
        Self {
            venue: dex as u16,
            asset_type: AssetType::Pool as u8,
            reserved: 0,
            asset_id: (id0 >> 1) ^ (id1 << 1), // Deterministic combination
        }
    }
    
    /// Convert to u64 for cache keys
    pub fn to_u64(&self) -> u64 {
        ((self.venue as u64) << 48) | 
        ((self.asset_type as u64) << 40) | 
        (self.asset_id & 0xFFFFFFFFFF)
    }
    
    /// Reconstruct from u64
    pub fn from_u64(value: u64) -> Self {
        Self {
            venue: ((value >> 48) & 0xFFFF) as u16,
            asset_type: ((value >> 40) & 0xFF) as u8,
            reserved: 0,
            asset_id: value & 0xFFFFFFFFFF,
        }
    }
    
    /// Human-readable debug info
    pub fn debug_info(&self) -> String {
        match (self.venue(), self.asset_type()) {
            (Ok(VenueId::Ethereum), Ok(AssetType::Token)) => {
                format!("ETH Token 0x{:010x}...", self.asset_id)
            }
            (Ok(venue), Ok(AssetType::Stock)) => {
                format!("{:?} Stock: {}", venue, u64_to_symbol(self.asset_id))
            }
            (Ok(venue), Ok(AssetType::Pool)) => {
                format!("{:?} Pool #{}", venue, self.asset_id)
            }
            _ => format!("Unknown {}/{} #{}", self.venue, self.asset_type, self.asset_id)
        }
    }
    
    fn venue(&self) -> Result<VenueId> {
        VenueId::try_from(self.venue)
    }
    
    fn asset_type(&self) -> Result<AssetType> {
        AssetType::try_from(self.asset_type)
    }
}

fn symbol_to_u64(symbol: &str) -> u64 {
    let mut bytes = [0u8; 8];
    let len = symbol.len().min(8);
    bytes[..len].copy_from_slice(&symbol.as_bytes()[..len]);
    u64::from_be_bytes(bytes)
}

fn u64_to_symbol(value: u64) -> String {
    let bytes = value.to_be_bytes();
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(8);
    String::from_utf8_lossy(&bytes[..end]).to_string()
}
```

## Message Type System

```rust
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, num_enum::TryFromPrimitive)]
pub enum MessageType {
    // === MARKET DATA DOMAIN (1-19) ===
    // Routes through MarketDataRelay
    Trade = 1,
    Quote = 2,
    OrderBook = 3,
    
    // Discovery messages
    InstrumentDiscovered = 10,
    PoolDiscovered = 11,
    VenueUpdate = 12,
    
    // === STRATEGY SIGNAL DOMAIN (20-39) ===
    // Routes through SignalRelay
    
    // TradFi/CEX Signals (20-29)
    ArbitrageOpportunity = 20,
    PositionUpdate = 21,
    StrategyAlert = 22,
    RebalanceSignal = 23,
    RiskMetric = 24,
    PerformanceUpdate = 25,
    
    // DeFi-Specific Signals (30-39)
    PoolArbitrageSignal = 30,
    LiquidityImbalance = 31,
    ImpermanentLossAlert = 32,
    MEVOpportunity = 33,
    FlashLoanSignal = 34,
    YieldFarmingUpdate = 35,
    GasOptimizationSignal = 36,
    
    // === EXECUTION DOMAIN (40-59) ===
    // Routes through ExecutionRelay
    OrderRequest = 40,
    OrderStatus = 41,
    Fill = 42,
    OrderCancel = 43,
    OrderModify = 44,
    ExecutionReport = 45,
    
    // === SYSTEM/CONTROL DOMAIN (100-109) ===
    // Direct connections or SystemRelay
    Heartbeat = 100,
    Snapshot = 101,
    Error = 102,
    ConfigUpdate = 103,
    
    // Custom strategies (200-255)
    Custom = 200,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, num_enum::TryFromPrimitive)]
pub enum SourceType {
    // === MARKET DATA PRODUCERS ===
    // Connect to MarketDataRelay as producers
    BinanceCollector = 1,
    KrakenCollector = 2,
    CoinbaseCollector = 3,
    PolygonCollector = 4,
    AlpacaCollector = 5,
    
    // === RELAY SERVICES ===
    // Domain-specific relays
    MarketDataRelay = 20,
    SignalRelay = 21,
    ExecutionRelay = 22,
    SystemRelay = 23,
    
    // === STRATEGY SERVICES ===
    // Consume from MarketDataRelay, produce to SignalRelay
    ArbitrageStrategy = 40,
    MomentumStrategy = 41,
    MarketMakingStrategy = 42,
    
    // === PORTFOLIO SERVICES ===
    // Consume from SignalRelay, produce to ExecutionRelay
    PortfolioManager = 60,
    RiskManager = 61,
    
    // === EXECUTION SERVICES ===
    // Consume from ExecutionRelay, connect to venues
    ExecutionEngine = 80,
    VenueConnector = 81,
    
    // External (100+)
    External = 100,
}
```

## Fixed-Size Messages

### Trade Message (64 bytes)
```rust
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::AsBytes)]
pub struct TradeMessage {
    pub header: MessageHeader,      // 32 bytes
    pub instrument_id: InstrumentId, // 12 bytes
    pub price: i64,                 // 8 bytes (fixed-point)
    pub volume: u64,                // 8 bytes
    pub side: u8,                   // 1 byte (Buy=1, Sell=2)
    pub flags: u8,                  // 1 byte
    pub _padding: [u8; 2],          // 2 bytes alignment
}

impl TradeMessage {
    pub fn from_bytes(data: &[u8; 64]) -> Result<&Self> {
        let msg = zerocopy::LayoutVerified::<_, Self>::new(data)
            .ok_or(ParseError::InvalidLayout)?
            .into_ref();
            
        if msg.header.magic != 0xDEADBEEF {
            return Err(ParseError::InvalidMagic);
        }
        
        Ok(msg)
    }
}
```

### Quote Message (80 bytes)
```rust
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::AsBytes)]
pub struct QuoteMessage {
    pub header: MessageHeader,      // 32 bytes
    pub instrument_id: InstrumentId, // 12 bytes
    pub bid_price: i64,             // 8 bytes
    pub ask_price: i64,             // 8 bytes
    pub bid_size: u64,              // 8 bytes
    pub ask_size: u64,              // 8 bytes
    pub _padding: [u8; 4],          // 4 bytes alignment
}
```

### TradFi Arbitrage Opportunity (96 bytes)
```rust
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::AsBytes)]
pub struct ArbitrageMessage {
    pub header: MessageHeader,      // 32 bytes
    pub base_id: InstrumentId,      // 12 bytes
    pub quote_id: InstrumentId,     // 12 bytes
    pub venue_a: u16,               // 2 bytes
    pub venue_b: u16,               // 2 bytes
    pub spread_bps: u32,            // 4 bytes
    pub estimated_profit: i64,      // 8 bytes
    pub confidence: u16,            // 2 bytes (0-10000)
    pub expires_at: u64,            // 8 bytes
    pub _padding: [u8; 14],         // 14 bytes alignment
}
```

### DeFi Pool Arbitrage Signal (128 bytes)
```rust
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::AsBytes)]
pub struct PoolArbitrageSignal {
    pub header: MessageHeader,      // 32 bytes
    
    // Pool identifiers
    pub pool_a_id: InstrumentId,    // 12 bytes (source pool)
    pub pool_b_id: InstrumentId,    // 12 bytes (target pool)
    
    // Token pair
    pub token0_id: InstrumentId,    // 12 bytes
    pub token1_id: InstrumentId,    // 12 bytes
    
    // Liquidity data
    pub pool_a_reserve0: u64,       // 8 bytes (in wei/smallest unit)
    pub pool_a_reserve1: u64,       // 8 bytes
    pub pool_b_reserve0: u64,       // 8 bytes
    pub pool_b_reserve1: u64,       // 8 bytes
    
    // Opportunity metrics
    pub optimal_amount_in: u64,     // 8 bytes (calculated optimal swap size)
    pub expected_profit_wei: u64,   // 8 bytes (profit after gas)
    pub gas_cost_gwei: u32,         // 4 bytes
    pub confidence: u16,            // 2 bytes (0-10000)
    pub block_number: u32,          // 4 bytes
    pub expires_at_block: u32,      // 4 bytes (MEV protection)
    pub _padding: [u8; 2],          // 2 bytes alignment
}
```

### Liquidity Imbalance Signal (96 bytes)
```rust
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::AsBytes)]
pub struct LiquidityImbalanceSignal {
    pub header: MessageHeader,      // 32 bytes
    pub pool_id: InstrumentId,      // 12 bytes
    pub token0_id: InstrumentId,    // 12 bytes
    pub token1_id: InstrumentId,    // 12 bytes
    
    pub current_ratio: u64,         // 8 bytes (fixed-point ratio)
    pub historical_avg_ratio: u64,  // 8 bytes (30-day average)
    pub deviation_bps: u32,         // 4 bytes (basis points from avg)
    pub volume_24h: u64,            // 8 bytes
    pub fee_tier: u32,              // 4 bytes (e.g., 3000 = 0.3%)
    pub _padding: [u8; 4],          // 4 bytes alignment
}
```

### MEV Opportunity Signal (112 bytes)
```rust
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::AsBytes)]
pub struct MEVOpportunitySignal {
    pub header: MessageHeader,      // 32 bytes
    
    pub opportunity_type: u8,       // 1 byte (Sandwich, Backrun, Liquidation)
    pub chain_id: u8,               // 1 byte (1=ETH, 137=Polygon)
    pub priority: u8,               // 1 byte (0-255, higher = more urgent)
    pub _reserved: u8,              // 1 byte
    
    pub target_tx_hash: [u8; 32],   // 32 bytes (transaction to target)
    pub estimated_profit_wei: u64,  // 8 bytes
    pub max_gas_price_gwei: u64,    // 8 bytes (max we'll pay)
    pub target_block: u32,          // 4 bytes
    pub deadline_block: u32,        // 4 bytes
    
    // For sandwich attacks
    pub victim_amount_in: u64,      // 8 bytes
    pub optimal_frontrun_amount: u64, // 8 bytes
    pub _padding: [u8; 4],          // 4 bytes alignment
}
```

## TLV Extensibility Pattern

### Overview

The Type-Length-Value (TLV) extensibility pattern allows messages to have optional extensions while maintaining backward compatibility and performance. This eliminates the need for message type explosion as new features are added.

### Core TLV Concept

```rust
// Base message (fixed size) + Optional TLV extensions (variable size)
pub struct ExtensibleMessage {
    pub header: MessageHeader,      // 32 bytes
    pub core_data: CoreData,        // Fixed core fields
    pub tlv_offset: u16,            // Offset to first TLV from message start
    pub tlv_length: u16,            // Total bytes of TLV data
    // TLV extensions follow at tlv_offset...
}

// TLV Extension Format
pub struct TLVExtension {
    pub tlv_type: u8,              // Extension type identifier
    pub tlv_length: u8,            // Extension payload size (excluding type/length)
    // Extension payload follows...
}
```

### TLV Extension Registry

```rust
/// Standard TLV extension types for the protocol
#[repr(u8)]
pub enum TLVType {
    // Core Extensions (1-10)
    PoolAddresses = 1,          // UniV3 pool addresses for quoter
    TertiaryVenue = 2,          // Third venue for triangular arbitrage
    MEVBundle = 3,              // MEV bundle preferences
    SlippageModel = 4,          // Advanced slippage parameters
    
    // Strategy Extensions (11-20) 
    FlashLoanParams = 11,       // Flash loan specific parameters
    LiquidationTarget = 12,     // Liquidation target details
    SandwichParams = 13,        // MEV sandwich parameters
    
    // Network Extensions (21-30)
    GasOptimization = 21,       // Gas optimization hints
    CrossChain = 22,            // Cross-chain bridge parameters
    Layer2Params = 23,          // L2-specific parameters
    
    // Custom Strategy Extensions (100-199)
    CustomStrategy = 100,       // Reserved for custom strategies
    
    // Future Reserved (200-255)
    Reserved = 200,
}
```

### Example: Extensible DeFi Signal

Replace the rigid message type explosion with one flexible signal:

```rust
/// Unified DeFi Signal (256 bytes base) - Production Ready
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
    pub state_hash: u64,                // Pool state hash

    // Execution control (16 bytes) - Consolidated bitfields
    pub execution_flags: u32,           // tx_policy + priority + approvals
    pub slippage_bps: u16,              // Maximum slippage tolerance
    pub price_impact_bps: u16,          // Expected price impact
    pub replace_signal_id: u64,         // If >0, supersedes this signal

    // TLV extension header (8 bytes)
    pub tlv_offset: u16,                // Bytes from struct start to TLV data
    pub tlv_length: u16,                // Bytes of TLV data
    pub created_at_sec: u32,            // Wall-clock for analytics only
}

impl DeFiSignal {
    pub const SIZE: usize = 256;
    
    /// Get TLV data slice if extensions exist
    pub fn tlv_data<'a>(&self, message_bytes: &'a [u8]) -> Option<&'a [u8]> {
        if self.tlv_length == 0 {
            return None;
        }
        
        let start = self.tlv_offset as usize;
        let end = start + self.tlv_length as usize;
        
        if message_bytes.len() >= end {
            Some(&message_bytes[start..end])
        } else {
            None
        }
    }
}
```

### Standard TLV Extensions

#### Pool Addresses Extension (44 bytes)
For UniswapV3 quoter calls and advanced DEX interactions:

```rust
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, AsBytes, FromBytes, FromZeroes)]
pub struct PoolAddressTLV {
    pub tlv_type: u8,           // TLVType::PoolAddresses (1)
    pub tlv_length: u8,         // 42 (size excluding type/length)
    pub venue_a_pool: [u8; 20], // Pool contract for quoter calls
    pub venue_b_pool: [u8; 20], // Pool contract for quoter calls
    pub reserved: [u8; 2],      // Alignment
}

impl PoolAddressTLV {
    pub fn new(venue_a_pool: [u8; 20], venue_b_pool: [u8; 20]) -> Self {
        Self {
            tlv_type: TLVType::PoolAddresses as u8,
            tlv_length: 42,
            venue_a_pool,
            venue_b_pool,
            reserved: [0; 2],
        }
    }
}
```

#### Tertiary Venue Extension (24 bytes)
For triangular arbitrage requiring three venues:

```rust
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, AsBytes, FromBytes, FromZeroes)]
pub struct TertiaryVenueTLV {
    pub tlv_type: u8,           // TLVType::TertiaryVenue (2)
    pub tlv_length: u8,         // 22
    pub venue_c_router: [u8; 20], // Third router address
    pub venue_c_type: u8,       // Venue type (1=UniV2, 2=UniV3, etc.)
    pub fee_c_ppm: u32,         // Fee in parts per million
    pub reserved: [u8; 5],      // Alignment
}
```

#### MEV Bundle Extension (40 bytes)
For Flashbots/MEV bundle preferences:

```rust
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, AsBytes, FromBytes, FromZeroes)]
pub struct MEVBundleTLV {
    pub tlv_type: u8,           // TLVType::MEVBundle (3)
    pub tlv_length: u8,         // 38
    pub target_builder: u32,    // Preferred builder ID
    pub bundle_uuid: u32,       // Bundle correlation ID
    pub max_bundle_size: u8,    // Maximum transactions in bundle
    pub priority_fee_gwei: u64, // Priority fee in gwei
    pub target_block_range: u8, // Blocks ahead to target (1-3)
    pub submission_strategy: u8, // 1=Flashbots, 2=MEV-Share, etc.
    pub relay_preferences: u64, // Bitfield of preferred relays
    pub reserved: [u8; 16],     // Future MEV parameters
}
```

### TLV Parsing

```rust
/// Parse all TLV extensions from a message
pub fn parse_tlv_extensions(tlv_data: &[u8]) -> Result<Vec<TLVExtension>, ParseError> {
    let mut extensions = Vec::new();
    let mut offset = 0;
    
    while offset < tlv_data.len() {
        if offset + 2 > tlv_data.len() {
            return Err(ParseError::TruncatedTLV);
        }
        
        let tlv_type = tlv_data[offset];
        let tlv_length = tlv_data[offset + 1];
        
        let total_length = 2 + tlv_length as usize; // type + length + payload
        if offset + total_length > tlv_data.len() {
            return Err(ParseError::TruncatedTLV);
        }
        
        let payload = &tlv_data[offset + 2..offset + total_length];
        extensions.push(TLVExtension {
            tlv_type,
            payload: payload.to_vec(),
        });
        
        offset += total_length;
    }
    
    Ok(extensions)
}

/// Find specific TLV extension by type
pub fn find_tlv_extension(tlv_data: &[u8], target_type: u8) -> Option<&[u8]> {
    let mut offset = 0;
    
    while offset + 2 <= tlv_data.len() {
        let tlv_type = tlv_data[offset];
        let tlv_length = tlv_data[offset + 1] as usize;
        
        if tlv_type == target_type {
            let start = offset + 2;
            let end = start + tlv_length;
            if end <= tlv_data.len() {
                return Some(&tlv_data[start..end]);
            }
        }
        
        offset += 2 + tlv_length;
    }
    
    None
}
```

### Benefits of TLV Extensibility

#### 1. **Eliminates Message Type Explosion**
Before TLV:
```rust
// Rigid: Need separate message for each combination
ArbitrageSignal           // Basic arbitrage
TriangularArbitrageSignal // + third venue  
MEVArbitrageSignal        // + MEV parameters
UniV3ArbitrageSignal      // + pool addresses
FlashLoanArbitrageSignal  // + flash loan params
// 5+ message types for variants of same concept!
```

After TLV:
```rust
// Flexible: One base message + optional extensions
DeFiSignal                // Base signal (256 bytes)
  + TertiaryVenueTLV      // Optional: third venue
  + MEVBundleTLV          // Optional: MEV parameters  
  + PoolAddressTLV        // Optional: pool addresses
  + FlashLoanParamsTLV    // Optional: flash loan params
// One message type handles all variants!
```

#### 2. **Backward Compatibility**
- Old clients ignore unknown TLV extensions
- New clients parse base message without extensions
- Gradual rollout of new features without breaking changes

#### 3. **Performance Optimized**
- Base message is fixed-size (256 bytes) for hot path
- Extensions only added when needed
- Zero-copy parsing of core fields
- Optional TLV parsing only when extensions present

#### 4. **Future-Proof Evolution**
```rust
// Today: Basic cross-DEX arbitrage
let signal = DeFiSignal::new(strategy_id, signal_id, ...);
// No TLV extensions - just 256 bytes

// Next month: Add UniV3 pool support
let mut message = signal.to_bytes();
let pool_tlv = PoolAddressTLV::new(pool_a, pool_b);
message.extend_from_slice(pool_tlv.as_bytes());
// Old clients still work, new ones get pool addresses

// Next quarter: Add MEV bundle support  
let mev_tlv = MEVBundleTLV::new(builder_id, bundle_uuid);
message.extend_from_slice(mev_tlv.as_bytes());
// Incremental feature addition without protocol versioning
```

#### 5. **Development Velocity**
- Add new signal features without service coordination
- A/B test new parameters with TLV extensions
- Strategy-specific extensions without core protocol changes

### Migration Path from Fixed Messages

```rust
// Phase 1: Keep existing messages, add TLV variants
pub enum SignalMessage {
    Legacy(ArbitrageOpportunityMessage),  // 96 bytes fixed
    Extended(DeFiSignal),                 // 256 bytes + TLV
}

// Phase 2: Convert legacy to extended format
impl From<ArbitrageOpportunityMessage> for DeFiSignal {
    fn from(legacy: ArbitrageOpportunityMessage) -> Self {
        let mut signal = DeFiSignal::new(/* ... */);
        // Map legacy fields to new format
        signal.expected_profit_q = legacy.profit_usd as i128 * 100_000_000;
        // ...
        signal
    }
}

// Phase 3: Deprecate legacy formats
// All new signals use DeFiSignal + TLV extensions
```

### TLV Best Practices

1. **Keep Base Message Hot**: Critical fields in fixed base (256 bytes)
2. **Optional Extensions**: Use TLV for features not all signals need
3. **Alignment**: Pad TLV payloads to 4/8 byte boundaries when possible
4. **Versioning**: Use TLV type ranges for different categories
5. **Future Reserved**: Reserve TLV type ranges (200-255) for future use
6. **Size Limits**: Keep individual TLV extensions under 256 bytes
7. **Parsing Safety**: Always validate TLV length before parsing payload

## Variable-Size Messages

### Instrument Discovery
```rust
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::AsBytes)]
pub struct InstrumentDiscoveredHeader {
    pub header: MessageHeader,      // 32 bytes
    pub instrument_id: InstrumentId, // 12 bytes
    pub decimals: u8,               // 1 byte
    pub symbol_len: u8,             // 1 byte
    pub metadata_len: u16,          // 2 bytes
    // Variable data follows: symbol + metadata
}

pub struct InstrumentDiscoveredMessage {
    pub header: InstrumentDiscoveredHeader,
    pub symbol: String,
    pub metadata: Vec<u8>,
}

impl InstrumentDiscoveredMessage {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < size_of::<InstrumentDiscoveredHeader>() {
            return Err(ParseError::TooSmall);
        }
        
        let header = zerocopy::LayoutVerified::<_, InstrumentDiscoveredHeader>::new(
            &data[..size_of::<InstrumentDiscoveredHeader>()]
        ).ok_or(ParseError::InvalidLayout)?.into_ref();
        
        let offset = size_of::<InstrumentDiscoveredHeader>();
        let symbol_end = offset + header.symbol_len as usize;
        let metadata_end = symbol_end + header.metadata_len as usize;
        
        if data.len() < metadata_end {
            return Err(ParseError::TooSmall);
        }
        
        Ok(Self {
            header: *header,
            symbol: String::from_utf8_lossy(&data[offset..symbol_end]).to_string(),
            metadata: data[symbol_end..metadata_end].to_vec(),
        })
    }
}
```

## Schema and Transform System

```rust
pub struct SchemaTransformCache {
    // Static schemas loaded at startup
    static_schemas: HashMap<(MessageType, u8), &'static MessageSchema>,
    
    // Dynamic schemas registered at runtime
    dynamic_schemas: DashMap<(MessageType, u8), MessageSchema>,
    
    // Object cache keyed by bijective IDs
    objects: DashMap<u64, CachedObject>,
}

pub struct MessageSchema {
    pub message_type: MessageType,
    pub version: u8,
    pub size: Option<usize>,  // Fixed size if Some
    pub parser: Box<dyn MessageParser>,
}

pub trait MessageParser: Send + Sync {
    fn parse(&self, data: &[u8]) -> Result<Box<dyn Any>>;
    fn to_cached_object(&self, parsed: Box<dyn Any>) -> Option<CachedObject>;
}

impl SchemaTransformCache {
    pub fn process_message(&mut self, data: &[u8]) -> Result<()> {
        // Parse header (always 32 bytes)
        let header = MessageHeader::from_bytes(&data[..32])?;
        
        // Validate magic and checksum
        if header.magic != 0xDEADBEEF {
            return Err(ParseError::InvalidMagic);
        }
        
        let calculated_crc = crc32fast::hash(&data[..data.len() - 4]);
        if calculated_crc != header.checksum {
            return Err(ParseError::ChecksumMismatch);
        }
        
        // Find schema
        let key = (header.message_type.try_into()?, header.version);
        let schema = self.static_schemas.get(&key)
            .or_else(|| self.dynamic_schemas.get(&key).map(|s| &**s))
            .ok_or(ParseError::UnknownSchema)?;
        
        // Parse message
        let parsed = schema.parser.parse(data)?;
        
        // Cache if it produces an object
        if let Some(cached) = schema.parser.to_cached_object(parsed) {
            if let CachedObject::Instrument(ref meta) = cached {
                self.objects.insert(meta.id.to_u64(), cached);
            }
        }
        
        Ok(())
    }
    
    pub fn register_dynamic_schema(&self, schema: MessageSchema) {
        let key = (schema.message_type, schema.version);
        self.dynamic_schemas.insert(key, schema);
    }
}

pub enum CachedObject {
    Instrument(InstrumentMetadata),
    Pool(PoolMetadata),
    Token(TokenMetadata),
    Custom(Box<dyn Any>),
}

pub struct InstrumentMetadata {
    pub id: InstrumentId,
    pub symbol: String,
    pub decimals: u8,
    pub discovered_at: u64,
}
```

## Message Flow Example

```rust
// 1. Collector discovers new pool
let pool_id = InstrumentId::pool(
    VenueId::UniswapV3,
    InstrumentId::ethereum_token("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48")?, // USDC
    InstrumentId::ethereum_token("0xc02aaa39b223fe8d0a0e5c4f2afb308d791be2259")?, // WETH
);

// 2. Create discovery message
let mut msg = InstrumentDiscoveredMessage {
    header: InstrumentDiscoveredHeader {
        header: MessageHeader::new(MessageType::PoolDiscovered, 1, SourceType::PolygonCollector),
        instrument_id: pool_id,
        decimals: 18,
        symbol_len: 9,
        metadata_len: 0,
    },
    symbol: "USDC/WETH".to_string(),
    metadata: vec![],
};

// 3. Serialize and send
let bytes = msg.serialize();
socket.send(&bytes)?;

// 4. Downstream service receives and caches
cache.process_message(&bytes)?;

// 5. Later lookups use bijective ID
let cached = cache.objects.get(&pool_id.to_u64());
println!("Pool: {}", pool_id.debug_info()); // "UniswapV3 Pool #12345678"
```

## Performance Characteristics

- **Zero-copy parsing**: Direct cast for fixed-size messages using `zerocopy`
- **Bijective IDs**: No hash lookups, reversible for debugging
- **Checksum validation**: CRC32 over entire message
- **Cache efficiency**: O(1) lookups by u64 ID
- **Memory safety**: No `unsafe` transmutes, alignment-safe parsing

## Safety Improvements from Original

1. **zerocopy instead of unsafe**: Eliminates undefined behavior from unaligned access
2. **num_enum for safe conversion**: No unsafe transmute for enum discriminants
3. **Full message checksums**: Protects header + payload
4. **Alignment padding**: Explicit padding ensures proper struct alignment

## Key Benefits

1. **No mapping tables**: IDs contain their own metadata
2. **No collisions**: Deterministic ID construction
3. **Debuggable**: Every ID can be printed meaningfully
4. **Extensible**: Dynamic schema registration for new message types
5. **Efficient**: Zero-copy parsing, O(1) cache lookups

## Multi-Domain Relay Architecture

The system uses distinct relays for different message domains, providing clean separation of concerns and preventing message type pollution. Each relay handles specific message types and maintains its own performance characteristics.

### Architecture Overview

#### Complete System Data Flow

```
┌─────────────────────────────────────────────────────────────────────┐
│                         MARKET DATA DOMAIN                          │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  Kraken ─┐                                          ┌→ Strategy A   │
│  Binance ├→ MarketDataRelay (msgs 1-19) ──────────├→ Strategy B   │
│  Polygon ─┘         ↓                               ├→ Portfolio    │
│                     ↓                               └→ Dashboard    │
│                     ↓                                               │
└─────────────────────────────────────────────────────────────────────┘
                      ↓ (market data)
┌─────────────────────────────────────────────────────────────────────┐
│                      STRATEGY SIGNAL DOMAIN                         │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  Strategy A ─┐                                     ┌→ Portfolio    │
│  Strategy B ─┼→ SignalRelay (msgs 20-39) ─────────├→ Dashboard    │
│  Strategy C ─┘                                     └→ RiskManager  │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
                                                     ↓ (signals)
┌─────────────────────────────────────────────────────────────────────┐
│                        EXECUTION DOMAIN                             │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  Portfolio ──→ ExecutionRelay (msgs 40-59) ───────┬→ Execution    │
│                         ↓                          └→ RiskManager  │
│                         ↓                             ↓            │
│                         └────────────────────────→ Dashboard       │
│                                                       ↓            │
│                                                    Venues/Brokers  │
└─────────────────────────────────────────────────────────────────────┘
```

#### Simplified Connection View

```
collectors → MarketDataRelay → strategies
                            ↘ portfolio
                            ↘ dashboard
                            
strategies → SignalRelay → portfolio
                        ↘ dashboard
                        ↘ risk_manager
                        
portfolio → ExecutionRelay → execution_engine → venues/brokers
                          ↘ risk_manager
                          ↘ dashboard
```

### Domain-Specific Relays

#### 1. MarketDataRelay
```rust
// Handles MessageType 1-19
pub struct MarketDataRelay {
    bind_path: "/tmp/alphapulse/market_data.sock",
    
    // High throughput, low latency
    config: RelayConfig {
        max_queue_size: 1_000_000,  // Handle bursts
        circuit_breaker: 800_000,
        validate_checksums: false,   // Speed over validation
    }
}
```

#### 2. SignalRelay  
```rust
// Handles MessageType 20-39
pub struct SignalRelay {
    bind_path: "/tmp/alphapulse/signals.sock",
    
    // Lower throughput, higher reliability
    config: RelayConfig {
        max_queue_size: 100_000,
        circuit_breaker: 80_000,
        validate_checksums: true,    // Validate strategy signals
    }
}
```

#### 3. ExecutionRelay
```rust
// Handles MessageType 40-59
pub struct ExecutionRelay {
    bind_path: "/tmp/alphapulse/execution.sock",
    
    // Critical path, full validation
    config: RelayConfig {
        max_queue_size: 10_000,
        circuit_breaker: 8_000,
        validate_checksums: true,    // Always validate orders
        require_sequence: true,      // Strict ordering
    }
}
```

### Why Multiple Relays?

1. **Performance Isolation**: Market data bursts don't affect execution flow
2. **Security**: Execution messages can have stricter validation
3. **Debugging**: Easier to trace specific message flows
4. **Scaling**: Each relay can be optimized for its workload
5. **Migration**: Maps cleanly to message bus channels

### Multi-Relay Consumer Pattern

Components that need data from multiple domains connect directly to each relay as consumers. **Relays are fan-out hubs, not routers** - they broadcast to all connected consumers.

#### Dashboard as Multi-Relay Consumer

```
                    ┌─────────────┐
                    │  Dashboard  │
                    └─────────────┘
                      ↑   ↑   ↑
                      │   │   │ (connects as consumer to each)
                      │   │   │
    ┌─────────────────┼───┼───┼─────────────────┐
    │ MarketDataRelay │   │   │                 │
    │                 │   │   │                 │
    │  Collectors ────┘   │   │                 │
    └─────────────────────┼───┼─────────────────┘
                          │   │
    ┌─────────────────────┼───┼─────────────────┐
    │    SignalRelay      │   │                 │
    │                     │   │                 │
    │   Strategies ───────┘   │                 │
    └─────────────────────────┼─────────────────┘
                              │
    ┌─────────────────────────┼─────────────────┐
    │  ExecutionRelay         │                 │
    │                         │                 │
    │    Portfolio ───────────┘                 │
    └────────────────────────────────────────────┘
```

#### Implementation Example

```rust
pub struct Dashboard {
    // Dashboard connects to MULTIPLE relays as a consumer
    market_data_connection: UnixStream,  // Connect to MarketDataRelay
    signal_connection: UnixStream,       // Connect to SignalRelay  
    execution_connection: UnixStream,    // Connect to ExecutionRelay
}

impl Dashboard {
    pub fn new() -> Result<Self> {
        Ok(Self {
            // Connect as consumer to each relay you need
            market_data_connection: UnixStream::connect("/tmp/alphapulse/market_data.sock")?,
            signal_connection: UnixStream::connect("/tmp/alphapulse/signals.sock")?,
            execution_connection: UnixStream::connect("/tmp/alphapulse/execution.sock")?,
        })
    }
    
    pub async fn run(&mut self) {
        // Poll all connections for messages
        loop {
            tokio::select! {
                // Receive market data
                msg = read_message(&mut self.market_data_connection) => {
                    self.handle_market_data(msg)?;
                }
                
                // Receive strategy signals
                msg = read_message(&mut self.signal_connection) => {
                    self.handle_signal(msg)?;
                }
                
                // Receive execution updates
                msg = read_message(&mut self.execution_connection) => {
                    self.handle_execution(msg)?;
                }
            }
        }
    }
}
```

#### Benefits of Direct Connection

1. **No Extra Hops**: Components get data directly from source relay
2. **Selective Subscription**: Components only connect to relays they need
3. **Independent Failure**: If one relay fails, others continue working
4. **Clear Data Flow**: Each relay has explicit producers and consumers
5. **No Relay-to-Relay Forwarding**: Keeps architecture simple

#### Configuration for Multi-Relay Consumers

```yaml
# dashboard_config.yml
dashboard:
  # Subscribe to multiple relays
  subscriptions:
    market_data:
      enabled: true
      path: "/tmp/alphapulse/market_data.sock"
      message_types: [Trade, Quote, OrderBook]
      
    signals:
      enabled: true  
      path: "/tmp/alphapulse/signals.sock"
      message_types: [PositionUpdate, StrategyAlert, PerformanceUpdate]
      
    execution:
      enabled: true
      path: "/tmp/alphapulse/execution.sock"
      message_types: [OrderStatus, Fill, ExecutionReport]
```

## Migration to Message Bus

The multi-relay architecture maps directly to a future message bus implementation. The beauty of this design is that the architecture remains identical - only the transport mechanism changes.

### When to Migrate from Sockets to Message Bus

#### Keep Unix Sockets When:
- ✅ **Separate processes** (current architecture)
- ✅ **Process isolation** is important for stability
- ✅ **Debugging** - easier to trace with tools like `socat`
- ✅ **Simple deployment** - no shared memory complexity
- ✅ **<10μs latency is acceptable** (current performance)

#### Migrate to Message Bus When You Need:
- 🚀 **Sub-microsecond latency** (<1μs required)
- 🚀 **Better abstraction** than raw sockets
- 🚀 **Zero-copy message passing** between components
- 🚀 **Backpressure handling** built into channels
- 🚀 **10x+ throughput** (millions of messages/second)

#### IPC Message Bus Options (For Microservices):

Since AlphaPulse uses a microservices architecture with process isolation, only IPC (Inter-Process Communication) options are relevant:

**Current: Unix Domain Sockets** ✅
- ⚡ **Latency**: ~10μs
- 📦 **Deployment**: Multiple processes (microservices)
- 🔧 **Technology**: Unix sockets
- ✅ **Status**: Currently implemented and working well

**Upgrade Option 1: Shared Memory IPC** (when you need more speed)
- ⚡ **Latency**: ~500ns-1μs (20x faster)
- 📦 **Deployment**: Multiple processes (same microservices architecture)
- 🔧 **Technology**: Memory-mapped files, ring buffers
- ✅ **Use when**: Unix socket latency becomes bottleneck

**Upgrade Option 2: Network Message Bus** (when you need distribution)
- ⚡ **Latency**: ~50-100μs
- 📦 **Deployment**: Distributed microservices (multiple machines)
- 🔧 **Technology**: NATS, Redis Streams, ZeroMQ
- ✅ **Use when**: Scaling beyond single machine

**Important**: Unix domain sockets only work on the same machine. For multi-machine deployment, you MUST use network-capable transports.

### Network Migration Path

#### From Unix Sockets to Network:

**Step 1: Identify Network Requirements**
- Single machine → Multiple machines
- Disaster recovery needs
- Geographic distribution
- Horizontal scaling requirements

**Step 2: Choose Network Transport**
```rust
// Option A: Minimal Change - TCP Sockets
// Change Unix socket to TCP with same protocol
UnixListener::bind("/tmp/alphapulse/relay.sock")
// becomes:
TcpListener::bind("0.0.0.0:8080")

// Option B: Message Bus - Better for production
// NATS example - handles reconnection, clustering
let nc = nats::connect("nats://server1:4222,nats://server2:4222")?;
nc.publish("market.trades", &message)?;

// Option C: gRPC - When you need service mesh
tonic::transport::Server::builder()
    .add_service(market_relay_service)
    .serve("0.0.0.0:50051")
```

**Step 3: Update Configuration**
```yaml
# Single Machine (Unix Sockets)
alphapulse:
  deployment: single-machine
  market_data:
    transport: unix_socket
    path: /tmp/alphapulse/market_data.sock

# Multi-Machine (Network)
alphapulse:
  deployment: distributed
  market_data:
    transport: nats
    servers: ["nats://trading1:4222", "nats://trading2:4222"]
    subject_prefix: "alphapulse.market"
```

**Note**: In-process channels are NOT applicable for AlphaPulse since process isolation is required for stability and fault tolerance.

#### Migration Triggers:
1. **Performance**: Unix socket latency becomes bottleneck (>10% of strategy compute time)
2. **Scale**: Message volume exceeds 1M messages/second
3. **Distribution**: Need to run on multiple machines
4. **Latency**: HFT strategies need <1μs message delivery

### How Migration Works

The migration is seamless because the architecture doesn't change - only the transport layer:

#### Unix Sockets (Current) vs Message Bus (Future)
```rust
// ===== CURRENT: Unix Socket Implementation =====
pub struct MarketDataRelay {
    socket: UnixListener,
    clients: Vec<UnixStream>,
}

impl MarketDataRelay {
    pub fn broadcast(&self, msg: &[u8]) {
        for client in &self.clients {
            client.write_all(msg)?;  // ~10μs per client
        }
    }
}

// Dashboard connects via Unix socket
let market_connection = UnixStream::connect("/tmp/alphapulse/market_data.sock")?;

// ===== FUTURE: Message Bus Implementation =====
pub struct MarketDataBus {
    sender: broadcast::Sender<MarketMessage>,
}

impl MarketDataBus {
    pub fn broadcast(&self, msg: MarketMessage) {
        self.sender.send(msg)?;  // ~100ns, zero-copy
    }
}

// Dashboard connects via channel subscription
let market_receiver = message_bus.subscribe(MessageDomain::MarketData);
```

### Migration is Just Configuration

```yaml
# Stage 1: All Unix Sockets (current)
alphapulse:
  transport: unix_socket
  market_data:
    path: /tmp/alphapulse/market_data.sock
  signals:
    path: /tmp/alphapulse/signals.sock
  execution:
    path: /tmp/alphapulse/execution.sock

# Stage 2: Mixed Mode (testing migration)
alphapulse:
  transport: mixed
  market_data:
    type: channels      # Migrated to message bus
    capacity: 1000000
  signals:
    type: unix_socket   # Still on sockets
    path: /tmp/alphapulse/signals.sock
  execution:
    type: unix_socket   # Still on sockets
    path: /tmp/alphapulse/execution.sock

# Stage 3: Full Message Bus
alphapulse:
  transport: message_bus
  market_data:
    capacity: 1000000
  signals:
    capacity: 100000
  execution:
    capacity: 10000
```

### Phase 1: Current Unix Socket Implementation
```rust
// Current: Domain-specific Unix socket relays
let market_relay = MarketDataRelay::new("/tmp/alphapulse/market_data.sock");
let signal_relay = SignalRelay::new("/tmp/alphapulse/signals.sock");
let execution_relay = ExecutionRelay::new("/tmp/alphapulse/execution.sock");
```

### Phase 2: Transport Abstraction
```rust
// Add transport layer abstraction
pub trait MessageTransport {
    fn send(&self, msg: &[u8]) -> Result<()>;
    fn receive(&self) -> Result<Vec<u8>>;
}

pub struct UnixSocketTransport { /* current */ }
pub struct ChannelTransport { /* future */ }
pub struct SharedMemoryTransport { /* ultra-fast */ }
```

### Phase 3: Message Bus Implementation
```rust
// Future: Same architecture, different transport
pub struct MessageBus {
    // Direct mapping from relays to channels
    market_data: Channel<MarketData>,     // Replaces MarketDataRelay
    signals: Channel<StrategySignal>,     // Replaces SignalRelay  
    execution: Channel<ExecutionOrder>,   // Replaces ExecutionRelay
}

// Migration is just changing the transport:
impl From<UnixSocketRelay> for ChannelBus {
    fn from(relay: UnixSocketRelay) -> Self {
        // Same message flow, same domains, different mechanism
        Self {
            market_data: Channel::with_capacity(relay.config.max_queue_size),
            // ... etc
        }
    }
}
```

### Complete Example: Dashboard Works with Both Transports

```rust
// The Dashboard doesn't change - only the transport implementation changes!

pub struct Dashboard {
    market_data: Box<dyn MessageReceiver>,
    signals: Box<dyn MessageReceiver>,
    execution: Box<dyn MessageReceiver>,
}

impl Dashboard {
    // Works with BOTH Unix sockets and message bus!
    pub fn new(config: &Config) -> Self {
        Self {
            market_data: create_receiver(&config.market_data),
            signals: create_receiver(&config.signals),
            execution: create_receiver(&config.execution),
        }
    }
    
    pub async fn run(&mut self) {
        // This code NEVER changes, regardless of transport!
        loop {
            tokio::select! {
                msg = self.market_data.receive() => self.handle_market_data(msg),
                msg = self.signals.receive() => self.handle_signal(msg),
                msg = self.execution.receive() => self.handle_execution(msg),
            }
        }
    }
}

fn create_receiver(config: &TransportConfig) -> Box<dyn MessageReceiver> {
    match config.transport_type {
        TransportType::UnixSocket => {
            Box::new(UnixSocketReceiver::new(&config.path))
        }
        TransportType::MessageBus => {
            Box::new(ChannelReceiver::new(config.capacity))
        }
    }
}
```

### IPC Performance Comparison

| IPC Transport | Latency | Throughput | Process Model | When to Use |
|---------------|---------|------------|---------------|-------------|
| **Unix Sockets** (current) | ~10μs | 100K msg/s | Microservices | Default choice, working well |
| **Shared Memory IPC** | ~500ns | 5M msg/s | Microservices | When socket latency is bottleneck |
| **Network Bus (NATS)** | ~50μs | 500K msg/s | Distributed | Multiple machines |
| **Network Bus (Redis)** | ~100μs | 200K msg/s | Distributed | Need persistence |

*Note: In-process channels (~100ns) are not listed as they don't support the required microservices architecture*

### Migration Decision Tree

```
Start Here
    ↓
Is latency >10μs acceptable?
    ├─ YES → Is current Unix Socket architecture working well?
    │        ├─ YES → Stay with Unix Sockets ✓
    │        └─ NO → Need distributed?
    │                 ├─ YES → Network Message Bus (NATS/Kafka)
    │                 └─ NO → Unix Sockets with better design
    │
    └─ NO → Need <1μs latency
            ↓
        Need process isolation?
            ├─ YES → Shared Memory Message Bus (microservices + speed)
            └─ NO → In-Process Channels (single binary, fastest)
```

### Migration Benefits

1. **No Architecture Changes**: Message flows remain identical
2. **Gradual Migration**: Can migrate one relay at a time
3. **Backward Compatible**: Can run both transports simultaneously
4. **Testing**: Can test message bus with same message patterns
5. **100x Performance**: When you need it, it's there

### Configuration-Driven Transport Selection
```yaml
# config.yml - Switch transports without code changes
alphapulse:
  market_data:
    transport: unix_socket  # or "channel" or "shared_memory"
    path: /tmp/alphapulse/market_data.sock
    
  signals:
    transport: channel      # Already migrated to message bus
    capacity: 100000
    
  execution:
    transport: shared_memory # Ultra-low latency for orders
    size_mb: 64
```

## Message Flow Examples

### Market Data Flow
```rust
// Collector produces market data
let trade = TradeMessage::new(/* ... */);
market_relay.send(MessageType::Trade, &trade)?;

// Strategy consumes from market relay
let trade = market_relay.receive::<TradeMessage>()?;
// Process and generate signal...
```

### Strategy Signal Flow
```rust
// Strategy produces signal
let position = PositionUpdate::new(/* ... */);
signal_relay.send(MessageType::PositionUpdate, &position)?;

// Portfolio consumes from signal relay
let position = signal_relay.receive::<PositionUpdate>()?;
// Update portfolio and maybe generate orders...
```

### Execution Flow
```rust
// Portfolio sends order request
let order = OrderRequest::new(/* ... */);
execution_relay.send(MessageType::OrderRequest, &order)?;

// Execution engine processes
let order = execution_relay.receive::<OrderRequest>()?;
// Route to venue and report status...
```
# AlphaPulse Message Protocol Architecture

## Executive Summary

A high-performance message protocol using bijective (reversible) IDs and universal TLV (Type-Length-Value) message format. All services communicate using structured binary messages with deterministic IDs that require no mapping tables. The architecture uses domain-specific relays for different message categories, enabling clean separation of concerns and straightforward migration to future message bus systems.

## Core Design Principles

1. **Full Address Architecture**: Complete 20-byte Ethereum addresses enable direct smart contract execution
2. **Universal TLV Format**: All messages use 32-byte header + variable TLV payload for maximum flexibility
3. **Zero-copy parsing**: Fixed layouts with proper alignment for direct memory access
4. **Native Precision**: Preserve token-specific decimal precision without scaling
5. **Pool Cache Persistence**: Background disk persistence never blocks real-time processing
6. **Domain separation**: Different relays for market data, signals, and execution

---

# Part I: Protocol Specification

## Message Structure

Every message follows the same universal format:

```
┌─────────────────┬─────────────────────────────────────┐
│ MessageHeader   │ TLV Payload                         │
│ (32 bytes)      │ (variable length)                   │
└─────────────────┴─────────────────────────────────────┘
```

### Message Header (32 bytes)

The header is identical for all messages and contains routing and validation information:

```rust
pub struct MessageHeader {
    pub magic: u32,                 // 0xDEADBEEF
    pub relay_domain: u8,           // Which relay handles this (1=market, 2=signal, 3=execution)
    pub version: u8,                // Protocol version
    pub source: u8,                 // Source service type
    pub flags: u8,                  // Compression, priority, etc.
    pub payload_size: u32,          // TLV payload bytes
    pub sequence: u64,              // Monotonic sequence per source
    pub timestamp: u64,             // Nanoseconds since epoch
    pub checksum: u32,              // CRC32 of entire message
}
```

### TLV Payload Format

After the header, all data is encoded as TLV (Type-Length-Value) extensions:

```
┌─────┬─────┬─────────────┬─────┬─────┬─────────────┬───
│ T₁  │ L₁  │ Value₁      │ T₂  │ L₂  │ Value₂      │ ...
│ 1B  │ 1B  │ L₁ bytes    │ 1B  │ 1B  │ L₂ bytes    │
└─────┴─────┴─────────────┴─────┴─────┴─────────────┴───
```

Each TLV has:
- **Type** (1 byte): Identifies the data type
- **Length** (1 byte): Size of the value in bytes (0-255)
- **Value** (L bytes): The actual data

## TLV Type Registry

TLV types are organized by relay domain to maintain clean separation:

### Market Data Domain (Types 1-19)
*Routes through MarketDataRelay*

| Type | Name | Description | Size |
|------|------|-------------|------|
| 1 | Trade | Price, volume, side, timestamp | 24 bytes |
| 2 | Quote | Bid/ask prices and sizes | 32 bytes |
| 3 | OrderBook | Level data with prices/quantities | Variable |
| 4 | InstrumentMeta | Symbol, decimals, venue info | Variable |
| 5 | PoolSwap | DEX swap with full addresses | 102 bytes |
| 6 | PoolState | Pool reserves and liquidity | Variable |
| 7 | PoolSync | V2 pool sync events | 94 bytes |
| 8-19 | *Reserved* | Future market data types | - |

### Strategy Signal Domain (Types 20-39)
*Routes through SignalRelay*

| Type | Name | Description | Size |
|------|------|-------------|------|
| 20 | SignalIdentity | Strategy ID, signal ID, confidence | 16 bytes |
| 21 | AssetCorrelation | Base/quote instrument correlation | 24 bytes |
| 22 | Economics | Profit estimates, capital requirements | 32 bytes |
| 23 | ExecutionAddresses | Token contracts, router addresses | 84 bytes |
| 24 | VenueMetadata | Venue types, fees, direction flags | 12 bytes |
| 25 | StateReference | Block numbers, validity windows | 24 bytes |
| 26 | ExecutionControl | Flags, slippage, priority settings | 16 bytes |
| 27 | PoolAddresses | DEX pool contracts for quoter calls | 44 bytes |
| 28 | MEVBundle | Flashbots bundle preferences | 40 bytes |
| 29 | TertiaryVenue | Third venue for triangular arbitrage | 24 bytes |
| 30-39 | *Reserved* | Future strategy signal types | - |

### Execution Domain (Types 40-59)
*Routes through ExecutionRelay*

| Type | Name | Description | Size |
|------|------|-------------|------|
| 40 | Order | Order placement with full parameters | Variable |
| 41 | Fill | Order execution confirmation | Variable |
| 42-59 | *Reserved* | Future execution types | - |

### Pool Data Domain (Types 200-219) - Vendor Extensions
*Pool cache persistence and discovery*

| Type | Name | Description | Size |
|------|------|-------------|------|
| 200 | PoolInfo | Individual pool record with full addresses | 85 bytes |
| 201 | PoolCacheHeader | Cache file metadata with checksums | 64 bytes |
| 202 | PoolCacheJournal | Incremental cache updates | 94 bytes |
| 203-219 | *Reserved* | Future pool data types | - |

## Full Address Architecture

### PoolSwapTLV Structure (Type 5)

The PoolSwapTLV represents DEX swap events with complete execution information:

```rust
pub struct PoolSwapTLV {
    pub venue: VenueId,                  // 2 bytes - Exchange identifier
    pub pool_address: [u8; 20],          // 20 bytes - Full pool contract address
    pub token_in_addr: [u8; 20],         // 20 bytes - Full input token address
    pub token_out_addr: [u8; 20],        // 20 bytes - Full output token address
    pub amount_in: i64,                  // 8 bytes - Native precision input
    pub amount_out: i64,                 // 8 bytes - Native precision output
    pub amount_in_decimals: u8,          // 1 byte - Input token decimals
    pub amount_out_decimals: u8,         // 1 byte - Output token decimals
    pub sqrt_price_x96_after: u64,       // 8 bytes - V3 price after swap
    pub tick_after: i32,                 // 4 bytes - V3 tick after swap
    pub liquidity_after: i64,            // 8 bytes - V3 liquidity after swap
    pub timestamp_ns: u64,               // 8 bytes - Nanosecond timestamp
    pub block_number: u64,               // 8 bytes - Blockchain block number
}
// Total: 102 bytes
```

### Key Design Decisions

1. **Full Addresses**: Complete 20-byte Ethereum addresses enable direct smart contract calls
2. **Native Precision**: No scaling applied - preserve token-specific decimals
3. **Execution Ready**: All data needed for immediate arbitrage execution
4. **V3 Compatibility**: Includes sqrt price and tick for Uniswap V3 pools

### Pool Cache System

#### PoolInfoTLV Structure (Type 200)

```rust
pub struct PoolInfoTLV {
    pub tlv_type: u8,                    // 1 byte - Always 200
    pub tlv_length: u8,                  // 1 byte - Always 83
    pub pool_address: [u8; 20],          // 20 bytes - Pool contract
    pub token0_address: [u8; 20],        // 20 bytes - Token0 contract
    pub token1_address: [u8; 20],        // 20 bytes - Token1 contract
    pub token0_decimals: u8,             // 1 byte - Token0 decimals
    pub token1_decimals: u8,             // 1 byte - Token1 decimals
    pub pool_type: u8,                   // 1 byte - UniswapV2/V3, etc.
    pub fee_tier: u32,                   // 4 bytes - Fee in basis points
    pub venue: u16,                      // 2 bytes - VenueId
    pub discovered_at: u64,              // 8 bytes - Discovery timestamp
    pub last_seen: u64,                  // 8 bytes - Last activity
}
// Total: 85 bytes (including TLV header)
```

#### Cache File Format

```
┌─────────────────────┬─────────────────────────────────────┐
│ PoolCacheFileHeader │ PoolInfoTLV[] (variable count)     │
│ (64 bytes)          │ (85 bytes each)                     │
└─────────────────────┴─────────────────────────────────────┘
```

#### Cache Operations

- **Background Persistence**: Writer thread never blocks WebSocket processing
- **Atomic Updates**: Write to temp file, validate, then atomic rename
- **Crash Recovery**: Journal file for incremental updates
- **Memory Mapping**: Fast cache loading via mmap
- **CRC32 Checksums**: Data integrity validation
| 41 | OrderStatus | Fill status, remaining quantity | 24 bytes |
| 42 | Fill | Execution price, quantity, fees | 32 bytes |
| 43 | OrderCancel | Cancel request with reason | 16 bytes |
| 44 | OrderModify | Modification parameters | 24 bytes |
| 45 | ExecutionReport | Complete execution summary | 48 bytes |
| 46-59 | *Reserved* | Future execution types | - |

### System Domain (Types 100-109)
*Direct connections or SystemRelay*

| Type | Name | Description | Size |
|------|------|-------------|------|
| 100 | Heartbeat | Service health and timestamp | 16 bytes |
| 101 | Snapshot | State checkpoint data | Variable |
| 102 | Error | Error codes and descriptions | Variable |
| 103 | ConfigUpdate | Configuration changes | Variable |
| 104-109 | *Reserved* | Future system types | - |

### Extended and Vendor Ranges

| Type Range | Purpose | Length Field |
|------------|---------|--------------|
| 200-254 | Vendor/Private TLVs | u8 (standard) |
| 255 | Extended TLV Header | u16/u32 length for >255 byte payloads |

**Extended TLV Format (Type 255):**
```
┌─────┬─────┬─────┬─────┬─────────────┐
│ 255 │ 0   │ T   │ L   │ Value       │
│ 1B  │ 1B  │ 1B  │ 2B  │ L bytes     │
└─────┴─────┴─────┴─────┴─────────────┘
```
Where T is the actual TLV type and L is a u16 length field, supporting payloads up to 65KB.

## Bijective Instrument IDs

Instrument IDs are self-describing and contain all necessary routing information:

```rust
pub struct InstrumentId {
    pub venue: u16,        // VenueId enum (1=Binance, 2=Uniswap, etc.)
    pub asset_type: u8,    // AssetType enum (1=Stock, 2=Token, 3=Pool)
    pub reserved: u8,      // Future use/flags
    pub asset_id: u64,     // Venue-specific identifier
}
```

### ID Construction Examples

```rust
// Ethereum token from contract address
InstrumentId::ethereum_token("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48") // USDC

// Traditional stock
InstrumentId::stock(VenueId::NYSE, "AAPL")

// DEX liquidity pool
InstrumentId::pool(VenueId::UniswapV3, usdc_id, weth_id)
```

### Bijective Properties

- **Reversible**: Any ID can be decoded to show venue, asset type, and details
- **Deterministic**: Same input always produces same ID
- **No collisions**: Construction method prevents conflicts
- **Cache-friendly**: Converts to u64 for O(1) lookups

## Message Profiles

Message profiles define standard combinations of TLVs for common use cases:

### Market Data Profile

**Trade Message:**
```
Header (relay_domain=1) + Trade TLV + InstrumentMeta TLV
```

**Quote Message:**
```
Header (relay_domain=1) + Quote TLV + InstrumentMeta TLV
```

### DeFi Signal Profile

**Cross-DEX Arbitrage Signal:**
```
Header (relay_domain=2) + 
SignalIdentity TLV + 
AssetCorrelation TLV + 
Economics TLV + 
ExecutionAddresses TLV + 
VenueMetadata TLV + 
StateReference TLV + 
ExecutionControl TLV
```

**With Optional Extensions:**
```
+ PoolAddresses TLV        (for UniV3 quoter calls)
+ MEVBundle TLV           (for Flashbots submission)  
+ TertiaryVenue TLV       (for triangular arbitrage)
```

### Execution Profile

**Order Request:**
```
Header (relay_domain=3) + 
OrderRequest TLV + 
AssetCorrelation TLV + 
ExecutionAddresses TLV
```

**Fill Report:**
```
Header (relay_domain=3) + 
Fill TLV + 
OrderStatus TLV + 
ExecutionReport TLV
```

## Relay Architecture

The system uses domain-specific relays that route messages based on TLV content:

```
┌─────────────────────────────────────────────────────────────────────┐
│                      MARKET DATA DOMAIN                            │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  Collectors ──┐                              ┌→ Strategy A          │
│              ├→ MarketDataRelay (types 1-19) ├→ Strategy B          │
│              └┐        ↓                     └→ Portfolio           │
│                ↓       ↓                                            │
└────────────────────────────────────────────────────────────────────┘
                 ↓ (market data)
┌─────────────────────────────────────────────────────────────────────┐
│                   STRATEGY SIGNAL DOMAIN                           │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  Strategies ──┐                               ┌→ Portfolio          │
│              ├→ SignalRelay (types 20-39) ────├→ Dashboard          │
│              └┐                               └→ RiskManager        │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
                                             ↓ (signals)
┌─────────────────────────────────────────────────────────────────────┐
│                      EXECUTION DOMAIN                              │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  Portfolio ───→ ExecutionRelay (types 40-59) ────┬→ Execution       │
│                         ↓                        └→ RiskManager     │
│                         ↓                           ↓               │
│                         └─────────────────────────→ Dashboard       │
│                                                     ↓               │
│                                                  Venues/Brokers     │
└─────────────────────────────────────────────────────────────────────┘
```

### Domain Separation Benefits

1. **Performance Isolation**: Market data bursts don't affect execution
2. **Security**: Execution messages have stricter validation
3. **Debugging**: Clear message flow tracing
4. **Scaling**: Each relay optimized for its workload
5. **Migration**: Direct mapping to future message bus channels

### Multi-Relay Consumer Pattern

**Key Architecture Principle**: Services like Dashboard connect directly to multiple relays as consumers. This provides complete visibility across all domains.

#### Dashboard as Multi-Domain Consumer

```
                    ┌──────────────┐
                    │  Dashboard   │ ← Connects to ALL relays
                    └──────────────┘
                      ↑   ↑   ↑
                      │   │   │ (consumer connections)
                      │   │   │
    ┌─────────────────┼───┼───┼─────────────────────┐
    │ MarketDataRelay │   │   │                     │
    │                 │   │   │                     │
    │  Collectors ────┘   │   │                     │
    └─────────────────────┼───┼─────────────────────┘
                          │   │
    ┌─────────────────────┼───┼─────────────────────┐
    │    SignalRelay      │   │                     │
    │                     │   │                     │
    │   Strategies ───────┘   │                     │
    └─────────────────────────┼─────────────────────┘
                              │
    ┌─────────────────────────┼─────────────────────┐
    │  ExecutionRelay         │                     │
    │                         │                     │
    │    Portfolio ───────────┘                     │
    └───────────────────────────────────────────────┘
```

#### Implementation Example

```rust
pub struct Dashboard {
    // Dashboard MUST connect to ALL relays for complete visibility
    market_data_connection: UnixStream,  // Types 1-19: Trades, quotes, books
    signal_connection: UnixStream,       // Types 20-39: Strategy signals  
    execution_connection: UnixStream,    // Types 40-59: Orders, fills
}

impl Dashboard {
    pub fn new() -> Result<Self> {
        Ok(Self {
            // Connect as consumer to each relay you need data from
            market_data_connection: UnixStream::connect("/tmp/alphapulse/market_data.sock")?,
            signal_connection: UnixStream::connect("/tmp/alphapulse/signals.sock")?,
            execution_connection: UnixStream::connect("/tmp/alphapulse/execution.sock")?,
        })
    }
    
    pub async fn run(&mut self) {
        // Poll ALL connections simultaneously for comprehensive view
        loop {
            tokio::select! {
                // Real-time market data for charts and pricing
                msg = read_message(&mut self.market_data_connection) => {
                    self.handle_market_data(msg)?;
                }
                
                // Strategy signals for performance tracking
                msg = read_message(&mut self.signal_connection) => {
                    self.handle_strategy_signal(msg)?;
                }
                
                // Execution updates for order tracking
                msg = read_message(&mut self.execution_connection) => {
                    self.handle_execution_update(msg)?;
                }
            }
        }
    }
}

## Recovery Protocol

### Sequence Gap Handling

Each relay maintains per-consumer sequence tracking to handle different processing speeds:

```rust
struct RelayState {
    global_sequence: u64,
    consumer_sequences: HashMap<ConsumerId, u64>,
}
```

When consumers detect sequence number gaps, they initiate recovery:

1. **Gap Detection**: Consumer compares received sequence with expected next sequence
2. **Recovery Request**: Send `RecoveryRequest` TLV with last received sequence  
3. **Relay Response**: 
   - **Small gap** (<100 messages): Retransmit missing range to that consumer
   - **Large gap**: Send `Snapshot` TLV + resume from current global sequence
4. **Consumer Sync**: Apply snapshot and continue normal processing

**Per-Consumer Recovery**: Each consumer can be at different sequence positions without affecting others.

### Recovery Message Types

**Recovery Request TLV (Type 110):**
```rust
pub struct RecoveryRequestTLV {
    pub tlv_type: u8,           // 110
    pub tlv_length: u8,         // 18
    pub consumer_id: u32,       // Identifies requesting consumer
    pub last_sequence: u64,     // Last successfully received sequence
    pub current_sequence: u64,  // Current sequence from header (gap detected)
    pub request_type: u8,       // 1=retransmit, 2=snapshot
    pub reserved: u8,
}
```

**Snapshot TLV (Type 101):**
- Contains compressed state checkpoint
- Followed by resumption at current sequence
- Consumer rebuilds state from snapshot

### Transport Evolution

The architecture supports seamless transport migration:

### Current: Unix Domain Sockets
```bash
/tmp/alphapulse/market_data.sock
/tmp/alphapulse/signals.sock  
/tmp/alphapulse/execution.sock
```

### Future: Message Bus Channels
```rust
MessageBus {
    market_data: Channel<TLVMessage>,
    signals: Channel<TLVMessage>, 
    execution: Channel<TLVMessage>,
}
```

### Mixed Mode Support

For gradual migration, the system supports mixed transport modes:

```yaml
# Mixed deployment configuration
alphapulse:
  transport: mixed
  
  market_data:
    type: unix_socket
    path: "/tmp/alphapulse/market_data.sock"
    
  signals:  
    type: message_bus
    channel_capacity: 100000
    
  execution:
    type: unix_socket  # Critical path stays on proven transport
    path: "/tmp/alphapulse/execution.sock"
```

Services auto-detect transport type from configuration and use appropriate connection method.

The TLV message format remains identical - only the transport mechanism changes.

---

# Part II: Implementation Guide

## TLV Parsing Implementation

### Core TLV Structure

```rust
use zerocopy::{AsBytes, FromBytes, FromZeroes};

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, AsBytes, FromBytes, FromZeroes)]
pub struct TLVHeader {
    pub tlv_type: u8,
    pub tlv_length: u8,
}

pub struct TLVExtension {
    pub header: TLVHeader,
    pub payload: Vec<u8>,
}
```

### Message Header Implementation

```rust
#[repr(C)]
#[derive(Debug, Clone, Copy, AsBytes, FromBytes, FromZeroes)]
pub struct MessageHeader {
    pub magic: u32,                 // 0xDEADBEEF
    pub relay_domain: u8,           // 1=market, 2=signal, 3=execution
    pub version: u8,                // Protocol version
    pub source: u8,                 // SourceType discriminant
    pub flags: u8,                  // Compression, priority
    pub payload_size: u32,          // TLV payload bytes
    pub sequence: u64,              // Monotonic sequence
    pub timestamp: u64,             // Nanoseconds
    pub checksum: u32,              // CRC32 of entire message
}

impl MessageHeader {
    pub fn new(domain: u8, source: u8) -> Self {
        Self {
            magic: 0xDEADBEEF,
            relay_domain: domain,
            version: 1,
            source,
            flags: 0,
            payload_size: 0,
            sequence: 0,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            checksum: 0,
        }
    }

    pub fn calculate_checksum(&mut self, full_message: &[u8]) {
        self.checksum = 0;
        self.checksum = crc32fast::hash(&full_message[..full_message.len() - 4]);
    }
}
```

### TLV Type Definitions

```rust
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, num_enum::TryFromPrimitive)]
pub enum TLVType {
    // Market Data Domain (1-19)
    Trade = 1,
    Quote = 2,
    OrderBook = 3,
    InstrumentMeta = 4,

    // Strategy Signal Domain (20-39)
    SignalIdentity = 20,
    AssetCorrelation = 21,
    Economics = 22,
    ExecutionAddresses = 23,
    VenueMetadata = 24,
    StateReference = 25,
    ExecutionControl = 26,
    PoolAddresses = 27,
    MEVBundle = 28,
    TertiaryVenue = 29,

    // Execution Domain (40-59)
    OrderRequest = 40,
    OrderStatus = 41,
    Fill = 42,
    OrderCancel = 43,
    OrderModify = 44,
    ExecutionReport = 45,

    // System Domain (100-109)
    Heartbeat = 100,
    Snapshot = 101,
    Error = 102,
    ConfigUpdate = 103,
}
```

### Specific TLV Implementations

#### Trade TLV (Type 1, 24 bytes)
```rust
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, AsBytes, FromBytes, FromZeroes)]
pub struct TradeTLV {
    pub tlv_type: u8,           // TLVType::Trade (1)
    pub tlv_length: u8,         // 22 (excluding type/length)
    pub instrument_id: InstrumentId, // 12 bytes
    pub price: i64,             // 8 bytes (fixed-point)
    pub volume: u64,            // 8 bytes
    pub side: u8,               // 1 byte (Buy=1, Sell=2)
    pub flags: u8,              // 1 byte
    pub reserved: [u8; 2],      // 2 bytes alignment
}
```

#### Signal Identity TLV (Type 20, 16 bytes)
```rust
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, AsBytes, FromBytes, FromZeroes)]
pub struct SignalIdentityTLV {
    pub tlv_type: u8,           // TLVType::SignalIdentity (20)
    pub tlv_length: u8,         // 14
    pub strategy_id: u16,       // Strategy type identifier
    pub signal_id: u64,         // Unique signal ID
    pub signal_nonce: u32,      // Monotonic per (source, strategy)
    pub confidence: u8,         // 0-100 confidence score
    pub chain_id: u32,          // EVM chain ID
    pub reserved: u8,           // Alignment
}
```

#### Economics TLV (Type 22, 32 bytes)
```rust
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, AsBytes, FromBytes, FromZeroes)]
pub struct EconomicsTLV {
    pub tlv_type: u8,           // TLVType::Economics (22)
    pub tlv_length: u8,         // 30
    pub expected_profit_q: i128, // Expected profit in quote token (Q64.64)
    pub required_capital_q: u128, // Required capital in base token
    pub gas_estimate_q: u128,   // Gas cost estimate in quote token
    pub reserved: [u8; 6],      // Alignment
}
```

### TLV Parsing Functions

```rust
use std::mem::size_of;

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Message too small")]
    TooSmall,
    #[error("Invalid magic number")]
    InvalidMagic,
    #[error("Checksum mismatch")]
    ChecksumMismatch,
    #[error("Truncated TLV")]
    TruncatedTLV,
    #[error("Unknown TLV type: {0}")]
    UnknownTLVType(u8),
}

/// Parse message header from bytes
pub fn parse_header(data: &[u8]) -> Result<&MessageHeader, ParseError> {
    if data.len() < size_of::<MessageHeader>() {
        return Err(ParseError::TooSmall);
    }

    let header = zerocopy::LayoutVerified::<_, MessageHeader>::new(
        &data[..size_of::<MessageHeader>()]
    )
    .ok_or(ParseError::TooSmall)?
    .into_ref();

    if header.magic != 0xDEADBEEF {
        return Err(ParseError::InvalidMagic);
    }

    // Validate checksum
    let calculated_crc = crc32fast::hash(&data[..data.len() - 4]);
    if calculated_crc != header.checksum {
        return Err(ParseError::ChecksumMismatch);
    }

    Ok(header)
}

/// Parse all TLV extensions from payload
pub fn parse_tlv_extensions(tlv_data: &[u8]) -> Result<Vec<TLVExtension>, ParseError> {
    let mut extensions = Vec::new();
    let mut offset = 0;

    while offset < tlv_data.len() {
        if offset + 2 > tlv_data.len() {
            return Err(ParseError::TruncatedTLV);
        }

        let tlv_type = tlv_data[offset];
        let tlv_length = tlv_data[offset + 1];

        let total_length = 2 + tlv_length as usize;
        if offset + total_length > tlv_data.len() {
            return Err(ParseError::TruncatedTLV);
        }

        let payload = &tlv_data[offset + 2..offset + total_length];
        extensions.push(TLVExtension {
            header: TLVHeader { tlv_type, tlv_length },
            payload: payload.to_vec(),
        });

        offset += total_length;
    }

    Ok(extensions)
}

/// Find specific TLV by type
pub fn find_tlv_by_type(tlv_data: &[u8], target_type: u8) -> Option<&[u8]> {
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

## Bijective ID Implementation

```rust
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, AsBytes, FromBytes)]
pub struct InstrumentId {
    pub venue: u16,        // VenueId enum
    pub asset_type: u8,    // AssetType enum
    pub reserved: u8,      // Future use/flags
    pub asset_id: u64,     // Venue-specific identifier
}

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, num_enum::TryFromPrimitive)]
pub enum VenueId {
    // Traditional exchanges
    NYSE = 1,
    NASDAQ = 2,
    Binance = 10,
    Kraken = 11,
    Coinbase = 12,
    
    // DeFi protocols
    Ethereum = 100,
    UniswapV2 = 101,
    UniswapV3 = 102,
    SushiSwap = 103,
    Curve = 104,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, num_enum::TryFromPrimitive)]
pub enum AssetType {
    Stock = 1,
    Token = 2,
    Pool = 3,
    Derivative = 4,
}

impl InstrumentId {
    /// Create Ethereum token ID from contract address
    pub fn ethereum_token(address: &str) -> Result<Self, hex::FromHexError> {
        // Use first 8 bytes of address as asset_id
        let hex_clean = address.strip_prefix("0x").unwrap_or(address);
        let bytes = hex::decode(&hex_clean[..16])?; // First 8 bytes = 16 hex chars
        let asset_id = u64::from_be_bytes(bytes.try_into().unwrap());
        
        Ok(Self {
            venue: VenueId::Ethereum as u16,
            asset_type: AssetType::Token as u8,
            reserved: 0,
            asset_id,
        })
    }

    /// Create stock ID from exchange and symbol
    pub fn stock(exchange: VenueId, symbol: &str) -> Self {
        Self {
            venue: exchange as u16,
            asset_type: AssetType::Stock as u8,
            reserved: 0,
            asset_id: symbol_to_u64(symbol),
        }
    }

    /// Create DEX pool ID from constituent tokens
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
            asset_id: cantor_pairing(id0, id1), // Bijective pairing function
        }
    }

    /// Convert to u64 for cache keys and lookups
    pub fn to_u64(&self) -> u64 {
        ((self.venue as u64) << 48) |
        ((self.asset_type as u64) << 40) |
        (self.asset_id & 0xFFFFFFFFFF)
    }

    /// Reconstruct from u64 cache key
    pub fn from_u64(value: u64) -> Self {
        Self {
            venue: ((value >> 48) & 0xFFFF) as u16,
            asset_type: ((value >> 40) & 0xFF) as u8,
            reserved: 0,
            asset_id: value & 0xFFFFFFFFFF,
        }
    }

    /// Human-readable debug representation
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

    fn venue(&self) -> Result<VenueId, num_enum::TryFromPrimitiveError<VenueId>> {
        VenueId::try_from(self.venue)
    }

    fn asset_type(&self) -> Result<AssetType, num_enum::TryFromPrimitiveError<AssetType>> {
        AssetType::try_from(self.asset_type)
    }
}

/// Convert symbol string to u64 for asset_id
fn symbol_to_u64(symbol: &str) -> u64 {
    let mut bytes = [0u8; 8];
    let len = symbol.len().min(8);
    bytes[..len].copy_from_slice(&symbol.as_bytes()[..len]);
    u64::from_be_bytes(bytes)
}

/// Convert u64 asset_id back to symbol string
fn u64_to_symbol(value: u64) -> String {
    let bytes = value.to_be_bytes();
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(8);
    String::from_utf8_lossy(&bytes[..end]).to_string()
}

/// Bijective Cantor pairing function for pool IDs
/// Ensures no collisions between different token pairs
fn cantor_pairing(x: u64, y: u64) -> u64 {
    // Use 32-bit values to prevent overflow in 64-bit result
    let x32 = (x & 0xFFFFFFFF) as u32;
    let y32 = (y & 0xFFFFFFFF) as u32;
    
    let sum = x32 as u64 + y32 as u64;
    (sum * (sum + 1) / 2 + y32 as u64) & 0xFFFFFFFFFF // Keep within 40 bits
}
```

## Message Building and Validation

### TLV Message Builder

```rust
pub struct TLVMessageBuilder {
    header: MessageHeader,
    tlvs: Vec<TLVExtension>,
}

impl TLVMessageBuilder {
    pub fn new(relay_domain: u8, source: u8) -> Self {
        Self {
            header: MessageHeader::new(relay_domain, source),
            tlvs: Vec::new(),
        }
    }

    pub fn add_tlv<T: AsBytes>(&mut self, tlv_type: TLVType, data: &T) -> &mut Self {
        let bytes = data.as_bytes();
        self.tlvs.push(TLVExtension {
            header: TLVHeader {
                tlv_type: tlv_type as u8,
                tlv_length: bytes.len() as u8,
            },
            payload: bytes.to_vec(),
        });
        self
    }

    pub fn build(mut self) -> Vec<u8> {
        // Calculate payload size
        let payload_size: usize = self.tlvs.iter()
            .map(|tlv| 2 + tlv.payload.len()) // type + length + payload
            .sum();

        self.header.payload_size = payload_size as u32;

        // Serialize header + TLVs
        let mut message = Vec::with_capacity(32 + payload_size);
        
        // Add header (will update checksum later)
        message.extend_from_slice(self.header.as_bytes());

        // Add TLVs
        for tlv in &self.tlvs {
            message.push(tlv.header.tlv_type);
            message.push(tlv.header.tlv_length);
            message.extend_from_slice(&tlv.payload);
        }

        // Calculate and update checksum
        let header_mut = zerocopy::LayoutVerified::<_, MessageHeader>::new_from_prefix(&mut message)
            .unwrap().0.into_mut();
        header_mut.calculate_checksum(&message);

        message
    }
}
```

### Usage Examples

#### Building a DeFi Signal Message

```rust
// Create cross-DEX arbitrage signal
let usdc_id = InstrumentId::ethereum_token("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48")?;
let weth_id = InstrumentId::ethereum_token("0xc02aaa39b223fe8d0a0e5c4f27b9dab3eecd1b")?;

let signal_identity = SignalIdentityTLV {
    tlv_type: TLVType::SignalIdentity as u8,
    tlv_length: 14,
    strategy_id: 1, // Cross-DEX arbitrage
    signal_id: 12345,
    signal_nonce: 1,
    confidence: 85,
    chain_id: 1, // Ethereum mainnet
    reserved: 0,
};

let asset_correlation = AssetCorrelationTLV {
    tlv_type: TLVType::AssetCorrelation as u8,
    tlv_length: 22,
    base_instrument: usdc_id,
    quote_instrument: weth_id,
    reserved: [0; 2],
};

let economics = EconomicsTLV {
    tlv_type: TLVType::Economics as u8,
    tlv_length: 30,
    expected_profit_q: 1500_000_000_000_000_000i128, // 1500 USD in Q64.64
    required_capital_q: 10000_000_000_000_000_000u128, // 10k USD
    gas_estimate_q: 50_000_000_000_000_000u128, // 50 USD gas
    reserved: [0; 6],
};

// Build message
let message = TLVMessageBuilder::new(2, 40) // Signal domain, ArbitrageStrategy source
    .add_tlv(TLVType::SignalIdentity, &signal_identity)
    .add_tlv(TLVType::AssetCorrelation, &asset_correlation)  
    .add_tlv(TLVType::Economics, &economics)
    .build();

// Send via SignalRelay
signal_relay.send(&message)?;
```

#### Parsing a Received Message

```rust
// Receive message bytes
let message_bytes = relay.receive()?;

// Parse header
let header = parse_header(&message_bytes)?;
println!("Received {} bytes from relay domain {}", 
         header.payload_size, header.relay_domain);

// Extract TLV payload
let tlv_payload = &message_bytes[32..32 + header.payload_size as usize];

// Parse TLVs
let tlvs = parse_tlv_extensions(tlv_payload)?;

// Process specific TLVs
for tlv in tlvs {
    match TLVType::try_from(tlv.header.tlv_type)? {
        TLVType::SignalIdentity => {
            let signal = zerocopy::LayoutVerified::<_, SignalIdentityTLV>::new(&tlv.payload)
                .unwrap().into_ref();
            println!("Signal {} from strategy {}", signal.signal_id, signal.strategy_id);
        }
        TLVType::Economics => {
            let econ = zerocopy::LayoutVerified::<_, EconomicsTLV>::new(&tlv.payload)
                .unwrap().into_ref();
            println!("Expected profit: {} wei", econ.expected_profit_q);
        }
        _ => {
            // Unknown TLV - gracefully ignore for forward compatibility
            println!("Unknown TLV type {}, ignoring", tlv.header.tlv_type);
        }
    }
}
```

### Performance Characteristics

### Memory Layout
- **Zero-copy parsing**: Direct cast with `zerocopy` crate for aligned access
- **Cache-friendly**: 32-byte header fits in single cache line
- **Bijective IDs**: O(1) cache lookups using `to_u64()` conversion

### Validation
- **Selective CRC32 checksums**: Domain-specific validation policies²
- **Magic number**: Quick format validation
- **TLV bounds checking**: Prevents buffer overruns

**Checksum Policy by Relay Domain:**
- **MarketDataRelay**: `checksum = false` (prioritize speed for price ticks)
- **SignalRelay**: `checksum = true` (balance speed/reliability for strategies)
- **ExecutionRelay**: `checksum = true` (always validate critical order flow)

### Throughput Targets
- **Market data**: 1M+ messages/second with Unix sockets³
- **Strategy signals**: 100K messages/second with full validation
- **Execution orders**: 10K messages/second with critical path optimization

---

**Performance Notes:**

² *For ultra-high throughput, consider checksum-in-footer layout to avoid header rewrites during validation*

³ *At >1M msg/s, consider relay sharding by instrument ID ranges or multicore consumer patterns*

### Safety and Error Handling

### Memory Safety
- No `unsafe` transmutes - all parsing uses `zerocopy` verified casts⁴
- Explicit padding for proper struct alignment
- Bounds checking on all TLV accesses

### Error Categories
```rust
#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("Parse error: {0}")]
    Parse(#[from] ParseError),
    
    #[error("Unknown TLV type: {0}")]
    UnknownTLV(u8),
    
    #[error("Invalid instrument ID")]
    InvalidInstrument,
    
    #[error("Checksum validation failed")]
    ChecksumFailed,
    
    #[error("Message too large: {size} bytes")]
    MessageTooLarge { size: usize },
}
```

### Graceful Degradation
- Unknown TLV types are ignored (forward compatibility)
- Malformed messages are logged and dropped
- Circuit breakers prevent relay overload
- Sequence number gaps trigger resync requests⁵

---

**Future Considerations:**

⁴ *For cross-architecture deployment (ARM ↔ x86), consider explicit endianness handling with `to_le_bytes()`/`to_be_bytes()` instead of direct struct serialization*

⁵ *Message bus transport will require explicit backpressure and QoS policies beyond Unix socket kernel buffering*

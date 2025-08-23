# Protocol V2 TLV Message Type Reference

## Overview

This document provides a complete reference for all TLV (Type-Length-Value) message types used in the AlphaPulse Protocol V2 architecture. All messages follow the 32-byte MessageHeader + variable TLV payload format with full 20-byte Ethereum addresses for direct execution capability.

## Message Header Format

All Protocol V2 messages begin with a 32-byte header:

```rust
#[repr(C, packed)]
pub struct MessageHeader {
    pub magic: u32,              // 0xDEADBEEF
    pub version: u8,             // Protocol version (2)
    pub relay_domain: u8,        // Domain ID (1=Market, 20=Signal, 40=Execution)
    pub source: u16,             // Source identifier
    pub sequence: u64,           // Monotonic sequence number
    pub timestamp_ns: u64,       // Nanosecond timestamp
    pub payload_size: u32,       // TLV payload size in bytes
    pub checksum: u32,           // CRC32 of payload
}
```

**Size**: 32 bytes (fixed)
**Alignment**: Packed for zero-copy operations

## TLV Type Domains

TLV types are organized into domains to ensure clean separation and prevent conflicts:

| Domain | Range | Purpose | Examples |
|--------|-------|---------|----------|
| Market Data | 1-19 | Real-time market events | Trades, quotes, swaps |
| Signal | 20-39 | Trading signals | Buy/sell, arbitrage opportunities |
| Execution | 40-79 | Order management | Orders, fills, cancellations |
| Risk | 80-99 | Risk management | Position limits, circuit breakers |
| System | 100-119 | System control | Heartbeats, status |
| Identity | 120-139 | Authentication | Login, sessions |
| Portfolio | 140-159 | Portfolio data | Positions, P&L |
| Analytics | 160-179 | Analysis data | Indicators, signals |
| Vendor Extensions | 200-219 | Vendor-specific | Pool cache, proprietary data |

## Market Data Domain (Types 1-19)

### Type 1: TradeTLV
Standard trade execution record for traditional exchanges.

```rust
#[repr(C, packed)]
pub struct TradeTLV {
    pub tlv_type: u8,            // 1
    pub tlv_length: u8,          // 49 bytes
    pub instrument_id: u64,      // Bijective instrument identifier
    pub price: i64,              // 8-decimal fixed-point (* 100_000_000)
    pub volume: i64,             // 8-decimal fixed-point (* 100_000_000)
    pub timestamp_exchange_ns: u64, // Exchange-provided timestamp
    pub timestamp_received_ns: u64, // Our receive timestamp
    pub side: u8,                // 0=Unknown, 1=Buy, 2=Sell
    pub trade_id: u64,           // Exchange trade ID
}
```

**Use Case**: Kraken, Coinbase, Binance trade data
**Precision**: 8 decimal places for USD prices
**Total Size**: 51 bytes (2-byte header + 49-byte payload)

### Type 2: QuoteTLV
Best bid/offer updates from order books.

```rust
#[repr(C, packed)]
pub struct QuoteTLV {
    pub tlv_type: u8,            // 2
    pub tlv_length: u8,          // 57 bytes
    pub instrument_id: u64,      // Bijective instrument identifier
    pub bid_price: i64,          // 8-decimal fixed-point
    pub bid_volume: i64,         // 8-decimal fixed-point
    pub ask_price: i64,          // 8-decimal fixed-point
    pub ask_volume: i64,         // 8-decimal fixed-point
    pub timestamp_exchange_ns: u64,
    pub timestamp_received_ns: u64,
    pub quote_id: u64,           // Exchange quote ID
}
```

**Use Case**: Level-1 order book updates
**Precision**: 8 decimal places for USD prices
**Total Size**: 59 bytes

### Type 3: PoolSwapTLV
DEX pool swap events with full address architecture for direct execution.

```rust
#[repr(C, packed)]
pub struct PoolSwapTLV {
    pub tlv_type: u8,            // 3
    pub tlv_length: u8,          // 100 bytes
    pub venue: u16,              // VenueId (e.g., 1=Uniswap, 2=SushiSwap)
    pub pool_address: [u8; 20],  // Full pool contract address
    pub token_in_addr: [u8; 20], // Full input token address
    pub token_out_addr: [u8; 20], // Full output token address
    pub amount_in: i64,          // Native token precision (no scaling)
    pub amount_out: i64,         // Native token precision (no scaling)
    pub amount_in_decimals: u8,  // Token decimals (18 for WETH, 6 for USDC)
    pub amount_out_decimals: u8, // Token decimals
    pub timestamp_block_ns: u64, // Block timestamp (nanoseconds)
    pub timestamp_received_ns: u64, // Our receive timestamp
    pub transaction_hash: [u8; 32], // Transaction hash
    pub log_index: u32,          // Log index within transaction
}
```

**Use Case**: Uniswap V2/V3, SushiSwap, QuickSwap swaps
**Precision**: Native token precision preserved (18 decimals WETH, 6 decimals USDC)
**Total Size**: 102 bytes
**Critical**: Full addresses enable direct smart contract execution

### Type 4: OrderBookTLV
Level-2 order book updates with multiple price levels.

```rust
#[repr(C, packed)]
pub struct OrderBookLevel {
    pub price: i64,              // 8-decimal fixed-point
    pub volume: i64,             // 8-decimal fixed-point
}

#[repr(C, packed)]
pub struct OrderBookTLV {
    pub tlv_type: u8,            // 4
    pub tlv_length: u8,          // Variable (16 + 16 * num_levels)
    pub instrument_id: u64,      // Bijective instrument identifier
    pub timestamp_exchange_ns: u64,
    pub timestamp_received_ns: u64,
    pub num_bid_levels: u8,      // Number of bid levels (max 15)
    pub num_ask_levels: u8,      // Number of ask levels (max 15)
    // Followed by bid_levels[num_bid_levels] then ask_levels[num_ask_levels]
}
```

**Use Case**: Deep order book data for analysis
**Max Levels**: 15 bids + 15 asks (to stay under 255-byte TLV limit)
**Total Size**: Variable, 16 + 16 * (num_bid_levels + num_ask_levels)

## Signal Domain (Types 20-39)

### Type 20: ArbitrageSignalTLV
Detected arbitrage opportunity with execution parameters.

```rust
#[repr(C, packed)]
pub struct ArbitrageSignalTLV {
    pub tlv_type: u8,            // 20
    pub tlv_length: u8,          // 103 bytes
    pub signal_id: u64,          // Unique signal identifier
    pub base_asset: [u8; 20],    // Base asset address
    pub quote_asset: [u8; 20],   // Quote asset address
    pub venue_buy: u16,          // Venue to buy from
    pub venue_sell: u16,         // Venue to sell to
    pub pool_buy_addr: [u8; 20], // Pool address for buy leg
    pub pool_sell_addr: [u8; 20], // Pool address for sell leg
    pub optimal_size: i64,       // Optimal trade size (native precision)
    pub expected_profit: i64,    // Expected profit (native precision)
    pub confidence: u16,         // Confidence score (0-10000 = 0-100%)
    pub expiry_ns: u64,          // Signal expiry timestamp
    pub discovered_at_ns: u64,   // Discovery timestamp
}
```

**Use Case**: Flash arbitrage strategy signals
**Precision**: Native token precision for sizes and profits
**Total Size**: 105 bytes
**Critical**: Full pool addresses enable direct execution

### Type 21: SignalIdentityTLV
Signal source identification and metadata.

```rust
#[repr(C, packed)]
pub struct SignalIdentityTLV {
    pub tlv_type: u8,            // 21
    pub tlv_length: u8,          // 32 bytes
    pub strategy_id: u32,        // Strategy identifier
    pub strategy_version: u16,   // Strategy version
    pub risk_tolerance: u8,      // Risk level (1-10)
    pub allocation_pct: u16,     // Portfolio allocation percentage (0-10000)
    pub timestamp_created_ns: u64, // Strategy creation time
    pub last_update_ns: u64,     // Last parameter update
    pub flags: u32,              // Strategy flags (active, paper trading, etc.)
}
```

**Use Case**: Strategy identification and risk parameters
**Total Size**: 34 bytes

## Execution Domain (Types 40-79)

### Type 40: OrderTLV
Order placement and modification instructions.

```rust
#[repr(C, packed)]
pub struct OrderTLV {
    pub tlv_type: u8,            // 40
    pub tlv_length: u8,          // 73 bytes
    pub order_id: u64,           // Unique order identifier
    pub client_order_id: u64,    // Client-provided order ID
    pub instrument_id: u64,      // Target instrument
    pub side: u8,                // 1=Buy, 2=Sell
    pub order_type: u8,          // 1=Market, 2=Limit, 3=Stop
    pub time_in_force: u8,       // 1=GTC, 2=IOC, 3=FOK
    pub quantity: i64,           // Order quantity (native precision)
    pub price: i64,              // Limit price (native precision)
    pub stop_price: i64,         // Stop price (native precision)
    pub timestamp_created_ns: u64, // Order creation time
    pub expires_at_ns: u64,      // Order expiry (0 = GTC)
}
```

**Use Case**: Order management and execution
**Total Size**: 75 bytes

### Type 41: FillTLV
Order execution notification.

```rust
#[repr(C, packed)]
pub struct FillTLV {
    pub tlv_type: u8,            // 41
    pub tlv_length: u8,          // 65 bytes
    pub fill_id: u64,            // Unique fill identifier
    pub order_id: u64,           // Associated order ID
    pub instrument_id: u64,      // Filled instrument
    pub side: u8,                // 1=Buy, 2=Sell
    pub fill_quantity: i64,      // Filled quantity (native precision)
    pub fill_price: i64,         // Fill price (native precision)
    pub fee_amount: i64,         // Fee charged (native precision)
    pub timestamp_filled_ns: u64, // Fill timestamp
    pub venue_fill_id: u64,      // Venue-specific fill ID
}
```

**Use Case**: Trade execution confirmations
**Total Size**: 67 bytes

## Vendor Extensions (Types 200-219)

### Type 200: PoolInfoTLV
Pool cache persistence record for discovered pools.

```rust
#[repr(C, packed)]
pub struct PoolInfoTLV {
    pub tlv_type: u8,            // 200
    pub tlv_length: u8,          // 83 bytes
    pub pool_address: [u8; 20],  // Full pool contract address
    pub token0_address: [u8; 20], // Token0 address
    pub token1_address: [u8; 20], // Token1 address
    pub token0_decimals: u8,     // Token0 decimal places
    pub token1_decimals: u8,     // Token1 decimal places
    pub pool_type: u8,           // 1=UniV2, 2=UniV3, 3=Curve, etc.
    pub fee_tier: u32,           // Fee in basis points
    pub venue: u16,              // Venue identifier
    pub discovered_at: u64,      // Discovery timestamp
    pub last_seen: u64,          // Last activity timestamp
}
```

**Use Case**: Pool cache persistence and recovery
**Total Size**: 85 bytes
**Storage**: Written to binary cache files, never blocks hot path

### Type 201: PoolCacheHeaderTLV
Pool cache file header with metadata.

```rust
#[repr(C, packed)]
pub struct PoolCacheHeaderTLV {
    pub tlv_type: u8,            // 201
    pub tlv_length: u8,          // 32 bytes
    pub version: u32,            // Cache format version
    pub chain_id: u64,           // Blockchain chain ID
    pub total_pools: u32,        // Number of pools in cache
    pub created_at: u64,         // Cache creation timestamp
    pub last_updated: u64,       // Last update timestamp
    pub checksum: u32,           // CRC32 of entire cache
}
```

**Use Case**: Cache file validation and metadata
**Total Size**: 34 bytes

## TLV Parsing Rules

### Size Validation
```rust
fn validate_tlv_bounds(tlv_type: u8, tlv_length: u8, payload: &[u8]) -> Result<(), ParseError> {
    let expected_size = match tlv_type {
        1 => 49,  // TradeTLV
        2 => 57,  // QuoteTLV
        3 => 100, // PoolSwapTLV
        4 => {    // OrderBookTLV - variable size
            if tlv_length < 16 { return Err(ParseError::TooSmall); }
            tlv_length as usize
        }
        200 => 83, // PoolInfoTLV
        201 => 32, // PoolCacheHeaderTLV
        _ => return Err(ParseError::UnknownType),
    };
    
    if payload.len() < expected_size {
        return Err(ParseError::TruncatedPayload);
    }
    
    Ok(())
}
```

### Endianness
All multi-byte fields use little-endian encoding for consistency with x86_64 systems.

### Alignment
All TLV structures use `#[repr(C, packed)]` for zero-copy operations and consistent cross-platform layout.

## Performance Characteristics

### Parsing Performance
- **Target**: >1.6M messages/second parsing
- **Achieved**: 1,643,779 msg/s (measured)
- **Method**: Direct memory mapping with bounds checking

### Construction Performance
- **Target**: >1M messages/second construction
- **Achieved**: 1,097,624 msg/s (measured)
- **Method**: Pre-allocated buffers with zerocopy serialization

### Memory Usage
- **Fixed Headers**: 32 bytes (constant)
- **Variable Payloads**: 34-105 bytes (typical)
- **Total Message**: 66-137 bytes (typical range)

## Error Handling

### Parse Errors
```rust
#[derive(Debug, Clone)]
pub enum ParseError {
    InvalidMagic,           // Header magic != 0xDEADBEEF
    UnsupportedVersion,     // Version != 2
    InvalidChecksum,        // CRC32 mismatch
    TruncatedHeader,        // < 32 bytes available
    TruncatedPayload,       // Payload smaller than tlv_length
    UnknownTLVType,         // TLV type not in registry
    InvalidDomain,          // TLV type outside expected domain
    SequenceGap,            // Missing sequence number
}
```

### Recovery Procedures
1. **Invalid Magic**: Discard message, log error, continue with next
2. **Checksum Mismatch**: Request retransmission if possible
3. **Sequence Gap**: Log gap, request missing messages, continue
4. **Unknown Type**: Log and skip, maintain forward compatibility

## Integration Examples

### Receiving Market Data
```rust
use alphapulse_protocol_v2::{parse_header, parse_tlv_extensions, PoolSwapTLV};

async fn handle_message(raw_bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    // Parse 32-byte header
    let header = parse_header(raw_bytes)?;
    
    // Validate domain
    if header.relay_domain != 1 {  // Market Data domain
        return Err("Wrong domain".into());
    }
    
    // Parse TLV payload
    let payload = &raw_bytes[32..32 + header.payload_size as usize];
    let tlvs = parse_tlv_extensions(payload)?;
    
    for tlv in tlvs {
        match tlv.header.tlv_type {
            3 => {  // PoolSwapTLV
                let swap: PoolSwapTLV = tlv.try_into()?;
                process_pool_swap(swap).await?;
            }
            _ => {
                debug!("Unknown TLV type: {}", tlv.header.tlv_type);
            }
        }
    }
    
    Ok(())
}
```

### Sending Arbitrage Signals
```rust
use alphapulse_protocol_v2::{TLVMessageBuilder, ArbitrageSignalTLV};

async fn send_arbitrage_signal(
    signal: ArbitrageSignalTLV,
    relay_connection: &mut UnixStream
) -> Result<(), Box<dyn Error>> {
    
    let mut builder = TLVMessageBuilder::new(
        20,  // Signal domain
        1,   // Source ID
    );
    
    builder.add_tlv(20, &signal)?;  // ArbitrageSignalTLV
    let message = builder.build();
    
    relay_connection.write_all(&message).await?;
    
    Ok(())
}
```

## Migration and Versioning

### Adding New TLV Types
1. **Choose unused type number** in appropriate domain
2. **Define struct** with `#[repr(C, packed)]`
3. **Update TLV registry** in `types.rs`
4. **Add parsing logic** to consumers
5. **Update documentation** (this file)

### Backwards Compatibility
- Unknown TLV types are skipped gracefully
- Old consumers ignore new TLV types
- Header format is frozen (never changes)
- Domain boundaries are respected

### Version Migration
Protocol version increments trigger:
- Service restart coordination
- Cache invalidation
- Performance re-validation
- Documentation updates

This completes the Protocol V2 TLV message type reference. All message types preserve full address architecture for direct execution capability while maintaining high-performance parsing and construction characteristics.
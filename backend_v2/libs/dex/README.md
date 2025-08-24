# AlphaPulse DEX Library

A shared library for DEX (Decentralized Exchange) functionality, providing canonical ABI definitions and event decoding for various DEX protocols.

## Overview

This library consolidates DEX-related functionality that was previously scattered across multiple services, implementing the "One Canonical Source" principle. It provides type-safe event decoding, semantic validation, and protocol detection for major DEX protocols.

## Architecture

```text
libs/dex/
├── src/
│   ├── abi/
│   │   ├── events.rs      # Event structures and decoders
│   │   ├── uniswap_v2.rs  # V2 protocol ABIs
│   │   └── uniswap_v3.rs  # V3 protocol ABIs
│   └── lib.rs             # Public API
├── tests/
│   └── integration_tests.rs # Comprehensive test suite
└── Cargo.toml
```

## Supported Protocols

- **Uniswap V2**: Original AMM protocol with constant product formula
- **Uniswap V3**: Concentrated liquidity with tick-based pricing
- **SushiSwap V2**: Uniswap V2 compatible fork
- **QuickSwap V2/V3**: Polygon-native DEX protocols

## Features

### 🔍 **Protocol Detection**
Automatic detection of DEX protocol based on log structure and pool addresses:

```rust
use alphapulse_dex::abi::detect_dex_protocol;

let protocol = detect_dex_protocol(&pool_address, &log);
match protocol {
    DEXProtocol::UniswapV3 => println!("V3 pool detected"),
    DEXProtocol::UniswapV2 => println!("V2 pool detected"),
    _ => println!("Other protocol"),
}
```

### 📊 **Event Decoding**
Type-safe decoding with semantic validation:

```rust
use alphapulse_dex::abi::{SwapEventDecoder, DEXProtocol};

// Decode swap event based on protocol
let swap = SwapEventDecoder::decode_swap_event(&log, DEXProtocol::UniswapV3)?;

println!("Amount in: {}", swap.amount_in);
println!("Amount out: {}", swap.amount_out);
println!("Token0 -> Token1: {}", swap.token_in_is_token0);
```

### 🛡️ **Overflow Protection**
Safe handling of large blockchain values:

```rust
// Automatically handles overflow with warning
let safe_amount = SwapEventDecoder::safe_u256_to_i64(large_value)?;
```

### ⚡ **Zero-Copy Operations**
Efficient parsing using ethabi with minimal allocations.

## Usage

### Basic Event Decoding

```rust
use alphapulse_dex::abi::*;

// Detect protocol
let protocol = detect_dex_protocol(&log.address, &log);

// Decode swap event
let swap = SwapEventDecoder::decode_swap_event(&log, protocol)?;

// Extract validated data
println!("Pool: {:x?}", swap.pool_address);
println!("Trade: {} -> {}", swap.amount_in, swap.amount_out);
```

### Mint/Burn Events

```rust
// Decode liquidity provision
let mint = MintEventDecoder::decode_mint_event(&log, protocol)?;
let burn = BurnEventDecoder::decode_burn_event(&log, protocol)?;

// Access tick ranges (V3 only)
if protocol == DEXProtocol::UniswapV3 {
    println!("Tick range: {} to {}", mint.tick_lower, mint.tick_upper);
}
```

### Error Handling

```rust
match SwapEventDecoder::decode_swap_event(&log, protocol) {
    Ok(swap) => process_swap(swap),
    Err(DecodingError::MissingField(field)) => {
        eprintln!("Missing required field: {}", field);
    }
    Err(DecodingError::ValueOverflow { value }) => {
        eprintln!("Value too large: {}", value);
    }
    Err(DecodingError::UnsupportedProtocol(proto)) => {
        eprintln!("Protocol {:?} not supported", proto);
    }
    Err(e) => eprintln!("Other error: {}", e),
}
```

## Data Structures

### ValidatedSwap
```rust
pub struct ValidatedSwap {
    pub pool_address: [u8; 20],      // Full pool contract address
    pub amount_in: i64,              // Input amount (native precision)
    pub amount_out: i64,             // Output amount (native precision)
    pub token_in_is_token0: bool,    // Trade direction
    pub sqrt_price_x96_after: u128,  // V3: Price after swap
    pub tick_after: i32,             // V3: Tick after swap
    pub liquidity_after: u128,       // V3: Liquidity after swap
    pub dex_protocol: DEXProtocol,   // Protocol used
}
```

### ValidatedMint/Burn
Similar structures for liquidity provision events, with tick ranges for V3 protocols.

## Migration Guide

### From Scattered ABI Code

**Before:** Multiple ABI definitions in different services
```rust
// OLD: services_v2/adapters/src/input/collectors/abi_events.rs
use crate::input::collectors::abi_events::SwapEventDecoder;
```

**After:** Single shared library
```rust
// NEW: Use shared library
use alphapulse_dex::abi::SwapEventDecoder;
```

### Migration Steps

1. **Update Cargo.toml** - Add dependency:
   ```toml
   alphapulse-dex = { workspace = true }
   ```

2. **Update imports** - Replace local imports:
   ```rust
   // Replace local ABI imports with shared library
   use alphapulse_dex::abi::*;
   ```

3. **Remove duplicate files** - Delete old ABI definitions
4. **Update tests** - Use new shared types and functions

## Performance Characteristics

- **Protocol Detection**: ~100ns per call (simple pattern matching)
- **ABI Construction**: ~1μs per event type (cached static data)
- **Event Decoding**: ~10-50μs depending on protocol complexity
- **Memory Usage**: Minimal allocations, zero-copy where possible

## Testing

Comprehensive test suite covers:

### Unit Tests
- ABI structure validation
- Protocol detection accuracy
- Overflow handling
- Error scenarios

### Integration Tests
- Real-world event decoding
- Cross-protocol compatibility
- Performance benchmarks

### Run Tests
```bash
# Unit tests
cargo test --package alphapulse-dex

# Integration tests
cargo test --package alphapulse-dex --test integration_tests

# Benchmarks
cargo test --package alphapulse-dex benchmarks --release
```

## Development

### Adding New Protocols

1. **Add Protocol Variant**:
   ```rust
   // In src/abi/mod.rs
   pub enum DEXProtocol {
       // ... existing
       NewProtocolV2,
   }
   ```

2. **Create ABI Module**:
   ```rust
   // src/abi/new_protocol.rs
   pub fn swap_event() -> Event { /* ... */ }
   ```

3. **Update Detection Logic**:
   ```rust
   // In detect_dex_protocol()
   if addr_matches_new_protocol(&pool_address) {
       return DEXProtocol::NewProtocolV2;
   }
   ```

4. **Add Decoder Support**:
   ```rust
   // In SwapEventDecoder::decode_swap_event()
   DEXProtocol::NewProtocolV2 => {
       Self::decode_new_protocol_swap(pool_address, raw_log, protocol)
   }
   ```

5. **Add Tests** - Comprehensive test coverage for new protocol

### Contributing Guidelines

- **No Breaking Changes Without Migration Path**: Maintain backward compatibility
- **Comprehensive Testing**: All new functionality must have tests
- **Performance Conscious**: Benchmark performance-critical paths
- **Documentation**: Update this README for any API changes
- **Follow AlphaPulse Patterns**: Consistent error handling, naming conventions

## Future Enhancements

### Planned Features
- [ ] Support for more DEX protocols (Balancer, Curve)
- [ ] Advanced validation rules (slippage detection, MEV analysis)
- [ ] Batch processing optimization
- [ ] Cross-chain protocol support

### Performance Optimizations
- [ ] SIMD-optimized protocol detection
- [ ] Custom allocator for high-frequency decoding
- [ ] Zero-allocation fast paths

## Related Components

- **Protocol V2**: Core messaging protocol (`protocol_v2/`)
- **State Management**: Pool cache and state tracking (`libs/state/market/`)
- **Adapters**: Exchange collectors using DEX ABIs (`services_v2/adapters/`)
- **Strategies**: Trading strategies consuming DEX events (`services_v2/strategies/`)

## License

Part of the AlphaPulse trading system. See project root for license details.
# Migration Plan: VenueID → DEXProtocol for DEX Operations

## Executive Summary

Remove redundant VenueID abstraction for DEX operations, replacing it with direct DEXProtocol + chain_id identification. This simplifies the codebase while maintaining all necessary routing and execution information.

## Current State Analysis

### Problems with Current Design

1. **Redundancy**: VenueID duplicates information already present in:
   - Pool contract addresses (determines protocol via factory)
   - Active Web3 connection (determines chain)
   - DEXProtocol enum (determines AMM math and router)

2. **Unnecessary Abstraction**: DEXes aren't "venues" like traditional exchanges - they're deterministic smart contracts

3. **Maintenance Overhead**: Adding new DEX requires updating multiple enums and mappings

### Current Usage Points

```rust
// Current pattern
VenueId::UniswapV3  // Maps to chain_id=1, protocol=UniswapV3
VenueId::QuickSwap  // Maps to chain_id=137, protocol=QuickswapV2

// Information is duplicated across:
- libs/codec/src/instrument_id.rs (VenueId enum)
- libs/types/src/protocol/tlv/pool_state.rs (DEXProtocol enum)
- services_v2/adapters/polygon_adapter/src/constants.rs (router addresses)
```

## Proposed Architecture

### Core Changes

1. **Replace VenueID with ChainProtocol struct**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChainProtocol {
    pub chain_id: u32,
    pub protocol: DEXProtocol,
}
```

2. **Consolidate DEXProtocol enum** (single source of truth):
```rust
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DEXProtocol {
    UniswapV2 = 0,
    UniswapV3 = 1,
    SushiswapV2 = 2,
    QuickswapV2 = 3,
    QuickswapV3 = 4,
    CurveStableSwap = 5,
    BalancerV2 = 6,
    PancakeSwapV2 = 7,
}

impl DEXProtocol {
    /// Get router address for this protocol on given chain
    pub fn router_address(&self, chain_id: u32) -> Option<[u8; 20]> {
        match (self, chain_id) {
            (DEXProtocol::UniswapV3, 1) => Some(hex!("E592427A0AEce92De3Edee1F18E0157C05861564")),
            (DEXProtocol::UniswapV3, 137) => Some(hex!("E592427A0AEce92De3Edee1F18E0157C05861564")),
            (DEXProtocol::QuickswapV2, 137) => Some(hex!("a5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff")),
            // ... etc
            _ => None,
        }
    }
    
    /// Get factory address for pool discovery
    pub fn factory_address(&self, chain_id: u32) -> Option<[u8; 20]> {
        // Similar mapping
    }
    
    /// AMM math variant for this protocol
    pub fn math_variant(&self) -> AMMVariant {
        match self {
            DEXProtocol::UniswapV2 | DEXProtocol::SushiswapV2 | DEXProtocol::QuickswapV2 => {
                AMMVariant::ConstantProduct
            }
            DEXProtocol::UniswapV3 | DEXProtocol::QuickswapV3 => {
                AMMVariant::ConcentratedLiquidity
            }
            DEXProtocol::CurveStableSwap => AMMVariant::StableSwap,
            DEXProtocol::BalancerV2 => AMMVariant::WeightedPool,
            _ => AMMVariant::ConstantProduct,
        }
    }
}
```

3. **Simplified PoolInfo struct**:
```rust
pub struct PoolInfo {
    pub pool_address: [u8; 20],
    pub token0: [u8; 20],
    pub token1: [u8; 20],
    pub token0_decimals: u8,
    pub token1_decimals: u8,
    pub protocol: DEXProtocol,  // Instead of venue
    pub fee_tier: Option<u32>,
    pub discovered_at: u64,
    pub last_seen: u64,
    // chain_id derived from Web3 connection context
}
```

4. **Modified InstrumentId for Pools**:
```rust
impl InstrumentId {
    /// Create pool instrument ID
    pub fn pool(
        chain_id: u32,
        protocol: DEXProtocol,
        pool_address: [u8; 20],
    ) -> Result<Self, CodecError> {
        // Use pool address as primary identifier
        // Protocol stored in reserved byte
        // Chain can be encoded in venue field or derived from context
        
        Ok(Self {
            symbol: pool_address[0..16].try_into()?,
            venue: chain_id as u16,  // Repurpose for chain_id
            asset_type: AssetType::Pool as u8,
            reserved: protocol as u8,  // Store protocol in reserved byte
        })
    }
}
```

## Implementation Steps

### Phase 1: Create New Structures (Non-Breaking)
1. Add `ChainProtocol` struct to `libs/codec/src/protocol_constants.rs`
2. Extend `DEXProtocol` enum with router/factory methods
3. Create compatibility layer for existing VenueId usage

### Phase 2: Update Core Components
1. Modify `PoolInfo` struct to use `DEXProtocol` instead of `VenueId`
2. Update pool cache persistence (TLV format changes)
3. Adjust pool discovery logic to use protocol detection

### Phase 3: Migrate Service Layer
1. Update `polygon_adapter` to use `DEXProtocol` directly
2. Modify arbitrage strategies to work with new structure
3. Update dashboard message converters

### Phase 4: Clean Up InstrumentId
1. Deprecate DEX-specific VenueId variants
2. Repurpose venue field for chain_id in pool instruments
3. Use reserved byte for protocol storage

### Phase 5: Remove Legacy Code
1. Remove VenueId DEX variants (UniswapV2, UniswapV3, etc.)
2. Clean up redundant mappings
3. Update all tests

## Migration Checklist

### Files to Modify

#### Core Protocol
- [ ] `libs/codec/src/instrument_id.rs` - Remove DEX VenueIds
- [ ] `libs/codec/src/protocol_constants.rs` - Add ChainProtocol, router mappings
- [ ] `libs/types/src/protocol/tlv/pool_state.rs` - Enhance DEXProtocol
- [ ] `libs/types/src/protocol/tlv/pool_cache.rs` - Update cache format

#### Services
- [ ] `services_v2/adapters/pool_cache/mod.rs` - Use DEXProtocol
- [ ] `services_v2/adapters/polygon_adapter/src/constants.rs` - Consolidate routers
- [ ] `services_v2/adapters/dex_utils/src/abi/mod.rs` - Protocol detection
- [ ] `services_v2/strategies/flash_arbitrage/src/signal_output.rs` - Update references

#### Tests
- [ ] `tests/e2e/tests/arbitrage_dashboard_e2e.rs` - Update pool creation
- [ ] `libs/codec/tests/codec_tests.rs` - New InstrumentId tests
- [ ] `libs/types/tests/validation/instrument_id_bijection.rs` - Validate bijection

## Benefits

1. **Simpler Mental Model**: Pool identified by address + protocol, not artificial "venue"
2. **Reduced Duplication**: Single source of truth for protocol properties
3. **Easier Extension**: Adding new DEX only requires updating DEXProtocol enum
4. **Better Performance**: Direct protocol lookup instead of venue→protocol mapping
5. **Cleaner Execution Path**: Router address directly from protocol + chain

## Risks and Mitigations

### Risk 1: Breaking Existing InstrumentId Bijection
**Mitigation**: Create compatibility layer during migration, validate all existing IDs still work

### Risk 2: Lost Chain Information
**Mitigation**: Store chain_id in repurposed venue field or derive from connection context

### Risk 3: Test Breakage
**Mitigation**: Update tests incrementally, maintain backward compatibility during migration

## Performance Impact

- **Positive**: Fewer indirections (venue→protocol→router becomes protocol→router)
- **Positive**: Smaller memory footprint (remove redundant venue storage)
- **Neutral**: Same TLV message size (repurposing existing fields)

## Success Criteria

1. All DEX operations work without VenueId references
2. Pool discovery and caching use DEXProtocol directly
3. Execution routing determined by protocol + chain
4. No performance regression in message processing
5. Cleaner, more maintainable codebase

## Timeline Estimate

- Phase 1: 2 hours (new structures, compatibility layer)
- Phase 2: 3 hours (core component updates)
- Phase 3: 2 hours (service layer migration)
- Phase 4: 2 hours (InstrumentId cleanup)
- Phase 5: 1 hour (legacy removal)
- Testing: 2 hours (validation and regression testing)

**Total: ~12 hours of focused development**

## Next Steps

1. Review and approve this plan
2. Create feature branch `refactor/venue-to-protocol`
3. Implement Phase 1 with backward compatibility
4. Test thoroughly at each phase
5. Coordinate with team on breaking changes
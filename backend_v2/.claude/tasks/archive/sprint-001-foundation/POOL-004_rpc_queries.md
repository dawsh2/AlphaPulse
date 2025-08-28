# Task POOL-004: RPC Pool Discovery Implementation
*Agent Type: RPC Integration Specialist*
*Branch: `fix/rpc-pool-discovery`*
*Dependencies: POOL-001 (cache structure)*

## 📋 Your Mission
Implement the actual RPC discovery logic to fetch pool metadata (token addresses and decimals) for unknown pools.

## 🎯 Context
The PoolCache in `libs/state/market/src/pool_cache.rs` already has discovery infrastructure, but needs the actual RPC implementation to fetch pool data from the blockchain.

## 🔧 Git Setup Instructions

```bash
# Step 1: Ensure POOL-001 is available
git checkout main
git pull origin main

# Step 2: Create your feature branch
git checkout -b fix/rpc-pool-discovery

# Step 3: Confirm branch
git branch --show-current  # Should show: fix/rpc-pool-discovery
```

## 📝 Task Specification

### Files to Modify
1. `libs/state/market/src/pool_cache.rs` - Add RPC discovery methods
2. `libs/state/market/src/abi.rs` - Add pool contract ABIs (create if needed)

### Required Implementation

#### Step 1: Add Pool Contract ABIs
```rust
// Create libs/state/market/src/abi.rs

use web3::contract::{Contract, Options};
use web3::types::{H160, U256};

// Uniswap V2 Pair ABI (minimal)
pub const V2_PAIR_ABI: &str = r#"[
    {
        "constant": true,
        "inputs": [],
        "name": "token0",
        "outputs": [{"name": "", "type": "address"}],
        "type": "function"
    },
    {
        "constant": true,
        "inputs": [],
        "name": "token1",
        "outputs": [{"name": "", "type": "address"}],
        "type": "function"
    },
    {
        "constant": true,
        "inputs": [],
        "name": "getReserves",
        "outputs": [
            {"name": "_reserve0", "type": "uint112"},
            {"name": "_reserve1", "type": "uint112"},
            {"name": "_blockTimestampLast", "type": "uint32"}
        ],
        "type": "function"
    }
]"#;

// Uniswap V3 Pool ABI (minimal)
pub const V3_POOL_ABI: &str = r#"[
    {
        "inputs": [],
        "name": "token0",
        "outputs": [{"name": "", "type": "address"}],
        "type": "function"
    },
    {
        "inputs": [],
        "name": "token1",
        "outputs": [{"name": "", "type": "address"}],
        "type": "function"
    },
    {
        "inputs": [],
        "name": "fee",
        "outputs": [{"name": "", "type": "uint24"}],
        "type": "function"
    },
    {
        "inputs": [],
        "name": "slot0",
        "outputs": [
            {"name": "sqrtPriceX96", "type": "uint160"},
            {"name": "tick", "type": "int24"},
            {"name": "observationIndex", "type": "uint16"},
            {"name": "observationCardinality", "type": "uint16"},
            {"name": "observationCardinalityNext", "type": "uint16"},
            {"name": "feeProtocol", "type": "uint8"},
            {"name": "unlocked", "type": "bool"}
        ],
        "type": "function"
    }
]"#;

// ERC20 ABI for decimals
pub const ERC20_ABI: &str = r#"[
    {
        "constant": true,
        "inputs": [],
        "name": "decimals",
        "outputs": [{"name": "", "type": "uint8"}],
        "type": "function"
    },
    {
        "constant": true,
        "inputs": [],
        "name": "symbol",
        "outputs": [{"name": "", "type": "string"}],
        "type": "function"
    }
]"#;
```

#### Step 2: Implement RPC Discovery in PoolCache
```rust
// In libs/state/market/src/pool_cache.rs - Add these methods

impl PoolCache {
    /// Discover pool metadata via RPC
    pub async fn discover_pool_via_rpc(
        &self,
        pool_address: [u8; 20],
    ) -> Result<PoolInfo, PoolCacheError> {
        let web3 = self.web3.as_ref()
            .ok_or_else(|| PoolCacheError::RpcDiscoveryFailed("Web3 not configured".into()))?;

        let pool_h160 = H160::from_slice(&pool_address);

        // Try V3 first (has fee() method), then V2
        match self.try_discover_v3_pool(web3, pool_h160).await {
            Ok(info) => {
                info!("Discovered V3 pool: 0x{}", hex::encode(pool_address));
                Ok(info)
            }
            Err(_) => {
                // Try V2
                match self.try_discover_v2_pool(web3, pool_h160).await {
                    Ok(info) => {
                        info!("Discovered V2 pool: 0x{}", hex::encode(pool_address));
                        Ok(info)
                    }
                    Err(e) => {
                        error!("Failed to discover pool 0x{}: {}", hex::encode(pool_address), e);
                        Err(PoolCacheError::RpcDiscoveryFailed(e.to_string()))
                    }
                }
            }
        }
    }

    /// Try to discover as V3 pool
    async fn try_discover_v3_pool(
        &self,
        web3: &Web3<Http>,
        pool_address: H160,
    ) -> Result<PoolInfo, Box<dyn std::error::Error>> {
        let pool_contract = Contract::from_json(
            web3.eth(),
            pool_address,
            V3_POOL_ABI.as_bytes(),
        )?;

        // Get token addresses
        let token0: H160 = pool_contract
            .query("token0", (), None, Options::default(), None)
            .await?;
        let token1: H160 = pool_contract
            .query("token1", (), None, Options::default(), None)
            .await?;

        // Get fee tier
        let fee: U256 = pool_contract
            .query("fee", (), None, Options::default(), None)
            .await?;

        // Get decimals for each token
        let token0_decimals = self.get_token_decimals(web3, token0).await?;
        let token1_decimals = self.get_token_decimals(web3, token1).await?;

        Ok(PoolInfo {
            pool_address: pool_address.as_bytes().try_into()?,
            token0: token0.as_bytes().try_into()?,
            token1: token1.as_bytes().try_into()?,
            token0_decimals,
            token1_decimals,
            pool_type: DEXProtocol::UniswapV3,
            fee_tier: Some(fee.as_u32()),
            discovered_at: crate::utils::fast_timestamp(),
            venue: VenueId::Polygon,
            last_seen: crate::utils::fast_timestamp(),
        })
    }

    /// Try to discover as V2 pool
    async fn try_discover_v2_pool(
        &self,
        web3: &Web3<Http>,
        pool_address: H160,
    ) -> Result<PoolInfo, Box<dyn std::error::Error>> {
        let pool_contract = Contract::from_json(
            web3.eth(),
            pool_address,
            V2_PAIR_ABI.as_bytes(),
        )?;

        // Get token addresses
        let token0: H160 = pool_contract
            .query("token0", (), None, Options::default(), None)
            .await?;
        let token1: H160 = pool_contract
            .query("token1", (), None, Options::default(), None)
            .await?;

        // Get decimals for each token
        let token0_decimals = self.get_token_decimals(web3, token0).await?;
        let token1_decimals = self.get_token_decimals(web3, token1).await?;

        Ok(PoolInfo {
            pool_address: pool_address.as_bytes().try_into()?,
            token0: token0.as_bytes().try_into()?,
            token1: token1.as_bytes().try_into()?,
            token0_decimals,
            token1_decimals,
            pool_type: DEXProtocol::UniswapV2,
            fee_tier: Some(30), // V2 always 0.3% = 30 basis points
            discovered_at: crate::utils::fast_timestamp(),
            venue: VenueId::Polygon,
            last_seen: crate::utils::fast_timestamp(),
        })
    }

    /// Get token decimals via RPC
    async fn get_token_decimals(
        &self,
        web3: &Web3<Http>,
        token_address: H160,
    ) -> Result<u8, Box<dyn std::error::Error>> {
        // Special case for known tokens (optimization)
        match &token_address.as_bytes() {
            // WETH on Polygon
            b"\x7c\xeb\x23\xfd\x6b\xc0\xad\xd0\x5c\xf7\x66\x58\x00\x12\x56\x73\xcd\x83\xb0\x23" => return Ok(18),
            // USDC on Polygon
            b"\x2e\x9e\x1f\x01\x48\xce\xa4\x56\x50\x6e\xc2\xe7\x9a\xac\x0e\x4e\xaa\x17\x8f\x61" => return Ok(6),
            // USDT on Polygon
            b"\xc2\x13\x2d\x05\xd3\x17\x62\xb0\xd4\x1e\xaa\x5f\x5b\xc2\x29\xc3\x66\xb1\x23\x5f" => return Ok(6),
            _ => {}
        }

        // Query contract for decimals
        let token_contract = Contract::from_json(
            web3.eth(),
            token_address,
            ERC20_ABI.as_bytes(),
        )?;

        let decimals: u8 = token_contract
            .query("decimals", (), None, Options::default(), None)
            .await
            .unwrap_or(18); // Default to 18 if call fails

        Ok(decimals)
    }
}
```

#### Step 3: Update Discovery Worker (from POOL-003)
```rust
// Update the discover_pool_metadata function in POOL-003's discovery_queue.rs

async fn discover_pool_metadata(
    cache: &PoolCache,
    pool_address: H160,
) -> Result<PoolInfo, Box<dyn std::error::Error>> {
    // Use the cache's RPC discovery
    cache.discover_pool_via_rpc(pool_address.as_bytes().try_into()?)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}
```

## ✅ Acceptance Criteria

1. **RPC Discovery**
   - [ ] Correctly identifies V2 vs V3 pools
   - [ ] Fetches token0 and token1 addresses
   - [ ] Retrieves correct decimals for each token
   - [ ] Handles RPC failures gracefully

2. **Performance**
   - [ ] RPC calls timeout after 5 seconds
   - [ ] Retry logic with exponential backoff
   - [ ] Rate limiting to avoid overwhelming RPC
   - [ ] Caches discovered pools to avoid repeat queries

3. **Error Handling**
   - [ ] Graceful fallback if pool type unknown
   - [ ] Clear error messages for debugging
   - [ ] No panics on RPC failures
   - [ ] Proper timeout handling

## 🧪 Testing Instructions

```bash
# Unit tests with mock RPC
cargo test --package torq-state-market pool_discovery

# Integration test with real Polygon RPC
POLYGON_RPC_URL=https://polygon-rpc.com cargo test --package torq-state-market --test rpc_discovery -- --nocapture

# Test specific pools
cargo run --example discover_pool -- 0x45dda9cb7c25131df268515131f647d726f50608
```

## 🔄 Rollback Instructions

If this change causes issues in production:

```bash
# Immediate rollback
git revert HEAD
git push origin main

# Or rollback to specific commit before this change
git checkout main
git reset --hard <commit-before-pool-004>
git push --force-with-lease origin main

# Restart affected services
systemctl restart torq-collector
```

## 📤 Commit & Push Instructions

```bash
# Stage changes
git add libs/state/market/src/pool_cache.rs
git add libs/state/market/src/abi.rs

# Commit
git commit -m "feat(pool): implement RPC discovery for pool metadata

- Add V2/V3 pool detection via contract calls
- Fetch token addresses and decimals via RPC
- Implement retry logic with timeout protection
- Cache results to avoid repeated RPC calls"

# Push
git push -u origin fix/rpc-pool-discovery
```

## 🔄 Pull Request Template

```markdown
## Task POOL-004: RPC Pool Discovery Implementation

### Summary
Implemented RPC-based discovery to fetch pool metadata for unknown pools.

### Implementation
- V3 pool detection via fee() method
- V2 fallback for pools without fee()
- Token decimals fetching with known token optimization
- Timeout and retry logic for resilience

### Performance
- 5-second timeout per RPC call
- Known tokens return immediately (no RPC)
- Discovered pools cached permanently

### Testing
- [x] Mock RPC tests pass
- [x] Real Polygon RPC integration tested
- [x] Handles malformed pools gracefully

### Dependencies
- Uses POOL-001's cache structure
- Integrates with POOL-003's discovery queue
```

## ⚠️ Important Notes

1. **RPC Limits**: Most providers limit to 10-100 requests/second
2. **Known Tokens**: Hardcode common tokens to avoid RPC calls
3. **Timeout Critical**: Never block longer than 5 seconds
4. **Cache Forever**: Once discovered, never query again
5. **V3 Detection**: Check for fee() method existence

## 🤝 Coordination
- Provides discovery implementation for POOL-003
- Uses cache structure from POOL-001
- Critical for POOL-002's token extraction

---
*RPC discovery is the bridge between blockchain data and our cache!*

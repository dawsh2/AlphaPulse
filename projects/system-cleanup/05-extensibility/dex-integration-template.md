# DEX Protocol Integration Template

## Template: Adding a New DEX Protocol

This template provides a standardized approach for integrating any new decentralized exchange (DEX) protocol into the AlphaPulse system.

## Step 1: Protocol Discovery Phase

### 1.1 Smart Contract Analysis
```yaml
# exchanges/dex/NEW_DEX/docs/contract_analysis.yaml
protocol_name: "NewDEX"
protocol_version: "v2"
documentation_url: "https://docs.newdex.org"
chain_id: 137  # Polygon

contracts:
  factory:
    address: "0x..."
    abi_url: "https://..."
    creation_block: 12345678
    verified: true
    
  router:
    address: "0x..."
    version: "v2"
    functions:
      - swapExactTokensForTokens
      - addLiquidity
      - removeLiquidity
    
  pools:
    type: "constant_product"  # or "stable", "weighted", "concentrated"
    fee_structure: "0.3%"  # or dynamic
    
event_signatures:
  swap: "Swap(address,uint256,uint256,uint256,uint256,address)"
  sync: "Sync(uint112,uint112)"
  mint: "Mint(address,uint256,uint256)"
  burn: "Burn(address,uint256,uint256,address)"
  
gas_optimization:
  multicall_supported: true
  batch_size: 100  # events per call
  block_range: 1000  # blocks per query
```

### 1.2 On-Chain Data Mapping
```yaml
# exchanges/dex/NEW_DEX/docs/event_mapping.yaml
swap_event:
  fields:
    sender: "event.args[0]"
    amount0In: "event.args[1]"
    amount1In: "event.args[2]"
    amount0Out: "event.args[3]"
    amount1Out: "event.args[4]"
    to: "event.args[5]"
    
  derived_fields:
    price:
      formula: "(amount1Out + amount1In) / (amount0Out + amount0In)"
      decimals: "token0.decimals - token1.decimals"
      
    volume:
      formula: "amount0In > 0 ? amount0In : amount0Out"
      conversion: "wei_to_decimal"
      
    side:
      logic: "amount0In > 0 ? 'sell' : 'buy'"
      
    timestamp:
      source: "block.timestamp"
      format: "unix_seconds"
      
pool_state:
  reserve0: "getReserves()[0]"
  reserve1: "getReserves()[1]"
  total_supply: "totalSupply()"
  fee: "factory.getFee(pair)"  # if dynamic
```

## Step 2: Implementation

### 2.1 Event Monitor Implementation
```rust
// exchanges/dex/NEW_DEX/src/monitor.rs
use ethers::prelude::*;
use crate::protocol::{TradeMessage, Side};
use crate::dex::template::{DexMonitor, MonitorConfig};

pub struct NewDexMonitor {
    config: MonitorConfig,
    provider: Provider<WebSocket>,
    factory: Factory<Provider<WebSocket>>,
    event_decoder: EventDecoder,
    pool_cache: PoolCache,
}

impl DexMonitor for NewDexMonitor {
    const PROTOCOL_NAME: &'static str = "newdex";
    const FACTORY_ADDRESS: &'static str = "0x...";
    const CREATION_BLOCK: u64 = 12345678;
    
    async fn initialize(&mut self) -> Result<(), MonitorError> {
        // Connect to Ankr WebSocket for mempool access
        self.provider = Provider::<WebSocket>::connect(
            "wss://rpc.ankr.com/polygon_ws/<API_KEY>"
        ).await?;
        
        // Load factory contract
        self.factory = Factory::new(
            Self::FACTORY_ADDRESS.parse()?,
            self.provider.clone()
        );
        
        // Discover and cache all pools
        self.discover_pools().await?;
        
        // Subscribe to events
        self.subscribe_to_events().await?;
        
        Ok(())
    }
    
    async fn discover_pools(&mut self) -> Result<Vec<Address>, Error> {
        // Use PairCreated events to find all pools
        let filter = self.factory
            .event::<PairCreatedFilter>()
            .from_block(Self::CREATION_BLOCK);
            
        let events = filter.query().await?;
        
        for event in events {
            let pool = Pool {
                address: event.pair,
                token0: event.token0,
                token1: event.token1,
                fee: self.get_pool_fee(event.pair).await?,
                reserves: self.get_reserves(event.pair).await?,
            };
            
            self.pool_cache.insert(event.pair, pool);
        }
        
        Ok(self.pool_cache.keys().cloned().collect())
    }
    
    async fn process_swap_event(&mut self, event: SwapEvent) -> Result<TradeMessage, Error> {
        let pool = self.pool_cache.get(&event.address)
            .ok_or(Error::UnknownPool)?;
            
        // Calculate price from swap amounts
        let price = self.calculate_price(
            event.amount0_in,
            event.amount1_in,
            event.amount0_out,
            event.amount1_out,
            pool.token0_decimals,
            pool.token1_decimals
        )?;
        
        // Determine trade side
        let side = if event.amount0_in > 0 {
            Side::Sell
        } else {
            Side::Buy
        };
        
        // Get block timestamp
        let block = self.provider
            .get_block(event.block_number)
            .await?
            .ok_or(Error::BlockNotFound)?;
            
        let timestamp_ns = block.timestamp.as_u64() * 1_000_000_000;
        
        Ok(TradeMessage {
            price: (price * 1e8) as i64,  // Fixed-point 8 decimals
            volume: self.normalize_volume(event, pool)?,
            timestamp_ns,
            side,
            symbol_hash: pool.symbol_hash,
        })
    }
    
    fn calculate_price(
        &self,
        amount0_in: U256,
        amount1_in: U256,
        amount0_out: U256,
        amount1_out: U256,
        decimals0: u8,
        decimals1: u8,
    ) -> Result<f64, Error> {
        // Handle Wei to decimal conversion
        let in0 = amount0_in.as_u128() as f64 / 10_f64.powi(decimals0 as i32);
        let in1 = amount1_in.as_u128() as f64 / 10_f64.powi(decimals1 as i32);
        let out0 = amount0_out.as_u128() as f64 / 10_f64.powi(decimals0 as i32);
        let out1 = amount1_out.as_u128() as f64 / 10_f64.powi(decimals1 as i32);
        
        // Price = token1_amount / token0_amount
        let token1_total = in1 + out1;
        let token0_total = in0 + out0;
        
        if token0_total == 0.0 {
            return Err(Error::InvalidSwap);
        }
        
        Ok(token1_total / token0_total)
    }
}
```

### 2.2 Mempool Integration
```rust
// exchanges/dex/NEW_DEX/src/mempool.rs
use ankr_sdk::{MempoolTransaction, WebSocketClient};

pub struct MempoolMonitor {
    ankr_client: WebSocketClient,
    router_address: Address,
    method_signatures: HashMap<[u8; 4], String>,
}

impl MempoolMonitor {
    pub async fn connect() -> Result<Self, Error> {
        let client = WebSocketClient::new(
            "wss://rpc.ankr.com/polygon_ws/<API_KEY>",
            SubscriptionType::PendingTransactions
        ).await?;
        
        let mut signatures = HashMap::new();
        signatures.insert(
            keccak256("swapExactTokensForTokens")[..4].try_into()?,
            "swap".to_string()
        );
        
        Ok(Self {
            ankr_client: client,
            router_address: "0x...".parse()?,
            method_signatures: signatures,
        })
    }
    
    pub async fn monitor_pending_swaps(&mut self) -> Result<(), Error> {
        let mut stream = self.ankr_client.subscribe_pending_txs().await?;
        
        while let Some(tx) = stream.next().await {
            if tx.to == Some(self.router_address) {
                self.process_pending_swap(tx).await?;
            }
        }
        
        Ok(())
    }
    
    async fn process_pending_swap(&self, tx: MempoolTransaction) -> Result<(), Error> {
        // Decode swap parameters
        let method_id = &tx.input[..4];
        
        if let Some(method) = self.method_signatures.get(method_id) {
            // Extract swap details for MEV analysis
            let swap_data = self.decode_swap_data(&tx.input)?;
            
            // Check for MEV opportunities
            if self.is_mev_opportunity(&swap_data) {
                self.alert_mev_detector(swap_data).await?;
            }
        }
        
        Ok(())
    }
}
```

## Step 3: Testing

### 3.1 Smart Contract Interaction Tests
```rust
// exchanges/dex/NEW_DEX/src/tests/contract_tests.rs
#[cfg(test)]
mod tests {
    use super::*;
    use ethers::utils::Anvil;
    
    #[tokio::test]
    async fn test_pool_discovery() {
        // Fork mainnet for testing
        let anvil = Anvil::new()
            .fork("https://polygon-rpc.com")
            .fork_block_number(45000000)
            .spawn();
            
        let provider = Provider::try_from(anvil.endpoint()).unwrap();
        
        let mut monitor = NewDexMonitor::new(provider);
        let pools = monitor.discover_pools().await.unwrap();
        
        assert!(!pools.is_empty());
        
        // Verify pool data
        for pool_address in pools {
            let pool = monitor.pool_cache.get(&pool_address).unwrap();
            assert!(pool.reserves.0 > 0);
            assert!(pool.reserves.1 > 0);
        }
    }
    
    #[tokio::test]
    async fn test_event_parsing() {
        let sample_event = SwapEvent {
            sender: "0x...".parse().unwrap(),
            amount0_in: U256::from(1000000000000000000u64), // 1 token
            amount1_in: U256::zero(),
            amount0_out: U256::zero(),
            amount1_out: U256::from(2500000000), // 2500 USDC
            to: "0x...".parse().unwrap(),
        };
        
        let monitor = NewDexMonitor::new_test();
        let trade = monitor.process_swap_event(sample_event).await.unwrap();
        
        // Price should be 2500 USDC per token
        assert_eq!(trade.price, 250000000000); // 2500 * 1e8
        assert_eq!(trade.side, Side::Sell);
    }
    
    #[tokio::test]
    async fn test_wei_conversion_precision() {
        let test_cases = vec![
            (U256::from(1), 18, 0.000000000000000001),
            (U256::from(1000000000000000000u64), 18, 1.0),
            (U256::from(1000000), 6, 1.0), // USDC
        ];
        
        for (wei, decimals, expected) in test_cases {
            let result = wei_to_decimal(wei, decimals);
            assert!((result - expected).abs() < 1e-15);
        }
    }
}
```

### 3.2 Gas Optimization Tests
```rust
// exchanges/dex/NEW_DEX/src/tests/gas_tests.rs
#[tokio::test]
async fn test_multicall_efficiency() {
    let provider = get_test_provider();
    let multicall = Multicall::new(provider.clone());
    
    // Test batching 100 pool state queries
    let mut calls = vec![];
    for pool in &test_pools[..100] {
        calls.push(pool.get_reserves_call());
    }
    
    let start = Instant::now();
    let results = multicall.aggregate(calls).await.unwrap();
    let duration = start.elapsed();
    
    println!("Multicall for 100 pools: {:?}", duration);
    assert!(duration < Duration::from_secs(1));
    
    // Compare to individual calls
    let start = Instant::now();
    for pool in &test_pools[..10] { // Only 10 to avoid timeout
        pool.get_reserves().await.unwrap();
    }
    let individual_duration = start.elapsed();
    
    // Multicall should be >10x faster per pool
    assert!(duration.as_millis() / 100 < individual_duration.as_millis() / 10);
}

#[tokio::test]
async fn test_event_query_optimization() {
    let monitor = NewDexMonitor::new(get_test_provider());
    
    // Test different block ranges
    let ranges = vec![100, 500, 1000, 5000];
    
    for range in ranges {
        let start = Instant::now();
        let events = monitor.query_events_in_range(0, range).await.unwrap();
        let duration = start.elapsed();
        
        println!("Query {} blocks: {:?}, events: {}", range, duration, events.len());
        
        // Should stay under 5 seconds even for large ranges
        assert!(duration < Duration::from_secs(5));
    }
}
```

### 3.3 MEV Detection Tests
```python
# exchanges/dex/NEW_DEX/tests/test_mev_detection.py
import pytest
from web3 import Web3
from decimal import Decimal

class TestMEVDetection:
    @pytest.fixture
    def sample_transactions(self):
        """Load sample mempool transactions"""
        return [
            {
                "hash": "0x...",
                "to": "0xROUTER",
                "input": "0x38ed1739...",  # swapExactTokensForTokens
                "gasPrice": Web3.toWei(100, 'gwei'),
                "value": 0,
            },
            # More sample transactions
        ]
    
    def test_sandwich_attack_detection(self, sample_transactions):
        """Test detection of sandwich attack patterns"""
        detector = MEVDetector()
        
        # Create sandwich attack scenario
        victim_tx = sample_transactions[0]
        front_run = create_front_run_tx(victim_tx)
        back_run = create_back_run_tx(victim_tx)
        
        # Detect pattern
        is_sandwich = detector.detect_sandwich(
            [front_run, victim_tx, back_run]
        )
        
        assert is_sandwich
        assert detector.calculate_mev_profit(front_run, back_run) > 0
    
    def test_arbitrage_opportunity(self):
        """Test cross-DEX arbitrage detection"""
        pools = [
            {"dex": "newdex", "price": Decimal("2500.50")},
            {"dex": "uniswap", "price": Decimal("2495.00")},
        ]
        
        opportunity = detect_arbitrage(pools)
        
        assert opportunity is not None
        assert opportunity["profit_percent"] > Decimal("0.1")
        assert opportunity["path"] == ["uniswap", "newdex"]
```

## Step 4: Performance Validation

### 4.1 Event Processing Benchmarks
```rust
// exchanges/dex/NEW_DEX/benches/event_processing.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_event_decoding(c: &mut Criterion) {
    let raw_log = create_sample_log();
    
    c.bench_function("decode_swap_event", |b| {
        b.iter(|| {
            decode_swap_event(black_box(&raw_log))
        });
    });
}

fn benchmark_price_calculation(c: &mut Criterion) {
    let amounts = (
        U256::from(1000000000000000000u64),
        U256::zero(),
        U256::zero(),
        U256::from(2500000000u64),
    );
    
    c.bench_function("calculate_price_from_swap", |b| {
        b.iter(|| {
            calculate_price(
                black_box(amounts.0),
                black_box(amounts.1),
                black_box(amounts.2),
                black_box(amounts.3),
                18,
                6,
            )
        });
    });
}

fn benchmark_mempool_processing(c: &mut Criterion) {
    let pending_tx = create_sample_pending_tx();
    let monitor = MempoolMonitor::new_test();
    
    c.bench_function("process_mempool_tx", |b| {
        b.iter(|| {
            monitor.process_pending_swap(black_box(&pending_tx))
        });
    });
}

criterion_group!(
    benches, 
    benchmark_event_decoding, 
    benchmark_price_calculation,
    benchmark_mempool_processing
);
criterion_main!(benches);
```

### 4.2 Gas Cost Analysis
```python
# exchanges/dex/NEW_DEX/tests/test_gas_costs.py
import asyncio
from web3 import Web3

async def test_gas_optimization():
    """Analyze gas costs for different query strategies"""
    w3 = Web3(Web3.HTTPProvider('https://polygon-rpc.com'))
    
    strategies = [
        {"name": "individual_calls", "batch_size": 1},
        {"name": "small_batch", "batch_size": 10},
        {"name": "medium_batch", "batch_size": 50},
        {"name": "large_batch", "batch_size": 100},
    ]
    
    results = {}
    
    for strategy in strategies:
        gas_used = await measure_gas_usage(
            strategy["batch_size"],
            num_pools=100
        )
        
        results[strategy["name"]] = {
            "total_gas": gas_used,
            "gas_per_pool": gas_used / 100,
            "cost_usd": calculate_cost(gas_used),
        }
    
    print("Gas Optimization Results:")
    for name, metrics in results.items():
        print(f"{name}: {metrics['gas_per_pool']} gas/pool")
    
    # Assert optimal batch size
    assert results["medium_batch"]["gas_per_pool"] < \
           results["individual_calls"]["gas_per_pool"] * 0.2
```

## Step 5: Documentation

### 5.1 Auto-Generated Documentation
```python
# exchanges/dex/NEW_DEX/docs/generate_docs.py
import json
from pathlib import Path
from web3 import Web3

def generate_dex_documentation():
    """Generate documentation from contract ABIs and configs"""
    
    # Load contract ABIs
    factory_abi = load_abi("factory.json")
    router_abi = load_abi("router.json")
    pair_abi = load_abi("pair.json")
    
    doc = f"""
# {config['protocol_name']} Integration Documentation

## Protocol Overview
- **Type**: {config['protocol_type']}
- **Chain**: {config['chain_name']} (ID: {config['chain_id']})
- **Fee Model**: {config['fee_structure']}
- **TVL**: ${get_tvl():,.2f}

## Smart Contracts

### Factory Contract
- **Address**: `{config['factory_address']}`
- **Deployment Block**: {config['creation_block']}
- **Verified**: ✅

### Router Contract
- **Address**: `{config['router_address']}`
- **Version**: {config['router_version']}

## Event Signatures
"""
    
    # Document events
    for event in factory_abi:
        if event['type'] == 'event':
            signature = generate_event_signature(event)
            doc += f"\n### {event['name']}\n"
            doc += f"- **Signature**: `{signature}`\n"
            doc += f"- **Topics**: {len(event['inputs'])}\n"
            doc += document_event_fields(event)
    
    # Generate integration examples
    doc += generate_integration_examples()
    
    # Save documentation
    Path("README.md").write_text(doc)
    
    # Generate test fixtures
    generate_test_fixtures_from_chain()
    
    # Create monitoring dashboard config
    generate_grafana_dashboard()

def generate_integration_examples():
    """Create code examples for common operations"""
    return f"""
## Integration Examples

### Monitoring Swap Events
```rust
let monitor = NewDexMonitor::new(provider).await?;
let mut event_stream = monitor.subscribe_swaps().await?;

while let Some(swap) = event_stream.next().await {
    println!("Swap detected: {{:?}}", swap);
}
```

### Calculating Pool Price
```rust
let pool = monitor.get_pool(pool_address).await?;
let price = pool.calculate_price()?;
println!("Current price: {{}}", price);
```

### MEV Opportunity Detection
```rust
let mempool = MempoolMonitor::connect().await?;
mempool.on_opportunity(|opp| {{
    if opp.profit > MIN_PROFIT {{
        execute_strategy(opp).await?;
    }}
}});
```
"""
```

### 5.2 Integration Checklist
```markdown
# NEW_DEX Integration Checklist

## Discovery Phase
- [ ] Smart contract addresses identified
- [ ] ABIs downloaded and verified
- [ ] Event signatures documented
- [ ] Fee structure understood
- [ ] Pool discovery method tested
- [ ] Sample events collected from chain

## Implementation Phase
- [ ] Event monitor implemented
- [ ] Price calculation tested
- [ ] Wei conversion handling correct
- [ ] Pool state tracking working
- [ ] Mempool integration complete
- [ ] Error handling comprehensive

## Testing Phase
- [ ] Unit tests passing (100% coverage)
- [ ] Integration tests with forked chain
- [ ] MEV detection tests passing
- [ ] Gas optimization verified
- [ ] Performance benchmarks met:
  - [ ] Event processing <100μs
  - [ ] Price calculation <10μs
  - [ ] Mempool processing <50μs
- [ ] Precision tests passing (no Wei loss)

## Documentation Phase
- [ ] Contract addresses documented
- [ ] Event mappings complete
- [ ] Integration examples provided
- [ ] Gas costs analyzed
- [ ] MEV strategies documented
- [ ] Troubleshooting guide created

## Deployment Phase
- [ ] Ankr WebSocket configured
- [ ] RPC endpoints tested
- [ ] Monitoring dashboards created
- [ ] Alerts configured
- [ ] Production validation complete

## Sign-off
- [ ] Engineering review
- [ ] Security audit (contract interaction)
- [ ] Performance validation
- [ ] Documentation complete
```

## Step 6: Configuration

### 6.1 DEX Configuration Template
```yaml
# exchanges/dex/NEW_DEX/config/default.yaml
protocol:
  name: "newdex"
  display_name: "New DEX Protocol"
  type: "amm"  # amm, orderbook, hybrid
  version: "v2"
  
blockchain:
  chain_id: 137  # Polygon
  rpc:
    http: "https://polygon-rpc.com"
    ws: "wss://rpc.ankr.com/polygon_ws/${ANKR_API_KEY}"
  
  multicall:
    address: "0x11ce4B23bD875D7F5C6a31084f55fDe1e9A87507"
    enabled: true
    batch_size: 100
    
contracts:
  factory:
    address: "${NEW_DEX_FACTORY_ADDRESS}"
    creation_block: 12345678
    
  router:
    address: "${NEW_DEX_ROUTER_ADDRESS}"
    version: "02"
    
  pools:
    min_liquidity: 1000  # USD
    fee_tiers: [0.01, 0.05, 0.30, 1.00]  # percentages
    
event_monitoring:
  enabled: true
  block_range: 1000  # blocks per query
  confirmation_blocks: 3
  
  filters:
    min_volume_usd: 100
    exclude_tokens: []  # blacklist
    include_tokens: []  # whitelist (empty = all)
    
mempool:
  enabled: true
  monitor_pending: true
  
  mev_detection:
    sandwich_threshold: 0.5  # %
    arbitrage_threshold: 0.1  # %
    
data_processing:
  decimal_precision: 18
  price_decimals: 8
  volume_decimals: 8
  
gas_optimization:
  max_gas_price: 500  # gwei
  priority_fee: 2  # gwei
  
monitoring:
  metrics:
    enabled: true
    prefix: "dex.newdex"
    
  health_check:
    enabled: true
    interval: 60000  # ms
    max_block_lag: 5
    
  alerts:
    large_swap_threshold: 100000  # USD
    mev_opportunity_threshold: 1000  # USD
```

### 6.2 MEV Strategy Configuration
```yaml
# exchanges/dex/NEW_DEX/config/mev_strategies.yaml
strategies:
  sandwich:
    enabled: true
    min_profit_usd: 50
    max_gas_price: 1000  # gwei
    slippage_tolerance: 0.5  # %
    
    front_run:
      gas_premium: 1.2  # multiplier
      max_position_size: 10000  # USD
      
    back_run:
      gas_premium: 1.1
      profit_taking: 0.95  # take 95% of profit
      
  arbitrage:
    enabled: true
    min_profit_usd: 20
    max_hops: 3
    
    paths:
      - ["newdex", "uniswap", "newdex"]
      - ["newdex", "sushiswap", "newdex"]
      
  liquidation:
    enabled: false  # requires additional setup
    
  jit_liquidity:
    enabled: true
    min_swap_size: 50000  # USD
    max_il_tolerance: 0.2  # %
```

This template ensures every new DEX integration follows blockchain best practices and handles the unique challenges of on-chain data.
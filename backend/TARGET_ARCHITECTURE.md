# Target Architecture & File Structure Alignment

## 📁 Current Structure vs Target Structure

### Current (Scattered)
```
backend/
├── *.py                          # ~50 loose Python scripts
├── services/
│   ├── exchange_collector/       # Rust WebSocket collector
│   ├── capital_arb_bot/         # Partial Rust implementation
│   └── ws_bridge/                # WebSocket bridge
├── scripts/                      # Execution scripts
├── arbitrage_bot/                # New Rust bot (not integrated)
└── contracts/                    # Solidity contracts
```

### Target (Per Architecture Diagram)
```
backend/
├── hot-path/                     # 5-35μs latency critical path
│   ├── collectors/               # Data ingestion (Rust)
│   │   ├── polygon_dex/         # DEX data collector
│   │   └── mempool/             # Ankr mempool monitor
│   ├── relay/                   # Enhanced relay server
│   │   └── src/                 # Fan-out + DeFi routing
│   ├── detection/               # DeFi opportunity detection
│   │   ├── arbitrage/          # Cross-DEX scanner
│   │   ├── mev/                # MEV analyzer
│   │   └── liquidation/        # Position monitor
│   └── execution/               # Trade execution
│       ├── capital/             # Direct DEX trading
│       └── flash_loan/          # Aave V3 integration
├── contracts/                   # Smart contracts
│   ├── solidity/               # Standard contracts
│   │   ├── FlashLoanArbitrage.sol
│   │   └── LiquidationBot.sol
│   └── huff/                   # Gas-optimized contracts
│       ├── CompoundArbitrage.huff
│       └── FastSwap.huff
├── storage/                    # Data persistence
│   ├── timescale/             # Market + DeFi data
│   └── redis/                 # Cache + mempool
└── scripts/                   # Utility scripts
    ├── deploy/                # Contract deployment
    ├── test/                  # Testing utilities
    └── monitor/               # Monitoring tools
```

## 🏗️ Migration Plan

### Phase 1: Quick Win (Today)
```bash
# Test current setup for first trade
./quick_first_trade.py

# If successful, we have baseline working
```

### Phase 2: Restructure (Week 1)
```bash
# Create target structure
mkdir -p hot-path/{collectors,relay,detection,execution}
mkdir -p contracts/{solidity,huff}
mkdir -p storage/{timescale,redis}

# Move existing components
mv services/exchange_collector hot-path/collectors/polygon_dex
mv arbitrage_bot hot-path/execution/capital
```

### Phase 3: Implement Missing Components (Week 2-3)

#### A. Hot Path Components (5-35μs latency)
```rust
// hot-path/relay/src/main.rs
pub struct EnhancedRelay {
    collectors: Vec<DataCollector>,
    detectors: Vec<OpportunityDetector>,
    executors: Vec<Executor>,
}

impl EnhancedRelay {
    pub async fn process_tick(&mut self) {
        // 1. Collect data (parallel)
        let data = join_all(self.collectors.collect()).await;
        
        // 2. Detect opportunities (parallel)
        let opportunities = join_all(self.detectors.detect(&data)).await;
        
        // 3. Execute profitable ones
        for opp in opportunities {
            if opp.profit_usd > self.min_profit {
                self.execute(opp).await;
            }
        }
    }
}
```

#### B. MEV Analyzer
```rust
// hot-path/detection/mev/src/lib.rs
pub struct MevAnalyzer {
    mempool_monitor: MempoolMonitor,
    predictive_model: PredictiveModel,
}

impl MevAnalyzer {
    pub async fn analyze(&self, tx: Transaction) -> MevOpportunity {
        // Detect sandwich opportunities
        // Predict gas wars
        // Identify liquidations
    }
}
```

#### C. Liquidation Hunter
```rust
// hot-path/detection/liquidation/src/lib.rs
pub struct LiquidationHunter {
    aave_monitor: AaveMonitor,
    compound_monitor: CompoundMonitor,
}

impl LiquidationHunter {
    pub async fn scan_positions(&self) -> Vec<LiquidationTarget> {
        // Monitor health factors
        // Calculate profitability
        // Queue for execution
    }
}
```

## 🔄 Data Flow (Per Mermaid Diagram)

### 1. Ingestion (WebSocket Streams)
```
Polygon DEX → Collector → Enhanced Relay
Ankr Mempool → Collector → Enhanced Relay
```

### 2. Detection (Parallel Processing)
```
Enhanced Relay → Opportunity Detector
Enhanced Relay → MEV Analyzer
Enhanced Relay → Liquidation Hunter
```

### 3. Execution (Smart Routing)
```
Detector → Capital Arbitrage → DEX Routers
Detector → Flash Loan Engine → Aave V3 → DEX Routers
Hunter → Liquidation Contract → Aave V3
```

### 4. Storage (Hot Path)
```
TimescaleDB: Historical data, backtesting
Redis: Live mempool, current opportunities
```

## 📊 Performance Targets

| Component | Current | Target | Improvement |
|-----------|---------|--------|-------------|
| Data Ingestion | ~100ms | 5μs | 20,000x |
| Opportunity Detection | ~1s | 10μs | 100,000x |
| Execution Decision | ~500ms | 5μs | 100,000x |
| Total Latency | ~2s | 35μs | 57,000x |

## 🚀 Quick Start Path

While we build the full architecture:

1. **Get First Trade** (Today)
```bash
export PRIVATE_KEY="YOUR_WALLET_KEY"
./quick_first_trade.py  # Find and execute one trade
```

2. **Run Simple Bot** (This Week)
```bash
python3 auto_arbitrage_bot.py  # Continuous monitoring
```

3. **Deploy Huff Contract** (Next Week)
```bash
huffc contracts/huff/CompoundArbitrage.huff --bin-runtime
# Deploy for 70% gas savings
```

4. **Full System** (Month)
- Complete hot path implementation
- All components in Rust
- Sub-millisecond latency
- 10+ token compound arbitrage

## 🎯 Success Metrics

### Phase 1 (Baseline)
- [ ] Execute 1 profitable trade
- [ ] Verify system works end-to-end
- [ ] Document gas costs and profitability

### Phase 2 (Optimization)
- [ ] Deploy Huff contracts
- [ ] Reduce gas by 70%
- [ ] Implement compound paths (3-5 hops)

### Phase 3 (Scale)
- [ ] Full 10+ hop compound arbitrage
- [ ] Sub-millisecond detection
- [ ] $100+ daily profit
- [ ] 80% success rate

## 📝 Notes

The current implementation works but doesn't match our target architecture:
- Missing hot path optimization
- No compound arbitrage
- No Huff gas optimization
- Scattered file structure

However, we can start earning with the simple bot while building toward the target architecture!
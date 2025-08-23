# AlphaPulse End-to-End Tests

Comprehensive testing suite for validating the complete AlphaPulse trading system from data ingestion to execution.

## Quick Start

### Run Polygon Arbitrage Tests

Test the complete Polygon DEX arbitrage pipeline with real market data:

```bash
# Run Polygon arbitrage validation
./scripts/e2e_test.sh arbitrage

# Or run directly
cd tests/e2e
cargo run --release --bin e2e_runner -- --scenario polygon --live-data --timeout 300
```

### Run All Tests

```bash
# Comprehensive test suite
./scripts/e2e_test.sh full

# Individual scenarios
./scripts/e2e_test.sh basic       # Basic connectivity
./scripts/e2e_test.sh comprehensive  # All scenarios with mock data
```

## Test Scenarios

### 🎯 Polygon Arbitrage Test (`polygon_arbitrage`)

**What it validates:**
- Real Polygon DEX data collection (Uniswap V2/V3, SushiSwap)
- Flash arbitrage opportunity detection
- V3 math calculations with 8-decimal precision
- Execution signal generation
- End-to-end latency (<50ms target)

**Target pairs:** WETH/USDC, WMATIC/USDC, WBTC/USDC

**Expected results:**
- ✅ Detects 3+ arbitrage opportunities in 3 minutes
- ✅ Maintains <50ms detection latency
- ✅ Validates profit estimates >$10 USD
- ✅ Preserves 8-decimal precision through pipeline

### 📊 Kraken Signals Test (`kraken_to_dashboard`)

**What it validates:**
- Kraken WebSocket data ingestion
- Signal strategy processing
- Dashboard WebSocket streaming
- Message structure validation

## Manual Testing

### Live Polygon Arbitrage Validation

```bash
# Run with live data for 10 minutes
cd tests/e2e
cargo test --release test_live_polygon_arbitrage -- --ignored --nocapture
```

This will:
1. Connect to live Polygon DEX APIs
2. Monitor WETH/USDC, WMATIC/USDC, WBTC/USDC pairs
3. Detect real arbitrage opportunities
4. Validate V3 math calculations
5. Report detailed metrics and opportunities found

### Quick System Health Check

```bash
cd tests/e2e
cargo test test_polygon_arbitrage_detection --release
```

## Configuration

Key test parameters in `config/system.toml`:

```toml
[strategies.kraken_signals]
min_confidence_threshold = 60
max_position_size_usd = 1000.0

[monitoring]
max_latency_ms = 100
min_throughput_msg_per_sec = 100.0
```

## Interpreting Results

### Success Criteria

**Polygon Arbitrage Test:**
- ✅ **Arbitrage Detection:** Finds 3+ opportunities
- ✅ **Latency:** <50ms detection time
- ✅ **Precision:** 8-decimal accuracy maintained
- ✅ **Profit Validation:** Realistic profit estimates
- ✅ **Pool Coverage:** Updates from all target pairs

**Example Success Output:**
```
🎯 Arbitrage opportunity detected: profit=$15.75, spread=0.25%
⚡ Execution signal generated: action=flash_swap, amount=1000.0
✅ Successfully detected 5 arbitrage opportunities
```

### Failure Modes

**Common issues:**
- 🔴 **No opportunities found:** Market may be efficient (normal)
- 🔴 **High latency:** Network/processing bottlenecks
- 🔴 **Precision loss:** TLV conversion errors
- 🔴 **Pool data missing:** DEX API connectivity issues

## Architecture Validation

The E2E tests validate this complete data flow:

```
Polygon DEX APIs → Adapter → TLV Protocol → Flash Arbitrage Engine → Execution Signals
                                ↓
                          Dashboard WebSocket ← JSON Converter
```

### Key Components Tested

1. **Data Collection:** Polygon DEX adapters with real market data
2. **Protocol:** TLV message serialization/deserialization
3. **Strategy:** Flash arbitrage detection with V3 math
4. **Execution:** Signal generation and relay distribution
5. **Dashboard:** Real-time WebSocket streaming

## Performance Targets

- **Hot Path Latency:** <35μs (TLV processing)
- **Detection Latency:** <50ms (arbitrage opportunity)
- **Throughput:** >100 messages/second
- **Precision:** 8 decimal places preserved
- **Uptime:** 99.9% service availability

## Development Workflow

1. **Before committing:** Run `./scripts/e2e_test.sh basic`
2. **Before releases:** Run `./scripts/e2e_test.sh full`
3. **For arbitrage changes:** Run `./scripts/e2e_test.sh arbitrage`
4. **Live validation:** Run ignored tests manually

## Troubleshooting

### Test Failures

**Build errors:**
```bash
cargo build --release --workspace
```

**Network connectivity:**
```bash
# Check Polygon RPC access
curl -X POST https://polygon-rpc.com/ -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'
```

**Service startup issues:**
```bash
# Check logs
tail -f /tmp/alphapulse_logs/*.log

# Restart system
./scripts/start_system.sh restart
```

### Performance Issues

**High latency:**
- Check network connectivity to Polygon
- Verify no CPU throttling
- Monitor memory usage

**Low throughput:**
- Increase buffer sizes in config
- Check for blocking operations
- Profile critical path performance

## CI/CD Integration

```bash
# In CI pipeline
./scripts/e2e_test.sh comprehensive  # Skip live data tests
```

For production deployment validation, run with live data:
```bash
./scripts/e2e_test.sh arbitrage --live-data
```
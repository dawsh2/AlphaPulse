# System Maintenance Guide

## Overview

This guide covers essential maintenance procedures for the AlphaPulse trading system, focusing on Protocol V2 TLV architecture, pool cache management, and performance validation.

## Critical System Health Checks

### Daily Operations

#### 1. TLV Protocol Validation
```bash
# Run core protocol tests
cd backend_v2/protocol_v2
cargo test --test tlv_parsing
cargo test --test precision_validation
cargo test --test instrument_id_bijection

# Check for any TLV parsing errors in logs
grep -E "TLV.*error|parsing.*failed" /tmp/alphapulse/logs/*.log
```

#### 2. Performance Validation
```bash
# Verify message processing performance (target: >1M msg/s construction, >1.6M msg/s parsing)
cd backend_v2/protocol_v2
cargo run --bin test_protocol --release

# Expected output:
# Message construction: >1,000,000 msg/s
# Message parsing: >1,600,000 msg/s
# InstrumentID operations: >19,000,000 ops/s
```

#### 3. Pool Cache Health Check
```bash
# Check pool cache status
cd backend_v2/services_v2/adapters
cargo test pool_cache_manager --nocapture

# Monitor cache hit rates (target: >95%)
grep "cache_hit_rate" /tmp/alphapulse/logs/polygon_collector.log | tail -10

# Verify cache file integrity
ls -la /tmp/alphapulse/cache/
# Should see: pool_cache_chain_137.bin, pool_cache_journal_chain_137.bin
```

#### 4. Address Resolution Monitoring
```bash
# Check for truncated address usage (should be ZERO)
grep -r "Vec<u64>" backend_v2/ --include="*.rs" | grep -v "test"
# This should return NO results - all addresses should be [u8; 20]

# Monitor pool discovery queue
grep "pool_discovery_queue" /tmp/alphapulse/logs/polygon_collector.log | tail -5
```

### Weekly Operations

#### 1. TLV Type Registry Audit
```bash
# Check for TLV type conflicts
cd backend_v2/protocol_v2/src/tlv
grep -n "= [0-9]" types.rs | sort -k3 -n
# Verify no duplicate type numbers exist
```

#### 2. Performance Regression Testing
```bash
# Run full benchmark suite
cd backend_v2
cargo bench --workspace > benchmark_results.txt

# Compare against baseline (store previous results)
diff benchmark_results.txt benchmark_baseline.txt
```

#### 3. Pool Cache Maintenance
```bash
# Compact journal files (if size > 100MB)
cd /tmp/alphapulse/cache/
ls -la pool_cache_journal_*.bin
# If any journal > 100MB, restart collector to trigger compaction

# Verify cache recovery capability
cd backend_v2/services_v2/adapters
cargo test test_cache_recovery --nocapture
```

#### 4. Address Validation Sweep
```bash
# Ensure all DEX operations use full addresses
cd backend_v2
grep -r "pool_address" --include="*.rs" | grep -v "\[u8; 20\]"
# This should return NO results

# Check for proper address parsing
cargo test address_parsing --workspace
```

### Monthly Operations

#### 1. Complete System Validation
```bash
# Run full end-to-end tests
cd backend_v2/tests/e2e
cargo test --release -- --nocapture

# Validate data integrity across the pipeline
cd backend_v2/services_v2/adapters/tests
cargo test live_polygon_dex --release -- --nocapture
```

#### 2. Cache Performance Analysis
```bash
# Analyze cache performance trends
cd backend_v2
cargo run --bin cache_analyzer -- --days 30

# Pool discovery latency analysis
grep "pool_discovery_latency" /tmp/alphapulse/logs/*.log | \
  awk '{print $NF}' | sort -n | tail -100
```

#### 3. TLV Message Size Analysis
```bash
# Verify message sizes remain within bounds
cd backend_v2/protocol_v2
cargo run --bin message_size_analyzer

# Check for any oversized messages
grep "oversized.*message" /tmp/alphapulse/logs/*.log
```

## Emergency Procedures

### TLV Parsing Failures

#### Symptoms
- Relay logs show "TLV parsing failed"
- Message corruption errors
- Sequence gaps in message streams

#### Diagnosis
```bash
# Check for TLV structure violations
cd backend_v2/protocol_v2
cargo run --bin debug_tlv_parsing -- /tmp/alphapulse/logs/market_data_relay.log

# Validate message header integrity
grep -E "magic.*mismatch|payload_size.*invalid" /tmp/alphapulse/logs/*.log
```

#### Resolution
1. Stop all services: `pkill -f alphapulse`
2. Check TLV type registry for conflicts
3. Rebuild all services: `cargo build --workspace --release`
4. Restart system: `./scripts/start_system.sh`

### Pool Cache Corruption

#### Symptoms
- Cache files missing or corrupted
- Pool discovery failures
- Address resolution errors

#### Diagnosis
```bash
# Check cache file integrity
cd /tmp/alphapulse/cache/
file pool_cache_*.bin
# Should show: "data" not "ASCII text" or errors

# Verify CRC checksums
cd backend_v2/services_v2/adapters
cargo run --bin cache_validator -- /tmp/alphapulse/cache/
```

#### Resolution
1. Stop polygon collector
2. Remove corrupted cache files: `rm /tmp/alphapulse/cache/pool_cache_*`
3. Restart collector (will rebuild cache from scratch)
4. Monitor cache rebuilding: `tail -f /tmp/alphapulse/logs/polygon_collector.log`

### Performance Degradation

#### Symptoms
- Message processing < 1M msg/s
- High latency spikes
- Memory usage increasing

#### Diagnosis
```bash
# Check system resources
top -p $(pgrep -f alphapulse)
free -h
iostat -x 1 5

# Profile message processing
cd backend_v2
cargo build --release
perf record -g ./target/release/market_data_relay
perf report
```

#### Resolution
1. Check for competing processes
2. Verify NUMA topology and CPU pinning
3. Review recent code changes for performance regressions
4. Consider service restart if memory leaks detected

### Address Resolution Failures

#### Symptoms
- "Unknown pool address" errors
- Failed arbitrage executions
- Missing pool metadata

#### Diagnosis
```bash
# Check RPC endpoint health
curl -X POST https://rpc.ankr.com/polygon \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'

# Verify pool discovery queue
grep "pool_discovery.*failed" /tmp/alphapulse/logs/polygon_collector.log
```

#### Resolution
1. Switch to backup RPC endpoint
2. Clear failed discovery cache
3. Restart collector with fresh state
4. Monitor pool discovery success rate

## Performance Benchmarks

### Target Metrics

| Component | Metric | Target | Critical Threshold |
|-----------|--------|--------|--------------------|
| Message Construction | msg/s | >1,000,000 | <500,000 |
| Message Parsing | msg/s | >1,600,000 | <800,000 |
| InstrumentID Ops | ops/s | >19,000,000 | <10,000,000 |
| Pool Cache Hit Rate | % | >95% | <90% |
| Hot Path Latency | μs | <35 | >100 |
| Memory per Service | MB | <50 | >100 |

### Automated Monitoring

```bash
# Add to crontab for automated monitoring
# 0 */6 * * * /usr/local/bin/alphapulse_health_check.sh

#!/bin/bash
# /usr/local/bin/alphapulse_health_check.sh

cd /opt/alphapulse/backend_v2

# Performance check
perf_result=$(cargo run --bin test_protocol --release 2>&1 | grep "msg/s")
if ! echo "$perf_result" | grep -q "1,[0-9]{3},[0-9]{3}"; then
    echo "ALERT: Performance degradation detected" | mail -s "AlphaPulse Alert" ops@company.com
fi

# Cache health check
cache_size=$(du -s /tmp/alphapulse/cache/ | cut -f1)
if [ "$cache_size" -gt 1000000 ]; then  # 1GB
    echo "ALERT: Cache size exceeded 1GB" | mail -s "AlphaPulse Alert" ops@company.com
fi

# Service health check
for service in market_data_relay polygon_collector; do
    if ! pgrep -f "$service" > /dev/null; then
        echo "ALERT: Service $service is down" | mail -s "AlphaPulse Alert" ops@company.com
    fi
done
```

## Configuration Management

### TLV Type Registry

**CRITICAL**: Never reuse TLV type numbers. Always append new types.

```rust
// backend_v2/protocol_v2/src/tlv/types.rs
pub enum TLVType {
    // Market Data Domain (1-19) - NEVER CHANGE
    Trade = 1,
    Quote = 2,
    OrderBook = 3,
    // ... existing types
    
    // NEW TYPES: Always use next available number
    NewMarketDataType = 19,  // Next available in domain
}
```

### Pool Cache Configuration

```toml
# backend_v2/services_v2/adapters/config.toml
[pool_cache]
cache_dir = "/tmp/alphapulse/cache"
max_journal_size_mb = 100
compaction_interval_hours = 24
rpc_endpoints = [
    "https://rpc.ankr.com/polygon",
    "https://polygon-rpc.com"
]
discovery_timeout_ms = 5000
max_pending_discoveries = 1000
```

## Backup and Recovery

### Pool Cache Backup
```bash
# Daily backup of pool cache
rsync -av /tmp/alphapulse/cache/ /backup/alphapulse/cache/$(date +%Y%m%d)/

# Restore from backup
rsync -av /backup/alphapulse/cache/20241220/ /tmp/alphapulse/cache/
sudo chown alphapulse:alphapulse /tmp/alphapulse/cache/*
```

### Configuration Backup
```bash
# Backup service configurations
tar -czf /backup/alphapulse/config_$(date +%Y%m%d).tar.gz \
    backend_v2/services_v2/*/config.toml \
    backend_v2/relays/config/
```

## Troubleshooting Quick Reference

### Common Issues

| Issue | Quick Check | Solution |
|-------|-------------|----------|
| TLV parsing errors | `grep "TLV.*error" logs/` | Rebuild all services |
| Cache corruption | `file /tmp/alphapulse/cache/*` | Delete cache, restart |
| High memory usage | `ps aux \| grep alphapulse` | Check for memory leaks |
| Pool discovery fails | `curl RPC_ENDPOINT` | Switch RPC endpoint |
| Performance degradation | `cargo run --bin test_protocol` | Profile with perf |
| Address truncation | `grep Vec<u64> backend_v2/` | Must be [u8; 20] |

### Debug Commands

```bash
# Enable detailed logging
RUST_LOG=debug cargo run --bin market_data_relay

# Monitor message flow
tail -f /tmp/alphapulse/logs/*.log | grep -E "sequence|cache_hit"

# Test specific components
cd backend_v2/protocol_v2
cargo test --test tlv_parsing -- --nocapture

# Validate cache integrity
cd backend_v2/services_v2/adapters
cargo test pool_cache_manager -- --nocapture
```

## Contact Information

For critical system issues:
- **Performance Issues**: Run benchmarks, check system resources
- **Cache Corruption**: Follow emergency procedures above
- **TLV Protocol Issues**: Check type registry, rebuild services
- **Address Resolution**: Verify RPC endpoints, check discovery queue

Remember: The system is designed for full address architecture with pool cache persistence. Any use of truncated addresses or blocking pool discovery indicates a critical issue requiring immediate attention.
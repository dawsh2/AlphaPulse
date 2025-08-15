# AlphaPulse E2E Data Validation Pipeline

## Overview

This comprehensive test suite validates data integrity throughout the AlphaPulse data pipeline, from exchange WebSocket inputs through binary protocol processing to final JSON outputs at the frontend.

## Data Flow Architecture

```
Exchange WebSockets → Exchange Collectors → Unix Socket (Binary) → 
Relay Server → Unix Socket (Binary) → WS Bridge → WebSocket (JSON) → Frontend
```

## Test Components

### 1. **test_orchestrator.py** - Main Test Coordinator
- Manages service lifecycle (start/stop)
- Coordinates simultaneous data capture from Unix socket and WebSocket
- Generates comprehensive test reports
- Validates message flow and sequence continuity

### 2. **protocol_validator.py** - Binary Protocol Validation
- Connects to Unix socket and captures binary messages
- Decodes all message types (Trade, OrderBook, L2 Snapshot/Delta, Heartbeat, Symbol Mapping)
- Validates binary message structure and field values
- Tracks sequence numbers and detects message drops

### 3. **ws_data_interceptor.py** - WebSocket Data Capture
- Captures JSON messages from the WS bridge
- Tracks unique symbols and exchanges
- Provides time-series data extraction
- Saves captured data for analysis

### 4. **comparison_engine.py** - Data Comparison & Validation
- Compares binary protocol data with JSON output
- Validates fixed-point to floating-point conversions
- Checks price consistency and decimal handling
- Identifies arbitrage opportunities
- Measures end-to-end latency

### 5. **test_decimal_precision.py** - Decimal Precision Tests
- Tests fixed-point conversion accuracy (8 decimal places)
- Validates token-specific decimal handling:
  - USDC/USDT: 6 decimals
  - WETH/DAI: 18 decimals
  - WBTC: 8 decimals
- Verifies price pair consistency
- Tests edge cases and boundary conditions

## Running the Tests

### Quick Test (No Services)
```bash
# Run decimal precision tests only
python3 test_decimal_precision.py
```

### Full E2E Test
```bash
# Run complete test suite with live services
./run_e2e_tests.sh

# Custom capture duration (default: 60 seconds)
./run_e2e_tests.sh --duration 120

# Quick mode (skip service startup)
./run_e2e_tests.sh --mode quick
```

### Individual Components
```bash
# Run orchestrator with existing services
python3 test_orchestrator.py --no-services --duration 30

# Capture binary data only
python3 protocol_validator.py

# Capture WebSocket data only
python3 ws_data_interceptor.py
```

## Test Configuration

### Environment Variables
- `CAPTURE_DURATION`: Data capture duration in seconds (default: 60)
- `TEST_MODE`: Test mode - full, quick, or services-only (default: full)
- `EXCHANGE_NAME`: Exchange to test (default: alpaca)
- `RUST_LOG`: Rust service log level (default: info)

### Test Tolerances
- Price comparison: 0.01% (0.0001 relative difference)
- Fixed-point conversion: < 1e-10 absolute difference
- Latency threshold: < 100ms for 95th percentile

## Validation Checks

### 1. Data Integrity
- ✅ Binary message structure validation
- ✅ Fixed-point to float conversion accuracy
- ✅ Decimal precision for different token types
- ✅ Price consistency across pipeline stages

### 2. Message Flow
- ✅ Sequence number continuity
- ✅ Symbol hash consistency
- ✅ No message drops under normal load
- ✅ Timestamp ordering

### 3. Performance
- ✅ End-to-end latency measurements
- ✅ Throughput capacity
- ✅ Message processing rates
- ✅ Queue depth monitoring

## Test Reports

### Generated Files
- `e2e_test_report_YYYYMMDD_HHMMSS.json` - Complete test results
- `binary_capture_YYYYMMDD_HHMMSS.json` - Captured binary messages
- `ws_capture_YYYYMMDD_HHMMSS.json` - Captured WebSocket messages
- `comparison_report.json` - Data comparison results
- `decimal_precision_report.json` - Decimal handling test results

### Report Structure
```json
{
  "test_info": {
    "timestamp": "ISO-8601",
    "duration_seconds": 60,
    "capture_duration": 60
  },
  "summary": {
    "total_binary_messages": 1234,
    "total_ws_messages": 1234,
    "overall_status": "PASS/FAIL"
  },
  "integrity_validation": {
    "protocol_validation": {...},
    "data_comparison": {...}
  },
  "message_flow_validation": {
    "sequence_continuity": true,
    "symbol_consistency": true,
    "issues": []
  }
}
```

## Success Criteria

The E2E test suite passes when:
1. Protocol validation pass rate > 95%
2. Sequence continuity maintained
3. Symbol consistency verified
4. No critical data integrity issues
5. Latency within acceptable bounds

## Troubleshooting

### Common Issues

1. **Socket Connection Failed**
   - Ensure relay server is running
   - Check socket path: `/tmp/alphapulse/relay.sock`
   - Verify permissions on socket file

2. **No Messages Captured**
   - Check if exchange collectors are running
   - Verify WebSocket bridge is active
   - Ensure test exchanges have active data feeds

3. **Decimal Precision Failures**
   - Review token decimal configurations
   - Check fixed-point conversion logic
   - Verify price ranges for token pairs

4. **High Latency Warnings**
   - Check system load
   - Verify network connectivity
   - Review service configurations

## Future Enhancements

- [ ] Stress testing with high message volumes
- [ ] Multi-exchange simultaneous testing
- [ ] Automated performance regression detection
- [ ] Historical data replay testing
- [ ] Network failure simulation
- [ ] Data corruption detection
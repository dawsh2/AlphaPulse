# End-to-End Tracing System Testing Guide

## 🎯 Testing Overview

This guide validates the complete distributed tracing flow:
**Polygon Collector → Market Data Relay → Arbitrage Strategy → TraceCollector → Web Visualizer**

## 🚀 System Startup Sequence

### 1. Start TraceCollector (Foundation)
```bash
cd /Users/daws/alphapulse/backend_v2/services_v2/observability/trace_collector
cargo run --release
```
**Expected Output:**
- `📊 TraceCollector listening on /tmp/alphapulse/trace_collector.sock`
- `✅ TraceCollector API server listening on http://0.0.0.0:8080`

### 2. Start Market Data Relay (Central Hub)
```bash
cd /Users/daws/alphapulse/backend_v2/scripts
cargo run --bin start_market_data_relay
```
**Expected Output:**  
- `🚀 Starting MarketDataRelay (Domain 1)`
- `📊 MarketDataRelay connected to TraceCollector`
- `✅ MarketDataRelay listening for connections`

### 3. Start Polygon Collector (Data Source)
```bash
cd /Users/daws/alphapulse/backend_v2/services_v2/adapters
cargo run --bin polygon_publisher
```
**Expected Output:**
- `🚀 Starting Polygon DEX WebSocket collector`
- `📊 Connected to TraceCollector at /tmp/alphapulse/trace_collector.sock`
- `✅ Primary WebSocket connection established`

### 4. Start Arbitrage Strategy (Consumer)
```bash
cd /Users/daws/alphapulse/backend_v2/services_v2/strategies/flash_arbitrage
cargo run --release
```
**Expected Output:**
- `Starting MarketDataRelay consumer: /tmp/alphapulse/market_data.sock`
- `📊 ArbitrageStrategy connected to TraceCollector`
- `Connected to MarketDataRelay socket`

### 5. Open Web Visualizer
```bash
cd /Users/daws/alphapulse/backend_v2/services_v2/observability/trace_collector/web
python3 -m http.server 3000
```
**Then navigate to:** `http://localhost:3000`

## 🔍 Validation Checkpoints

### Phase 1: Service Connections
**Check Unix Socket Creation:**
```bash
ls -la /tmp/alphapulse/
# Should show:
# trace_collector.sock
# market_data.sock
```

**Verify TraceCollector API:**
```bash
curl http://localhost:8080/api/health
# Should return JSON health status
```

### Phase 2: Trace Event Flow
**Monitor TraceCollector Logs:**
Look for these trace events in sequence:

1. **DataCollected** (Polygon Collector):
   ```
   📊 Sent trace event: DataCollected
   Processing event: <trace_id> from PolygonCollector
   ```

2. **MessageReceived** (Market Data Relay):
   ```
   📊 Sent trace event: MessageReceived
   Relayed X bytes from conn_1 to Y consumers
   ```

3. **MessageReceived** (Arbitrage Strategy):
   ```
   📊 Sent trace event: MessageReceived
   📨 Received market data message: X bytes
   ```

4. **ExecutionTriggered** (When profitable opportunity found):
   ```
   📊 Sent trace event: ExecutionTriggered
   🎯 Arbitrage opportunity detected: profit=$X.XX
   ```

### Phase 3: Web Visualizer Validation

**Real-time Dashboard Checks:**
- ✅ Service status indicators show green (healthy)
- ✅ Message flows animate between service nodes
- ✅ Statistics update every second
- ✅ Click service nodes to see trace details

**Expected Flow Animation:**
```
Polygon Collector (🔗) → Market Data Relay (📡) → Arbitrage Strategy (🎯)
     PULSE               FLOW ARROW              PULSE
```

**API Data Validation:**
```bash
# Get active traces
curl http://localhost:8080/api/traces?limit=10

# Get dashboard summary  
curl http://localhost:8080/api/dashboard

# Get collector health
curl http://localhost:8080/api/health
```

## 🧪 Test Scenarios

### Test 1: Basic Message Flow
**Trigger:** WebSocket message from Polygon arrives
**Expected Trace Sequence:**
1. `DataCollected` (Polygon Collector)
2. `MessageProcessed` (Polygon Collector) 
3. `MessageSent` (Polygon Collector)
4. `MessageReceived` (Market Data Relay)
5. `MessageSent` (Market Data Relay)
6. `MessageReceived` (Arbitrage Strategy)
7. `MessageProcessed` (Arbitrage Strategy)

### Test 2: Arbitrage Detection
**Trigger:** Profitable swap opportunity detected
**Expected Additional Events:**
8. `ExecutionTriggered` (Arbitrage Strategy)
**Visualizer:** 
- Strategy node pulses green
- "Arbitrage opportunity" appears in service details

### Test 3: Error Handling  
**Trigger:** Disconnect Arbitrage Strategy
**Expected Behavior:**
- Market Data Relay continues operating
- Service status shows red for Strategy
- Error traces appear in visualizer
- No ExecutionTriggered events

### Test 4: Performance Under Load
**Trigger:** High-frequency Polygon events
**Expected Metrics:**
- Messages/sec > 10 in visualizer
- Average latency < 50ms
- Error rate < 5%
- No trace buffer overflows

## 📊 Performance Benchmarks

### Target Metrics:
- **End-to-End Latency**: < 100ms (WebSocket → Execution Decision)
- **Trace Event Rate**: > 100 events/sec
- **Memory Usage**: < 100MB per service
- **CPU Usage**: < 20% per service

### Measurement Commands:
```bash
# Monitor trace event throughput
curl http://localhost:8080/api/stats | jq '.data.events_per_second'

# Check service memory usage
ps aux | grep -E "(trace_collector|polygon|market_data|arbitrage)"

# Monitor trace processing latency
tail -f /tmp/trace_collector.log | grep "Processing event"
```

## 🐛 Troubleshooting

### Common Issues:

**1. Unix Socket Connection Errors:**
```bash
# Check socket permissions
sudo chmod 666 /tmp/alphapulse/*.sock

# Clear stale sockets
rm /tmp/alphapulse/*.sock
```

**2. TraceCollector Not Receiving Events:**
- Verify socket path in service configs
- Check firewall/SELinux settings
- Validate JSON serialization

**3. Web Visualizer Shows No Data:**
- Confirm TraceCollector API is running on port 8080
- Check browser console for CORS errors
- Validate API endpoint responses

**4. High Latency/Missing Traces:**
- Increase trace buffer sizes
- Check for network bottlenecks
- Verify timestamp synchronization

## ✅ Success Criteria

The end-to-end tracing system is validated when:

1. **🔄 Message Flow Visible**: Real-time traces flow through all services
2. **📊 Metrics Accurate**: Dashboard shows realistic throughput/latency data  
3. **🎯 Arbitrage Detection**: ExecutionTriggered events appear for opportunities
4. **🖥️ Visualizer Functional**: Web dashboard updates with live trace data
5. **⚡ Performance Acceptable**: < 100ms end-to-end latency maintained

## 📈 Production Readiness

For production deployment, ensure:
- **Persistent Storage**: TraceCollector writes to database
- **Authentication**: API endpoints secured with proper auth
- **Monitoring**: Alerts on trace collection failures
- **Scaling**: Multiple TraceCollector instances for redundancy
- **Retention**: Automatic trace cleanup after configurable period

---

**🎉 Completion Status**: The distributed tracing system provides complete observability into AlphaPulse's message flow, enabling real-time monitoring of component connections and performance optimization.
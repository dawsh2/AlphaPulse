# Live Streaming Pipeline Operations Manual

## Current Working State (as of 2025-08-25)
The E2E pipeline is successfully streaming live DEX events from Polygon to the frontend dashboard console.

### Working Pipeline Flow
```
Polygon WebSocket → Polygon Collector → Market Data Relay → Dashboard WebSocket Server → Frontend Console
```

## Critical Fix Applied
The dashboard relay consumer was only broadcasting Trade messages (type 1). Fixed to broadcast pool events:
- PoolSwap (type 11)
- PoolSync (type 16)
- Other pool event types (10, 12, 13)

## Startup Sequence (ORDER MATTERS!)

### 1. Start Relays First
```bash
cd /Users/daws/alphapulse/backend_v2
nohup target/release/market_data_relay > /tmp/alphapulse/logs/market_data_relay.log 2>&1 &
nohup target/release/signal_relay > /tmp/alphapulse/logs/signal_relay.log 2>&1 &
nohup target/release/execution_relay > /tmp/alphapulse/logs/execution_relay.log 2>&1 &
```

### 2. Start Polygon Collector
```bash
target/release/polygon > /tmp/alphapulse/logs/polygon_collector.log 2>&1 &
```

### 3. Start Dashboard WebSocket Server
```bash
target/release/alphapulse-dashboard-websocket > /tmp/alphapulse/logs/dashboard_websocket.log 2>&1 &
```

### 4. Start Frontend Dashboard
```bash
cd ../frontend
npm run dev:dashboard
```

## Verification Checklist

### Backend Verification
```bash
# Check polygon is processing events
tail -f /tmp/alphapulse/logs/polygon_collector.log | grep "Processed"
# Should see: "📊 Processed XXX DEX events (latency: Xμs)"

# Check relays are connected
ps aux | grep -E '(market_data_relay|signal_relay|execution_relay)' | grep -v grep
# All three should be running

# Check dashboard is connected to relays
tail -f /tmp/alphapulse/logs/dashboard_websocket.log | grep "Connected"
# Should see: "Connected to MarketData relay", "Connected to Signal relay", "Connected to Execution relay"
```

### Frontend Verification
Open browser console at http://localhost:5174
Should see:
```
🔄 Received pool swap: {pool_id: "0x...", token_in_symbol: "0x...", ...}
📨 Received WebSocket message: {msg_type: "pool_swap", ...}
```

## Common Issues & Solutions

### Issue: "Connection refused (os error 61)"
**Cause**: Relays not running or crashed
**Solution**: Restart relays in order (market_data, signal, execution)

### Issue: Dashboard receives data but doesn't display
**Cause**: TLV message types not being broadcast
**Solution**: Ensure relay_consumer.rs broadcasts types 10, 11, 12, 13, 16

### Issue: Frontend console shows no messages
**Cause**: WebSocket connection dropped
**Solution**: Check dashboard WebSocket server is running on port 8080

## Code Locations

### Critical Files
- **Relay Consumer**: `services_v2/dashboard/websocket_server/src/relay_consumer.rs`
  - Lines 389-416: Pool event broadcasting logic
- **Frontend Handler**: `frontend/src/dashboard/components/DeFiArbitrageTable.tsx`
  - Line 173: Pool swap message handling
- **Polygon Collector**: `services_v2/adapters/src/bin/polygon/polygon.rs`
  - Processes live DEX events from WebSocket

## Performance Metrics (Current)
- **Polygon Events**: 1000+ events processed
- **Latency**: 8-16μs per event
- **WebSocket Stability**: Stable with auto-reconnect
- **Memory Usage**: <100MB per service

## DO NOT REGRESS
1. Always start relays BEFORE collectors
2. Dashboard relay consumer MUST broadcast pool event types (10, 11, 12, 13, 16)
3. Frontend checks `message.msg_type` not `message.type`
4. Use `/tmp/alphapulse/*.sock` for Unix sockets

## Next Steps
1. Fix UI display (data received but not rendering in tables)
2. Implement arbitrage strategy processing
3. Connect arbitrage signals to Signal Relay
# WebSocket Disconnect Issue Analysis

## Problem Description
The frontend WebSocket clients disconnect from the dashboard server after receiving initial pool swap messages. After reconnection, no more pool swap messages are received even though:
1. Polygon collector continues processing events (2000+ events)
2. Dashboard WebSocket server reconnects to all relays
3. Frontend clients reconnect and send subscription messages

## Timeline of Events
- **01:22:47.472**: All 3 WebSocket clients disconnect simultaneously
- **01:22:47.790**: New client connects immediately
- **01:22:49.009**: Two more clients reconnect
- After reconnection: Only heartbeats are sent, no pool swap messages

## Root Cause Analysis

### Likely Causes:
1. **Message Buffer Overflow**: The relay consumer might be accumulating messages without processing them properly
2. **Broadcasting Logic Issue**: After the fix to broadcast pool events (types 10, 11, 12, 13, 16), there might be an issue with the broadcasting mechanism
3. **Client State Management**: The client manager might not be properly tracking subscriptions after reconnection

### Code Investigation Points:
1. **relay_consumer.rs**: Lines 195-240 - Message processing loop
   - Uses different parsing logic for MarketData vs other domains
   - Accumulates messages in a buffer before processing
   
2. **process_relay_message**: Lines 348-475
   - Only broadcasts certain TLV types
   - Might be silently dropping messages

## Temporary Workaround
Restart the dashboard WebSocket server:
```bash
pkill -f alphapulse-dashboard-websocket
sleep 1
nohup target/release/alphapulse-dashboard-websocket > /tmp/alphapulse/logs/dashboard_websocket.log 2>&1 &
```

## Permanent Fix Required
1. Add proper debug logging to track message flow
2. Implement message queue monitoring
3. Add reconnection handling that preserves subscriptions
4. Consider implementing a heartbeat-based dead connection detection

## Testing Steps
1. Monitor for disconnections in dashboard logs
2. Check if messages are being read from relays but not broadcast
3. Verify client subscription state after reconnection
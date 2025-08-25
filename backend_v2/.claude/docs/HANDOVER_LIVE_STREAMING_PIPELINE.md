# Handover Document: Live Streaming Pipeline Message Flow Issue

## Executive Summary
The AlphaPulse live streaming pipeline experiences intermittent message flow where the dashboard receives bursts of pool_swap messages (50-100 messages) then complete silence. The root cause is a **blocking read operation** in the relay consumer that waits indefinitely for new data after processing available messages.

## Current State of the Pipeline

### Working Components ✅
1. **Polygon Collector**: Successfully processing 2000+ DEX events
2. **Market Data Relay**: Running and forwarding messages
3. **Dashboard WebSocket Server**: Accepting frontend connections on port 8080
4. **Frontend**: Receiving messages in console but not rendering in UI

### Issue: Intermittent Message Flow 🔴
- **Symptom**: Frontend receives 50-100 pool_swap messages in rapid succession, then silence
- **Pattern**: Bursts occur when relay buffer fills, then blocking read waits for more data
- **Impact**: Dashboard appears frozen despite live events being processed

## Root Cause Analysis

### The Blocking Read Problem
**Location**: `services_v2/dashboard/websocket_server/src/relay_consumer.rs:196`

```rust
loop {
    match stream.read(&mut buffer).await {  // ← BLOCKS HERE
        Ok(0) => {
            warn!("{:?} relay connection closed", domain);
            break;
        }
        Ok(bytes_read) => {
            // Process messages...
        }
    }
}
```

### Message Flow Sequence
1. Relay accumulates messages in its internal buffer
2. Consumer connects and reads available data (8KB buffer)
3. Processes all complete TLV messages from buffer
4. **BLOCKS** on next `stream.read()` waiting for more data
5. Relay only sends data when buffer threshold reached or timeout
6. Results in burst pattern: rapid messages → long silence → rapid messages

### Evidence from Logs
```
01:25:29.220 - 01:25:35.816: 90+ "Broadcasted pool_swap message" (BURST)
01:25:35.816 - 01:27:00.000: SILENCE (only heartbeats)
```

## Immediate Fix Plan

### Option 1: Non-Blocking Read with Timeout (Recommended)
```rust
use tokio::time::timeout;
use std::time::Duration;

loop {
    match timeout(Duration::from_millis(100), stream.read(&mut buffer)).await {
        Ok(Ok(0)) => break,  // Connection closed
        Ok(Ok(bytes_read)) => {
            // Process messages
        }
        Ok(Err(e)) => {
            error!("Read error: {}", e);
            break;
        }
        Err(_) => {
            // Timeout - no data available, continue loop
            // This prevents blocking and allows other async tasks to run
            continue;
        }
    }
}
```

### Option 2: Use tokio::select! for Multiplexing
```rust
use tokio::select;

loop {
    select! {
        result = stream.read(&mut buffer) => {
            match result {
                Ok(0) => break,
                Ok(bytes_read) => {
                    // Process messages
                }
                Err(e) => {
                    error!("Read error: {}", e);
                    break;
                }
            }
        }
        _ = tokio::time::sleep(Duration::from_millis(50)) => {
            // Periodic wakeup to prevent indefinite blocking
        }
    }
}
```

## Implementation Steps

### 1. Fix Relay Consumer Blocking (Priority: HIGH)
**File**: `services_v2/dashboard/websocket_server/src/relay_consumer.rs`
**Lines**: 195-265
**Action**: Replace blocking read with non-blocking timeout approach

### 2. Add Buffer Monitoring (Priority: MEDIUM)
```rust
// Add metrics for debugging
let mut last_message_time = Instant::now();
let mut messages_in_burst = 0;

// In processing loop:
messages_in_burst += 1;
if last_message_time.elapsed() > Duration::from_secs(1) {
    debug!("Burst complete: {} messages in {:?}", 
           messages_in_burst, last_message_time.elapsed());
    messages_in_burst = 0;
    last_message_time = Instant::now();
}
```

### 3. Fix Frontend Rendering (Priority: HIGH)
**Issue**: Messages received in console but not displayed in UI
**File**: `frontend/src/dashboard/components/DeFiArbitrageTable.tsx`
**Problem**: Pool swaps not being added to arbitrage opportunities table
**Solution**: Create separate pool events table or adapt arbitrage table

## Testing & Validation

### Test Script
```bash
#!/bin/bash
# Test continuous message flow

# Start pipeline
./scripts/start_live_streaming_pipeline.sh --restart

# Monitor for bursts
echo "Monitoring for message bursts..."
tail -f /tmp/alphapulse/logs/dashboard_websocket.log | \
  awk '/Broadcasted pool_swap/ {
    count++
    current_time = systime()
    if (last_time && current_time - last_time > 2) {
      print "BURST DETECTED: " count " messages, gap: " (current_time - last_time) "s"
      count = 0
    }
    last_time = current_time
  }'
```

### Success Criteria
1. ✅ Continuous message flow (no gaps > 2 seconds during active trading)
2. ✅ Frontend displays pool events in real-time
3. ✅ No WebSocket disconnections
4. ✅ Message rate matches Polygon event rate

## Quick Start Commands

### Start Complete Pipeline
```bash
cd /Users/daws/alphapulse/backend_v2
./scripts/start_live_streaming_pipeline.sh --restart
```

### Monitor Health
```bash
# Check message flow
tail -f /tmp/alphapulse/logs/dashboard_websocket.log | grep "Broadcasted"

# Check Polygon events
tail -f /tmp/alphapulse/logs/polygon_collector.log | grep "Processed"

# Check frontend console
# Open browser dev tools at http://localhost:3001
```

### Debug Blocking Issue
```bash
# Attach strace to see blocking read
sudo strace -p $(pgrep -f alphapulse-dashboard-websocket) 2>&1 | grep -E "read|recv"
```

## Architecture Recommendations

### 1. Message Queue Pattern
Instead of direct Unix socket reads, implement a message queue:
- Relay → Redis Streams/NATS → Consumer
- Provides buffering, persistence, and non-blocking consumption

### 2. WebSocket Streaming Mode
Configure relay for streaming mode with:
- Smaller buffer thresholds
- Periodic flush intervals
- Keep-alive messages during quiet periods

### 3. Circuit Breaker Pattern
Add circuit breaker to handle relay disconnections gracefully:
- Exponential backoff on reconnect
- Message buffering during disconnection
- Automatic recovery

## Known Issues & Workarounds

### Issue 1: Messages Not Rendering in UI
**Workaround**: Messages are in browser console, can be logged for debugging
**Fix**: Implement dedicated pool events table in frontend

### Issue 2: WebSocket Disconnections
**Workaround**: Frontend auto-reconnects but loses subscription state
**Fix**: Persist subscription state server-side

### Issue 3: Relay Buffer Accumulation
**Workaround**: Restart pipeline periodically
**Fix**: Implement non-blocking read (this document)

## Contact & Support

For questions about this handover:
- **Pipeline Architecture**: See `backend_v2/docs/protocol.md`
- **TLV Protocol**: See `backend_v2/protocol_v2/README.md`
- **Frontend Issues**: See `frontend/src/dashboard/README.md`

## Appendix: File Locations

### Backend
- Relay Consumer: `backend_v2/services_v2/dashboard/websocket_server/src/relay_consumer.rs`
- Market Data Relay: `backend_v2/relays/src/market_data.rs`
- Polygon Collector: `backend_v2/services_v2/adapters/src/bin/polygon/polygon.rs`

### Frontend
- WebSocket Hook: `frontend/src/dashboard/hooks/useWebSocketFirehose.ts`
- DeFi Table: `frontend/src/dashboard/components/DeFiArbitrageTable.tsx`

### Scripts
- Pipeline Startup: `backend_v2/scripts/start_live_streaming_pipeline.sh`
- Health Check: `backend_v2/scripts/monitor_connections.sh`

---
**Document Version**: 1.0
**Date**: 2025-08-25
**Status**: Blocking issue identified, fix ready for implementation
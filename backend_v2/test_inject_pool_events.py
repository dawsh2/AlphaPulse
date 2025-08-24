#!/usr/bin/env python3
"""
Quick test to inject synthetic pool events into the relay
to verify the relay → dashboard → frontend pipeline works.
"""

import socket
import struct
import json
import time

def create_pool_event_message():
    """Create a simple message that looks like pool data"""
    # Create a fake TLV-like message with pool data
    # For testing, we'll send a JSON message directly to see if the relay forwards it
    
    pool_event = {
        "msg_type": "pool_swap",
        "timestamp": int(time.time() * 1000),
        "venue_name": "Polygon", 
        "pool_address": "0x45dda9cb7c25131df268515131f647d726f50608",
        "token0_symbol": "USDC",
        "token1_symbol": "WETH",
        "amount0_delta": -1000000,  # -1 USDC
        "amount1_delta": 250000000000000,  # +0.00025 WETH
        "fee_paid": 500,  # 0.5 USDC
        "block_number": 12345678,
        "log_index": 42
    }
    
    return json.dumps(pool_event).encode('utf-8')

def inject_test_events():
    """Inject test pool events into the Unix socket"""
    try:
        print("🧪 Testing pool event injection into relay...")
        
        # Connect to relay Unix socket
        sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        sock.connect("/tmp/alphapulse/market_data.sock")
        print("✅ Connected to market data relay")
        
        # Send a few test events
        for i in range(3):
            message = create_pool_event_message()
            
            # Send message length first, then message (simple framing)
            length = struct.pack('<I', len(message))
            sock.send(length)
            sock.send(message)
            
            print(f"📤 Sent test pool event {i+1}: {len(message)} bytes")
            time.sleep(0.1)
        
        print("✅ Test events sent!")
        print("📱 Check frontend at http://localhost:5177")
        print("🔍 Check WebSocket test script for receipt")
        
        sock.close()
        
    except Exception as e:
        print(f"❌ Error: {e}")

if __name__ == "__main__":
    inject_test_events()
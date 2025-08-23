#!/usr/bin/env python3
"""
Direct ANKR WebSocket test to isolate timing patterns.
This bypasses our collector entirely to see raw ANKR delivery timing.
"""

import asyncio
import websockets
import json
import time
from datetime import datetime
import os
from dotenv import load_dotenv

# Load environment variables
load_dotenv()

ANKR_API_KEY = os.getenv('ANKR_API_KEY')
if not ANKR_API_KEY:
    print("❌ ANKR_API_KEY not found in .env")
    exit(1)

WS_URL = f"wss://rpc.ankr.com/polygon/ws/{ANKR_API_KEY}"

# UniswapV3 Swap event signature
UNISWAP_V3_SWAP = "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67"
# UniswapV2 Swap event signature  
UNISWAP_V2_SWAP = "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822"

async def test_ankr_timing():
    print(f"🔗 Connecting directly to ANKR WebSocket...")
    print(f"📡 URL: {WS_URL}")
    
    try:
        async with websockets.connect(WS_URL) as websocket:
            # Subscribe to V3 swaps only for cleaner output
            subscription = {
                "jsonrpc": "2.0",
                "id": 1,
                "method": "eth_subscribe",
                "params": [
                    "logs",
                    {
                        "topics": [UNISWAP_V3_SWAP]
                    }
                ]
            }
            
            await websocket.send(json.dumps(subscription))
            print("✅ Sent V3 swap subscription")
            
            swap_count = 0
            last_receive_time = None
            
            while True:
                # Record precise receive time
                receive_time = time.time()
                
                message = await websocket.recv()
                data = json.loads(message)
                
                # Check for subscription confirmation
                if data.get("id") == 1 and data.get("result"):
                    print(f"🔗 Subscription confirmed: {data['result']}")
                    continue
                
                # Check for swap events
                if data.get("params") and data.get("params", {}).get("result"):
                    swap_count += 1
                    
                    # Format timestamp with microsecond precision
                    dt = datetime.fromtimestamp(receive_time)
                    time_str = dt.strftime("%H:%M:%S.%f")
                    
                    # Calculate gap from previous event
                    gap_ms = ""
                    if last_receive_time:
                        gap = (receive_time - last_receive_time) * 1000
                        gap_ms = f" (+{gap:.1f}ms)"
                    
                    print(f"🔍 RAW ANKR delivered swap #{swap_count} at {time_str}{gap_ms}")
                    
                    last_receive_time = receive_time
                    
                    # Show first few events in detail
                    if swap_count <= 3:
                        result = data["params"]["result"]
                        print(f"    Pool: {result.get('address', 'unknown')}")
                        print(f"    Block: {result.get('blockNumber', 'unknown')}")
                        print(f"    TxHash: {result.get('transactionHash', 'unknown')[:20]}...")
                
    except websockets.exceptions.ConnectionClosed:
        print("❌ WebSocket connection closed")
    except Exception as e:
        print(f"❌ Error: {e}")

if __name__ == "__main__":
    print("🧪 Testing ANKR WebSocket timing patterns directly")
    print("📊 This will show raw message delivery timing from ANKR")
    print("⏱️  Press Ctrl+C to stop")
    print()
    
    try:
        asyncio.run(test_ankr_timing())
    except KeyboardInterrupt:
        print("\n🛑 Test stopped by user")
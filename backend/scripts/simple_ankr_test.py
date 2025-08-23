#!/usr/bin/env python3
import asyncio
import websockets
import json
import time
from datetime import datetime

# Use your ANKR API key directly
ANKR_API_KEY = "e6fac469b91ea8fd98406aca0820653ae6fe5c2400f44819450f6022dd2792e2"
WS_URL = f"wss://rpc.ankr.com/polygon/ws/{ANKR_API_KEY}"

async def test_timing():
    print(f"🔗 Testing ANKR directly at {datetime.now().strftime('%H:%M:%S')}")
    
    async with websockets.connect(WS_URL) as ws:
        # Subscribe to V3 swaps
        sub = {
            "jsonrpc": "2.0", "id": 1, "method": "eth_subscribe",
            "params": ["logs", {"topics": ["0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67"]}]
        }
        await ws.send(json.dumps(sub))
        
        count = 0
        last_time = None
        
        async for message in ws:
            recv_time = time.time()
            data = json.loads(message)
            
            if data.get("id") == 1:
                print(f"✅ Connected to ANKR WebSocket")
                continue
                
            if data.get("params", {}).get("result"):
                count += 1
                dt = datetime.fromtimestamp(recv_time)
                time_str = dt.strftime("%H:%M:%S.%f")
                
                gap = ""
                if last_time:
                    gap_ms = (recv_time - last_time) * 1000
                    gap = f" (+{gap_ms:.1f}ms)"
                
                print(f"⚡ RAW ANKR swap #{count} at {time_str}{gap}")
                last_time = recv_time
                
                if count >= 20:  # Stop after 20 events
                    break

asyncio.run(test_timing())
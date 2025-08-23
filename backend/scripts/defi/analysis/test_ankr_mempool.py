#!/usr/bin/env python3
"""
Test ANKR mempool access to see if we can get pending transactions in real-time.
This will determine if mempool monitoring is feasible with ANKR.
"""

import asyncio
import websockets
import json
import time
from datetime import datetime

# Your ANKR API key
ANKR_API_KEY = "e6fac469b91ea8fd98406aca0820653ae6fe5c2400f44819450f6022dd2792e2"
WS_URL = f"wss://rpc.ankr.com/polygon/ws/{ANKR_API_KEY}"
HTTP_URL = f"https://rpc.ankr.com/polygon/{ANKR_API_KEY}"

async def test_mempool_access():
    print(f"🧪 Testing ANKR mempool access at {datetime.now().strftime('%H:%M:%S')}")
    print(f"📡 WebSocket: {WS_URL}")
    
    try:
        async with websockets.connect(WS_URL) as ws:
            # Subscribe to pending transactions
            mempool_sub = {
                "jsonrpc": "2.0",
                "id": 1,
                "method": "eth_subscribe",
                "params": ["newPendingTransactions"]
            }
            
            await ws.send(json.dumps(mempool_sub))
            print("📤 Sent newPendingTransactions subscription...")
            
            start_time = time.time()
            tx_count = 0
            
            async for message in ws:
                data = json.loads(message)
                
                # Check subscription response
                if data.get("id") == 1:
                    if data.get("result"):
                        print(f"✅ Mempool subscription successful: {data['result']}")
                    elif data.get("error"):
                        print(f"❌ Mempool subscription failed: {data['error']}")
                        return
                    continue
                
                # Check for pending transaction
                if data.get("params", {}).get("result"):
                    tx_count += 1
                    tx_hash = data["params"]["result"]
                    
                    elapsed = time.time() - start_time
                    rate = tx_count / elapsed if elapsed > 0 else 0
                    
                    print(f"⚡ Pending tx #{tx_count}: {tx_hash[:20]}... (rate: {rate:.1f}/sec)")
                    
                    # Stop after 30 transactions or 30 seconds to avoid spam
                    if tx_count >= 30 or elapsed > 30:
                        print(f"\n📊 Results after {elapsed:.1f}s:")
                        print(f"   Pending transactions: {tx_count}")
                        print(f"   Average rate: {rate:.1f} tx/sec")
                        
                        if tx_count > 0:
                            print(f"✅ ANKR supports mempool access!")
                            print(f"🔄 Testing transaction fetching...")
                            
                            # Test fetching a pending transaction
                            import aiohttp
                            async with aiohttp.ClientSession() as session:
                                fetch_req = {
                                    "jsonrpc": "2.0",
                                    "id": 1,
                                    "method": "eth_getTransactionByHash",
                                    "params": [tx_hash]
                                }
                                
                                async with session.post(HTTP_URL, json=fetch_req) as resp:
                                    if resp.status == 200:
                                        result = await resp.json()
                                        if result.get("result"):
                                            tx = result["result"]
                                            print(f"✅ Successfully fetched pending tx:")
                                            print(f"   From: {tx.get('from', 'unknown')}")
                                            print(f"   To: {tx.get('to', 'unknown')}")
                                            print(f"   Value: {tx.get('value', '0x0')}")
                                            print(f"   Data: {tx.get('input', '0x')[:50]}...")
                                        else:
                                            print(f"❌ Failed to fetch tx: {result}")
                                    else:
                                        print(f"❌ HTTP error fetching tx: {resp.status}")
                        else:
                            print(f"❌ No pending transactions received")
                        
                        break
                        
    except websockets.exceptions.ConnectionClosed:
        print("❌ WebSocket connection closed")
    except Exception as e:
        print(f"❌ Error: {e}")

if __name__ == "__main__":
    print("🧪 Testing ANKR Mempool Access")
    print("📊 This will check if ANKR provides real-time pending transactions")
    print("⏱️  Will test for 30 transactions or 30 seconds")
    print()
    
    try:
        asyncio.run(test_mempool_access())
    except KeyboardInterrupt:
        print("\n🛑 Test stopped by user")
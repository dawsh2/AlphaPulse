#!/usr/bin/env python3
"""
Test WebSocket client for the new in-memory streaming server
"""
import asyncio
import websockets
import json

async def test_websocket():
    uri = "ws://localhost:8001/ws/trades"
    print(f"🔗 Connecting to in-memory streaming server at {uri}")
    
    try:
        async with websockets.connect(uri) as websocket:
            print("✅ Connected to In-Memory Real-Time Stream!")
            
            # Listen for messages for 30 seconds
            trade_count = 0
            start_time = asyncio.get_event_loop().time()
            
            while asyncio.get_event_loop().time() - start_time < 30:
                try:
                    message = await asyncio.wait_for(websocket.recv(), timeout=0.5)
                    data = json.loads(message)
                    
                    if data.get('type') == 'connected':
                        print(f"📡 {data.get('message')}")
                    elif data.get('type') == 'trade':
                        trade_count += 1
                        print(f"🎯 Trade #{trade_count}: {data.get('exchange')} {data.get('symbol')} ${data.get('price')} [{data.get('side')}]")
                    else:
                        print(f"📨 Message: {data.get('type', 'unknown')}")
                        
                except asyncio.TimeoutError:
                    # No message received, continue waiting
                    pass
                except Exception as e:
                    print(f"❌ Error: {e}")
                    break
            
            print(f"\n📊 Summary: Received {trade_count} trades in 30 seconds")
            if trade_count > 0:
                print(f"💫 Average rate: {trade_count/30:.1f} trades/second")
                    
    except Exception as e:
        print(f"❌ Connection failed: {e}")

if __name__ == "__main__":
    print("🧪 Testing In-Memory Real-Time WebSocket Stream...")
    print("⚡ Direct exchange → frontend streaming")
    print("💾 Async database persistence\n")
    asyncio.run(test_websocket())
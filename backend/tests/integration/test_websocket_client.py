#!/usr/bin/env python3
"""
Test WebSocket client to verify FastAPI streaming works
"""
import asyncio
import websockets
import json

async def test_websocket():
    uri = "ws://localhost:8000/ws/trades"
    print(f"🔗 Connecting to {uri}")
    
    try:
        async with websockets.connect(uri) as websocket:
            print("✅ Connected to FastAPI WebSocket!")
            
            # Listen for messages for 10 seconds
            for i in range(20):
                try:
                    message = await asyncio.wait_for(websocket.recv(), timeout=1.0)
                    data = json.loads(message)
                    print(f"📨 Received: {data.get('type', 'unknown')} - {data}")
                except asyncio.TimeoutError:
                    print(f"⏳ Waiting for messages... ({i+1}/20)")
                except Exception as e:
                    print(f"❌ Error: {e}")
                    break
                    
    except Exception as e:
        print(f"❌ Connection failed: {e}")

if __name__ == "__main__":
    print("🧪 Testing FastAPI WebSocket connection...")
    asyncio.run(test_websocket())
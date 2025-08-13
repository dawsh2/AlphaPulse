#!/usr/bin/env python3
"""Test Kraken WebSocket connection"""

import asyncio
import websockets
import json
import time

async def test_kraken():
    url = 'wss://ws.kraken.com'
    print(f"Connecting to {url}...")
    
    try:
        async with websockets.connect(url) as websocket:
            print("✓ Connected successfully!")
            
            # Subscribe to BTC/USD orderbook
            subscribe_msg = {
                "event": "subscribe",
                "pair": ["XBT/USD"],
                "subscription": {
                    "name": "book",
                    "depth": 10
                }
            }
            
            print(f"Sending subscription: {json.dumps(subscribe_msg, indent=2)}")
            await websocket.send(json.dumps(subscribe_msg))
            
            # Listen for 5 messages
            for i in range(5):
                message = await asyncio.wait_for(websocket.recv(), timeout=10)
                data = json.loads(message)
                
                if isinstance(data, dict):
                    print(f"\nMessage {i+1} (dict): {data.get('event', 'unknown event')}")
                    if data.get('event') == 'subscriptionStatus':
                        print(f"  Status: {data.get('status')}")
                        print(f"  Pair: {data.get('pair')}")
                elif isinstance(data, list) and len(data) >= 3:
                    print(f"\nMessage {i+1} (array): Channel={data[2] if len(data) > 2 else 'unknown'}")
                    if len(data) > 3:
                        print(f"  Pair: {data[3]}")
                    if len(data) > 1 and isinstance(data[1], dict):
                        if 'as' in data[1] or 'bs' in data[1]:
                            print(f"  Type: Initial snapshot")
                            print(f"  Asks: {len(data[1].get('as', []))} levels")
                            print(f"  Bids: {len(data[1].get('bs', []))} levels")
                        elif 'a' in data[1] or 'b' in data[1]:
                            print(f"  Type: Update")
                            print(f"  Ask updates: {len(data[1].get('a', []))}")
                            print(f"  Bid updates: {len(data[1].get('b', []))}")
            
            print("\n✓ Test completed successfully!")
            
    except asyncio.TimeoutError:
        print("✗ Timeout waiting for message")
    except Exception as e:
        print(f"✗ Error: {e}")

if __name__ == "__main__":
    print("Testing Kraken WebSocket connection...")
    print("-" * 40)
    asyncio.run(test_kraken())
#!/usr/bin/env python3
import asyncio
import websockets
import json

async def test_symbols():
    uri = "ws://localhost:8765/stream"
    
    try:
        async with websockets.connect(uri) as websocket:
            print("Connected to WebSocket")
            
            # Subscribe to all symbols
            subscribe_msg = {
                "msg_type": "subscribe",
                "channels": ["trades", "orderbook"],
                "symbols": ["AAPL", "MSFT", "GOOGL", "TSLA", "SPY", "QQQ", "NVDA", "META", "AMD"]
            }
            await websocket.send(json.dumps(subscribe_msg))
            
            # Collect messages for a few seconds
            alpaca_symbols = set()
            count = 0
            max_messages = 100
            
            while count < max_messages:
                message = await websocket.recv()
                data = json.loads(message)
                
                if data.get('exchange') == 'alpaca':
                    symbol = data.get('symbol', 'UNKNOWN')
                    alpaca_symbols.add(symbol)
                    
                count += 1
                
            print(f"\nAlpaca symbols seen: {sorted(alpaca_symbols)}")
            
            # Check if we're seeing the expected symbols
            expected = {"AAPL", "MSFT", "GOOGL", "TSLA", "SPY", "QQQ", "NVDA", "META", "AMD", "AMZN"}
            if alpaca_symbols & expected:
                print("✅ Stock symbols are being mapped correctly!")
            else:
                print("❌ Still seeing UNKNOWN symbols, not the expected stock symbols")
                
    except Exception as e:
        print(f"Error: {e}")

asyncio.run(test_symbols())
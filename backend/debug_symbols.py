#!/usr/bin/env python3
import asyncio
import websockets
import json
from collections import defaultdict

async def debug_symbols():
    uri = "ws://localhost:8765/stream"
    
    try:
        async with websockets.connect(uri) as websocket:
            print("Connected to WebSocket, collecting symbol IDs...")
            
            # Don't send subscribe, just listen to everything
            
            # Collect messages for a few seconds
            symbol_ids = defaultdict(set)
            count = 0
            max_messages = 200
            
            while count < max_messages:
                message = await websocket.recv()
                data = json.loads(message)
                
                exchange = data.get('exchange', 'unknown')
                symbol = data.get('symbol', 'UNKNOWN')
                symbol_ids[exchange].add(symbol)
                    
                count += 1
                
            print("\nSymbols by exchange:")
            for exchange, symbols in symbol_ids.items():
                print(f"\n{exchange}: {sorted(symbols)}")
                
    except Exception as e:
        print(f"Error: {e}")

asyncio.run(debug_symbols())
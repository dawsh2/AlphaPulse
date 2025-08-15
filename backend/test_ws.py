#!/usr/bin/env python3
import asyncio
import websockets
import json

async def test():
    async with websockets.connect('ws://localhost:8765') as ws:
        print('Connected to ws-bridge')
        msg_counts = {}
        
        async for message in ws:
            try:
                data = json.loads(message)
                msg_type = data.get('msg_type', 'unknown')
                msg_counts[msg_type] = msg_counts.get(msg_type, 0) + 1
                
                if msg_type == 'l2_snapshot':
                    print(f'\nGot L2 snapshot! Hash: {data.get("symbol_hash")}, Bids: {len(data.get("bids", []))}, Asks: {len(data.get("asks", []))}')
                    return
                
                # Print summary every 100 messages
                total = sum(msg_counts.values())
                if total % 100 == 0:
                    print(f'Messages: {msg_counts}')
                    
            except Exception as e:
                print(f'Error: {e}')

asyncio.run(test())

#!/usr/bin/env python3
import asyncio
import websockets
import json

async def test():
    uri = 'ws://localhost:8765'
    print(f'Connecting to {uri}...')
    
    try:
        async with websockets.connect(uri) as ws:
            print('Connected!')
            count = 0
            snapshots_seen = set()
            
            while count < 20:
                msg = await ws.recv()
                data = json.loads(msg)
                msg_type = data.get('msg_type')
                symbol_hash = data.get('symbol_hash')
                
                if msg_type == 'l2_snapshot':
                    symbol = data.get('symbol', 'UNKNOWN')
                    bids = data.get('bids', [])
                    asks = data.get('asks', [])
                    if bids and asks:
                        best_bid = max(float(b['price']) for b in bids)
                        best_ask = min(float(a['price']) for a in asks)
                        spread = best_ask - best_bid
                        print(f'✓ [SNAPSHOT] {symbol}: Bid=${best_bid:.2f}, Ask=${best_ask:.2f}, Spread=${spread:.2f}')
                        if spread < 0:
                            print('  ⚠️  NEGATIVE SPREAD!')
                        snapshots_seen.add(symbol_hash)
                
                elif msg_type == 'l2_delta':
                    if symbol_hash not in snapshots_seen:
                        print(f'  ⚠️  Delta before snapshot for {symbol_hash}')
                    else:
                        symbol = data.get('symbol', 'UNKNOWN')
                        print(f'  → Delta for {symbol} with {len(data.get("updates", []))} updates')
                
                count += 1
            
            print(f'\nTest complete. Snapshots seen: {len(snapshots_seen)}')
    except Exception as e:
        print(f'Error: {e}')

asyncio.run(test())
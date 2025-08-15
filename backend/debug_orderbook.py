#!/usr/bin/env python3
import asyncio
import websockets
import json
from collections import defaultdict

orderbooks = {}
symbol_map = {}
pending_deltas = {}  # Queue deltas until snapshot arrives

async def apply_delta(book, delta_msg):
    """Apply a delta message to an orderbook"""
    for update in delta_msg['updates']:
        price = float(update['price'])
        size = float(update['size'])
        side = book['bids'] if update['side'] == 'bid' else book['asks']
        
        if update['action'] == 'delete' or size == 0:
            side.pop(price, None)
        else:
            side[price] = size

async def process_message(msg):
    global orderbooks, symbol_map, pending_deltas
    
    if msg['msg_type'] == 'symbol_mapping':
        symbol_map[msg['symbol_hash']] = msg['symbol']
        print(f"Symbol mapping: {msg['symbol_hash']} -> {msg['symbol']}")
        
    elif msg['msg_type'] == 'l2_snapshot':
        symbol_hash = msg['symbol_hash']
        symbol = msg.get('symbol') or symbol_map.get(symbol_hash, f"UNKNOWN_{symbol_hash}")
        
        # Initialize orderbook from snapshot
        orderbooks[symbol_hash] = {
            'symbol': symbol,
            'bids': {float(level['price']): float(level['size']) for level in msg['bids']},
            'asks': {float(level['price']): float(level['size']) for level in msg['asks']},
            'sequence': msg.get('sequence', 0)
        }
        
        print(f"\n✓ Received snapshot for {symbol} (hash: {symbol_hash})")
        print(f"  {len(orderbooks[symbol_hash]['bids'])} bids, {len(orderbooks[symbol_hash]['asks'])} asks")
        
        # Process any queued deltas for this symbol
        if symbol_hash in pending_deltas:
            print(f"  → Applying {len(pending_deltas[symbol_hash])} queued deltas...")
            for delta in pending_deltas[symbol_hash]:
                await apply_delta(orderbooks[symbol_hash], delta)
            del pending_deltas[symbol_hash]
        
        # Check spread
        if orderbooks[symbol_hash]['bids'] and orderbooks[symbol_hash]['asks']:
            best_bid = max(orderbooks[symbol_hash]['bids'].keys())
            best_ask = min(orderbooks[symbol_hash]['asks'].keys())
            spread = best_ask - best_bid
            print(f"  Best Bid=${best_bid:.2f}, Best Ask=${best_ask:.2f}, Spread=${spread:.2f}")
            if spread < 0:
                print(f"  ⚠️  NEGATIVE SPREAD AFTER SNAPSHOT!")
                
    elif msg['msg_type'] == 'l2_delta':
        symbol_hash = msg['symbol_hash']
        if symbol_hash not in orderbooks:
            # Queue delta until snapshot arrives
            if symbol_hash not in pending_deltas:
                pending_deltas[symbol_hash] = []
                print(f"  ⚠️  Queuing deltas for {symbol_hash} (no snapshot yet)")
            pending_deltas[symbol_hash].append(msg)
            return
            
        book = orderbooks[symbol_hash]
        symbol = book['symbol']
        
        # Apply updates
        await apply_delta(book, msg)
        
        # Check spread after delta
        if book['bids'] and book['asks']:
            best_bid = max(book['bids'].keys())
            best_ask = min(book['asks'].keys())
            spread = best_ask - best_bid
            
            if spread < 0:
                print(f"\n[DELTA] {symbol}: Best Bid=${best_bid:.2f}, Best Ask=${best_ask:.2f}, Spread=${spread:.2f}")
                print(f"  ⚠️  NEGATIVE SPREAD AFTER DELTA!")
                print(f"  Updates applied: {msg['updates'][:3]}...")  # Show first 3 updates
                
                # Show top 5 levels
                top_bids = sorted(book['bids'].items(), key=lambda x: -x[0])[:5]
                top_asks = sorted(book['asks'].items(), key=lambda x: x[0])[:5]
                print(f"  Top Bids: {top_bids}")
                print(f"  Top Asks: {top_asks}")

async def main():
    uri = "ws://localhost:8765"
    print(f"Connecting to {uri}...")
    
    async with websockets.connect(uri) as websocket:
        print("Connected! Monitoring orderbook...")
        
        while True:
            try:
                message = await websocket.recv()
                data = json.loads(message)
                await process_message(data)
            except Exception as e:
                print(f"Error: {e}")
                break

if __name__ == "__main__":
    asyncio.run(main())
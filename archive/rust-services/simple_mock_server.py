#!/usr/bin/env python3
"""Simple mock WebSocket server to test the dashboard"""

import asyncio
import json
import time
import websockets
from datetime import datetime

async def handle_client(websocket, path):
    """Handle WebSocket connection from dashboard"""
    print(f"🔌 New client connected from {websocket.remote_address}")
    trade_id = 0
    
    try:
        # Send initial connection confirmation
        await websocket.send(json.dumps({
            "type": "subscribed",
            "message": "Connected to mock data server"
        }))
        
        while True:
            # Generate batch of mock trades
            for _ in range(5):
                trade_id += 1
                
                # Create trade message
                trade = {
                    "type": "Trade",
                    "data": {
                        "timestamp": int(time.time()),
                        "symbol": "BTC-USD" if trade_id % 2 == 0 else "ETH-USD",
                        "exchange": ["coinbase", "kraken", "binance"][trade_id % 3],
                        "price": 50000 + (trade_id % 1000) if trade_id % 2 == 0 else 3000 + (trade_id % 100),
                        "volume": 0.1 + (trade_id % 10) * 0.01,
                        "side": "buy" if trade_id % 3 == 0 else "sell",
                        "trade_id": f"mock_{trade_id}"
                    }
                }
                
                await websocket.send(json.dumps(trade))
            
            # Send system stats every 50 trades
            if trade_id % 50 == 0:
                stats = {
                    "type": "SystemStats",
                    "data": {
                        "latency_us": 250,
                        "active_clients": 1,
                        "trades_processed": trade_id,
                        "deltas_processed": 0,
                        "active_feeds": 1,
                        "trade_feeds": 1,
                        "delta_feeds": 0
                    }
                }
                await websocket.send(json.dumps(stats))
                print(f"📊 Sent {trade_id} trades")
            
            # Control rate - 50 trades/sec
            await asyncio.sleep(0.1)
            
    except websockets.exceptions.ConnectionClosed:
        print(f"🔌 Client disconnected")
    except Exception as e:
        print(f"❌ Error: {e}")

async def main():
    """Start the WebSocket server"""
    print("🚀 Starting mock WebSocket server on ws://localhost:3001/ws")
    async with websockets.serve(handle_client, "localhost", 3001):
        print("✅ Server listening on port 3001")
        await asyncio.Future()  # run forever

if __name__ == "__main__":
    asyncio.run(main())
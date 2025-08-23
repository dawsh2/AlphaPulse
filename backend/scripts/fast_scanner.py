#!/usr/bin/env python3
"""Fast arbitrage scanner using WebSocket stream and Redis cache"""

import asyncio
import websockets
import json
import redis
from web3 import Web3
from collections import defaultdict
import time

# Connect to Redis
r = redis.Redis(host='localhost', port=6379, decode_responses=True)

# Web3 connection
w3 = Web3(Web3.HTTPProvider('https://polygon.publicnode.com'))

# Token addresses
USDC = '0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174'
WPOL = '0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270'

class FastScanner:
    def __init__(self):
        self.pools = {}  # pool_address -> {token0, token1, reserve0, reserve1, fee, type}
        self.opportunities = []
        
    async def connect_websocket(self):
        """Connect to Polygon WebSocket and monitor Sync events"""
        ws_url = "wss://polygon.publicnode.com"
        
        # Subscribe to Sync events (emitted on every swap)
        subscription = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_subscribe",
            "params": ["logs", {
                "topics": [
                    "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1"  # Sync
                ]
            }]
        }
        
        async with websockets.connect(ws_url) as ws:
            await ws.send(json.dumps(subscription))
            print("📡 Connected to WebSocket")
            
            while True:
                try:
                    message = await ws.recv()
                    data = json.loads(message)
                    
                    if 'params' in data and 'result' in data['params']:
                        await self.process_sync_event(data['params']['result'])
                        
                except Exception as e:
                    print(f"WebSocket error: {e}")
                    await asyncio.sleep(1)
                    
    async def process_sync_event(self, event):
        """Process Sync event and update pool state"""
        pool_address = event['address'].lower()
        
        # Decode reserves from event data
        data = event['data']
        if len(data) >= 130:  # 0x + 64 chars for reserve0 + 64 chars for reserve1
            reserve0 = int(data[2:66], 16)
            reserve1 = int(data[66:130], 16)
            
            # Cache in Redis for fast access
            pool_key = f"pool:{pool_address}"
            pool_data = {
                'reserve0': reserve0,
                'reserve1': reserve1,
                'block': int(event['blockNumber'], 16),
                'timestamp': int(time.time())
            }
            
            # Store with 60 second expiry
            r.setex(pool_key, 60, json.dumps(pool_data))
            
            # Check for arbitrage opportunities
            self.check_arbitrage_fast(pool_address, reserve0, reserve1)
            
    def check_arbitrage_fast(self, pool, reserve0, reserve1):
        """Fast arbitrage check using cached data"""
        # Get all USDC/WPOL pools from cache
        pattern = "pool:*"
        
        for key in r.scan_iter(match=pattern):
            other_pool = key.replace("pool:", "")
            if other_pool == pool:
                continue
                
            try:
                data = json.loads(r.get(key))
                other_r0 = data['reserve0']
                other_r1 = data['reserve1']
                
                # Calculate price difference (simplified)
                price1 = reserve1 / reserve0 if reserve0 > 0 else 0
                price2 = other_r1 / other_r0 if other_r0 > 0 else 0
                
                if price1 > 0 and price2 > 0:
                    spread = abs(price1 - price2) / min(price1, price2) * 100
                    
                    if spread > 0.1:  # 0.1% minimum spread
                        # Calculate optimal trade size based on reserves
                        # Using x*y=k formula
                        optimal_size = self.calculate_optimal_size(
                            reserve0, reserve1, other_r0, other_r1
                        )
                        
                        if optimal_size > 0:
                            profit = optimal_size * spread / 100
                            
                            opportunity = {
                                'buy': pool if price1 < price2 else other_pool,
                                'sell': other_pool if price1 < price2 else pool,
                                'spread': spread,
                                'size': optimal_size,
                                'profit': profit,
                                'timestamp': time.time()
                            }
                            
                            # Store in Redis
                            r.zadd('opportunities', {json.dumps(opportunity): profit})
                            
                            if profit > 0.01:  # $0.01 minimum
                                print(f"\n💰 OPPORTUNITY: ${profit:.4f} profit")
                                print(f"   Spread: {spread:.3f}%")
                                print(f"   Size: ${optimal_size:.2f}")
                                print(f"   Buy:  {opportunity['buy'][:10]}...")
                                print(f"   Sell: {opportunity['sell'][:10]}...")
                                
            except Exception as e:
                pass
                
    def calculate_optimal_size(self, r0_buy, r1_buy, r0_sell, r1_sell):
        """Calculate optimal trade size based on pool reserves"""
        # Simplified calculation - in reality this needs more complex math
        # considering price impact on both pools
        
        # Max size is 1% of smaller pool's liquidity
        buy_liquidity = (r0_buy * r1_buy) ** 0.5
        sell_liquidity = (r0_sell * r1_sell) ** 0.5
        
        max_size = min(buy_liquidity, sell_liquidity) * 0.01
        
        # Convert to USD (assuming reserve0 is USDC with 6 decimals)
        max_size_usd = max_size / 10**6
        
        # Cap at reasonable size
        return min(max_size_usd, 1000)
        
    async def run_scanner(self):
        """Run the fast scanner"""
        print("⚡ Fast Arbitrage Scanner")
        print("📊 Using WebSocket + Redis cache")
        print("🚀 Starting...\n")
        
        # Start WebSocket monitoring
        await self.connect_websocket()
        
async def main():
    scanner = FastScanner()
    await scanner.run_scanner()
    
if __name__ == "__main__":
    asyncio.run(main())
#!/usr/bin/env python3
"""
Test WebSocket latency for various free Polygon RPC providers
"""

import asyncio
import json
import time
import statistics
from typing import List, Dict, Tuple
import websockets
from websockets.exceptions import WebSocketException

# Free Polygon WebSocket endpoints to test
ENDPOINTS = [
    ("Polygon Public RPC", "wss://polygon-rpc.com"),
    ("Polygon Bor PublicNode", "wss://polygon-bor.publicnode.com"),
    ("Ankr Free Tier", "wss://rpc.ankr.com/polygon/ws"),
    ("GetBlock.io Free", "wss://matic.getblock.io/mainnet/ws/"),
    ("BlastAPI Free", "wss://polygon-mainnet.public.blastapi.io"),
    ("Polygon Mumbai (testnet)", "wss://polygon-mumbai-bor.publicnode.com"),
]

class WebSocketLatencyTester:
    def __init__(self):
        self.results = {}
        
    async def test_endpoint(self, name: str, url: str, num_tests: int = 5) -> Dict:
        """Test a single WebSocket endpoint"""
        print(f"\n📡 Testing {name}: {url}")
        
        latencies = []
        connect_times = []
        errors = 0
        
        for i in range(num_tests):
            try:
                # Measure connection time
                connect_start = time.perf_counter()
                async with websockets.connect(url, timeout=10) as ws:
                    connect_time = (time.perf_counter() - connect_start) * 1000
                    connect_times.append(connect_time)
                    
                    # Subscribe to latest block
                    subscribe_msg = {
                        "jsonrpc": "2.0",
                        "id": 1,
                        "method": "eth_subscribe",
                        "params": ["newHeads"]
                    }
                    
                    # Measure round-trip time
                    start_time = time.perf_counter()
                    await ws.send(json.dumps(subscribe_msg))
                    
                    # Wait for subscription confirmation
                    response = await asyncio.wait_for(ws.recv(), timeout=5)
                    latency = (time.perf_counter() - start_time) * 1000
                    latencies.append(latency)
                    
                    data = json.loads(response)
                    if "result" in data:
                        print(f"  ✅ Test {i+1}: {latency:.2f}ms (connect: {connect_time:.2f}ms)")
                    else:
                        print(f"  ⚠️  Test {i+1}: Unexpected response")
                        errors += 1
                        
            except asyncio.TimeoutError:
                print(f"  ❌ Test {i+1}: Timeout")
                errors += 1
            except WebSocketException as e:
                print(f"  ❌ Test {i+1}: WebSocket error: {e}")
                errors += 1
            except Exception as e:
                print(f"  ❌ Test {i+1}: Error: {e}")
                errors += 1
            
            # Small delay between tests
            await asyncio.sleep(0.5)
        
        if latencies:
            return {
                "name": name,
                "url": url,
                "avg_latency": statistics.mean(latencies),
                "min_latency": min(latencies),
                "max_latency": max(latencies),
                "median_latency": statistics.median(latencies),
                "avg_connect_time": statistics.mean(connect_times) if connect_times else None,
                "success_rate": (num_tests - errors) / num_tests * 100,
                "errors": errors
            }
        else:
            return {
                "name": name,
                "url": url,
                "avg_latency": None,
                "success_rate": 0,
                "errors": errors
            }
    
    async def test_block_subscription(self, name: str, url: str) -> Dict:
        """Test receiving actual block updates"""
        print(f"\n📦 Testing block updates for {name}")
        
        try:
            async with websockets.connect(url, timeout=10) as ws:
                # Subscribe to new blocks
                subscribe_msg = {
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "eth_subscribe",
                    "params": ["newHeads"]
                }
                
                await ws.send(json.dumps(subscribe_msg))
                
                # Wait for subscription confirmation
                response = await asyncio.wait_for(ws.recv(), timeout=5)
                data = json.loads(response)
                
                if "result" in data:
                    subscription_id = data["result"]
                    print(f"  ✅ Subscribed with ID: {subscription_id}")
                    
                    # Wait for first block
                    print("  ⏳ Waiting for new block (up to 30s)...")
                    block_start = time.perf_counter()
                    
                    block_msg = await asyncio.wait_for(ws.recv(), timeout=30)
                    block_time = (time.perf_counter() - block_start) * 1000
                    
                    block_data = json.loads(block_msg)
                    if "params" in block_data:
                        block_number = int(block_data["params"]["result"]["number"], 16)
                        print(f"  ✅ Received block #{block_number} in {block_time:.2f}ms")
                        return {"block_received": True, "block_time": block_time}
                    
        except Exception as e:
            print(f"  ❌ Error: {e}")
            
        return {"block_received": False, "block_time": None}
    
    async def run_all_tests(self):
        """Run latency tests on all endpoints"""
        print("🚀 POLYGON WEBSOCKET LATENCY TEST")
        print("=" * 60)
        print("Testing free WebSocket endpoints for lowest latency...")
        
        # Test basic latency
        for name, url in ENDPOINTS:
            result = await self.test_endpoint(name, url)
            self.results[name] = result
        
        # Print summary
        print("\n" + "=" * 60)
        print("📊 LATENCY TEST RESULTS")
        print("=" * 60)
        
        # Sort by average latency
        sorted_results = sorted(
            [(k, v) for k, v in self.results.items() if v["avg_latency"] is not None],
            key=lambda x: x[1]["avg_latency"]
        )
        
        if sorted_results:
            print("\n🏆 RANKINGS (by average latency):")
            for i, (name, result) in enumerate(sorted_results, 1):
                print(f"\n{i}. {name}")
                print(f"   Average Latency: {result['avg_latency']:.2f}ms")
                print(f"   Min/Max: {result['min_latency']:.2f}ms / {result['max_latency']:.2f}ms")
                print(f"   Connect Time: {result['avg_connect_time']:.2f}ms")
                print(f"   Success Rate: {result['success_rate']:.0f}%")
                
            # Winner
            winner_name, winner_result = sorted_results[0]
            print("\n" + "=" * 60)
            print(f"🥇 FASTEST ENDPOINT: {winner_name}")
            print(f"   URL: {winner_result['url']}")
            print(f"   Latency: {winner_result['avg_latency']:.2f}ms")
            print("=" * 60)
            
            # Test block subscription on winner
            print("\nTesting real-time block updates on fastest endpoint...")
            block_result = await self.test_block_subscription(winner_name, winner_result['url'])
            
            if block_result["block_received"]:
                print(f"\n✅ Successfully receiving real-time blocks!")
                print(f"   Block latency: {block_result['block_time']:.2f}ms")
            
            return winner_result['url']
        else:
            print("\n❌ All endpoints failed!")
            return None

async def main():
    tester = WebSocketLatencyTester()
    best_endpoint = await tester.run_all_tests()
    
    if best_endpoint:
        print(f"\n💡 RECOMMENDATION:")
        print(f"   Update polygon.rs to use: {best_endpoint}")
        print(f"   This endpoint has the lowest latency and is free!")
    
if __name__ == "__main__":
    asyncio.run(main())
#!/usr/bin/env python3
"""
Expanded test for free Polygon WebSocket and RPC endpoints
Tests both WebSocket and HTTP latency for more providers
"""

import asyncio
import json
import time
import statistics
import aiohttp
from typing import List, Dict, Tuple
import websockets
from websockets.exceptions import WebSocketException

# Expanded list of free Polygon endpoints
WEBSOCKET_ENDPOINTS = [
    # Public nodes
    ("Polygon Bor PublicNode", "wss://polygon-bor.publicnode.com"),
    ("Polygon RPC Official", "wss://polygon-rpc.com"),
    ("Polygon RPC Alternative", "wss://rpc-mainnet.matic.network"),
    ("Polygon RPC Matic", "wss://rpc-mainnet.maticvigil.com"),
    
    # Ankr variants
    ("Ankr Public", "wss://rpc.ankr.com/polygon/ws"),
    ("Ankr Direct", "wss://polygon.ankr.com"),
    
    # Blast variants
    ("BlastAPI Public", "wss://polygon-mainnet.public.blastapi.io"),
    ("Blast RPC", "wss://polygon.public-rpc.com"),
    
    # Other free providers
    ("Chainstack Public", "wss://polygon-mainnet.chainstackapi.com"),
    ("Moralis Speedy", "wss://speedy-nodes-nyc.moralis.io/polygon/mainnet/ws"),
    ("QuickNode Free", "wss://polygon-mainnet.quicknode.pro"),
    ("Alchemy Free", "wss://polygon-mainnet.g.alchemy.com/v2/demo"),
    ("Infura Free", "wss://polygon-mainnet.infura.io/ws/v3/9aa3d95b3bc440fa88ea12eaa4456161"),
    
    # Alternative endpoints
    ("MaticVigil", "wss://rpc-mainnet.maticvigil.com/ws"),
    ("Polygon Archive", "wss://polygon-archive.allthatnode.com"),
    ("1RPC", "wss://1rpc.io/matic"),
    ("Pokt Network", "wss://poly-rpc.gateway.pokt.network"),
    ("Dwellir", "wss://polygon-rpc.dwellir.com"),
    
    # Community nodes
    ("0xPolygon", "wss://polygon.llamarpc.com"),
    ("Polygon API", "wss://polygon.api.onfinality.io/public-ws"),
    ("BlockPI", "wss://polygon.blockpi.network/v1/ws/public"),
    ("NodeReal", "wss://polygon-mainnet.nodereal.io/ws/v1/public"),
    ("GetBlock", "wss://go.getblock.io/polygon"),
    
    # Decentralized providers
    ("Pocket Portal", "wss://polygon-mainnet.gateway.pokt.network/v1/ws"),
    ("Grove City", "wss://polygon-mainnet.rpc.grove.city/v1/ws"),
]

# HTTP endpoints for comparison
HTTP_ENDPOINTS = [
    ("Polygon Bor PublicNode", "https://polygon-bor.publicnode.com"),
    ("Polygon RPC Official", "https://polygon-rpc.com"),
    ("Ankr Public", "https://rpc.ankr.com/polygon"),
    ("BlastAPI Public", "https://polygon-mainnet.public.blastapi.io"),
    ("1RPC", "https://1rpc.io/matic"),
    ("Chainstack", "https://polygon-mainnet.chainstackapi.com"),
    ("LlamaRPC", "https://polygon.llamarpc.com"),
    ("BlockPI", "https://polygon.blockpi.network/v1/rpc/public"),
    ("Alchemy Demo", "https://polygon-mainnet.g.alchemy.com/v2/demo"),
]

class ExpandedLatencyTester:
    def __init__(self):
        self.ws_results = {}
        self.http_results = {}
        
    async def test_websocket_endpoint(self, name: str, url: str, num_tests: int = 3) -> Dict:
        """Test a WebSocket endpoint with reduced tests for speed"""
        print(f"  Testing {name}...", end=" ")
        
        latencies = []
        connect_times = []
        errors = 0
        
        for i in range(num_tests):
            try:
                connect_start = time.perf_counter()
                timeout = aiohttp.ClientTimeout(total=5)
                
                async with websockets.connect(url, open_timeout=5, close_timeout=5) as ws:
                    connect_time = (time.perf_counter() - connect_start) * 1000
                    connect_times.append(connect_time)
                    
                    # Test eth_blockNumber for speed
                    request = {
                        "jsonrpc": "2.0",
                        "id": 1,
                        "method": "eth_blockNumber",
                        "params": []
                    }
                    
                    start_time = time.perf_counter()
                    await ws.send(json.dumps(request))
                    response = await asyncio.wait_for(ws.recv(), timeout=3)
                    latency = (time.perf_counter() - start_time) * 1000
                    latencies.append(latency)
                    
            except (asyncio.TimeoutError, WebSocketException, ConnectionError) as e:
                errors += 1
            except Exception as e:
                errors += 1
            
            await asyncio.sleep(0.1)  # Small delay between tests
        
        if latencies:
            avg_latency = statistics.mean(latencies)
            print(f"✅ {avg_latency:.0f}ms")
            return {
                "name": name,
                "url": url,
                "avg_latency": avg_latency,
                "min_latency": min(latencies),
                "max_latency": max(latencies),
                "avg_connect_time": statistics.mean(connect_times) if connect_times else None,
                "success_rate": (num_tests - errors) / num_tests * 100,
                "errors": errors
            }
        else:
            print(f"❌ Failed")
            return {
                "name": name,
                "url": url,
                "avg_latency": None,
                "success_rate": 0,
                "errors": errors
            }
    
    async def test_http_endpoint(self, name: str, url: str, num_tests: int = 3) -> Dict:
        """Test HTTP RPC endpoint latency"""
        print(f"  Testing {name}...", end=" ")
        
        latencies = []
        errors = 0
        
        for i in range(num_tests):
            try:
                request = {
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "eth_blockNumber",
                    "params": []
                }
                
                timeout = aiohttp.ClientTimeout(total=5)
                async with aiohttp.ClientSession(timeout=timeout) as session:
                    start_time = time.perf_counter()
                    async with session.post(url, json=request) as response:
                        if response.status == 200:
                            await response.json()
                            latency = (time.perf_counter() - start_time) * 1000
                            latencies.append(latency)
                        else:
                            errors += 1
                            
            except Exception as e:
                errors += 1
            
            await asyncio.sleep(0.1)
        
        if latencies:
            avg_latency = statistics.mean(latencies)
            print(f"✅ {avg_latency:.0f}ms")
            return {
                "name": name,
                "url": url,
                "avg_latency": avg_latency,
                "min_latency": min(latencies),
                "max_latency": max(latencies),
                "success_rate": (num_tests - errors) / num_tests * 100,
                "errors": errors
            }
        else:
            print(f"❌ Failed")
            return {
                "name": name,
                "url": url,
                "avg_latency": None,
                "success_rate": 0,
                "errors": errors
            }
    
    async def test_swap_event_subscription(self, name: str, url: str) -> bool:
        """Test if endpoint supports swap event subscriptions"""
        try:
            async with websockets.connect(url, open_timeout=5) as ws:
                # Subscribe to swap events
                subscription = {
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "eth_subscribe",
                    "params": [
                        "logs",
                        {
                            "topics": [
                                "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822"
                            ]
                        }
                    ]
                }
                
                await ws.send(json.dumps(subscription))
                response = await asyncio.wait_for(ws.recv(), timeout=5)
                data = json.loads(response)
                
                # Check if subscription was successful
                if "result" in data:
                    return True
                    
        except Exception:
            pass
        
        return False
    
    async def run_all_tests(self):
        """Run comprehensive latency tests"""
        print("🚀 EXPANDED POLYGON ENDPOINT LATENCY TEST")
        print("=" * 70)
        
        # Test WebSocket endpoints
        print("\n📡 Testing WebSocket Endpoints...")
        print("-" * 70)
        
        tasks = []
        for name, url in WEBSOCKET_ENDPOINTS:
            tasks.append(self.test_websocket_endpoint(name, url))
        
        # Run tests in batches to avoid overwhelming
        batch_size = 5
        for i in range(0, len(tasks), batch_size):
            batch = tasks[i:i+batch_size]
            results = await asyncio.gather(*batch, return_exceptions=True)
            for result in results:
                if isinstance(result, dict):
                    self.ws_results[result["name"]] = result
        
        # Test HTTP endpoints for comparison
        print("\n🌐 Testing HTTP RPC Endpoints...")
        print("-" * 70)
        
        http_tasks = []
        for name, url in HTTP_ENDPOINTS:
            http_tasks.append(self.test_http_endpoint(name, url))
        
        for i in range(0, len(http_tasks), batch_size):
            batch = http_tasks[i:i+batch_size]
            results = await asyncio.gather(*batch, return_exceptions=True)
            for result in results:
                if isinstance(result, dict):
                    self.http_results[result["name"]] = result
        
        # Print results
        self.print_results()
        
        # Test swap event support on top performers
        await self.test_event_support()
    
    def print_results(self):
        """Print formatted results"""
        print("\n" + "=" * 70)
        print("📊 WEBSOCKET LATENCY RESULTS")
        print("=" * 70)
        
        # Sort by latency
        sorted_ws = sorted(
            [(k, v) for k, v in self.ws_results.items() if v["avg_latency"] is not None],
            key=lambda x: x[1]["avg_latency"]
        )
        
        if sorted_ws:
            print("\n🏆 TOP 10 FASTEST WEBSOCKET ENDPOINTS:")
            for i, (name, result) in enumerate(sorted_ws[:10], 1):
                print(f"\n{i}. {name}")
                print(f"   Latency: {result['avg_latency']:.1f}ms (min: {result['min_latency']:.1f}ms)")
                print(f"   Success: {result['success_rate']:.0f}%")
                print(f"   URL: {result['url']}")
        
        print("\n" + "=" * 70)
        print("📊 HTTP RPC LATENCY RESULTS")
        print("=" * 70)
        
        sorted_http = sorted(
            [(k, v) for k, v in self.http_results.items() if v["avg_latency"] is not None],
            key=lambda x: x[1]["avg_latency"]
        )
        
        if sorted_http:
            print("\n🏆 TOP 5 FASTEST HTTP ENDPOINTS:")
            for i, (name, result) in enumerate(sorted_http[:5], 1):
                print(f"\n{i}. {name}")
                print(f"   Latency: {result['avg_latency']:.1f}ms")
                print(f"   URL: {result['url']}")
    
    async def test_event_support(self):
        """Test swap event support on top endpoints"""
        print("\n" + "=" * 70)
        print("🔄 TESTING SWAP EVENT SUPPORT")
        print("=" * 70)
        
        # Get top 5 WebSocket endpoints
        sorted_ws = sorted(
            [(k, v) for k, v in self.ws_results.items() if v["avg_latency"] is not None],
            key=lambda x: x[1]["avg_latency"]
        )[:5]
        
        print("\nTesting DEX swap event subscriptions on fastest endpoints...")
        
        for name, result in sorted_ws:
            print(f"\n{name}:", end=" ")
            supports_events = await self.test_swap_event_subscription(name, result["url"])
            if supports_events:
                print("✅ Supports swap events!")
            else:
                print("❌ No swap event support")
        
        # Final recommendation
        print("\n" + "=" * 70)
        print("💡 RECOMMENDATIONS")
        print("=" * 70)
        
        # Find best endpoint with event support
        for name, result in sorted_ws:
            supports_events = await self.test_swap_event_subscription(name, result["url"])
            if supports_events:
                print(f"\n🥇 BEST ENDPOINT FOR DEX DATA:")
                print(f"   Name: {name}")
                print(f"   URL: {result['url']}")
                print(f"   Latency: {result['avg_latency']:.1f}ms")
                print(f"   ✅ Supports real-time swap events")
                print(f"\n   Update polygon.rs with this URL for best performance!")
                break

async def main():
    tester = ExpandedLatencyTester()
    await tester.run_all_tests()

if __name__ == "__main__":
    asyncio.run(main())
#!/usr/bin/env python3
"""
POL Price Pipeline E2E Test
==========================

This test traces data flow through the entire pipeline:
1. Collector receives swap data from blockchain
2. Collector calculates price and creates symbol hash  
3. Collector sends via Unix socket to relay
4. Relay forwards to WS bridge
5. WS bridge maps hash back to symbol and outputs price

Goal: Verify A --> B --> A mapping integrity and find root cause of POL price issue.
"""

import asyncio
import json
import time
import websockets
import subprocess
import signal
import sys
from pathlib import Path

class POLPipelineTest:
    def __init__(self):
        self.processes = []
        self.collected_data = {
            'raw_prices': [],
            'symbol_mappings': [],
            'websocket_data': []
        }
        
    async def setup_clean_environment(self):
        """Kill existing processes and clean sockets"""
        print("🧹 Cleaning environment...")
        subprocess.run(["pkill", "-f", "exchange_collector|relay_server|ws_bridge"], 
                      capture_output=True)
        subprocess.run(["rm", "-rf", "/tmp/alphapulse"], capture_output=True)
        subprocess.run(["mkdir", "-p", "/tmp/alphapulse"], capture_output=True)
        await asyncio.sleep(2)
        
    async def start_relay_server(self):
        """Start relay server with debug logging"""
        print("🚀 Starting relay server...")
        env = {"RUST_LOG": "debug"}
        proc = await asyncio.create_subprocess_exec(
            "./target/release/relay-server",
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            env=env,
            cwd="/Users/daws/alphapulse/backend"
        )
        self.processes.append(proc)
        await asyncio.sleep(3)  # Wait for socket creation
        return proc
        
    async def start_polygon_collector(self):
        """Start Polygon collector with debug logging"""
        print("🔗 Starting Polygon collector...")
        env = {"RUST_LOG": "debug", "EXCHANGE_NAME": "polygon"}
        proc = await asyncio.create_subprocess_exec(
            "./target/release/exchange-collector",
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            env=env,
            cwd="/Users/daws/alphapulse/backend"
        )
        self.processes.append(proc)
        await asyncio.sleep(2)
        return proc
        
    async def start_ws_bridge(self):
        """Start WebSocket bridge with debug logging"""
        print("🌉 Starting WS Bridge...")
        env = {"RUST_LOG": "debug"}
        proc = await asyncio.create_subprocess_exec(
            "./target/release/ws-bridge",
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            env=env,
            cwd="/Users/daws/alphapulse/backend"
        )
        self.processes.append(proc)
        await asyncio.sleep(2)
        return proc
        
    async def monitor_logs(self, proc, name):
        """Monitor process logs and extract relevant data"""
        print(f"📊 Monitoring {name} logs...")
        
        async def read_stream(stream, stream_name):
            while True:
                try:
                    line = await stream.readline()
                    if not line:
                        break
                    line_str = line.decode('utf-8').strip()
                    
                    # Extract POL-related data
                    if "POL" in line_str:
                        if "🔍 PRICE CALCULATION DEBUG" in line_str:
                            print(f"[{name}] {line_str}")
                            
                        elif "raw_price calculation" in line_str:
                            # Extract raw price
                            if "$" in line_str:
                                price = line_str.split("$")[1].split()[0]
                                self.collected_data['raw_prices'].append({
                                    'source': name,
                                    'price': price,
                                    'timestamp': time.time()
                                })
                                
                        elif "SymbolMapping" in line_str and "POL" in line_str:
                            print(f"[{name}] Symbol Mapping: {line_str}")
                            self.collected_data['symbol_mappings'].append({
                                'source': name,
                                'data': line_str,
                                'timestamp': time.time()
                            })
                            
                except Exception as e:
                    print(f"Error reading {name} {stream_name}: {e}")
                    break
                    
        # Monitor both stdout and stderr
        await asyncio.gather(
            read_stream(proc.stdout, "stdout"),
            read_stream(proc.stderr, "stderr")
        )
        
    async def connect_websocket(self):
        """Connect to WebSocket and monitor POL data"""
        print("🔌 Connecting to WebSocket...")
        try:
            async with websockets.connect("ws://localhost:8765/stream") as websocket:
                print("✅ WebSocket connected")
                
                start_time = time.time()
                pol_count = 0
                
                while time.time() - start_time < 60:  # Monitor for 1 minute
                    try:
                        message = await asyncio.wait_for(websocket.recv(), timeout=5.0)
                        data = json.loads(message)
                        
                        # Look for POL trades
                        if data.get('symbol', '').find('POL') != -1:
                            pol_count += 1
                            price = data.get('price', 0)
                            symbol = data.get('symbol', '')
                            
                            print(f"📈 POL Trade #{pol_count}: {symbol} @ ${price}")
                            
                            self.collected_data['websocket_data'].append({
                                'symbol': symbol,
                                'price': price,
                                'data': data,
                                'timestamp': time.time()
                            })
                            
                    except asyncio.TimeoutError:
                        continue
                    except Exception as e:
                        print(f"WebSocket error: {e}")
                        break
                        
        except Exception as e:
            print(f"Failed to connect to WebSocket: {e}")
            
    def analyze_results(self):
        """Analyze collected data to find the root cause"""
        print("\n" + "="*50)
        print("📊 PIPELINE ANALYSIS RESULTS")
        print("="*50)
        
        print(f"\n📥 Raw Prices Collected: {len(self.collected_data['raw_prices'])}")
        for entry in self.collected_data['raw_prices'][:5]:  # Show first 5
            print(f"  {entry['source']}: ${entry['price']}")
            
        print(f"\n🔗 Symbol Mappings: {len(self.collected_data['symbol_mappings'])}")
        for entry in self.collected_data['symbol_mappings'][:5]:
            print(f"  {entry['source']}: {entry['data'][:100]}...")
            
        print(f"\n📡 WebSocket Data: {len(self.collected_data['websocket_data'])}")
        for entry in self.collected_data['websocket_data'][:5]:
            print(f"  {entry['symbol']}: ${entry['price']}")
            
        # Compare input vs output prices
        if self.collected_data['raw_prices'] and self.collected_data['websocket_data']:
            raw_price = float(self.collected_data['raw_prices'][0]['price'])
            ws_price = float(self.collected_data['websocket_data'][0]['price'])
            
            print(f"\n🔍 PRICE COMPARISON:")
            print(f"  Raw calculated price: ${raw_price:.6f}")
            print(f"  WebSocket output price: ${ws_price:.6f}")
            
            if abs(raw_price - ws_price) < 0.000001:
                print("  ✅ Prices match - issue is in calculation, not pipeline")
            else:
                ratio = ws_price / raw_price if raw_price > 0 else 0
                print(f"  ❌ Price mismatch - ratio: {ratio:.2f}x")
                print("  🔍 Issue is in the pipeline transformation")
                
    async def cleanup(self):
        """Clean up processes"""
        print("🛑 Cleaning up...")
        for proc in self.processes:
            try:
                proc.terminate()
                await asyncio.wait_for(proc.wait(), timeout=5.0)
            except:
                proc.kill()
                
    async def run_test(self):
        """Run the complete E2E test"""
        try:
            await self.setup_clean_environment()
            
            # Start services
            relay = await self.start_relay_server()
            collector = await self.start_polygon_collector()
            bridge = await self.start_ws_bridge()
            
            # Start monitoring in parallel
            monitoring_tasks = [
                self.monitor_logs(relay, "Relay"),
                self.monitor_logs(collector, "Collector"), 
                self.monitor_logs(bridge, "Bridge"),
                self.connect_websocket()
            ]
            
            print("🎯 Running E2E test for 60 seconds...")
            await asyncio.wait_for(
                asyncio.gather(*monitoring_tasks, return_exceptions=True),
                timeout=70
            )
            
        except Exception as e:
            print(f"Test error: {e}")
        finally:
            await self.cleanup()
            self.analyze_results()

if __name__ == "__main__":
    test = POLPipelineTest()
    
    def signal_handler(sig, frame):
        print("\n🛑 Test interrupted")
        asyncio.create_task(test.cleanup())
        sys.exit(0)
        
    signal.signal(signal.SIGINT, signal_handler)
    
    asyncio.run(test.run_test())
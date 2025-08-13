#!/usr/bin/env python3
"""Test WebSocket client to measure end-to-end latency"""
import asyncio
import json
import time
import websockets
from datetime import datetime

class LatencyTester:
    def __init__(self):
        self.trades_received = 0
        self.latencies = []
        self.start_time = time.time()
        
    async def connect_and_measure(self):
        uri = "ws://localhost:8765/ws"
        print(f"📡 Connecting to {uri}...")
        
        try:
            async with websockets.connect(uri) as websocket:
                print("✅ Connected to WebSocket server")
                
                # Subscribe to trades
                subscribe_msg = json.dumps({
                    "type": "subscribe",
                    "channels": ["trades"]
                })
                await websocket.send(subscribe_msg)
                
                # Receive trades and measure latency
                while True:
                    message = await websocket.recv()
                    receive_time = time.time() * 1_000_000_000  # Convert to nanoseconds
                    
                    try:
                        data = json.loads(message)
                        if data.get("type") == "trade":
                            trade = data.get("data", {})
                            
                            # Calculate latency from trade timestamp
                            trade_timestamp_ns = trade.get("timestamp", 0) * 1_000_000_000
                            if trade_timestamp_ns > 0:
                                latency_ns = receive_time - trade_timestamp_ns
                                latency_ms = latency_ns / 1_000_000
                                self.latencies.append(latency_ms)
                            
                            self.trades_received += 1
                            
                            # Print stats every 100 trades
                            if self.trades_received % 100 == 0:
                                self.print_stats()
                            
                            # Print sample trade
                            if self.trades_received == 1:
                                print(f"\n📊 First Trade Received:")
                                print(f"   Symbol: {trade.get('symbol')}")
                                print(f"   Exchange: {trade.get('exchange')}")
                                print(f"   Price: ${trade.get('price', 0):,.2f}")
                                print(f"   Volume: {trade.get('volume', 0):.4f}")
                                print(f"   Side: {trade.get('side')}")
                                if self.latencies:
                                    print(f"   Latency: {self.latencies[-1]:.2f}ms")
                                
                    except json.JSONDecodeError:
                        print(f"Failed to parse message: {message}")
                        
        except Exception as e:
            print(f"❌ Connection error: {e}")
            
    def print_stats(self):
        if not self.latencies:
            return
            
        avg_latency = sum(self.latencies) / len(self.latencies)
        min_latency = min(self.latencies)
        max_latency = max(self.latencies)
        
        # Calculate percentiles
        sorted_latencies = sorted(self.latencies)
        p50 = sorted_latencies[len(sorted_latencies) // 2]
        p95 = sorted_latencies[int(len(sorted_latencies) * 0.95)]
        p99 = sorted_latencies[int(len(sorted_latencies) * 0.99)]
        
        elapsed = time.time() - self.start_time
        rate = self.trades_received / elapsed if elapsed > 0 else 0
        
        print(f"\n⚡ Performance Stats (after {self.trades_received} trades):")
        print(f"   Throughput: {rate:.1f} trades/sec")
        print(f"   Avg Latency: {avg_latency:.2f}ms")
        print(f"   Min Latency: {min_latency:.2f}ms")
        print(f"   Max Latency: {max_latency:.2f}ms")
        print(f"   P50 Latency: {p50:.2f}ms")
        print(f"   P95 Latency: {p95:.2f}ms")
        print(f"   P99 Latency: {p99:.2f}ms")

async def main():
    print("🚀 AlphaPulse WebSocket Latency Tester\n")
    tester = LatencyTester()
    await tester.connect_and_measure()

if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\n\n👋 Test interrupted by user")
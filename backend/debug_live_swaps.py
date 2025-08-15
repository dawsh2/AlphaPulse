#!/usr/bin/env python3
"""
Live POL Swap Debug Tool
========================

This tool connects to our live system and captures the exact raw data 
we're getting from POL swaps to identify the root cause of the 18.4x error.
"""

import asyncio
import json
import websockets
import subprocess
import time

class LiveSwapDebugger:
    def __init__(self):
        self.collected_swaps = []
        self.processes = []
        
    async def start_minimal_pipeline(self):
        """Start just the minimal components needed"""
        print("🚀 Starting minimal pipeline for debugging...")
        
        # Start relay server
        relay_proc = await asyncio.create_subprocess_exec(
            "./target/release/relay-server",
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            env={"RUST_LOG": "debug"},
            cwd="/Users/daws/alphapulse/backend"
        )
        self.processes.append(relay_proc)
        await asyncio.sleep(2)
        
        # Start Polygon collector  
        collector_proc = await asyncio.create_subprocess_exec(
            "./target/release/exchange-collector", 
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            env={"RUST_LOG": "debug", "EXCHANGE_NAME": "polygon"},
            cwd="/Users/daws/alphapulse/backend"
        )
        self.processes.append(collector_proc)
        await asyncio.sleep(2)
        
        # Start WS bridge
        bridge_proc = await asyncio.create_subprocess_exec(
            "./target/release/ws-bridge",
            stdout=asyncio.subprocess.PIPE, 
            stderr=asyncio.subprocess.PIPE,
            env={"RUST_LOG": "debug"},
            cwd="/Users/daws/alphapulse/backend"
        )
        self.processes.append(bridge_proc)
        await asyncio.sleep(2)
        
        print("✅ Pipeline started")
        return collector_proc
        
    async def monitor_collector_logs(self, collector_proc):
        """Monitor collector logs for POL swap debug data"""
        print("👂 Monitoring collector logs for POL swaps...")
        
        async def read_stream(stream, name):
            while True:
                try:
                    line = await stream.readline()
                    if not line:
                        break
                    line_str = line.decode('utf-8').strip()
                    
                    # Look for ANY swap activity first
                    if "💱 Real-time swap:" in line_str:
                        print(f"\n📊 REAL-TIME SWAP DETECTED:")
                        print(f"   {line_str}")
                        
                    elif "📊 Processing DEX swap event" in line_str:
                        print(f"\n⚡ SWAP EVENT RECEIVED:")
                        print(f"   {line_str}")
                        
                    elif "🔍 Identifying pool:" in line_str:
                        print(f"\n🏊 POOL IDENTIFICATION:")
                        print(f"   {line_str}")
                        
                    # POL-specific logging
                    elif "🔍 Raw swap amounts for POL" in line_str:
                        print(f"\n📊 POL RAW SWAP DATA:")
                        print(f"   {line_str}")
                        self.parse_and_store_raw_data(line_str)
                        
                    elif "🔍 PRICE CALCULATION DEBUG for POL" in line_str:
                        print(f"\n💰 POL PRICE CALCULATION:")
                        print(f"   {line_str}")
                        
                    # General WebSocket activity
                    elif "📡 Subscribing to DEX swap events" in line_str:
                        print(f"\n🔌 WEBSOCKET SUBSCRIPTION:")
                        print(f"   {line_str}")
                        
                    elif "✅ Subscribed to real-time DEX swap events" in line_str:
                        print(f"   {line_str}")
                        
                    elif "WebSocket connection failed" in line_str or "Failed to connect" in line_str:
                        print(f"\n❌ CONNECTION ISSUE:")
                        print(f"   {line_str}")
                        
                except Exception as e:
                    break
                    
        # Monitor both stdout and stderr
        await asyncio.gather(
            read_stream(collector_proc.stdout, "stdout"),
            read_stream(collector_proc.stderr, "stderr")
        )
        
    def parse_and_store_raw_data(self, line):
        """Extract raw amounts from debug line"""
        try:
            # Parse line like: "🔍 Raw swap amounts for POL/USDC: token0_in_raw=1000, token1_in_raw=0, token0_out_raw=0, token1_out_raw=230000000"
            parts = line.split(": ")[1]  # Get part after ": "
            amounts = {}
            for part in parts.split(", "):
                key, value = part.split("=")
                amounts[key] = float(value)
                
            swap_data = {
                'timestamp': time.time(),
                'raw_amounts': amounts,
                'line': line
            }
            self.collected_swaps.append(swap_data)
            
            print(f"📈 CAPTURED SWAP #{len(self.collected_swaps)}:")
            print(f"   token0_in_raw: {amounts.get('token0_in_raw', 0):.0f}")
            print(f"   token1_in_raw: {amounts.get('token1_in_raw', 0):.0f}") 
            print(f"   token0_out_raw: {amounts.get('token0_out_raw', 0):.0f}")
            print(f"   token1_out_raw: {amounts.get('token1_out_raw', 0):.0f}")
            
            # Calculate what this should give us
            self.analyze_swap_data(amounts)
            
        except Exception as e:
            print(f"Error parsing swap data: {e}")
            
    def analyze_swap_data(self, amounts):
        """Analyze the raw amounts to see what price they would produce"""
        token0_in = amounts.get('token0_in_raw', 0) / (10**18)  # POL
        token1_in = amounts.get('token1_in_raw', 0) / (10**6)   # USDC  
        token0_out = amounts.get('token0_out_raw', 0) / (10**18) # POL
        token1_out = amounts.get('token1_out_raw', 0) / (10**6)  # USDC
        
        print(f"   📊 DECIMAL ADJUSTED:")
        print(f"      POL_in:  {token0_in:.2f}")
        print(f"      USDC_in: {token1_in:.2f}")
        print(f"      POL_out: {token0_out:.2f}")
        print(f"      USDC_out: {token1_out:.2f}")
        
        # Calculate price
        if token0_in > 0 and token1_out > 0:
            # Selling POL for USDC
            price = token1_out / token0_in
            print(f"   💰 PRICE: ${price:.6f} USDC per POL (selling POL)")
        elif token1_in > 0 and token0_out > 0:
            # Selling USDC for POL  
            price = token1_in / token0_out
            print(f"   💰 PRICE: ${price:.6f} USDC per POL (buying POL)")
        else:
            print(f"   ❓ UNCLEAR SWAP DIRECTION")
            
        if price < 0.05:
            print(f"   ❌ WRONG PRICE! Should be ~$0.23")
            factor = 0.23 / price
            print(f"   📐 Correction factor: {factor:.1f}x")
        elif 0.15 <= price <= 0.35:
            print(f"   ✅ REASONABLE PRICE")
        else:
            print(f"   ❓ UNEXPECTED PRICE")
            
    async def run_debug_session(self):
        """Run a live debugging session"""
        print("🔍 POL SWAP LIVE DEBUGGER")
        print("="*40)
        
        try:
            collector = await self.start_minimal_pipeline()
            
            print("⏳ Waiting for POL swaps... (60 seconds)")
            print("   Press Ctrl+C to stop early\n")
            
            # Monitor for 60 seconds
            await asyncio.wait_for(
                self.monitor_collector_logs(collector),
                timeout=60
            )
            
        except asyncio.TimeoutError:
            print(f"\n⏰ Monitoring completed")
        except KeyboardInterrupt:
            print(f"\n🛑 Stopped by user")
        finally:
            await self.cleanup()
            
        print(f"\n📊 SUMMARY:")
        print(f"   Captured {len(self.collected_swaps)} POL swaps")
        
        if self.collected_swaps:
            print(f"\n🎯 PATTERN ANALYSIS:")
            for i, swap in enumerate(self.collected_swaps[:5]):  # Show first 5
                amounts = swap['raw_amounts']
                print(f"   Swap {i+1}: token0_in={amounts.get('token0_in_raw', 0):.0f}, token1_out={amounts.get('token1_out_raw', 0):.0f}")
            
    async def cleanup(self):
        """Clean up processes"""
        print("🧹 Cleaning up...")
        for proc in self.processes:
            try:
                proc.terminate()
                await asyncio.wait_for(proc.wait(), timeout=5)
            except:
                proc.kill()

async def main():
    debugger = LiveSwapDebugger()
    await debugger.run_debug_session()

if __name__ == "__main__":
    asyncio.run(main())
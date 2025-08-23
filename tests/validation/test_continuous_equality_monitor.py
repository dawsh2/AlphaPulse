#!/usr/bin/env python3
"""
Continuous Equality Monitor

Runs continuously, comparing pipeline input to frontend output in real-time.
Stops immediately upon finding ANY inequality.

This is the ultimate validation: if the pipeline preserves data perfectly,
this test should run forever. If it finds inequality, it halts and reports.
"""

import asyncio
import websockets
import json
import time
import socket
import struct
from typing import Dict, List, Any, Optional
from collections import defaultdict
import threading

class ContinuousEqualityMonitor:
    """Monitors pipeline input vs frontend output continuously"""
    
    def __init__(self):
        self.pipeline_data = {}  # symbol_id -> latest data from pipeline
        self.frontend_data = {}  # symbol -> latest data from frontend  
        self.symbol_mapping = {}  # symbol_id -> symbol string
        self.running = True
        self.inequality_found = False
        self.inequality_details = None
        
    def monitor_pipeline_input(self):
        """Monitor pipeline input via Unix socket in separate thread"""
        try:
            sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            sock.connect('/tmp/alphapulse/polygon.sock')
            sock.settimeout(1.0)
            
            print("📡 Monitoring pipeline input...")
            
            while self.running:
                try:
                    # Read message header
                    header = sock.recv(8)
                    if len(header) != 8:
                        continue
                        
                    magic, msg_type, size, sequence = struct.unpack('<HHHH', header)
                    
                    if magic != 0x03FE:
                        continue
                        
                    # Read message body
                    body = sock.recv(size)
                    if len(body) != size:
                        continue
                    
                    # Parse message types
                    if msg_type == 1:  # TRADE message
                        trade_data = self.parse_trade_message(body)
                        if trade_data:
                            symbol_id = trade_data['symbol_id']
                            self.pipeline_data[symbol_id] = trade_data
                            
                    elif msg_type == 8:  # SYMBOL_MAPPING message  
                        mapping = self.parse_symbol_mapping(body)
                        if mapping:
                            self.symbol_mapping[mapping['symbol_id']] = mapping['symbol']
                            
                except socket.timeout:
                    continue
                except Exception as e:
                    if self.running:
                        print(f"⚠️ Pipeline monitoring error: {e}")
                    break
            
            sock.close()
            
        except Exception as e:
            print(f"❌ Failed to monitor pipeline input: {e}")
    
    def parse_trade_message(self, body: bytes) -> Optional[Dict]:
        """Parse binary trade message"""
        try:
            if len(body) < 64:
                return None
                
            fields = struct.unpack('<QQQQQQQf', body)
            
            return {
                "symbol_id": fields[0],
                "price": fields[1] / 100000000.0,  # Convert from fixed-point
                "volume": fields[2] / 100000000.0,
                "liquidity": fields[3] / 100000000.0,
                "gas_cost": fields[4] / 100000000.0,
                "timestamp": time.time()
            }
            
        except Exception:
            return None
    
    def parse_symbol_mapping(self, body: bytes) -> Optional[Dict]:
        """Parse symbol mapping message"""
        try:
            # First 8 bytes: symbol_id (uint64)
            symbol_id = struct.unpack('<Q', body[:8])[0]
            
            # Rest: null-terminated string
            symbol_bytes = body[8:]
            null_pos = symbol_bytes.find(b'\x00')
            if null_pos >= 0:
                symbol = symbol_bytes[:null_pos].decode('utf-8')
            else:
                symbol = symbol_bytes.decode('utf-8')
                
            return {
                "symbol_id": symbol_id,
                "symbol": symbol
            }
            
        except Exception:
            return None
    
    async def monitor_frontend_output(self):
        """Monitor frontend output via WebSocket"""
        try:
            uri = "ws://127.0.0.1:8765"
            async with websockets.connect(uri) as websocket:
                print("📱 Monitoring frontend output...")
                
                while self.running:
                    try:
                        message = await asyncio.wait_for(websocket.recv(), timeout=1.0)
                        data = json.loads(message)
                        
                        if data.get('msg_type') == 'trade':
                            symbol = data.get('symbol', '')
                            frontend_trade = {
                                "price": data.get('price', 0),
                                "volume": data.get('volume', 0),
                                "liquidity": data.get('liquidity', 0),
                                "gas_cost": data.get('gas_cost', 0),
                                "timestamp": time.time()
                            }
                            
                            self.frontend_data[symbol] = frontend_trade
                            
                            # Check for equality immediately
                            await self.check_equality(symbol, frontend_trade)
                            
                    except asyncio.TimeoutError:
                        continue
                    except Exception as e:
                        if self.running:
                            print(f"⚠️ Frontend monitoring error: {e}")
                        break
                        
        except Exception as e:
            print(f"❌ Failed to monitor frontend output: {e}")
    
    async def check_equality(self, symbol: str, frontend_trade: Dict):
        """Check if frontend data exactly matches pipeline data"""
        # Find matching pipeline data by symbol
        symbol_id = None
        for sid, sym in self.symbol_mapping.items():
            if sym == symbol:
                symbol_id = sid
                break
        
        if symbol_id is None:
            return  # No mapping found yet
            
        if symbol_id not in self.pipeline_data:
            return  # No pipeline data yet
            
        pipeline_trade = self.pipeline_data[symbol_id]
        
        # Check exact equality (floating point precision tolerance only)
        price_equal = abs(pipeline_trade['price'] - frontend_trade['price']) < 1e-10
        volume_equal = abs(pipeline_trade['volume'] - frontend_trade['volume']) < 1e-10
        liquidity_equal = abs(pipeline_trade['liquidity'] - frontend_trade['liquidity']) < 1e-10
        gas_equal = abs(pipeline_trade['gas_cost'] - frontend_trade['gas_cost']) < 1e-10
        
        if not (price_equal and volume_equal and liquidity_equal and gas_equal):
            # INEQUALITY FOUND - STOP EVERYTHING
            self.inequality_found = True
            self.running = False
            
            self.inequality_details = {
                "symbol": symbol,
                "symbol_id": symbol_id,
                "pipeline_data": pipeline_trade,
                "frontend_data": frontend_trade,
                "mismatches": {
                    "price": not price_equal,
                    "volume": not volume_equal, 
                    "liquidity": not liquidity_equal,
                    "gas_cost": not gas_equal
                },
                "timestamp": time.time()
            }
            
            print(f"\n🚨 INEQUALITY DETECTED! Stopping monitor...")
            print(f"Symbol: {symbol}")
            print(f"Pipeline Price: {pipeline_trade['price']}")
            print(f"Frontend Price: {frontend_trade['price']}")
            
            if not price_equal:
                print(f"❌ Price mismatch: {pipeline_trade['price']} != {frontend_trade['price']}")
            if not volume_equal:
                print(f"❌ Volume mismatch: {pipeline_trade['volume']} != {frontend_trade['volume']}")
            if not liquidity_equal:
                print(f"❌ Liquidity mismatch: {pipeline_trade['liquidity']} != {frontend_trade['liquidity']}")
            if not gas_equal:
                print(f"❌ Gas cost mismatch: {pipeline_trade['gas_cost']} != {frontend_trade['gas_cost']}")
        else:
            # Perfect match - continue monitoring
            print(f"✅ Perfect match: {symbol} (price: ${frontend_trade['price']:.8f})")
    
    async def run_continuous_monitor(self, max_duration: int = 300):
        """Run continuous monitoring until inequality found or timeout"""
        print("=" * 80)
        print("CONTINUOUS EQUALITY MONITOR")
        print("Monitoring pipeline input vs frontend output continuously")
        print("Will STOP immediately upon finding ANY inequality")
        print("=" * 80)
        
        # Start pipeline monitoring in background thread
        pipeline_thread = threading.Thread(target=self.monitor_pipeline_input)
        pipeline_thread.daemon = True
        pipeline_thread.start()
        
        # Give pipeline thread time to start
        await asyncio.sleep(1)
        
        print(f"🔄 Starting continuous monitoring (max {max_duration}s)...")
        print("💡 If this runs without stopping, the pipeline preserves data perfectly")
        print("🚨 If this stops, an inequality was found")
        
        start_time = time.time()
        
        try:
            # Monitor frontend and check equality
            await asyncio.wait_for(
                self.monitor_frontend_output(), 
                timeout=max_duration
            )
            
        except asyncio.TimeoutError:
            self.running = False
            elapsed = time.time() - start_time
            
            if not self.inequality_found:
                print(f"\n🎉 SUCCESS: Monitored for {elapsed:.1f}s without finding inequality!")
                print("✅ Pipeline appears to preserve data perfectly")
                return {
                    "status": "PASSED",
                    "duration": elapsed,
                    "inequality_found": False,
                    "message": f"No inequalities found in {elapsed:.1f}s of monitoring"
                }
        
        # If we get here, inequality was found
        elapsed = time.time() - start_time
        
        return {
            "status": "FAILED",
            "duration": elapsed,
            "inequality_found": True,
            "inequality_details": self.inequality_details,
            "message": f"Inequality found after {elapsed:.1f}s of monitoring"
        }

async def main():
    monitor = ContinuousEqualityMonitor()
    
    print("🎯 CONTINUOUS EQUALITY MONITOR")
    print("This test monitors pipeline input vs frontend output continuously.")
    print("If data preservation is perfect, this should run without stopping.")
    print("If inequality is found, it stops immediately and reports the issue.")
    print()
    
    result = await monitor.run_continuous_monitor(max_duration=60)  # 1 minute max
    
    if result["status"] == "PASSED":
        print("\n🎉 SUCCESS: No inequalities found!")
        print("The pipeline preserves data exactly.")
        return 0
    else:
        print("\n💥 FAILURE: Inequality detected!")
        print("The pipeline does not preserve data exactly.")
        if result.get("inequality_details"):
            details = result["inequality_details"]
            print(f"\nDetails:")
            print(f"  Symbol: {details['symbol']}")
            print(f"  Mismatches: {details['mismatches']}")
        return 1

if __name__ == "__main__":
    exit_code = asyncio.run(main())
    exit(exit_code)
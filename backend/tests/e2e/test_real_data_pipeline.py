#!/usr/bin/env python3
"""
Real End-to-End Data Pipeline Test

This test validates the ACTUAL data flow using real components:
1. Start actual relay server
2. Start actual exchange collector  
3. Start actual ws_bridge
4. Connect to real WebSocket and validate data flow
5. Verify SymbolMapping and Trade message delivery

NO SIMULATION - tests actual components and real message flow.
"""

import asyncio
import subprocess
import time
import sys
import os
import socket
import json
import websockets
import signal
from pathlib import Path
from typing import Dict, List, Any, Optional
from dataclasses import dataclass, field
import threading
import queue

@dataclass
class ComponentProcess:
    """Manages a component process"""
    name: str
    process: Optional[subprocess.Popen] = None
    is_running: bool = False
    startup_time: float = 0.0

@dataclass
class RealDataFlowTrace:
    """Traces actual data through real components"""
    websocket_data: Dict[str, Any]
    symbol_hash: str
    human_readable_symbol: str
    timestamp_received: float
    latency_us: Optional[int] = None
    precision_preserved: bool = False
    validation_passed: bool = False

class RealDataPipelineTest:
    """Tests actual data pipeline with real components"""
    
    def __init__(self):
        self.backend_dir = Path(__file__).parent.parent.parent
        self.components: Dict[str, ComponentProcess] = {
            'relay': ComponentProcess('relay-server'),
            'collector': ComponentProcess('exchange-collector'),
            'ws_bridge': ComponentProcess('ws-bridge')
        }
        self.websocket_messages: List[Dict[str, Any]] = []
        self.symbol_mappings: Dict[str, str] = {}  # hash -> human name
        self.test_results = {
            'symbol_mappings_received': 0,
            'trade_messages_received': 0,
            'human_readable_symbols': 0,
            'precision_errors': [],
            'connection_successful': False
        }
        
    async def run_complete_pipeline_test(self, duration_seconds: int = 30) -> bool:
        """Run complete real pipeline test"""
        print("🧪 Starting REAL Data Pipeline Test")
        print("   Testing actual components with real data flow")
        print(f"   Duration: {duration_seconds} seconds")
        
        try:
            # Step 1: Start all components in correct order
            if not await self.start_all_components():
                return False
                
            # Step 2: Connect to WebSocket and collect data
            if not await self.collect_websocket_data(duration_seconds):
                return False
                
            # Step 3: Validate data flow and integrity
            success = await self.validate_data_flow()
            
            # Step 4: Generate test report
            self.generate_test_report()
            
            return success
            
        finally:
            await self.cleanup_all_components()
    
    async def start_all_components(self) -> bool:
        """Start all components in correct order"""
        print("🔧 Starting components in order...")
        
        # Change to backend directory
        os.chdir(self.backend_dir)
        
        # Step 1: Start relay server first
        if not await self.start_relay_server():
            return False
            
        # Step 2: Wait for relay to be ready
        await self.wait_for_relay_ready()
        
        # Step 3: Start ws_bridge
        if not await self.start_ws_bridge():
            return False
            
        # Step 4: Start exchange collector last
        if not await self.start_exchange_collector():
            return False
            
        # Step 5: Wait for all connections to establish
        await asyncio.sleep(3)
        
        return True
    
    async def start_relay_server(self) -> bool:
        """Start the actual relay server"""
        print("   🚀 Starting relay server...")
        
        try:
            self.components['relay'].process = subprocess.Popen(
                ["./target/release/relay-server"],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True
            )
            
            # Give it time to start
            await asyncio.sleep(2)
            
            if self.components['relay'].process.poll() is None:
                self.components['relay'].is_running = True
                print("   ✅ Relay server started")
                return True
            else:
                stdout, stderr = self.components['relay'].process.communicate()
                print(f"   ❌ Relay server failed: {stderr}")
                return False
                
        except Exception as e:
            print(f"   ❌ Failed to start relay server: {e}")
            return False
    
    async def wait_for_relay_ready(self):
        """Wait for relay server to be ready"""
        print("   ⏳ Waiting for relay server socket...")
        
        for attempt in range(20):
            try:
                sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
                sock.connect("/tmp/alphapulse/relay.sock")
                sock.close()
                print("   ✅ Relay server socket ready")
                return
            except (FileNotFoundError, ConnectionRefusedError):
                await asyncio.sleep(0.5)
                
        raise Exception("Relay server socket not ready after 10 seconds")
    
    async def start_ws_bridge(self) -> bool:
        """Start the actual ws_bridge"""
        print("   🌉 Starting ws_bridge...")
        
        try:
            self.components['ws_bridge'].process = subprocess.Popen(
                ["./target/release/ws-bridge"],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True
            )
            
            # Give it time to start
            await asyncio.sleep(2)
            
            if self.components['ws_bridge'].process.poll() is None:
                self.components['ws_bridge'].is_running = True
                print("   ✅ WS bridge started")
                return True
            else:
                stdout, stderr = self.components['ws_bridge'].process.communicate()
                print(f"   ❌ WS bridge failed: {stderr}")
                return False
                
        except Exception as e:
            print(f"   ❌ Failed to start ws_bridge: {e}")
            return False
    
    async def start_exchange_collector(self) -> bool:
        """Start the actual exchange collector"""
        print("   📡 Starting exchange collector...")
        
        try:
            env = os.environ.copy()
            env["EXCHANGE_NAME"] = "polygon"
            env["RUST_LOG"] = "info"  # Less verbose for test
            
            self.components['collector'].process = subprocess.Popen(
                ["./target/release/exchange-collector"],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                env=env
            )
            
            # Give it time to start and send SymbolMapping messages
            await asyncio.sleep(3)
            
            if self.components['collector'].process.poll() is None:
                self.components['collector'].is_running = True
                print("   ✅ Exchange collector started")
                return True
            else:
                stdout, stderr = self.components['collector'].process.communicate()
                print(f"   ❌ Exchange collector failed: {stderr}")
                return False
                
        except Exception as e:
            print(f"   ❌ Failed to start exchange collector: {e}")
            return False
    
    async def collect_websocket_data(self, duration_seconds: int) -> bool:
        """Connect to WebSocket and collect real data"""
        print(f"   📊 Collecting WebSocket data for {duration_seconds}s...")
        
        try:
            uri = "ws://127.0.0.1:8765/stream"
            async with websockets.connect(uri) as websocket:
                self.test_results['connection_successful'] = True
                print("   ✅ WebSocket connected")
                
                # Collect messages for specified duration
                start_time = time.time()
                
                while time.time() - start_time < duration_seconds:
                    try:
                        # Wait for message with timeout
                        message = await asyncio.wait_for(
                            websocket.recv(), 
                            timeout=1.0
                        )
                        
                        # Parse and store message
                        data = json.loads(message)
                        self.websocket_messages.append(data)
                        
                        # Process different message types
                        if data.get('msg_type') == 'symbol_mapping':
                            self.process_symbol_mapping(data)
                        elif data.get('msg_type') == 'trade':
                            self.process_trade_message(data)
                            
                    except asyncio.TimeoutError:
                        continue  # No message within timeout, continue
                        
                print(f"   ✅ Collected {len(self.websocket_messages)} messages")
                return True
                
        except Exception as e:
            print(f"   ❌ WebSocket collection failed: {e}")
            return False
    
    def process_symbol_mapping(self, data: Dict[str, Any]):
        """Process SymbolMapping message"""
        symbol_hash = data.get('symbol_hash', '')
        symbol_name = data.get('symbol', '')
        
        if symbol_hash and symbol_name:
            self.symbol_mappings[symbol_hash] = symbol_name
            self.test_results['symbol_mappings_received'] += 1
            print(f"   📝 Symbol mapping: {symbol_name} ({symbol_hash})")
    
    def process_trade_message(self, data: Dict[str, Any]):
        """Process Trade message"""
        self.test_results['trade_messages_received'] += 1
        
        symbol = data.get('symbol')
        if symbol and not symbol.startswith('UNKNOWN_SYMBOL'):
            self.test_results['human_readable_symbols'] += 1
            
        # Check for precision preservation
        price = data.get('price', 0)
        if isinstance(price, (int, float)) and price > 0:
            # Simple precision check - ensure reasonable price ranges
            if 0.001 <= price <= 1000000:  # Reasonable DeFi price range
                pass  # Precision looks good
            else:
                self.test_results['precision_errors'].append(f"Suspicious price: {price}")
    
    async def validate_data_flow(self) -> bool:
        """Validate the complete data flow"""
        print("   🔍 Validating data flow...")
        
        results = self.test_results
        
        # Check basic connectivity
        if not results['connection_successful']:
            print("   ❌ WebSocket connection failed")
            return False
        
        # Check SymbolMapping messages
        if results['symbol_mappings_received'] == 0:
            print("   ❌ No SymbolMapping messages received")
            return False
        
        # Check Trade messages
        if results['trade_messages_received'] == 0:
            print("   ❌ No Trade messages received")
            return False
            
        # Check symbol resolution
        symbol_resolution_rate = (results['human_readable_symbols'] / 
                                max(1, results['trade_messages_received'])) * 100
        
        if symbol_resolution_rate < 90:  # At least 90% should resolve
            print(f"   ❌ Low symbol resolution rate: {symbol_resolution_rate:.1f}%")
            return False
            
        # Check precision
        if len(results['precision_errors']) > 0:
            print(f"   ❌ Precision errors detected: {results['precision_errors']}")
            return False
        
        print("   ✅ Data flow validation passed")
        return True
    
    def generate_test_report(self):
        """Generate comprehensive test report"""
        print("\n" + "=" * 80)
        print("REAL DATA PIPELINE TEST RESULTS")
        print("=" * 80)
        
        results = self.test_results
        
        print(f"📊 Message Statistics:")
        print(f"   WebSocket Messages: {len(self.websocket_messages)}")
        print(f"   SymbolMapping Messages: {results['symbol_mappings_received']}")
        print(f"   Trade Messages: {results['trade_messages_received']}")
        print(f"   Human-Readable Symbols: {results['human_readable_symbols']}")
        
        if results['trade_messages_received'] > 0:
            resolution_rate = (results['human_readable_symbols'] / 
                             results['trade_messages_received']) * 100
            print(f"   Symbol Resolution Rate: {resolution_rate:.1f}%")
        
        print(f"\n🔧 Component Status:")
        for name, component in self.components.items():
            status = "✅ Running" if component.is_running else "❌ Stopped"
            print(f"   {name}: {status}")
        
        print(f"\n🧪 Validation Results:")
        print(f"   Connection: {'✅' if results['connection_successful'] else '❌'}")
        print(f"   Data Flow: {'✅' if results['trade_messages_received'] > 0 else '❌'}")
        print(f"   Symbol Resolution: {'✅' if results['human_readable_symbols'] > 0 else '❌'}")
        print(f"   Precision: {'✅' if len(results['precision_errors']) == 0 else '❌'}")
        
        # Show sample messages
        if len(self.websocket_messages) > 0:
            print(f"\n📝 Sample Messages:")
            for i, msg in enumerate(self.websocket_messages[:3]):
                msg_type = msg.get('msg_type', 'unknown')
                symbol = msg.get('symbol', 'N/A')
                print(f"   {i+1}. {msg_type}: {symbol}")
    
    async def cleanup_all_components(self):
        """Clean up all component processes"""
        print("\n🧹 Cleaning up components...")
        
        for name, component in self.components.items():
            if component.process and component.is_running:
                try:
                    component.process.terminate()
                    await asyncio.sleep(1)
                    if component.process.poll() is None:
                        component.process.kill()
                    component.is_running = False
                    print(f"   ✅ {name} cleaned up")
                except Exception as e:
                    print(f"   ⚠️  Error cleaning up {name}: {e}")

async def main():
    """Run the real data pipeline test"""
    print("🚀 Real Data Pipeline Test Suite")
    print("=" * 50)
    
    test = RealDataPipelineTest()
    
    try:
        success = await test.run_complete_pipeline_test(duration_seconds=20)
        
        if success:
            print("\n" + "=" * 50)
            print("✅ REAL DATA PIPELINE TEST PASSED")
            print("   All components working with real data flow!")
            sys.exit(0)
        else:
            print("\n" + "=" * 50)
            print("❌ REAL DATA PIPELINE TEST FAILED")
            print("   Pipeline has issues with real data flow")
            sys.exit(1)
            
    except KeyboardInterrupt:
        print("\n⚠️  Test interrupted by user")
        await test.cleanup_all_components()
        sys.exit(1)
    except Exception as e:
        print(f"\n💥 Test failed with exception: {e}")
        await test.cleanup_all_components()
        sys.exit(1)

if __name__ == "__main__":
    asyncio.run(main())
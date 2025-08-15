#!/usr/bin/env python3
"""
Phase 1: Enhanced Pipeline Debug - Capture ALL Messages
=====================================================

This script traces the exact flow through the pipeline:
1. Unix Socket: /tmp/alphapulse/polygon.sock (collector → relay)  
2. Relay Server: forwarding to /tmp/alphapulse/relay.sock
3. WS Bridge: receiving from relay and broadcasting
4. WebSocket: ws://127.0.0.1:8765/stream

The goal is to identify EXACTLY where Polygon trades are being lost.
"""

import asyncio
import websockets
import json
import time
import socket
import struct
import threading
import queue
from typing import Dict, List, Any, Optional
import logging

# Configure detailed logging
logging.basicConfig(
    level=logging.DEBUG,
    format='%(asctime)s [%(levelname)s] %(name)s: %(message)s',
    handlers=[
        logging.FileHandler('/Users/daws/alphapulse/backend/tests/e2e/pipeline_debug.log'),
        logging.StreamHandler()
    ]
)

logger = logging.getLogger('PipelineDebug')

class PipelineMonitor:
    """Monitors all stages of the AlphaPulse pipeline"""
    
    def __init__(self):
        self.unix_socket_messages = queue.Queue()
        self.relay_messages = queue.Queue() 
        self.websocket_messages = queue.Queue()
        self.running = True
        
    async def run_complete_pipeline_debug(self, duration_seconds: int = 30):
        """Run comprehensive pipeline debugging"""
        logger.info(f"🔍 Starting complete pipeline debug ({duration_seconds}s)")
        
        # Start all monitoring tasks concurrently
        tasks = [
            self._monitor_unix_socket_traffic(),
            self._monitor_relay_server_traffic(), 
            self._monitor_websocket_traffic(duration_seconds),
            self._analyze_pipeline_flow(duration_seconds)
        ]
        
        try:
            await asyncio.gather(*tasks)
        except Exception as e:
            logger.error(f"Pipeline monitoring failed: {e}")
        finally:
            self.running = False
            
    async def _monitor_unix_socket_traffic(self):
        """Monitor Unix socket traffic between collector and relay"""
        logger.info("🔌 Monitoring Unix socket traffic: /tmp/alphapulse/polygon.sock")
        
        def unix_socket_monitor():
            try:
                # Try to connect as a client to see if relay is listening
                sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
                sock.settimeout(5.0)
                sock.connect('/tmp/alphapulse/relay.sock')
                logger.info("✅ Successfully connected to relay server")
                
                buffer = b''
                while self.running:
                    try:
                        data = sock.recv(4096)
                        if not data:
                            logger.warning("❌ Relay connection closed")
                            break
                            
                        buffer += data
                        
                        # Parse message headers to identify message types
                        while len(buffer) >= 8:  # Minimum header size
                            if buffer[0] != 0xFE:  # Magic byte check
                                logger.error(f"❌ Invalid magic byte: 0x{buffer[0]:02x}")
                                buffer = buffer[1:]  # Skip invalid byte
                                continue
                                
                            message_type = buffer[1]
                            length = struct.unpack('<H', buffer[2:4])[0]
                            sequence = struct.unpack('<I', buffer[4:8])[0]
                            
                            total_length = 8 + length
                            if len(buffer) < total_length:
                                break  # Wait for more data
                                
                            # Extract complete message
                            message = buffer[:total_length]
                            buffer = buffer[total_length:]
                            
                            message_info = {
                                'source': 'relay_server',
                                'type': self._decode_message_type(message_type),
                                'length': length,
                                'sequence': sequence,
                                'timestamp': time.time(),
                                'raw_data': message[:32].hex()  # First 32 bytes for debug
                            }
                            
                            self.relay_messages.put(message_info)
                            logger.debug(f"📦 Relay message: {message_info['type']} (seq={sequence}, len={length})")
                            
                    except socket.timeout:
                        continue
                    except Exception as e:
                        logger.error(f"❌ Unix socket monitor error: {e}")
                        break
                        
            except Exception as e:
                logger.error(f"❌ Failed to connect to relay server: {e}")
                logger.info("💡 This might be why trades aren't reaching the dashboard!")
                
        # Run in thread to avoid blocking
        thread = threading.Thread(target=unix_socket_monitor, daemon=True)
        thread.start()
        
        # Wait for thread or timeout
        await asyncio.sleep(30)
        
    async def _monitor_relay_server_traffic(self):
        """Check if relay server is actually running and receiving Polygon data"""
        logger.info("🔄 Checking relay server status...")
        
        # Check if relay server socket exists
        import os
        relay_socket_path = '/tmp/alphapulse/relay.sock'
        polygon_socket_path = '/tmp/alphapulse/polygon.sock'
        
        if os.path.exists(relay_socket_path):
            logger.info("✅ Relay server socket exists: /tmp/alphapulse/relay.sock")
        else:
            logger.error("❌ Relay server socket missing: /tmp/alphapulse/relay.sock")
            logger.info("💡 Start relay server: cargo run --bin relay-server")
            
        if os.path.exists(polygon_socket_path):
            logger.info("✅ Polygon collector socket exists: /tmp/alphapulse/polygon.sock")
        else:
            logger.error("❌ Polygon collector socket missing: /tmp/alphapulse/polygon.sock")
            logger.info("💡 Start exchange collector: cargo run --bin exchange-collector polygon")
            
        # Check socket permissions
        try:
            import stat
            if os.path.exists(relay_socket_path):
                mode = os.stat(relay_socket_path).st_mode
                logger.info(f"📋 Relay socket permissions: {oct(stat.S_IMODE(mode))}")
        except Exception as e:
            logger.error(f"❌ Cannot check socket permissions: {e}")
            
    async def _monitor_websocket_traffic(self, duration: int):
        """Monitor WebSocket traffic from WS Bridge"""
        logger.info("🌐 Monitoring WebSocket traffic: ws://127.0.0.1:8765/stream")
        
        try:
            uri = "ws://127.0.0.1:8765/stream"
            async with websockets.connect(uri, timeout=10) as websocket:
                logger.info("✅ Connected to WebSocket")
                
                start_time = time.time()
                message_count = 0
                polygon_trades = 0
                
                while time.time() - start_time < duration:
                    try:
                        message = await asyncio.wait_for(websocket.recv(), timeout=2.0)
                        message_count += 1
                        
                        try:
                            data = json.loads(message)
                            
                            # Check if this is a Polygon trade
                            symbol = data.get('symbol', '')
                            exchange = data.get('exchange_id', '')
                            msg_type = data.get('msg_type', '')
                            
                            if 'quickswap' in symbol.lower() or 'polygon' in str(exchange).lower():
                                polygon_trades += 1
                                logger.info(f"🎯 POLYGON TRADE FOUND: {symbol} - {data.get('price', 'N/A')}")
                                
                            websocket_info = {
                                'source': 'websocket',
                                'data': data,
                                'timestamp': time.time(),
                                'is_polygon': 'quickswap' in symbol.lower()
                            }
                            
                            self.websocket_messages.put(websocket_info)
                            
                        except json.JSONDecodeError:
                            logger.warning(f"❌ Invalid JSON from WebSocket: {message[:100]}")
                            
                    except asyncio.TimeoutError:
                        continue
                        
                logger.info(f"📊 WebSocket Summary: {message_count} total messages, {polygon_trades} Polygon trades")
                
        except Exception as e:
            logger.error(f"❌ WebSocket connection failed: {e}")
            logger.info("💡 Check if ws-bridge is running: cargo run --bin ws-bridge")
            
    async def _analyze_pipeline_flow(self, duration: int):
        """Analyze message flow through entire pipeline"""
        logger.info("🔬 Analyzing complete pipeline flow...")
        
        await asyncio.sleep(duration + 2)  # Wait for data collection
        
        # Collect all messages
        relay_msgs = []
        websocket_msgs = []
        
        while not self.relay_messages.empty():
            relay_msgs.append(self.relay_messages.get())
            
        while not self.websocket_messages.empty():
            websocket_msgs.append(self.websocket_messages.get())
            
        logger.info("=" * 60)
        logger.info("PIPELINE FLOW ANALYSIS")
        logger.info("=" * 60)
        
        # Analyze relay server messages
        logger.info(f"📡 Relay Server Messages: {len(relay_msgs)}")
        if relay_msgs:
            message_types = {}
            for msg in relay_msgs:
                msg_type = msg['type']
                message_types[msg_type] = message_types.get(msg_type, 0) + 1
                
            for msg_type, count in message_types.items():
                logger.info(f"   {msg_type}: {count} messages")
                
            # Check for trade messages specifically
            trade_msgs = [m for m in relay_msgs if m['type'] == 'TRADE']
            logger.info(f"   🎯 TRADE messages from relay: {len(trade_msgs)}")
            
        else:
            logger.error("❌ NO MESSAGES FROM RELAY SERVER!")
            logger.info("💡 Possible issues:")
            logger.info("   - Relay server not running")
            logger.info("   - Polygon collector not connected to relay")
            logger.info("   - Relay not forwarding messages")
            
        # Analyze WebSocket messages  
        logger.info(f"🌐 WebSocket Messages: {len(websocket_msgs)}")
        if websocket_msgs:
            symbol_counts = {}
            polygon_msgs = []
            
            for msg in websocket_msgs:
                data = msg['data']
                symbol = data.get('symbol', 'UNKNOWN')
                symbol_counts[symbol] = symbol_counts.get(symbol, 0) + 1
                
                if msg['is_polygon']:
                    polygon_msgs.append(msg)
                    
            logger.info(f"   Unique symbols: {len(symbol_counts)}")
            for symbol, count in list(symbol_counts.items())[:10]:  # Top 10
                logger.info(f"   {symbol}: {count} messages")
                
            logger.info(f"   🎯 POLYGON messages on WebSocket: {len(polygon_msgs)}")
            
            if polygon_msgs:
                logger.info("✅ POLYGON DATA IS REACHING WEBSOCKET!")
                for msg in polygon_msgs[:3]:  # Show first 3
                    data = msg['data']
                    logger.info(f"   Example: {data.get('symbol')} @ {data.get('price', 'N/A')}")
            else:
                logger.error("❌ NO POLYGON DATA ON WEBSOCKET!")
                
        else:
            logger.error("❌ NO WEBSOCKET MESSAGES!")
            logger.info("💡 Possible issues:")
            logger.info("   - WS Bridge not running")
            logger.info("   - WS Bridge not connected to relay")
            logger.info("   - No data flowing through pipeline")
            
        # Pipeline diagnosis
        logger.info("=" * 60)
        logger.info("PIPELINE DIAGNOSIS")
        logger.info("=" * 60)
        
        if len(relay_msgs) == 0 and len(websocket_msgs) == 0:
            logger.error("🚫 COMPLETE PIPELINE FAILURE")
            logger.info("💡 Likely causes:")
            logger.info("   1. Relay server not running")
            logger.info("   2. Exchange collector not running")
            logger.info("   3. WS Bridge not running")
            
        elif len(relay_msgs) > 0 and len(websocket_msgs) == 0:
            logger.error("🔌 RELAY → WEBSOCKET FAILURE")
            logger.info("💡 Relay is working but WS Bridge is not forwarding")
            logger.info("   Check: WS Bridge connection to relay server")
            
        elif len(relay_msgs) == 0 and len(websocket_msgs) > 0:
            logger.warning("⚠️  WEBSOCKET WORKING, BUT NO RELAY DATA")
            logger.info("💡 WebSocket has data but not from relay")
            logger.info("   Check: Relay server and collector connections")
            
        else:
            polygon_in_relay = len([m for m in relay_msgs if m['type'] == 'TRADE'])
            polygon_in_ws = len([m for m in websocket_msgs if m['is_polygon']])
            
            if polygon_in_relay > 0 and polygon_in_ws > 0:
                logger.info("✅ PIPELINE IS WORKING!")
                logger.info(f"   {polygon_in_relay} trades from relay → {polygon_in_ws} on WebSocket")
            elif polygon_in_relay > 0 and polygon_in_ws == 0:
                logger.error("🔄 TRADES IN RELAY BUT NOT ON WEBSOCKET")
                logger.info("💡 Relay receives trades but WS Bridge isn't forwarding Polygon data")
            else:
                logger.error("📭 NO POLYGON TRADES IN PIPELINE")
                logger.info("💡 Polygon collector may not be sending trade data")
                
    def _decode_message_type(self, type_byte: int) -> str:
        """Decode message type from byte value"""
        types = {
            0x01: "TRADE",
            0x02: "ORDERBOOK", 
            0x03: "HEARTBEAT",
            0x04: "METRICS",
            0x05: "L2_SNAPSHOT",
            0x06: "L2_DELTA",
            0x07: "L2_RESET",
            0x08: "SYMBOL_MAPPING",
            0x09: "ARBITRAGE_OPPORTUNITY",
            0x0A: "STATUS_UPDATE"
        }
        return types.get(type_byte, f"UNKNOWN_0x{type_byte:02x}")

async def run_phase1_debug():
    """Run Phase 1 of pipeline debugging"""
    print("=" * 80)
    print("PHASE 1: ENHANCED PIPELINE DEBUG - CAPTURE ALL MESSAGES")
    print("=" * 80)
    print("Monitoring complete pipeline:")
    print("  1. Unix Socket: /tmp/alphapulse/polygon.sock")  
    print("  2. Relay Server: /tmp/alphapulse/relay.sock")
    print("  3. WebSocket: ws://127.0.0.1:8765/stream")
    print("=" * 80)
    
    monitor = PipelineMonitor()
    await monitor.run_complete_pipeline_debug(duration_seconds=15)
    
    print("\n" + "=" * 80)
    print("PHASE 1 DEBUG COMPLETE")
    print("=" * 80)
    print("Check pipeline_debug.log for detailed analysis")
    print("Next steps based on findings:")
    print("  - If no relay messages: Check relay server and collector")
    print("  - If relay works but no WebSocket: Check WS Bridge")
    print("  - If both work but no Polygon: Check collector configuration")

if __name__ == "__main__":
    asyncio.run(run_phase1_debug())
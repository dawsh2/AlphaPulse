#!/usr/bin/env python3
"""Simple arbitrage monitor using Unix socket stream from exchange collector"""

import asyncio
import json
import struct
import time
from collections import defaultdict

# Socket path
UNIX_SOCKET_PATH = "/tmp/alphapulse/relay.sock"

class SimpleArbitrageMonitor:
    def __init__(self):
        self.pools = defaultdict(dict)
        self.last_update = time.time()
        
    async def connect_unix_socket(self):
        """Connect to exchange-collector's Unix socket"""
        print(f"🔌 Connecting to Unix socket: {UNIX_SOCKET_PATH}")
        
        try:
            reader, writer = await asyncio.open_unix_connection(UNIX_SOCKET_PATH)
            print(f"✅ Unix socket connected")
            
            msg_count = 0
            trade_count = 0
            
            while True:
                try:
                    # Read message header (8 bytes)
                    header = await reader.readexactly(8)
                    if not header:
                        break
                    
                    # Parse header: magic(2) + type(1) + reserved(1) + length(2) + sequence(2)
                    magic = struct.unpack('<H', header[0:2])[0]
                    msg_type = header[2]
                    msg_length = struct.unpack('<H', header[4:6])[0]
                    sequence = struct.unpack('<H', header[6:8])[0]
                    
                    # Read rest of message
                    payload_length = msg_length - 8
                    if payload_length > 0:
                        payload = await reader.readexactly(payload_length)
                    else:
                        payload = b''
                    
                    msg_count += 1
                    
                    # Process based on message type
                    if msg_type == 0x01:  # TRADE
                        trade_count += 1
                        # Parse trade message: exchange(4) + instrument(8) + price(8) + volume(8) + timestamp(8) + side(1) + ...
                        if len(payload) >= 36:
                            exchange = struct.unpack('<I', payload[0:4])[0]
                            instrument = struct.unpack('<Q', payload[4:12])[0]
                            price = struct.unpack('<Q', payload[12:20])[0]
                            volume = struct.unpack('<Q', payload[20:28])[0]
                            timestamp = struct.unpack('<Q', payload[28:36])[0]
                            
                            # Convert price and volume from fixed-point
                            price_float = price / 1e8
                            volume_float = volume / 1e8
                            
                            print(f"📊 Trade #{trade_count}: Exchange: {exchange}, Instrument: {instrument:016x}, Price: ${price_float:.6f}, Volume: {volume_float:.6f}")
                            
                            # Track for arbitrage
                            key = f"{exchange}:{instrument}"
                            self.pools[key] = {
                                'exchange': exchange,
                                'instrument': instrument,
                                'price': price_float,
                                'volume': volume_float,
                                'timestamp': timestamp
                            }
                            
                            # Check for arbitrage opportunities
                            self.check_arbitrage(instrument)
                            
                    elif msg_type == 0x03:  # HEARTBEAT
                        if msg_count % 10 == 0:
                            print(f"💓 Heartbeat #{sequence}")
                    elif msg_type == 0x08:  # SYMBOL_MAPPING
                        pass  # Skip for now
                    elif msg_type == 0x0a:  # STATUS_UPDATE
                        pass  # Skip for now
                    elif msg_type == 0x0b:  # MESSAGE_TRACE
                        pass  # Skip for now
                    else:
                        if msg_count % 100 == 0:
                            print(f"📦 Message type {msg_type:02x}, sequence {sequence}")
                    
                except asyncio.IncompleteReadError:
                    print("Connection closed by server")
                    break
                except Exception as e:
                    print(f"Error processing message: {e}")
                    import traceback
                    traceback.print_exc()
                    await asyncio.sleep(0.1)
                    
        except FileNotFoundError:
            print(f"⚠️  Unix socket not found at {UNIX_SOCKET_PATH}")
            print("Make sure exchange_collector is running with Unix socket enabled")
        except Exception as e:
            print(f"❌ Unix socket connection failed: {e}")
            
    def check_arbitrage(self, instrument):
        """Check for arbitrage opportunities for a given instrument"""
        # Find all exchanges with this instrument
        exchanges = []
        for key, data in self.pools.items():
            if data['instrument'] == instrument:
                exchanges.append(data)
        
        if len(exchanges) >= 2:
            # Sort by price
            exchanges.sort(key=lambda x: x['price'])
            
            # Check spread between lowest and highest
            low = exchanges[0]
            high = exchanges[-1]
            
            if low['price'] > 0:
                spread = (high['price'] - low['price']) / low['price'] * 100
                
                if spread > 0.3:  # 0.3% minimum spread
                    print(f"\n💰 ARBITRAGE OPPORTUNITY!")
                    print(f"   Instrument: {instrument:016x}")
                    print(f"   Buy at Exchange {low['exchange']}: ${low['price']:.6f}")
                    print(f"   Sell at Exchange {high['exchange']}: ${high['price']:.6f}")
                    print(f"   Spread: {spread:.3f}%")
                    print(f"   Max Volume: {min(low['volume'], high['volume']):.6f}\n")
                    
    async def run(self):
        """Main event loop"""
        print("⚡ Simple Arbitrage Monitor")
        print("📊 Listening to Unix socket stream from exchange collector\n")
        
        await self.connect_unix_socket()
        
if __name__ == "__main__":
    monitor = SimpleArbitrageMonitor()
    try:
        asyncio.run(monitor.run())
    except KeyboardInterrupt:
        print("\n👋 Shutting down...")
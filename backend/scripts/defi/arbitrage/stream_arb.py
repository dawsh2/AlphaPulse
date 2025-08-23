#!/usr/bin/env python3
"""Direct stream arbitrage monitor - connects to exchange collector's Unix socket"""

import socket
import struct
import json
import time
from collections import defaultdict

# Unix socket path (from exchange collector)
SOCKET_PATH = "/tmp/alphapulse_trades.sock"

class StreamArbitrage:
    def __init__(self):
        self.pools = defaultdict(dict)  # pool -> {price, liquidity, timestamp}
        self.opportunities = []
        
    def connect_to_stream(self):
        """Connect to Unix socket stream"""
        try:
            # Create Unix socket
            sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            sock.connect(SOCKET_PATH)
            print(f"✅ Connected to stream at {SOCKET_PATH}")
            
            while True:
                # Read message length (4 bytes)
                length_data = sock.recv(4)
                if not length_data:
                    break
                    
                msg_length = struct.unpack('I', length_data)[0]
                
                # Read message
                msg_data = sock.recv(msg_length)
                
                # Parse and process
                self.process_message(msg_data)
                
        except FileNotFoundError:
            print(f"❌ Socket not found at {SOCKET_PATH}")
            print("Make sure exchange-collector is running")
            return False
        except Exception as e:
            print(f"❌ Error: {e}")
            return False
            
    def process_message(self, data):
        """Process stream message"""
        try:
            # Parse message (assuming JSON format)
            msg = json.loads(data)
            
            if 'pool' in msg:
                pool = msg['pool']
                price = msg.get('price', 0)
                liquidity = msg.get('liquidity', 0)
                
                # Update pool state
                self.pools[pool] = {
                    'price': price,
                    'liquidity': liquidity,
                    'timestamp': time.time()
                }
                
                # Check for arbitrage
                self.check_arbitrage()
                
        except Exception as e:
            pass  # Ignore parse errors
            
    def check_arbitrage(self):
        """Check for arbitrage opportunities in real-time"""
        # Compare all pools
        for pool1, data1 in self.pools.items():
            for pool2, data2 in self.pools.items():
                if pool1 >= pool2:
                    continue
                    
                price1 = data1.get('price', 0)
                price2 = data2.get('price', 0)
                
                if price1 <= 0 or price2 <= 0:
                    continue
                    
                spread = abs(price1 - price2) / min(price1, price2) * 100
                
                if spread > 0.5:  # 0.5% minimum
                    print(f"💰 OPPORTUNITY: {spread:.2f}% spread")
                    print(f"   Pool1: {pool1[:10]}... @ {price1:.6f}")
                    print(f"   Pool2: {pool2[:10]}... @ {price2:.6f}")
                    print(f"   Execute NOW!\n")
                    
    def run(self):
        """Main loop"""
        print("🚀 Stream Arbitrage Monitor")
        print("📡 Connecting to exchange collector stream...")
        print("⚡ INSTANT detection - no HTTP calls!\n")
        
        self.connect_to_stream()

if __name__ == "__main__":
    monitor = StreamArbitrage()
    monitor.run()
#!/usr/bin/env python3
"""
Mock data generator to simulate live market data for testing the Unix socket architecture.
This will generate realistic BTC-USD and ETH-USD trade data and send it through the Unix socket pipeline.
"""

import asyncio
import json
import random
import time
import struct
from datetime import datetime
import socket
import os

class MockDataGenerator:
    def __init__(self):
        self.symbols = ["BTC-USD", "ETH-USD"]
        self.prices = {
            "BTC-USD": 95000.0 + random.uniform(-1000, 1000),
            "ETH-USD": 3500.0 + random.uniform(-100, 100)
        }
        self.socket_path = "/tmp/alphapulse/kraken.sock"
        
    async def generate_trade_data(self):
        """Generate realistic trade data"""
        while True:
            symbol = random.choice(self.symbols)
            
            # Create price movement (random walk with slight upward bias)
            price_change = random.uniform(-0.5, 0.6)  # Slight upward bias
            self.prices[symbol] += price_change
            
            # Ensure prices stay realistic
            if symbol == "BTC-USD":
                self.prices[symbol] = max(90000, min(100000, self.prices[symbol]))
            else:  # ETH-USD
                self.prices[symbol] = max(3000, min(4000, self.prices[symbol]))
            
            trade = {
                "msg_type": "trade",
                "timestamp": int(time.time() * 1000),  # milliseconds
                "symbol": symbol,
                "exchange": "kraken",
                "price": round(self.prices[symbol], 2),
                "volume": round(random.uniform(0.001, 2.0), 6),
                "side": random.choice(["buy", "sell"]),
                "data": {
                    "trade_id": f"mock_{int(time.time() * 1000000)}"
                }
            }
            
            await self.send_to_socket(trade)
            await asyncio.sleep(random.uniform(0.1, 0.5))  # 2-10 trades per second
    
    async def send_to_socket(self, data):
        """Send data to Unix socket (simulating what the exchange collector would do)"""
        try:
            # Convert to JSON bytes
            json_data = json.dumps(data).encode('utf-8')
            
            # Connect to relay server's kraken listener
            sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            sock.connect(self.socket_path)
            
            # Send length prefix + data
            length = len(json_data)
            sock.send(struct.pack('!I', length))
            sock.send(json_data)
            sock.close()
            
            print(f"Sent {data['symbol']} trade: ${data['price']:.2f} ({data['side']})")
            
        except Exception as e:
            print(f"Failed to send data: {e}")
            await asyncio.sleep(1)  # Wait before retry

    async def run(self):
        """Start the mock data generator"""
        print("Starting mock data generator...")
        print(f"Generating trades for: {', '.join(self.symbols)}")
        print(f"Sending to Unix socket: {self.socket_path}")
        print("Press Ctrl+C to stop")
        
        await self.generate_trade_data()

if __name__ == "__main__":
    generator = MockDataGenerator()
    try:
        asyncio.run(generator.run())
    except KeyboardInterrupt:
        print("\nStopping mock data generator...")
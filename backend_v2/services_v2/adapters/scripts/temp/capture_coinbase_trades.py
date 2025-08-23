#!/usr/bin/env python3
"""
Temporary script to capture real Coinbase WebSocket trade data for adapter development.

Usage: python capture_coinbase_trades.py --duration 300 --output tests/fixtures/coinbase/

This script connects to live Coinbase WebSocket streams and captures real trade data
for use in developing and validating the Coinbase adapter.

IMPORTANT: This is a temporary utility script. Delete after capturing sufficient samples.
"""

import asyncio
import websockets
import json
import argparse
import os
from datetime import datetime, timezone
import sys

class CoinbaseDataCapture:
    def __init__(self, output_dir: str, duration: int = 300):
        self.output_dir = output_dir
        self.duration = duration
        self.captured_trades = []
        self.captured_l2updates = []
        self.symbols = ["BTC-USD", "ETH-USD", "LTC-USD"]  # Major pairs for testing
        
    async def capture_data(self):
        """Connect to Coinbase WebSocket and capture real market data."""
        
        uri = "wss://ws-feed.exchange.coinbase.com"
        
        # Subscription message for trades and level2 data
        subscribe_message = {
            "type": "subscribe",
            "product_ids": self.symbols,
            "channels": [
                "matches"  # Coinbase uses 'matches' for trade data, not 'trades'
            ]
        }
        
        print(f"🔌 Connecting to Coinbase WebSocket: {uri}")
        print(f"📡 Subscribing to: {self.symbols}")
        print(f"⏱️  Capturing for {self.duration} seconds...")
        
        try:
            async with websockets.connect(uri) as websocket:
                # Send subscription
                await websocket.send(json.dumps(subscribe_message))
                print("✅ Subscription sent")
                
                # Capture data for specified duration
                start_time = asyncio.get_event_loop().time()
                
                while (asyncio.get_event_loop().time() - start_time) < self.duration:
                    try:
                        # Set timeout to avoid hanging
                        raw_message = await asyncio.wait_for(websocket.recv(), timeout=10.0)
                        
                        # Parse message
                        message = json.loads(raw_message)
                        
                        # Process different message types
                        if message.get("type") == "match":
                            self.captured_trades.append({
                                "timestamp_captured": datetime.now(timezone.utc).isoformat(),
                                "raw_message": message
                            })
                            print(f"📊 Trade captured: {message.get('product_id')} - "
                                  f"${message.get('price')} x {message.get('size')}")
                            
                        elif message.get("type") == "l2update":
                            self.captured_l2updates.append({
                                "timestamp_captured": datetime.now(timezone.utc).isoformat(),
                                "raw_message": message
                            })
                            print(f"📈 L2 update captured: {message.get('product_id')}")
                            
                        elif message.get("type") in ["subscriptions", "heartbeat"]:
                            # Control messages - log but don't save
                            print(f"🔧 Control message: {message.get('type')}")
                            
                        elif message.get("type") == "error":
                            print(f"❌ Error message: {message.get('message', 'Unknown error')}")
                            
                        else:
                            print(f"❓ Unknown message type: {message.get('type')}")
                            
                    except asyncio.TimeoutError:
                        print("⏰ WebSocket timeout - continuing...")
                        continue
                    except json.JSONDecodeError as e:
                        print(f"❌ JSON decode error: {e}")
                        continue
                        
        except Exception as e:
            print(f"❌ Connection error: {e}")
            sys.exit(1)
            
    def save_samples(self):
        """Save captured data to fixture files."""
        
        # Ensure output directory exists
        os.makedirs(self.output_dir, exist_ok=True)
        
        # Save trade samples
        if self.captured_trades:
            trade_file = os.path.join(self.output_dir, "trades_real_samples.json")
            with open(trade_file, 'w') as f:
                json.dump(self.captured_trades, f, indent=2)
            print(f"💾 Saved {len(self.captured_trades)} trade samples to {trade_file}")
            
            # Also save just the raw messages for easier parsing during development
            raw_trades_file = os.path.join(self.output_dir, "trades_raw.json")
            raw_trades = [sample["raw_message"] for sample in self.captured_trades]
            with open(raw_trades_file, 'w') as f:
                json.dump(raw_trades, f, indent=2)
            print(f"💾 Saved raw trade messages to {raw_trades_file}")
        
        # Save L2 update samples  
        if self.captured_l2updates:
            l2_file = os.path.join(self.output_dir, "l2updates_real_samples.json")
            with open(l2_file, 'w') as f:
                json.dump(self.captured_l2updates, f, indent=2)
            print(f"💾 Saved {len(self.captured_l2updates)} L2 update samples to {l2_file}")
            
        # Generate analysis report
        self.generate_analysis_report()
        
    def generate_analysis_report(self):
        """Generate analysis of captured data for documentation."""
        
        analysis = {
            "capture_session": {
                "timestamp": datetime.now(timezone.utc).isoformat(),
                "duration_seconds": self.duration,
                "symbols_monitored": self.symbols
            },
            "trade_data_analysis": {},
            "l2_data_analysis": {}
        }
        
        # Analyze trade messages
        if self.captured_trades:
            sample_trade = self.captured_trades[0]["raw_message"]
            analysis["trade_data_analysis"] = {
                "message_count": len(self.captured_trades),
                "sample_message_structure": sample_trade,
                "observed_fields": list(sample_trade.keys()),
                "price_format": type(sample_trade.get("price", "")).__name__,
                "size_format": type(sample_trade.get("size", "")).__name__,
                "time_format": type(sample_trade.get("time", "")).__name__
            }
            
        # Analyze L2 messages
        if self.captured_l2updates:
            sample_l2 = self.captured_l2updates[0]["raw_message"]
            analysis["l2_data_analysis"] = {
                "message_count": len(self.captured_l2updates),
                "sample_message_structure": sample_l2,
                "observed_fields": list(sample_l2.keys())
            }
            
        # Save analysis
        analysis_file = os.path.join(self.output_dir, "data_analysis_report.json")
        with open(analysis_file, 'w') as f:
            json.dump(analysis, f, indent=2)
        print(f"📋 Data analysis report saved to {analysis_file}")

async def main():
    parser = argparse.ArgumentParser(description="Capture real Coinbase WebSocket data")
    parser.add_argument("--duration", type=int, default=300, 
                       help="Capture duration in seconds (default: 300)")
    parser.add_argument("--output", type=str, default="tests/fixtures/coinbase/",
                       help="Output directory for captured data")
    
    args = parser.parse_args()
    
    print("🔥 Coinbase Data Capture Utility")
    print("=" * 50)
    print("This script captures REAL live data from Coinbase WebSocket streams")
    print("for use in developing and validating the Coinbase adapter.")
    print("=" * 50)
    
    # Create capture instance
    capture = CoinbaseDataCapture(args.output, args.duration)
    
    # Capture data
    await capture.capture_data()
    
    # Save samples
    capture.save_samples()
    
    print("\n✅ Data capture complete!")
    print(f"📁 Check {args.output} for captured samples")
    print("\n🗑️  Remember to delete this script after capturing sufficient data")

if __name__ == "__main__":
    asyncio.run(main())
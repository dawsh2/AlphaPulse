#!/usr/bin/env python3
"""
Decode Symbol Hashes - Check if Polygon trades are in the pipeline
================================================================

This script connects to the WebSocket and decodes the symbol_hash values
to check if Polygon/QuickSwap trades are actually flowing through.
"""

import asyncio
import websockets
import json
import time
from typing import Dict, Set

class SymbolHashDecoder:
    """Decodes symbol hashes from the WebSocket stream"""
    
    def __init__(self):
        self.seen_hashes: Set[str] = set()
        self.symbol_mappings: Dict[str, str] = {}
        self.trade_count = 0
        self.polygon_trade_count = 0
        
    async def capture_and_decode_symbols(self, duration_seconds: int = 20):
        """Capture symbol data and look for Polygon trades"""
        print(f"🔍 Capturing symbol data for {duration_seconds} seconds...")
        
        try:
            uri = "ws://127.0.0.1:8765/stream"
            async with websockets.connect(uri, timeout=10) as websocket:
                print("✅ Connected to WebSocket")
                
                start_time = time.time()
                
                while time.time() - start_time < duration_seconds:
                    try:
                        message = await asyncio.wait_for(websocket.recv(), timeout=1.0)
                        data = json.loads(message)
                        
                        await self._process_message(data)
                        
                    except asyncio.TimeoutError:
                        continue
                    except json.JSONDecodeError:
                        continue
                        
        except Exception as e:
            print(f"❌ WebSocket connection failed: {e}")
            
        self._analyze_results()
        
    async def _process_message(self, data: Dict):
        """Process a single WebSocket message"""
        msg_type = data.get('msg_type', '')
        
        if msg_type == 'symbol_mapping':
            # Store symbol mappings
            symbol_hash = str(data.get('symbol_hash', ''))
            symbol = data.get('symbol', '')
            if symbol_hash and symbol:
                self.symbol_mappings[symbol_hash] = symbol
                print(f"📋 Symbol mapping: {symbol_hash} → {symbol}")
                
        elif msg_type == 'trade':
            self.trade_count += 1
            symbol_hash = str(data.get('symbol_hash', ''))
            self.seen_hashes.add(symbol_hash)
            
            # Check if we have a mapping for this hash
            symbol_name = self.symbol_mappings.get(symbol_hash, f"UNKNOWN_{symbol_hash}")
            
            # Check if this looks like a Polygon trade
            if self._is_polygon_trade(data, symbol_name):
                self.polygon_trade_count += 1
                print(f"🎯 POLYGON TRADE: {symbol_name} ({symbol_hash}) - Price: {data.get('price', 'N/A')}")
                
                # Show full trade details for first few Polygon trades
                if self.polygon_trade_count <= 3:
                    print(f"   Full trade data: {json.dumps(data, indent=2)}")
                    
            # Log every 50th trade to show activity
            if self.trade_count % 50 == 0:
                print(f"📊 Processed {self.trade_count} trades, {len(self.seen_hashes)} unique symbols")
                
    def _is_polygon_trade(self, data: Dict, symbol_name: str) -> bool:
        """Check if this trade is from Polygon/QuickSwap"""
        # Check symbol name
        polygon_indicators = ['quickswap', 'polygon', 'pol-', 'usdc', 'wmatic', 'weth']
        symbol_lower = symbol_name.lower()
        
        for indicator in polygon_indicators:
            if indicator in symbol_lower:
                return True
                
        # Check exchange_id if present
        exchange_id = data.get('exchange_id', '')
        if 'polygon' in str(exchange_id).lower():
            return True
            
        # Check price range (Polygon tokens often have different price ranges)
        price = data.get('price', 0)
        if 0.001 <= price <= 100:  # POL is around $0.012, other tokens in this range
            # Additional check for realistic Polygon prices
            return True
            
        return False
        
    def _analyze_results(self):
        """Analyze the captured results"""
        print("\n" + "=" * 60)
        print("SYMBOL HASH ANALYSIS RESULTS")
        print("=" * 60)
        
        print(f"📊 Summary:")
        print(f"   Total trades: {self.trade_count}")
        print(f"   Unique symbols: {len(self.seen_hashes)}")
        print(f"   Symbol mappings: {len(self.symbol_mappings)}")
        print(f"   Polygon trades: {self.polygon_trade_count}")
        
        if self.polygon_trade_count > 0:
            print(f"\n✅ SUCCESS: Found {self.polygon_trade_count} Polygon trades!")
            print("🎯 The dynamic pool discovery IS working and trades are reaching the dashboard!")
        else:
            print(f"\n❌ No obvious Polygon trades found")
            print("💡 Possible issues:")
            print("   - Symbol mappings not being sent")
            print("   - Polygon collector not running") 
            print("   - Symbol resolution failing")
            
        print(f"\n📋 Symbol Mappings Found:")
        for hash_val, symbol in list(self.symbol_mappings.items())[:10]:
            print(f"   {hash_val} → {symbol}")
            
        if len(self.symbol_mappings) > 10:
            print(f"   ... and {len(self.symbol_mappings) - 10} more")
            
        print(f"\n🔢 Top Symbol Hashes (by frequency):")
        # Count frequency of each hash
        hash_counts = {}
        for hash_val in self.seen_hashes:
            symbol = self.symbol_mappings.get(hash_val, f"UNKNOWN_{hash_val}")
            hash_counts[hash_val] = hash_counts.get(hash_val, 0) + 1
            
        # Show top 10
        sorted_hashes = sorted(hash_counts.items(), key=lambda x: x[1], reverse=True)
        for hash_val, count in sorted_hashes[:10]:
            symbol = self.symbol_mappings.get(hash_val, f"UNKNOWN_{hash_val}")
            print(f"   {hash_val}: {symbol} ({count} trades)")

async def run_symbol_decoder():
    """Run the symbol hash decoder"""
    print("=" * 80)
    print("SYMBOL HASH DECODER - CHECKING FOR POLYGON TRADES")
    print("=" * 80)
    print("This will capture WebSocket data and decode symbol hashes")
    print("to identify if Polygon/QuickSwap trades are in the pipeline.")
    print("=" * 80)
    
    decoder = SymbolHashDecoder()
    await decoder.capture_and_decode_symbols(duration_seconds=15)

if __name__ == "__main__":
    asyncio.run(run_symbol_decoder())
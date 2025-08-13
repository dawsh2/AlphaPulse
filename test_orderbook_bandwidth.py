#!/usr/bin/env python3
"""Test script to measure orderbook bandwidth reduction"""
import redis
import json
import time
from collections import defaultdict

def measure_orderbook_bandwidth():
    """Measure the size difference between full orderbooks and what deltas would be"""
    r = redis.Redis(host='localhost', port=6379)
    
    print("📊 Measuring OrderBook Bandwidth Usage")
    print("=" * 50)
    
    # Get orderbook streams
    streams = [s.decode() for s in r.keys("orderbooks:*")][:4]  # First 4 streams
    
    total_full_size = 0
    total_delta_size = 0
    update_count = 0
    symbol_stats = defaultdict(lambda: {'full': 0, 'delta': 0, 'updates': 0})
    
    for stream in streams:
        try:
            # Get last 20 orderbook updates
            updates = r.xrevrange(stream, count=20)
            
            previous_levels = None
            
            for msg_id, fields_dict in updates:
                try:
                    # Parse the orderbook
                    bids_str = fields_dict.get(b'bids', b'[]').decode()
                    asks_str = fields_dict.get(b'asks', b'[]').decode()
                    symbol = fields_dict.get(b'symbol', b'unknown').decode()
                    
                    bids = json.loads(bids_str) if bids_str != '[]' else []
                    asks = json.loads(asks_str) if asks_str != '[]' else []
                    
                    # Calculate full orderbook size
                    full_size = len(json.dumps({'bids': bids, 'asks': asks}).encode())
                    total_full_size += full_size
                    symbol_stats[symbol]['full'] += full_size
                    
                    # Calculate delta size (if we had previous levels)
                    delta_size = full_size  # First update is full size
                    if previous_levels is not None:
                        prev_bids, prev_asks = previous_levels
                        
                        # Find changes
                        bid_changes = []
                        ask_changes = []
                        
                        # Find bid changes (simplified delta calculation)
                        bid_dict = {level[0]: level[1] for level in bids}
                        prev_bid_dict = {level[0]: level[1] for level in prev_bids}
                        
                        for price, size in bid_dict.items():
                            if price not in prev_bid_dict or prev_bid_dict[price] != size:
                                bid_changes.append([price, size])
                        
                        for price in prev_bid_dict:
                            if price not in bid_dict:
                                bid_changes.append([price, 0])  # Removal
                        
                        # Same for asks
                        ask_dict = {level[0]: level[1] for level in asks}
                        prev_ask_dict = {level[0]: level[1] for level in prev_asks}
                        
                        for price, size in ask_dict.items():
                            if price not in prev_ask_dict or prev_ask_dict[price] != size:
                                ask_changes.append([price, size])
                        
                        for price in prev_ask_dict:
                            if price not in ask_dict:
                                ask_changes.append([price, 0])
                        
                        # Delta size is much smaller
                        delta = {'bid_changes': bid_changes, 'ask_changes': ask_changes}
                        delta_size = len(json.dumps(delta).encode())
                    
                    total_delta_size += delta_size
                    symbol_stats[symbol]['delta'] += delta_size
                    symbol_stats[symbol]['updates'] += 1
                    
                    previous_levels = (bids.copy(), asks.copy())
                    update_count += 1\n                    \n                except Exception as e:\n                    print(f\"Error parsing message: {e}\")\n                    continue\n                    \n        except Exception as e:\n            print(f\"Error reading stream {stream}: {e}\")\n            continue\n    \n    if update_count > 0:\n        avg_full_size = total_full_size / update_count\n        avg_delta_size = total_delta_size / update_count\n        reduction_ratio = ((total_full_size - total_delta_size) / total_full_size) * 100\n        \n        print(f\"\\n📈 Bandwidth Analysis ({update_count} updates):\")\n        print(f\"   Full OrderBooks: {total_full_size:,} bytes\")\n        print(f\"   Delta Updates: {total_delta_size:,} bytes\")\n        print(f\"   Average Full Size: {avg_full_size:.0f} bytes\")\n        print(f\"   Average Delta Size: {avg_delta_size:.0f} bytes\")\n        print(f\"   🎯 Bandwidth Reduction: {reduction_ratio:.1f}%\")\n        print(f\"   📊 Compression Ratio: {total_full_size/total_delta_size:.1f}x\")\n        \n        print(f\"\\n📋 Per-Symbol Breakdown:\")\n        for symbol, stats in symbol_stats.items():\n            if stats['updates'] > 0:\n                symbol_reduction = ((stats['full'] - stats['delta']) / stats['full']) * 100\n                print(f\"   {symbol}: {symbol_reduction:.1f}% reduction ({stats['updates']} updates)\")\n    else:\n        print(\"❌ No orderbook data found\")\n    \n    print(\"\\n\" + \"=\"*50)\n    print(\"✅ Analysis complete!\")\n\nif __name__ == \"__main__\":\n    measure_orderbook_bandwidth()
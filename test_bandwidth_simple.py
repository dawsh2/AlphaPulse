#!/usr/bin/env python3
"""Simple test script to measure orderbook bandwidth reduction"""
import redis
import json

def test_bandwidth_reduction():
    print("🎯 OrderBook Bandwidth Reduction Analysis")
    print("=" * 50)
    
    r = redis.Redis(host='localhost', port=6379)
    
    # Get one orderbook stream
    stream = "orderbooks:coinbase:BTC-USD"
    
    try:
        # Get last 10 updates
        updates = r.xrevrange(stream, count=10)
        
        if not updates:
            print("❌ No orderbook data found")
            return
            
        print(f"📊 Analyzing {len(updates)} updates from {stream}")
        
        total_full_size = 0
        estimated_delta_size = 0
        
        previous_bids = None
        previous_asks = None
        
        for i, (msg_id, fields) in enumerate(updates):
            # Parse orderbook
            bids_str = fields.get(b'bids', b'[]').decode()
            asks_str = fields.get(b'asks', b'[]').decode()
            
            bids = json.loads(bids_str) if bids_str != '[]' else []
            asks = json.loads(asks_str) if asks_str != '[]' else []
            
            # Calculate full orderbook size
            full_book = {'bids': bids, 'asks': asks}
            full_size = len(json.dumps(full_book).encode())
            total_full_size += full_size
            
            # Estimate delta size
            if previous_bids is None:
                # First update is always full size
                estimated_delta_size += full_size
            else:
                # Count changes
                changes = 0
                
                # Compare bids
                bid_set = set(tuple(level) for level in bids)
                prev_bid_set = set(tuple(level) for level in previous_bids)
                changes += len(bid_set.symmetric_difference(prev_bid_set))
                
                # Compare asks  
                ask_set = set(tuple(level) for level in asks)
                prev_ask_set = set(tuple(level) for level in previous_asks)
                changes += len(ask_set.symmetric_difference(prev_ask_set))
                
                # Each change is about 24 bytes [price, volume] + metadata
                delta_size = max(50, changes * 24 + 50)  # Minimum 50 bytes overhead
                estimated_delta_size += delta_size
                
                if i < 3:  # Show first few examples
                    print(f"   Update {i+1}: {full_size} bytes full, ~{delta_size} bytes delta ({changes} changes)")
            
            previous_bids = bids.copy()
            previous_asks = asks.copy()
        
        # Calculate results
        avg_full = total_full_size / len(updates)
        avg_delta = estimated_delta_size / len(updates)
        reduction = ((total_full_size - estimated_delta_size) / total_full_size) * 100
        compression = total_full_size / estimated_delta_size
        
        print(f"\n📈 Results:")
        print(f"   Full OrderBooks: {total_full_size:,} bytes")
        print(f"   Estimated Deltas: {estimated_delta_size:,} bytes") 
        print(f"   🎯 Bandwidth Reduction: {reduction:.1f}%")
        print(f"   📊 Compression Ratio: {compression:.1f}x")
        print(f"   Average Full: {avg_full:.0f} bytes")
        print(f"   Average Delta: {avg_delta:.0f} bytes")
        
    except Exception as e:
        print(f"❌ Error: {e}")
    
    print("\n" + "=" * 50)

if __name__ == "__main__":
    test_bandwidth_reduction()
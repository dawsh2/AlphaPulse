#!/usr/bin/env python3
"""
Fetch complete Kraken data and save to JSON
Avoids DuckDB lock issues
"""

import requests
import time
import json
from datetime import datetime

def fetch_kraken_batch(start_time):
    """Fetch a batch of Kraken data"""
    try:
        url = "http://localhost:5002/api/proxy/kraken/0/public/OHLC"
        params = {
            'pair': 'XBTUSD',
            'interval': 1,  # 1 minute
            'since': int(start_time)
        }
        
        response = requests.get(url, params=params, timeout=10)
        data = response.json()
        
        if data.get('error'):
            if 'Too many requests' in str(data['error']):
                print(f"  Rate limited, waiting 10 seconds...")
                time.sleep(10)
                return [], None
            print(f"  API error: {data['error']}")
            return [], None
        
        result = data.get('result', {})
        pair_key = next((k for k in result.keys() if 'XBT' in k and k != 'last'), None)
        
        if not pair_key:
            return [], None
        
        ohlc_data = result[pair_key]
        
        if not ohlc_data:
            return [], None
        
        # Convert to our format
        bars = []
        for candle in ohlc_data:
            bars.append({
                'timestamp': int(candle[0]),
                'symbol': 'BTC/USD',
                'exchange': 'kraken',
                'open': float(candle[1]),
                'high': float(candle[2]),
                'low': float(candle[3]),
                'close': float(candle[4]),
                'volume': float(candle[6])
            })
        
        last_timestamp = int(ohlc_data[-1][0]) if ohlc_data else None
        return bars, last_timestamp
        
    except Exception as e:
        print(f"  Error: {e}")
        return [], None

def main():
    print("=" * 60)
    print("KRAKEN DATA FETCHER TO JSON")
    print("=" * 60)
    
    # Target: Match Coinbase start time (2025-08-03 17:32:00)
    # That's timestamp 1754347920
    coinbase_start = 1754347920
    current_time = int(time.time())
    
    print(f"Fetching from {datetime.fromtimestamp(coinbase_start)} to now")
    print("This will fetch approximately 11,000+ bars to match Coinbase")
    
    all_data = []
    fetch_start = coinbase_start
    batch_num = 0
    
    print("\nFetching in batches...")
    print("-" * 40)
    
    while fetch_start < current_time and batch_num < 30:
        batch_num += 1
        print(f"\nBatch {batch_num}: {datetime.fromtimestamp(fetch_start)}")
        
        bars, last_ts = fetch_kraken_batch(fetch_start)
        
        if not bars:
            print("  No data, waiting 3 seconds...")
            time.sleep(3)
            # Try once more
            bars, last_ts = fetch_kraken_batch(fetch_start)
            
            if not bars:
                print("  Still no data, moving forward 12 hours")
                fetch_start += (12 * 3600)
                continue
        
        print(f"  Got {len(bars)} bars (total: {len(all_data) + len(bars)})")
        
        # Add only unique bars
        existing_timestamps = {bar['timestamp'] for bar in all_data}
        new_bars = [bar for bar in bars if bar['timestamp'] not in existing_timestamps]
        all_data.extend(new_bars)
        print(f"  Added {len(new_bars)} unique bars")
        
        # Move to next batch
        if last_ts and last_ts > fetch_start:
            fetch_start = last_ts + 60
        else:
            fetch_start += (12 * 3600)
        
        # Rate limiting
        time.sleep(1.5)
        
        # Check if we have enough data
        if len(all_data) >= 11000:
            print(f"\nReached target of 11,000+ bars")
            break
    
    print("\n" + "-" * 40)
    print(f"Fetch complete! Total bars: {len(all_data)}")
    
    if all_data:
        # Sort by timestamp
        all_data.sort(key=lambda x: x['timestamp'])
        
        # Save to JSON
        output_file = '../market_data/kraken_complete.json'
        with open(output_file, 'w') as f:
            json.dump({
                'exchange': 'kraken',
                'symbol': 'BTC/USD',
                'interval': '1m',
                'bar_count': len(all_data),
                'first_timestamp': all_data[0]['timestamp'],
                'last_timestamp': all_data[-1]['timestamp'],
                'first_datetime': datetime.fromtimestamp(all_data[0]['timestamp']).isoformat(),
                'last_datetime': datetime.fromtimestamp(all_data[-1]['timestamp']).isoformat(),
                'data': all_data
            }, f, indent=2)
        
        print(f"\nData saved to: {output_file}")
        print(f"First bar: {datetime.fromtimestamp(all_data[0]['timestamp'])}")
        print(f"Last bar: {datetime.fromtimestamp(all_data[-1]['timestamp'])}")
        print(f"\nNext step: Run import_json_to_duckdb.py to load into database")

if __name__ == "__main__":
    main()
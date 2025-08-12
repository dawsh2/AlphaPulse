#!/usr/bin/env python3
"""
Fetch Kraken data and save it directly without DuckDB conflicts
Saves to JSON first, then can be imported later
"""

import requests
import time
import json
import pandas as pd
from datetime import datetime

def fetch_kraken_batch(start_time):
    """Fetch a batch of Kraken data starting from start_time"""
    try:
        # Use the Flask proxy endpoint on port 5002
        url = "http://localhost:5002/api/proxy/kraken/0/public/OHLC"
        params = {
            'pair': 'XBTUSD',
            'interval': 1,  # 1 minute bars
            'since': int(start_time)
        }
        
        print(f"  Fetching from {datetime.fromtimestamp(start_time)}...")
        response = requests.get(url, params=params, timeout=10)
        data = response.json()
        
        if data.get('error'):
            print(f"  Kraken API error: {data['error']}")
            return [], None
        
        result = data.get('result', {})
        # Find the data key (usually XXBTZUSD)
        pair_key = next((k for k in result.keys() if 'XBT' in k and k != 'last'), None)
        
        if not pair_key:
            print("  No data key found")
            return [], None
        
        ohlc_data = result[pair_key]
        
        if not ohlc_data:
            print("  No data returned")
            return [], None
        
        print(f"  Got {len(ohlc_data)} bars")
        
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
        
        # Return data and timestamp of last bar
        last_timestamp = int(ohlc_data[-1][0]) if ohlc_data else None
        return bars, last_timestamp
        
    except Exception as e:
        print(f"  Error fetching batch: {e}")
        return [], None

def main():
    """Fetch Kraken data for the last 7 days and save to JSON"""
    print("=" * 60)
    print("KRAKEN DATA FETCHER (JSON Output)")
    print("=" * 60)
    
    # We know we need about 10,517 bars to match Coinbase
    # Kraken returns ~720 bars per request
    # So we need to make multiple requests going back in time
    
    current_time = int(time.time())
    all_data = []
    
    # Start from 7 days ago and work forward
    periods_to_fetch = [
        (current_time - (7 * 24 * 3600), "7 days ago"),
        (current_time - (6 * 24 * 3600), "6 days ago"),
        (current_time - (5 * 24 * 3600), "5 days ago"),
        (current_time - (4 * 24 * 3600), "4 days ago"),
        (current_time - (3 * 24 * 3600), "3 days ago"),
        (current_time - (2 * 24 * 3600), "2 days ago"),
        (current_time - (1 * 24 * 3600), "1 day ago"),
        (current_time - (12 * 3600), "12 hours ago"),
        (current_time - (6 * 3600), "6 hours ago"),
        (current_time - (3 * 3600), "3 hours ago"),
        (current_time - (1 * 3600), "1 hour ago"),
    ]
    
    for start_time, description in periods_to_fetch:
        print(f"\nFetching from {description}:")
        batch_data, last_timestamp = fetch_kraken_batch(start_time)
        
        if batch_data:
            # Only add new data (no duplicates based on timestamp)
            existing_timestamps = {bar['timestamp'] for bar in all_data}
            new_bars = [bar for bar in batch_data if bar['timestamp'] not in existing_timestamps]
            all_data.extend(new_bars)
            print(f"  Added {len(new_bars)} new bars (total: {len(all_data)})")
        
        # Rate limiting
        time.sleep(1.5)
    
    print("\n" + "-" * 40)
    print(f"Fetch complete! Total unique bars: {len(all_data)}")
    
    if all_data:
        # Sort by timestamp
        all_data.sort(key=lambda x: x['timestamp'])
        
        # Save to JSON file
        output_file = '../market_data/kraken_data_full.json'
        with open(output_file, 'w') as f:
            json.dump({
                'symbol': 'BTC/USD',
                'exchange': 'kraken',
                'interval': '1m',
                'bars_count': len(all_data),
                'first_timestamp': all_data[0]['timestamp'],
                'last_timestamp': all_data[-1]['timestamp'],
                'data': all_data
            }, f, indent=2)
        
        print(f"\nData saved to: {output_file}")
        print(f"First bar: {datetime.fromtimestamp(all_data[0]['timestamp'])}")
        print(f"Last bar: {datetime.fromtimestamp(all_data[-1]['timestamp'])}")
        print(f"\nTo import this data into DuckDB later, use the import_kraken_json.py script")
    else:
        print("\nNo data was fetched")

if __name__ == "__main__":
    main()
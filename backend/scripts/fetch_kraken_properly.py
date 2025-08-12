#!/usr/bin/env python3
"""
Fetch Kraken OHLC data properly with pagination
Uses correct symbol XXBTZUSD and handles the 720 bar limit
"""

import requests
import time
import pandas as pd
from datetime import datetime, timedelta
import json

def fetch_kraken_ohlc(since_timestamp):
    """
    Fetch OHLC data from Kraken
    Returns up to 720 bars starting from since_timestamp
    """
    url = "https://api.kraken.com/0/public/OHLC"
    params = {
        'pair': 'XXBTZUSD',  # Correct symbol for BTC/USD
        'interval': 1,        # 1 minute
        'since': int(since_timestamp)
    }
    
    try:
        response = requests.get(url, params=params, timeout=10)
        data = response.json()
        
        if data.get('error') and data['error']:
            print(f"  Error: {data['error']}")
            return None, None
        
        result = data.get('result', {})
        
        # The data is under 'XXBTZUSD' key
        ohlc_data = result.get('XXBTZUSD', [])
        
        if not ohlc_data:
            print("  No data returned")
            return None, None
        
        print(f"  Got {len(ohlc_data)} bars")
        
        # Return data and the timestamp of the last bar
        last_timestamp = int(ohlc_data[-1][0]) if ohlc_data else None
        
        return ohlc_data, last_timestamp
        
    except Exception as e:
        print(f"  Error fetching: {e}")
        return None, None

def main():
    print("=" * 60)
    print("KRAKEN OHLC FETCHER - Proper Pagination")
    print("=" * 60)
    
    # Calculate 10 days ago (to exceed our 7-day Coinbase data)
    days_back = 10
    current_time = int(time.time())
    start_time = current_time - (days_back * 24 * 3600)
    
    print(f"Fetching {days_back} days of 1-minute data")
    print(f"From: {datetime.fromtimestamp(start_time)}")
    print(f"To: {datetime.fromtimestamp(current_time)}")
    print(f"Expected bars: ~{days_back * 24 * 60} (14,400 for 10 days)")
    print("-" * 40)
    
    all_bars = []
    since = start_time
    batch_num = 0
    
    while since < current_time:
        batch_num += 1
        print(f"\nBatch {batch_num}: Fetching from {datetime.fromtimestamp(since)}")
        
        ohlc_data, last_ts = fetch_kraken_ohlc(since)
        
        if not ohlc_data:
            print("  No data, waiting 2 seconds and retrying...")
            time.sleep(2)
            ohlc_data, last_ts = fetch_kraken_ohlc(since)
            
            if not ohlc_data:
                print("  Still no data, stopping")
                break
        
        # Convert to our format
        for bar in ohlc_data:
            all_bars.append({
                'timestamp': int(bar[0]),
                'open': float(bar[1]),
                'high': float(bar[2]),
                'low': float(bar[3]),
                'close': float(bar[4]),
                'volume': float(bar[6]),  # Skip VWAP at index 5
                'symbol': 'BTC/USD',
                'exchange': 'kraken'
            })
        
        print(f"  Total bars so far: {len(all_bars)}")
        
        # Move to next batch
        if last_ts:
            since = last_ts + 60  # Start from 1 minute after last bar
        else:
            # If no last timestamp, move forward by estimated batch size
            since = since + (720 * 60)  # 720 minutes forward
        
        # Stop if we've reached current time
        if since >= current_time:
            print("  Reached current time")
            break
        
        # Rate limiting - Kraken allows 1 request per second for public data
        time.sleep(1.5)
        
        # Safety limit
        if batch_num >= 30:
            print("  Reached batch limit")
            break
    
    print("\n" + "=" * 60)
    print(f"FETCH COMPLETE: {len(all_bars)} bars")
    print("=" * 60)
    
    if all_bars:
        # Remove duplicates based on timestamp
        unique_bars = []
        seen_timestamps = set()
        for bar in all_bars:
            if bar['timestamp'] not in seen_timestamps:
                unique_bars.append(bar)
                seen_timestamps.add(bar['timestamp'])
        
        all_bars = unique_bars
        print(f"Unique bars: {len(all_bars)}")
        
        # Sort by timestamp
        all_bars.sort(key=lambda x: x['timestamp'])
        
        # Save to JSON
        output_file = '../market_data/kraken_ohlc_10days.json'
        with open(output_file, 'w') as f:
            json.dump({
                'exchange': 'kraken',
                'symbol': 'BTC/USD',
                'interval': '1m',
                'bar_count': len(all_bars),
                'first_timestamp': all_bars[0]['timestamp'],
                'last_timestamp': all_bars[-1]['timestamp'],
                'first_datetime': datetime.fromtimestamp(all_bars[0]['timestamp']).isoformat(),
                'last_datetime': datetime.fromtimestamp(all_bars[-1]['timestamp']).isoformat(),
                'data': all_bars
            }, f, indent=2)
        
        print(f"\nData saved to: {output_file}")
        print(f"First bar: {datetime.fromtimestamp(all_bars[0]['timestamp'])}")
        print(f"Last bar: {datetime.fromtimestamp(all_bars[-1]['timestamp'])}")
        print(f"Total bars: {len(all_bars)}")
        
        # Compare with what we need
        coinbase_start = datetime(2025, 8, 3, 17, 32)  # Coinbase starts Aug 3
        our_start = datetime.fromtimestamp(all_bars[0]['timestamp'])
        
        if our_start <= coinbase_start:
            print(f"\n✅ Success! We have data from before Coinbase start ({coinbase_start})")
        else:
            print(f"\n⚠️  Our data starts at {our_start}, after Coinbase ({coinbase_start})")
            
        print("\nNext step: Run import_json_to_duckdb.py ../market_data/kraken_ohlc_10days.json")

if __name__ == "__main__":
    main()
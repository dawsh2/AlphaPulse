#!/usr/bin/env python3
"""
Simple script to fetch recent Kraken data to match Coinbase
Can be run from the Develop page terminal
"""

import requests
import time
import json

def fetch_kraken_recent(hours=24):
    """Fetch recent Kraken data for the last N hours"""
    
    # Calculate timestamps
    end_time = int(time.time())
    start_time = end_time - (hours * 3600)
    
    print(f"Fetching last {hours} hours of Kraken BTC/USD data...")
    print(f"From: {time.strftime('%Y-%m-%d %H:%M:%S', time.localtime(start_time))}")
    print(f"To: {time.strftime('%Y-%m-%d %H:%M:%S', time.localtime(end_time))}")
    
    all_candles = []
    current_since = start_time
    
    while current_since < end_time:
        try:
            # Direct Kraken API call
            url = "https://api.kraken.com/0/public/OHLC"
            params = {
                'pair': 'XBTUSD',
                'interval': 1,  # 1 minute
                'since': current_since
            }
            
            response = requests.get(url, params=params, timeout=10)
            data = response.json()
            
            if data.get('error'):
                print(f"Error: {data['error']}")
                break
            
            result = data.get('result', {})
            # Kraken returns data under a key like 'XXBTZUSD'
            pair_key = next((k for k in result.keys() if 'XBT' in k), None)
            
            if not pair_key:
                print("No data found")
                break
                
            ohlc_data = result[pair_key]
            
            if not ohlc_data:
                break
                
            print(f"Fetched {len(ohlc_data)} candles...")
            
            # Convert to our format
            for candle in ohlc_data:
                all_candles.append([
                    int(candle[0]),      # timestamp
                    float(candle[1]),    # open
                    float(candle[2]),    # high  
                    float(candle[3]),    # low
                    float(candle[4]),    # close
                    float(candle[6])     # volume
                ])
            
            # Update for next request
            if ohlc_data:
                current_since = int(ohlc_data[-1][0]) + 60
            
            # Rate limit
            time.sleep(1)
            
            # Stop if we've reached the end time
            if current_since >= end_time:
                break
                
        except Exception as e:
            print(f"Error: {e}")
            break
    
    print(f"\nTotal candles fetched: {len(all_candles)}")
    
    # Save to file for easy import
    output_file = '/tmp/kraken_data.json'
    with open(output_file, 'w') as f:
        json.dump({
            'symbol': 'BTC/USD',
            'exchange': 'kraken', 
            'interval': '1m',
            'candles': all_candles
        }, f)
    
    print(f"Data saved to: {output_file}")
    print("\nTo import this data, run:")
    print("import json")
    print("with open('/tmp/kraken_data.json') as f:")
    print("    data = json.load(f)")
    print("# Then save to your database")
    
    return all_candles

if __name__ == "__main__":
    # Fetch last 7 days of data
    fetch_kraken_recent(hours=24*7)
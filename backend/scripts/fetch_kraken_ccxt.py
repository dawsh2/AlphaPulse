#!/usr/bin/env python3
"""
Fetch Kraken data using ccxt library
This should properly handle pagination and historical data
"""

import ccxt
import pandas as pd
import time
import json
from datetime import datetime

def main():
    print("=" * 60)
    print("KRAKEN FETCHER using CCXT")
    print("=" * 60)
    
    # Connect to Kraken
    exchange = ccxt.kraken()
    
    symbol = 'BTC/USD'
    timeframe = '1m'
    limit = 1000  # Max bars per request (Kraken's limit is actually 720)
    
    # Calculate since timestamp (10 days ago in milliseconds)
    days_back = 10
    since = int((time.time() - days_back * 24 * 60 * 60) * 1000)
    
    print(f"Fetching {days_back} days of {timeframe} data for {symbol}")
    print(f"Starting from: {datetime.fromtimestamp(since/1000)}")
    print("-" * 40)
    
    # Fetch data with pagination
    ohlcv = []
    batch_num = 0
    
    while True:
        batch_num += 1
        print(f"\nBatch {batch_num}: Fetching from {datetime.fromtimestamp(since/1000)}")
        
        try:
            candles = exchange.fetch_ohlcv(symbol, timeframe, since, limit)
            
            if not candles:
                print("  No more data")
                break
            
            print(f"  Got {len(candles)} candles")
            ohlcv.extend(candles)
            
            # Update since to the last candle's timestamp + 1 minute
            since = candles[-1][0] + 60 * 1000
            
            print(f"  Total candles so far: {len(ohlcv)}")
            
            # If we got less than the limit, we've reached the end
            if len(candles) < limit:
                print("  Reached end of available data")
                break
            
            # Check if we've reached current time
            if since >= int(time.time() * 1000):
                print("  Reached current time")
                break
            
            # Rate limiting - be respectful
            time.sleep(1)
            
        except Exception as e:
            print(f"  Error: {e}")
            break
        
        # Safety limit
        if batch_num >= 30:
            print("  Reached batch limit")
            break
    
    print("\n" + "=" * 60)
    print(f"FETCH COMPLETE: {len(ohlcv)} candles")
    print("=" * 60)
    
    if ohlcv:
        # Convert to DataFrame
        df = pd.DataFrame(ohlcv, columns=['timestamp', 'open', 'high', 'low', 'close', 'volume'])
        df['timestamp'] = df['timestamp'] // 1000  # Convert to seconds
        df['datetime'] = pd.to_datetime(df['timestamp'], unit='s')
        df['symbol'] = 'BTC/USD'
        df['exchange'] = 'kraken'
        
        print(f"\nData range:")
        print(f"  First: {df['datetime'].min()}")
        print(f"  Last: {df['datetime'].max()}")
        print(f"  Total bars: {len(df)}")
        
        # Save to JSON
        output_file = '../market_data/kraken_ccxt_data.json'
        
        # Prepare data for JSON
        data_dict = df[['timestamp', 'symbol', 'exchange', 'open', 'high', 'low', 'close', 'volume']].to_dict('records')
        
        with open(output_file, 'w') as f:
            json.dump({
                'exchange': 'kraken',
                'symbol': 'BTC/USD',
                'interval': '1m',
                'bar_count': len(data_dict),
                'first_timestamp': int(df['timestamp'].min()),
                'last_timestamp': int(df['timestamp'].max()),
                'first_datetime': str(df['datetime'].min()),
                'last_datetime': str(df['datetime'].max()),
                'data': data_dict
            }, f, indent=2)
        
        print(f"\nData saved to: {output_file}")
        
        # Check if we have enough to match Coinbase
        coinbase_start = datetime(2025, 8, 3, 17, 32)
        our_start = df['datetime'].min()
        
        if pd.Timestamp(our_start) <= pd.Timestamp(coinbase_start):
            print(f"\n✅ Success! We have data from before Coinbase start ({coinbase_start})")
        else:
            print(f"\n⚠️  Our data starts at {our_start}, after Coinbase ({coinbase_start})")
            print(f"   We're missing {(pd.Timestamp(coinbase_start) - pd.Timestamp(our_start)).days} days")

if __name__ == "__main__":
    main()
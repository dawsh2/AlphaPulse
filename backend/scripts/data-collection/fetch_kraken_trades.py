#!/usr/bin/env python3
"""
Fetch Kraken historical trades data and convert to OHLC
Uses the Trades endpoint which has no limit on historical data
"""

import requests
import time
import pandas as pd
from datetime import datetime
import json
import numpy as np

def fetch_trades_batch(pair='XBTUSD', since=0):
    """
    Fetch a batch of trades from Kraken
    since=0 means from the beginning of the market
    """
    try:
        url = "https://api.kraken.com/0/public/Trades"
        params = {
            'pair': pair,
            'since': since
        }
        
        response = requests.get(url, params=params, timeout=10)
        data = response.json()
        
        if data.get('error'):
            print(f"  API error: {data['error']}")
            return [], None
        
        result = data.get('result', {})
        
        # Get the last timestamp for pagination
        last = result.get('last')
        
        # Find the trades data (key varies)
        trades_key = None
        for key in result.keys():
            if key != 'last' and isinstance(result[key], list):
                trades_key = key
                break
        
        if not trades_key:
            return [], None
        
        trades = result[trades_key]
        return trades, last
        
    except Exception as e:
        print(f"  Error: {e}")
        return [], None

def trades_to_ohlc(trades_df, interval='1T'):
    """
    Convert trades to OHLC bars
    interval: pandas frequency string (1T = 1 minute, 5T = 5 minutes, etc.)
    """
    if trades_df.empty:
        return pd.DataFrame()
    
    # Set timestamp as index
    trades_df.set_index('timestamp', inplace=True)
    
    # Resample to create OHLC
    ohlc = trades_df['price'].resample(interval).ohlc()
    ohlc['volume'] = trades_df['volume'].resample(interval).sum()
    
    # Remove empty bars
    ohlc = ohlc.dropna()
    
    # Reset index
    ohlc.reset_index(inplace=True)
    
    return ohlc

def main():
    print("=" * 60)
    print("KRAKEN TRADES FETCHER - Full Historical Data")
    print("=" * 60)
    
    # Target: Get data from Aug 3, 2025 (timestamp 1754347920)
    # But let's start from 7 days before current time to match Coinbase
    current_time = int(time.time())
    start_time = current_time - (7 * 24 * 3600)  # 7 days ago
    
    # For Trades API, we use nanoseconds
    since_ns = start_time * 1000000000
    
    print(f"Fetching trades from {datetime.fromtimestamp(start_time)}")
    print("This will fetch ALL trades and convert to 1-minute OHLC")
    print("-" * 40)
    
    all_trades = []
    batch_num = 0
    last_timestamp = since_ns
    
    while batch_num < 100:  # Safety limit
        batch_num += 1
        print(f"\nBatch {batch_num}:")
        print(f"  Fetching from {last_timestamp}...")
        
        trades, new_last = fetch_trades_batch('XBTUSD', last_timestamp)
        
        if not trades:
            print("  No more trades")
            break
        
        print(f"  Got {len(trades)} trades")
        
        # Convert trades to DataFrame format
        for trade in trades:
            all_trades.append({
                'price': float(trade[0]),
                'volume': float(trade[1]),
                'timestamp': pd.Timestamp(float(trade[2]), unit='s'),
                'side': trade[3],  # b=buy, s=sell
                'type': trade[4],  # m=market, l=limit
                'misc': trade[5] if len(trade) > 5 else ''
            })
        
        # Check if we've reached current time
        if new_last:
            last_trade_time = float(new_last) / 1000000000
            print(f"  Last trade time: {datetime.fromtimestamp(last_trade_time)}")
            
            if last_trade_time >= current_time - 60:  # Within last minute
                print("  Reached current time")
                break
            
            last_timestamp = new_last
        else:
            break
        
        # Rate limiting - be respectful to free API
        time.sleep(1)
        
        # Save progress every 10 batches
        if batch_num % 10 == 0 and all_trades:
            print(f"\nSaving progress ({len(all_trades)} trades so far)...")
            temp_df = pd.DataFrame(all_trades)
            temp_df.to_json('../market_data/kraken_trades_temp.json', orient='records')
    
    print("\n" + "-" * 40)
    print(f"Fetch complete! Total trades: {len(all_trades)}")
    
    if all_trades:
        print("\nConverting trades to OHLC...")
        trades_df = pd.DataFrame(all_trades)
        
        # Convert to 1-minute OHLC
        ohlc_df = trades_to_ohlc(trades_df.copy(), interval='1T')
        
        print(f"Created {len(ohlc_df)} OHLC bars")
        
        # Add exchange and symbol columns
        ohlc_df['exchange'] = 'kraken'
        ohlc_df['symbol'] = 'BTC/USD'
        
        # Convert timestamp to unix timestamp
        ohlc_df['timestamp'] = ohlc_df['timestamp'].astype(np.int64) // 10**9
        
        # Save to JSON
        output_file = '../market_data/kraken_ohlc_from_trades.json'
        output_data = {
            'exchange': 'kraken',
            'symbol': 'BTC/USD',
            'interval': '1m',
            'bar_count': len(ohlc_df),
            'first_timestamp': int(ohlc_df['timestamp'].min()),
            'last_timestamp': int(ohlc_df['timestamp'].max()),
            'data': ohlc_df.to_dict('records')
        }
        
        with open(output_file, 'w') as f:
            json.dump(output_data, f, indent=2)
        
        print(f"\nOHLC data saved to: {output_file}")
        print(f"First bar: {datetime.fromtimestamp(ohlc_df['timestamp'].min())}")
        print(f"Last bar: {datetime.fromtimestamp(ohlc_df['timestamp'].max())}")
        print(f"Total bars: {len(ohlc_df)}")
        
        # Also save trades for reference
        trades_file = '../market_data/kraken_trades_full.json'
        with open(trades_file, 'w') as f:
            json.dump({
                'trades_count': len(all_trades),
                'first_trade': all_trades[0] if all_trades else None,
                'last_trade': all_trades[-1] if all_trades else None,
                'data': all_trades[:10000]  # Save first 10k trades as sample
            }, f, indent=2, default=str)
        
        print(f"Trades sample saved to: {trades_file}")

if __name__ == "__main__":
    main()
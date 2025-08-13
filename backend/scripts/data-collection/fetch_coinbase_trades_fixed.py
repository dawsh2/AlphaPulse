#!/usr/bin/env python3
"""
Fetch Coinbase trade-by-trade tick data - Fixed version
Properly handles pagination backwards through time
"""

import requests
import time
import json
import pandas as pd
from datetime import datetime, timedelta

def fetch_coinbase_trades_batch(product_id='BTC-USD', before=None, limit=100):
    """
    Fetch trades from Coinbase
    Returns trades and the next 'before' cursor for pagination
    """
    url = f"https://api.exchange.coinbase.com/products/{product_id}/trades"
    
    params = {'limit': limit}
    if before:
        params['before'] = before
    
    headers = {
        'User-Agent': 'AlphaPulse/1.0'
    }
    
    try:
        response = requests.get(url, params=params, headers=headers, timeout=10)
        
        if response.status_code == 429:
            print("  Rate limited, waiting 3 seconds...")
            time.sleep(3)
            return [], None
        
        if response.status_code != 200:
            print(f"  Error: Status {response.status_code}")
            return [], None
        
        trades = response.json()
        
        # Get the trade_id of the last (oldest) trade for next pagination
        next_before = None
        if trades and len(trades) > 0:
            # The last trade in the list is the oldest
            next_before = trades[-1]['trade_id']
        
        return trades, next_before
        
    except Exception as e:
        print(f"  Error: {e}")
        return [], None

def main():
    print("=" * 60)
    print("COINBASE TRADES FETCHER - FIXED")
    print("=" * 60)
    
    # We want 7 days of data to match Kraken
    target_days = 7
    current_time = time.time()
    target_start = current_time - (target_days * 24 * 3600)
    
    print(f"Target: {target_days} days of tick data")
    print(f"From: {datetime.fromtimestamp(target_start)}")
    print(f"To: {datetime.fromtimestamp(current_time)}")
    print("-" * 40)
    
    all_trades = []
    before_cursor = None  # Start from most recent
    batch_num = 0
    oldest_time = current_time
    total_fetched = 0
    
    # Coinbase returns trades newest first, paginating backwards
    while oldest_time > target_start and batch_num < 1000:  # Safety limit
        batch_num += 1
        print(f"\nBatch {batch_num}:")
        
        # Fetch batch
        trades, next_cursor = fetch_coinbase_trades_batch('BTC-USD', before=before_cursor, limit=100)
        
        if not trades:
            print("  No trades returned, retrying...")
            time.sleep(2)
            trades, next_cursor = fetch_coinbase_trades_batch('BTC-USD', before=before_cursor, limit=100)
            
            if not trades:
                print("  Still no trades, stopping")
                break
        
        batch_count = len(trades)
        total_fetched += batch_count
        print(f"  Got {batch_count} trades (total: {total_fetched})")
        
        # Process trades
        for trade in trades:
            # Parse timestamp
            trade_time = pd.Timestamp(trade['time']).timestamp()
            
            # Track oldest
            if trade_time < oldest_time:
                oldest_time = trade_time
            
            # Add to collection if within our target range
            if trade_time >= target_start:
                all_trades.append({
                    'timestamp': trade_time,
                    'datetime': trade['time'],
                    'trade_id': str(trade['trade_id']),
                    'price': float(trade['price']),
                    'size': float(trade['size']),
                    'side': trade['side'],
                    'exchange': 'coinbase',
                    'symbol': 'BTC-USD'
                })
        
        print(f"  Oldest in batch: {datetime.fromtimestamp(oldest_time)}")
        print(f"  Collected trades: {len(all_trades)}")
        
        # Check if we've gone far enough back
        if oldest_time <= target_start:
            print(f"  Reached target time!")
            break
        
        # Update cursor for next batch
        if next_cursor:
            before_cursor = next_cursor
        else:
            print("  No next cursor, stopping")
            break
        
        # Rate limiting - be respectful
        time.sleep(0.2)
        
        # Save progress periodically
        if batch_num % 100 == 0 and all_trades:
            print(f"\nSaving progress ({len(all_trades)} trades)...")
            with open('../market_data/coinbase_trades_progress.json', 'w') as f:
                json.dump(all_trades[:1000], f)  # Save sample
    
    print("\n" + "=" * 60)
    print(f"FETCH COMPLETE")
    print("=" * 60)
    
    if all_trades:
        print(f"Total trades collected: {len(all_trades)}")
        
        # Sort by timestamp
        all_trades.sort(key=lambda x: x['timestamp'])
        
        # Get summary
        first_time = all_trades[0]['timestamp']
        last_time = all_trades[-1]['timestamp']
        duration_hours = (last_time - first_time) / 3600
        
        print(f"\nData summary:")
        print(f"  First trade: {datetime.fromtimestamp(first_time)}")
        print(f"  Last trade: {datetime.fromtimestamp(last_time)}")
        print(f"  Duration: {duration_hours:.1f} hours")
        print(f"  Avg trades/min: {len(all_trades) / (duration_hours * 60):.1f}")
        
        # Save to JSON
        output_file = '../market_data/coinbase_trades.json'
        with open(output_file, 'w') as f:
            json.dump({
                'exchange': 'coinbase',
                'symbol': 'BTC-USD',
                'trade_count': len(all_trades),
                'first_timestamp': first_time,
                'last_timestamp': last_time,
                'first_datetime': datetime.fromtimestamp(first_time).isoformat(),
                'last_datetime': datetime.fromtimestamp(last_time).isoformat(),
                'data': all_trades
            }, f, indent=2)
        
        print(f"\nData saved to: {output_file}")
        
        # Create 1-minute OHLC for comparison
        df = pd.DataFrame(all_trades)
        df['dt'] = pd.to_datetime(df['timestamp'], unit='s')
        df.set_index('dt', inplace=True)
        
        ohlc = df['price'].resample('1min').ohlc()
        ohlc['volume'] = df['size'].resample('1min').sum()
        ohlc = ohlc.dropna()
        
        print(f"\nGenerated {len(ohlc)} 1-minute bars from {len(all_trades)} trades")
        
        print("\n✅ Ready for tick-level arbitrage analysis!")
    else:
        print("\nNo trades collected")

if __name__ == "__main__":
    main()
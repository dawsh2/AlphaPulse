#!/usr/bin/env python3
"""
Fetch Coinbase trade-by-trade tick data
Matches the time range we have for Kraken trades
"""

import requests
import time
import json
import pandas as pd
from datetime import datetime, timedelta
import duckdb

def fetch_coinbase_trades(product_id='BTC-USD', before=None, after=None, limit=100):
    """
    Fetch trades from Coinbase
    Returns trades and pagination cursor
    """
    url = f"https://api.exchange.coinbase.com/products/{product_id}/trades"
    
    params = {'limit': limit}
    if before:
        params['before'] = before
    if after:
        params['after'] = after
    
    try:
        response = requests.get(url, params=params, timeout=10)
        
        if response.status_code == 429:
            print("  Rate limited, waiting 2 seconds...")
            time.sleep(2)
            return [], None
        
        if response.status_code != 200:
            print(f"  Error: Status {response.status_code}")
            return [], None
        
        trades = response.json()
        
        # Get pagination cursor from headers
        before_cursor = response.headers.get('cb-before')
        
        return trades, before_cursor
        
    except Exception as e:
        print(f"  Error: {e}")
        return [], None

def main():
    print("=" * 60)
    print("COINBASE TRADES FETCHER")
    print("=" * 60)
    
    # Target: Match our Kraken data range (Aug 4-11, 2025)
    # We'll fetch backwards from current time
    
    # Check what range we have for Kraken
    try:
        conn = duckdb.connect('../market_data/market_data.duckdb', read_only=True)
        kraken_info = conn.execute("""
            SELECT 
                COUNT(*) as bars,
                to_timestamp(MIN(timestamp)) as first_bar,
                to_timestamp(MAX(timestamp)) as last_bar
            FROM ohlcv 
            WHERE symbol = 'BTC/USD' AND exchange = 'kraken'
        """).fetchone()
        conn.close()
        
        print(f"Kraken data range: {kraken_info[1]} to {kraken_info[2]}")
        print(f"Kraken bars: {kraken_info[0]}")
        
        # Convert to timestamp
        target_start = pd.Timestamp(kraken_info[1]).timestamp()
        target_end = pd.Timestamp(kraken_info[2]).timestamp()
        
    except Exception as e:
        print(f"Could not check Kraken range: {e}")
        # Default to 7 days ago
        target_end = time.time()
        target_start = target_end - (7 * 24 * 3600)
    
    print(f"\nFetching Coinbase trades from {datetime.fromtimestamp(target_start)} to {datetime.fromtimestamp(target_end)}")
    print("-" * 40)
    
    all_trades = []
    before_cursor = None
    batch_num = 0
    oldest_time = target_end
    
    # Coinbase returns trades in reverse chronological order (newest first)
    # We'll paginate backwards until we reach our target start time
    
    while oldest_time > target_start and batch_num < 500:  # Safety limit
        batch_num += 1
        print(f"\nBatch {batch_num}:")
        
        # Fetch batch
        trades, new_cursor = fetch_coinbase_trades('BTC-USD', before=before_cursor, limit=100)
        
        if not trades:
            if batch_num > 1:
                # Try once more with longer wait
                print("  No trades, waiting 5 seconds and retrying...")
                time.sleep(5)
                trades, new_cursor = fetch_coinbase_trades('BTC-USD', before=before_cursor, limit=100)
            
            if not trades:
                print("  No more trades available")
                break
        
        print(f"  Got {len(trades)} trades")
        
        # Convert trades to our format
        for trade in trades:
            trade_time = pd.Timestamp(trade['time']).timestamp()
            
            # Only include trades within our target range
            if trade_time >= target_start and trade_time <= target_end:
                all_trades.append({
                    'timestamp': trade_time,
                    'trade_id': trade['trade_id'],
                    'price': float(trade['price']),
                    'size': float(trade['size']),
                    'side': trade['side'],  # 'buy' or 'sell'
                    'exchange': 'coinbase',
                    'symbol': 'BTC/USD'
                })
            
            # Track oldest time
            if trade_time < oldest_time:
                oldest_time = trade_time
        
        print(f"  Oldest trade: {datetime.fromtimestamp(oldest_time)}")
        print(f"  Total trades collected: {len(all_trades)}")
        
        # Check if we've gone far enough back
        if oldest_time <= target_start:
            print(f"  Reached target start time")
            break
        
        # Update cursor for next batch
        before_cursor = new_cursor
        
        if not before_cursor:
            print("  No pagination cursor, stopping")
            break
        
        # Rate limiting - Coinbase allows ~10 requests/second
        time.sleep(0.15)  # ~6-7 requests per second to be safe
        
        # Save progress every 50 batches
        if batch_num % 50 == 0 and all_trades:
            print(f"\nSaving progress ({len(all_trades)} trades)...")
            temp_file = '../market_data/coinbase_trades_temp.json'
            with open(temp_file, 'w') as f:
                json.dump(all_trades, f)
    
    print("\n" + "-" * 40)
    print(f"Fetch complete! Total trades: {len(all_trades)}")
    
    if all_trades:
        # Sort by timestamp (oldest first)
        all_trades.sort(key=lambda x: x['timestamp'])
        
        # Convert to DataFrame for analysis
        df = pd.DataFrame(all_trades)
        df['datetime'] = pd.to_datetime(df['timestamp'], unit='s')
        
        print(f"\nTrade data summary:")
        print(f"  First trade: {df['datetime'].min()}")
        print(f"  Last trade: {df['datetime'].max()}")
        print(f"  Total trades: {len(df)}")
        print(f"  Average trades per minute: {len(df) / ((df['timestamp'].max() - df['timestamp'].min()) / 60):.1f}")
        
        # Save to JSON
        output_file = '../market_data/coinbase_trades.json'
        with open(output_file, 'w') as f:
            json.dump({
                'exchange': 'coinbase',
                'symbol': 'BTC/USD',
                'trade_count': len(all_trades),
                'first_timestamp': float(df['timestamp'].min()),
                'last_timestamp': float(df['timestamp'].max()),
                'first_datetime': str(df['datetime'].min()),
                'last_datetime': str(df['datetime'].max()),
                'data': all_trades
            }, f, indent=2)
        
        print(f"\nData saved to: {output_file}")
        
        # Also create OHLC from trades for comparison
        print("\nConverting to 1-minute OHLC for comparison...")
        df.set_index('datetime', inplace=True)
        ohlc = df['price'].resample('1min').ohlc()
        ohlc['volume'] = df['size'].resample('1min').sum()
        ohlc = ohlc.dropna()
        
        print(f"Created {len(ohlc)} OHLC bars from trades")
        
        # Compare with existing Coinbase OHLC data
        print("\nComparison with existing Coinbase OHLC:")
        print(f"  Trades: {len(all_trades)}")
        print(f"  Generated OHLC bars: {len(ohlc)}")
        print(f"  Existing OHLC bars in DB: ~11,445")
        
        print("\nNext steps:")
        print("1. Import trades to a new 'trades' table in DuckDB")
        print("2. Use trades for more accurate arbitrage analysis")
        print("3. Calculate actual executable arbitrage opportunities")

if __name__ == "__main__":
    main()
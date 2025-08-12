#!/usr/bin/env python3
"""
Enhanced script to fetch Kraken data in batches to match Coinbase data volume
Handles the ~720 bar API limitation by making multiple paginated requests
"""

import duckdb
import requests
import time
import pandas as pd
from datetime import datetime, timedelta

def check_current_balance():
    """Check how much data we currently have from each exchange"""
    try:
        conn = duckdb.connect('../market_data/market_data.duckdb', read_only=True)
        
        query = """
        SELECT 
            exchange,
            COUNT(*) as bar_count,
            to_timestamp(MIN(timestamp)) as first_bar,
            to_timestamp(MAX(timestamp)) as last_bar
        FROM ohlcv
        WHERE symbol = 'BTC/USD'
        GROUP BY exchange
        ORDER BY exchange
        """
        
        result = conn.execute(query).fetchall()
        conn.close()
        
        balance = {}
        for row in result:
            balance[row[0]] = {
                'count': row[1],
                'first': row[2],
                'last': row[3]
            }
        
        return balance
    except Exception as e:
        print(f"Error checking balance: {e}")
        return {}

def fetch_kraken_batch(start_time, max_bars=720):
    """
    Fetch a batch of Kraken data starting from start_time
    Returns the data and the timestamp of the last bar fetched
    """
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

def save_to_duckdb(data):
    """Save fetched data to DuckDB"""
    if not data:
        print("No data to save")
        return False
    
    try:
        df = pd.DataFrame(data)
        
        # Connect with write access
        conn = duckdb.connect('../market_data/market_data.duckdb')
        
        # Create table if it doesn't exist
        conn.execute("""
            CREATE TABLE IF NOT EXISTS ohlcv (
                timestamp BIGINT,
                symbol VARCHAR,
                exchange VARCHAR,
                open DOUBLE,
                high DOUBLE,
                low DOUBLE,
                close DOUBLE,
                volume DOUBLE
            )
        """)
        
        # Create a temporary table with the new data
        conn.execute("CREATE TEMPORARY TABLE temp_ohlcv AS SELECT * FROM df")
        
        # Insert only new data (avoid duplicates)
        conn.execute("""
            INSERT INTO ohlcv 
            SELECT * FROM temp_ohlcv t
            WHERE NOT EXISTS (
                SELECT 1 FROM ohlcv o 
                WHERE o.timestamp = t.timestamp 
                AND o.exchange = t.exchange 
                AND o.symbol = t.symbol
            )
        """)
        
        # Get the count of inserted rows
        inserted = conn.execute("SELECT COUNT(*) FROM temp_ohlcv").fetchone()[0]
        
        conn.close()
        print(f"Saved {inserted} new records to database")
        return True
        
    except Exception as e:
        print(f"Error saving to database: {e}")
        return False

def fetch_kraken_to_match_coinbase():
    """Main function to fetch enough Kraken data to match Coinbase"""
    print("=" * 60)
    print("KRAKEN DATA FETCHER")
    print("=" * 60)
    
    # Check current balance
    balance = check_current_balance()
    
    if not balance:
        print("Could not check data balance")
        return
    
    print("\nCurrent data balance:")
    for exchange, info in balance.items():
        print(f"  {exchange}: {info['count']:,} bars ({info['first']} to {info['last']})")
    
    if 'coinbase' not in balance:
        print("\nNo Coinbase data found to match")
        return
    
    coinbase_count = balance['coinbase']['count']
    kraken_count = balance.get('kraken', {}).get('count', 0)
    
    if kraken_count >= coinbase_count:
        print(f"\nKraken already has {kraken_count:,} bars, matching or exceeding Coinbase's {coinbase_count:,}")
        return
    
    bars_needed = coinbase_count - kraken_count
    print(f"\nNeed to fetch {bars_needed:,} more bars for Kraken")
    
    # Calculate how many batches we need (Kraken returns ~720 bars per request)
    batches_needed = (bars_needed // 700) + 1
    print(f"Will fetch in approximately {batches_needed} batches")
    
    # Start from 7 days ago to get recent data
    current_time = int(time.time())
    start_time = current_time - (7 * 24 * 3600)  # 7 days ago
    
    all_data = []
    batch_count = 0
    total_fetched = 0
    
    print(f"\nStarting fetch from {datetime.fromtimestamp(start_time)}")
    print("-" * 40)
    
    while total_fetched < bars_needed and batch_count < batches_needed + 5:  # Extra batches for safety
        batch_count += 1
        print(f"\nBatch {batch_count}:")
        
        # Fetch a batch
        batch_data, last_timestamp = fetch_kraken_batch(start_time)
        
        if not batch_data:
            print("  No more data available")
            break
        
        all_data.extend(batch_data)
        total_fetched += len(batch_data)
        
        print(f"  Total fetched so far: {total_fetched:,} bars")
        
        # Move to the next batch (1 minute after the last bar)
        if last_timestamp:
            start_time = last_timestamp + 60
            
            # Stop if we've reached current time
            if start_time >= current_time:
                print("  Reached current time")
                break
        else:
            break
        
        # Rate limiting - Kraken allows 1 request per second for public endpoints
        time.sleep(1.5)
    
    print("-" * 40)
    print(f"\nFetch complete! Total bars fetched: {len(all_data):,}")
    
    if all_data:
        print("\nSaving to database...")
        if save_to_duckdb(all_data):
            print("Data saved successfully!")
            
            # Check new balance
            print("\n" + "=" * 60)
            print("UPDATED DATA BALANCE")
            print("=" * 60)
            new_balance = check_current_balance()
            for exchange, info in new_balance.items():
                print(f"  {exchange}: {info['count']:,} bars")
        else:
            print("Failed to save data")
    else:
        print("\nNo data was fetched")

if __name__ == "__main__":
    fetch_kraken_to_match_coinbase()
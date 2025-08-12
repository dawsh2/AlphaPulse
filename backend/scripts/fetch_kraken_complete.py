#!/usr/bin/env python3
"""
Fetch complete Kraken data to match Coinbase starting point
Handles API rate limits and fetches in batches
"""

import duckdb
import requests
import time
import pandas as pd
from datetime import datetime, timedelta
import json

def check_current_data():
    """Check current data in DuckDB"""
    try:
        conn = duckdb.connect('../market_data/market_data.duckdb', read_only=True)
        
        query = """
        SELECT 
            exchange,
            COUNT(*) as bar_count,
            to_timestamp(MIN(timestamp)) as first_bar,
            to_timestamp(MAX(timestamp)) as last_bar,
            MIN(timestamp) as min_ts,
            MAX(timestamp) as max_ts
        FROM ohlcv
        WHERE symbol = 'BTC/USD'
        GROUP BY exchange
        ORDER BY exchange
        """
        
        result = conn.execute(query).fetchall()
        conn.close()
        
        data = {}
        for row in result:
            data[row[0]] = {
                'count': row[1],
                'first': row[2],
                'last': row[3],
                'min_ts': row[4],
                'max_ts': row[5]
            }
        
        return data
    except Exception as e:
        print(f"Error checking data: {e}")
        return {}

def fetch_kraken_batch(start_time, end_time=None):
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
            candle_time = int(candle[0])
            # Only include bars within our desired range
            if end_time is None or candle_time <= end_time:
                bars.append({
                    'timestamp': candle_time,
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

def save_to_duckdb(data):
    """Save data to DuckDB"""
    if not data:
        return False
    
    try:
        df = pd.DataFrame(data)
        conn = duckdb.connect('../market_data/market_data.duckdb')
        
        # Create table if needed
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
        
        # Create temporary table
        conn.execute("CREATE TEMPORARY TABLE temp_ohlcv AS SELECT * FROM df")
        
        # Insert only new data
        result = conn.execute("""
            INSERT INTO ohlcv 
            SELECT * FROM temp_ohlcv t
            WHERE NOT EXISTS (
                SELECT 1 FROM ohlcv o 
                WHERE o.timestamp = t.timestamp 
                AND o.exchange = t.exchange 
                AND o.symbol = t.symbol
            )
            RETURNING *
        """)
        
        inserted_count = len(result.fetchall())
        conn.close()
        
        print(f"  Inserted {inserted_count} new records")
        return True
        
    except Exception as e:
        print(f"  Error saving: {e}")
        return False

def main():
    print("=" * 60)
    print("KRAKEN DATA FETCHER - Complete Historical Data")
    print("=" * 60)
    
    # Check current state
    current_data = check_current_data()
    
    if not current_data:
        print("Could not check current data")
        return
    
    print("\nCurrent data:")
    for exchange, info in current_data.items():
        print(f"  {exchange}: {info['count']:,} bars")
        print(f"    From: {info['first']}")
        print(f"    To: {info['last']}")
    
    if 'coinbase' not in current_data:
        print("\nNo Coinbase data to match")
        return
    
    # Get Coinbase start time
    coinbase_start = current_data['coinbase']['min_ts']
    coinbase_end = current_data['coinbase']['max_ts']
    
    print(f"\nNeed to fetch Kraken data from {datetime.fromtimestamp(coinbase_start)} to now")
    
    # Calculate time ranges to fetch (work backwards from now)
    current_time = int(time.time())
    
    # We'll fetch in ~12 hour chunks (720 bars) working backwards
    all_fetched_data = []
    fetch_start = coinbase_start
    
    print("\nFetching data in batches...")
    print("-" * 40)
    
    batch_num = 0
    total_bars = 0
    
    while fetch_start < current_time:
        batch_num += 1
        print(f"\nBatch {batch_num}: Starting from {datetime.fromtimestamp(fetch_start)}")
        
        bars, last_ts = fetch_kraken_batch(fetch_start)
        
        if not bars:
            print("  No data returned, waiting 2 seconds...")
            time.sleep(2)
            # Try once more
            bars, last_ts = fetch_kraken_batch(fetch_start)
            
            if not bars:
                print("  Still no data, moving forward 12 hours")
                fetch_start += (12 * 3600)
                continue
        
        print(f"  Got {len(bars)} bars")
        all_fetched_data.extend(bars)
        total_bars += len(bars)
        
        # Move to next batch
        if last_ts:
            fetch_start = last_ts + 60  # Start from 1 minute after last bar
        else:
            fetch_start += (12 * 3600)  # Move forward 12 hours
        
        # Save periodically (every 5000 bars)
        if len(all_fetched_data) >= 5000:
            print(f"\nSaving batch of {len(all_fetched_data)} bars...")
            if save_to_duckdb(all_fetched_data):
                all_fetched_data = []  # Clear after successful save
            else:
                print("Failed to save, will retry later")
        
        # Rate limiting
        time.sleep(1.5)
        
        # Stop after reasonable number of batches
        if batch_num >= 30:  # About 15 days of data
            print("\nReached maximum batches")
            break
    
    # Save any remaining data
    if all_fetched_data:
        print(f"\nSaving final batch of {len(all_fetched_data)} bars...")
        save_to_duckdb(all_fetched_data)
    
    print("\n" + "=" * 60)
    print("FETCH COMPLETE")
    print("=" * 60)
    
    # Check final state
    final_data = check_current_data()
    if final_data:
        print("\nFinal data state:")
        for exchange, info in final_data.items():
            print(f"  {exchange}: {info['count']:,} bars")
            print(f"    From: {info['first']}")
            print(f"    To: {info['last']}")
        
        if 'kraken' in final_data and 'coinbase' in final_data:
            diff = final_data['coinbase']['count'] - final_data['kraken']['count']
            if diff > 0:
                print(f"\nStill need {diff:,} more Kraken bars to match Coinbase")
            else:
                print(f"\nKraken now has equal or more data than Coinbase!")

if __name__ == "__main__":
    main()
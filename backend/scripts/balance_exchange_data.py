#!/usr/bin/env python3
"""
Script to check and balance exchange data in DuckDB
Ensures we have equal amounts of data from Coinbase and Kraken
"""

import duckdb
import pandas as pd
import requests
import time
from datetime import datetime, timedelta

def check_data_balance():
    """Check how much data we have from each exchange"""
    conn = duckdb.connect('../market_data/market_data.duckdb', read_only=True)
    
    query = """
    SELECT 
        exchange,
        symbol,
        COUNT(*) as bar_count,
        MIN(timestamp) as first_bar,
        MAX(timestamp) as last_bar,
        MIN(timestamp) as first_timestamp_raw,
        MAX(timestamp) as last_timestamp_raw
    FROM ohlcv
    WHERE symbol = 'BTC/USD'
    GROUP BY exchange, symbol
    ORDER BY exchange
    """
    
    df = conn.execute(query).df()
    
    # Convert timestamps properly
    df['first_bar'] = pd.to_datetime(df['first_bar'], unit='s')
    df['last_bar'] = pd.to_datetime(df['last_bar'], unit='s')
    
    print("=" * 60)
    print("CURRENT DATA BALANCE")
    print("=" * 60)
    print(df[['exchange', 'symbol', 'bar_count', 'first_bar', 'last_bar']])
    print()
    
    conn.close()
    return df

def fetch_kraken_data(start_timestamp, end_timestamp):
    """Fetch historical data from Kraken"""
    print(f"Fetching Kraken data from {datetime.fromtimestamp(start_timestamp)} to {datetime.fromtimestamp(end_timestamp)}")
    
    all_data = []
    current_since = start_timestamp
    
    while current_since < end_timestamp:
        try:
            # Use the proxy endpoint
            url = f"http://localhost:5001/api/proxy/kraken/0/public/OHLC"
            params = {
                'pair': 'XBTUSD',
                'interval': '1',  # 1 minute
                'since': int(current_since)
            }
            
            response = requests.get(url, params=params)
            data = response.json()
            
            if 'error' in data and data['error']:
                print(f"Kraken API error: {data['error']}")
                break
                
            # Find the result key
            result = data.get('result', {})
            pair_key = next((k for k in result.keys() if k != 'last'), None)
            
            if not pair_key:
                print("No data found")
                break
                
            ohlc_data = result[pair_key]
            
            if not ohlc_data:
                print("No more data available")
                break
                
            # Convert to our format
            for candle in ohlc_data:
                all_data.append({
                    'timestamp': int(candle[0]),
                    'open': float(candle[1]),
                    'high': float(candle[2]),
                    'low': float(candle[3]),
                    'close': float(candle[4]),
                    'volume': float(candle[6]),
                    'symbol': 'BTC/USD',
                    'exchange': 'kraken'
                })
            
            # Update the 'since' parameter for the next request
            if ohlc_data:
                current_since = int(ohlc_data[-1][0]) + 60  # Move to next minute after last candle
                print(f"Fetched {len(ohlc_data)} candles, total: {len(all_data)}")
            
            # Rate limiting
            time.sleep(0.5)
            
        except Exception as e:
            print(f"Error fetching data: {e}")
            break
    
    return all_data

def save_to_duckdb(data):
    """Save the fetched data to DuckDB"""
    if not data:
        print("No data to save")
        return
        
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
    
    # Insert data (DuckDB will handle duplicates)
    conn.execute("""
        INSERT INTO ohlcv 
        SELECT * FROM df
        ON CONFLICT DO NOTHING
    """)
    
    print(f"Saved {len(df)} records to database")
    conn.close()

def main():
    """Main function to balance exchange data"""
    # Check current balance
    df_balance = check_data_balance()
    
    if len(df_balance) < 2:
        print("Need data from both exchanges first")
        return
    
    coinbase_count = df_balance[df_balance['exchange'] == 'coinbase']['bar_count'].values[0]
    kraken_count = df_balance[df_balance['exchange'] == 'kraken']['bar_count'].values[0] if len(df_balance[df_balance['exchange'] == 'kraken']) > 0 else 0
    
    print(f"Coinbase bars: {coinbase_count}")
    print(f"Kraken bars: {kraken_count}")
    print(f"Difference: {coinbase_count - kraken_count}")
    
    if coinbase_count > kraken_count:
        print(f"\nNeed to fetch {coinbase_count - kraken_count} more bars for Kraken")
        
        # Get the time range we need to fill
        coinbase_first = df_balance[df_balance['exchange'] == 'coinbase']['first_timestamp_raw'].values[0]
        
        if kraken_count > 0:
            kraken_first = df_balance[df_balance['exchange'] == 'kraken']['first_timestamp_raw'].values[0]
            # Fetch data before the earliest Kraken data
            if coinbase_first < kraken_first:
                print(f"Fetching historical data before {datetime.fromtimestamp(kraken_first)}")
                new_data = fetch_kraken_data(coinbase_first, kraken_first)
                if new_data:
                    save_to_duckdb(new_data)
        else:
            print("No Kraken data found, fetching from Coinbase start time")
            # Fetch from the same starting point as Coinbase
            end_time = int(time.time())
            new_data = fetch_kraken_data(coinbase_first, end_time)
            if new_data:
                save_to_duckdb(new_data)
    
    # Check balance again
    print("\n" + "=" * 60)
    print("UPDATED DATA BALANCE")
    print("=" * 60)
    check_data_balance()

if __name__ == "__main__":
    main()
#!/usr/bin/env python3
"""
Import JSON data to DuckDB
"""

import duckdb
import json
import pandas as pd
import sys

def import_json_to_duckdb(json_file):
    """Import JSON data to DuckDB"""
    print(f"Loading data from {json_file}...")
    
    with open(json_file, 'r') as f:
        data = json.load(f)
    
    bars = data['data']
    print(f"Loaded {len(bars)} bars")
    
    if not bars:
        print("No data to import")
        return
    
    # Convert to DataFrame
    df = pd.DataFrame(bars)
    
    # Add datetime column
    df['datetime'] = pd.to_datetime(df['timestamp'], unit='s')
    
    # Reorder columns to match table structure
    df = df[['symbol', 'exchange', 'timestamp', 'datetime', 'open', 'high', 'low', 'close', 'volume']]
    
    print("\nConnecting to DuckDB...")
    try:
        conn = duckdb.connect('../market_data/market_data.duckdb')
        
        # Table already exists with correct structure
        
        # Create temporary table
        conn.execute("CREATE TEMPORARY TABLE temp_ohlcv AS SELECT * FROM df")
        
        # Check for existing data
        existing = conn.execute("""
            SELECT COUNT(*) FROM ohlcv 
            WHERE exchange = 'kraken' AND symbol = 'BTC/USD'
        """).fetchone()[0]
        
        print(f"Existing Kraken records: {existing}")
        
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
        """)
        
        # Get count of inserted rows
        inserted = conn.execute("SELECT COUNT(*) FROM temp_ohlcv").fetchone()[0]
        
        # Check final counts
        final_kraken = conn.execute("""
            SELECT COUNT(*) FROM ohlcv 
            WHERE exchange = 'kraken' AND symbol = 'BTC/USD'
        """).fetchone()[0]
        
        final_coinbase = conn.execute("""
            SELECT COUNT(*) FROM ohlcv 
            WHERE exchange = 'coinbase' AND symbol = 'BTC/USD'
        """).fetchone()[0]
        
        conn.close()
        
        print(f"\nImport complete!")
        print(f"Attempted to insert: {inserted} records")
        print(f"Final Kraken count: {final_kraken}")
        print(f"Final Coinbase count: {final_coinbase}")
        
        if final_kraken < final_coinbase:
            print(f"\nStill need {final_coinbase - final_kraken} more Kraken bars")
        else:
            print(f"\nKraken data now matches or exceeds Coinbase!")
        
    except Exception as e:
        print(f"Error: {e}")
        print("\nIf database is locked, try:")
        print("1. Close any notebooks using DuckDB")
        print("2. Run: curl -X POST http://localhost:5002/api/notebook/cleanup")
        print("3. Check for processes: lsof ../market_data/market_data.duckdb")

if __name__ == "__main__":
    if len(sys.argv) > 1:
        json_file = sys.argv[1]
    else:
        json_file = '../market_data/kraken_complete.json'
    
    import_json_to_duckdb(json_file)
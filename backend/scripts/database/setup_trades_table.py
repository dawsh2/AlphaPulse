#!/usr/bin/env python3
"""
Setup trades table in DuckDB for tick-by-tick data
This is better for arbitrage analysis than OHLC bars
"""

import duckdb
import json
import pandas as pd
from datetime import datetime

def setup_trades_table():
    """Create trades table and import existing trade data"""
    
    print("=" * 60)
    print("SETTING UP TRADES TABLE")
    print("=" * 60)
    
    # Connect to DuckDB
    conn = duckdb.connect('../market_data/market_data.duckdb')
    
    try:
        # Create trades table
        print("\nCreating trades table...")
        conn.execute("""
            CREATE TABLE IF NOT EXISTS trades (
                timestamp DOUBLE,
                datetime TIMESTAMP,
                symbol VARCHAR,
                exchange VARCHAR,
                price DOUBLE,
                size DOUBLE,
                side VARCHAR,
                trade_id VARCHAR,
                PRIMARY KEY (exchange, trade_id)
            )
        """)
        
        print("Trades table created/verified")
        
        # Check if we have Kraken trades JSON
        try:
            print("\nLoading Kraken trades...")
            with open('../market_data/kraken_trades_full.json', 'r') as f:
                kraken_data = json.load(f)
            
            # The data field contains sample trades
            kraken_trades = kraken_data['data']
            print(f"  Found {len(kraken_trades)} Kraken trade samples")
            
            if kraken_trades:
                # Convert to DataFrame
                df = pd.DataFrame(kraken_trades)
                
                # Add trade_id (Kraken doesn't provide one, so we'll create it)
                df['trade_id'] = ['kraken_' + str(i) for i in range(len(df))]
                
                # Ensure datetime column
                if 'timestamp' in df.columns and df['timestamp'].dtype == 'O':
                    # It's already a datetime string
                    df['datetime'] = pd.to_datetime(df['timestamp'])
                    df['timestamp'] = df['datetime'].astype('int64') / 1e9
                else:
                    df['datetime'] = pd.to_datetime(df['timestamp'], unit='s')
                
                # Select columns
                df = df[['timestamp', 'datetime', 'symbol', 'exchange', 'price', 'volume', 'side', 'trade_id']]
                df.rename(columns={'volume': 'size'}, inplace=True)
                
                # Insert into table
                conn.execute("INSERT OR IGNORE INTO trades SELECT * FROM df")
                
                kraken_count = conn.execute("SELECT COUNT(*) FROM trades WHERE exchange = 'kraken'").fetchone()[0]
                print(f"  Kraken trades in DB: {kraken_count}")
        
        except FileNotFoundError:
            print("  No Kraken trades file found")
        except Exception as e:
            print(f"  Error loading Kraken trades: {e}")
        
        # Check if we have Coinbase trades JSON
        try:
            print("\nLoading Coinbase trades...")
            with open('../market_data/coinbase_trades.json', 'r') as f:
                coinbase_data = json.load(f)
            
            coinbase_trades = coinbase_data['data']
            print(f"  Found {len(coinbase_trades)} Coinbase trades")
            
            if coinbase_trades:
                # Convert to DataFrame
                df = pd.DataFrame(coinbase_trades)
                
                # Ensure datetime column
                df['datetime'] = pd.to_datetime(df['timestamp'], unit='s')
                
                # Select columns
                df = df[['timestamp', 'datetime', 'symbol', 'exchange', 'price', 'size', 'side', 'trade_id']]
                
                # Insert into table
                conn.execute("INSERT OR IGNORE INTO trades SELECT * FROM df")
                
                coinbase_count = conn.execute("SELECT COUNT(*) FROM trades WHERE exchange = 'coinbase'").fetchone()[0]
                print(f"  Coinbase trades in DB: {coinbase_count}")
        
        except FileNotFoundError:
            print("  No Coinbase trades file found - run fetch_coinbase_trades.py first")
        except Exception as e:
            print(f"  Error loading Coinbase trades: {e}")
        
        # Show summary
        print("\n" + "=" * 60)
        print("TRADES TABLE SUMMARY")
        print("=" * 60)
        
        result = conn.execute("""
            SELECT 
                exchange,
                COUNT(*) as trade_count,
                to_timestamp(MIN(timestamp)) as first_trade,
                to_timestamp(MAX(timestamp)) as last_trade,
                ROUND(AVG(price), 2) as avg_price,
                ROUND(SUM(size), 2) as total_volume
            FROM trades
            GROUP BY exchange
            ORDER BY exchange
        """).fetchall()
        
        for row in result:
            print(f"\n{row[0].upper()}:")
            print(f"  Trades: {row[1]:,}")
            print(f"  Period: {row[2]} to {row[3]}")
            print(f"  Avg Price: ${row[4]:,.2f}")
            print(f"  Total Volume: {row[5]:.2f} BTC")
        
        # Create sample arbitrage query
        print("\n" + "=" * 60)
        print("SAMPLE ARBITRAGE QUERY")
        print("=" * 60)
        
        print("\nFinding potential arbitrage opportunities (price differences > $10):")
        
        arb_query = """
            WITH aligned_trades AS (
                SELECT 
                    DATE_TRUNC('minute', datetime) as minute,
                    exchange,
                    AVG(price) as avg_price,
                    COUNT(*) as trade_count
                FROM trades
                WHERE symbol = 'BTC/USD'
                GROUP BY minute, exchange
            ),
            spreads AS (
                SELECT 
                    cb.minute,
                    cb.avg_price as coinbase_price,
                    kr.avg_price as kraken_price,
                    cb.avg_price - kr.avg_price as spread,
                    ABS(cb.avg_price - kr.avg_price) as abs_spread,
                    cb.trade_count as cb_trades,
                    kr.trade_count as kr_trades
                FROM aligned_trades cb
                JOIN aligned_trades kr ON cb.minute = kr.minute
                WHERE cb.exchange = 'coinbase' AND kr.exchange = 'kraken'
            )
            SELECT 
                minute,
                ROUND(coinbase_price, 2) as cb_price,
                ROUND(kraken_price, 2) as kr_price,
                ROUND(spread, 2) as spread,
                CASE 
                    WHEN spread > 0 THEN 'Buy Kraken, Sell Coinbase'
                    ELSE 'Buy Coinbase, Sell Kraken'
                END as action
            FROM spreads
            WHERE abs_spread > 10
            ORDER BY abs_spread DESC
            LIMIT 10
        """
        
        try:
            arb_opportunities = conn.execute(arb_query).fetchall()
            
            if arb_opportunities:
                print("\nTop arbitrage opportunities found:")
                for opp in arb_opportunities:
                    print(f"  {opp[0]}: CB=${opp[1]:,.2f}, KR=${opp[2]:,.2f}, Spread=${opp[3]:,.2f} -> {opp[4]}")
            else:
                print("\nNo significant arbitrage opportunities found (spread > $10)")
        except Exception as e:
            print(f"\nCould not run arbitrage query: {e}")
        
        conn.close()
        
        print("\n" + "=" * 60)
        print("SETUP COMPLETE")
        print("=" * 60)
        print("\nNext steps:")
        print("1. Run fetch_coinbase_trades.py to get Coinbase tick data")
        print("2. Fetch more Kraken trades if needed")
        print("3. Use the trades table for accurate arbitrage analysis")
        print("4. Consider building order book reconstruction for L2 analysis")
        
    except Exception as e:
        print(f"Error setting up trades table: {e}")
        conn.close()

if __name__ == "__main__":
    setup_trades_table()
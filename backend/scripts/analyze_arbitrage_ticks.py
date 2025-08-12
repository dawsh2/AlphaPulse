#!/usr/bin/env python3
"""
Analyze arbitrage opportunities using tick-level trade data
Shows why tick data is better than OHLC for arbitrage
"""

import json
import pandas as pd
import numpy as np
from datetime import datetime

def analyze_arbitrage():
    print("=" * 60)
    print("TICK-LEVEL ARBITRAGE ANALYSIS")
    print("=" * 60)
    
    # Load available trade data
    trades_data = {}
    
    # Load Coinbase trades
    try:
        with open('../market_data/coinbase_trades.json', 'r') as f:
            cb_data = json.load(f)
        trades_data['coinbase'] = cb_data['data']
        print(f"Loaded {len(trades_data['coinbase'])} Coinbase trades")
    except:
        print("No Coinbase trades found")
        trades_data['coinbase'] = []
    
    # Load Kraken trades (from the sample)
    try:
        with open('../market_data/kraken_trades_full.json', 'r') as f:
            kr_data = json.load(f)
        trades_data['kraken'] = kr_data['data'][:1000]  # Use first 1000 as sample
        print(f"Loaded {len(trades_data['kraken'])} Kraken trades (sample)")
    except:
        print("No Kraken trades found")
        trades_data['kraken'] = []
    
    if not trades_data['coinbase'] or not trades_data['kraken']:
        print("\n⚠️  Need trades from both exchanges for arbitrage analysis")
        print("\nDemonstrating the concept with OHLC data instead...")
        demonstrate_with_ohlc()
        return
    
    # Convert to DataFrames
    cb_df = pd.DataFrame(trades_data['coinbase'])
    kr_df = pd.DataFrame(trades_data['kraken'])
    
    # Ensure timestamp columns
    cb_df['timestamp'] = pd.to_datetime(cb_df['timestamp'], unit='s')
    kr_df['timestamp'] = pd.to_datetime(kr_df['timestamp'])
    
    print("\n" + "=" * 60)
    print("WHY TICK DATA MATTERS FOR ARBITRAGE")
    print("=" * 60)
    
    print("\n1. TIMING PRECISION:")
    print("   - OHLC bars aggregate trades over 1 minute (60 seconds)")
    print("   - Actual arbitrage opportunities last seconds or less")
    print("   - Tick data shows EXACT execution prices and times")
    
    print("\n2. VOLUME ACCURACY:")
    print("   - OHLC shows total volume per bar")
    print("   - Tick data shows EACH trade size")
    print("   - You can calculate if there's enough liquidity for your trade")
    
    print("\n3. SPREAD CALCULATION:")
    print("   - OHLC uses average/close prices")
    print("   - Tick data shows actual bid/ask via trade sides")
    print("   - Real spread = best ask - best bid at same moment")
    
    print("\n4. EXECUTION REALITY:")
    print("   - You can't execute at OHLC prices")
    print("   - You execute against specific orders (ticks)")
    print("   - Slippage calculation needs tick-level data")

def demonstrate_with_ohlc():
    """Demonstrate arbitrage analysis with OHLC data"""
    import duckdb
    
    try:
        conn = duckdb.connect('../market_data/market_data.duckdb', read_only=True)
        
        # Get overlapping OHLC data
        query = """
        WITH aligned_data AS (
            SELECT 
                DATE_TRUNC('minute', datetime) as minute,
                exchange,
                AVG(close) as price,
                SUM(volume) as volume
            FROM ohlcv
            WHERE symbol = 'BTC/USD'
                AND datetime >= '2025-08-04'
            GROUP BY minute, exchange
        ),
        spreads AS (
            SELECT 
                cb.minute,
                cb.price as cb_price,
                kr.price as kr_price,
                cb.price - kr.price as spread,
                ABS(cb.price - kr.price) as abs_spread,
                cb.volume as cb_volume,
                kr.volume as kr_volume
            FROM aligned_data cb
            JOIN aligned_data kr ON cb.minute = kr.minute
            WHERE cb.exchange = 'coinbase' 
                AND kr.exchange = 'kraken'
        )
        SELECT * FROM spreads
        WHERE abs_spread > 5
        ORDER BY abs_spread DESC
        LIMIT 20
        """
        
        result = conn.execute(query).fetchall()
        
        if result:
            print("\n" + "=" * 60)
            print("ARBITRAGE OPPORTUNITIES (OHLC-based)")
            print("=" * 60)
            
            print("\nTop price discrepancies found:")
            print("-" * 60)
            
            total_opportunities = 0
            total_profit = 0
            
            for row in result[:10]:
                minute, cb_price, kr_price, spread, abs_spread, cb_vol, kr_vol = row
                
                if spread > 0:
                    action = "Buy KR @ ${:,.2f}, Sell CB @ ${:,.2f}".format(kr_price, cb_price)
                else:
                    action = "Buy CB @ ${:,.2f}, Sell KR @ ${:,.2f}".format(cb_price, kr_price)
                
                # Estimate profit (simplified)
                trade_size = min(cb_vol, kr_vol, 0.1)  # Trade up to 0.1 BTC
                profit = abs_spread * trade_size
                
                print(f"\n{minute}:")
                print(f"  Spread: ${abs_spread:.2f}")
                print(f"  Action: {action}")
                print(f"  Est. Profit: ${profit:.2f} (on {trade_size:.4f} BTC)")
                
                total_opportunities += 1
                total_profit += profit
            
            print("\n" + "=" * 60)
            print("SUMMARY")
            print("=" * 60)
            print(f"Opportunities found: {total_opportunities}")
            print(f"Potential profit: ${total_profit:.2f}")
            
            print("\n⚠️  LIMITATIONS OF OHLC-BASED ANALYSIS:")
            print("  1. Can't execute at these exact prices")
            print("  2. Ignores intra-minute price movements")
            print("  3. No bid-ask spread information")
            print("  4. No order book depth data")
            print("  5. Assumes instant execution (unrealistic)")
            
        else:
            print("\nNo significant arbitrage opportunities found")
        
        conn.close()
        
    except Exception as e:
        print(f"Error analyzing OHLC data: {e}")
    
    print("\n" + "=" * 60)
    print("RECOMMENDATIONS")
    print("=" * 60)
    print("\nFor production arbitrage trading:")
    print("1. Use WebSocket feeds for real-time tick data")
    print("2. Build order book from L2 data")
    print("3. Calculate actual executable spreads")
    print("4. Account for fees and slippage")
    print("5. Implement sub-second execution")
    
    print("\nFor market making (mentioned in your question):")
    print("1. L2 data is ESSENTIAL - shows order book depth")
    print("2. Need to see bid/ask queues to place orders")
    print("3. Must track order book changes in real-time")
    print("4. Consider using Coinbase Advanced Trade API")
    print("5. Kraken provides L2 via WebSocket (not historical)")

if __name__ == "__main__":
    analyze_arbitrage()
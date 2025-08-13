# Tick-Level Arbitrage & Market Making Analysis Template
# Comprehensive analysis using tick data from DuckDB

template = {
    "title": "Tick-Level Arbitrage & Market Making Analysis",
    "description": "Advanced tick-level analysis of arbitrage opportunities and market making potential using real streaming data",
    "cells": [
        {
            "type": "markdown",
            "content": """# Tick-Level Arbitrage & Market Making Analysis

This notebook analyzes:
1. **Cross-exchange arbitrage opportunities** (convergence betting)
   - Price differences between Coinbase and Kraken for same asset
   - Buy on cheaper exchange, sell on more expensive exchange
2. **Market making potential** on each exchange  
   - Bid-ask spreads within each exchange
   - Profit from providing liquidity
3. **Hybrid strategies** combining both approaches
4. **Hedging analysis** using more liquid platform (Coinbase)

**Important**: We analyze TWO different types of "spreads":
- **Cross-exchange spread**: Price difference between exchanges (for arbitrage)
- **Bid-ask spread**: Buy/sell price difference on same exchange (for market making)

Using real streaming trade data from Coinbase and Kraken stored in DuckDB."""
        },
        {
            "type": "code",
            "content": """# Import required libraries
import pandas as pd
import numpy as np
import duckdb
import matplotlib.pyplot as plt
import seaborn as sns
from datetime import datetime, timedelta
import warnings
warnings.filterwarnings('ignore')

# Set up plotting
plt.style.use('seaborn-v0_8')
sns.set_palette("husl")

print("="*80)
print("TICK-LEVEL ARBITRAGE & MARKET MAKING ANALYSIS")
print("="*80)"""
        },
        {
            "type": "code",
            "content": """# Connect to our live trade database
conn = duckdb.connect('../backend/market_data/market_data.duckdb', read_only=True)

# Data Overview & Quality Check
print("\\n" + "="*60)
print("1. DATA OVERVIEW")  
print("="*60)

overview_query = \"\"\"
SELECT 
    exchange,
    symbol,
    COUNT(*) as trade_count,
    MIN(datetime) as first_trade,
    MAX(datetime) as last_trade,
    ROUND(MIN(price), 2) as min_price,
    ROUND(MAX(price), 2) as max_price,
    ROUND(AVG(price), 2) as avg_price,
    ROUND(SUM(size), 4) as total_volume,
    ROUND(AVG(size), 6) as avg_trade_size,
    ROUND(COUNT(*) / EXTRACT(HOUR FROM (MAX(datetime) - MIN(datetime))) * 1.0, 0) as trades_per_hour
FROM trades 
GROUP BY exchange, symbol 
ORDER BY exchange, symbol
\"\"\"

overview_df = conn.execute(overview_query).df()
print(overview_df.to_string(index=False))

print(\"\\\\n📊 STREAMING VOLUME ANALYSIS:\")
for _, row in overview_df.iterrows():
    exchange = row['exchange'].upper()
    hourly_rate = row['trades_per_hour']
    total_volume = row['total_volume']
    time_span = f\"{row['first_trade']} to {row['last_trade']}\"
    
    print(f\"{exchange}:\")
    print(f\"  • {hourly_rate:,} trades/hour average\")
    print(f\"  • {total_volume:.2f} BTC total volume\")
    print(f\"  • Data span: {time_span}\")
    
    if exchange == 'COINBASE' and hourly_rate > 10000:
        print(f\"  • ✅ High liquidity - excellent for hedging\")
    elif exchange == 'KRAKEN' and hourly_rate > 1000:
        print(f\"  • ✅ Good liquidity - suitable for market making\")
    else:
        print(f\"  • ⚠️ Lower liquidity - higher impact costs\")"""
        },
        {
            "type": "code",
            "content": """# Data quality checks
print("\\n" + "-"*40)
print("DATA QUALITY CHECKS")
print("-"*40)

quality_checks = conn.execute(\"\"\"
SELECT
    'Total Trades' as metric,
    COUNT(*) as value
FROM trades
UNION ALL
SELECT
    'Unique Trade IDs' as metric,
    COUNT(DISTINCT trade_id) as value
FROM trades
UNION ALL
SELECT
    'Exchanges' as metric,
    COUNT(DISTINCT exchange) as value
FROM trades
UNION ALL
SELECT
    'Symbols' as metric,
    COUNT(DISTINCT symbol) as value
FROM trades
UNION ALL
SELECT
    'Price Anomalies (>$200k or <$1k)' as metric,
    COUNT(*) as value
FROM trades
WHERE price > 200000 OR price < 1000
\"\"\").df()

for _, row in quality_checks.iterrows():
    print(f"{row['metric']}: {row['value']:,}")"""
        },
        {
            "type": "code",
            "content": """# Cross-Exchange Spread Analysis using OHLCV data
print("\\n" + "="*60)
print("2. CROSS-EXCHANGE SPREAD ANALYSIS")
print("="*60)

# Use OHLCV data which has better overlap  
spread_query = \"\"\"
WITH ohlcv_spreads AS (
    SELECT
        cb.timestamp,
        to_timestamp(cb.timestamp) as datetime,
        cb.close as coinbase_price,
        kr.close as kraken_price,
        ABS(cb.close - kr.close) as spread,
        cb.close - kr.close as price_diff,
        ABS(cb.close - kr.close) / LEAST(cb.close, kr.close) * 100 as spread_pct,
        CASE 
            WHEN cb.close > kr.close THEN 'Buy Kraken, Sell Coinbase'
            WHEN kr.close > cb.close THEN 'Buy Coinbase, Sell Kraken'
            ELSE 'No Opportunity'
        END as direction,
        cb.volume as cb_volume,
        kr.volume as kr_volume
    FROM ohlcv cb
    JOIN ohlcv kr ON cb.timestamp = kr.timestamp AND cb.symbol = kr.symbol
    WHERE cb.exchange = 'coinbase' 
        AND kr.exchange = 'kraken'
        AND cb.symbol = 'BTC/USD'
    ORDER BY cb.timestamp DESC
    LIMIT 1000
)
SELECT * FROM ohlcv_spreads
\"\"\"

try:
    spreads_df = conn.execute(spread_query).df()
    
    if len(spreads_df) > 0:
        print(f"Analyzed {len(spreads_df)} synchronized 1-minute candles")
        
        print("\\nSPREAD STATISTICS:")
        print(f"Average Spread: ${spreads_df['spread'].mean():.2f}")
        print(f"Spread Std Dev: ${spreads_df['spread'].std():.2f}")
        print(f"Max Spread: ${spreads_df['spread'].max():.2f}")
        print(f"Min Spread: ${spreads_df['spread'].min():.2f}")
        print(f"Avg Spread %: {spreads_df['spread_pct'].mean():.4f}%")
        
        print("\\nDIRECTIONAL BREAKDOWN:")
        direction_counts = spreads_df['direction'].value_counts()
        for direction, count in direction_counts.items():
            print(f"{direction}: {count} opportunities")
            
        print("\\nPRICE DIFFERENCE STATS:")
        print(f"Avg price diff (Coinbase - Kraken): ${spreads_df['price_diff'].mean():.2f}")
        print(f"Times Coinbase higher: {(spreads_df['price_diff'] > 0).sum()}")
        print(f"Times Kraken higher: {(spreads_df['price_diff'] < 0).sum()}")
        
        # Store spreads_df for plotting later
        globals()['spreads_data'] = spreads_df
        
        print(f"\\nTime range: {spreads_df['datetime'].min()} to {spreads_df['datetime'].max()}")
        
        # ARBITRAGE OPPORTUNITIES ANALYSIS (included in same cell)
        print(f"\\n" + "-"*40)
        print(f"CROSS-EXCHANGE ARBITRAGE ANALYSIS")
        print(f"-"*40)
        
        print(f"✅ SUCCESS: Found {len(spreads_df)} synchronized 1-minute candles!")
        print(f"This confirms both exchanges trade simultaneously.")
        
        # Find arbitrage opportunities (spread > $10)
        arb_ops = spreads_df[spreads_df['spread'] > 10]
        print(f"\\nARBITRAGE OPPORTUNITIES (>$10 spread): {len(arb_ops)}")
        
        if len(arb_ops) > 0:
            print("\\nTop 5 Arbitrage Opportunities:")
            top_arbs = arb_ops.nlargest(5, 'spread')[['datetime', 'coinbase_price', 'kraken_price', 'spread', 'spread_pct', 'direction']]
            print(top_arbs.to_string(index=False))
            
            # Estimate profit potential
            avg_trade_size = 0.1  # BTC
            profit_per_trade = arb_ops['spread'].mean() * avg_trade_size
            time_range_hours = (spreads_df['datetime'].max() - spreads_df['datetime'].min()).total_seconds() / 3600
            opportunities_per_hour = len(arb_ops) / max(time_range_hours, 1)
            potential_hourly_profit = opportunities_per_hour * profit_per_trade
            
            print(f"\\nTHEORETICAL PROFIT (ignoring transfer costs):")
            print(f"Avg gross profit per arbitrage: ${profit_per_trade:.2f}")
            print(f"Estimated opportunities/hour: {opportunities_per_hour:.1f}")
            print(f"Potential gross hourly profit: ${potential_hourly_profit:.2f}")
            
            print(f"\\n❌ REALITY CHECK - WHY THIS DOESN'T WORK:")
            exchange_fees = profit_per_trade * 0.005  # 0.25% each side
            network_fees = 15  # Conservative estimate
            withdrawal_fees = 25  # Conservative estimate
            total_fees = exchange_fees + network_fees + withdrawal_fees
            
            print(f"Exchange fees (0.25% × 2): ${exchange_fees:.2f} per trade")
            print(f"Bitcoin network fees: ~${network_fees} per transfer")  
            print(f"Withdrawal fees: ~${withdrawal_fees} per exchange")
            print(f"Total fees: ~${total_fees:.2f} per trade")
            print(f"Net result: ${profit_per_trade - total_fees:.2f} per trade")
            
            if profit_per_trade - total_fees < 0:
                print(f"❌ UNPROFITABLE after all fees!")
            
            print(f"Transfer time: 10-60 minutes (massive price risk)")
            print(f"\\n💡 CONVERGENCE BETTING: Use price differences as signals, not instant arbitrage")
        else:
            print("No opportunities above $10 threshold.")
            print("This suggests markets are reasonably efficient.")
            
        print(f"\\n📊 MARKET EFFICIENCY METRICS:")
        print(f"Average spread: ${spreads_df['spread'].mean():.2f}")
        print(f"Median spread: ${spreads_df['spread'].median():.2f}")
        print(f"95th percentile spread: ${spreads_df['spread'].quantile(0.95):.2f}")
        efficiency_pct = (spreads_df['spread'] < 5).sum() / len(spreads_df) * 100
        print(f"Spreads under $5: {efficiency_pct:.1f}% of time")
        print(f"Market efficiency: {'High' if efficiency_pct > 80 else 'Moderate' if efficiency_pct > 60 else 'Low'}")
        
    else:
        print("No synchronized price data found. Checking data availability...")
        
        # Debug info
        debug_query = \"\"\"
        SELECT 
            exchange,
            COUNT(*) as candles,
            MIN(timestamp) as first_ts,
            MAX(timestamp) as last_ts
        FROM ohlcv 
        WHERE symbol = 'BTC/USD'
        GROUP BY exchange
        \"\"\"
        debug_df = conn.execute(debug_query).df()
        print("\\nOHLCV Data availability:")
        print(debug_df)
        globals()['spreads_data'] = None
        
except Exception as e:
    print(f"Error in spread analysis: {e}")
    globals()['spreads_data'] = None"""
        },
        {
            "type": "code",
            "content": """# Real Tick Analysis (Kraken)
print("\\n" + "="*60)
print("3. REAL TICK-LEVEL ANALYSIS")
print("="*60)

print("⚠️  IMPORTANT: Previous 'bid-ask spread' analysis was WRONG!")
print("We don't have order book data - only trade execution data.")
print("Let's analyze what we actually have: real tick-by-tick trades.")

# Real tick analysis using Kraken data
tick_analysis_query = \"\"\"
WITH kraken_ticks AS (
    SELECT 
        datetime,
        price,
        side,
        size,
        trade_id,
        LAG(price) OVER (ORDER BY datetime) as prev_price,
        LAG(side) OVER (ORDER BY datetime) as prev_side,
        EXTRACT(EPOCH FROM (datetime - LAG(datetime) OVER (ORDER BY datetime))) as time_gap
    FROM trades 
    WHERE exchange = 'kraken' AND symbol = 'BTC/USD'
        AND datetime >= CURRENT_TIMESTAMP - INTERVAL 6 HOUR
    ORDER BY datetime
),
price_levels AS (
    SELECT 
        ROUND(price, 0) as level,
        side,
        COUNT(*) as trade_count,
        SUM(size) as volume,
        AVG(size) as avg_trade_size,
        MIN(datetime) as first_trade,
        MAX(datetime) as last_trade
    FROM kraken_ticks
    GROUP BY ROUND(price, 0), side
    HAVING COUNT(*) >= 2
)
SELECT * FROM price_levels 
ORDER BY level DESC, side
LIMIT 20
\"\"\"

try:
    # Use the same connection from earlier
    tick_data = conn.execute(tick_analysis_query).df()
    
    print(f"\\nKRAKEN TICK DATA ANALYSIS:")
    print(f"Found {len(tick_data)} price levels with 2+ trades")
    print("\\nTop price levels by activity:")
    print(tick_data.head(10).to_string(index=False))
    
    # Market making simulation
    print("\\n" + "-"*50)
    print("MARKET MAKING SIMULATION")
    print("-"*50)
    
    simulation_query = \"\"\"
    WITH buy_levels AS (
        SELECT ROUND(price, 0) as level, COUNT(*) as trades, SUM(size) as volume
        FROM trades 
        WHERE exchange = 'kraken' AND symbol = 'BTC/USD'
            AND side = 'buy'
            AND datetime >= CURRENT_TIMESTAMP - INTERVAL 6 HOUR
        GROUP BY ROUND(price, 0)
        HAVING COUNT(*) >= 2
    ),
    sell_levels AS (
        SELECT ROUND(price, 0) as level, COUNT(*) as trades, SUM(size) as volume
        FROM trades 
        WHERE exchange = 'kraken' AND symbol = 'BTC/USD'
            AND side = 'sell'
            AND datetime >= CURRENT_TIMESTAMP - INTERVAL 6 HOUR
        GROUP BY ROUND(price, 0)
        HAVING COUNT(*) >= 2
    )
    SELECT 
        b.level as buy_level,
        s.level as sell_level,
        s.level - b.level as spread,
        b.trades as buy_trades,
        s.trades as sell_trades,
        LEAST(b.volume, s.volume) as potential_volume
    FROM buy_levels b
    CROSS JOIN sell_levels s
    WHERE s.level > b.level
        AND s.level - b.level >= 5
    ORDER BY spread ASC
    LIMIT 10
    \"\"\"
    
    opportunities = conn.execute(simulation_query).df()
    
    if len(opportunities) > 0:
        print(f"Found {len(opportunities)} potential market making opportunities:")
        print(opportunities.to_string(index=False))
        
        print("\\n" + "-"*30)
        print("PROFIT ANALYSIS")
        print("-"*30)
        
        for i, row in opportunities.head(5).iterrows():
            spread = row['spread']
            buy_level = row['buy_level']
            sell_level = row['sell_level']
            
            # Market making economics
            gross_profit_per_btc = spread * 0.5  # Capture 50% of spread
            avg_price = (buy_level + sell_level) / 2
            fee_per_btc = avg_price * 0.0025  # 0.25% Kraken retail fee
            net_profit_per_btc = gross_profit_per_btc - fee_per_btc
            
            print(f"\\nSpread ${spread}:")
            print(f"  Gross profit: ${gross_profit_per_btc:.2f}/BTC")
            print(f"  Kraken fee (0.25%): ${fee_per_btc:.2f}/BTC")
            print(f"  NET profit: ${net_profit_per_btc:.2f}/BTC")
            
            if net_profit_per_btc > 0:
                print(f"  ✅ Profitable!")
            else:
                print(f"  ❌ Unprofitable - fees too high")
                
        print(f"\\n💡 KEY INSIGHTS:")
        print(f"1. We have REAL tick data showing actual trading activity")
        print(f"2. Price levels show where trades actually happen")
        print(f"3. But retail fees (0.25%) kill most opportunities")
        print(f"4. Need institutional rates (0.00%) to be profitable")
                
    else:
        print("No market making opportunities found with current criteria")
        
    # Store data for visualization
    globals()['tick_analysis_data'] = tick_data
    globals()['market_opportunities'] = opportunities if len(opportunities) > 0 else None

except Exception as e:
    print(f"Error in tick analysis: {e}")
    globals()['tick_analysis_data'] = None
    globals()['market_opportunities'] = None"""
        },
        {
            "type": "code", 
            "content": """# Visualize spread analysis
if 'spreads_data' in globals() and spreads_data is not None and len(spreads_data) > 0:
    fig, axes = plt.subplots(3, 1, figsize=(15, 12))
    
    # Price comparison over time
    axes[0].plot(spreads_data['datetime'], spreads_data['coinbase_price'], 
                label='Coinbase', alpha=0.7, linewidth=2)
    axes[0].plot(spreads_data['datetime'], spreads_data['kraken_price'], 
                label='Kraken', alpha=0.7, linewidth=2)
    axes[0].set_ylabel('Price (USD)')
    axes[0].set_title('BTC/USD Prices by Exchange (1-Minute Windows)')
    axes[0].legend()
    axes[0].grid(True, alpha=0.3)
    
    # Cross-exchange spread (always positive)
    axes[1].plot(spreads_data['datetime'], spreads_data['spread'], 
                color='purple', alpha=0.7, linewidth=2)
    axes[1].axhline(y=10, color='red', linestyle='--', alpha=0.5, label='Arbitrage threshold ($10)')
    axes[1].fill_between(spreads_data['datetime'], spreads_data['spread'], 0, alpha=0.3)
    axes[1].set_ylabel('Spread (USD)')
    axes[1].set_title('Cross-Exchange Spread (Always Positive for Arbitrage)')
    axes[1].legend()
    axes[1].grid(True, alpha=0.3)
    
    # Percentage spread
    axes[2].plot(spreads_data['datetime'], spreads_data['spread_pct'], 
                color='green', alpha=0.7, linewidth=2)
    axes[2].axhline(y=0.1, color='red', linestyle='--', alpha=0.5, label='Profit threshold (0.1%)')
    axes[2].axhline(y=0, color='black', linestyle='-', alpha=0.3)
    axes[2].fill_between(spreads_data['datetime'], spreads_data['spread_pct'], 0, alpha=0.3)
    axes[2].set_ylabel('Spread (%)')
    axes[2].set_xlabel('Time')
    axes[2].set_title('Percentage Spread with Arbitrage Threshold')
    axes[2].legend()
    axes[2].grid(True, alpha=0.3)
    
    plt.tight_layout()
    plt.show()
    
    print("Spread visualization complete!")
else:
    print("No spread data available for visualization.")"""
        },
        {
            "type": "code",
            "content": """# Volume and Volatility Analysis
print("\\n" + "="*60)
print("4. VOLUME & VOLATILITY CONDITIONAL ANALYSIS")
print("="*60)

print("Analysis: How do opportunities change with market conditions?")
print()

# Volume-based opportunity analysis
volume_conditions_query = '''
WITH hourly_activity AS (
    SELECT 
        DATE_TRUNC('hour', datetime) as hour,
        exchange,
        COUNT(*) as trades_per_hour,
        SUM(size) as volume_btc,
        AVG(price) as avg_price,
        STDDEV(price) as price_volatility,
        MAX(price) - MIN(price) as price_range
    FROM trades 
    WHERE symbol = 'BTC/USD'
        AND datetime >= CURRENT_TIMESTAMP - INTERVAL 24 HOUR
    GROUP BY hour, exchange
    HAVING COUNT(*) >= 10
),
activity_categories AS (
    SELECT *,
        CASE 
            WHEN trades_per_hour >= 5000 THEN 'Very High'
            WHEN trades_per_hour >= 2000 THEN 'High'
            WHEN trades_per_hour >= 500 THEN 'Medium'
            ELSE 'Low'
        END as volume_category,
        CASE 
            WHEN price_volatility >= 50 THEN 'High Vol'
            WHEN price_volatility >= 20 THEN 'Medium Vol'
            ELSE 'Low Vol'
        END as volatility_category
    FROM hourly_activity
    WHERE price_volatility IS NOT NULL
)
SELECT 
    exchange,
    volume_category,
    volatility_category,
    COUNT(*) as hours,
    AVG(trades_per_hour) as avg_trades_hour,
    AVG(volume_btc) as avg_volume_btc,
    AVG(price_range) as avg_hourly_range,
    AVG(price_volatility) as avg_volatility
FROM activity_categories
GROUP BY exchange, volume_category, volatility_category
ORDER BY exchange, avg_trades_hour DESC
'''

volume_data = conn.execute(volume_conditions_query).df()

print("VOLUME & VOLATILITY CONDITIONS:")
for exchange in ['coinbase', 'kraken']:
    exchange_data = volume_data[volume_data['exchange'] == exchange]
    if len(exchange_data) > 0:
        print(f"\\n{exchange.upper()}:")
        
        for _, row in exchange_data.iterrows():
            vol_cat = row['volume_category']
            vol_cat_short = row['volatility_category']
            hours = row['hours']
            avg_trades = row['avg_trades_hour']
            avg_range = row['avg_hourly_range']
            
            print(f"  {vol_cat} Volume + {vol_cat_short}: {hours} hours")
            print(f"    {avg_trades:.0f} trades/hr, ${avg_range:.2f} hourly range")
            
            # Trading recommendations
            if vol_cat == 'Very High' and 'High' in vol_cat_short:
                print(f"    🚀 OPTIMAL: High volume + volatility = best opportunities")
            elif vol_cat in ['High', 'Very High']:
                print(f"    ✅ GOOD: Sufficient volume for execution")
            elif 'High' in vol_cat_short:
                print(f"    ⚡ VOLATILE: Wider spreads but higher risk")
            else:
                print(f"    ⚠️ CAUTION: Limited opportunities")"""
        },
        {
            "type": "code",
            "content": """# Correlation with Coinbase Deviation Analysis
print("\\n" + "="*60)
print("5. DEVIATION FROM COINBASE ANALYSIS")
print("="*60)

print("Analysis: How does Kraken price deviation correlate with opportunities?")
print()

deviation_analysis_query = '''
WITH price_comparison AS (
    SELECT
        DATE_TRUNC('minute', datetime) as minute,
        exchange,
        AVG(price) as avg_price,
        COUNT(*) as trade_count,
        SUM(size) as volume,
        STDDEV(price) as price_volatility
    FROM trades
    WHERE symbol = 'BTC/USD'
        AND datetime >= CURRENT_TIMESTAMP - INTERVAL 12 HOUR
    GROUP BY minute, exchange
    HAVING COUNT(*) >= 1
),
deviations AS (
    SELECT
        cb.minute,
        cb.avg_price as coinbase_price,
        kr.avg_price as kraken_price,
        cb.avg_price - kr.avg_price as price_deviation,
        ABS(cb.avg_price - kr.avg_price) as abs_deviation,
        (cb.avg_price - kr.avg_price) / cb.avg_price * 100 as deviation_pct,
        cb.trade_count as cb_activity,
        kr.trade_count as kr_activity,
        cb.volume as cb_volume,
        kr.volume as kr_volume,
        COALESCE(cb.price_volatility, 0) + COALESCE(kr.price_volatility, 0) as combined_volatility
    FROM price_comparison cb
    JOIN price_comparison kr ON cb.minute = kr.minute
    WHERE cb.exchange = 'coinbase' 
        AND kr.exchange = 'kraken'
),
deviation_categories AS (
    SELECT *,
        CASE 
            WHEN ABS(deviation_pct) >= 0.1 THEN 'Large Deviation (>0.1%)'
            WHEN ABS(deviation_pct) >= 0.05 THEN 'Medium Deviation (0.05-0.1%)'
            WHEN ABS(deviation_pct) >= 0.02 THEN 'Small Deviation (0.02-0.05%)'
            ELSE 'Tight Coupling (<0.02%)'
        END as deviation_category,
        CASE 
            WHEN price_deviation > 0 THEN 'Coinbase Premium'
            WHEN price_deviation < 0 THEN 'Kraken Premium'
            ELSE 'Equal'
        END as premium_direction
    FROM deviations
)
SELECT 
    deviation_category,
    premium_direction,
    COUNT(*) as minutes,
    AVG(abs_deviation) as avg_abs_deviation,
    AVG(ABS(deviation_pct)) as avg_abs_deviation_pct,
    AVG(cb_activity) as avg_cb_trades,
    AVG(kr_activity) as avg_kr_trades,
    AVG(combined_volatility) as avg_volatility
FROM deviation_categories
GROUP BY deviation_category, premium_direction
ORDER BY avg_abs_deviation DESC
'''

deviation_data = conn.execute(deviation_analysis_query).df()

print("DEVIATION FROM COINBASE ANALYSIS:")
if len(deviation_data) > 0:
    total_minutes = deviation_data['minutes'].sum()
    
    for _, row in deviation_data.iterrows():
        category = row['deviation_category']
        direction = row['premium_direction']
        minutes = row['minutes']
        pct_time = (minutes / total_minutes) * 100
        avg_dev = row['avg_abs_deviation']
        avg_vol = row['avg_volatility']
        
        print(f"\\n{category} - {direction}:")
        print(f"  Frequency: {minutes} minutes ({pct_time:.1f}% of time)")
        print(f"  Avg deviation: ${avg_dev:.2f}")
        print(f"  Avg volatility: ${avg_vol:.2f}")
        
        # Opportunity assessment
        if 'Large' in category:
            print(f"  🎯 CONVERGENCE OPPORTUNITY: High probability mean reversion")
        elif 'Medium' in category:
            print(f"  ✅ MODERATE OPPORTUNITY: Good signal strength")
        elif 'Small' in category:
            print(f"  ⚠️ WEAK SIGNAL: Limited profit potential")
        else:
            print(f"  📊 EFFICIENT MARKET: Prices tightly coupled")
    
    # Correlation analysis
    print(f"\\nKEY INSIGHTS:")
    large_dev_time = deviation_data[deviation_data['deviation_category'].str.contains('Large')]['minutes'].sum()
    large_dev_pct = (large_dev_time / total_minutes) * 100
    
    print(f"• Large deviations occur {large_dev_pct:.1f}% of time")
    
    cb_premium_time = deviation_data[deviation_data['premium_direction'] == 'Coinbase Premium']['minutes'].sum()
    cb_premium_pct = (cb_premium_time / total_minutes) * 100
    
    print(f"• Coinbase trades at premium {cb_premium_pct:.1f}% of time")
    print(f"• Price coupling efficiency: {'High' if large_dev_pct < 5 else 'Moderate' if large_dev_pct < 15 else 'Low'}")
    
else:
    print("❌ No deviation data found - check synchronization")"""
        },
        {
            "type": "code",
            "content": """# Enhanced Volatility Correlation Analysis
print("\\n" + "="*60)
print("6. VOLATILITY-OPPORTUNITY CORRELATION")
print("="*60)

print("Analysis: How do volatility spikes correlate with trading opportunities?")
print()

volatility_correlation_query = '''
WITH volatility_windows AS (
    SELECT 
        DATE_TRUNC('minute', datetime) as minute,
        exchange,
        COUNT(*) as trade_count,
        AVG(price) as avg_price,
        MIN(price) as min_price,
        MAX(price) as max_price,
        MAX(price) - MIN(price) as price_range,
        STDDEV(price) as price_std,
        SUM(size) as volume
    FROM trades 
    WHERE symbol = 'BTC/USD'
        AND datetime >= CURRENT_TIMESTAMP - INTERVAL 8 HOUR
    GROUP BY minute, exchange
    HAVING COUNT(*) >= 3
),
volatility_analysis AS (
    SELECT 
        minute,
        exchange,
        trade_count,
        avg_price,
        price_range,
        price_std,
        volume,
        CASE 
            WHEN price_range >= 20 THEN 'High Volatility'
            WHEN price_range >= 10 THEN 'Medium Volatility'
            WHEN price_range >= 5 THEN 'Low Volatility'
            ELSE 'Very Low Volatility'
        END as volatility_level
    FROM volatility_windows
)
SELECT 
    exchange,
    volatility_level,
    COUNT(*) as periods,
    AVG(trade_count) as avg_trades_per_min,
    AVG(price_range) as avg_price_range,
    AVG(volume) as avg_volume,
    AVG(price_std) as avg_price_std
FROM volatility_analysis
GROUP BY exchange, volatility_level
ORDER BY exchange, avg_price_range DESC
'''

volatility_data = conn.execute(volatility_correlation_query).df()

print("VOLATILITY-OPPORTUNITY CORRELATION:")
for exchange in ['coinbase', 'kraken']:
    exchange_data = volatility_data[volatility_data['exchange'] == exchange]
    if len(exchange_data) > 0:
        print(f"\\n{exchange.upper()} VOLATILITY ANALYSIS:")
        
        total_periods = exchange_data['periods'].sum()
        
        for _, row in exchange_data.iterrows():
            vol_level = row['volatility_level']
            periods = row['periods']
            pct_time = (periods / total_periods) * 100
            avg_range = row['avg_price_range']
            avg_trades = row['avg_trades_per_min']
            
            print(f"  {vol_level}: {periods} minutes ({pct_time:.1f}%)")
            print(f"    Avg range: ${avg_range:.2f}, {avg_trades:.1f} trades/min")
            
            # Opportunity correlation
            if vol_level == 'High Volatility':
                print(f"    🚀 PRIME TIME: Volatility creates wider spreads")
            elif vol_level == 'Medium Volatility':
                print(f"    ✅ ACTIVE: Good opportunity/risk balance")
            elif vol_level == 'Low Volatility':
                print(f"    📊 STEADY: Consistent but limited opportunities")
            else:
                print(f"    😴 QUIET: Minimal opportunities, tight spreads")

# Market regime identification
print(f"\\nMARKET REGIME IDENTIFICATION:")
print(f"Based on volatility distribution, current market shows:")

high_vol_pct = volatility_data[volatility_data['volatility_level'] == 'High Volatility']['periods'].sum()
total_all_periods = volatility_data['periods'].sum()
high_vol_percentage = (high_vol_pct / total_all_periods) * 100 if total_all_periods > 0 else 0

if high_vol_percentage > 20:
    print(f"🌪️  VOLATILE REGIME: {high_vol_percentage:.1f}% high volatility periods")
    print(f"   Strategy: Focus on volatility capture, wider stop losses")
elif high_vol_percentage > 10:
    print(f"⚡ ACTIVE REGIME: {high_vol_percentage:.1f}% high volatility periods") 
    print(f"   Strategy: Balanced approach, opportunistic entries")
else:
    print(f"📊 STABLE REGIME: {high_vol_percentage:.1f}% high volatility periods")
    print(f"   Strategy: Focus on tight spreads, low-risk strategies")"""
        },
        {
            "type": "code",
            "content": """# Hybrid Strategy Analysis
print("\\n" + "="*60)
print("7. HYBRID STRATEGY: CONVERGENCE + MARKET MAKING")
print("="*60)

# Reconnect to database (each cell is independent)
conn = duckdb.connect('../backend/market_data/market_data.duckdb', read_only=True)

print(\"\"\"
HYBRID STRATEGY CONCEPT:
1. MARKET MAKE on the less liquid exchange (Kraken)
   - Provide liquidity, earn spreads
   - Lower competition, wider spreads
   
2. HEDGE positions on more liquid exchange (Coinbase) 
   - Instant hedge execution
   - Lower slippage
   - Better price discovery
   
3. CONVERGENCE BETTING when spreads are wide
   - Buy low exchange, sell high exchange
   - Unwind when prices converge
   
4. RISK MANAGEMENT
   - Delta-neutral overall position
   - Automatic hedging triggers
   - Position size limits
\"\"\")

# Analyze hedging effectiveness using minute-level aggregation
hedge_query = \"\"\"
WITH price_comparison AS (
    SELECT
        DATE_TRUNC('minute', datetime) as minute,
        exchange,
        symbol,
        AVG(price) as avg_price,
        COUNT(*) as trades,
        SUM(size) as volume
    FROM trades
    WHERE symbol = 'BTC/USD'
        AND datetime >= CURRENT_TIMESTAMP - INTERVAL 6 HOUR
    GROUP BY minute, exchange, symbol
),
hedge_analysis AS (
    SELECT
        cb.minute,
        cb.avg_price as coinbase_price,
        kr.avg_price as kraken_price,
        cb.trades as cb_liquidity,
        kr.trades as kr_liquidity,
        cb.volume as cb_volume,
        kr.volume as kr_volume,
        ABS(cb.avg_price - kr.avg_price) as spread
    FROM price_comparison cb
    JOIN price_comparison kr ON cb.minute = kr.minute
    WHERE cb.exchange = 'coinbase' 
        AND kr.exchange = 'kraken'
)
SELECT
    AVG(coinbase_price) as avg_cb_price,
    AVG(kraken_price) as avg_kr_price,
    AVG(spread) as avg_spread,
    AVG(cb_liquidity) as avg_cb_trades_per_min,
    AVG(kr_liquidity) as avg_kr_trades_per_min,
    AVG(cb_volume) as avg_cb_volume_per_min,
    AVG(kr_volume) as avg_kr_volume_per_min,
    COUNT(*) as minutes_analyzed
FROM hedge_analysis
\"\"\"

try:
    # Use the same connection from earlier cells
    hedge_df = conn.execute(hedge_query).df()
    
    if len(hedge_df) > 0 and hedge_df.iloc[0]['minutes_analyzed'] > 0:
        row = hedge_df.iloc[0]
        
        print(f"\\nHEDGING ANALYSIS ({row['minutes_analyzed']:.0f} minutes):")
        print(f"Average spread: ${row['avg_spread']:.2f}")
        print(f"Coinbase liquidity: {row['avg_cb_trades_per_min']:.1f} trades/min")
        print(f"Kraken liquidity: {row['avg_kr_trades_per_min']:.1f} trades/min")
        print(f"Liquidity ratio (CB/KR): {row['avg_cb_trades_per_min']/max(row['avg_kr_trades_per_min'], 1):.1f}x")
        
        # Strategy recommendation
        liquidity_ratio = row['avg_cb_trades_per_min']/max(row['avg_kr_trades_per_min'], 1)
        avg_spread = row['avg_spread']
        
        print(f"\\nSTRATEGY RECOMMENDATION:")
        if liquidity_ratio > 5 and avg_spread > 5:
            print("✅ HYBRID strategy recommended:")
            print("   - Market make on Kraken (lower competition)")
            print("   - Hedge on Coinbase (higher liquidity)")
            print("   - Convergence bet when spread > $10")
        elif avg_spread > 20:
            print("✅ CONVERGENCE strategy recommended:")
            print("   - Focus on arbitrage opportunities")
            print("   - Wide spreads indicate good profit potential")
        elif liquidity_ratio > 10:
            print("✅ MARKET MAKING on Kraken recommended:")
            print("   - Much lower competition than Coinbase")
            print("   - Use Coinbase for hedging")
        else:
            print("⚠️  More data needed for reliable recommendation")

except Exception as e:
    print(f"Error in hybrid analysis: {e}")"""
        },
        {
            "type": "code",
            "content": """# Risk Metrics & Implementation
print("\\n" + "="*60)
print("5. RISK METRICS & IMPLEMENTATION")
print("="*60)

# Reconnect to database (each cell is independent)
conn = duckdb.connect('../backend/market_data/market_data.duckdb', read_only=True)

# Volatility analysis
vol_query = \"\"\"
WITH price_changes AS (
    SELECT
        exchange,
        symbol,
        datetime,
        price,
        LAG(price) OVER (PARTITION BY exchange, symbol ORDER BY datetime) as prev_price
    FROM trades
    WHERE symbol = 'BTC/USD'
        AND datetime >= CURRENT_TIMESTAMP - INTERVAL 6 HOUR
),
returns AS (
    SELECT
        exchange,
        (price - prev_price) / prev_price as return_pct,
        ABS((price - prev_price) / prev_price) as abs_return_pct
    FROM price_changes
    WHERE prev_price IS NOT NULL
)
SELECT
    exchange,
    COUNT(*) as observations,
    AVG(return_pct) * 100 as avg_return_pct,
    STDDEV(return_pct) * 100 as volatility_pct,
    AVG(abs_return_pct) * 100 as avg_abs_return_pct,
    MAX(abs_return_pct) * 100 as max_abs_return_pct
FROM returns
GROUP BY exchange
\"\"\"

try:
    vol_df = conn.execute(vol_query).df()
    
    print("VOLATILITY ANALYSIS:")
    for _, row in vol_df.iterrows():
        exchange = row['exchange'].upper()
        print(f"\\n{exchange}:")
        print(f"  Observations: {row['observations']:,}")
        print(f"  Average return: {row['avg_return_pct']:.4f}%")
        print(f"  Volatility (std): {row['volatility_pct']:.4f}%")
        print(f"  Avg absolute move: {row['avg_abs_return_pct']:.4f}%")
        print(f"  Max single move: {row['max_abs_return_pct']:.4f}%")

except Exception as e:
    print(f"Error in volatility analysis: {e}")"""
        },
        {
            "type": "code", 
            "content": """# Implementation Notes & Next Steps
print(f"\\nIMPLEMENTATION NOTES:")
print(f"1. START SMALL: Begin with 0.01 BTC position sizes")
print(f"2. REAL-TIME MONITORING: WebSocket feeds are critical")  
print(f"3. LATENCY MATTERS: Co-location near exchanges recommended")
print(f"4. FEE STRUCTURE: ")
print(f"   - Coinbase Pro: 0.00%-0.50% maker, 0.04%-0.50% taker")
print(f"   - Kraken: 0.00%-0.16% maker, 0.10%-0.26% taker")
print(f"5. RISK LIMITS: Max 1% of portfolio per strategy")
print(f"6. HEDGING TRIGGERS: Auto-hedge when position > 0.1 BTC")

print(f"\\nNEXT STEPS:")
print(f"1. Build real-time spread monitoring dashboard")
print(f"2. Implement paper trading system")
print(f"3. Add order book data (L2) for better spread estimation")
print(f"4. Create automated hedging system")
print(f"5. Backtest strategies on longer time periods")

print(f"\\n" + "="*60)
print(f"FINAL REALITY CHECK: RETAIL VS INSTITUTIONAL")
print(f"="*60)

print(f\"\"\"
🏦 INSTITUTIONAL ADVANTAGES:
• Fee tiers: 0.00% maker fees vs 0.25% retail
• Co-location: <1ms latency vs 50-200ms retail
• Capital: $10M+ positions vs $10K retail limits
• Technology: FPGA trading engines vs browser-based execution
• Market access: Direct exchange feeds vs delayed data

🏠 RETAIL REALITY:
• Fees kill most arbitrage opportunities
• Latency makes instant arbitrage impossible  
• Small positions limit profit potential
• Manual execution introduces delays and errors
• Limited to convergence betting (not pure arbitrage)

💡 REALISTIC RETAIL STRATEGIES:
1. CONVERGENCE SIGNALS: Use spreads to predict direction
2. MARKET REGIME DETECTION: Wide spreads = volatility opportunities
3. PAIR TRADING: Long/short correlated assets
4. TREND FOLLOWING: Use cross-exchange momentum
5. VOLATILITY TRADING: Exploit regime changes

❌ AVOID: Trying to compete with HFT firms on pure arbitrage
✅ FOCUS: Longer-term strategies using cross-exchange signals
\"\"\")

# Close database connection
try:
    conn.close()
except:
    pass  # Connection may already be closed
    
print(f"\\n" + "="*80)
print(f"ANALYSIS COMPLETE - Realistic strategies identified above")
print(f"="*80)"""
        },
        {
            "type": "markdown",
            "content": """## Summary

This analysis provides:

### Key Metrics
- **Data Quality**: Trade count, unique IDs, time ranges
- **Spread Analysis**: Cross-exchange price differences over time
- **Market Making**: Estimated bid-ask spreads and profit potential
- **Volatility**: Risk metrics for position sizing

### Strategy Recommendations
- **Hybrid Approach**: Combine market making on less liquid exchange with hedging on more liquid one
- **Convergence Betting**: Exploit temporary price differences between exchanges
- **Risk Management**: Automated hedging triggers and position limits

### Implementation Considerations
- Start with small position sizes (0.01 BTC)
- Real-time data feeds essential for timing
- Account for exchange fees in profit calculations
- Monitor for regulatory changes affecting arbitrage

### Next Development Steps
1. Real-time dashboard for live monitoring
2. Paper trading system for strategy testing
3. Order book integration for better execution
4. Automated risk management system"""
        }
    ]
}
#!/usr/bin/env python3
"""
Tick-Level Arbitrage & Market Making Analysis
Using real streaming trade data from Coinbase and Kraken

This notebook analyzes:
1. Cross-exchange arbitrage opportunities (convergence betting)
2. Market making potential on each exchange
3. Hybrid strategies combining both approaches
4. Hedging analysis using more liquid platform (Coinbase)
"""

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
print("="*80)

# Connect to our live trade database
conn = duckdb.connect('../market_data/market_data.duckdb', read_only=True)

# =============================================================================
# 1. DATA OVERVIEW & QUALITY CHECK
# =============================================================================

print("\n" + "="*60)
print("1. DATA OVERVIEW")
print("="*60)

# Get basic statistics
overview_query = """
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
    ROUND(AVG(size), 6) as avg_trade_size
FROM trades 
GROUP BY exchange, symbol 
ORDER BY exchange, symbol
"""

overview_df = conn.execute(overview_query).df()
print(overview_df.to_string(index=False))

# Data quality checks
print("\n" + "-"*40)
print("DATA QUALITY CHECKS")
print("-"*40)

quality_checks = conn.execute("""
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
""").df()

for _, row in quality_checks.iterrows():
    print(f"{row['metric']}: {row['value']:,}")

# =============================================================================
# 2. CROSS-EXCHANGE SPREAD ANALYSIS
# =============================================================================

print("\n" + "="*60)
print("2. CROSS-EXCHANGE SPREAD ANALYSIS")
print("="*60)

# Get synchronized price data (using 5-second windows)
spread_query = """
WITH price_windows AS (
    SELECT
        DATE_TRUNC('second', datetime) + 
        INTERVAL (EXTRACT(second FROM datetime)::int / 5) * 5 SECOND as time_window,
        exchange,
        symbol,
        AVG(price) as avg_price,
        MIN(price) as min_price,
        MAX(price) as max_price,
        COUNT(*) as trades_in_window,
        SUM(size) as volume_in_window
    FROM trades
    WHERE symbol = 'BTC/USD'  -- Focus on BTC first
        AND datetime >= CURRENT_TIMESTAMP - INTERVAL '30 minutes'  -- Recent data
    GROUP BY time_window, exchange, symbol
    HAVING COUNT(*) >= 1  -- At least 1 trade in window
),
spreads AS (
    SELECT
        cb.time_window,
        cb.avg_price as coinbase_price,
        kr.avg_price as kraken_price,
        cb.avg_price - kr.avg_price as spread,
        ABS(cb.avg_price - kr.avg_price) as abs_spread,
        (cb.avg_price - kr.avg_price) / kr.avg_price * 100 as spread_pct,
        cb.trades_in_window as cb_trades,
        kr.trades_in_window as kr_trades,
        cb.volume_in_window as cb_volume,
        kr.volume_in_window as kr_volume
    FROM price_windows cb
    JOIN price_windows kr ON cb.time_window = kr.time_window 
        AND cb.symbol = kr.symbol
    WHERE cb.exchange = 'coinbase' 
        AND kr.exchange = 'kraken'
)
SELECT * FROM spreads
ORDER BY time_window DESC
LIMIT 100
"""

try:
    spreads_df = conn.execute(spread_query).df()
    
    if len(spreads_df) > 0:
        print(f"Analyzed {len(spreads_df)} synchronized 5-second windows")
        
        print("\nSPREAD STATISTICS:")
        print(f"Average Spread: ${spreads_df['spread'].mean():.2f}")
        print(f"Spread Std Dev: ${spreads_df['spread'].std():.2f}")
        print(f"Max Spread: ${spreads_df['spread'].max():.2f}")
        print(f"Min Spread: ${spreads_df['spread'].min():.2f}")
        print(f"Avg Absolute Spread: ${spreads_df['abs_spread'].mean():.2f}")
        print(f"Avg Spread %: {spreads_df['spread_pct'].mean():.4f}%")
        
        # Find arbitrage opportunities (spread > $10)
        arb_ops = spreads_df[spreads_df['abs_spread'] > 10]
        print(f"\nARBITRAGE OPPORTUNITIES (>$10 spread): {len(arb_ops)}")
        
        if len(arb_ops) > 0:
            print("\nTop 5 Arbitrage Opportunities:")
            top_arbs = arb_ops.nlargest(5, 'abs_spread')[['time_window', 'coinbase_price', 'kraken_price', 'spread', 'spread_pct']]
            print(top_arbs.to_string(index=False))
            
            # Estimate profit potential
            avg_trade_size = 0.1  # BTC
            profit_per_trade = arb_ops['abs_spread'].mean() * avg_trade_size
            opportunities_per_hour = len(arb_ops) * 2  # Extrapolate from 30min to 1hr
            potential_hourly_profit = opportunities_per_hour * profit_per_trade
            
            print(f"\nPROFIT ESTIMATION (for {avg_trade_size} BTC trades):")
            print(f"Avg profit per arbitrage: ${profit_per_trade:.2f}")
            print(f"Estimated opportunities/hour: {opportunities_per_hour}")
            print(f"Potential hourly profit: ${potential_hourly_profit:.2f}")
    else:
        print("No synchronized price data found. Need more overlapping trades.")
        
except Exception as e:
    print(f"Error in spread analysis: {e}")

# =============================================================================
# 3. MARKET MAKING ANALYSIS
# =============================================================================

print("\n" + "="*60)
print("3. MARKET MAKING ANALYSIS")
print("="*60)

# Analyze bid-ask spread proxies using trade patterns
mm_query = """
WITH trade_sequences AS (
    SELECT
        *,
        LAG(price) OVER (PARTITION BY exchange, symbol ORDER BY datetime) as prev_price,
        LAG(side) OVER (PARTITION BY exchange, symbol ORDER BY datetime) as prev_side,
        LAG(datetime) OVER (PARTITION BY exchange, symbol ORDER BY datetime) as prev_time,
        EXTRACT(EPOCH FROM (datetime - LAG(datetime) OVER (PARTITION BY exchange, symbol ORDER BY datetime))) as time_gap
    FROM trades
    WHERE symbol = 'BTC/USD'
        AND datetime >= CURRENT_TIMESTAMP - INTERVAL '30 minutes'
),
spread_estimates AS (
    SELECT
        exchange,
        datetime,
        price,
        prev_price,
        side,
        prev_side,
        ABS(price - prev_price) as price_change,
        time_gap,
        CASE 
            WHEN side != prev_side AND time_gap < 10 THEN ABS(price - prev_price)
            ELSE NULL
        END as estimated_spread
    FROM trade_sequences
    WHERE prev_price IS NOT NULL
)
SELECT
    exchange,
    COUNT(*) as total_trades,
    AVG(price_change) as avg_price_change,
    AVG(time_gap) as avg_time_between_trades,
    AVG(estimated_spread) as avg_estimated_spread,
    COUNT(estimated_spread) as spread_samples,
    MIN(estimated_spread) as min_spread,
    MAX(estimated_spread) as max_spread
FROM spread_estimates
GROUP BY exchange
"""

try:
    mm_df = conn.execute(mm_query).df()
    
    print("MARKET MAKING POTENTIAL:")
    for _, row in mm_df.iterrows():
        exchange = row['exchange'].upper()
        avg_spread = row['avg_estimated_spread'] if pd.notna(row['avg_estimated_spread']) else 0
        trade_freq = 1 / row['avg_time_between_trades'] if row['avg_time_between_trades'] > 0 else 0
        
        print(f"\n{exchange}:")
        print(f"  Total trades analyzed: {row['total_trades']:,}")
        print(f"  Avg time between trades: {row['avg_time_between_trades']:.1f}s")
        print(f"  Trade frequency: {trade_freq:.3f} trades/second")
        print(f"  Estimated bid-ask spread: ${avg_spread:.2f}")
        print(f"  Spread samples: {row['spread_samples']}")
        
        if avg_spread > 0:
            # Market making profit estimation
            capture_rate = 0.5  # Assume we capture 50% of spread
            trade_size = 0.01   # BTC per market make
            trades_per_hour = trade_freq * 3600 * 0.1  # Conservative: participate in 10% of trades
            profit_per_trade = avg_spread * capture_rate * trade_size
            hourly_profit = trades_per_hour * profit_per_trade
            
            print(f"  Market Making Potential:")
            print(f"    Profit per trade (0.01 BTC): ${profit_per_trade:.4f}")
            print(f"    Estimated trades/hour: {trades_per_hour:.1f}")
            print(f"    Potential hourly profit: ${hourly_profit:.2f}")

except Exception as e:
    print(f"Error in market making analysis: {e}")

# =============================================================================
# 4. HYBRID STRATEGY ANALYSIS
# =============================================================================

print("\n" + "="*60)
print("4. HYBRID STRATEGY: CONVERGENCE + MARKET MAKING")
print("="*60)

print("""
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
""")

# Analyze hedging effectiveness
hedge_query = """
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
        AND datetime >= CURRENT_TIMESTAMP - INTERVAL '30 minutes'
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
"""

try:
    hedge_df = conn.execute(hedge_query).df()
    
    if len(hedge_df) > 0 and hedge_df.iloc[0]['minutes_analyzed'] > 0:
        row = hedge_df.iloc[0]
        
        print(f"\nHEDGING ANALYSIS ({row['minutes_analyzed']:.0f} minutes):")
        print(f"Average spread: ${row['avg_spread']:.2f}")
        print(f"Coinbase liquidity: {row['avg_cb_trades_per_min']:.1f} trades/min")
        print(f"Kraken liquidity: {row['avg_kr_trades_per_min']:.1f} trades/min")
        print(f"Liquidity ratio (CB/KR): {row['avg_cb_trades_per_min']/max(row['avg_kr_trades_per_min'], 1):.1f}x")
        
        # Strategy recommendation
        liquidity_ratio = row['avg_cb_trades_per_min']/max(row['avg_kr_trades_per_min'], 1)
        avg_spread = row['avg_spread']
        
        print(f"\nSTRATEGY RECOMMENDATION:")
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
    print(f"Error in hybrid analysis: {e}")

# =============================================================================
# 5. RISK METRICS & IMPLEMENTATION NOTES
# =============================================================================

print("\n" + "="*60)
print("5. RISK METRICS & IMPLEMENTATION")
print("="*60)

# Volatility analysis
vol_query = """
WITH price_changes AS (
    SELECT
        exchange,
        symbol,
        datetime,
        price,
        LAG(price) OVER (PARTITION BY exchange, symbol ORDER BY datetime) as prev_price
    FROM trades
    WHERE symbol = 'BTC/USD'
        AND datetime >= CURRENT_TIMESTAMP - INTERVAL '30 minutes'
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
"""

try:
    vol_df = conn.execute(vol_query).df()
    
    print("VOLATILITY ANALYSIS:")
    for _, row in vol_df.iterrows():
        exchange = row['exchange'].upper()
        print(f"\n{exchange}:")
        print(f"  Observations: {row['observations']:,}")
        print(f"  Average return: {row['avg_return_pct']:.4f}%")
        print(f"  Volatility (std): {row['volatility_pct']:.4f}%")
        print(f"  Avg absolute move: {row['avg_abs_return_pct']:.4f}%")
        print(f"  Max single move: {row['max_abs_return_pct']:.4f}%")

except Exception as e:
    print(f"Error in volatility analysis: {e}")

print(f"\nIMPLEMENTATION NOTES:")
print(f"1. START SMALL: Begin with 0.01 BTC position sizes")
print(f"2. REAL-TIME MONITORING: WebSocket feeds are critical")  
print(f"3. LATENCY MATTERS: Co-location near exchanges recommended")
print(f"4. FEE STRUCTURE: ")
print(f"   - Coinbase Pro: 0.00%-0.50% maker, 0.04%-0.50% taker")
print(f"   - Kraken: 0.00%-0.16% maker, 0.10%-0.26% taker")
print(f"5. RISK LIMITS: Max 1% of portfolio per strategy")
print(f"6. HEDGING TRIGGERS: Auto-hedge when position > 0.1 BTC")

print(f"\nNEXT STEPS:")
print(f"1. Build real-time spread monitoring dashboard")
print(f"2. Implement paper trading system")
print(f"3. Add order book data (L2) for better spread estimation")
print(f"4. Create automated hedging system")
print(f"5. Backtest strategies on longer time periods")

conn.close()
print(f"\n" + "="*80)
print(f"ANALYSIS COMPLETE - Check results above for strategy recommendations")
print(f"="*80)
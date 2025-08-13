# Trade-Level Analysis Template
# Real analysis of what we can measure with actual streaming trade data

template = {
    "title": "Trade-Level Market Analysis",
    "description": "Analysis based on actual streaming trade execution data from Coinbase and Kraken",
    "cells": [
        {
            "type": "markdown",
            "content": """# Trade-Level Market Analysis

## What We Actually Have:
- **Live trade execution data** from Coinbase and Kraken WebSocket feeds
- **Timestamp, price, size, side** for every trade
- **NO order book data** (bids/asks) - only executed trades

## What We Can Analyze:
1. **Trade patterns**: Frequency, size distribution, side clustering
2. **Cross-exchange price differences**: For convergence trading signals
3. **Volume-based opportunities**: When liquidity is high/low
4. **Price clustering**: Where trades actually happen
5. **Execution timing**: How trades cluster in time

## What We CANNOT Analyze (without order book):
- True bid-ask spreads
- Market depth
- Order queue positions
- Real market making opportunities

Let's work with what we have and do it properly."""
        },
        {
            "type": "code",
            "content": """# Setup and Data Overview
import pandas as pd
import numpy as np
import duckdb
import matplotlib.pyplot as plt
import seaborn as sns
from datetime import datetime, timedelta
import warnings
warnings.filterwarnings('ignore')

plt.style.use('seaborn-v0_8')
sns.set_palette("husl")

print("="*80)
print("TRADE-LEVEL MARKET ANALYSIS")
print("="*80)
print("Analysis based on actual streaming trade data")
print("NO estimates, NO guesswork, NO made-up spreads")"""
        },
        {
            "type": "code",
            "content": """# Connect to our live trade database
conn = duckdb.connect('../backend/market_data/market_data.duckdb', read_only=True)

print("\\n" + "="*60)
print("DATA INVENTORY")  
print("="*60)

# What data do we actually have?
data_inventory = conn.execute('''
SELECT 
    exchange,
    symbol,
    COUNT(*) as total_trades,
    MIN(datetime) as first_trade,
    MAX(datetime) as last_trade,
    ROUND(AVG(price), 2) as avg_price,
    ROUND(SUM(size), 4) as total_volume_btc,
    ROUND(COUNT(*) / EXTRACT(HOUR FROM (MAX(datetime) - MIN(datetime))) * 1.0, 0) as trades_per_hour
FROM trades 
WHERE symbol = 'BTC/USD'
GROUP BY exchange, symbol 
ORDER BY exchange
''').df()

print("ACTUAL STREAMING DATA:")
for _, row in data_inventory.iterrows():
    exchange = row['exchange'].upper()
    print(f"\\n{exchange}:")
    print(f"  • {row['total_trades']:,} trades recorded")
    print(f"  • {row['trades_per_hour']:,} trades/hour average")
    print(f"  • {row['total_volume_btc']:.2f} BTC total volume")
    print(f"  • Data: {row['first_trade']} to {row['last_trade']}")
    
    # Liquidity assessment
    if row['trades_per_hour'] > 5000:
        print(f"  • ✅ HIGH LIQUIDITY: Excellent for execution")
    elif row['trades_per_hour'] > 1000:
        print(f"  • ✅ GOOD LIQUIDITY: Suitable for trading")
    else:
        print(f"  • ⚠️ LOWER LIQUIDITY: Higher impact costs expected")"""
        },
        {
            "type": "code",
            "content": """# Trade Pattern Analysis
print("\\n" + "="*60)
print("TRADE PATTERN ANALYSIS")
print("="*60)

# Analyze trade clustering and gaps for each exchange
for exchange in ['coinbase', 'kraken']:
    print(f"\\n{exchange.upper()} TRADE PATTERNS:")
    
    pattern_query = f'''
    WITH trade_gaps AS (
        SELECT 
            datetime,
            price,
            side,
            size,
            LAG(datetime) OVER (ORDER BY datetime) as prev_time,
            LAG(price) OVER (ORDER BY datetime) as prev_price,
            LAG(side) OVER (ORDER BY datetime) as prev_side,
            EXTRACT(EPOCH FROM (datetime - LAG(datetime) OVER (ORDER BY datetime))) as time_gap_seconds
        FROM trades 
        WHERE exchange = '{exchange}' AND symbol = 'BTC/USD'
            AND datetime >= CURRENT_TIMESTAMP - INTERVAL 4 HOUR
        ORDER BY datetime
    ),
    side_transitions AS (
        SELECT 
            side,
            prev_side,
            COUNT(*) as occurrences,
            AVG(ABS(price - prev_price)) as avg_price_jump,
            AVG(time_gap_seconds) as avg_time_gap
        FROM trade_gaps
        WHERE prev_side IS NOT NULL 
            AND time_gap_seconds < 300  -- Within 5 minutes
            AND ABS(price - prev_price) < 100  -- Filter obvious errors
        GROUP BY side, prev_side
    )
    SELECT * FROM side_transitions
    ORDER BY occurrences DESC
    '''
    
    patterns = conn.execute(pattern_query).df()
    
    for _, row in patterns.iterrows():
        transition = f"{row['prev_side']} → {row['side']}"
        print(f"  {transition}: {row['occurrences']} times")
        print(f"    Avg price jump: ${row['avg_price_jump']:.2f}")
        print(f"    Avg time gap: {row['avg_time_gap']:.1f} seconds")
        
        # Look for same-side clustering vs alternating
        if row['side'] == row['prev_side']:
            print(f"    📊 CLUSTERING: Same-side trades cluster together")
        else:
            print(f"    🔄 ALTERNATING: Side changes indicate spread crossing")"""
        },
        {
            "type": "code", 
            "content": """# Cross-Exchange Price Differences (for convergence signals)
print("\\n" + "="*60)
print("CROSS-EXCHANGE CONVERGENCE ANALYSIS")
print("="*60)

print("IMPORTANT: This is for CONVERGENCE TRADING, not instant arbitrage")
print("Use price differences as directional signals, not execution opportunities")
print()

# Use minute-level aggregation for synchronization
convergence_query = '''
WITH minute_prices AS (
    SELECT
        DATE_TRUNC('minute', datetime) as minute,
        exchange,
        AVG(price) as avg_price,
        COUNT(*) as trade_count,
        SUM(size) as volume
    FROM trades
    WHERE symbol = 'BTC/USD'
        AND datetime >= CURRENT_TIMESTAMP - INTERVAL 6 HOUR
    GROUP BY minute, exchange
    HAVING COUNT(*) >= 1  -- At least 1 trade per minute
),
price_comparison AS (
    SELECT
        cb.minute,
        cb.avg_price as coinbase_price,
        kr.avg_price as kraken_price,
        cb.avg_price - kr.avg_price as price_diff,
        ABS(cb.avg_price - kr.avg_price) as abs_diff,
        cb.trade_count as cb_activity,
        kr.trade_count as kr_activity,
        cb.volume as cb_volume,
        kr.volume as kr_volume
    FROM minute_prices cb
    JOIN minute_prices kr ON cb.minute = kr.minute
    WHERE cb.exchange = 'coinbase' 
        AND kr.exchange = 'kraken'
)
SELECT
    COUNT(*) as synchronized_minutes,
    AVG(abs_diff) as avg_price_diff,
    MIN(abs_diff) as min_diff,
    MAX(abs_diff) as max_diff,
    STDDEV(abs_diff) as diff_volatility,
    AVG(CASE WHEN price_diff > 0 THEN 1 ELSE 0 END) as coinbase_higher_pct,
    AVG(cb_activity) as avg_cb_trades_per_min,
    AVG(kr_activity) as avg_kr_trades_per_min
FROM price_comparison
'''

convergence_stats = conn.execute(convergence_query).df()

if len(convergence_stats) > 0 and convergence_stats.iloc[0]['synchronized_minutes'] > 0:
    stats = convergence_stats.iloc[0]
    
    print(f"SYNCHRONIZED DATA: {stats['synchronized_minutes']} minutes")
    print(f"Average price difference: ${stats['avg_price_diff']:.2f}")
    print(f"Range: ${stats['min_diff']:.2f} - ${stats['max_diff']:.2f}")
    print(f"Volatility: ${stats['diff_volatility']:.2f}")
    print(f"Coinbase higher: {stats['coinbase_higher_pct']*100:.1f}% of time")
    print()
    
    print("CONVERGENCE TRADING SIGNALS:")
    if stats['diff_volatility'] > 20:
        print("✅ HIGH VOLATILITY: Good for convergence betting")
        print("   Strategy: Buy on cheaper exchange when spread > 2σ")
    elif stats['avg_price_diff'] > 10:
        print("✅ PERSISTENT SPREAD: Potential systematic difference")
        print("   Strategy: Long-term position on consistently cheaper exchange")
    else:
        print("⚠️ TIGHT COUPLING: Limited convergence opportunities")
        print("   Strategy: Focus on volatility breakout signals")
        
    # Volume-based signals
    liquidity_ratio = stats['avg_cb_trades_per_min'] / max(stats['avg_kr_trades_per_min'], 1)
    print(f"\\nLIQUIDITY RATIO (CB/KR): {liquidity_ratio:.1f}x")
    if liquidity_ratio > 5:
        print("✅ Use Coinbase for execution, Kraken for signals")
    else:
        print("⚠️ Similar liquidity - execution costs matter more")
else:
    print("❌ NO SYNCHRONIZED DATA: Check data collection")"""
        },
        {
            "type": "code",
            "content": """# Volume-Conditional Trading Opportunities  
print("\\n" + "="*60)
print("VOLUME-CONDITIONAL OPPORTUNITIES")
print("="*60)

print("Analysis: When does trading become more/less profitable?")
print()

# Analyze opportunities by volume conditions
volume_analysis_query = '''
WITH minute_activity AS (
    SELECT 
        DATE_TRUNC('minute', datetime) as minute,
        exchange,
        COUNT(*) as trade_count,
        SUM(size) as volume,
        MAX(price) - MIN(price) as price_range,
        STDDEV(price) as price_volatility
    FROM trades 
    WHERE symbol = 'BTC/USD'
        AND datetime >= CURRENT_TIMESTAMP - INTERVAL 6 HOUR
    GROUP BY minute, exchange
    HAVING COUNT(*) >= 2
),
volume_categories AS (
    SELECT *,
        CASE 
            WHEN trade_count >= 20 THEN 'Very High'
            WHEN trade_count >= 10 THEN 'High'
            WHEN trade_count >= 5 THEN 'Medium'
            ELSE 'Low'
        END as activity_level
    FROM minute_activity
    WHERE price_range > 0  -- Filter periods with actual price movement
)
SELECT 
    exchange,
    activity_level,
    COUNT(*) as minutes,
    AVG(price_range) as avg_price_range,
    AVG(trade_count) as avg_trades_per_min,
    AVG(volume) as avg_volume_btc,
    AVG(price_volatility) as avg_volatility
FROM volume_categories
GROUP BY exchange, activity_level
ORDER BY exchange, avg_trades_per_min DESC
'''

volume_data = conn.execute(volume_analysis_query).df()

for exchange in ['coinbase', 'kraken']:
    exchange_data = volume_data[volume_data['exchange'] == exchange]
    if len(exchange_data) > 0:
        print(f"{exchange.upper()} VOLUME CONDITIONS:")
        
        for _, row in exchange_data.iterrows():
            level = row['activity_level']
            minutes = row['minutes']
            avg_range = row['avg_price_range']
            avg_trades = row['avg_trades_per_min']
            
            print(f"  {level} Activity: {minutes} minutes")
            print(f"    {avg_trades:.1f} trades/min, ${avg_range:.2f} price range")
            
            # Trading recommendations based on activity
            if level == 'Very High' and avg_range > 5:
                print(f"    ✅ EXCELLENT: High volume + volatility = tight spreads")
            elif level == 'High':
                print(f"    ✅ GOOD: Sufficient liquidity for normal position sizes")
            elif level == 'Low' and exchange == 'kraken':
                print(f"    ⚠️ OPPORTUNITY: Less competition, potentially wider spreads")
            else:
                print(f"    ⚠️ CAUTION: Higher impact costs expected")
        print()"""
        },
        {
            "type": "code",
            "content": """# Price Level Analysis (Where trades actually happen)
print("\\n" + "="*60)
print("PRICE LEVEL EXECUTION ANALYSIS")
print("="*60)

print("Where do trades actually execute? Price clustering reveals market structure.")
print()

for exchange in ['coinbase', 'kraken']:
    print(f"{exchange.upper()} PRICE CLUSTERING:")
    
    clustering_query = f'''
    WITH price_levels AS (
        SELECT 
            ROUND(price, 0) as price_level,
            side,
            COUNT(*) as trade_count,
            SUM(size) as total_volume,
            AVG(size) as avg_trade_size,
            MIN(datetime) as first_trade,
            MAX(datetime) as last_trade,
            EXTRACT(EPOCH FROM (MAX(datetime) - MIN(datetime))) / 3600 as hours_active
        FROM trades 
        WHERE exchange = '{exchange}' AND symbol = 'BTC/USD'
            AND datetime >= CURRENT_TIMESTAMP - INTERVAL 4 HOUR
        GROUP BY ROUND(price, 0), side
        HAVING COUNT(*) >= 3  -- At least 3 trades at this level
    ),
    level_analysis AS (
        SELECT 
            price_level,
            SUM(CASE WHEN side = 'buy' THEN trade_count ELSE 0 END) as buy_trades,
            SUM(CASE WHEN side = 'sell' THEN trade_count ELSE 0 END) as sell_trades,
            SUM(trade_count) as total_trades,
            SUM(total_volume) as level_volume,
            AVG(hours_active) as avg_hours_active
        FROM price_levels
        GROUP BY price_level
        HAVING SUM(trade_count) >= 5
    )
    SELECT * FROM level_analysis
    ORDER BY total_trades DESC
    LIMIT 10
    '''
    
    levels = conn.execute(clustering_query).df()
    
    if len(levels) > 0:
        for _, row in levels.iterrows():
            level = row['price_level']
            buy_trades = row['buy_trades']
            sell_trades = row['sell_trades']
            total = row['total_trades']
            volume = row['level_volume']
            
            print(f"  ${level:,.0f}: {total} trades ({buy_trades} buys, {sell_trades} sells)")
            print(f"    Volume: {volume:.4f} BTC")
            
            # Analyze buy/sell balance
            if buy_trades > sell_trades * 1.5:
                print(f"    📈 BUY PRESSURE: Strong demand at this level")
            elif sell_trades > buy_trades * 1.5:
                print(f"    📉 SELL PRESSURE: Strong supply at this level")
            else:
                print(f"    ⚖️ BALANCED: Good two-way flow")
                
        print(f"\\n  KEY INSIGHT: These are the ACTUAL prices where trades execute")
        print(f"  Use for: Position entry/exit, support/resistance levels")
    else:
        print(f"  No significant price clustering found")
    
    print()"""
        },
        {
            "type": "code",
            "content": """# Execution Timing Analysis
print("\\n" + "="*60)
print("EXECUTION TIMING PATTERNS")
print("="*60)

print("When do trades cluster? Timing patterns reveal market microstructure.")
print()

timing_query = '''
WITH trade_intervals AS (
    SELECT 
        exchange,
        datetime,
        price,
        side,
        size,
        LAG(datetime) OVER (PARTITION BY exchange ORDER BY datetime) as prev_time,
        EXTRACT(EPOCH FROM (datetime - LAG(datetime) OVER (PARTITION BY exchange ORDER BY datetime))) as interval_seconds
    FROM trades 
    WHERE symbol = 'BTC/USD'
        AND datetime >= CURRENT_TIMESTAMP - INTERVAL 2 HOUR
    ORDER BY exchange, datetime
),
interval_categories AS (
    SELECT 
        exchange,
        CASE 
            WHEN interval_seconds <= 1 THEN 'Rapid (≤1s)'
            WHEN interval_seconds <= 5 THEN 'Fast (1-5s)'
            WHEN interval_seconds <= 30 THEN 'Normal (5-30s)'
            WHEN interval_seconds <= 300 THEN 'Slow (30s-5m)'
            ELSE 'Very Slow (>5m)'
        END as timing_category,
        COUNT(*) as occurrences,
        AVG(interval_seconds) as avg_interval,
        AVG(size) as avg_trade_size
    FROM trade_intervals
    WHERE interval_seconds IS NOT NULL 
        AND interval_seconds < 3600  -- Filter obvious gaps
    GROUP BY exchange, timing_category
)
SELECT * FROM interval_categories
ORDER BY exchange, avg_interval
'''

timing_data = conn.execute(timing_query).df()

for exchange in ['coinbase', 'kraken']:
    exchange_data = timing_data[timing_data['exchange'] == exchange]
    if len(exchange_data) > 0:
        print(f"{exchange.upper()} TIMING PATTERNS:")
        
        total_trades = exchange_data['occurrences'].sum()
        
        for _, row in timing_data[timing_data['exchange'] == exchange].iterrows():
            category = row['timing_category']
            count = row['occurrences']
            pct = (count / total_trades) * 100
            avg_size = row['avg_trade_size']
            
            print(f"  {category}: {count:,} trades ({pct:.1f}%)")
            print(f"    Average trade size: {avg_size:.6f} BTC")
            
            # Market microstructure insights
            if 'Rapid' in category and pct > 20:
                print(f"    🚀 HIGH FREQUENCY: Algorithmic trading present")
            elif 'Fast' in category and pct > 30:
                print(f"    ⚡ ACTIVE MARKET: Good for execution")
            elif 'Slow' in category and pct > 40:
                print(f"    🐌 QUIET MARKET: Higher impact costs expected")
        print()

print("\\nKEY TAKEAWAY: Timing patterns help optimize execution strategy")
print("- Rapid periods: More competition, tighter spreads")
print("- Slow periods: Less competition, potentially wider spreads")"""
        },
        {
            "type": "code",
            "content": """# Trading Strategy Development
print("\\n" + "="*60)
print("ACTIONABLE TRADING STRATEGIES")
print("="*60)

print("Based on ACTUAL data analysis, here are realistic strategies:")
print()

print("1. 📊 VOLUME-CONDITIONAL EXECUTION:")
print("   - Monitor trades/minute on both exchanges")
print("   - Execute large orders during high-volume periods")
print("   - Avoid execution during low-activity periods on Kraken")
print()

print("2. 🎯 PRICE LEVEL TARGETING:")
print("   - Use identified price clusters for limit orders")
print("   - Place orders at levels with historical two-way flow")
print("   - Avoid levels with strong directional bias")
print()

print("3. ⏰ TIMING OPTIMIZATION:")
print("   - Execute during 'Fast' or 'Normal' timing periods")
print("   - Break up large orders during 'Rapid' periods")
print("   - Use market orders sparingly during 'Slow' periods")
print()

print("4. 🔄 CONVERGENCE SIGNALS:")
print("   - Use cross-exchange price differences as directional signals")
print("   - Not for instant arbitrage, but for predicting price movement")
print("   - Combine with volume analysis for confirmation")
print()

print("5. 💡 MICROSTRUCTURE EXPLOITATION:")
print("   - Trade in direction of recent clustering")
print("   - Fade extreme price differences between exchanges")
print("   - Use Coinbase for execution, Kraken data for signals")
print()

# Summary statistics for strategy parameters
summary_query = '''
SELECT 
    'Cross-Exchange Spread' as metric,
    AVG(ABS(cb.close - kr.close)) as avg_value,
    MIN(ABS(cb.close - kr.close)) as min_value,
    MAX(ABS(cb.close - kr.close)) as max_value
FROM ohlcv cb
JOIN ohlcv kr ON cb.timestamp = kr.timestamp
WHERE cb.exchange = 'coinbase' AND kr.exchange = 'kraken'
    AND cb.symbol = 'BTC/USD' AND kr.symbol = 'BTC/USD'
UNION ALL
SELECT 
    'Kraken Trade Frequency',
    COUNT(*) / EXTRACT(HOUR FROM (MAX(datetime) - MIN(datetime))),
    0,
    COUNT(*)
FROM trades
WHERE exchange = 'kraken' AND symbol = 'BTC/USD'
UNION ALL
SELECT 
    'Coinbase Trade Frequency',
    COUNT(*) / EXTRACT(HOUR FROM (MAX(datetime) - MIN(datetime))),
    0,
    COUNT(*)
FROM trades
WHERE exchange = 'coinbase' AND symbol = 'BTC/USD'
'''

strategy_params = conn.execute(summary_query).df()
print("\\nSTRATEGY PARAMETERS:")
for _, row in strategy_params.iterrows():
    metric = row['metric']
    avg_val = row['avg_value']
    print(f"  {metric}: {avg_val:.2f}")"""
        },
        {
            "type": "code",
            "content": """# Next Steps for Enhanced Analysis
print("\\n" + "="*60)
print("DEVELOPMENT ROADMAP")
print("="*60)

print("To improve this analysis, we need:")
print()

print("📡 DATA COLLECTION ENHANCEMENTS:")
print("1. Add WebSocket order book feeds (Level 2)")
print("   - Coinbase: wss://ws-feed.exchange.coinbase.com (level2 channel)")
print("   - Kraken: wss://ws.kraken.com (book channel)")
print("   - Store bid/ask snapshots and updates")
print()

print("2. Enhanced trade data")
print("   - Market vs limit order classification")  
print("   - Aggressor side identification")
print("   - Trade size categorization (retail vs institutional)")
print()

print("🔧 ANALYSIS IMPROVEMENTS:")
print("3. Order book reconstruction")
print("   - Maintain live bid/ask spreads")
print("   - Calculate market depth")
print("   - Track spread changes over time")
print()

print("4. Real-time signal generation")
print("   - Volume-weighted average prices")
print("   - Momentum indicators from trade flow")
print("   - Cross-exchange basis monitoring")
print()

print("💹 STRATEGY IMPLEMENTATION:")
print("5. Paper trading system")
print("   - Simulate strategy performance")
print("   - Test execution algorithms")
print("   - Measure slippage and impact")
print()

print("6. Risk management framework")
print("   - Position sizing rules")
print("   - Stop-loss mechanisms")
print("   - Exposure monitoring")
print()

# Check current data collection status
print("\\nCURRENT STATUS CHECK:")
try:
    recent_data = conn.execute('''
    SELECT 
        exchange,
        COUNT(*) as trades_last_hour,
        MAX(datetime) as latest_trade
    FROM trades 
    WHERE datetime >= CURRENT_TIMESTAMP - INTERVAL 1 HOUR
        AND symbol = 'BTC/USD'
    GROUP BY exchange
    ''').df()
    
    if len(recent_data) > 0:
        print("✅ Data collection is active:")
        for _, row in recent_data.iterrows():
            print(f"  {row['exchange']}: {row['trades_last_hour']} trades in last hour")
            print(f"    Latest: {row['latest_trade']}")
    else:
        print("❌ No recent data - check WebSocket services")
        
except Exception as e:
    print(f"❌ Database error: {e}")

# Close connection
conn.close()

print(f"\\n" + "="*80)
print(f"ANALYSIS COMPLETE - All results based on actual trade data")
print(f"="*80)"""
        },
        {
            "type": "markdown",
            "content": """## Summary

This analysis is based entirely on **actual streaming trade execution data** with no estimates or guesswork.

### Key Findings:
- **Volume patterns** show when markets are most/least liquid
- **Price clustering** reveals where trades actually execute  
- **Timing analysis** shows market microstructure
- **Cross-exchange differences** provide convergence signals

### Realistic Strategies:
1. **Volume-conditional execution** - Trade when liquidity is high
2. **Price level targeting** - Use historical execution points
3. **Timing optimization** - Execute during active periods
4. **Convergence signals** - Use price differences for direction

### Missing for Market Making:
- Order book data (bids/asks)
- Real-time spread measurements
- Queue position tracking
- Competition analysis

### Next Steps:
1. Add Level 2 order book data collection
2. Implement real-time signal generation
3. Build paper trading system for strategy testing
4. Develop risk management framework

This foundation provides honest analysis of trading opportunities using available data."""
        }
    ]
}
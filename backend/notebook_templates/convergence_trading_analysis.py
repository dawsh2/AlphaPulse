# Cross-Exchange Convergence Trading Analysis
# Analyzes price divergence and convergence patterns between Coinbase and Kraken

template = {
    "title": "Cross-Exchange Convergence Trading Analysis",
    "description": "Analyze price divergence patterns and convergence probabilities between Coinbase and Kraken for profitable trading signals",
    "cells": [
        {
            "type": "markdown",
            "content": """# Cross-Exchange Convergence Trading Analysis

## Strategy Overview:
**Convergence trading** exploits temporary price differences between exchanges, betting that prices will converge back to equilibrium.

### Key Concepts:
1. **Price Divergence**: When Coinbase and Kraken prices drift apart
2. **Convergence Probability**: Likelihood prices will return to normal spread
3. **Threshold Analysis**: At what spread size do convergence odds increase?
4. **Risk Management**: Using hedging and correlation analysis

### Trading Approach:
- **Not arbitrage**: We don't simultaneously buy/sell (too slow, high fees)
- **Directional betting**: When spread > threshold, bet on convergence
- **Statistical edge**: Use historical patterns to predict mean reversion

### Data Requirements:
- Minute-level price data from both exchanges
- Volume and volatility measurements  
- Statistical analysis of convergence patterns"""
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
from scipy import stats
import warnings
warnings.filterwarnings('ignore')

plt.style.use('seaborn-v0_8')
sns.set_palette("husl")

print("="*80)
print("CROSS-EXCHANGE CONVERGENCE TRADING ANALYSIS")
print("="*80)
print("Focus: Statistical analysis of price convergence patterns")
print("Goal: Identify profitable divergence/convergence opportunities")"""
        },
        {
            "type": "code",
            "content": """# Connect and validate data availability
conn = duckdb.connect('../backend/market_data/market_data.duckdb', read_only=True)

print("\\n" + "="*60)
print("DATA VALIDATION & SYNCHRONIZATION")  
print("="*60)

# Check synchronized price data availability
sync_check_query = '''
WITH timezone_normalized AS (
    SELECT 
        exchange,
        symbol,
        price,
        size,
        CASE 
            WHEN exchange = 'kraken' THEN datetime - INTERVAL 7 HOUR  -- Convert UTC to local
            ELSE datetime  -- Coinbase already local
        END as normalized_datetime
    FROM trades 
    WHERE symbol = 'BTC/USD'
        AND datetime >= CURRENT_TIMESTAMP - INTERVAL 24 HOUR
),
minute_data AS (
    SELECT 
        DATE_TRUNC('minute', normalized_datetime) as minute,
        exchange,
        COUNT(*) as trades,
        AVG(price) as avg_price,
        SUM(size) as volume
    FROM timezone_normalized
    GROUP BY minute, exchange
    HAVING COUNT(*) >= 1
),
sync_periods AS (
    SELECT 
        cb.minute,
        cb.avg_price as coinbase_price,
        kr.avg_price as kraken_price,
        cb.volume as cb_volume,
        kr.volume as kr_volume,
        cb.trades as cb_trades,
        kr.trades as kr_trades
    FROM minute_data cb
    JOIN minute_data kr ON cb.minute = kr.minute
    WHERE cb.exchange = 'coinbase' AND kr.exchange = 'kraken'
)
SELECT 
    COUNT(*) as synchronized_minutes,
    MIN(minute) as first_sync_time,
    MAX(minute) as last_sync_time,
    AVG(coinbase_price) as avg_cb_price,
    AVG(kraken_price) as avg_kr_price,
    AVG(cb_volume) as avg_cb_volume,
    AVG(kr_volume) as avg_kr_volume
FROM sync_periods
'''

sync_data = conn.execute(sync_check_query).df()

if len(sync_data) > 0 and sync_data.iloc[0]['synchronized_minutes'] > 0:
    sync_stats = sync_data.iloc[0]
    
    print(f"✅ DATA QUALITY CHECK:")
    print(f"Synchronized periods: {sync_stats['synchronized_minutes']:,} minutes")
    print(f"Time range: {sync_stats['first_sync_time']} to {sync_stats['last_sync_time']}")
    print(f"Coinbase avg price: ${sync_stats['avg_cb_price']:,.2f}")
    print(f"Kraken avg price: ${sync_stats['avg_kr_price']:,.2f}")
    print(f"Average volumes: CB {sync_stats['avg_cb_volume']:.4f}, KR {sync_stats['avg_kr_volume']:.4f} BTC/min")
    
    hours_coverage = sync_stats['synchronized_minutes'] / 60
    print(f"Data coverage: {hours_coverage:.1f} hours")
    
    if sync_stats['synchronized_minutes'] >= 100:
        print("✅ Sufficient data for convergence analysis")
    else:
        print("⚠️ Limited data - results may be less reliable")
else:
    print("❌ No synchronized data found - check data collection")"""
        },
        {
            "type": "code",
            "content": """# Price Divergence Analysis
print("\\n" + "="*60)
print("PRICE DIVERGENCE PATTERNS")
print("="*60)

# Calculate price differences and spreads over time
divergence_query = '''
WITH timezone_normalized AS (
    SELECT 
        exchange,
        symbol,
        price,
        size,
        CASE 
            WHEN exchange = 'kraken' THEN datetime - INTERVAL 7 HOUR
            ELSE datetime
        END as normalized_datetime
    FROM trades 
    WHERE symbol = 'BTC/USD'
        AND datetime >= CURRENT_TIMESTAMP - INTERVAL 12 HOUR
),
minute_prices AS (
    SELECT 
        DATE_TRUNC('minute', normalized_datetime) as minute,
        exchange,
        AVG(price) as avg_price,
        COUNT(*) as trades,
        SUM(size) as volume,
        STDDEV(price) as price_volatility
    FROM timezone_normalized
    GROUP BY minute, exchange
    HAVING COUNT(*) >= 1
),
price_spreads AS (
    SELECT 
        cb.minute,
        cb.avg_price as coinbase_price,
        kr.avg_price as kraken_price,
        cb.avg_price - kr.avg_price as price_diff,
        ABS(cb.avg_price - kr.avg_price) as abs_spread,
        (cb.avg_price - kr.avg_price) / ((cb.avg_price + kr.avg_price) / 2) * 100 as spread_pct,
        cb.trades as cb_activity,
        kr.trades as kr_activity,
        cb.volume as cb_volume,
        kr.volume as kr_volume,
        COALESCE(cb.price_volatility, 0) as cb_volatility,
        COALESCE(kr.price_volatility, 0) as kr_volatility
    FROM minute_prices cb
    JOIN minute_prices kr ON cb.minute = kr.minute
    WHERE cb.exchange = 'coinbase' AND kr.exchange = 'kraken'
    ORDER BY cb.minute DESC
)
SELECT * FROM price_spreads
LIMIT 500
'''

spreads_df = conn.execute(divergence_query).df()

if len(spreads_df) > 0:
    print(f"SPREAD DISTRIBUTION ANALYSIS ({len(spreads_df)} periods):")
    
    # Basic spread statistics
    print(f"\\nBASIC STATISTICS:")
    print(f"Mean absolute spread: ${spreads_df['abs_spread'].mean():.2f}")
    print(f"Median absolute spread: ${spreads_df['abs_spread'].median():.2f}")
    print(f"Std deviation: ${spreads_df['abs_spread'].std():.2f}")
    print(f"95th percentile: ${spreads_df['abs_spread'].quantile(0.95):.2f}")
    print(f"99th percentile: ${spreads_df['abs_spread'].quantile(0.99):.2f}")
    
    # Directional bias analysis
    cb_higher = (spreads_df['price_diff'] > 0).sum()
    kr_higher = (spreads_df['price_diff'] < 0).sum()
    equal = (spreads_df['price_diff'] == 0).sum()
    
    total_periods = len(spreads_df)
    print(f"\\nDIRECTIONAL BIAS:")
    print(f"Coinbase higher: {cb_higher} periods ({cb_higher/total_periods*100:.1f}%)")
    print(f"Kraken higher: {kr_higher} periods ({kr_higher/total_periods*100:.1f}%)")
    print(f"Equal prices: {equal} periods ({equal/total_periods*100:.1f}%)")
    
    # Spread magnitude categories
    print(f"\\nSPREAD MAGNITUDE DISTRIBUTION:")
    spread_categories = [
        ("Tight (<$5)", (spreads_df['abs_spread'] < 5).sum()),
        ("Small ($5-$15)", ((spreads_df['abs_spread'] >= 5) & (spreads_df['abs_spread'] < 15)).sum()),
        ("Medium ($15-$30)", ((spreads_df['abs_spread'] >= 15) & (spreads_df['abs_spread'] < 30)).sum()),
        ("Large ($30-$50)", ((spreads_df['abs_spread'] >= 30) & (spreads_df['abs_spread'] < 50)).sum()),
        ("Very Large (>$50)", (spreads_df['abs_spread'] >= 50).sum())
    ]
    
    for category, count in spread_categories:
        pct = (count / total_periods) * 100
        print(f"  {category}: {count} periods ({pct:.1f}%)")
    
    # Store for later analysis
    globals()['spreads_data'] = spreads_df
    
else:
    print("❌ No spread data available")
    globals()['spreads_data'] = None"""
        },
        {
            "type": "code",
            "content": """# Convergence Probability Analysis
print("\\n" + "="*60)
print("CONVERGENCE PROBABILITY BY SPREAD SIZE")
print("="*60)

if 'spreads_data' in globals() and spreads_data is not None:
    
    # Define spread thresholds for analysis
    thresholds = [5, 10, 15, 20, 30, 50]
    
    print("Analysis: Do larger spreads have higher convergence probability?")
    print()
    
    convergence_results = []
    
    for threshold in thresholds:
        # Find periods where spread exceeded threshold
        large_spreads = spreads_data[spreads_data['abs_spread'] >= threshold].copy()
        
        if len(large_spreads) == 0:
            continue
            
        # Look ahead to see convergence within next 1-6 periods
        convergence_windows = [1, 2, 3, 6]  # 1, 2, 3, 6 minutes ahead
        
        for window in convergence_windows:
            convergences = 0
            total_opportunities = 0
            
            for idx, row in large_spreads.iterrows():
                current_spread = row['abs_spread']
                current_time = row['minute']
                
                # Look for convergence in next 'window' minutes
                future_data = spreads_data[
                    (spreads_data['minute'] > current_time) & 
                    (spreads_data['minute'] <= current_time + pd.Timedelta(minutes=window))
                ]
                
                if len(future_data) > 0:
                    total_opportunities += 1
                    # Convergence = spread reduces by at least 50%
                    min_future_spread = future_data['abs_spread'].min()
                    if min_future_spread <= current_spread * 0.5:
                        convergences += 1
            
            if total_opportunities > 0:
                convergence_rate = (convergences / total_opportunities) * 100
                convergence_results.append({
                    'threshold': threshold,
                    'window': window,
                    'opportunities': total_opportunities,
                    'convergences': convergences,
                    'rate': convergence_rate
                })
    
    # Display convergence analysis
    if convergence_results:
        print("CONVERGENCE PROBABILITY TABLE:")
        print("Threshold  | Window | Opportunities | Convergences | Rate")
        print("-" * 55)
        
        for result in convergence_results:
            print(f"${result['threshold']:>8} | {result['window']:>6}min | {result['opportunities']:>12} | {result['convergences']:>11} | {result['rate']:>5.1f}%")
        
        # Find optimal thresholds
        print(f"\\nOPTIMAL CONVERGENCE THRESHOLDS:")
        
        # Group by threshold to find best performing
        threshold_performance = {}
        for result in convergence_results:
            thresh = result['threshold']
            if thresh not in threshold_performance:
                threshold_performance[thresh] = []
            threshold_performance[thresh].append(result)
        
        for thresh in sorted(threshold_performance.keys()):
            results = threshold_performance[thresh]
            avg_rate = np.mean([r['rate'] for r in results])
            total_ops = sum([r['opportunities'] for r in results])
            
            print(f"${thresh} threshold: {avg_rate:.1f}% avg convergence rate ({total_ops} total opportunities)")
            
            if avg_rate > 60:
                print(f"  ✅ HIGH PROBABILITY: Good convergence signal")
            elif avg_rate > 40:
                print(f"  ⚡ MODERATE: Reasonable signal strength")  
            else:
                print(f"  ⚠️ LOW: Weak convergence signal")
        
        globals()['convergence_results'] = convergence_results
    else:
        print("❌ Insufficient data for convergence analysis")
        
else:
    print("❌ No spread data available for convergence analysis")"""
        },
        {
            "type": "code",
            "content": """# Volume and Volatility Correlation
print("\\n" + "="*60)
print("VOLUME & VOLATILITY IMPACT ON CONVERGENCE")
print("="*60)

if 'spreads_data' in globals() and spreads_data is not None:
    
    print("Analysis: How do volume and volatility affect convergence patterns?")
    print()
    
    # Categorize periods by volume and volatility
    spreads_with_categories = spreads_data.copy()
    
    # Volume categories (based on combined volume)
    spreads_with_categories['total_volume'] = spreads_with_categories['cb_volume'] + spreads_with_categories['kr_volume']
    volume_75th = spreads_with_categories['total_volume'].quantile(0.75)
    volume_25th = spreads_with_categories['total_volume'].quantile(0.25)
    
    spreads_with_categories['volume_category'] = pd.cut(
        spreads_with_categories['total_volume'],
        bins=[0, volume_25th, volume_75th, float('inf')],
        labels=['Low Volume', 'Medium Volume', 'High Volume']
    )
    
    # Volatility categories (based on combined volatility)
    spreads_with_categories['total_volatility'] = spreads_with_categories['cb_volatility'] + spreads_with_categories['kr_volatility']
    vol_75th = spreads_with_categories['total_volatility'].quantile(0.75)
    vol_25th = spreads_with_categories['total_volatility'].quantile(0.25)
    
    spreads_with_categories['volatility_category'] = pd.cut(
        spreads_with_categories['total_volatility'],
        bins=[0, vol_25th, vol_75th, float('inf')],
        labels=['Low Volatility', 'Medium Volatility', 'High Volatility']
    )
    
    # Analyze convergence by conditions
    print("CONVERGENCE RATES BY MARKET CONDITIONS:")
    print()
    
    # Volume impact
    print("📊 VOLUME IMPACT:")
    for vol_cat in ['Low Volume', 'Medium Volume', 'High Volume']:
        vol_data = spreads_with_categories[spreads_with_categories['volume_category'] == vol_cat]
        
        if len(vol_data) > 0:
            # Look at large spreads (>$15) in this volume category
            large_spreads_vol = vol_data[vol_data['abs_spread'] >= 15]
            
            avg_spread = vol_data['abs_spread'].mean()
            large_spread_pct = (len(large_spreads_vol) / len(vol_data)) * 100
            
            print(f"  {vol_cat}: {len(vol_data)} periods")
            print(f"    Average spread: ${avg_spread:.2f}")
            print(f"    Large spreads (>$15): {large_spread_pct:.1f}%")
            
            # Simple convergence analysis: how often do large spreads shrink?
            if len(large_spreads_vol) >= 5:
                print(f"    ✅ Sufficient data for convergence analysis")
            else:
                print(f"    ⚠️ Limited large spread events")
    
    print(f"\\n⚡ VOLATILITY IMPACT:")
    for vol_cat in ['Low Volatility', 'Medium Volatility', 'High Volatility']:
        vol_data = spreads_with_categories[spreads_with_categories['volatility_category'] == vol_cat]
        
        if len(vol_data) > 0:
            large_spreads_vol = vol_data[vol_data['abs_spread'] >= 15]
            
            avg_spread = vol_data['abs_spread'].mean()
            large_spread_pct = (len(large_spreads_vol) / len(vol_data)) * 100
            max_spread = vol_data['abs_spread'].max()
            
            print(f"  {vol_cat}: {len(vol_data)} periods")
            print(f"    Average spread: ${avg_spread:.2f}")
            print(f"    Max spread: ${max_spread:.2f}")
            print(f"    Large spreads (>$15): {large_spread_pct:.1f}%")
            
            if vol_cat == 'High Volatility' and large_spread_pct > 20:
                print(f"    🚀 HIGH OPPORTUNITY PERIODS: Volatility creates divergence")
            elif large_spread_pct > 10:
                print(f"    ✅ MODERATE OPPORTUNITIES: Some divergence events")
            else:
                print(f"    📊 STABLE: Limited divergence opportunities")
    
    # Combined analysis
    print(f"\\n🎯 OPTIMAL TRADING CONDITIONS:")
    
    # Find best combination of volume/volatility for large spreads
    best_conditions = spreads_with_categories[
        (spreads_with_categories['abs_spread'] >= 20)
    ].copy()
    
    if len(best_conditions) > 0:
        print(f"Periods with spreads ≥$20: {len(best_conditions)}")
        
        # Most common conditions for large spreads
        vol_dist = best_conditions['volume_category'].value_counts()
        vol_dist_name = vol_dist.index[0] if len(vol_dist) > 0 else 'Unknown'
        
        volatility_dist = best_conditions['volatility_category'].value_counts()
        volatility_dist_name = volatility_dist.index[0] if len(volatility_dist) > 0 else 'Unknown'
        
        print(f"Most common volume condition: {vol_dist_name}")
        print(f"Most common volatility condition: {volatility_dist_name}")
        
        avg_cb_premium = best_conditions['price_diff'].mean()
        if avg_cb_premium > 0:
            print(f"Direction bias: Coinbase typically ${avg_cb_premium:.2f} higher during large spreads")
        else:
            print(f"Direction bias: Kraken typically ${abs(avg_cb_premium):.2f} higher during large spreads")
    
    globals()['categorized_spreads'] = spreads_with_categories
    
else:
    print("❌ No data available for volume/volatility analysis")"""
        },
        {
            "type": "code",
            "content": """# Hedging Analysis with Coinbase
print("\\n" + "="*60)
print("HEDGING EFFECTIVENESS ANALYSIS")
print("="*60)

if 'spreads_data' in globals() and spreads_data is not None:
    
    print("Analysis: Can we hedge convergence trades using Coinbase?")
    print("Strategy: Long/Short Kraken + hedge on Coinbase")
    print()
    
    # Hedging scenarios analysis
    hedging_scenarios = []
    
    # Find significant divergence periods (spread >= $20)
    large_divergences = spreads_data[spreads_data['abs_spread'] >= 20].copy()
    
    if len(large_divergences) > 0:
        print(f"HEDGING SIMULATION ({len(large_divergences)} large divergence events):")
        print()
        
        for idx, row in large_divergences.head(10).iterrows():  # Analyze first 10 events
            current_spread = row['abs_spread']
            cb_price = row['coinbase_price']
            kr_price = row['kraken_price']
            price_diff = row['price_diff']  # CB - KR
            current_time = row['minute']
            
            # Determine trade direction
            if price_diff > 0:  # Coinbase higher
                trade_direction = "Short Coinbase, Long Kraken"
                entry_signal = f"CB premium of ${price_diff:.2f}"
            else:  # Kraken higher  
                trade_direction = "Long Coinbase, Short Kraken"
                entry_signal = f"KR premium of ${abs(price_diff):.2f}"
            
            # Look ahead 5 minutes to see outcome
            future_data = spreads_data[
                (spreads_data['minute'] > current_time) & 
                (spreads_data['minute'] <= current_time + pd.Timedelta(minutes=5))
            ]
            
            if len(future_data) > 0:
                final_spread = future_data['abs_spread'].iloc[-1]
                final_cb = future_data['coinbase_price'].iloc[-1]
                final_kr = future_data['kraken_price'].iloc[-1]
                
                # Calculate P&L
                cb_price_change = final_cb - cb_price
                kr_price_change = final_kr - kr_price
                spread_change = final_spread - current_spread
                
                if price_diff > 0:  # Was short CB, long KR
                    pnl = -cb_price_change + kr_price_change
                else:  # Was long CB, short KR
                    pnl = cb_price_change - kr_price_change
                
                # Convergence success
                convergence_success = final_spread < current_spread * 0.7  # 30% reduction
                
                hedging_scenarios.append({
                    'time': current_time,
                    'entry_spread': current_spread,
                    'exit_spread': final_spread,
                    'direction': trade_direction,
                    'pnl': pnl,
                    'convergence': convergence_success,
                    'spread_change': spread_change
                })
        
        # Analyze hedging results
        if hedging_scenarios:
            successful_trades = [s for s in hedging_scenarios if s['pnl'] > 0]
            convergent_trades = [s for s in hedging_scenarios if s['convergence']]
            
            total_pnl = sum([s['pnl'] for s in hedging_scenarios])
            avg_pnl = total_pnl / len(hedging_scenarios)
            win_rate = (len(successful_trades) / len(hedging_scenarios)) * 100
            convergence_rate = (len(convergent_trades) / len(hedging_scenarios)) * 100
            
            print(f"HEDGING RESULTS:")
            print(f"Total trades analyzed: {len(hedging_scenarios)}")
            print(f"Profitable trades: {len(successful_trades)} ({win_rate:.1f}%)")
            print(f"Convergence events: {len(convergent_trades)} ({convergence_rate:.1f}%)")
            print(f"Average P&L per trade: ${avg_pnl:.2f}")
            print(f"Total P&L: ${total_pnl:.2f}")
            
            if win_rate > 60:
                print(f"✅ POSITIVE EXPECTANCY: Hedging strategy shows promise")
            elif win_rate > 40:
                print(f"⚡ MIXED RESULTS: Strategy needs refinement")
            else:
                print(f"❌ NEGATIVE EXPECTANCY: Current approach unprofitable")
            
            # Best performing conditions
            profitable_scenarios = [s for s in hedging_scenarios if s['pnl'] > 0]
            if profitable_scenarios:
                avg_entry_spread = np.mean([s['entry_spread'] for s in profitable_scenarios])
                print(f"\\nBest entry spread: ${avg_entry_spread:.2f} average for profitable trades")
                
                long_cb_profits = [s for s in profitable_scenarios if 'Long Coinbase' in s['direction']]
                short_cb_profits = [s for s in profitable_scenarios if 'Short Coinbase' in s['direction']]
                
                print(f"Long Coinbase bias: {len(long_cb_profits)} profitable")
                print(f"Short Coinbase bias: {len(short_cb_profits)} profitable")
        
        globals()['hedging_results'] = hedging_scenarios
        
    else:
        print("❌ No large divergence events found for hedging analysis")
        
else:
    print("❌ No data available for hedging analysis")"""
        },
        {
            "type": "code",
            "content": """# Trend and Momentum Correlation
print("\\n" + "="*60)
print("TREND & MOMENTUM CORRELATION ANALYSIS")
print("="*60)

if 'spreads_data' in globals() and spreads_data is not None:
    
    print("Analysis: How do price trends affect convergence patterns?")
    print()
    
    # Calculate price momentum and trends
    spreads_analysis = spreads_data.copy()
    spreads_analysis = spreads_analysis.sort_values('minute')
    
    # 5-minute price momentum
    spreads_analysis['cb_momentum_5m'] = spreads_analysis['coinbase_price'].pct_change(5) * 100
    spreads_analysis['kr_momentum_5m'] = spreads_analysis['kraken_price'].pct_change(5) * 100
    
    # 15-minute price momentum 
    spreads_analysis['cb_momentum_15m'] = spreads_analysis['coinbase_price'].pct_change(15) * 100
    spreads_analysis['kr_momentum_15m'] = spreads_analysis['kraken_price'].pct_change(15) * 100
    
    # Trend strength (rolling standard deviation)
    spreads_analysis['cb_trend_strength'] = spreads_analysis['coinbase_price'].rolling(10).std()
    spreads_analysis['kr_trend_strength'] = spreads_analysis['kraken_price'].rolling(10).std()
    
    # Remove NaN values
    spreads_analysis = spreads_analysis.dropna()
    
    if len(spreads_analysis) > 50:
        print(f"MOMENTUM & TREND ANALYSIS ({len(spreads_analysis)} periods):")
        
        # Categorize by momentum
        momentum_threshold = 0.1  # 0.1% threshold
        
        # Coinbase momentum categories
        cb_strong_up = spreads_analysis['cb_momentum_5m'] > momentum_threshold
        cb_strong_down = spreads_analysis['cb_momentum_5m'] < -momentum_threshold
        cb_sideways = (spreads_analysis['cb_momentum_5m'].abs() <= momentum_threshold)
        
        print(f"\\n📈 COINBASE MOMENTUM IMPACT:")
        
        # Analyze spreads during different momentum periods
        for condition, name in [(cb_strong_up, 'Strong Up'), (cb_strong_down, 'Strong Down'), (cb_sideways, 'Sideways')]:
            subset = spreads_analysis[condition]
            if len(subset) > 0:
                avg_spread = subset['abs_spread'].mean()
                large_spreads = (subset['abs_spread'] >= 15).sum()
                large_spread_pct = (large_spreads / len(subset)) * 100
                
                print(f"  {name} trend: {len(subset)} periods")
                print(f"    Average spread: ${avg_spread:.2f}")
                print(f"    Large spreads (≥$15): {large_spread_pct:.1f}%")
                
                if large_spread_pct > 15:
                    print(f"    🎯 HIGH DIVERGENCE: Trending creates opportunities")
                elif large_spread_pct > 5:
                    print(f"    ⚡ MODERATE: Some divergence during trends")
                else:
                    print(f"    📊 STABLE: Limited divergence")
        
        # Cross-exchange momentum divergence
        print(f"\\n🔄 MOMENTUM DIVERGENCE ANALYSIS:")
        
        # When momentum differs significantly between exchanges
        momentum_diff = spreads_analysis['cb_momentum_5m'] - spreads_analysis['kr_momentum_5m']
        
        large_momentum_diff = spreads_analysis[momentum_diff.abs() > 0.2]  # >0.2% momentum difference
        
        if len(large_momentum_diff) > 0:
            avg_spread_during_divergence = large_momentum_diff['abs_spread'].mean()
            avg_normal_spread = spreads_analysis[momentum_diff.abs() <= 0.2]['abs_spread'].mean()
            
            print(f"Periods with momentum divergence: {len(large_momentum_diff)}")
            print(f"Average spread during momentum divergence: ${avg_spread_during_divergence:.2f}")
            print(f"Average spread during momentum alignment: ${avg_normal_spread:.2f}")
            
            if avg_spread_during_divergence > avg_normal_spread * 1.5:
                print(f"✅ MOMENTUM DIVERGENCE SIGNAL: Creates wider spreads")
            else:
                print(f"⚠️ LIMITED IMPACT: Momentum divergence doesn't significantly affect spreads")
        
        # Correlation analysis
        print(f"\\n📊 CORRELATION ANALYSIS:")
        
        # Correlation between spread size and momentum
        spread_momentum_corr = spreads_analysis['abs_spread'].corr(spreads_analysis['cb_momentum_5m'].abs())
        spread_volatility_corr = spreads_analysis['abs_spread'].corr(spreads_analysis['cb_trend_strength'])
        
        print(f"Spread vs CB momentum correlation: {spread_momentum_corr:.3f}")
        print(f"Spread vs CB trend strength correlation: {spread_volatility_corr:.3f}")
        
        if abs(spread_momentum_corr) > 0.3:
            print(f"✅ STRONG MOMENTUM RELATIONSHIP: Use momentum for timing")
        elif abs(spread_momentum_corr) > 0.1:
            print(f"⚡ WEAK MOMENTUM RELATIONSHIP: Some predictive value")
        else:
            print(f"❌ NO MOMENTUM RELATIONSHIP: Momentum not predictive")
            
        # Volume momentum analysis
        if 'total_volume' in spreads_analysis.columns:
            spreads_analysis['volume_momentum'] = spreads_analysis['total_volume'].pct_change(3) * 100
            volume_spread_corr = spreads_analysis['abs_spread'].corr(spreads_analysis['volume_momentum'])
            
            print(f"Spread vs volume momentum correlation: {volume_spread_corr:.3f}")
            
            if abs(volume_spread_corr) > 0.2:
                print(f"✅ VOLUME SIGNAL: Volume changes predict spreads")
            else:
                print(f"📊 WEAK VOLUME SIGNAL: Limited predictive value")
        
        globals()['trend_analysis'] = spreads_analysis
        
    else:
        print("❌ Insufficient data for trend analysis")
        
else:
    print("❌ No data available for trend analysis")"""
        },
        {
            "type": "code",
            "content": """# Trading Strategy Formulation
print("\\n" + "="*60)
print("CONVERGENCE TRADING STRATEGY")
print("="*60)

print("Based on analysis, here's the optimal convergence trading approach:")
print()

# Compile recommendations from all analyses
strategy_recommendations = []

# Check if we have analysis results
if 'convergence_results' in globals() and convergence_results:
    # Find best threshold
    best_threshold_data = {}
    for result in convergence_results:
        thresh = result['threshold']
        if thresh not in best_threshold_data or result['rate'] > best_threshold_data[thresh]['rate']:
            best_threshold_data[thresh] = result
    
    best_threshold = max(best_threshold_data.keys(), key=lambda x: best_threshold_data[x]['rate'])
    best_rate = best_threshold_data[best_threshold]['rate']
    
    strategy_recommendations.append(f"ENTRY THRESHOLD: ${best_threshold} spread ({best_rate:.1f}% convergence rate)")

if 'hedging_results' in globals() and hedging_results:
    profitable_hedges = [h for h in hedging_results if h['pnl'] > 0]
    hedge_win_rate = (len(profitable_hedges) / len(hedging_results)) * 100
    
    if hedge_win_rate > 50:
        strategy_recommendations.append(f"HEDGING: Recommended ({hedge_win_rate:.1f}% win rate)")
    else:
        strategy_recommendations.append(f"HEDGING: Not recommended ({hedge_win_rate:.1f}% win rate)")

if 'categorized_spreads' in globals():
    # Volume recommendations
    high_vol_data = categorized_spreads[categorized_spreads['volume_category'] == 'High Volume']
    if len(high_vol_data) > 0:
        avg_spread_high_vol = high_vol_data['abs_spread'].mean()
        strategy_recommendations.append(f"VOLUME FILTER: Trade during high volume (avg spread ${avg_spread_high_vol:.2f})")

print("🎯 CONVERGENCE TRADING STRATEGY:")
for i, rec in enumerate(strategy_recommendations, 1):
    print(f"{i}. {rec}")

print(f"\\n📋 COMPLETE TRADING RULES:")
print(f"")
print(f"ENTRY CONDITIONS:")
print(f"• Spread ≥ ${best_threshold if 'convergence_results' in globals() and convergence_results else 20}")
print(f"• High volume period (if possible)")
print(f"• Clear directional bias (CB premium or KR premium)")
print(f"")
print(f"POSITION SIZING:")
print(f"• Risk 1-2% of portfolio per trade")
print(f"• Position size = Risk Amount / Expected Spread")
print(f"• Example: $1000 risk ÷ $30 spread = 33 units")
print(f"")
print(f"EXECUTION:")
print(f"• Primary: Take position on cheaper exchange")
print(f"• Hedge: Optional opposite position on expensive exchange")
print(f"• Timing: Enter during momentum divergence")
print(f"")
print(f"EXIT CONDITIONS:")
print(f"• Target: 50-70% spread reduction")
print(f"• Stop: Spread increases 100% from entry")
print(f"• Time: Exit after 5-10 minutes maximum")
print(f"")
print(f"RISK MANAGEMENT:")
print(f"• Maximum 3 concurrent positions")
print(f"• Daily loss limit: 5% of trading capital")
print(f"• Review strategy weekly based on performance")

# Generate summary statistics
if 'spreads_data' in globals() and spreads_data is not None:
    total_periods = len(spreads_data)
    large_spread_periods = (spreads_data['abs_spread'] >= 20).sum()
    opportunity_rate = (large_spread_periods / total_periods) * 100
    
    print(f"\\n📊 OPPORTUNITY FREQUENCY:")
    print(f"Analysis period: {total_periods} minutes")
    print(f"Trading opportunities (≥$20 spread): {large_spread_periods}")
    print(f"Opportunity rate: {opportunity_rate:.1f}% of time")
    
    if opportunity_rate > 10:
        print(f"✅ HIGH FREQUENCY: Plenty of trading opportunities")
    elif opportunity_rate > 5:
        print(f"⚡ MODERATE FREQUENCY: Regular opportunities available")
    else:
        print(f"⚠️ LOW FREQUENCY: Limited opportunities - be selective")

# Close database connection
conn.close()

print(f"\\n" + "="*80)
print(f"CONVERGENCE TRADING ANALYSIS COMPLETE")
print(f"="*80)"""
        },
        {
            "type": "markdown",
            "content": """## Summary

This analysis provides a complete framework for **cross-exchange convergence trading** based on actual market data.

### Key Findings:
- **Spread Thresholds**: Optimal entry points for convergence trades
- **Convergence Probabilities**: Statistical likelihood of mean reversion
- **Volume/Volatility Impact**: How market conditions affect opportunities
- **Hedging Effectiveness**: Whether cross-exchange hedging improves results
- **Momentum Correlation**: How trends affect divergence patterns

### Trading Strategy:
1. **Monitor spread size** - Enter when spreads exceed historical thresholds
2. **Time entries** - Use volume and momentum indicators for optimal timing
3. **Manage risk** - Use appropriate position sizing and stop losses
4. **Consider hedging** - If analysis shows positive expectancy

### Risk Considerations:
- **Not arbitrage** - Prices may not converge as expected
- **Execution risk** - Slippage and timing matter
- **Market regime changes** - Strategy effectiveness may vary
- **Capital requirements** - Need sufficient margin for positions

### Next Steps:
1. **Paper trade** the strategy to validate findings
2. **Monitor performance** and adjust thresholds
3. **Add real-time alerts** for optimal entry conditions
4. **Refine** based on actual trading results"""
        }
    ]
}
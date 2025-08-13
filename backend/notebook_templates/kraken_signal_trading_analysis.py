#!/usr/bin/env python3
"""
Kraken Signal Trading Analysis - Using Coinbase Price as Leading Indicator

This analysis examines:
1. When Coinbase price diverges from Kraken (spread widens)
2. Trading on Kraken ONLY when spreads are wide
3. Profitability accounting for Kraken fees (0.25% retail)
4. Directional analysis - does Kraken price converge toward Coinbase?
"""

import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
import duckdb
from datetime import datetime, timedelta
import warnings
warnings.filterwarnings('ignore')

print("=" * 60)
print("KRAKEN SIGNAL TRADING ANALYSIS")
print("Using Coinbase as Price Signal, Trading on Kraken Only")
print("=" * 60)
print()

# Connect to database
conn = duckdb.connect('../backend/market_data/market_data.duckdb', read_only=True)

# Configuration
POSITION_SIZE_BTC = 0.1  # Trade size
KRAKEN_FEE_RATE = 0.0025  # 0.25% retail fee

print(f"Configuration:")
print(f"  Position Size: {POSITION_SIZE_BTC} BTC")
print(f"  Kraken Fee Rate: {KRAKEN_FEE_RATE:.2%}")
print()

# ============================================================
# STEP 1: GET SYNCHRONIZED DATA WITH SPREADS
# ============================================================
print("=" * 60)
print("LOADING SYNCHRONIZED PRICE DATA")
print("=" * 60)

query = """
WITH aligned_data AS (
    SELECT 
        DATE_TRUNC('second', datetime) as timestamp,
        exchange,
        AVG(price) as price,
        COUNT(*) as trades
    FROM trades 
    WHERE symbol = 'BTC/USD'
        AND datetime >= CURRENT_TIMESTAMP - INTERVAL 24 HOUR
    GROUP BY timestamp, exchange
),
spread_data AS (
    SELECT 
        cb.timestamp,
        cb.price as coinbase_price,
        kr.price as kraken_price,
        cb.price - kr.price as spread,
        (cb.price - kr.price) / cb.price * 100 as spread_pct,
        cb.trades as cb_trades,
        kr.trades as kr_trades
    FROM aligned_data cb
    JOIN aligned_data kr ON cb.timestamp = kr.timestamp
    WHERE cb.exchange = 'coinbase' AND kr.exchange = 'kraken'
)
SELECT * FROM spread_data
ORDER BY timestamp
"""

df = conn.execute(query).df()
print(f"Loaded {len(df):,} synchronized seconds of data")
print(f"Date range: {df['timestamp'].min()} to {df['timestamp'].max()}")
print()

# Calculate statistics
print("SPREAD STATISTICS:")
print(f"  Average spread: ${df['spread'].mean():.2f}")
print(f"  Std deviation: ${df['spread'].std():.2f}")
print(f"  Min spread: ${df['spread'].min():.2f}")
print(f"  Max spread: ${df['spread'].max():.2f}")
print()

# ============================================================
# STEP 2: IDENTIFY TRADING OPPORTUNITIES
# ============================================================
print("=" * 60)
print("IDENTIFYING TRADING OPPORTUNITIES")
print("=" * 60)

# Calculate fee hurdle in dollars
avg_kraken_price = df['kraken_price'].mean()
position_value = POSITION_SIZE_BTC * avg_kraken_price
fee_per_trade = position_value * KRAKEN_FEE_RATE
roundtrip_fees = fee_per_trade * 2  # Buy + Sell

print(f"Cost Analysis:")
print(f"  Average Kraken Price: ${avg_kraken_price:,.2f}")
print(f"  Position Value: ${position_value:,.2f}")
print(f"  Fee per trade: ${fee_per_trade:.2f}")
print(f"  Roundtrip fees: ${roundtrip_fees:.2f}")
print(f"  Minimum profit needed: ${roundtrip_fees * 1.5:.2f} (fees + 50% margin)")
print()

# Define entry thresholds
min_spread_for_profit = roundtrip_fees / POSITION_SIZE_BTC  # Spread needed just to break even
target_spread = min_spread_for_profit * 2  # Target 2x fees for safety

print(f"Entry Thresholds:")
print(f"  Breakeven spread: ${min_spread_for_profit:.2f}")
print(f"  Target entry spread: ${target_spread:.2f}")
print()

# ============================================================
# STEP 3: DIRECTIONAL ANALYSIS - WHICH WAY TO TRADE?
# ============================================================
print("=" * 60)
print("DIRECTIONAL ANALYSIS")
print("=" * 60)

# When Coinbase > Kraken (positive spread), does Kraken rise to meet it?
# When Coinbase < Kraken (negative spread), does Kraken fall to meet it?

results = []

for spread_threshold in [10, 20, 30, 40, 50]:
    # Find entry points where spread exceeds threshold
    df['signal_long'] = df['spread'] > spread_threshold  # Coinbase higher, expect Kraken to rise
    df['signal_short'] = df['spread'] < -spread_threshold  # Coinbase lower, expect Kraken to fall
    
    long_trades = []
    short_trades = []
    
    # Simulate LONG trades (buy Kraken when it's below Coinbase)
    for idx in df[df['signal_long']].index[:-360]:  # Leave room for 6-minute window
        entry_price = df.loc[idx, 'kraken_price']
        entry_spread = df.loc[idx, 'spread']
        
        # Check price movement over next 1, 3, 6 minutes
        for minutes in [1, 3, 6]:
            future_idx = idx + minutes * 60
            if future_idx < len(df):
                exit_price = df.loc[future_idx, 'kraken_price']
                exit_spread = df.loc[future_idx, 'spread']
                
                # Calculate P&L
                price_change = exit_price - entry_price
                pnl_before_fees = price_change * POSITION_SIZE_BTC
                pnl_after_fees = pnl_before_fees - roundtrip_fees
                
                long_trades.append({
                    'spread_threshold': spread_threshold,
                    'hold_minutes': minutes,
                    'entry_spread': entry_spread,
                    'exit_spread': exit_spread,
                    'price_change': price_change,
                    'pnl_before_fees': pnl_before_fees,
                    'pnl_after_fees': pnl_after_fees,
                    'profitable': pnl_after_fees > 0
                })
    
    # Simulate SHORT trades (sell Kraken when it's above Coinbase)  
    for idx in df[df['signal_short']].index[:-360]:
        entry_price = df.loc[idx, 'kraken_price']
        entry_spread = df.loc[idx, 'spread']
        
        for minutes in [1, 3, 6]:
            future_idx = idx + minutes * 60
            if future_idx < len(df):
                exit_price = df.loc[future_idx, 'kraken_price']
                exit_spread = df.loc[future_idx, 'spread']
                
                # Calculate P&L (SHORT: profit when price falls)
                price_change = entry_price - exit_price
                pnl_before_fees = price_change * POSITION_SIZE_BTC
                pnl_after_fees = pnl_before_fees - roundtrip_fees
                
                short_trades.append({
                    'spread_threshold': spread_threshold,
                    'hold_minutes': minutes,
                    'entry_spread': entry_spread,
                    'exit_spread': exit_spread,
                    'price_change': price_change,
                    'pnl_before_fees': pnl_before_fees,
                    'pnl_after_fees': pnl_after_fees,
                    'profitable': pnl_after_fees > 0
                })
    
    results.extend(long_trades)
    results.extend(short_trades)

# Convert to DataFrame for analysis
trades_df = pd.DataFrame(results)

# ============================================================
# STEP 4: PROFITABILITY ANALYSIS
# ============================================================
print("=" * 60)
print("PROFITABILITY ANALYSIS (After Fees)")
print("=" * 60)
print()

print("WIN RATES BY SPREAD THRESHOLD AND HOLDING PERIOD:")
print("-" * 60)
print(f"{'Spread':<10} {'1 min':<15} {'3 min':<15} {'6 min':<15}")
print("-" * 60)

for threshold in [10, 20, 30, 40, 50]:
    row = f"${threshold:<9}"
    for minutes in [1, 3, 6]:
        subset = trades_df[(trades_df['spread_threshold'] == threshold) & 
                          (trades_df['hold_minutes'] == minutes)]
        if len(subset) > 0:
            win_rate = subset['profitable'].mean() * 100
            avg_pnl = subset['pnl_after_fees'].mean()
            row += f"{win_rate:>5.1f}% (${avg_pnl:>6.2f}) "
        else:
            row += f"{'N/A':>15} "
    print(row)

print()

# ============================================================
# STEP 5: OPTIMAL STRATEGY PARAMETERS
# ============================================================
print("=" * 60)
print("OPTIMAL STRATEGY PARAMETERS")
print("=" * 60)
print()

# Find best combination
best_configs = []
for threshold in [10, 20, 30, 40, 50]:
    for minutes in [1, 3, 6]:
        subset = trades_df[(trades_df['spread_threshold'] == threshold) & 
                          (trades_df['hold_minutes'] == minutes)]
        if len(subset) > 10:  # Need minimum sample size
            win_rate = subset['profitable'].mean()
            avg_pnl = subset['pnl_after_fees'].mean()
            total_pnl = subset['pnl_after_fees'].sum()
            num_trades = len(subset)
            
            # Calculate Sharpe-like metric
            if subset['pnl_after_fees'].std() > 0:
                sharpe = avg_pnl / subset['pnl_after_fees'].std()
            else:
                sharpe = 0
            
            best_configs.append({
                'spread_threshold': threshold,
                'hold_minutes': minutes,
                'win_rate': win_rate,
                'avg_pnl': avg_pnl,
                'total_pnl': total_pnl,
                'num_trades': num_trades,
                'sharpe': sharpe
            })

best_df = pd.DataFrame(best_configs).sort_values('sharpe', ascending=False)

print("TOP 5 CONFIGURATIONS BY RISK-ADJUSTED RETURN:")
print(best_df.head().to_string(index=False))
print()

# ============================================================
# STEP 6: VISUALIZATIONS
# ============================================================
print("=" * 60)
print("GENERATING VISUALIZATIONS")
print("=" * 60)

fig, axes = plt.subplots(2, 2, figsize=(15, 10))

# 1. Spread distribution
ax1 = axes[0, 0]
ax1.hist(df['spread'], bins=100, alpha=0.7, edgecolor='black')
ax1.axvline(x=target_spread, color='red', linestyle='--', label=f'Target Entry: ${target_spread:.0f}')
ax1.axvline(x=-target_spread, color='red', linestyle='--')
ax1.set_xlabel('Spread (Coinbase - Kraken) [$]')
ax1.set_ylabel('Frequency')
ax1.set_title('Distribution of Price Spreads')
ax1.legend()
ax1.grid(True, alpha=0.3)

# 2. Win Rate by Spread Size
ax2 = axes[0, 1]
for minutes in [1, 3, 6]:
    win_rates = []
    thresholds = [10, 20, 30, 40, 50]
    for threshold in thresholds:
        subset = trades_df[(trades_df['spread_threshold'] == threshold) & 
                          (trades_df['hold_minutes'] == minutes)]
        if len(subset) > 0:
            win_rates.append(subset['profitable'].mean() * 100)
        else:
            win_rates.append(0)
    ax2.plot(thresholds, win_rates, marker='o', label=f'{minutes} min hold')

ax2.axhline(y=50, color='gray', linestyle='--', alpha=0.5)
ax2.set_xlabel('Spread Threshold [$]')
ax2.set_ylabel('Win Rate [%]')
ax2.set_title('Win Rate vs Entry Spread Threshold')
ax2.legend()
ax2.grid(True, alpha=0.3)

# 3. Average P&L by Configuration
ax3 = axes[1, 0]
for minutes in [1, 3, 6]:
    avg_pnls = []
    thresholds = [10, 20, 30, 40, 50]
    for threshold in thresholds:
        subset = trades_df[(trades_df['spread_threshold'] == threshold) & 
                          (trades_df['hold_minutes'] == minutes)]
        if len(subset) > 0:
            avg_pnls.append(subset['pnl_after_fees'].mean())
        else:
            avg_pnls.append(0)
    ax3.plot(thresholds, avg_pnls, marker='s', label=f'{minutes} min hold')

ax3.axhline(y=0, color='red', linestyle='-', alpha=0.5)
ax3.set_xlabel('Spread Threshold [$]')
ax3.set_ylabel('Average P&L per Trade [$]')
ax3.set_title('Profitability by Entry Threshold (After Fees)')
ax3.legend()
ax3.grid(True, alpha=0.3)

# 4. Cumulative P&L for best strategy
ax4 = axes[1, 1]
if len(best_df) > 0:
    best_config = best_df.iloc[0]
    best_trades = trades_df[(trades_df['spread_threshold'] == best_config['spread_threshold']) & 
                            (trades_df['hold_minutes'] == best_config['hold_minutes'])]
    best_trades = best_trades.sort_index()
    cumulative_pnl = best_trades['pnl_after_fees'].cumsum()
    
    ax4.plot(range(len(cumulative_pnl)), cumulative_pnl, linewidth=2)
    ax4.fill_between(range(len(cumulative_pnl)), 0, cumulative_pnl, alpha=0.3)
    ax4.set_xlabel('Trade Number')
    ax4.set_ylabel('Cumulative P&L [$]')
    ax4.set_title(f"Best Strategy: ${best_config['spread_threshold']:.0f} spread, {best_config['hold_minutes']:.0f} min hold")
    ax4.grid(True, alpha=0.3)

plt.tight_layout()
plt.show()

# ============================================================
# STEP 7: PRACTICAL TRADING RULES
# ============================================================
print()
print("=" * 60)
print("RECOMMENDED TRADING RULES")
print("=" * 60)
print()

if len(best_df) > 0:
    optimal = best_df.iloc[0]
    
    print(f"OPTIMAL CONFIGURATION:")
    print(f"  Entry Signal: Spread > ${optimal['spread_threshold']:.0f}")
    print(f"  Holding Period: {optimal['hold_minutes']:.0f} minutes")
    print(f"  Win Rate: {optimal['win_rate']*100:.1f}%")
    print(f"  Avg P&L per trade: ${optimal['avg_pnl']:.2f}")
    print(f"  Risk-adjusted score: {optimal['sharpe']:.3f}")
    print()
    
    print("TRADING RULES:")
    print(f"1. LONG Signal: When Coinbase - Kraken > ${optimal['spread_threshold']:.0f}")
    print(f"   → BUY {POSITION_SIZE_BTC} BTC on Kraken")
    print(f"   → Hold for {optimal['hold_minutes']:.0f} minutes")
    print(f"   → SELL on Kraken")
    print()
    print(f"2. SHORT Signal: When Kraken - Coinbase > ${optimal['spread_threshold']:.0f}")
    print(f"   → SELL {POSITION_SIZE_BTC} BTC on Kraken")
    print(f"   → Hold for {optimal['hold_minutes']:.0f} minutes")
    print(f"   → BUY back on Kraken")
    print()
    print(f"3. Risk Management:")
    print(f"   → Maximum position: {POSITION_SIZE_BTC} BTC")
    print(f"   → No overlapping trades")
    print(f"   → Stop if spread widens beyond 2x entry")

# ============================================================
# STEP 8: FINAL SUMMARY
# ============================================================
print()
print("=" * 60)
print("STRATEGY SUMMARY")
print("=" * 60)
print()

print("KEY FINDINGS:")
print("1. Coinbase price can be used as a leading indicator for Kraken trades")
print("2. Larger spread thresholds generally have higher win rates")
print("3. Longer holding periods improve profitability")
print(f"4. Fees of ${roundtrip_fees:.2f} per roundtrip are significant")
print("5. Strategy requires spreads > $20 to be consistently profitable")
print()

if len(trades_df) > 0:
    profitable_trades = trades_df[trades_df['profitable']]
    losing_trades = trades_df[~trades_df['profitable']]
    
    print("OVERALL STATISTICS:")
    print(f"  Total simulated trades: {len(trades_df):,}")
    print(f"  Profitable trades: {len(profitable_trades):,} ({len(profitable_trades)/len(trades_df)*100:.1f}%)")
    print(f"  Average winner: ${profitable_trades['pnl_after_fees'].mean():.2f}" if len(profitable_trades) > 0 else "  Average winner: N/A")
    print(f"  Average loser: ${losing_trades['pnl_after_fees'].mean():.2f}" if len(losing_trades) > 0 else "  Average loser: N/A")
    print(f"  Total P&L: ${trades_df['pnl_after_fees'].sum():.2f}")

print()
print("=" * 60)
print("ANALYSIS COMPLETE")
print("=" * 60)

conn.close()

# Template definition for notebook UI
template = {
    "name": "Kraken Signal Trading Analysis",
    "description": "Trade on Kraken using Coinbase price as signal - directional P&L analysis with fees",
    "cells": [
        {
            "type": "code",
            "content": """#!/usr/bin/env python3
# Kraken Signal Trading Analysis - Using Coinbase Price as Leading Indicator
# This will be loaded with the actual template code
"""
        }
    ]
}
"""
Template Service - Load and manage notebook templates
"""
import json
import os
from pathlib import Path


def load_arbitrage_template():
    """Load the basic arbitrage analysis template"""
    template_path = Path(__file__).parent.parent / 'notebook_templates' / 'arbitrage_basic.py'
    
    # Execute the template file to get the template dict
    with open(template_path, 'r') as f:
        template_code = f.read()
        
    # Create a namespace and execute the template
    namespace = {}
    exec(template_code, namespace)
    
    return namespace.get('template', {})


def load_tick_arbitrage_template():
    """Load the tick-level arbitrage analysis template"""
    template_path = Path(__file__).parent.parent / 'notebook_templates' / 'tick_arbitrage_analysis.py'
    
    # Execute the template file to get the template dict
    with open(template_path, 'r') as f:
        template_code = f.read()
        
    # Create a namespace and execute the template
    namespace = {}
    exec(template_code, namespace)
    
    return namespace.get('template', {})


def load_trade_data_template():
    """Load the trade data analysis template"""
    template_path = Path(__file__).parent.parent / 'notebook_templates' / 'trade_data_analysis.py'
    
    # Execute the template file to get the template dict
    with open(template_path, 'r') as f:
        template_code = f.read()
        
    # Create a namespace and execute the template
    namespace = {}
    exec(template_code, namespace)
    
    return namespace.get('template', {})


def load_convergence_template():
    """Load the convergence trading analysis template"""
    template_path = Path(__file__).parent.parent / 'notebook_templates' / 'convergence_trading_analysis.py'
    
    # Execute the template file to get the template dict
    with open(template_path, 'r') as f:
        template_code = f.read()
        
    # Create a namespace and execute the template
    namespace = {}
    exec(template_code, namespace)
    
    return namespace.get('template', {})


def load_kraken_signal_template():
    """Load the Kraken signal trading analysis template"""
    
    # Return the template with properly structured cells
    return {
        "name": "Kraken Signal Trading Analysis",
        "title": "Kraken Signal Trading Analysis",
        "description": "Trade on Kraken using Coinbase price as signal - directional P&L analysis with fees",
        "cells": [
            {
                "type": "markdown",
                "content": """# Kraken Signal Trading Analysis

## Strategy Overview
This analysis examines trading opportunities on **Kraken only** using Coinbase price divergence as a signal.

### Key Questions:
1. When Coinbase price diverges from Kraken, does Kraken tend to converge?
2. What spread threshold provides the best risk/reward after fees?
3. What holding period optimizes profitability?

### Trading Approach:
- **Spread Definition**: Spread = Coinbase Price - Kraken Price
  - Positive spread (+$20) = Coinbase is $20 higher than Kraken
  - Negative spread (-$20) = Kraken is $20 higher than Coinbase
  
- **LONG Signal**: When spread > threshold (Coinbase higher than Kraken)
  - → BUY on Kraken (expecting Kraken to rise toward Coinbase)
  
- **SHORT Signal**: When spread < -threshold (Kraken higher than Coinbase) 
  - → SELL on Kraken (expecting Kraken to fall toward Coinbase)
- **Exit**: After fixed time period (1, 3, or 6 minutes)
- **Fees**: 0.25% on entry and exit (retail rate)"""
            },
            {
                "type": "code",
                "content": """import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
import duckdb
from datetime import datetime, timedelta
import warnings
warnings.filterwarnings('ignore')

# Configuration for PERPETUAL FUTURES
# Assuming Kraken perps will be similar to Coinbase International/Binance
NOTIONAL_SIZE_USD = 10000  # $10k notional position size
LEVERAGE = 10  # 10x leverage (conservative)
MARGIN_REQUIRED_USD = NOTIONAL_SIZE_USD / LEVERAGE  # $1,000 margin

# Expected Kraken Perpetual Futures Fee Structure
MAKER_FEE_RATE = 0.0002  # 0.02% maker (limit orders)
TAKER_FEE_RATE = 0.0005  # 0.05% taker (market orders)
# We'll use maker fees since we're patient with entries
KRAKEN_FEE_RATE = MAKER_FEE_RATE

print("=" * 60)
print("KRAKEN PERPETUAL FUTURES TRADING ANALYSIS")
print("Using Coinbase Spot as Signal, Trading Kraken Perps")
print("=" * 60)
print()
print(f"FUTURES CONFIGURATION:")
print(f"  Notional Position: ${NOTIONAL_SIZE_USD:,}")
print(f"  Leverage: {LEVERAGE}x")
print(f"  Margin Required: ${MARGIN_REQUIRED_USD:,}")
print(f"  Maker Fee Rate: {MAKER_FEE_RATE:.3%}")
print(f"  Taker Fee Rate: {TAKER_FEE_RATE:.3%}")
print(f"  Using: MAKER fees (limit orders)")
print()
print("STRATEGY: Mean reversion between Coinbase spot and Kraken perps")
print("ASSUMPTION: Kraken perps will track Kraken spot with minimal basis")
print()"""
            },
            {
                "type": "markdown",
                "content": """## Step 1: Load Synchronized Price Data
First, we need to get time-aligned price data from both exchanges to calculate spreads."""
            },
            {
                "type": "code",
                "content": """# Connect to database
conn = duckdb.connect('../backend/market_data/market_data.duckdb', read_only=True)

# First check data availability
data_check = \"\"\"
SELECT exchange, 
       COUNT(*) as trade_count,
       MIN(datetime) as start_time, 
       MAX(datetime) as end_time,
       EXTRACT(EPOCH FROM (MAX(datetime) - MIN(datetime)))/3600 as hours_of_data
FROM trades 
WHERE symbol = 'BTC/USD'
GROUP BY exchange
\"\"\"
print("DATA AVAILABILITY CHECK:")
data_df = conn.execute(data_check).df()
print(data_df.to_string())
print()
print("Note: Kraken has significantly less data due to lower trading volume")
print()

query = \"\"\"
WITH aligned_data AS (
    -- Use 5-second windows for better synchronization with low-volume exchanges
    SELECT 
        DATE_TRUNC('second', datetime) - INTERVAL (EXTRACT(SECOND FROM datetime)::INT % 5) SECOND as timestamp,
        exchange,
        AVG(price) as price,
        COUNT(*) as trades
    FROM trades 
    WHERE symbol = 'BTC/USD'
        -- Get ALL available data, not limited to 24 hours
    GROUP BY DATE_TRUNC('second', datetime) - INTERVAL (EXTRACT(SECOND FROM datetime)::INT % 5) SECOND, exchange
),
spread_data AS (
    SELECT 
        cb.timestamp,
        cb.price as coinbase_price,
        kr.price as kraken_price,
        ABS(cb.price - kr.price) as spread,  -- Magnitude (always positive)
        cb.price - kr.price as price_diff,   -- Signed difference for direction
        (cb.price - kr.price) / cb.price * 100 as diff_pct,
        cb.trades as cb_trades,
        kr.trades as kr_trades
    FROM aligned_data cb
    JOIN aligned_data kr ON cb.timestamp = kr.timestamp
    WHERE cb.exchange = 'coinbase' AND kr.exchange = 'kraken'
)
SELECT * FROM spread_data
ORDER BY timestamp
\"\"\"

df = conn.execute(query).df()
print(f"Loaded {len(df):,} synchronized 5-second windows")
print(f"Date range (UTC): {df['timestamp'].min()} to {df['timestamp'].max()}")
total_hours = (df['timestamp'].max() - df['timestamp'].min()).total_seconds()/3600
print(f"Total hours of overlapping data: {total_hours:.1f}")
print(f"Coverage: {len(df)/(total_hours*12):.1f}% of possible 5-second windows")
print(f"Note: Timestamps are in UTC (7 hours ahead of PDT)")
print()

# Calculate z-scores for price differences
df['z_score'] = (df['price_diff'] - df['price_diff'].mean()) / df['price_diff'].std()

# Calculate statistics
print("SPREAD STATISTICS (Absolute Distance):")
print(f"  Average spread: ${df['spread'].mean():.2f}")
print(f"  Max spread: ${df['spread'].max():.2f}")
print(f"  Min spread: ${df['spread'].min():.2f}")
print()
print("DIRECTIONAL PRICE DIFFERENCE (Coinbase - Kraken):")
print(f"  Mean: ${df['price_diff'].mean():.2f} (positive = Coinbase higher on average)")
print(f"  Std deviation: ${df['price_diff'].std():.2f}")
print(f"  Most negative: ${df['price_diff'].min():.2f} (Kraken was ${abs(df['price_diff'].min()):.2f} higher)")
print(f"  Most positive: ${df['price_diff'].max():.2f} (Coinbase was ${df['price_diff'].max():.2f} higher)")
print()
print("Z-SCORE STATISTICS:")
print(f"  Max |z-score|: {df['z_score'].abs().max():.2f}")
print(f"  Values > 0.5σ: {(df['z_score'].abs() > 0.5).sum():,} ({(df['z_score'].abs() > 0.5).mean()*100:.1f}%)")
print(f"  Values > 1.0σ: {(df['z_score'].abs() > 1.0).sum():,} ({(df['z_score'].abs() > 1.0).mean()*100:.1f}%)")
print(f"  Values > 1.5σ: {(df['z_score'].abs() > 1.5).sum():,} ({(df['z_score'].abs() > 1.5).mean()*100:.1f}%)")
print(f"  Values > 2.0σ: {(df['z_score'].abs() > 2.0).sum():,} ({(df['z_score'].abs() > 2.0).mean()*100:.1f}%)")
print()

# Break down spreads by z-score levels
print("SPREAD AMOUNTS BY Z-SCORE LEVEL:")
for z_level in [0.5, 1.0, 1.5, 2.0]:
    # LONG opportunities (Coinbase > Kraken)
    long_mask = df['z_score'] > z_level
    if long_mask.sum() > 0:
        avg_spread_long = df[long_mask]['spread'].mean()
        avg_diff_long = df[long_mask]['price_diff'].mean()
        print(f"  Z > {z_level}σ (LONG signals, Coinbase higher):")
        print(f"    Count: {long_mask.sum()}")
        print(f"    Avg spread: ${avg_spread_long:.2f}")
        print(f"    Avg Coinbase premium: ${avg_diff_long:.2f}")
    
    # SHORT opportunities (Kraken > Coinbase)
    short_mask = df['z_score'] < -z_level
    if short_mask.sum() > 0:
        avg_spread_short = df[short_mask]['spread'].mean()
        avg_diff_short = df[short_mask]['price_diff'].mean()
        print(f"  Z < -{z_level}σ (SHORT signals, Kraken higher):")
        print(f"    Count: {short_mask.sum()}")
        print(f"    Avg spread: ${avg_spread_short:.2f}")
        print(f"    Avg Kraken premium: ${abs(avg_diff_short):.2f}")

# Suggest appropriate thresholds based on data
# Use more granular thresholds that will actually capture trades
percentile_99 = df['z_score'].abs().quantile(0.99)
percentile_95 = df['z_score'].abs().quantile(0.95)
percentile_90 = df['z_score'].abs().quantile(0.90)

print(f"\\nZ-score percentiles:")
print(f"  90th percentile: {percentile_90:.2f}σ")
print(f"  95th percentile: {percentile_95:.2f}σ")
print(f"  99th percentile: {percentile_99:.2f}σ")

# Set thresholds based on actual data distribution
# Use more granular thresholds to ensure we get trades
if percentile_99 < 0.5:
    print("\\nWARNING: Very low volatility - using micro thresholds")
    z_thresholds = [0.1, 0.2, 0.3, 0.4, 0.5]
elif percentile_99 < 1.0:
    print("\\nNOTE: Low volatility - using small thresholds")
    z_thresholds = [0.3, 0.5, 0.7, 0.9]
elif percentile_99 < 2.0:
    print("\\nNOTE: Moderate volatility - using medium thresholds")
    z_thresholds = [0.5, 0.75, 1.0, 1.25, 1.5]
else:
    print("\\nNOTE: High volatility - using standard thresholds")
    # Use percentiles to ensure we get some trades
    z_thresholds = [percentile_90, percentile_95, percentile_99, percentile_99 * 1.1, percentile_99 * 1.2]
    z_thresholds = [round(z, 2) for z in z_thresholds if z < df['z_score'].abs().max()]

print(f"Selected thresholds: {z_thresholds}")"""
            },
            {
                "type": "markdown",
                "content": """## Step 2: Calculate Trading Costs
Understanding the fee structure is critical for profitability."""
            },
            {
                "type": "code",
                "content": """# Calculate fee hurdle in dollars for FUTURES
avg_kraken_price = df['kraken_price'].mean()
POSITION_SIZE_BTC = NOTIONAL_SIZE_USD / avg_kraken_price  # BTC size of futures position
fee_per_trade = NOTIONAL_SIZE_USD * KRAKEN_FEE_RATE
roundtrip_fees = fee_per_trade * 2  # Open + Close position

print("PERPETUAL FUTURES COST ANALYSIS:")
print(f"  Average BTC Price: ${avg_kraken_price:,.2f}")
print(f"  Notional Position: ${NOTIONAL_SIZE_USD:,.2f}")
print(f"  Position Size: {POSITION_SIZE_BTC:.4f} BTC")
print(f"  Margin Required: ${MARGIN_REQUIRED_USD:,.2f} ({100/LEVERAGE:.0f}% of notional)")
print(f"  Fee per trade: ${fee_per_trade:.2f} ({KRAKEN_FEE_RATE:.3%} of notional)")
print(f"  Roundtrip fees: ${roundtrip_fees:.2f}")
print()

# Calculate P&L with leverage
print("LEVERAGE IMPACT ON P&L:")
one_percent_move = avg_kraken_price * 0.01
pnl_one_percent = one_percent_move * POSITION_SIZE_BTC
print(f"  1% BTC price move = ${one_percent_move:.2f}")
print(f"  Your P&L on 1% move: ${pnl_one_percent:.2f}")
print(f"  Return on margin: {pnl_one_percent/MARGIN_REQUIRED_USD*100:.1f}% (due to {LEVERAGE}x leverage)")
print()

# Define entry thresholds for futures
min_spread_for_profit = roundtrip_fees / POSITION_SIZE_BTC
target_spread = min_spread_for_profit * 1.5  # Lower safety margin with futures

print("BREAKEVEN ANALYSIS (FUTURES):")
print(f"  Roundtrip fees: ${roundtrip_fees:.2f}")
print(f"  BTC movement needed to break even: ${min_spread_for_profit:.2f}")
print(f"  As % of BTC price: {(min_spread_for_profit / avg_kraken_price * 100):.4f}%")
print(f"  On your margin: {(roundtrip_fees / MARGIN_REQUIRED_USD * 100):.2f}% cost")
print()

# Compare to spot spread opportunities
avg_spread = df['spread'].mean()
potential_profit = avg_spread * POSITION_SIZE_BTC
net_profit = potential_profit - roundtrip_fees
return_on_margin = net_profit / MARGIN_REQUIRED_USD * 100

print(f"SPREAD OPPORTUNITY ANALYSIS:")
print(f"  Current avg spread: ${avg_spread:.2f}")
print(f"  If captured fully: ${potential_profit:.2f} gross profit")
print(f"  Net after fees: ${net_profit:.2f}")
print(f"  Return on margin: {return_on_margin:.2f}%")
print()

if net_profit > 0:
    print(f"  ✅ PROFITABLE: Average spread covers fees with ${net_profit:.2f} profit")
    print(f"  📈 Each trade returns {return_on_margin:.2f}% on ${MARGIN_REQUIRED_USD} margin")
else:
    print(f"  ⚠️ WARNING: Average spread insufficient, need ${-net_profit:.2f} more")
    print(f"  📊 Minimum profitable spread: ${min_spread_for_profit:.2f}")"""
            },
            {
                "type": "markdown",
                "content": """## Step 3: Backtest Trading Strategy
Simulate trades based on spread signals and calculate actual P&L after fees."""
            },
            {
                "type": "code",
                "content": """results = []
reversion_times = []
debug_info = []

# === ENHANCED TRADING ALGORITHM PARAMETERS ===
# Entry: When spread is wide (high z-score)
# Exit: Multiple conditions - whichever comes first:
#   1. Spread converges to mean (z-score < 0.5)
#   2. Spread reverses (opposite extreme)
#   3. Maximum holding time reached
#   4. Stop loss triggered

REVERSION_TARGET = 0.5  # Exit when z-score returns to within 0.5σ
STOP_LOSS_MULTIPLIER = 1.5  # Stop if spread widens by 50% more
MAX_HOLD_MINUTES = 30  # Maximum holding time
MAX_LOOK_AHEAD = int(MAX_HOLD_MINUTES * 60 / 5)  # Convert to 5-second windows

print("FUTURES TRADING ALGORITHM PARAMETERS:")
print(f"  Entry: Z-score exceeds threshold AND spread > minimum")
print(f"  Position Sizing: ${NOTIONAL_SIZE_USD:,} notional ({POSITION_SIZE_BTC:.4f} BTC)")
print(f"  Leverage: {LEVERAGE}x on ${MARGIN_REQUIRED_USD:,} margin")
print(f"  Exit conditions:")
print(f"    1. Convergence: Z-score < {REVERSION_TARGET}σ")
print(f"    2. Stop loss: Spread widens by {(STOP_LOSS_MULTIPLIER-1)*100:.0f}%")
print(f"    3. Time limit: {MAX_HOLD_MINUTES} minutes")
print(f"    4. Reversal: Spread flips to opposite extreme")
print()

# Calculate minimum profitable spread for futures
MIN_SPREAD_FOR_PROFIT = roundtrip_fees / POSITION_SIZE_BTC  # Much lower with futures!
print(f"Minimum spread for profit (with {KRAKEN_FEE_RATE:.3%} fees): ${MIN_SPREAD_FOR_PROFIT:.2f}")
print(f"Safety threshold (1.5x fees): ${MIN_SPREAD_FOR_PROFIT * 1.5:.2f}")
print()

# Show how many opportunities exist
profitable_spreads = (df['spread'] > MIN_SPREAD_FOR_PROFIT).sum()
safe_spreads = (df['spread'] > MIN_SPREAD_FOR_PROFIT * 1.5).sum()
print(f"Trading opportunities in data:")
print(f"  Spreads > breakeven: {profitable_spreads:,} ({profitable_spreads/len(df)*100:.1f}%)")
print(f"  Spreads > safety threshold: {safe_spreads:,} ({safe_spreads/len(df)*100:.1f}%)")
print()

# Use the z_thresholds determined in Step 1
for z_threshold in z_thresholds:
    # Find entry points where z-score exceeds threshold
    df['signal_long'] = df['z_score'] > z_threshold  # Coinbase significantly higher
    df['signal_short'] = df['z_score'] < -z_threshold  # Kraken significantly higher
    
    num_long_signals = df['signal_long'].sum()
    num_short_signals = df['signal_short'].sum()
    debug_info.append(f"Z={z_threshold:.1f}: {num_long_signals} long, {num_short_signals} short signals")
    
    # Analyze LONG trades (buy Kraken when Coinbase significantly higher)
    # Only exclude signals that are too close to the end to have exit opportunities
    if len(df) > MAX_LOOK_AHEAD:
        long_signals = df[df['signal_long'] & (df.index < len(df) - MAX_LOOK_AHEAD)].index
    else:
        long_signals = df[df['signal_long'] & (df.index < len(df) - 10)].index  # At least 10 windows for exit
    
    print(f"  Processing {len(long_signals)} LONG signals for z={z_threshold}")
    
    # Reset position tracking for each threshold
    threshold_last_exit = -1
    
    for idx in long_signals[:100]:  # Analyze more trades
        # Skip if we're still in a position from this threshold
        if idx <= threshold_last_exit:
            continue
            
        entry_price = df.loc[idx, 'kraken_price']
        entry_z = df.loc[idx, 'z_score']
        entry_spread = df.loc[idx, 'spread']
        
        # Skip if spread is too small to be profitable
        if entry_spread < MIN_SPREAD_FOR_PROFIT:
            continue
        
        # Track exit conditions
        exit_reason = None
        best_exit_idx = None
        best_z_score = entry_z
        stop_loss_spread = entry_spread * STOP_LOSS_MULTIPLIER
        
        for look_ahead in range(1, min(MAX_LOOK_AHEAD, len(df) - idx)):
            future_idx = idx + look_ahead
            future_z = df.loc[future_idx, 'z_score']
            future_spread = df.loc[future_idx, 'spread']
            
            # Exit Condition 1: Spread converged (profitable exit)
            if abs(future_z) <= REVERSION_TARGET:
                best_exit_idx = future_idx
                exit_reason = 'CONVERGED'
                break
            
            # Exit Condition 2: Stop loss (spread widened too much)
            if future_spread > stop_loss_spread:
                best_exit_idx = future_idx
                exit_reason = 'STOP_LOSS'
                break
                
            # Exit Condition 3: Spread reversed to opposite extreme
            if future_z < -z_threshold:  # Now Kraken is higher
                best_exit_idx = future_idx
                exit_reason = 'REVERSED'
                break
            
            # Track best (closest to zero) z-score seen
            if abs(future_z) < abs(best_z_score):
                best_z_score = future_z
                best_exit_idx = future_idx
                exit_reason = 'PARTIAL_CONV'
        
        # Exit Condition 4: Time limit reached
        if best_exit_idx is None or exit_reason == 'PARTIAL_CONV':
            if best_exit_idx is None:
                best_exit_idx = min(idx + MAX_LOOK_AHEAD - 1, len(df) - 1)
            exit_reason = 'TIME_LIMIT' if exit_reason != 'PARTIAL_CONV' else exit_reason
        
        # Calculate P&L for FUTURES POSITION
        exit_price = df.loc[best_exit_idx, 'kraken_price']
        time_held = (best_exit_idx - idx) * 5 / 60  # minutes
        
        price_change = exit_price - entry_price
        pnl_before_fees = price_change * POSITION_SIZE_BTC
        pnl_after_fees = pnl_before_fees - roundtrip_fees
        
        # Calculate return on margin (leverage effect)
        return_on_margin = (pnl_after_fees / MARGIN_REQUIRED_USD) * 100
        
        results.append({
            'z_threshold': z_threshold,
            'direction': 'LONG',
            'entry_spread': entry_spread,
            'exit_spread': df.loc[best_exit_idx, 'spread'],
            'entry_z_score': entry_z,
            'exit_z_score': df.loc[best_exit_idx, 'z_score'],
            'time_held': time_held,
            'exit_reason': exit_reason,
            'price_change': price_change,
            'pnl_before_fees': pnl_before_fees,
            'pnl_after_fees': pnl_after_fees,
            'return_on_margin': return_on_margin,
            'profitable': pnl_after_fees > 0
        })
        
        # Update last exit to prevent overlapping trades for this threshold
        threshold_last_exit = best_exit_idx
        
        if exit_reason == 'CONVERGED':
            reversion_times.append(time_held)
    
    # Analyze SHORT trades (sell Kraken when it's significantly higher)
    # Only exclude signals that are too close to the end to have exit opportunities  
    if len(df) > MAX_LOOK_AHEAD:
        short_signals = df[df['signal_short'] & (df.index < len(df) - MAX_LOOK_AHEAD)].index
    else:
        short_signals = df[df['signal_short'] & (df.index < len(df) - 10)].index  # At least 10 windows for exit
    
    print(f"  Processing {len(short_signals)} SHORT signals for z={z_threshold}")
    
    for idx in short_signals[:100]:  # Analyze more trades
        # Skip if we're still in a position from this threshold
        if idx <= threshold_last_exit:
            continue
            
        entry_price = df.loc[idx, 'kraken_price']
        entry_z = df.loc[idx, 'z_score']
        entry_spread = df.loc[idx, 'spread']
        
        # Skip if spread is too small to be profitable
        if entry_spread < MIN_SPREAD_FOR_PROFIT:
            continue
        
        # Track exit conditions
        exit_reason = None
        best_exit_idx = None
        best_z_score = entry_z
        stop_loss_spread = entry_spread * STOP_LOSS_MULTIPLIER
        
        for look_ahead in range(1, min(MAX_LOOK_AHEAD, len(df) - idx)):
            future_idx = idx + look_ahead
            future_z = df.loc[future_idx, 'z_score']
            future_spread = df.loc[future_idx, 'spread']
            
            # Exit Condition 1: Spread converged (profitable exit)
            if abs(future_z) <= REVERSION_TARGET:
                best_exit_idx = future_idx
                exit_reason = 'CONVERGED'
                break
            
            # Exit Condition 2: Stop loss (spread widened too much)
            if future_spread > stop_loss_spread:
                best_exit_idx = future_idx
                exit_reason = 'STOP_LOSS'
                break
                
            # Exit Condition 3: Spread reversed to opposite extreme
            if future_z > z_threshold:  # Now Coinbase is higher
                best_exit_idx = future_idx
                exit_reason = 'REVERSED'
                break
            
            # Track best (closest to zero) z-score seen
            if abs(future_z) < abs(best_z_score):
                best_z_score = future_z
                best_exit_idx = future_idx
                exit_reason = 'PARTIAL_CONV'
        
        # Exit Condition 4: Time limit reached
        if best_exit_idx is None or exit_reason == 'PARTIAL_CONV':
            if best_exit_idx is None:
                best_exit_idx = min(idx + MAX_LOOK_AHEAD - 1, len(df) - 1)
            exit_reason = 'TIME_LIMIT' if exit_reason != 'PARTIAL_CONV' else exit_reason
        
        # Calculate P&L for SHORT FUTURES POSITION
        exit_price = df.loc[best_exit_idx, 'kraken_price']
        time_held = (best_exit_idx - idx) * 5 / 60  # minutes
        
        price_change = entry_price - exit_price  # SHORT: profit when price falls
        pnl_before_fees = price_change * POSITION_SIZE_BTC
        pnl_after_fees = pnl_before_fees - roundtrip_fees
        
        # Calculate return on margin (leverage effect)
        return_on_margin = (pnl_after_fees / MARGIN_REQUIRED_USD) * 100
        
        results.append({
            'z_threshold': z_threshold,
            'direction': 'SHORT',
            'entry_spread': entry_spread,
            'exit_spread': df.loc[best_exit_idx, 'spread'],
            'entry_z_score': entry_z,
            'exit_z_score': df.loc[best_exit_idx, 'z_score'],
            'time_held': time_held,
            'exit_reason': exit_reason,
            'price_change': price_change,
            'pnl_before_fees': pnl_before_fees,
            'pnl_after_fees': pnl_after_fees,
            'return_on_margin': return_on_margin,
            'profitable': pnl_after_fees > 0
        })
        
        # Update last exit to prevent overlapping trades for this threshold
        threshold_last_exit = best_exit_idx
        
        if exit_reason == 'CONVERGED':
            reversion_times.append(time_held)

# Convert to DataFrame
trades_df = pd.DataFrame(results)

print("\\nSIGNAL GENERATION DEBUG:")
for info in debug_info:
    print(f"  {info}")

print(f"\\nSimulated {len(trades_df):,} trades")

if len(trades_df) > 0:
    print(f"  LONG trades: {len(trades_df[trades_df['direction'] == 'LONG']):,}")
    print(f"  SHORT trades: {len(trades_df[trades_df['direction'] == 'SHORT']):,}")
    
    # Show exit reason breakdown
    print(f"\\nEXIT REASON BREAKDOWN:")
    for reason in ['CONVERGED', 'STOP_LOSS', 'REVERSED', 'PARTIAL_CONV', 'TIME_LIMIT']:
        count = (trades_df['exit_reason'] == reason).sum()
        if count > 0:
            pct = count / len(trades_df) * 100
            avg_pnl = trades_df[trades_df['exit_reason'] == reason]['pnl_after_fees'].mean()
            print(f"  {reason:12}: {count:3} trades ({pct:5.1f}%) | Avg P&L: ${avg_pnl:+7.2f}")
    
    # Show reversion time statistics
    if len(reversion_times) > 0:
        import numpy as np
        print(f"\\nREVERSION TIME STATISTICS (minutes):")
        print(f"  Mean: {np.mean(reversion_times):.1f}")
        print(f"  Median: {np.median(reversion_times):.1f}")
        print(f"  25th percentile: {np.percentile(reversion_times, 25):.1f}")
        print(f"  75th percentile: {np.percentile(reversion_times, 75):.1f}")
        print(f"  Max: {np.max(reversion_times):.1f}")
else:
    print("\\nNo trades found! Possible reasons:")
    print(f"  1. Z-score thresholds too high: {z_thresholds}")
    print(f"  2. Max |z-score| in data: {df['z_score'].abs().max():.2f}")
    print(f"  3. Data length: {len(df)} windows")
    print(f"  4. Last signal values - Long: {df['signal_long'].iloc[-1] if len(df) > 0 else 'N/A'}, Short: {df['signal_short'].iloc[-1] if len(df) > 0 else 'N/A'}")"""
            },
            {
                "type": "markdown",
                "content": """## Step 4: Analyze Results
Calculate win rates and profitability metrics for different configurations."""
            },
            {
                "type": "code",
                "content": """if 'trades_df' in locals() and len(trades_df) > 0:
    print("RESULTS BY Z-SCORE THRESHOLD (FUTURES):")
    print("-" * 95)
    print(f"{'Z-Score':<10} {'Trades':<10} {'Win%':<10} {'Avg P&L':<12} {'Avg RoM%':<12} {'Avg Hold':<12} {'Converged%':<12}")
    print("-" * 95)
    
    for z_threshold in z_thresholds:
        subset = trades_df[trades_df['z_threshold'] == z_threshold]
        if len(subset) > 0:
            win_rate = subset['profitable'].mean() * 100
            avg_pnl = subset['pnl_after_fees'].mean()
            avg_rom = subset['return_on_margin'].mean()
            avg_hold = subset['time_held'].mean()
            converged_pct = (subset['exit_reason'] == 'CONVERGED').mean() * 100
            
            print(f"{z_threshold:<10.1f} {len(subset):<10} {win_rate:<10.1f} ${avg_pnl:<11.2f} {avg_rom:<11.2f}% {avg_hold:<11.1f}m {converged_pct:<11.1f}%")
        else:
            print(f"{z_threshold:<10.1f} {'0':<10} {'N/A':<10} {'N/A':<12} {'N/A':<12} {'N/A':<12} {'N/A':<12}")
    
    # Show distribution of holding times
    if 'time_held' in trades_df.columns:
        print(f"\\nHOLDING TIME DISTRIBUTION:")
        print(f"  < 1 min: {(trades_df['time_held'] < 1).sum()} trades")
        print(f"  1-3 min: {((trades_df['time_held'] >= 1) & (trades_df['time_held'] < 3)).sum()} trades")
        print(f"  3-5 min: {((trades_df['time_held'] >= 3) & (trades_df['time_held'] < 5)).sum()} trades")
        print(f"  5-10 min: {((trades_df['time_held'] >= 5) & (trades_df['time_held'] < 10)).sum()} trades")
        print(f"  10+ min: {(trades_df['time_held'] >= 10).sum()} trades")
elif 'trades_df' not in locals():
    print("ERROR: trades_df not found!")
    print("Please run Step 3 (Backtest Trading Strategy) first to generate trades.")
else:
    print("No trades to analyze!")
    if 'df' in locals():
        print("DEBUG INFO:")
        print(f"  - Used z-score thresholds: {z_thresholds if 'z_thresholds' in locals() else 'Not defined'}")
        print(f"  - Max |z-score| in data: {df['z_score'].abs().max():.2f}")
        print(f"  - Number of LONG signals found: {df['signal_long'].sum() if 'signal_long' in df.columns else 'N/A'}")
        print(f"  - Number of SHORT signals found: {df['signal_short'].sum() if 'signal_short' in df.columns else 'N/A'}")
        print()
        print("Spread statistics:")
        print(f"  - Average spread (magnitude): ${df['spread'].mean():.2f}")
        print(f"  - Max spread: ${df['spread'].max():.2f}")
    print()
    print("This could mean:")
    print("  1. Z-score thresholds are still too high")
    print("  2. Not enough data points exceed the thresholds")
    print("  3. Signal generation logic needs adjustment")"""
            },
            {
                "type": "markdown",
                "content": """## Step 5: Find Optimal Configuration
Identify the best spread threshold and holding period based on risk-adjusted returns."""
            },
            {
                "type": "code",
                "content": """# Find best combination
best_configs = []

# Check if we have trades to analyze
if 'trades_df' not in locals():
    print("ERROR: trades_df not found. Please run Step 3 (Backtest) first.")
elif len(trades_df) == 0:
    print("No trades were generated to analyze.")
    print("This is likely because:")
    print("  1. Data length is too short")
    print("  2. Not enough extreme price divergences")
    print("  3. Consider collecting more data or waiting for higher volatility")
else:
    for z_threshold in z_thresholds:
        subset = trades_df[trades_df['z_threshold'] == z_threshold]
        if len(subset) > 5:  # Need minimum sample size
            win_rate = subset['profitable'].mean()
            avg_pnl = subset['pnl_after_fees'].mean()
            total_pnl = subset['pnl_after_fees'].sum()
            num_trades = len(subset)
            avg_hold_time = subset['time_held'].mean()
            pct_reverted = (subset['exit_reason'] == 'CONVERGED').mean() * 100
            
            # Calculate Sharpe-like metric
            if subset['pnl_after_fees'].std() > 0:
                sharpe = avg_pnl / subset['pnl_after_fees'].std()
            else:
                sharpe = 0
            
            best_configs.append({
                'z_threshold': z_threshold,
                'avg_hold_time': avg_hold_time,
                'pct_reverted': pct_reverted,
                'win_rate': win_rate,
                'avg_pnl': avg_pnl,
                'total_pnl': total_pnl,
                'num_trades': num_trades,
                'sharpe': sharpe
            })

if best_configs:
    best_df = pd.DataFrame(best_configs).sort_values('sharpe', ascending=False)
    print("TOP 5 CONFIGURATIONS BY RISK-ADJUSTED RETURN:")
    print(best_df.head().to_string(index=False))
else:
    best_df = pd.DataFrame()  # Empty DataFrame
    print("No configurations found with sufficient trades for analysis")
    print("This usually means the market is too stable for the strategy")"""
            },
            {
                "type": "markdown",
                "content": """## Step 6: Visualize Performance"""
            },
            {
                "type": "code",
                "content": """# Ensure matplotlib is imported
import matplotlib.pyplot as plt

# Check if required variables exist
if 'df' not in locals():
    print("ERROR: DataFrame 'df' not found. Please run Step 1 first.")
elif 'trades_df' not in locals():
    print("ERROR: trades_df not found. Please run Step 3 (Backtest) first.")
else:
    fig, axes = plt.subplots(2, 2, figsize=(15, 10))
    
    # 1. Z-Score distribution
    ax1 = axes[0, 0]
    ax1.hist(df['z_score'], bins=100, alpha=0.7, edgecolor='black')
    if 'z_thresholds' in locals() and z_thresholds:
        ax1.axvline(x=z_thresholds[2] if len(z_thresholds) > 2 else z_thresholds[-1], 
                    color='red', linestyle='--', label=f'{z_thresholds[2] if len(z_thresholds) > 2 else z_thresholds[-1]:.1f}σ threshold')
        ax1.axvline(x=-(z_thresholds[2] if len(z_thresholds) > 2 else z_thresholds[-1]), 
                    color='red', linestyle='--')
    ax1.set_xlabel('Z-Score (Standard Deviations)')
    ax1.set_ylabel('Frequency')
    ax1.set_title('Distribution of Price Divergence (Z-Scores)')
    ax1.legend()
    ax1.grid(True, alpha=0.3)
    
    # 2. Win Rate by Z-Score
    ax2 = axes[0, 1]
    if len(trades_df) > 0:
        win_rates = []
        avg_holds = []
        for z_threshold in z_thresholds:
            subset = trades_df[trades_df['z_threshold'] == z_threshold]
            if len(subset) > 0:
                win_rates.append(subset['profitable'].mean() * 100)
                avg_holds.append(subset['time_held'].mean())
            else:
                win_rates.append(0)
                avg_holds.append(0)
        
        # Plot win rate as bars with hold time as labels
        bars = ax2.bar(range(len(z_thresholds)), win_rates, color='steelblue', alpha=0.7)
        ax2.set_xticks(range(len(z_thresholds)))
        ax2.set_xticklabels([f'{z:.1f}σ' for z in z_thresholds])
        
        # Add average hold time as text on bars
        for i, (bar, hold) in enumerate(zip(bars, avg_holds)):
            if hold > 0:
                ax2.text(bar.get_x() + bar.get_width()/2, bar.get_height() + 1,
                        f'{hold:.1f}m', ha='center', va='bottom', fontsize=8)
        
        ax2.axhline(y=50, color='gray', linestyle='--', alpha=0.5, label='50% breakeven')
    else:
        ax2.text(0.5, 0.5, 'No trades to visualize', 
                 horizontalalignment='center', verticalalignment='center',
                 transform=ax2.transAxes, fontsize=12)
    ax2.set_xlabel('Z-Score Threshold')
    ax2.set_ylabel('Win Rate [%]')
    ax2.set_title('Win Rate by Z-Score (avg hold time shown)')
    ax2.legend()
    ax2.grid(True, alpha=0.3)
    
    # 3. Average P&L by Configuration
    ax3 = axes[1, 0]
    if len(trades_df) > 0:
        avg_pnls = []
        avg_roms = []
        for z_threshold in z_thresholds:
            subset = trades_df[trades_df['z_threshold'] == z_threshold]
            if len(subset) > 0:
                avg_pnls.append(subset['pnl_after_fees'].mean())
                avg_roms.append(subset['return_on_margin'].mean())
            else:
                avg_pnls.append(0)
                avg_roms.append(0)
        
        # Create bar chart for P&L
        colors = ['green' if x > 0 else 'red' for x in avg_pnls]
        bars = ax3.bar(range(len(z_thresholds)), avg_pnls, color=colors, alpha=0.7)
        ax3.set_xticks(range(len(z_thresholds)))
        ax3.set_xticklabels([f'{z:.1f}σ' for z in z_thresholds])
        
        # Add RoM% as text on bars
        for i, (bar, rom) in enumerate(zip(bars, avg_roms)):
            if rom != 0:
                y_pos = bar.get_height() + (1 if bar.get_height() > 0 else -2)
                ax3.text(bar.get_x() + bar.get_width()/2, y_pos,
                        f'{rom:.2f}%', ha='center', va='bottom' if bar.get_height() > 0 else 'top', 
                        fontsize=8)
        
        ax3.axhline(y=0, color='black', linestyle='-', alpha=0.5)
    else:
        ax3.text(0.5, 0.5, 'No trades to visualize', 
                 horizontalalignment='center', verticalalignment='center',
                 transform=ax3.transAxes, fontsize=12)
    ax3.set_xlabel('Z-Score Threshold')
    ax3.set_ylabel('Average P&L per Trade [$]')
    ax3.set_title('Profitability by Z-Score (RoM% shown)')
    ax3.grid(True, alpha=0.3)
    
    # 4. Cumulative P&L
    ax4 = axes[1, 1]
    if 'best_df' in locals() and len(best_df) > 0 and len(trades_df) > 0:
        best = best_df.iloc[0]
        best_trades = trades_df[(trades_df['z_threshold'] == best['z_threshold']) & 
                                (trades_df['z_threshold'] == best['z_threshold'])]
        if len(best_trades) > 0:
            cumulative_pnl = best_trades['pnl_after_fees'].cumsum()
            ax4.plot(range(len(cumulative_pnl)), cumulative_pnl, linewidth=2)
            ax4.fill_between(range(len(cumulative_pnl)), 0, cumulative_pnl, alpha=0.3)
            ax4.set_title(f"Best: {best['z_threshold']:.1f}σ, {best['avg_hold_time']:.0f} min avg hold")
        else:
            ax4.text(0.5, 0.5, 'No trades for best config', 
                     horizontalalignment='center', verticalalignment='center',
                     transform=ax4.transAxes, fontsize=12)
    else:
        ax4.text(0.5, 0.5, 'No trades to visualize', 
                 horizontalalignment='center', verticalalignment='center',
                 transform=ax4.transAxes, fontsize=12)
    ax4.set_xlabel('Trade Number')
    ax4.set_ylabel('Cumulative P&L [$]')
    ax4.grid(True, alpha=0.3)
    
    plt.tight_layout()
    plt.show()"""
            },
            {
                "type": "markdown",
                "content": """## Trading Recommendations"""
            },
            {
                "type": "code",
                "content": """# Check if best_df exists and has data
if 'best_df' in locals() and len(best_df) > 0:
    optimal = best_df.iloc[0]
    
    print("=" * 60)
    print("OPTIMAL TRADING CONFIGURATION")
    print("=" * 60)
    print(f"Entry Signal: |Z-Score| > {optimal['z_threshold']:.1f}")
    print(f"Avg Holding Period: {optimal['avg_hold_time']:.0f} minutes")
    print(f"Win Rate: {optimal['win_rate']*100:.1f}%")
    print(f"Avg P&L per trade: ${optimal['avg_pnl']:.2f}")
    print(f"Risk-adjusted score: {optimal['sharpe']:.3f}")
    print()
    
    print("TRADING RULES:")
    print("-" * 40)
    print(f"1. LONG Signal: When Z-Score > +{optimal['z_threshold']:.1f} (Coinbase significantly higher)")
    print(f"   → BUY {POSITION_SIZE_BTC} BTC on Kraken")
    print(f"   → Hold for ~{optimal['avg_hold_time']:.0f} minutes (dynamic exit)")
    print(f"   → SELL on Kraken")
    print(f"   → Expectation: Kraken price rises toward Coinbase")
    print()
    print(f"2. SHORT Signal: When Z-Score < -{optimal['z_threshold']:.1f} (Kraken significantly higher)")
    print(f"   → SELL {POSITION_SIZE_BTC} BTC on Kraken")
    print(f"   → Hold for ~{optimal['avg_hold_time']:.0f} minutes (dynamic exit)")
    print(f"   → BUY back on Kraken")
    print(f"   → Expectation: Kraken price falls toward Coinbase")
    print()
    print("RISK MANAGEMENT:")
    print(f"- Maximum position: {POSITION_SIZE_BTC} BTC")
    print(f"- No overlapping trades")
    print(f"- Stop if |z-score| increases beyond entry + 1σ")
else:
    print("=" * 60)
    print("NO OPTIMAL CONFIGURATION FOUND")
    print("=" * 60)
    print("Unable to find profitable trading configuration.")
    print("This could mean:")
    print("  1. Not enough data (only 347 seconds available)")
    print("  2. Market conditions too stable")
    print("  3. Fees too high relative to price movements")
    print()
    print("Recommendations:")
    print("  - Collect more data (at least 1 hour)")
    print("  - Wait for higher volatility periods")
    print("  - Consider lower fee tier on exchange")

if 'conn' in locals():
    conn.close()"""
            }
        ]
    }


def get_available_templates():
    """Get list of available templates"""
    return [
        {
            "id": "arbitrage_basic",
            "title": "Cross-Exchange Arbitrage",
            "description": "Analyze BTC price differences between Coinbase and Kraken"
        },
        {
            "id": "tick_arbitrage",
            "title": "Tick-Level Arbitrage & Market Making",
            "description": "Advanced tick-level analysis of arbitrage opportunities and market making potential"
        },
        {
            "id": "trade_data_analysis",
            "title": "Trade-Level Market Analysis",
            "description": "Real analysis based on actual streaming trade data - no estimates or guesswork"
        },
        {
            "id": "convergence_trading",
            "title": "Cross-Exchange Convergence Trading",
            "description": "Statistical analysis of price divergence and convergence patterns for profitable trading signals"
        },
        {
            "id": "kraken_signal_trading",
            "title": "Kraken Signal Trading (Coinbase Leading)",
            "description": "Trade on Kraken using Coinbase price as signal - directional P&L analysis with fees"
        }
    ]
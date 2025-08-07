#!/usr/bin/env python3
"""
Vectorized optimization approach - calculate all indicators once, test all parameters.
"""

import numpy as np
import pandas as pd
from pathlib import Path
from datetime import datetime
import time

from nautilus_trader.persistence.catalog import ParquetDataCatalog
from nautilus_trader.model.data import Bar


def calculate_all_emas(prices, periods):
    """Calculate EMAs for all periods at once."""
    emas = {}
    
    for period in periods:
        ema = pd.Series(prices).ewm(span=period, adjust=False).mean().values
        emas[period] = ema
    
    return emas


def simulate_ema_cross_vectorized(prices, fast_ema, slow_ema, trade_size=100):
    """Simulate EMA cross strategy using vectorized operations."""
    
    # Generate signals
    signals = np.where(fast_ema > slow_ema, 1, -1)
    
    # Find position changes
    position_changes = np.diff(signals, prepend=0)
    
    # Entry points (0 -> 1 or -1)
    entries = np.where(position_changes != 0)[0]
    
    if len(entries) == 0:
        return {
            'num_trades': 0,
            'pnl': 0,
            'pnl_pct': 0,
            'win_rate': 0,
            'avg_trade': 0,
            'max_drawdown': 0
        }
    
    # Calculate P&L for each trade
    trades = []
    for i in range(len(entries) - 1):
        entry_idx = entries[i]
        exit_idx = entries[i + 1]
        
        entry_price = prices[entry_idx]
        exit_price = prices[exit_idx]
        
        # Long or short based on signal
        if signals[entry_idx] == 1:  # Long
            pnl = (exit_price - entry_price) * trade_size
        else:  # Short
            pnl = (entry_price - exit_price) * trade_size
        
        trades.append({
            'entry_idx': entry_idx,
            'exit_idx': exit_idx,
            'entry_price': entry_price,
            'exit_price': exit_price,
            'pnl': pnl,
            'return': pnl / (entry_price * trade_size)
        })
    
    if not trades:
        return {
            'num_trades': 0,
            'pnl': 0,
            'pnl_pct': 0,
            'win_rate': 0,
            'avg_trade': 0,
            'max_drawdown': 0
        }
    
    # Calculate metrics
    trade_pnls = [t['pnl'] for t in trades]
    total_pnl = sum(trade_pnls)
    num_trades = len(trades)
    winners = sum(1 for pnl in trade_pnls if pnl > 0)
    win_rate = (winners / num_trades * 100) if num_trades > 0 else 0
    avg_trade = total_pnl / num_trades if num_trades > 0 else 0
    
    # Calculate drawdown
    cumulative_pnl = np.cumsum([0] + trade_pnls)
    running_max = np.maximum.accumulate(cumulative_pnl)
    drawdown = cumulative_pnl - running_max
    max_drawdown = abs(min(drawdown))
    
    return {
        'num_trades': num_trades,
        'pnl': total_pnl,
        'pnl_pct': (total_pnl / 100_000) * 100,
        'win_rate': win_rate,
        'avg_trade': avg_trade,
        'max_drawdown': max_drawdown,
        'trades': trades  # Keep detailed trades for analysis
    }


def optimize_vectorized(bars, fast_periods, slow_periods):
    """Run vectorized optimization across all parameter combinations."""
    
    # Extract close prices
    prices = np.array([float(bar.close) for bar in bars])
    timestamps = np.array([bar.ts_event for bar in bars])
    
    print(f"\nCalculating EMAs for all periods...")
    
    # Calculate all EMAs once
    all_periods = sorted(set(fast_periods + slow_periods))
    emas = calculate_all_emas(prices, all_periods)
    
    print(f"Calculated {len(emas)} EMAs")
    
    # Test all combinations
    results = []
    total_combinations = sum(1 for f in fast_periods for s in slow_periods if f < s)
    completed = 0
    
    print(f"\nTesting {total_combinations} parameter combinations...")
    
    for fast in fast_periods:
        for slow in slow_periods:
            if fast >= slow:
                continue
            
            # Run simulation with pre-calculated EMAs
            result = simulate_ema_cross_vectorized(
                prices,
                emas[fast],
                emas[slow]
            )
            
            result['fast_period'] = fast
            result['slow_period'] = slow
            results.append(result)
            
            completed += 1
            if completed % 10 == 0:
                print(f"Progress: {completed}/{total_combinations}")
    
    return results


def compare_with_transaction_costs(results, bars):
    """Add transaction cost analysis to results."""
    
    print("\n" + "="*60)
    print("TRANSACTION COST IMPACT")
    print("="*60)
    
    # Different fee scenarios
    fee_scenarios = [
        ("No fees", 0),
        ("Retail ($0.50/100 shares)", 0.50),
        ("Pro ($0.35/100 shares)", 0.35),
        ("Institutional ($0.10/100 shares)", 0.10)
    ]
    
    # Pick best strategy without fees
    best_no_fees = max(results, key=lambda x: x['pnl'])
    
    print(f"\nBest strategy (no fees): Fast={best_no_fees['fast_period']}, Slow={best_no_fees['slow_period']}")
    print(f"Base P&L: ${best_no_fees['pnl']:.2f}")
    print(f"Trades: {best_no_fees['num_trades']}")
    
    print("\nImpact of fees:")
    for name, fee_per_100 in fee_scenarios:
        # Each trade is 100 shares, entry and exit
        total_fees = best_no_fees['num_trades'] * fee_per_100 * 2  # *2 for round trip
        net_pnl = best_no_fees['pnl'] - total_fees
        net_pct = (net_pnl / 100_000) * 100
        
        print(f"{name:.<30} ${net_pnl:>8.2f} ({net_pct:>6.2f}%)")


def main():
    """Run vectorized optimization."""
    
    print("\n" + "="*60)
    print("VECTORIZED OPTIMIZATION (MOST EFFICIENT)")
    print("="*60)
    
    # Load data
    catalog_path = Path.cwd() / "catalog"
    catalog = ParquetDataCatalog(catalog_path)
    
    bars = catalog.query(
        data_cls=Bar,
        identifiers=["NVDA.ALPACA-1-MINUTE-LAST-EXTERNAL"],
    )
    
    print(f"\nLoaded {len(bars):,} bars")
    
    # Parameters to test
    fast_periods = list(range(5, 35, 5))  # 5, 10, 15, 20, 25, 30
    slow_periods = list(range(20, 65, 5))  # 20, 25, 30, ..., 60
    
    # Time the optimization
    start_time = time.time()
    results = optimize_vectorized(bars, fast_periods, slow_periods)
    elapsed = time.time() - start_time
    
    # Convert to DataFrame
    df = pd.DataFrame(results)
    df = df.drop('trades', axis=1)  # Remove detailed trades for display
    df_sorted = df.sort_values('pnl_pct', ascending=False)
    
    # Show results
    print("\n" + "="*60)
    print("TOP 10 RESULTS")
    print("="*60)
    
    print("\n{:<6} {:<6} {:<8} {:<8} {:<8} {:<10} {:<10}".format(
        "Fast", "Slow", "P&L %", "Trades", "Win %", "Avg Trade", "Max DD"
    ))
    print("-" * 70)
    
    for _, row in df_sorted.head(10).iterrows():
        print("{:<6} {:<6} {:<7.2f}% {:<8} {:<7.1f}% ${:<9.2f} ${:<9.0f}".format(
            row['fast_period'],
            row['slow_period'],
            row['pnl_pct'],
            row['num_trades'],
            row['win_rate'],
            row['avg_trade'],
            row['max_drawdown']
        ))
    
    # Performance analysis
    print("\n" + "="*60)
    print("PERFORMANCE ANALYSIS")
    print("="*60)
    
    print(f"\nOptimization completed in: {elapsed:.2f}s")
    print(f"Parameter combinations tested: {len(results)}")
    print(f"Time per combination: {elapsed/len(results)*1000:.1f}ms")
    
    # Data efficiency
    total_bars = len(bars)
    print(f"\nData efficiency:")
    print(f"  - Bars loaded: {total_bars:,} (only once!)")
    print(f"  - Traditional approach would load: {total_bars * len(results):,}")
    print(f"  - Efficiency gain: {len(results)}x")
    
    # Transaction cost analysis
    compare_with_transaction_costs(results, bars)
    
    # Strategy insights
    print("\n" + "="*60)
    print("INSIGHTS")
    print("="*60)
    
    avg_pnl = df['pnl_pct'].mean()
    profitable = len(df[df['pnl_pct'] > 0])
    
    print(f"\nOverall performance:")
    print(f"  - Average P&L: {avg_pnl:.2f}%")
    print(f"  - Profitable combinations: {profitable}/{len(df)} ({profitable/len(df)*100:.1f}%)")
    
    # Best by trade count
    df_low_trades = df[df['num_trades'] < 1000].sort_values('pnl_pct', ascending=False)
    if not df_low_trades.empty:
        best_low_freq = df_low_trades.iloc[0]
        print(f"\nBest low-frequency strategy (<1000 trades):")
        print(f"  - Fast={best_low_freq['fast_period']}, Slow={best_low_freq['slow_period']}")
        print(f"  - P&L: {best_low_freq['pnl_pct']:.2f}% with {best_low_freq['num_trades']} trades")
    
    print("\n💡 Key advantages of vectorized approach:")
    print("   1. Calculate indicators only once")
    print("   2. No backtest engine overhead") 
    print("   3. Pure numpy operations (C speed)")
    print("   4. Easy to add transaction costs")
    print("   5. Can test 1000s of parameters in seconds")
    
    # Save results
    df_sorted.to_csv("optimization_results_vectorized.csv", index=False)
    print(f"\nResults saved to: optimization_results_vectorized.csv")


if __name__ == "__main__":
    main()
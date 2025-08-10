#!/usr/bin/env python3
"""
Analyze saved signal traces from Parquet file.
"""

import pandas as pd
import numpy as np
from pathlib import Path


def analyze_signal_traces(filepath):
    """Analyze signal traces from Parquet file."""
    
    print("\n" + "="*60)
    print("SIGNAL TRACE ANALYSIS")
    print("="*60)
    
    # Load signals
    df = pd.read_parquet(filepath)
    print(f"\nLoaded {len(df):,} signals from {filepath}")
    
    # Basic info
    print(f"\nDate range: {df['datetime'].min()} to {df['datetime'].max()}")
    print(f"Symbol: {df['symbol'].iloc[0]}")
    print(f"Strategy: {df['strategy_id'].iloc[0]}")
    
    # Signal distribution
    print("\n" + "-"*40)
    print("SIGNAL DISTRIBUTION")
    print("-"*40)
    
    signal_counts = df['signal'].value_counts().sort_index()
    for signal, count in signal_counts.items():
        pct = count / len(df) * 100
        signal_name = {-1: "Short", 0: "Neutral", 1: "Long"}.get(signal, "Unknown")
        print(f"{signal_name}: {count:,} ({pct:.1f}%)")
    
    # Position analysis
    print("\n" + "-"*40)
    print("POSITION ANALYSIS")
    print("-"*40)
    
    position_counts = df['position_side'].value_counts()
    for side, count in position_counts.items():
        pct = count / len(df) * 100
        print(f"{side}: {count:,} ({pct:.1f}%)")
    
    # Signal changes (entries/exits)
    signal_changes = df[df['signal'] != df['signal'].shift()].copy()
    print(f"\nTotal signal changes: {len(signal_changes)}")
    
    # Calculate trade durations
    if len(signal_changes) > 1:
        durations = []
        for i in range(len(signal_changes) - 1):
            duration = signal_changes.iloc[i+1]['timestamp'] - signal_changes.iloc[i]['timestamp']
            durations.append(duration / 1e9 / 60)  # Convert to minutes
        
        print(f"\nAverage signal duration: {np.mean(durations):.1f} minutes")
        print(f"Min duration: {np.min(durations):.1f} minutes")
        print(f"Max duration: {np.max(durations):.1f} minutes")
    
    # EMA analysis
    print("\n" + "-"*40)
    print("EMA ANALYSIS")
    print("-"*40)
    
    df['ema_spread'] = df['fast_ema'] - df['slow_ema']
    df['ema_spread_pct'] = (df['ema_spread'] / df['slow_ema']) * 100
    
    print(f"\nEMA Spread Statistics:")
    print(f"Mean: ${df['ema_spread'].mean():.3f} ({df['ema_spread_pct'].mean():.3f}%)")
    print(f"Std: ${df['ema_spread'].std():.3f} ({df['ema_spread_pct'].std():.3f}%)")
    print(f"Max: ${df['ema_spread'].max():.3f} ({df['ema_spread_pct'].max():.3f}%)")
    print(f"Min: ${df['ema_spread'].min():.3f} ({df['ema_spread_pct'].min():.3f}%)")
    
    # Find crossover points
    crossovers = df[
        ((df['signal'] == 1) & (df['signal'].shift() == -1)) |
        ((df['signal'] == -1) & (df['signal'].shift() == 1))
    ]
    
    print(f"\nEMA Crossovers: {len(crossovers)}")
    
    # Signal quality metrics
    print("\n" + "-"*40)
    print("SIGNAL QUALITY")
    print("-"*40)
    
    # Calculate signal strength (absolute EMA spread at signal change)
    signal_changes['signal_strength'] = signal_changes['ema_diff'].abs()
    
    print(f"\nSignal Strength at Changes:")
    print(f"Mean: ${signal_changes['signal_strength'].mean():.3f}")
    print(f"Median: ${signal_changes['signal_strength'].median():.3f}")
    
    # Strong vs weak signals
    threshold = signal_changes['signal_strength'].median()
    strong_signals = signal_changes[signal_changes['signal_strength'] > threshold]
    weak_signals = signal_changes[signal_changes['signal_strength'] <= threshold]
    
    print(f"\nStrong signals (>${threshold:.3f}): {len(strong_signals)}")
    print(f"Weak signals (<={threshold:.3f}): {len(weak_signals)}")
    
    # P&L Analysis (if available)
    if 'unrealized_pnl' in df.columns:
        print("\n" + "-"*40)
        print("P&L ANALYSIS")
        print("-"*40)
        
        # Find max drawdown
        cumulative_pnl = df['unrealized_pnl'].cumsum()
        running_max = cumulative_pnl.cummax()
        drawdown = cumulative_pnl - running_max
        max_drawdown = drawdown.min()
        
        print(f"\nMax Drawdown: ${max_drawdown:.2f}")
        
        # Best and worst positions
        max_pnl_idx = df['unrealized_pnl'].idxmax()
        min_pnl_idx = df['unrealized_pnl'].idxmin()
        
        if not pd.isna(max_pnl_idx):
            print(f"\nBest unrealized P&L: ${df.loc[max_pnl_idx, 'unrealized_pnl']:.2f} "
                  f"at {df.loc[max_pnl_idx, 'datetime']}")
        
        if not pd.isna(min_pnl_idx):
            print(f"Worst unrealized P&L: ${df.loc[min_pnl_idx, 'unrealized_pnl']:.2f} "
                  f"at {df.loc[min_pnl_idx, 'datetime']}")
    
    # Export key signals
    print("\n" + "-"*40)
    print("EXPORTING KEY SIGNALS")
    print("-"*40)
    
    # Save signal changes to CSV for further analysis
    output_dir = Path("signal_traces")
    signal_changes_file = output_dir / "signal_changes.csv"
    
    export_df = signal_changes[['datetime', 'close', 'fast_ema', 'slow_ema', 
                               'signal', 'position_side', 'signal_strength']]
    export_df.to_csv(signal_changes_file, index=False)
    
    print(f"\nExported {len(export_df)} signal changes to: {signal_changes_file}")
    
    # Create summary report
    summary = {
        'Total Signals': len(df),
        'Date Range': f"{df['datetime'].min()} to {df['datetime'].max()}",
        'Long Signals': len(df[df['signal'] == 1]),
        'Short Signals': len(df[df['signal'] == -1]),
        'Signal Changes': len(signal_changes),
        'Avg Signal Duration (min)': np.mean(durations) if 'durations' in locals() else 'N/A',
        'EMA Crossovers': len(crossovers),
        'Mean EMA Spread': f"${df['ema_spread'].mean():.3f}",
        'Max Drawdown': f"${max_drawdown:.2f}" if 'max_drawdown' in locals() else 'N/A',
    }
    
    summary_file = output_dir / "signal_summary.txt"
    with open(summary_file, 'w') as f:
        for key, value in summary.items():
            f.write(f"{key}: {value}\n")
    
    print(f"\nSaved summary to: {summary_file}")
    
    return df


def create_signal_chart_data(df):
    """Create data suitable for charting."""
    
    print("\n" + "-"*40)
    print("CREATING CHART DATA")
    print("-"*40)
    
    # Prepare data for visualization
    chart_data = df[['datetime', 'close', 'fast_ema', 'slow_ema', 'signal']].copy()
    
    # Add signal markers
    chart_data['buy_signal'] = np.where(
        (df['signal'] == 1) & (df['signal'].shift() != 1), 
        df['close'], 
        np.nan
    )
    
    chart_data['sell_signal'] = np.where(
        (df['signal'] == -1) & (df['signal'].shift() != -1), 
        df['close'], 
        np.nan
    )
    
    # Save for plotting
    chart_file = Path("signal_traces") / "chart_data.csv"
    chart_data.to_csv(chart_file, index=False)
    
    print(f"Saved chart data to: {chart_file}")
    print("Use this data to plot:")
    print("  - Price line")
    print("  - Fast EMA line")
    print("  - Slow EMA line")
    print("  - Buy signals (green arrows)")
    print("  - Sell signals (red arrows)")


if __name__ == "__main__":
    # Analyze the demo signals
    signal_file = Path("signal_traces") / "nvda_signals_demo.parquet"
    
    if signal_file.exists():
        df = analyze_signal_traces(signal_file)
        create_signal_chart_data(df)
    else:
        print(f"Signal file not found: {signal_file}")
        print("Run simple_signal_recording.py first to generate signals")
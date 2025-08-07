#!/usr/bin/env python3
"""
Run backtest and save signal traces to Parquet.
"""

from decimal import Decimal
from pathlib import Path
import pandas as pd

from nautilus_trader.backtest.engine import BacktestEngine, BacktestEngineConfig
from nautilus_trader.config import LoggingConfig
from nautilus_trader.model.currencies import USD
from nautilus_trader.model.data import Bar, BarType
from nautilus_trader.model.enums import AccountType, OmsType
from nautilus_trader.model.identifiers import InstrumentId, Symbol, Venue
from nautilus_trader.model.instruments import Equity
from nautilus_trader.model.objects import Money, Price, Quantity
from nautilus_trader.persistence.catalog import ParquetDataCatalog

from strategy_with_signals import EMACrossWithSignals, EMACrossConfig


def run_backtest_with_signal_recording():
    """Run backtest and save signal traces."""
    
    print("\n" + "="*60)
    print("BACKTEST WITH SIGNAL RECORDING")
    print("="*60)
    
    # Load data
    catalog_path = Path.cwd() / "catalog"
    catalog = ParquetDataCatalog(catalog_path)
    
    bars = catalog.query(
        data_cls=Bar,
        identifiers=["NVDA.ALPACA-1-MINUTE-LAST-EXTERNAL"],
    )[:5000]  # Use 5000 bars for demo
    
    print(f"\nLoaded {len(bars):,} bars")
    
    # Setup
    venue = Venue("ALPACA")
    instrument_id = InstrumentId(Symbol("NVDA"), venue)
    bar_type = BarType.from_str("NVDA.ALPACA-1-MINUTE-LAST-EXTERNAL")
    
    # Configure engine
    config = BacktestEngineConfig(
        trader_id="SIGNAL-001",
        logging=LoggingConfig(log_level="INFO"),
    )
    
    engine = BacktestEngine(config=config)
    
    # Add venue
    engine.add_venue(
        venue=venue,
        oms_type=OmsType.HEDGING,
        account_type=AccountType.MARGIN,
        base_currency=USD,
        starting_balances=[Money(100_000, USD)],
    )
    
    # Create instrument
    instrument = Equity(
        instrument_id=instrument_id,
        raw_symbol=Symbol("NVDA"),
        currency=USD,
        price_precision=2,
        price_increment=Price(0.01, 2),
        lot_size=Quantity.from_int(1),
        isin=None,
        ts_event=0,
        ts_init=0,
    )
    
    engine.add_instrument(instrument)
    engine.add_data(bars)
    
    # Configure strategy
    strategy_config = EMACrossConfig(
        strategy_id="EMA-10-30",
        instrument_id=instrument_id,
        bar_type=bar_type,
        fast_ema_period=10,
        slow_ema_period=30,
        trade_size=Decimal(100),
        request_bars=False,
        subscribe_trade_ticks=False,
        subscribe_quote_ticks=False,
    )
    
    strategy = EMACrossWithSignals(config=strategy_config)
    engine.add_strategy(strategy)
    
    # Run backtest
    print("\nRunning backtest...")
    engine.run()
    
    # Save signal traces
    output_dir = Path("signal_traces")
    output_dir.mkdir(exist_ok=True)
    
    filepath = output_dir / "nvda_ema_signals.parquet"
    strategy.save_signal_traces(str(filepath))
    
    # Display results
    print("\n" + "="*60)
    print("BACKTEST RESULTS")
    print("="*60)
    
    account = engine.cache.accounts()[0]
    positions = engine.cache.positions_closed()
    
    print(f"\nFinal balance: ${float(account.balance_total(USD)):,.2f}")
    print(f"Total P&L: ${float(account.balance_total(USD)) - 100_000:,.2f}")
    print(f"Total trades: {len(positions)}")
    
    # Load and analyze signal traces
    print("\n" + "="*60)
    print("SIGNAL TRACE ANALYSIS")
    print("="*60)
    
    df = pd.read_parquet(filepath)
    
    print(f"\nTotal signals recorded: {len(df):,}")
    print(f"Date range: {df['datetime'].min()} to {df['datetime'].max()}")
    
    # Signal distribution
    signal_counts = df['signal'].value_counts().sort_index()
    print("\nSignal distribution:")
    for signal, count in signal_counts.items():
        signal_name = {-1: "Short", 0: "Neutral", 1: "Long"}.get(signal, "Unknown")
        print(f"  {signal_name}: {count:,} ({count/len(df)*100:.1f}%)")
    
    # Position changes
    position_changes = df[df['signal'] != df['signal'].shift()].copy()
    print(f"\nPosition changes: {len(position_changes)}")
    
    # Average time in position
    if len(position_changes) > 1:
        time_diffs = position_changes['datetime'].diff().dropna()
        avg_time = time_diffs.mean()
        print(f"Average time between signals: {avg_time}")
    
    # EMA spread statistics
    print("\nEMA spread statistics:")
    print(f"  Mean diff: {df['ema_diff'].mean():.2f}")
    print(f"  Std diff: {df['ema_diff'].std():.2f}")
    print(f"  Max diff: {df['ema_diff'].max():.2f}")
    print(f"  Min diff: {df['ema_diff'].min():.2f}")
    
    # Save additional analytics
    analytics_file = output_dir / "signal_analytics.csv"
    
    # Create analytics DataFrame
    analytics = pd.DataFrame({
        'timestamp': position_changes.index,
        'datetime': position_changes['datetime'],
        'price': position_changes['close'],
        'signal': position_changes['signal'],
        'fast_ema': position_changes['fast_ema'],
        'slow_ema': position_changes['slow_ema'],
        'ema_diff_pct': position_changes['ema_diff_pct'],
    })
    
    analytics.to_csv(analytics_file, index=False)
    print(f"\nSaved signal analytics to: {analytics_file}")
    
    print("\n" + "="*60)
    print("FILES CREATED")
    print("="*60)
    print(f"1. Signal traces: {filepath}")
    print(f"2. Summary: {filepath.with_suffix('.txt').name.replace('.txt', '_summary.txt')}")
    print(f"3. Analytics: {analytics_file}")
    
    return filepath


def analyze_signals_advanced(filepath):
    """Advanced analysis of signal traces."""
    
    print("\n" + "="*60)
    print("ADVANCED SIGNAL ANALYSIS")
    print("="*60)
    
    df = pd.read_parquet(filepath)
    
    # Create visualizable data
    viz_data = pd.DataFrame({
        'datetime': df['datetime'],
        'close': df['close'],
        'fast_ema': df['fast_ema'],
        'slow_ema': df['slow_ema'],
        'signal': df['signal'],
        'position': df['position'],
        'unrealized_pnl': df['unrealized_pnl'],
    })
    
    # Find best and worst signals
    signal_changes = df[df['signal'] != df['signal'].shift()].copy()
    
    if len(signal_changes) > 1:
        # Calculate P&L for each signal
        signal_pnls = []
        
        for i in range(len(signal_changes) - 1):
            entry = signal_changes.iloc[i]
            exit = signal_changes.iloc[i + 1]
            
            if entry['signal'] == 1:  # Long
                pnl = exit['close'] - entry['close']
            elif entry['signal'] == -1:  # Short
                pnl = entry['close'] - exit['close']
            else:
                pnl = 0
                
            signal_pnls.append({
                'entry_time': entry['datetime'],
                'exit_time': exit['datetime'],
                'entry_price': entry['close'],
                'exit_price': exit['close'],
                'signal': entry['signal'],
                'pnl': pnl,
                'pnl_pct': pnl / entry['close'] * 100,
                'duration': exit['datetime'] - entry['datetime'],
            })
        
        pnl_df = pd.DataFrame(signal_pnls)
        
        print("\nTop 5 winning signals:")
        for _, row in pnl_df.nlargest(5, 'pnl').iterrows():
            print(f"  {row['entry_time']} -> {row['exit_time']}: "
                  f"${row['pnl']:.2f} ({row['pnl_pct']:.2f}%)")
        
        print("\nTop 5 losing signals:")
        for _, row in pnl_df.nsmallest(5, 'pnl').iterrows():
            print(f"  {row['entry_time']} -> {row['exit_time']}: "
                  f"${row['pnl']:.2f} ({row['pnl_pct']:.2f}%)")
    
    # Signal quality metrics
    print("\nSignal quality metrics:")
    
    # How often does signal change lead to immediate profit?
    immediate_profit = 0
    for i in range(len(signal_changes) - 1):
        curr = signal_changes.iloc[i]
        
        # Look ahead 5 bars
        future_idx = df.index.get_loc(signal_changes.index[i])
        if future_idx + 5 < len(df):
            future_prices = df.iloc[future_idx:future_idx+5]['close']
            
            if curr['signal'] == 1:  # Long signal
                if any(future_prices > curr['close']):
                    immediate_profit += 1
            elif curr['signal'] == -1:  # Short signal
                if any(future_prices < curr['close']):
                    immediate_profit += 1
    
    if len(signal_changes) > 0:
        print(f"  Signals with immediate profit (5 bars): "
              f"{immediate_profit}/{len(signal_changes)-1} "
              f"({immediate_profit/(len(signal_changes)-1)*100:.1f}%)")


if __name__ == "__main__":
    # Run backtest and save signals
    filepath = run_backtest_with_signal_recording()
    
    # Perform advanced analysis
    analyze_signals_advanced(filepath)
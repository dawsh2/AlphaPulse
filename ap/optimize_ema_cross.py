#!/usr/bin/env python3
"""
Grid search optimization for EMA Cross strategy parameters.
"""

from decimal import Decimal
from pathlib import Path
from datetime import datetime
import itertools
import pandas as pd
from concurrent.futures import ProcessPoolExecutor, as_completed
import multiprocessing

from nautilus_trader.backtest.engine import BacktestEngine, BacktestEngineConfig
from nautilus_trader.config import LoggingConfig
from nautilus_trader.model.currencies import USD
from nautilus_trader.model.data import Bar, BarType
from nautilus_trader.model.enums import AccountType, OmsType
from nautilus_trader.model.identifiers import InstrumentId, Symbol, Venue
from nautilus_trader.model.instruments import Equity
from nautilus_trader.model.objects import Money, Price, Quantity
from nautilus_trader.persistence.catalog import ParquetDataCatalog

import sys
sys.path.insert(0, str(Path(__file__).parent / "nt_reference"))
from examples.strategies.ema_cross import EMACross, EMACrossConfig


def run_single_backtest(params):
    """Run a single backtest with given parameters."""
    fast_period, slow_period, trade_size = params
    
    # Skip invalid combinations
    if fast_period >= slow_period:
        return None
    
    # Load data
    catalog_path = Path.cwd() / "catalog"
    catalog = ParquetDataCatalog(catalog_path)
    
    bar_type_str = "NVDA.ALPACA-1-MINUTE-LAST-EXTERNAL"
    bars = catalog.query(
        data_cls=Bar,
        identifiers=[bar_type_str],
    )
    
    if not bars:
        return None
    
    # Setup
    venue = Venue("ALPACA")
    instrument_id = InstrumentId(Symbol("NVDA"), venue)
    bar_type = BarType.from_str(bar_type_str)
    
    # Configure engine
    config = BacktestEngineConfig(
        trader_id=f"OPTIMIZER-{fast_period}-{slow_period}",
        logging=LoggingConfig(log_level="ERROR"),  # Quiet
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
    
    # Get or create instrument
    instruments = list(catalog.instruments())
    instrument = None
    for inst in instruments:
        if inst.id == instrument_id:
            instrument = inst
            break
    
    if not instrument:
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
        instrument_id=instrument_id,
        bar_type=bar_type,
        fast_ema_period=fast_period,
        slow_ema_period=slow_period,
        trade_size=Decimal(trade_size),
        request_bars=False,
        subscribe_trade_ticks=False,
        subscribe_quote_ticks=False,
    )
    
    strategy = EMACross(config=strategy_config)
    engine.add_strategy(strategy)
    
    # Run backtest
    engine.run()
    
    # Get results
    account = engine.cache.accounts()[0]
    positions = engine.cache.positions_closed()
    
    final_balance = float(account.balance_total(USD))
    pnl = final_balance - 100_000
    pnl_pct = (pnl / 100_000) * 100
    
    # Calculate metrics
    num_trades = len(positions)
    if num_trades > 0:
        winners = [p for p in positions if p.realized_pnl.as_double() > 0]
        win_rate = len(winners) / num_trades * 100
        
        pnls = [p.realized_pnl.as_double() for p in positions]
        avg_trade = sum(pnls) / len(pnls) if pnls else 0
        
        # Calculate max drawdown (simplified)
        cumulative_pnl = 0
        peak_pnl = 0
        max_drawdown = 0
        
        for p in positions:
            cumulative_pnl += p.realized_pnl.as_double()
            peak_pnl = max(peak_pnl, cumulative_pnl)
            drawdown = peak_pnl - cumulative_pnl
            max_drawdown = max(max_drawdown, drawdown)
    else:
        win_rate = 0
        avg_trade = 0
        max_drawdown = 0
    
    return {
        'fast_period': fast_period,
        'slow_period': slow_period,
        'trade_size': trade_size,
        'final_balance': final_balance,
        'pnl': pnl,
        'pnl_pct': pnl_pct,
        'num_trades': num_trades,
        'win_rate': win_rate,
        'avg_trade': avg_trade,
        'max_drawdown': max_drawdown,
        'sharpe_ratio': pnl / max_drawdown if max_drawdown > 0 else 0
    }


def main():
    """Run grid search optimization."""
    
    print("\n" + "="*60)
    print("EMA CROSS PARAMETER OPTIMIZATION")
    print("="*60)
    
    # Define parameter ranges
    fast_periods = [5, 10, 15, 20, 25, 30]
    slow_periods = [10, 20, 30, 40, 50, 60]
    trade_sizes = [100]  # Keep constant for now
    
    # Create all combinations
    param_combinations = list(itertools.product(fast_periods, slow_periods, trade_sizes))
    
    # Filter valid combinations (fast < slow)
    valid_combinations = [(f, s, t) for f, s, t in param_combinations if f < s]
    
    print(f"\nTesting {len(valid_combinations)} parameter combinations...")
    print(f"Fast EMA periods: {fast_periods}")
    print(f"Slow EMA periods: {slow_periods}")
    print(f"Trade sizes: {trade_sizes}")
    
    # Run backtests in parallel
    results = []
    max_workers = min(multiprocessing.cpu_count() - 1, 8)
    
    print(f"\nRunning backtests with {max_workers} parallel workers...")
    
    with ProcessPoolExecutor(max_workers=max_workers) as executor:
        # Submit all tasks
        future_to_params = {executor.submit(run_single_backtest, params): params 
                           for params in valid_combinations}
        
        # Process completed tasks
        completed = 0
        for future in as_completed(future_to_params):
            result = future.result()
            if result:
                results.append(result)
            completed += 1
            if completed % 5 == 0:
                print(f"Completed {completed}/{len(valid_combinations)} backtests...")
    
    # Convert to DataFrame for analysis
    df = pd.DataFrame(results)
    
    # Sort by PnL
    df_sorted = df.sort_values('pnl_pct', ascending=False)
    
    print("\n" + "="*60)
    print("TOP 10 PARAMETER COMBINATIONS BY P&L")
    print("="*60)
    
    print("\n{:<6} {:<6} {:<10} {:<10} {:<8} {:<8} {:<8} {:<8}".format(
        "Fast", "Slow", "Final Bal", "P&L %", "Trades", "Win %", "Avg Trade", "Max DD"
    ))
    print("-" * 80)
    
    for _, row in df_sorted.head(10).iterrows():
        print("{:<6} {:<6} ${:<9,.0f} {:<9.2f}% {:<8} {:<7.1f}% ${:<8.2f} ${:<8.0f}".format(
            row['fast_period'],
            row['slow_period'],
            row['final_balance'],
            row['pnl_pct'],
            row['num_trades'],
            row['win_rate'],
            row['avg_trade'],
            row['max_drawdown']
        ))
    
    print("\n" + "="*60)
    print("WORST 5 PARAMETER COMBINATIONS")
    print("="*60)
    
    for _, row in df_sorted.tail(5).iterrows():
        print("{:<6} {:<6} ${:<9,.0f} {:<9.2f}% {:<8} {:<7.1f}%".format(
            row['fast_period'],
            row['slow_period'],
            row['final_balance'],
            row['pnl_pct'],
            row['num_trades'],
            row['win_rate']
        ))
    
    # Save full results
    output_file = "optimization_results.csv"
    df_sorted.to_csv(output_file, index=False)
    print(f"\nFull results saved to: {output_file}")
    
    # Analysis
    print("\n" + "="*60)
    print("ANALYSIS")
    print("="*60)
    
    best_params = df_sorted.iloc[0]
    print(f"\nBest Parameters:")
    print(f"  Fast EMA: {best_params['fast_period']}")
    print(f"  Slow EMA: {best_params['slow_period']}")
    print(f"  P&L: ${best_params['pnl']:.2f} ({best_params['pnl_pct']:.2f}%)")
    print(f"  Trades: {best_params['num_trades']}")
    print(f"  Win Rate: {best_params['win_rate']:.1f}%")
    
    # Look for patterns
    avg_by_fast = df.groupby('fast_period')['pnl_pct'].mean()
    avg_by_slow = df.groupby('slow_period')['pnl_pct'].mean()
    
    print(f"\nAverage P&L by Fast Period:")
    for period, avg_pnl in avg_by_fast.items():
        print(f"  {period}: {avg_pnl:.2f}%")
    
    print(f"\nAverage P&L by Slow Period:")
    for period, avg_pnl in avg_by_slow.items():
        print(f"  {period}: {avg_pnl:.2f}%")
    
    print("\n" + "="*60)


if __name__ == "__main__":
    import warnings
    warnings.filterwarnings('ignore')
    main()
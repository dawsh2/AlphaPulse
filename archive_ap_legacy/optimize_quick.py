#!/usr/bin/env python3
"""
Quick optimization test with fewer parameters to see results faster.
"""

from decimal import Decimal
from pathlib import Path
from datetime import datetime
import pandas as pd
import time

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


def run_single_backtest(fast_period, slow_period, bars):
    """Run a single backtest with given parameters."""
    
    # Skip invalid combinations
    if fast_period >= slow_period:
        return None
    
    # Setup
    venue = Venue("ALPACA")
    instrument_id = InstrumentId(Symbol("NVDA"), venue)
    bar_type = BarType.from_str("NVDA.ALPACA-1-MINUTE-LAST-EXTERNAL")
    
    # Configure engine
    config = BacktestEngineConfig(
        trader_id=f"OPTIMIZER-{fast_period}-{slow_period}",
        logging=LoggingConfig(log_level="ERROR"),
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
        instrument_id=instrument_id,
        bar_type=bar_type,
        fast_ema_period=fast_period,
        slow_ema_period=slow_period,
        trade_size=Decimal(100),
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
    else:
        win_rate = 0
        avg_trade = 0
    
    return {
        'fast_period': fast_period,
        'slow_period': slow_period,
        'final_balance': final_balance,
        'pnl': pnl,
        'pnl_pct': pnl_pct,
        'num_trades': num_trades,
        'win_rate': win_rate,
        'avg_trade': avg_trade
    }


def main():
    """Run quick optimization."""
    
    print("\n" + "="*60)
    print("QUICK EMA CROSS OPTIMIZATION (6 MONTHS DATA)")
    print("="*60)
    
    # Load data
    catalog_path = Path.cwd() / "catalog"
    catalog = ParquetDataCatalog(catalog_path)
    
    bar_type_str = "NVDA.ALPACA-1-MINUTE-LAST-EXTERNAL"
    bars = catalog.query(
        data_cls=Bar,
        identifiers=[bar_type_str],
    )
    
    print(f"\nLoaded {len(bars):,} bars")
    
    # Show date range
    first_date = datetime.fromtimestamp(bars[0].ts_event / 1e9)
    last_date = datetime.fromtimestamp(bars[-1].ts_event / 1e9)
    print(f"Date range: {first_date.date()} to {last_date.date()}")
    print(f"Days: {(last_date - first_date).days}")
    
    # Test fewer parameter combinations for speed
    fast_periods = [10, 20, 30]
    slow_periods = [20, 40, 60]
    
    print(f"\nTesting parameters:")
    print(f"Fast EMA: {fast_periods}")
    print(f"Slow EMA: {slow_periods}")
    
    # Run backtests
    results = []
    combinations = [(f, s) for f in fast_periods for s in slow_periods if f < s]
    
    print(f"\nRunning {len(combinations)} backtests...")
    
    for i, (fast, slow) in enumerate(combinations):
        print(f"\rProgress: {i+1}/{len(combinations)} - Testing Fast={fast}, Slow={slow}", end="")
        
        start_time = time.time()
        result = run_single_backtest(fast, slow, bars)
        elapsed = time.time() - start_time
        
        if result:
            results.append(result)
            print(f" -> P&L: {result['pnl_pct']:.2f}% ({elapsed:.1f}s)")
    
    print("\n")
    
    # Convert to DataFrame
    df = pd.DataFrame(results)
    df_sorted = df.sort_values('pnl_pct', ascending=False)
    
    # Show results
    print("="*60)
    print("RESULTS (SORTED BY P&L)")
    print("="*60)
    
    print("\n{:<6} {:<6} {:<12} {:<8} {:<8} {:<8} {:<10}".format(
        "Fast", "Slow", "Final Bal", "P&L %", "Trades", "Win %", "Avg Trade"
    ))
    print("-" * 70)
    
    for _, row in df_sorted.iterrows():
        print("{:<6} {:<6} ${:<11,.0f} {:<7.2f}% {:<8} {:<7.1f}% ${:<9.2f}".format(
            row['fast_period'],
            row['slow_period'],
            row['final_balance'],
            row['pnl_pct'],
            row['num_trades'],
            row['win_rate'],
            row['avg_trade']
        ))
    
    # Analysis
    print("\n" + "="*60)
    print("ANALYSIS")
    print("="*60)
    
    best = df_sorted.iloc[0]
    worst = df_sorted.iloc[-1]
    
    print(f"\nBest Parameters:")
    print(f"  Fast={best['fast_period']}, Slow={best['slow_period']}")
    print(f"  P&L: ${best['pnl']:.2f} ({best['pnl_pct']:.2f}%)")
    print(f"  Trades: {best['num_trades']}")
    print(f"  Win Rate: {best['win_rate']:.1f}%")
    
    print(f"\nWorst Parameters:")
    print(f"  Fast={worst['fast_period']}, Slow={worst['slow_period']}")
    print(f"  P&L: ${worst['pnl']:.2f} ({worst['pnl_pct']:.2f}%)")
    
    # Overall observation
    avg_pnl = df['pnl_pct'].mean()
    positive_count = len(df[df['pnl_pct'] > 0])
    
    print(f"\nOverall:")
    print(f"  Average P&L: {avg_pnl:.2f}%")
    print(f"  Profitable combinations: {positive_count}/{len(df)}")
    
    if avg_pnl < 0:
        print("\n⚠️  WARNING: EMA Cross strategy shows poor performance on NVDA")
        print("   Consider testing other strategies or adding filters")
    
    print("\n" + "="*60)


if __name__ == "__main__":
    main()
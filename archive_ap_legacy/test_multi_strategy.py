#!/usr/bin/env python3
"""
Test multi-strategy optimization with just a few parameters.
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


def run_comparison():
    """Compare single vs multi-strategy approach."""
    
    print("\n" + "="*60)
    print("COMPARING OPTIMIZATION APPROACHES")
    print("="*60)
    
    # Load data (use only 1 week for speed)
    catalog_path = Path.cwd() / "catalog"
    catalog = ParquetDataCatalog(catalog_path)
    
    bars_all = catalog.query(
        data_cls=Bar,
        identifiers=["NVDA.ALPACA-1-MINUTE-LAST-EXTERNAL"],
    )
    
    # Use only 1 week of data for quick test
    bars = bars_all[:7*390]  # ~7 trading days
    
    print(f"\nUsing {len(bars):,} bars (1 week) for speed")
    
    # Test parameters
    param_combinations = [(10, 20), (15, 30), (20, 40)]
    
    print(f"\nTesting {len(param_combinations)} parameter combinations:")
    for f, s in param_combinations:
        print(f"  Fast={f}, Slow={s}")
    
    # Setup common elements
    venue = Venue("ALPACA")
    instrument_id = InstrumentId(Symbol("NVDA"), venue)
    bar_type = BarType.from_str("NVDA.ALPACA-1-MINUTE-LAST-EXTERNAL")
    
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
    
    # 1. Traditional approach - separate backtests
    print("\n" + "-"*60)
    print("TRADITIONAL APPROACH (Separate Backtests)")
    print("-"*60)
    
    traditional_start = time.time()
    traditional_results = []
    
    for fast, slow in param_combinations:
        # Create new engine for each
        config = BacktestEngineConfig(
            trader_id=f"TRAD-{fast}-{slow}",
            logging=LoggingConfig(log_level="ERROR"),
        )
        
        engine = BacktestEngine(config=config)
        
        engine.add_venue(
            venue=venue,
            oms_type=OmsType.HEDGING,
            account_type=AccountType.MARGIN,
            base_currency=USD,
            starting_balances=[Money(100_000, USD)],
        )
        
        engine.add_instrument(instrument)
        engine.add_data(bars)
        
        # Add strategy
        strategy_config = EMACrossConfig(
            strategy_id=f"EMA-{fast}-{slow}",
            instrument_id=instrument_id,
            bar_type=bar_type,
            fast_ema_period=fast,
            slow_ema_period=slow,
            trade_size=Decimal(100),
            request_bars=False,
            subscribe_trade_ticks=False,
            subscribe_quote_ticks=False,
        )
        
        strategy = EMACross(config=strategy_config)
        engine.add_strategy(strategy)
        
        # Run
        start = time.time()
        engine.run()
        elapsed = time.time() - start
        
        # Get results
        positions = engine.cache.positions_closed()
        account = engine.cache.accounts()[0]
        pnl = float(account.balance_total(USD)) - 100_000
        
        print(f"  Fast={fast}, Slow={slow}: {len(positions)} trades, "
              f"P&L=${pnl:.2f}, Time={elapsed:.2f}s")
        
        traditional_results.append({
            'fast': fast,
            'slow': slow,
            'trades': len(positions),
            'pnl': pnl,
            'time': elapsed
        })
    
    traditional_total = time.time() - traditional_start
    
    # 2. Efficient approach - single backtest
    print("\n" + "-"*60)
    print("EFFICIENT APPROACH (Single Backtest)")
    print("-"*60)
    
    efficient_start = time.time()
    
    # Create ONE engine
    config = BacktestEngineConfig(
        trader_id="MULTI-001",
        logging=LoggingConfig(log_level="ERROR"),
    )
    
    engine = BacktestEngine(config=config)
    
    engine.add_venue(
        venue=venue,
        oms_type=OmsType.HEDGING,
        account_type=AccountType.MARGIN,
        base_currency=USD,
        starting_balances=[Money(100_000 * len(param_combinations), USD)],
    )
    
    engine.add_instrument(instrument)
    engine.add_data(bars)
    
    # Add ALL strategies
    strategies = {}
    for fast, slow in param_combinations:
        strategy_config = EMACrossConfig(
            strategy_id=f"EMA-{fast}-{slow}",
            instrument_id=instrument_id,
            bar_type=bar_type,
            fast_ema_period=fast,
            slow_ema_period=slow,
            trade_size=Decimal(100),
            request_bars=False,
            subscribe_trade_ticks=False,
            subscribe_quote_ticks=False,
        )
        
        strategy = EMACross(config=strategy_config)
        engine.add_strategy(strategy)
        strategies[f"{fast}-{slow}"] = strategy
    
    # Run ONCE
    print(f"  Running {len(strategies)} strategies simultaneously...")
    engine.run()
    
    efficient_total = time.time() - efficient_start
    
    # Extract results
    all_positions = engine.cache.positions_closed()
    
    for key, strategy in strategies.items():
        fast, slow = map(int, key.split('-'))
        positions = [p for p in all_positions if p.strategy_id == strategy.id]
        
        if positions:
            pnl = sum(p.realized_pnl.as_double() for p in positions)
        else:
            pnl = 0
        
        print(f"  Fast={fast}, Slow={slow}: {len(positions)} trades, P&L=${pnl:.2f}")
    
    # Summary
    print("\n" + "="*60)
    print("PERFORMANCE COMPARISON")
    print("="*60)
    
    print(f"\nTraditional approach:")
    print(f"  Total time: {traditional_total:.2f}s")
    print(f"  Bars processed: {len(bars) * len(param_combinations):,}")
    
    print(f"\nEfficient approach:")
    print(f"  Total time: {efficient_total:.2f}s")
    print(f"  Bars processed: {len(bars):,}")
    
    speedup = traditional_total / efficient_total
    print(f"\nSpeedup: {speedup:.1f}x faster!")
    print(f"Efficiency: {(1 - efficient_total/traditional_total)*100:.0f}% time saved")
    
    print("\n✅ Benefits of multi-strategy approach:")
    print("   - Data loaded and processed only once")
    print("   - All strategies share the same market replay")
    print("   - Maintains event-driven architecture")
    print("   - Scales to 50+ strategies easily")


if __name__ == "__main__":
    run_comparison()
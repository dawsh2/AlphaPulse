#!/usr/bin/env python3
"""
Fixed efficient optimization with proper result extraction.
"""

from decimal import Decimal
from pathlib import Path
from datetime import datetime
import pandas as pd
import time
from collections import defaultdict

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


def run_multi_strategy_efficient(param_combinations, bars):
    """Run multiple strategies with efficient result extraction."""
    
    # Setup
    venue = Venue("ALPACA")
    instrument_id = InstrumentId(Symbol("NVDA"), venue)
    bar_type = BarType.from_str("NVDA.ALPACA-1-MINUTE-LAST-EXTERNAL")
    
    # Configure engine
    config = BacktestEngineConfig(
        trader_id="MULTI-001",
        logging=LoggingConfig(log_level="ERROR"),
    )
    
    engine = BacktestEngine(config=config)
    
    # Add venue
    engine.add_venue(
        venue=venue,
        oms_type=OmsType.HEDGING,
        account_type=AccountType.MARGIN,
        base_currency=USD,
        starting_balances=[Money(100_000 * len(param_combinations), USD)],
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
    
    # Map to track strategy params
    strategy_map = {}  # actual_id -> (fast, slow)
    id_counter = 0
    
    # Add all strategies
    for fast, slow in param_combinations:
        if fast >= slow:
            continue
            
        # Create unique prefix for this ID counter
        prefix = f"S{id_counter:02d}"
        
        # Configure strategy
        strategy_config = EMACrossConfig(
            strategy_id=f"{prefix}-{fast}-{slow}",
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
        
        # Map the actual generated ID to parameters
        # NT will transform our ID, so we need to track what it becomes
        actual_id = str(strategy.id)
        strategy_map[actual_id] = (fast, slow)
        id_counter += 1
    
    print(f"Running {len(strategy_map)} strategies in single backtest...")
    
    # Run backtest ONCE
    start_time = time.time()
    engine.run()
    elapsed = time.time() - start_time
    
    print(f"Backtest completed in {elapsed:.1f}s")
    
    # Efficient result extraction using single pass
    positions_by_strategy = defaultdict(list)
    
    # Single pass through all positions
    for position in engine.cache.positions_closed():
        sid = str(position.strategy_id)
        positions_by_strategy[sid].append(position)
    
    # Build results
    results = []
    for actual_id, (fast, slow) in strategy_map.items():
        positions = positions_by_strategy.get(actual_id, [])
        
        if positions:
            # Calculate metrics
            total_pnl = sum(p.realized_pnl.as_double() for p in positions)
            pnl_pct = (total_pnl / 100_000) * 100
            
            winners = [p for p in positions if p.realized_pnl.as_double() > 0]
            win_rate = len(winners) / len(positions) * 100 if positions else 0
            
            avg_trade = total_pnl / len(positions) if positions else 0
        else:
            total_pnl = 0
            pnl_pct = 0
            win_rate = 0
            avg_trade = 0
        
        results.append({
            'fast_period': fast,
            'slow_period': slow,
            'pnl': total_pnl,
            'pnl_pct': pnl_pct,
            'num_trades': len(positions),
            'win_rate': win_rate,
            'avg_trade': avg_trade
        })
    
    return results, elapsed


def main():
    """Run efficient optimization with fixes."""
    
    print("\n" + "="*60)
    print("EFFICIENT MULTI-STRATEGY OPTIMIZATION (FIXED)")
    print("="*60)
    
    # Load data - use subset for testing
    catalog_path = Path.cwd() / "catalog"
    catalog = ParquetDataCatalog(catalog_path)
    
    bars_all = catalog.query(
        data_cls=Bar,
        identifiers=["NVDA.ALPACA-1-MINUTE-LAST-EXTERNAL"],
    )
    
    # Use 1 month of data for reasonable test
    bars = bars_all[:20000]  # ~1 month
    
    print(f"\nLoaded {len(bars):,} bars (~1 month)")
    
    # Test with moderate number of parameters
    fast_periods = [5, 10, 15, 20, 25]
    slow_periods = [20, 30, 40, 50]
    
    param_combinations = [(f, s) for f in fast_periods for s in slow_periods]
    valid_combinations = [(f, s) for f, s in param_combinations if f < s]
    
    print(f"\nTesting {len(valid_combinations)} parameter combinations")
    
    # Compare approaches
    print("\n" + "-"*60)
    print("APPROACH COMPARISON")
    print("-"*60)
    
    # 1. Time traditional approach (just 3 for comparison)
    print("\nTraditional approach (3 backtests):")
    trad_start = time.time()
    
    for i, (fast, slow) in enumerate(valid_combinations[:3]):
        bt_start = time.time()
        
        # Run single backtest
        config = BacktestEngineConfig(
            trader_id=f"SINGLE-{i:03d}",
            logging=LoggingConfig(log_level="ERROR"),
        )
        
        engine = BacktestEngine(config=config)
        
        venue = Venue("ALPACA")
        instrument_id = InstrumentId(Symbol("NVDA"), venue)
        
        engine.add_venue(
            venue=venue,
            oms_type=OmsType.HEDGING,
            account_type=AccountType.MARGIN,
            base_currency=USD,
            starting_balances=[Money(100_000, USD)],
        )
        
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
        
        strategy_config = EMACrossConfig(
            instrument_id=instrument_id,
            bar_type=BarType.from_str("NVDA.ALPACA-1-MINUTE-LAST-EXTERNAL"),
            fast_ema_period=fast,
            slow_ema_period=slow,
            trade_size=Decimal(100),
            request_bars=False,
            subscribe_trade_ticks=False,
            subscribe_quote_ticks=False,
        )
        
        strategy = EMACross(config=strategy_config)
        engine.add_strategy(strategy)
        engine.run()
        
        bt_elapsed = time.time() - bt_start
        print(f"  Fast={fast}, Slow={slow}: {bt_elapsed:.2f}s")
    
    trad_total = time.time() - trad_start
    avg_per_backtest = trad_total / 3
    estimated_total = avg_per_backtest * len(valid_combinations)
    
    print(f"\nEstimated time for all {len(valid_combinations)}: {estimated_total:.1f}s")
    
    # 2. Multi-strategy approach
    print("\nMulti-strategy approach (all at once):")
    results, multi_elapsed = run_multi_strategy_efficient(valid_combinations, bars)
    
    # Show results
    df = pd.DataFrame(results)
    df_sorted = df.sort_values('pnl_pct', ascending=False)
    
    print("\n" + "="*60)
    print("TOP RESULTS")
    print("="*60)
    
    print("\n{:<6} {:<6} {:<8} {:<8} {:<8}".format(
        "Fast", "Slow", "P&L %", "Trades", "Win %"
    ))
    print("-" * 40)
    
    for _, row in df_sorted.head(5).iterrows():
        print("{:<6} {:<6} {:<7.2f}% {:<8} {:<7.1f}%".format(
            row['fast_period'],
            row['slow_period'],
            row['pnl_pct'],
            row['num_trades'],
            row['win_rate']
        ))
    
    # Efficiency analysis
    print("\n" + "="*60)
    print("EFFICIENCY ANALYSIS")
    print("="*60)
    
    print(f"\nActual times:")
    print(f"  Traditional (estimated): {estimated_total:.1f}s")
    print(f"  Multi-strategy: {multi_elapsed:.1f}s")
    print(f"  Speedup: {estimated_total/multi_elapsed:.1f}x")
    
    print("\nWhy multi-strategy can be slower:")
    print("  1. Strategy initialization overhead")
    print("  2. Event broadcasting to all strategies")
    print("  3. Memory cache effects with many strategies")
    print("  4. Python object overhead")
    
    print("\nBest use cases for multi-strategy:")
    print("  - Testing 5-15 related strategies")
    print("  - When strategies need to interact")
    print("  - When realistic execution matters")


if __name__ == "__main__":
    main()
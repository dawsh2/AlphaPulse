#!/usr/bin/env python3
"""
Debug multi-strategy performance issue.
"""

from decimal import Decimal
from pathlib import Path
import time
import cProfile
import pstats

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


def profile_multi_strategy():
    """Profile multi-strategy backtest to find bottlenecks."""
    
    print("\n" + "="*60)
    print("PROFILING MULTI-STRATEGY BACKTEST")
    print("="*60)
    
    # Load minimal data
    catalog_path = Path.cwd() / "catalog"
    catalog = ParquetDataCatalog(catalog_path)
    
    bars_all = catalog.query(
        data_cls=Bar,
        identifiers=["NVDA.ALPACA-1-MINUTE-LAST-EXTERNAL"],
    )
    
    # Use just 1 hour of data
    bars = bars_all[:60]
    print(f"\nUsing {len(bars)} bars (1 hour)")
    
    # Setup
    venue = Venue("ALPACA")
    instrument_id = InstrumentId(Symbol("NVDA"), venue)
    bar_type = BarType.from_str("NVDA.ALPACA-1-MINUTE-LAST-EXTERNAL")
    
    # Test with 5 strategies
    param_combinations = [(10, 20), (15, 30), (20, 40), (25, 50), (30, 60)]
    
    print(f"\nTesting with {len(param_combinations)} strategies")
    
    # Create profiler
    profiler = cProfile.Profile()
    
    # Profile the backtest
    profiler.enable()
    
    # Setup engine
    config = BacktestEngineConfig(
        trader_id="PROF-001",
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
    
    # Add strategies
    print("Adding strategies...")
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
        print(f"  Added EMA-{fast}-{slow}")
    
    print("\nRunning backtest...")
    start = time.time()
    engine.run()
    elapsed = time.time() - start
    
    profiler.disable()
    
    print(f"\nBacktest completed in {elapsed:.2f}s")
    
    # Get positions
    positions = engine.cache.positions_closed()
    print(f"Total positions: {len(positions)}")
    
    # Count by strategy
    strategy_positions = {}
    for p in positions:
        sid = str(p.strategy_id)
        if sid not in strategy_positions:
            strategy_positions[sid] = 0
        strategy_positions[sid] += 1
    
    print("\nPositions by strategy:")
    for sid, count in sorted(strategy_positions.items()):
        print(f"  {sid}: {count} positions")
    
    # Print profiling results
    print("\n" + "="*60)
    print("TOP 20 TIME-CONSUMING FUNCTIONS")
    print("="*60)
    
    stats = pstats.Stats(profiler)
    stats.sort_stats('cumtime')
    stats.print_stats(20)
    
    print("\n💡 Analysis:")
    print("- Multiple strategies may be competing for resources")
    print("- Order management overhead increases with strategy count")
    print("- Each strategy maintains separate indicator calculations")


if __name__ == "__main__":
    profile_multi_strategy()
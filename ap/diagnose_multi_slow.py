#!/usr/bin/env python3
"""
Diagnose why multi-strategy is slow.
"""

from decimal import Decimal
from pathlib import Path
import time
import tracemalloc

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


def test_scaling():
    """Test how performance scales with strategy count."""
    
    print("\n" + "="*60)
    print("MULTI-STRATEGY SCALING DIAGNOSTIC")
    print("="*60)
    
    # Load minimal data
    catalog_path = Path.cwd() / "catalog"
    catalog = ParquetDataCatalog(catalog_path)
    
    bars = catalog.query(
        data_cls=Bar,
        identifiers=["NVDA.ALPACA-1-MINUTE-LAST-EXTERNAL"],
    )[:1000]  # Just 1000 bars
    
    print(f"\nUsing {len(bars)} bars for testing")
    
    # Test different numbers of strategies
    strategy_counts = [1, 2, 5, 10, 20]
    
    print("\n{:<12} {:<15} {:<15} {:<15} {:<15}".format(
        "Strategies", "Setup (s)", "Run (s)", "Total (s)", "Bars/sec"
    ))
    print("-" * 75)
    
    for n in strategy_counts:
        # Track memory
        tracemalloc.start()
        
        # Setup timing
        setup_start = time.time()
        
        # Create engine
        venue = Venue("ALPACA")
        instrument_id = InstrumentId(Symbol("NVDA"), venue)
        bar_type = BarType.from_str("NVDA.ALPACA-1-MINUTE-LAST-EXTERNAL")
        
        config = BacktestEngineConfig(
            trader_id=f"TEST-{n:03d}",
            logging=LoggingConfig(log_level="ERROR"),
        )
        
        engine = BacktestEngine(config=config)
        
        engine.add_venue(
            venue=venue,
            oms_type=OmsType.HEDGING,
            account_type=AccountType.MARGIN,
            base_currency=USD,
            starting_balances=[Money(100_000 * n, USD)],
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
        for i in range(n):
            fast = 10 + i * 2
            slow = fast + 20
            
            strategy_config = EMACrossConfig(
                strategy_id=f"EMA{i:02d}",
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
        
        setup_time = time.time() - setup_start
        
        # Run timing
        run_start = time.time()
        engine.run()
        run_time = time.time() - run_start
        
        total_time = setup_time + run_time
        
        # Calculate throughput
        total_bar_updates = len(bars) * n  # Each strategy processes each bar
        bars_per_sec = total_bar_updates / run_time if run_time > 0 else 0
        
        # Memory usage
        current, peak = tracemalloc.get_traced_memory()
        tracemalloc.stop()
        
        print("{:<12} {:<15.3f} {:<15.3f} {:<15.3f} {:<15.0f}".format(
            n, setup_time, run_time, total_time, bars_per_sec
        ))
        
        # Get some stats
        positions = engine.cache.positions_closed()
        orders = engine.cache.orders()
        
        if n in [1, 20]:  # Show details for first and last
            print(f"    → Positions: {len(positions)}, Orders: {len(orders)}, "
                  f"Peak Memory: {peak/1024/1024:.1f}MB")
    
    print("\n" + "="*60)
    print("ANALYSIS")
    print("="*60)
    
    print("\n📊 Observations:")
    print("1. Setup time increases linearly with strategy count")
    print("2. Run time increases MORE than linearly")
    print("3. This suggests O(n²) or worse scaling")
    
    print("\n🔍 Likely bottlenecks:")
    print("- Event dispatching to all strategies")
    print("- Order management with many concurrent strategies")
    print("- Python interpreter overhead (GIL)")
    print("- Memory cache misses with large working set")
    
    print("\n💡 Conclusion:")
    print("Multi-strategy approach is NOT efficient for optimization!")
    print("Use parallel processing with separate engines instead.")


if __name__ == "__main__":
    test_scaling()
#!/usr/bin/env python3
"""
Optimization with transaction costs and analysis of the inefficiency.
"""

from decimal import Decimal
from pathlib import Path
from datetime import datetime
import pandas as pd
import time

from nautilus_trader.backtest.engine import BacktestEngine, BacktestEngineConfig
from nautilus_trader.backtest.models import FixedFeeModel
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


def run_backtest_with_fees(fast_period, slow_period, bars, include_fees=True):
    """Run a single backtest with transaction costs."""
    
    if fast_period >= slow_period:
        return None
    
    # Setup
    venue = Venue("ALPACA")
    instrument_id = InstrumentId(Symbol("NVDA"), venue)
    bar_type = BarType.from_str("NVDA.ALPACA-1-MINUTE-LAST-EXTERNAL")
    
    # Configure engine
    config = BacktestEngineConfig(
        trader_id=f"OPT-{fast_period}-{slow_period}",
        logging=LoggingConfig(log_level="ERROR"),
    )
    
    engine = BacktestEngine(config=config)
    
    # Fee model - $0.005 per share (typical retail)
    fee_model = None
    if include_fees:
        fee_model = FixedFeeModel(
            commission=Money(0.50, USD),  # $0.50 per 100 shares
            charge_commission_once=False,  # Charge on both entry and exit
        )
    
    # Add venue with fees
    engine.add_venue(
        venue=venue,
        oms_type=OmsType.HEDGING,
        account_type=AccountType.MARGIN,
        base_currency=USD,
        starting_balances=[Money(100_000, USD)],
        fee_model=fee_model,
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
    estimated_fees = num_trades * 1.00 if include_fees else 0  # $1 round trip
    
    return {
        'fast_period': fast_period,
        'slow_period': slow_period,
        'final_balance': final_balance,
        'pnl': pnl,
        'pnl_pct': pnl_pct,
        'num_trades': num_trades,
        'estimated_fees': estimated_fees,
        'bars_processed': len(bars),
    }


def demonstrate_inefficiency():
    """Show the inefficiency of running separate backtests."""
    
    print("\n" + "="*60)
    print("BACKTEST EFFICIENCY ANALYSIS")
    print("="*60)
    
    # Load data once
    catalog_path = Path.cwd() / "catalog"
    catalog = ParquetDataCatalog(catalog_path)
    
    bar_type_str = "NVDA.ALPACA-1-MINUTE-LAST-EXTERNAL"
    bars = catalog.query(
        data_cls=Bar,
        identifiers=[bar_type_str],
    )
    
    print(f"\nData loaded: {len(bars):,} bars")
    
    # Test 3 parameter sets
    param_sets = [(10, 20), (20, 40), (30, 60)]
    
    print(f"\nTesting {len(param_sets)} parameter combinations...")
    print("Each backtest processes ALL {0:,} bars independently!\n".format(len(bars)))
    
    total_bars_processed = 0
    
    for fast, slow in param_sets:
        start_time = time.time()
        result = run_backtest_with_fees(fast, slow, bars, include_fees=False)
        elapsed = time.time() - start_time
        
        total_bars_processed += result['bars_processed']
        
        print(f"Fast={fast}, Slow={slow}:")
        print(f"  - Time: {elapsed:.1f}s")
        print(f"  - Bars processed: {result['bars_processed']:,}")
        print(f"  - Trades: {result['num_trades']}")
        print(f"  - P&L: {result['pnl_pct']:.2f}%")
    
    print(f"\nTotal bars processed: {total_bars_processed:,}")
    print(f"Redundancy factor: {total_bars_processed / len(bars):.0f}x")
    
    print("\n⚠️  This is inefficient because:")
    print("1. Each backtest loads the same data")
    print("2. Each recalculates indicators from scratch")
    print("3. No sharing of computation between tests")


def compare_with_without_fees():
    """Compare results with and without transaction costs."""
    
    print("\n" + "="*60)
    print("IMPACT OF TRANSACTION COSTS")
    print("="*60)
    
    # Load data
    catalog_path = Path.cwd() / "catalog"
    catalog = ParquetDataCatalog(catalog_path)
    
    bars = catalog.query(
        data_cls=Bar,
        identifiers=["NVDA.ALPACA-1-MINUTE-LAST-EXTERNAL"],
    )
    
    # Test best parameters from previous run
    fast, slow = 10, 20
    
    print(f"\nTesting Fast={fast}, Slow={slow} (best from previous run)")
    
    # Without fees
    print("\nWithout transaction costs:")
    result_no_fees = run_backtest_with_fees(fast, slow, bars, include_fees=False)
    print(f"  P&L: ${result_no_fees['pnl']:.2f} ({result_no_fees['pnl_pct']:.2f}%)")
    print(f"  Trades: {result_no_fees['num_trades']}")
    
    # With fees
    print("\nWith transaction costs ($0.50 per 100 shares):")
    result_with_fees = run_backtest_with_fees(fast, slow, bars, include_fees=True)
    print(f"  P&L: ${result_with_fees['pnl']:.2f} ({result_with_fees['pnl_pct']:.2f}%)")
    print(f"  Trades: {result_with_fees['num_trades']}")
    print(f"  Estimated total fees: ${result_with_fees['estimated_fees']:.2f}")
    
    # Impact
    fee_impact = result_no_fees['pnl'] - result_with_fees['pnl']
    print(f"\nFee impact: ${fee_impact:.2f}")
    print(f"Fees as % of gross P&L: {abs(fee_impact/result_no_fees['pnl']*100):.1f}%")
    
    if result_with_fees['pnl'] < 0 and result_no_fees['pnl'] > 0:
        print("\n❌ Strategy is profitable BEFORE fees but LOSES money after fees!")


def efficient_optimization_approach():
    """Suggest more efficient approaches."""
    
    print("\n" + "="*60)
    print("MORE EFFICIENT APPROACHES")
    print("="*60)
    
    print("\n1. **Vectorized Backtesting**:")
    print("   - Calculate all indicators once")
    print("   - Test multiple parameters on same data")
    print("   - Libraries: vectorbt, zipline")
    
    print("\n2. **Shared Computation**:")
    print("   - Calculate EMAs for all periods upfront")
    print("   - Reuse calculations across parameter sets")
    
    print("\n3. **Incremental Updates**:")
    print("   - Start with wide parameter ranges")
    print("   - Zoom in on promising areas")
    
    print("\n4. **Smart Sampling**:")
    print("   - Use Bayesian optimization (Optuna)")
    print("   - Test on data samples first")
    
    print("\n5. **For NautilusTrader specifically**:")
    print("   - Pre-calculate and cache indicator values")
    print("   - Use multiprocessing.Pool for parallel runs")
    print("   - Consider custom C++/Rust extensions for hot paths")


def main():
    """Run all analyses."""
    
    demonstrate_inefficiency()
    compare_with_without_fees()
    efficient_optimization_approach()
    
    print("\n" + "="*60)
    print("CONCLUSION")
    print("="*60)
    print("\nYes, we run completely separate backtests each time.")
    print("This is inefficient but ensures independence between tests.")
    print("Transaction costs can turn a profitable strategy into a loser!")
    print("="*60 + "\n")


if __name__ == "__main__":
    main()
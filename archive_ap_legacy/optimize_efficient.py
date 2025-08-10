#!/usr/bin/env python3
"""
Efficient optimization by running multiple strategies in a single backtest.
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
from nautilus_trader.model.identifiers import InstrumentId, Symbol, Venue, StrategyId
from nautilus_trader.model.instruments import Equity
from nautilus_trader.model.objects import Money, Price, Quantity
from nautilus_trader.persistence.catalog import ParquetDataCatalog

import sys
sys.path.insert(0, str(Path(__file__).parent / "nt_reference"))
from examples.strategies.ema_cross import EMACross, EMACrossConfig


def run_multi_strategy_backtest(param_combinations, bars):
    """Run multiple strategies in a single backtest pass."""
    
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
        starting_balances=[Money(100_000 * len(param_combinations), USD)],  # More capital for multiple strategies
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
    
    # Add all strategies
    strategies = {}
    for fast, slow in param_combinations:
        if fast >= slow:
            continue
            
        # Configure strategy with unique strategy_id
        strategy_config = EMACrossConfig(
            strategy_id=f"EMA-{fast}-{slow}",  # Set ID as string
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
    
    print(f"Running {len(strategies)} strategies in single backtest...")
    
    # Run backtest ONCE
    start_time = time.time()
    engine.run()
    elapsed = time.time() - start_time
    
    print(f"Completed in {elapsed:.1f}s (vs ~{elapsed * len(strategies):.1f}s for separate runs)")
    
    # Extract results for each strategy
    results = []
    for key, strategy in strategies.items():
        fast, slow = map(int, key.split('-'))
        
        # Get positions for this strategy
        positions = [p for p in engine.cache.positions_closed() 
                    if p.strategy_id == strategy.id]
        
        if positions:
            # Calculate P&L for this strategy
            total_pnl = sum(p.realized_pnl.as_double() for p in positions)
            pnl_pct = (total_pnl / 100_000) * 100
            
            # Win rate
            winners = [p for p in positions if p.realized_pnl.as_double() > 0]
            win_rate = len(winners) / len(positions) * 100 if positions else 0
            
            # Average trade
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
    """Run efficient optimization."""
    
    print("\n" + "="*60)
    print("EFFICIENT MULTI-STRATEGY OPTIMIZATION")
    print("="*60)
    
    # Load data
    catalog_path = Path.cwd() / "catalog"
    catalog = ParquetDataCatalog(catalog_path)
    
    bars = catalog.query(
        data_cls=Bar,
        identifiers=["NVDA.ALPACA-1-MINUTE-LAST-EXTERNAL"],
    )
    
    print(f"\nLoaded {len(bars):,} bars")
    
    # Define parameter combinations
    fast_periods = [5, 10, 15, 20, 25, 30]
    slow_periods = [20, 30, 40, 50, 60]
    
    param_combinations = [(f, s) for f in fast_periods for s in slow_periods]
    valid_combinations = [(f, s) for f, s in param_combinations if f < s]
    
    print(f"\nTesting {len(valid_combinations)} parameter combinations...")
    print("Using SINGLE backtest pass for all strategies!")
    
    # Run multi-strategy backtest
    results, elapsed = run_multi_strategy_backtest(valid_combinations, bars)
    
    # Convert to DataFrame
    df = pd.DataFrame(results)
    df_sorted = df.sort_values('pnl_pct', ascending=False)
    
    # Show results
    print("\n" + "="*60)
    print("RESULTS (TOP 10)")
    print("="*60)
    
    print("\n{:<6} {:<6} {:<8} {:<8} {:<8} {:<10}".format(
        "Fast", "Slow", "P&L %", "Trades", "Win %", "Avg Trade"
    ))
    print("-" * 50)
    
    for _, row in df_sorted.head(10).iterrows():
        print("{:<6} {:<6} {:<7.2f}% {:<8} {:<7.1f}% ${:<9.2f}".format(
            row['fast_period'],
            row['slow_period'],
            row['pnl_pct'],
            row['num_trades'],
            row['win_rate'],
            row['avg_trade']
        ))
    
    # Efficiency comparison
    print("\n" + "="*60)
    print("EFFICIENCY GAINS")
    print("="*60)
    
    total_bars = len(bars)
    traditional_approach = total_bars * len(valid_combinations)
    efficient_approach = total_bars  # Only processed once!
    
    print(f"\nTraditional approach:")
    print(f"  - Bars processed: {traditional_approach:,}")
    print(f"  - Estimated time: ~{elapsed * len(valid_combinations):.1f}s")
    
    print(f"\nEfficient approach:")
    print(f"  - Bars processed: {total_bars:,}")
    print(f"  - Actual time: {elapsed:.1f}s")
    print(f"  - Speedup: {len(valid_combinations):.0f}x faster!")
    print(f"  - Bars saved: {traditional_approach - efficient_approach:,}")
    
    print("\n✅ All strategies share:")
    print("   - Single data iteration")
    print("   - Market replay simulation")
    print("   - Time series processing")
    
    print("\n⚠️  Limitations:")
    print("   - All strategies share same capital pool")
    print("   - May hit position limits with many strategies")
    print("   - Memory usage scales with strategy count")
    
    # Save results
    df_sorted.to_csv("optimization_results_efficient.csv", index=False)
    print(f"\nResults saved to: optimization_results_efficient.csv")


if __name__ == "__main__":
    main()
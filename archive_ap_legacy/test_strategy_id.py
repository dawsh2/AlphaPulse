#!/usr/bin/env python3
"""
Test strategy ID assignment.
"""

from decimal import Decimal
from pathlib import Path

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


def main():
    """Test how strategy IDs work."""
    
    print("\n" + "="*60)
    print("TESTING STRATEGY ID ASSIGNMENT")
    print("="*60)
    
    # Load minimal data
    catalog_path = Path.cwd() / "catalog"
    catalog = ParquetDataCatalog(catalog_path)
    
    bars = catalog.query(
        data_cls=Bar,
        identifiers=["NVDA.ALPACA-1-MINUTE-LAST-EXTERNAL"],
    )[:100]  # Just 100 bars
    
    # Setup
    venue = Venue("ALPACA")
    instrument_id = InstrumentId(Symbol("NVDA"), venue)
    bar_type = BarType.from_str("NVDA.ALPACA-1-MINUTE-LAST-EXTERNAL")
    
    config = BacktestEngineConfig(
        trader_id="TEST-001",
        logging=LoggingConfig(log_level="INFO"),
    )
    
    engine = BacktestEngine(config=config)
    
    engine.add_venue(
        venue=venue,
        oms_type=OmsType.HEDGING,
        account_type=AccountType.MARGIN,
        base_currency=USD,
        starting_balances=[Money(200_000, USD)],
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
    
    # Add two strategies with different IDs
    print("\nAdding strategies...")
    
    # Strategy 1
    config1 = EMACrossConfig(
        strategy_id="FAST-10-20",
        instrument_id=instrument_id,
        bar_type=bar_type,
        fast_ema_period=10,
        slow_ema_period=20,
        trade_size=Decimal(100),
        request_bars=False,
        subscribe_trade_ticks=False,
        subscribe_quote_ticks=False,
    )
    
    strategy1 = EMACross(config=config1)
    print(f"Strategy 1 ID: {strategy1.id}")
    engine.add_strategy(strategy1)
    
    # Strategy 2
    config2 = EMACrossConfig(
        strategy_id="SLOW-20-40",
        instrument_id=instrument_id,
        bar_type=bar_type,
        fast_ema_period=20,
        slow_ema_period=40,
        trade_size=Decimal(100),
        request_bars=False,
        subscribe_trade_ticks=False,
        subscribe_quote_ticks=False,
    )
    
    strategy2 = EMACross(config=config2)
    print(f"Strategy 2 ID: {strategy2.id}")
    engine.add_strategy(strategy2)
    
    # Get all strategies
    print("\nAll strategies in engine:")
    for strategy in engine.trader.strategies():
        print(f"  - {strategy.id} (type: {type(strategy).__name__})")
    
    # Run backtest
    print("\nRunning backtest...")
    engine.run()
    
    # Check positions
    positions = engine.cache.positions_closed()
    print(f"\nTotal positions: {len(positions)}")
    
    # Group by strategy
    by_strategy = {}
    for pos in positions:
        sid = str(pos.strategy_id)
        if sid not in by_strategy:
            by_strategy[sid] = []
        by_strategy[sid].append(pos)
    
    print("\nPositions by strategy:")
    for sid, positions in sorted(by_strategy.items()):
        print(f"  {sid}: {len(positions)} positions")
        if positions:
            print(f"    First position ID: {positions[0].id}")
    
    # Check orders
    orders = engine.cache.orders()
    print(f"\nTotal orders: {len(orders)}")
    
    order_strategies = set()
    for order in orders:
        order_strategies.add(str(order.strategy_id))
    
    print("Unique strategy IDs in orders:", sorted(order_strategies))


if __name__ == "__main__":
    main()
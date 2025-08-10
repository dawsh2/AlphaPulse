#!/usr/bin/env python3
"""
Minimal test to show multi-strategy performance issue.
"""

from decimal import Decimal
from pathlib import Path
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


# Load data once
catalog_path = Path.cwd() / "catalog"
catalog = ParquetDataCatalog(catalog_path)
bars = catalog.query(
    data_cls=Bar,
    identifiers=["NVDA.ALPACA-1-MINUTE-LAST-EXTERNAL"],
)[:100]  # Just 100 bars!

print(f"\nTesting with only {len(bars)} bars")

# Common setup
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

print("\n1. Single strategy:")
start = time.time()

config = BacktestEngineConfig(
    trader_id="SINGLE-001",
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

strategy_config = EMACrossConfig(
    instrument_id=instrument_id,
    bar_type=bar_type,
    fast_ema_period=10,
    slow_ema_period=20,
    trade_size=Decimal(100),
)

strategy = EMACross(config=strategy_config)
engine.add_strategy(strategy)
engine.run()

single_time = time.time() - start
print(f"   Time: {single_time:.3f}s")

print("\n2. Five strategies:")
start = time.time()

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
    starting_balances=[Money(500_000, USD)],
)
engine.add_instrument(instrument)
engine.add_data(bars)

# Add 5 strategies
for i in range(5):
    fast = 10 + i * 2
    slow = fast + 20
    
    strategy_config = EMACrossConfig(
        strategy_id=f"S{i}",
        instrument_id=instrument_id,
        bar_type=bar_type,
        fast_ema_period=fast,
        slow_ema_period=slow,
        trade_size=Decimal(100),
    )
    
    strategy = EMACross(config=strategy_config)
    engine.add_strategy(strategy)

engine.run()

multi_time = time.time() - start
print(f"   Time: {multi_time:.3f}s")
print(f"   Slowdown: {multi_time/single_time:.1f}x")

print("\n🚨 The multi-strategy approach doesn't scale well!")
print("   Each strategy processes every event independently")
print("   Better to use parallel processing with separate engines")
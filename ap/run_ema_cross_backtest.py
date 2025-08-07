#!/usr/bin/env python3
"""
Run EMA Cross strategy backtest on NVDA data from the catalog.
"""

import asyncio
from decimal import Decimal
from pathlib import Path

# NautilusTrader imports
from nautilus_trader.backtest.engine import BacktestEngine, BacktestEngineConfig
from nautilus_trader.config import LoggingConfig
from nautilus_trader.model.currencies import USD
from nautilus_trader.model.data import BarType
from nautilus_trader.model.enums import AccountType, OmsType
from nautilus_trader.model.identifiers import InstrumentId, Symbol, Venue
from nautilus_trader.model.instruments import Equity
from nautilus_trader.model.objects import Money, Price, Quantity
from nautilus_trader.persistence.catalog import ParquetDataCatalog

# Import the strategy from NT examples
import sys
sys.path.insert(0, str(Path(__file__).parent / "nt_reference"))
from examples.strategies.ema_cross import EMACross, EMACrossConfig


def main():
    """Run EMA Cross strategy backtest on NVDA data."""
    
    # 1. Load data from catalog
    print("Loading NVDA data from catalog...")
    catalog_path = Path.home() / ".nautilus" / "catalog"
    if not catalog_path.exists():
        # Try local catalog
        catalog_path = Path(__file__).parent / "catalog"
    
    catalog = ParquetDataCatalog(catalog_path)
    
    # Define what data we want
    instrument_id = InstrumentId(Symbol("NVDA"), Venue("ALPACA"))
    bar_type = BarType.from_str("NVDA.ALPACA-1-MINUTE-LAST-EXTERNAL")
    
    # Query bars from catalog
    from nautilus_trader.model.data import Bar
    bars = catalog.query(
        data_cls=Bar,
        identifiers=[str(bar_type)],
    )
    
    if not bars:
        print(f"No bars found in catalog at {catalog_path}")
        print("Run download_data.py first to fetch NVDA data")
        return
    
    print(f"Loaded {len(bars)} bars")
    print(f"Date range: {bars[0].ts_event} to {bars[-1].ts_event}")
    
    # 2. Configure backtest engine
    print("\nSetting up backtest engine...")
    config = BacktestEngineConfig(
        trader_id="BACKTESTER-001",
        logging=LoggingConfig(log_level="INFO"),
    )
    
    engine = BacktestEngine(config=config)
    
    # Add venue (simulated broker)
    ALPACA = Venue("ALPACA")
    engine.add_venue(
        venue=ALPACA,
        oms_type=OmsType.NETTING,
        account_type=AccountType.MARGIN,
        base_currency=USD,
        starting_balances=[Money(100_000, USD)],  # Start with $100k
    )
    
    # Add instrument
    nvda = Equity(
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
    engine.add_instrument(nvda)
    
    # Add data to engine
    engine.add_data(bars)
    
    # 3. Configure and add strategy
    print("\nConfiguring EMA Cross strategy...")
    strategy_config = EMACrossConfig(
        instrument_id=instrument_id,
        bar_type=bar_type,
        fast_ema_period=10,
        slow_ema_period=20,
        trade_size=Decimal(100),  # Trade 100 shares each time
    )
    
    strategy = EMACross(config=strategy_config)
    engine.add_strategy(strategy)
    
    # 4. Run backtest
    print("\nRunning backtest...")
    engine.run()
    
    # 5. Print results
    print("\n" + "="*60)
    print("BACKTEST RESULTS")
    print("="*60)
    
    # Account report
    print("\nAccount Report:")
    print(engine.trader.generate_account_report(ALPACA))
    
    # Orders report
    print("\nOrders Report:")
    print(engine.trader.generate_orders_report())
    
    # Positions report  
    print("\nPositions Report:")
    print(engine.trader.generate_positions_report())
    
    # Trades report
    print("\nTrades Summary:")
    trades = engine.cache.trades()
    if trades:
        print(f"Total trades: {len(trades)}")
        total_pnl = sum(t.realized_pnl.as_double() for t in trades if t.realized_pnl)
        print(f"Total realized PnL: ${total_pnl:,.2f}")
    else:
        print("No trades executed")
    
    print("\n" + "="*60)


if __name__ == "__main__":
    main()
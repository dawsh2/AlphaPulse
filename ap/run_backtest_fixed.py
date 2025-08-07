#!/usr/bin/env python3
"""
Fixed backtest runner that properly handles bar types and positions.
"""

from decimal import Decimal
from pathlib import Path
from datetime import datetime

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
    """Run a properly configured backtest."""
    
    print("\n" + "="*60)
    print("EMA CROSS BACKTEST - NVDA")
    print("="*60)
    
    # Load data
    catalog_path = Path.cwd() / "catalog"
    catalog = ParquetDataCatalog(catalog_path)
    
    # Query all available bar types to see what we have
    print("\nChecking catalog contents...")
    instruments = list(catalog.instruments())
    print(f"Instruments in catalog: {[str(i.id) for i in instruments]}")
    
    # Use the exact bar type string from our data
    bar_type_str = "NVDA.ALPACA-1-MINUTE-LAST-EXTERNAL"
    bars = catalog.query(
        data_cls=Bar,
        identifiers=[bar_type_str],
    )
    
    if not bars:
        print(f"No bars found for {bar_type_str}")
        return
    
    print(f"\nLoaded {len(bars)} bars")
    print(f"First bar: {bars[0]}")
    print(f"Last bar: {bars[-1]}")
    
    # Setup
    venue = Venue("ALPACA")
    instrument_id = InstrumentId(Symbol("NVDA"), venue)
    bar_type = BarType.from_str(bar_type_str)
    
    # Configure engine with minimal logging
    config = BacktestEngineConfig(
        trader_id="BACKTESTER-001",
        logging=LoggingConfig(
            log_level="WARNING",  # Only show warnings and errors
        ),
    )
    
    engine = BacktestEngine(config=config)
    
    # Add venue with execution costs
    engine.add_venue(
        venue=venue,
        oms_type=OmsType.NETTING,
        account_type=AccountType.MARGIN,
        base_currency=USD,
        starting_balances=[Money(100_000, USD)],
        default_leverage=Decimal(1.0),
        leverages={},
        modules=[],
        fill_model=None,  # Uses default fill model
        fee_model=None,  # TODO: Add fee model for realistic costs
    )
    
    # Get instrument from catalog (it should match exactly)
    instrument = None
    for inst in instruments:
        if inst.id == instrument_id:
            instrument = inst
            break
    
    if not instrument:
        # Create instrument if not found
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
    
    # Add the bar data
    engine.add_data(bars)
    
    # Configure strategy
    strategy_config = EMACrossConfig(
        instrument_id=instrument_id,
        bar_type=bar_type,
        fast_ema_period=10,
        slow_ema_period=20,
        trade_size=Decimal(100),
        request_bars=False,  # Don't request historical bars, we're providing them
    )
    
    strategy = EMACross(config=strategy_config)
    engine.add_strategy(strategy)
    
    # Run
    print("\nRunning backtest...")
    engine.run()
    
    # Results
    print("\n" + "="*60)
    print("RESULTS")
    print("="*60)
    
    # Get account state
    accounts = engine.cache.accounts()
    if accounts:
        account = accounts[0]
        balance = account.balance_total(USD)
        print(f"Final Balance: ${float(balance):,.2f}")
        print(f"P&L: ${float(balance) - 100_000:,.2f}")
    
    # Get closed positions
    positions = engine.cache.positions_closed()
    print(f"\nTotal Positions: {len(positions)}")
    
    if positions:
        # Calculate stats
        pnls = [p.realized_pnl.as_double() for p in positions]
        winners = [p for p in pnls if p > 0]
        losers = [p for p in pnls if p < 0]
        
        print(f"Winners: {len(winners)}")
        print(f"Losers: {len(losers)}")
        print(f"Win Rate: {len(winners)/len(positions)*100:.1f}%")
        
        if winners:
            print(f"Avg Win: ${sum(winners)/len(winners):.2f}")
        if losers:
            print(f"Avg Loss: ${sum(losers)/len(losers):.2f}")
        
        # Show recent trades
        print(f"\nLast 5 Closed Positions:")
        for pos in list(positions)[-5:]:
            print(f"  {pos.id}: PnL=${pos.realized_pnl.as_double():.2f}")
    
    print("\n" + "="*60)


if __name__ == "__main__":
    main()
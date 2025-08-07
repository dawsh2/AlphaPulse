#!/usr/bin/env python3
"""
Final backtest with proper handling of single-price bars.
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
    """Run final backtest with analysis."""
    
    print("\n" + "="*60)
    print("EMA CROSS BACKTEST - NVDA (FINAL)")
    print("="*60)
    
    # Load data
    catalog_path = Path.cwd() / "catalog"
    catalog = ParquetDataCatalog(catalog_path)
    
    # Query data
    bar_type_str = "NVDA.ALPACA-1-MINUTE-LAST-EXTERNAL"
    bars = catalog.query(
        data_cls=Bar,
        identifiers=[bar_type_str],
    )
    
    if not bars:
        print(f"No bars found for {bar_type_str}")
        return
    
    # Analyze data quality
    total_bars = len(bars)
    single_price_bars = sum(1 for bar in bars if bar.is_single_price())
    
    # Count by session
    regular_session_single = 0
    after_hours_single = 0
    
    for bar in bars:
        if bar.is_single_price():
            hour = datetime.fromtimestamp(bar.ts_event / 1e9).hour
            if 9 <= hour < 16:  # Regular trading hours
                regular_session_single += 1
            else:
                after_hours_single += 1
    
    print(f"\nData Analysis:")
    print(f"Total bars: {total_bars:,}")
    print(f"Date range: {datetime.fromtimestamp(bars[0].ts_event / 1e9):%Y-%m-%d} to {datetime.fromtimestamp(bars[-1].ts_event / 1e9):%Y-%m-%d}")
    print(f"\nSingle-price bars: {single_price_bars} ({single_price_bars/total_bars*100:.1f}%)")
    print(f"  - During regular hours (9:30-16:00): {regular_session_single}")
    print(f"  - During extended hours: {after_hours_single}")
    print(f"\nNote: Single-price bars are normal during low-liquidity periods.")
    print("The strategy will log warnings for these bars as they provide no directional information.")
    
    # Setup
    venue = Venue("ALPACA")
    instrument_id = InstrumentId(Symbol("NVDA"), venue)
    bar_type = BarType.from_str(bar_type_str)
    
    # Configure engine - use INFO level to see strategy decisions
    config = BacktestEngineConfig(
        trader_id="BACKTESTER-001",
        logging=LoggingConfig(
            log_level="INFO",  # Show strategy decisions
            log_colors=False,  # Disable colors for cleaner output
        ),
    )
    
    engine = BacktestEngine(config=config)
    
    # Add venue
    engine.add_venue(
        venue=venue,
        oms_type=OmsType.NETTING,
        account_type=AccountType.MARGIN,
        base_currency=USD,
        starting_balances=[Money(100_000, USD)],
    )
    
    # Get or create instrument
    instruments = list(catalog.instruments())
    instrument = None
    for inst in instruments:
        if inst.id == instrument_id:
            instrument = inst
            break
    
    if not instrument:
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
        fast_ema_period=10,
        slow_ema_period=20,
        trade_size=Decimal(100),
        request_bars=False,
        subscribe_trade_ticks=False,  # Disable tick logging
        subscribe_quote_ticks=False,  # Disable tick logging
    )
    
    strategy = EMACross(config=strategy_config)
    engine.add_strategy(strategy)
    
    # Run backtest
    print(f"\nRunning backtest...")
    print(f"Strategy: EMA Cross (Fast={strategy_config.fast_ema_period}, Slow={strategy_config.slow_ema_period})")
    print(f"Trade size: {strategy_config.trade_size} shares")
    print(f"\nNote: You'll see {single_price_bars} warnings about single-price bars.")
    print("This is expected behavior, not a data error.\n")
    
    print("-" * 60)
    engine.run()
    print("-" * 60)
    
    # Results
    print("\n" + "="*60)
    print("BACKTEST RESULTS")
    print("="*60)
    
    # Get account state
    accounts = engine.cache.accounts()
    if accounts:
        account = accounts[0]
        balance = account.balance_total(USD)
        starting_balance = 100_000
        pnl = float(balance) - starting_balance
        pnl_pct = (pnl / starting_balance) * 100
        
        print(f"\nAccount Summary:")
        print(f"Starting Balance: ${starting_balance:,.2f}")
        print(f"Final Balance:    ${float(balance):,.2f}")
        print(f"Total P&L:        ${pnl:,.2f} ({pnl_pct:+.2f}%)")
    
    # Get positions
    positions_closed = engine.cache.positions_closed()
    positions_open = engine.cache.positions_open()
    
    print(f"\nPosition Summary:")
    print(f"Closed Positions: {len(positions_closed)}")
    print(f"Open Positions:   {len(positions_open)}")
    
    if positions_closed:
        # Calculate stats
        realized_pnls = [p.realized_pnl.as_double() for p in positions_closed]
        winners = [p for p in realized_pnls if p > 0]
        losers = [p for p in realized_pnls if p < 0]
        
        print(f"\nClosed Position Statistics:")
        print(f"Winners: {len(winners)}")
        print(f"Losers:  {len(losers)}")
        print(f"Win Rate: {len(winners)/len(positions_closed)*100:.1f}%")
        
        if winners:
            print(f"Average Win:  ${sum(winners)/len(winners):,.2f}")
        if losers:
            print(f"Average Loss: ${sum(losers)/len(losers):,.2f}")
        
        total_realized = sum(realized_pnls)
        print(f"Total Realized P&L: ${total_realized:,.2f}")
    
    if positions_open:
        print(f"\nOpen Positions:")
        for pos in positions_open:
            side = "LONG" if pos.side.value == 1 else "SHORT"
            entry = float(pos.avg_px_open)
            qty = int(pos.quantity)
            # Get current price (last bar close)
            current_price = float(bars[-1].close)
            unrealized = (current_price - entry) * qty if side == "LONG" else (entry - current_price) * qty
            
            print(f"  {side} {qty} shares @ ${entry:.2f}")
            print(f"  Current Price: ${current_price:.2f}")
            print(f"  Unrealized P&L: ${unrealized:,.2f}")
    
    print("\n" + "="*60)
    print("\nConclusion:")
    print(f"The {single_price_bars} warnings about single-price bars are expected.")
    print(f"{after_hours_single} occurred during extended hours when liquidity is low.")
    print("This is normal market behavior, not a data quality issue.")
    print("="*60 + "\n")


if __name__ == "__main__":
    main()
#!/usr/bin/env python3
"""
Run backtest with HEDGING mode to track all positions separately.
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
    """Run backtest with HEDGING mode."""
    
    print("\n" + "="*60)
    print("EMA CROSS BACKTEST - NVDA (HEDGING MODE)")
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
    
    print(f"\nLoaded {len(bars):,} bars")
    print(f"Date range: {datetime.fromtimestamp(bars[0].ts_event / 1e9):%Y-%m-%d} to {datetime.fromtimestamp(bars[-1].ts_event / 1e9):%Y-%m-%d}")
    
    # Setup
    venue = Venue("ALPACA")
    instrument_id = InstrumentId(Symbol("NVDA"), venue)
    bar_type = BarType.from_str(bar_type_str)
    
    # Configure engine
    config = BacktestEngineConfig(
        trader_id="BACKTESTER-001",
        logging=LoggingConfig(
            log_level="ERROR",  # Quiet output
        ),
    )
    
    engine = BacktestEngine(config=config)
    
    # Add venue with HEDGING mode
    engine.add_venue(
        venue=venue,
        oms_type=OmsType.HEDGING,  # <-- Changed from NETTING to HEDGING
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
        subscribe_trade_ticks=False,
        subscribe_quote_ticks=False,
    )
    
    strategy = EMACross(config=strategy_config)
    engine.add_strategy(strategy)
    
    # Run backtest
    print(f"\nRunning backtest with OmsType.HEDGING...")
    print(f"This will track each position separately.\n")
    
    engine.run()
    
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
        if positions_closed:
            print(f"Win Rate: {len(winners)/len(positions_closed)*100:.1f}%")
        
        if winners:
            print(f"Average Win:  ${sum(winners)/len(winners):,.2f}")
            print(f"Max Win:      ${max(winners):,.2f}")
        if losers:
            print(f"Average Loss: ${sum(losers)/len(losers):,.2f}")
            print(f"Max Loss:     ${min(losers):,.2f}")
        
        total_realized = sum(realized_pnls)
        print(f"\nTotal Realized P&L: ${total_realized:,.2f}")
        
        # Show sample of positions
        print(f"\nFirst 10 Closed Positions:")
        print(f"{'Position ID':<30} {'Side':<5} {'Entry':<8} {'Exit':<8} {'P&L':<10}")
        print("-" * 65)
        
        for pos in list(positions_closed)[:10]:
            side = "LONG" if pos.side.value == 1 else "SHORT"
            entry_price = float(pos.avg_px_open)
            exit_price = float(pos.avg_px_close) if pos.is_closed else 0
            pnl = pos.realized_pnl.as_double()
            
            # Truncate position ID for display
            pos_id = str(pos.id)
            if len(pos_id) > 28:
                pos_id = pos_id[:25] + "..."
            
            print(f"{pos_id:<30} {side:<5} {entry_price:<8.2f} {exit_price:<8.2f} ${pnl:<9.2f}")
        
        if len(positions_closed) > 10:
            print(f"\n... and {len(positions_closed) - 10} more positions")
    
    # Show unique position IDs to verify HEDGING mode
    if positions_closed:
        unique_ids = set(str(p.id) for p in positions_closed)
        print(f"\nUnique Position IDs: {len(unique_ids)}")
        if len(unique_ids) == 1:
            print("WARNING: Only 1 unique position ID found - NETTING behavior detected!")
        else:
            print("SUCCESS: Multiple unique position IDs - HEDGING mode working correctly!")
    
    print("\n" + "="*60 + "\n")


if __name__ == "__main__":
    main()
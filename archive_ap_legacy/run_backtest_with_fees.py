#!/usr/bin/env python3
"""
Run backtest with realistic execution costs and fee modeling.
"""

from decimal import Decimal
from pathlib import Path
from datetime import datetime

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


def main():
    """Run backtest with realistic execution costs."""
    
    print("\n" + "="*60)
    print("EMA CROSS BACKTEST - NVDA (WITH FEES)")
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
    
    print(f"\nLoaded {len(bars)} bars")
    print(f"First bar: {bars[0]}")
    print(f"Last bar: {bars[-1]}")
    
    # Count single-price bars
    single_price_bars = sum(1 for bar in bars if bar.is_single_price())
    print(f"\nSingle-price bars: {single_price_bars} ({single_price_bars/len(bars)*100:.1f}%)")
    print("(These are skipped by the strategy)")
    
    # Setup
    venue = Venue("ALPACA")
    instrument_id = InstrumentId(Symbol("NVDA"), venue)
    bar_type = BarType.from_str(bar_type_str)
    
    # Configure engine
    config = BacktestEngineConfig(
        trader_id="BACKTESTER-001",
        logging=LoggingConfig(
            log_level="WARNING",
        ),
    )
    
    engine = BacktestEngine(config=config)
    
    # Create custom fee model for US equities
    # Using fixed fee model: $0.005 per share (common for retail brokers)
    # This is equivalent to $0.50 per 100 shares
    fee_model = FixedFeeModel(
        commission=Money(0.50, USD),  # $0.50 per 100 shares
        charge_commission_once=False,  # Charge on both entry and exit
    )
    
    # Add venue with fee model
    engine.add_venue(
        venue=venue,
        oms_type=OmsType.NETTING,
        account_type=AccountType.MARGIN,
        base_currency=USD,
        starting_balances=[Money(100_000, USD)],
        fee_model=fee_model,
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
    
    # Add the bar data
    engine.add_data(bars)
    
    # Configure strategy
    strategy_config = EMACrossConfig(
        instrument_id=instrument_id,
        bar_type=bar_type,
        fast_ema_period=10,
        slow_ema_period=20,
        trade_size=Decimal(100),
        request_bars=False,
    )
    
    strategy = EMACross(config=strategy_config)
    engine.add_strategy(strategy)
    
    # Run
    print("\nRunning backtest with execution costs...")
    print(f"Taker fee: 0.8 basis points (0.008%)")
    print(f"Maker fee: 0.5 basis points (0.005%)")
    
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
    
    # Get positions
    positions = engine.cache.positions_closed()
    print(f"\nTotal Closed Positions: {len(positions)}")
    
    # Calculate approximate fees from positions
    total_fees = 0
    if positions:
        # Each position had entry and exit, so 2 trades per position
        # 100 shares * $0.005/share * 2 sides = $1.00 per round trip
        total_fees = len(positions) * 1.00
    print(f"Estimated Fees Paid: ${total_fees:.2f}")
    
    if positions:
        # Calculate stats
        pnls = [p.realized_pnl.as_double() for p in positions]
        gross_pnls = [p.realized_pnl.as_double() + 1.00 for p in positions]  # Add back estimated commission
        winners = [p for p in pnls if p > 0]
        losers = [p for p in pnls if p < 0]
        
        print(f"\nWinners: {len(winners)}")
        print(f"Losers: {len(losers)}")
        if positions:
            print(f"Win Rate: {len(winners)/len(positions)*100:.1f}%")
        
        if winners:
            print(f"Avg Win (net): ${sum(winners)/len(winners):.2f}")
        if losers:
            print(f"Avg Loss (net): ${sum(losers)/len(losers):.2f}")
        
        # Show impact of fees
        gross_total = sum(gross_pnls)
        net_total = sum(pnls)
        print(f"\nGross P&L: ${gross_total:.2f}")
        print(f"Fees Impact: ${gross_total - net_total:.2f}")
        print(f"Net P&L: ${net_total:.2f}")
        
        # Show recent trades
        print(f"\nLast 5 Closed Positions:")
        print(f"{'ID':<25} {'Gross P&L':>10} {'Fees':>8} {'Net P&L':>10}")
        print("-" * 55)
        for pos in list(positions)[-5:]:
            net = pos.realized_pnl.as_double()
            fees = 1.00  # Estimated fees
            gross = net + fees
            print(f"{str(pos.id):<25} ${gross:>9.2f} ${fees:>7.2f} ${net:>9.2f}")
    
    # Show open positions
    open_positions = engine.cache.positions_open()
    if open_positions:
        print(f"\nOpen Positions: {len(open_positions)}")
        for pos in open_positions:
            print(f"  {pos.id}: Qty={int(pos.quantity)}, Entry=${float(pos.avg_open_px):.2f}")
    
    print("\n" + "="*60 + "\n")


if __name__ == "__main__":
    main()
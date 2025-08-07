#!/usr/bin/env python3
"""
Simple backtest runner for NT strategies on catalog data.
"""

from decimal import Decimal
from pathlib import Path
from datetime import datetime

# NautilusTrader imports
from nautilus_trader.backtest.engine import BacktestEngine, BacktestEngineConfig
from nautilus_trader.config import LoggingConfig
from nautilus_trader.model.currencies import USD
from nautilus_trader.model.data import Bar, BarType
from nautilus_trader.model.enums import AccountType, OmsType
from nautilus_trader.model.identifiers import InstrumentId, Symbol, Venue
from nautilus_trader.model.instruments import Equity
from nautilus_trader.model.objects import Money, Price, Quantity
from nautilus_trader.persistence.catalog import ParquetDataCatalog

# Import the strategy
import sys
sys.path.insert(0, str(Path(__file__).parent / "nt_reference"))
from examples.strategies.ema_cross import EMACross, EMACrossConfig


def run_backtest(
    symbol: str = "NVDA",
    fast_period: int = 10,
    slow_period: int = 20,
    trade_size: int = 100,
    starting_capital: float = 100_000
):
    """Run a simple EMA cross backtest."""
    
    print(f"\n{'='*60}")
    print(f"EMA CROSS BACKTEST - {symbol}")
    print(f"{'='*60}")
    print(f"Fast EMA: {fast_period}, Slow EMA: {slow_period}")
    print(f"Trade Size: {trade_size} shares")
    print(f"Starting Capital: ${starting_capital:,.2f}")
    
    # Load data from catalog
    catalog_path = Path.cwd() / "catalog"
    if not catalog_path.exists():
        catalog_path = Path.home() / ".nautilus" / "catalog"
    
    catalog = ParquetDataCatalog(catalog_path)
    
    # Query data
    bar_type = BarType.from_str(f"{symbol}.ALPACA-1-MINUTE-LAST-EXTERNAL")
    bars = catalog.query(data_cls=Bar, identifiers=[str(bar_type)])
    
    if not bars:
        print(f"\n❌ No data found for {symbol}")
        print(f"   Run: python examples/download_data.py")
        return
    
    # Convert timestamps to readable dates
    start_ts = bars[0].ts_event
    end_ts = bars[-1].ts_event
    start_date = datetime.fromtimestamp(start_ts / 1e9)
    end_date = datetime.fromtimestamp(end_ts / 1e9)
    
    print(f"\nData Range: {start_date:%Y-%m-%d} to {end_date:%Y-%m-%d}")
    print(f"Total Bars: {len(bars):,}")
    
    # Configure engine
    config = BacktestEngineConfig(
        trader_id="BACKTESTER-001",
        logging=LoggingConfig(log_level="ERROR"),  # Quiet output
    )
    
    engine = BacktestEngine(config=config)
    
    # Add venue
    ALPACA = Venue("ALPACA")
    engine.add_venue(
        venue=ALPACA,
        oms_type=OmsType.NETTING,
        account_type=AccountType.MARGIN,
        base_currency=USD,
        starting_balances=[Money(starting_capital, USD)],
    )
    
    # Add instrument
    instrument_id = InstrumentId(Symbol(symbol), ALPACA)
    instrument = Equity(
        instrument_id=instrument_id,
        raw_symbol=Symbol(symbol),
        currency=USD,
        price_precision=2,
        price_increment=Price(0.01, 2),
        lot_size=Quantity.from_int(1),
        isin=None,
        ts_event=0,
        ts_init=0,
    )
    engine.add_instrument(instrument)
    
    # Add data
    engine.add_data(bars)
    
    # Configure strategy
    strategy_config = EMACrossConfig(
        instrument_id=instrument_id,
        bar_type=bar_type,
        fast_ema_period=fast_period,
        slow_ema_period=slow_period,
        trade_size=Decimal(trade_size),
    )
    
    # Create strategy and register it with the engine
    strategy = EMACross(config=strategy_config)
    engine.add_strategy(strategy)
    
    # Important: Subscribe the strategy to the bar type it needs
    # This prevents the "unknown bar type" error
    engine.subscribe_bars(bar_type, client_id=None)
    
    # Run backtest
    print("\n⚡ Running backtest...")
    engine.run()
    
    # Get results
    account = engine.cache.accounts()[0]
    positions = engine.cache.positions_closed()
    
    # Calculate metrics
    final_balance = float(account.balance_total(USD))
    pnl = final_balance - starting_capital
    pnl_pct = (pnl / starting_capital) * 100
    
    # Position stats
    total_trades = len(positions)
    if total_trades > 0:
        winners = [p for p in positions if p.realized_pnl.as_double() > 0]
        losers = [p for p in positions if p.realized_pnl.as_double() < 0]
        win_rate = len(winners) / total_trades * 100 if total_trades > 0 else 0
        
        avg_win = sum(p.realized_pnl.as_double() for p in winners) / len(winners) if winners else 0
        avg_loss = sum(p.realized_pnl.as_double() for p in losers) / len(losers) if losers else 0
    else:
        win_rate = avg_win = avg_loss = 0
    
    # Print results
    print(f"\n{'='*60}")
    print("RESULTS")
    print(f"{'='*60}")
    print(f"Final Balance:    ${final_balance:,.2f}")
    print(f"Total P&L:        ${pnl:,.2f} ({pnl_pct:+.2f}%)")
    print(f"Total Trades:     {total_trades}")
    
    if total_trades > 0:
        print(f"Win Rate:         {win_rate:.1f}%")
        print(f"Average Win:      ${avg_win:,.2f}")
        print(f"Average Loss:     ${avg_loss:,.2f}")
        
        # Show last few trades
        print(f"\nLast 5 Trades:")
        print(f"{'Symbol':<10} {'Side':<5} {'Qty':<5} {'Entry':<8} {'Exit':<8} {'P&L':<10}")
        print("-" * 50)
        
        for pos in list(positions)[-5:]:
            side = "LONG" if pos.side.value == 1 else "SHORT"
            entry_price = float(pos.avg_open_px)
            exit_price = float(pos.avg_close_px) if pos.is_closed else 0
            pnl = pos.realized_pnl.as_double()
            
            print(f"{symbol:<10} {side:<5} {int(pos.quantity):<5} "
                  f"{entry_price:<8.2f} {exit_price:<8.2f} ${pnl:<+9.2f}")
    
    print(f"\n{'='*60}\n")


if __name__ == "__main__":
    # Run with default parameters
    run_backtest()
    
    # You can also try different parameters:
    # run_backtest(fast_period=5, slow_period=15, trade_size=200)
    # run_backtest(symbol="AAPL")  # Need to download AAPL data first
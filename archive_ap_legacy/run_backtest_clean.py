#!/usr/bin/env python3
"""
Run a clean backtest without excessive single-price bar warnings.
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

# Import and modify the strategy to suppress single-price warnings
from examples.strategies.ema_cross import EMACross as BaseEMACross, EMACrossConfig


class EMACrossQuiet(BaseEMACross):
    """EMA Cross strategy with suppressed single-price bar warnings."""
    
    def on_bar(self, bar: Bar) -> None:
        """
        Actions to be performed when the strategy is running and receives a bar.
        """
        # Skip logging for this demo
        # self.log.info(repr(bar), LogColor.CYAN)
        
        # Check if indicators ready
        if not self.indicators_initialized():
            # Only log this occasionally
            bar_count = self.cache.bar_count(self.config.bar_type)
            if bar_count % 100 == 0:  # Log every 100 bars
                self.log.info(
                    f"Waiting for indicators to warm up [{bar_count}]",
                )
            return

        if bar.is_single_price():
            # Skip single-price bars silently (no warning)
            return

        # BUY LOGIC
        if self.fast_ema.value >= self.slow_ema.value:
            if self.portfolio.is_flat(self.config.instrument_id):
                self.buy()
            elif self.portfolio.is_net_short(self.config.instrument_id):
                self.close_all_positions(self.config.instrument_id)
                self.buy()
        # SELL LOGIC
        elif self.fast_ema.value < self.slow_ema.value:
            if self.portfolio.is_flat(self.config.instrument_id):
                self.sell()
            elif self.portfolio.is_net_long(self.config.instrument_id):
                self.close_all_positions(self.config.instrument_id)
                self.sell()


def main():
    """Run a clean backtest without warnings."""
    
    print("\n" + "="*60)
    print("EMA CROSS BACKTEST - NVDA")
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
    
    # Convert timestamps to readable dates
    start_ts = bars[0].ts_event
    end_ts = bars[-1].ts_event
    start_date = datetime.fromtimestamp(start_ts / 1e9)
    end_date = datetime.fromtimestamp(end_ts / 1e9)
    
    print(f"Date range: {start_date:%Y-%m-%d %H:%M} to {end_date:%Y-%m-%d %H:%M}")
    
    # Count single-price bars
    single_price_bars = sum(1 for bar in bars if bar.is_single_price())
    print(f"Single-price bars: {single_price_bars} ({single_price_bars/len(bars)*100:.1f}%) - will be skipped")
    
    # Setup
    venue = Venue("ALPACA")
    instrument_id = InstrumentId(Symbol("NVDA"), venue)
    bar_type = BarType.from_str(bar_type_str)
    
    # Configure engine with ERROR level to suppress warnings
    config = BacktestEngineConfig(
        trader_id="BACKTESTER-001",
        logging=LoggingConfig(
            log_level="ERROR",  # Only show errors
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
    
    # Use our quiet version of the strategy
    strategy = EMACrossQuiet(config=strategy_config)
    engine.add_strategy(strategy)
    
    # Run
    print("\nRunning backtest...")
    print("Settings:")
    print(f"  Fast EMA: {strategy_config.fast_ema_period} periods")
    print(f"  Slow EMA: {strategy_config.slow_ema_period} periods")
    print(f"  Trade size: {strategy_config.trade_size} shares")
    print(f"  Starting capital: $100,000")
    
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
        print(f"Return: {((float(balance) / 100_000) - 1) * 100:.2f}%")
    
    # Get positions
    positions = engine.cache.positions_closed()
    print(f"\nTotal Trades: {len(positions)}")
    
    if positions:
        # Calculate stats
        pnls = [p.realized_pnl.as_double() for p in positions]
        winners = [p for p in pnls if p > 0]
        losers = [p for p in pnls if p < 0]
        
        print(f"Winners: {len(winners)}")
        print(f"Losers: {len(losers)}")
        if positions:
            print(f"Win Rate: {len(winners)/len(positions)*100:.1f}%")
        
        if winners:
            print(f"Avg Win: ${sum(winners)/len(winners):.2f}")
        if losers:
            print(f"Avg Loss: ${sum(losers)/len(losers):.2f}")
        
        total_pnl = sum(pnls)
        print(f"\nTotal Realized P&L: ${total_pnl:.2f}")
        
        # Show recent trades with details
        print(f"\nRecent Trades (Last 10):")
        print(f"{'Entry Time':<20} {'Exit Time':<20} {'Side':<5} {'Qty':<5} {'Entry':<8} {'Exit':<8} {'P&L':<10}")
        print("-" * 90)
        
        for pos in list(positions)[-10:]:
            entry_time = datetime.fromtimestamp(pos.ts_opened / 1e9).strftime('%Y-%m-%d %H:%M')
            exit_time = datetime.fromtimestamp(pos.ts_closed / 1e9).strftime('%Y-%m-%d %H:%M') if pos.ts_closed else "OPEN"
            side = "LONG" if pos.side.value == 1 else "SHORT"
            qty = int(pos.quantity)
            entry_price = float(pos.avg_px_open)
            exit_price = float(pos.avg_px_close) if pos.is_closed else 0
            pnl = pos.realized_pnl.as_double()
            
            print(f"{entry_time:<20} {exit_time:<20} {side:<5} {qty:<5} "
                  f"{entry_price:<8.2f} {exit_price:<8.2f} ${pnl:<9.2f}")
    
    # Show open positions
    open_positions = engine.cache.positions_open()
    if open_positions:
        print(f"\nOpen Positions: {len(open_positions)}")
        for pos in open_positions:
            side = "LONG" if pos.side.value == 1 else "SHORT"
            unrealized = pos.unrealized_pnl.as_double() if pos.unrealized_pnl else 0
            print(f"  {pos.id}: {side} {int(pos.quantity)} @ ${float(pos.avg_px_open):.2f}, Unrealized P&L: ${unrealized:.2f}")
    
    print("\n" + "="*60 + "\n")


if __name__ == "__main__":
    main()
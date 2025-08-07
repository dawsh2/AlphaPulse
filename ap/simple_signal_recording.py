#!/usr/bin/env python3
"""
Simple example of recording strategy signals to Parquet.
"""

from decimal import Decimal
from pathlib import Path
import pandas as pd
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


# Global list to store signals
SIGNAL_TRACES = []


# Monkey patch the EMACross strategy to record signals
original_on_bar = EMACross.on_bar

def on_bar_with_recording(self, bar: Bar) -> None:
    """Extended on_bar that records signals."""
    
    # Call original logic
    original_on_bar(self, bar)
    
    # Record signal data if indicators are ready
    if hasattr(self, 'fast_ema') and hasattr(self, 'slow_ema'):
        if self.fast_ema.initialized and self.slow_ema.initialized:
            # Determine signal
            if self.fast_ema.value > self.slow_ema.value:
                signal = 1  # Long
            elif self.fast_ema.value < self.slow_ema.value:
                signal = -1  # Short
            else:
                signal = 0  # Neutral
            
            # Get position info
            positions = self.cache.positions_open(
                venue=self.config.instrument_id.venue,
                instrument_id=self.config.instrument_id,
                strategy_id=self.id,
            )
            
            position_side = 'FLAT'
            position_size = 0.0
            unrealized_pnl = 0.0
            
            if positions:
                pos = positions[0]
                position_side = pos.side.name
                position_size = float(pos.quantity)
                unrealized_pnl = float(pos.unrealized_pnl(bar.close).as_double())
            
            # Record trace
            trace = {
                'timestamp': bar.ts_event,
                'datetime': datetime.fromtimestamp(bar.ts_event / 1e9),
                'symbol': 'NVDA',
                'close': float(bar.close),
                'volume': int(bar.volume),
                'fast_ema': float(self.fast_ema.value),
                'slow_ema': float(self.slow_ema.value),
                'ema_diff': float(self.fast_ema.value - self.slow_ema.value),
                'signal': signal,
                'position_side': position_side,
                'position_size': position_size,
                'unrealized_pnl': unrealized_pnl,
                'strategy_id': str(self.id),
            }
            
            SIGNAL_TRACES.append(trace)

# Apply the patch
EMACross.on_bar = on_bar_with_recording


def main():
    """Run backtest and save signals."""
    
    print("\n" + "="*60)
    print("SIMPLE SIGNAL RECORDING EXAMPLE")
    print("="*60)
    
    # Load minimal data
    catalog_path = Path.cwd() / "catalog"
    catalog = ParquetDataCatalog(catalog_path)
    
    bars = catalog.query(
        data_cls=Bar,
        identifiers=["NVDA.ALPACA-1-MINUTE-LAST-EXTERNAL"],
    )[:1000]  # Just 1000 bars
    
    print(f"\nUsing {len(bars)} bars for demo")
    
    # Setup backtest
    venue = Venue("ALPACA")
    instrument_id = InstrumentId(Symbol("NVDA"), venue)
    bar_type = BarType.from_str("NVDA.ALPACA-1-MINUTE-LAST-EXTERNAL")
    
    config = BacktestEngineConfig(
        trader_id="DEMO-001",
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
    
    # Add strategy
    strategy_config = EMACrossConfig(
        instrument_id=instrument_id,
        bar_type=bar_type,
        fast_ema_period=10,
        slow_ema_period=30,
        trade_size=Decimal(100),
    )
    
    strategy = EMACross(config=strategy_config)
    engine.add_strategy(strategy)
    
    # Run backtest
    print("\nRunning backtest...")
    engine.run()
    
    # Save signals to Parquet
    print(f"\nRecorded {len(SIGNAL_TRACES)} signals")
    
    if SIGNAL_TRACES:
        # Convert to DataFrame
        df = pd.DataFrame(SIGNAL_TRACES)
        
        # Create output directory
        output_dir = Path("signal_traces")
        output_dir.mkdir(exist_ok=True)
        
        # Save to Parquet
        output_file = output_dir / "nvda_signals_demo.parquet"
        df.to_parquet(output_file, index=False, compression='snappy')
        
        print(f"Saved to: {output_file}")
        
        # Show sample
        print("\nFirst 5 signals:")
        print(df[['datetime', 'close', 'fast_ema', 'slow_ema', 'signal', 'position_side']].head())
        
        print("\nLast 5 signals:")
        print(df[['datetime', 'close', 'fast_ema', 'slow_ema', 'signal', 'position_side']].tail())
        
        # Signal statistics
        print("\nSignal Statistics:")
        print(f"Long signals: {len(df[df['signal'] == 1])}")
        print(f"Short signals: {len(df[df['signal'] == -1])}")
        print(f"Neutral signals: {len(df[df['signal'] == 0])}")
        
        # Position changes
        position_changes = df[df['signal'] != df['signal'].shift()]
        print(f"\nTotal position changes: {len(position_changes)}")
        
        # File info
        file_size = output_file.stat().st_size / 1024
        print(f"\nFile size: {file_size:.1f} KB")
        print(f"Compression ratio: {len(df) * 100 / file_size:.1f} rows/KB")


if __name__ == "__main__":
    main()
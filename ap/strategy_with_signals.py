#!/usr/bin/env python3
"""
Enhanced EMA Cross strategy that records signal traces to Parquet.
"""

from decimal import Decimal
from pathlib import Path
import pandas as pd
from datetime import datetime

from nautilus_trader.core.datetime import dt_to_unix_nanos
from nautilus_trader.indicators.average.ema import ExponentialMovingAverage
from nautilus_trader.model.data import Bar, BarType
from nautilus_trader.model.enums import OrderSide
from nautilus_trader.model.identifiers import InstrumentId
from nautilus_trader.model.instruments import Instrument
from nautilus_trader.model.orders import MarketOrder
from nautilus_trader.trading.strategy import Strategy, StrategyConfig

import sys
sys.path.insert(0, str(Path(__file__).parent / "nt_reference"))
from examples.strategies.ema_cross import EMACrossConfig


class EMACrossWithSignals(Strategy):
    """
    EMA Cross strategy that records all signals to a trace file.
    """
    
    def __init__(self, config: EMACrossConfig) -> None:
        super().__init__(config)
        
        self.instrument: Instrument = None
        self.fast_ema = ExponentialMovingAverage(config.fast_ema_period)
        self.slow_ema = ExponentialMovingAverage(config.slow_ema_period)
        
        # Signal trace storage
        self.signal_traces = []
        self._last_position = 0  # -1 short, 0 flat, 1 long
        
    def on_start(self) -> None:
        """Initialize strategy."""
        self.instrument = self.cache.instrument(self.config.instrument_id)
        if self.instrument is None:
            self.log.error(f"Could not find instrument for {self.config.instrument_id}")
            self.stop()
            return
            
        # Register indicators
        self.register_indicator_for_bars(self.config.bar_type, self.fast_ema)
        self.register_indicator_for_bars(self.config.bar_type, self.slow_ema)
        
        # Subscribe to bars
        self.subscribe_bars(self.config.bar_type)
        
    def on_bar(self, bar: Bar) -> None:
        """Process bar and record signals."""
        
        # Skip if indicators not ready
        if not self.fast_ema.initialized or not self.slow_ema.initialized:
            return
            
        # Get current values
        fast_value = self.fast_ema.value
        slow_value = self.slow_ema.value
        
        # Calculate signal
        if fast_value > slow_value:
            signal = 1  # Long signal
        elif fast_value < slow_value:
            signal = -1  # Short signal
        else:
            signal = 0  # Neutral
            
        # Record signal trace
        trace = {
            'timestamp': bar.ts_event,
            'datetime': datetime.fromtimestamp(bar.ts_event / 1e9),
            'symbol': str(self.config.instrument_id.symbol),
            'open': float(bar.open),
            'high': float(bar.high),
            'low': float(bar.low),
            'close': float(bar.close),
            'volume': int(bar.volume),
            'fast_ema': float(fast_value),
            'slow_ema': float(slow_value),
            'ema_diff': float(fast_value - slow_value),
            'ema_diff_pct': float((fast_value - slow_value) / slow_value * 100),
            'signal': signal,
            'position': self._last_position,
            'strategy_id': str(self.id),
        }
        
        # Add any active order info
        orders = self.cache.orders_open(
            venue=self.config.instrument_id.venue,
            instrument_id=self.config.instrument_id,
            strategy_id=self.id,
        )
        trace['open_orders'] = len(orders)
        
        # Add position info
        positions = self.cache.positions_open(
            venue=self.config.instrument_id.venue,
            instrument_id=self.config.instrument_id,
            strategy_id=self.id,
        )
        
        if positions:
            position = positions[0]
            trace['position_size'] = float(position.quantity)
            trace['position_side'] = position.side.name
            trace['unrealized_pnl'] = float(position.unrealized_pnl(bar.close).as_double())
        else:
            trace['position_size'] = 0.0
            trace['position_side'] = 'FLAT'
            trace['unrealized_pnl'] = 0.0
        
        self.signal_traces.append(trace)
        
        # Execute trading logic
        if signal != self._last_position:
            self._execute_signal(signal, bar)
            self._last_position = signal
            
    def _execute_signal(self, signal: int, bar: Bar) -> None:
        """Execute trades based on signal."""
        
        # Close any open positions first
        for position in self.cache.positions_open(
            venue=self.config.instrument_id.venue,
            instrument_id=self.config.instrument_id,
            strategy_id=self.id,
        ):
            self.close_position(position)
            
        # Open new position based on signal
        if signal == 1:  # Long
            self._go_long()
        elif signal == -1:  # Short
            self._go_short()
            
    def _go_long(self) -> None:
        """Enter long position."""
        order = self.order_factory.market(
            instrument_id=self.config.instrument_id,
            order_side=OrderSide.BUY,
            quantity=self.instrument.make_qty(self.config.trade_size),
        )
        self.submit_order(order)
        
    def _go_short(self) -> None:
        """Enter short position."""
        order = self.order_factory.market(
            instrument_id=self.config.instrument_id,
            order_side=OrderSide.SELL,
            quantity=self.instrument.make_qty(self.config.trade_size),
        )
        self.submit_order(order)
        
    def on_stop(self) -> None:
        """Save signal traces when strategy stops."""
        if self.signal_traces:
            self.save_signal_traces()
            
    def save_signal_traces(self, filepath: str = None) -> None:
        """Save signal traces to Parquet file."""
        if not self.signal_traces:
            self.log.warning("No signal traces to save")
            return
            
        # Convert to DataFrame
        df = pd.DataFrame(self.signal_traces)
        
        # Set timestamp as index
        df.set_index('timestamp', inplace=True)
        
        # Generate filename if not provided
        if filepath is None:
            timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
            filepath = f"signal_traces_{self.id}_{timestamp}.parquet"
            
        # Save to Parquet
        df.to_parquet(filepath, compression='snappy')
        
        self.log.info(f"Saved {len(df)} signal traces to {filepath}")
        
        # Also save summary statistics
        summary = {
            'strategy_id': str(self.id),
            'instrument': str(self.config.instrument_id),
            'fast_ema_period': self.config.fast_ema_period,
            'slow_ema_period': self.config.slow_ema_period,
            'total_signals': len(df),
            'long_signals': len(df[df['signal'] == 1]),
            'short_signals': len(df[df['signal'] == -1]),
            'neutral_signals': len(df[df['signal'] == 0]),
            'date_range': f"{df['datetime'].min()} to {df['datetime'].max()}",
        }
        
        summary_file = filepath.replace('.parquet', '_summary.txt')
        with open(summary_file, 'w') as f:
            for key, value in summary.items():
                f.write(f"{key}: {value}\n")
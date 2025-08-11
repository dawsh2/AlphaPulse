"""
EMA Cross Strategy for NautilusTrader
A simple exponential moving average crossover strategy.
"""

from nautilus_trader.trading.strategy import Strategy
from nautilus_trader.indicators.ema import ExponentialMovingAverage
from nautilus_trader.model.identifiers import InstrumentId
from nautilus_trader.model.data import Bar, BarType
from nautilus_trader.model.enums import OrderSide, TimeInForce
from nautilus_trader.model.orders import MarketOrder


class EMACrossStrategy(Strategy):
    """
    A simple EMA crossover strategy that generates signals when 
    fast EMA crosses slow EMA.
    """
    
    def __init__(
        self,
        instrument_id: InstrumentId,
        bar_type: BarType,
        fast_period: int = 10,
        slow_period: int = 30,
        trade_size: float = 1.0,
    ):
        super().__init__()
        
        # Configuration
        self.instrument_id = instrument_id
        self.bar_type = bar_type
        self.trade_size = trade_size
        
        # Create indicators
        self.fast_ema = ExponentialMovingAverage(fast_period)
        self.slow_ema = ExponentialMovingAverage(slow_period)
        
        # State
        self.position_side = None
        
    def on_start(self):
        """Called when the strategy starts."""
        self.register_indicator_for_bars(self.bar_type, self.fast_ema)
        self.register_indicator_for_bars(self.bar_type, self.slow_ema)
        self.subscribe_bars(self.bar_type)
        
    def on_bar(self, bar: Bar):
        """Handle bar data."""
        # Update indicators
        self.fast_ema.handle_bar(bar)
        self.slow_ema.handle_bar(bar)
        
        # Check if indicators are ready
        if not self.fast_ema.initialized or not self.slow_ema.initialized:
            return
            
        # Get current values
        fast_value = self.fast_ema.value
        slow_value = self.slow_ema.value
        
        # Generate signals
        if fast_value > slow_value and self.position_side != OrderSide.BUY:
            self._go_long()
        elif fast_value < slow_value and self.position_side != OrderSide.SELL:
            self._go_short()
            
    def _go_long(self):
        """Enter or flip to long position."""
        # Close short if exists
        if self.position_side == OrderSide.SELL:
            self._close_position()
            
        # Open long
        order = self.order_factory.market(
            instrument_id=self.instrument_id,
            order_side=OrderSide.BUY,
            quantity=self.trade_size,
            time_in_force=TimeInForce.IOC,
        )
        self.submit_order(order)
        self.position_side = OrderSide.BUY
        
    def _go_short(self):
        """Enter or flip to short position."""
        # Close long if exists
        if self.position_side == OrderSide.BUY:
            self._close_position()
            
        # Open short
        order = self.order_factory.market(
            instrument_id=self.instrument_id,
            order_side=OrderSide.SELL,
            quantity=self.trade_size,
            time_in_force=TimeInForce.IOC,
        )
        self.submit_order(order)
        self.position_side = OrderSide.SELL
        
    def _close_position(self):
        """Close current position."""
        if self.position_side == OrderSide.BUY:
            # Close long
            order = self.order_factory.market(
                instrument_id=self.instrument_id,
                order_side=OrderSide.SELL,
                quantity=self.trade_size,
                time_in_force=TimeInForce.IOC,
            )
            self.submit_order(order)
        elif self.position_side == OrderSide.SELL:
            # Close short
            order = self.order_factory.market(
                instrument_id=self.instrument_id,
                order_side=OrderSide.BUY,
                quantity=self.trade_size,
                time_in_force=TimeInForce.IOC,
            )
            self.submit_order(order)
            
        self.position_side = None
        
    def on_stop(self):
        """Called when the strategy stops."""
        # Close any open positions
        if self.position_side is not None:
            self._close_position()
"""
Momentum Strategy for NautilusTrader
Trades based on momentum indicators like RSI and rate of change.
"""

from nautilus_trader.trading.strategy import Strategy
from nautilus_trader.indicators.rsi import RelativeStrengthIndex
from nautilus_trader.model.identifiers import InstrumentId
from nautilus_trader.model.data import Bar, BarType
from nautilus_trader.model.enums import OrderSide, TimeInForce
from decimal import Decimal


class MomentumStrategy(Strategy):
    """
    Momentum-based trading strategy using RSI for overbought/oversold signals.
    """
    
    def __init__(
        self,
        instrument_id: InstrumentId,
        bar_type: BarType,
        rsi_period: int = 14,
        rsi_overbought: float = 70.0,
        rsi_oversold: float = 30.0,
        trade_size: float = 1.0,
    ):
        super().__init__()
        
        # Configuration
        self.instrument_id = instrument_id
        self.bar_type = bar_type
        self.trade_size = Decimal(str(trade_size))
        self.rsi_overbought = rsi_overbought
        self.rsi_oversold = rsi_oversold
        
        # Indicators
        self.rsi = RelativeStrengthIndex(rsi_period)
        
        # State
        self.in_position = False
        self.position_side = None
        
    def on_start(self):
        """Initialize the strategy."""
        self.register_indicator_for_bars(self.bar_type, self.rsi)
        self.subscribe_bars(self.bar_type)
        
    def on_bar(self, bar: Bar):
        """Process new bar data."""
        # Update indicators
        self.rsi.handle_bar(bar)
        
        # Wait for indicator to be ready
        if not self.rsi.initialized:
            return
            
        rsi_value = self.rsi.value
        
        # Generate trading signals
        if rsi_value < self.rsi_oversold and not self.in_position:
            # Oversold - potential buy signal
            self._enter_long()
            
        elif rsi_value > self.rsi_overbought and self.in_position and self.position_side == OrderSide.BUY:
            # Overbought - exit long
            self._exit_position()
            
        elif rsi_value > self.rsi_overbought and not self.in_position:
            # Overbought - potential short signal
            self._enter_short()
            
        elif rsi_value < self.rsi_oversold and self.in_position and self.position_side == OrderSide.SELL:
            # Oversold - exit short
            self._exit_position()
            
    def _enter_long(self):
        """Enter a long position."""
        order = self.order_factory.market(
            instrument_id=self.instrument_id,
            order_side=OrderSide.BUY,
            quantity=self.trade_size,
            time_in_force=TimeInForce.IOC,
        )
        self.submit_order(order)
        self.in_position = True
        self.position_side = OrderSide.BUY
        self._log.info(f"Entering LONG position at RSI {self.rsi.value:.2f}")
        
    def _enter_short(self):
        """Enter a short position."""
        order = self.order_factory.market(
            instrument_id=self.instrument_id,
            order_side=OrderSide.SELL,
            quantity=self.trade_size,
            time_in_force=TimeInForce.IOC,
        )
        self.submit_order(order)
        self.in_position = True
        self.position_side = OrderSide.SELL
        self._log.info(f"Entering SHORT position at RSI {self.rsi.value:.2f}")
        
    def _exit_position(self):
        """Exit current position."""
        if not self.in_position:
            return
            
        exit_side = OrderSide.SELL if self.position_side == OrderSide.BUY else OrderSide.BUY
        order = self.order_factory.market(
            instrument_id=self.instrument_id,
            order_side=exit_side,
            quantity=self.trade_size,
            time_in_force=TimeInForce.IOC,
        )
        self.submit_order(order)
        self.in_position = False
        self._log.info(f"Exiting position at RSI {self.rsi.value:.2f}")
        
    def on_stop(self):
        """Clean up when strategy stops."""
        if self.in_position:
            self._exit_position()
"""
Custom RSI Indicator
Enhanced RSI with divergence detection capabilities.
"""

from nautilus_trader.indicators.base import Indicator
from nautilus_trader.model.data import Bar
from collections import deque
import numpy as np


class CustomRSI(Indicator):
    """
    Custom RSI indicator with additional features for divergence detection.
    """
    
    def __init__(self, period: int = 14, smoothing: str = 'rma'):
        super().__init__(params=[("period", period), ("smoothing", smoothing)])
        self.period = period
        self.smoothing = smoothing
        
        # Price and RSI history for divergence detection
        self.price_history = deque(maxlen=period * 3)
        self.rsi_history = deque(maxlen=period * 3)
        
        # Internal calculation buffers
        self.gains = deque(maxlen=period)
        self.losses = deque(maxlen=period)
        self._value = 50.0  # Start neutral
        
        # Divergence tracking
        self.bullish_divergence = False
        self.bearish_divergence = False
        
    @property
    def value(self) -> float:
        """Current RSI value."""
        return self._value
        
    def handle_bar(self, bar: Bar) -> None:
        """Update RSI with new bar data."""
        self._increment_count()
        
        if self._count == 1:
            # First bar, just store the close price
            self.prev_close = float(bar.close)
            return
            
        # Calculate price change
        close_price = float(bar.close)
        change = close_price - self.prev_close
        
        # Separate gains and losses
        gain = max(0, change)
        loss = max(0, -change)
        
        self.gains.append(gain)
        self.losses.append(loss)
        
        # Need enough data
        if len(self.gains) < self.period:
            self.prev_close = close_price
            return
            
        # Calculate average gain and loss
        if self.smoothing == 'rma':
            # Relative Moving Average (Wilder's smoothing)
            avg_gain = sum(self.gains) / self.period
            avg_loss = sum(self.losses) / self.period
        else:
            # Simple Moving Average
            avg_gain = np.mean(self.gains)
            avg_loss = np.mean(self.losses)
            
        # Calculate RSI
        if avg_loss == 0:
            self._value = 100.0
        else:
            rs = avg_gain / avg_loss
            self._value = 100.0 - (100.0 / (1.0 + rs))
            
        # Store history for divergence detection
        self.price_history.append(close_price)
        self.rsi_history.append(self._value)
        
        # Check for divergences
        if len(self.price_history) >= 20:
            self._check_divergences()
            
        self.prev_close = close_price
        self._set_initialized(True)
        
    def _check_divergences(self):
        """Check for bullish and bearish divergences."""
        if len(self.price_history) < 20:
            return
            
        # Convert to arrays for easier analysis
        prices = np.array(self.price_history)
        rsi_values = np.array(self.rsi_history)
        
        # Find local minima and maxima
        price_peaks = self._find_peaks(prices)
        price_troughs = self._find_troughs(prices)
        rsi_peaks = self._find_peaks(rsi_values)
        rsi_troughs = self._find_troughs(rsi_values)
        
        # Check for bullish divergence (price makes lower low, RSI makes higher low)
        if len(price_troughs) >= 2 and len(rsi_troughs) >= 2:
            if (prices[price_troughs[-1]] < prices[price_troughs[-2]] and
                rsi_values[rsi_troughs[-1]] > rsi_values[rsi_troughs[-2]]):
                self.bullish_divergence = True
            else:
                self.bullish_divergence = False
                
        # Check for bearish divergence (price makes higher high, RSI makes lower high)
        if len(price_peaks) >= 2 and len(rsi_peaks) >= 2:
            if (prices[price_peaks[-1]] > prices[price_peaks[-2]] and
                rsi_values[rsi_peaks[-1]] < rsi_values[rsi_peaks[-2]]):
                self.bearish_divergence = True
            else:
                self.bearish_divergence = False
                
    def _find_peaks(self, data):
        """Find local maxima in data."""
        peaks = []
        for i in range(1, len(data) - 1):
            if data[i] > data[i-1] and data[i] > data[i+1]:
                peaks.append(i)
        return peaks
        
    def _find_troughs(self, data):
        """Find local minima in data."""
        troughs = []
        for i in range(1, len(data) - 1):
            if data[i] < data[i-1] and data[i] < data[i+1]:
                troughs.append(i)
        return troughs
        
    def reset(self):
        """Reset the indicator state."""
        super().reset()
        self.gains.clear()
        self.losses.clear()
        self.price_history.clear()
        self.rsi_history.clear()
        self._value = 50.0
        self.bullish_divergence = False
        self.bearish_divergence = False
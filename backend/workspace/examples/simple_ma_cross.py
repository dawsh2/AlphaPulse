"""
Simple Moving Average Crossover Strategy
A basic example strategy for AlphaPulse
"""

import numpy as np
import pandas as pd
from typing import Optional

class SimpleMAStrategy:
    def __init__(self, fast_period: int = 10, slow_period: int = 20):
        self.fast_period = fast_period
        self.slow_period = slow_period
        self.position = 0
        
    def calculate_signals(self, data: pd.DataFrame) -> pd.DataFrame:
        """Calculate trading signals based on MA crossover"""
        # Calculate moving averages
        data['ma_fast'] = data['close'].rolling(self.fast_period).mean()
        data['ma_slow'] = data['close'].rolling(self.slow_period).mean()
        
        # Generate signals
        data['signal'] = 0
        data.loc[data['ma_fast'] > data['ma_slow'], 'signal'] = 1
        data.loc[data['ma_fast'] < data['ma_slow'], 'signal'] = -1
        
        return data
    
    def backtest(self, data: pd.DataFrame) -> dict:
        """Run a simple backtest"""
        data = self.calculate_signals(data)
        
        # Calculate returns
        data['returns'] = data['close'].pct_change()
        data['strategy_returns'] = data['signal'].shift(1) * data['returns']
        
        # Calculate metrics
        total_return = (1 + data['strategy_returns']).prod() - 1
        sharpe_ratio = data['strategy_returns'].mean() / data['strategy_returns'].std() * np.sqrt(252)
        
        return {
            'total_return': total_return,
            'sharpe_ratio': sharpe_ratio,
            'num_trades': data['signal'].diff().abs().sum() / 2
        }

if __name__ == "__main__":
    print("Simple MA Crossover Strategy loaded successfully!")
    print("To run a backtest, load some market data and call strategy.backtest(data)")

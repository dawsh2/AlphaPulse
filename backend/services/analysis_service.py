"""
Analysis Service Layer - Business logic for market data analysis
Handles statistical analysis, risk metrics, and backtesting
"""
from typing import Dict, Any, Optional, List
import numpy as np
import pandas as pd
import logging

from data_manager import DataManager

logger = logging.getLogger(__name__)

class MarketAnalysisService:
    """Service layer for market analysis operations"""
    
    def __init__(self, data_manager: DataManager = None):
        self.data_manager = data_manager or DataManager()
    
    async def calculate_basic_statistics(self, symbol: str, exchange: str = "coinbase") -> Dict[str, float]:
        """Calculate basic statistics for a symbol
        
        Args:
            symbol: Trading symbol
            exchange: Exchange name
            
        Returns:
            Dictionary of statistics
        """
        try:
            stats = self.data_manager.calculate_statistics(symbol, exchange)
            return stats
        except Exception as e:
            logger.error(f"Statistics calculation failed for {symbol}: {e}")
            raise
    
    async def calculate_correlation_matrix(self, symbols: List[str], exchange: str = "coinbase") -> Dict[str, Any]:
        """Calculate correlation matrix for multiple symbols
        
        Args:
            symbols: List of symbols
            exchange: Exchange name
            
        Returns:
            Correlation matrix and statistics
        """
        try:
            correlations = {}
            statistics = {}
            
            # Calculate pairwise correlations
            for i, symbol1 in enumerate(symbols):
                correlations[symbol1] = {}
                statistics[symbol1] = await self.calculate_basic_statistics(symbol1, exchange)
                
                for j, symbol2 in enumerate(symbols):
                    if i == j:
                        correlations[symbol1][symbol2] = 1.0
                    elif j > i:  # Only calculate upper triangle
                        corr = self.data_manager.calculate_correlation(symbol1, symbol2, exchange)
                        correlations[symbol1][symbol2] = corr
                        # Mirror to lower triangle
                        if symbol2 not in correlations:
                            correlations[symbol2] = {}
                        correlations[symbol2][symbol1] = corr
            
            return {
                'correlations': correlations,
                'statistics': statistics,
                'symbols': symbols,
                'exchange': exchange
            }
        except Exception as e:
            logger.error(f"Correlation matrix calculation failed: {e}")
            raise
    
    async def calculate_rolling_statistics(
        self, 
        symbol: str, 
        window: int = 20, 
        exchange: str = "coinbase"
    ) -> Dict[str, Any]:
        """Calculate rolling statistics for a symbol
        
        Args:
            symbol: Trading symbol
            window: Rolling window size
            exchange: Exchange name
            
        Returns:
            Rolling statistics data
        """
        try:
            df = self.data_manager.get_ohlcv(symbol, exchange)
            
            if df.empty:
                return {
                    'status': 'error',
                    'message': 'No data available for symbol'
                }
            
            # Calculate returns
            df['returns'] = df['close'].pct_change()
            
            # Calculate rolling statistics
            df['rolling_mean'] = df['close'].rolling(window=window).mean()
            df['rolling_std'] = df['returns'].rolling(window=window).std()
            df['rolling_sharpe'] = (df['returns'].rolling(window=window).mean() / 
                                   df['returns'].rolling(window=window).std()) * np.sqrt(252)
            
            # Upper and lower bands (Bollinger Bands)
            df['upper_band'] = df['rolling_mean'] + (2 * df['close'].rolling(window=window).std())
            df['lower_band'] = df['rolling_mean'] - (2 * df['close'].rolling(window=window).std())
            
            # Convert to output format
            result = []
            for _, row in df.dropna().iterrows():
                result.append({
                    'timestamp': int(row['timestamp']),
                    'close': float(row['close']),
                    'rolling_mean': float(row['rolling_mean']),
                    'rolling_std': float(row['rolling_std']),
                    'rolling_sharpe': float(row['rolling_sharpe']) if not pd.isna(row['rolling_sharpe']) else 0,
                    'upper_band': float(row['upper_band']),
                    'lower_band': float(row['lower_band'])
                })
            
            return {
                'symbol': symbol,
                'exchange': exchange,
                'window': window,
                'data': result
            }
        except Exception as e:
            logger.error(f"Rolling statistics calculation failed: {e}")
            raise
    
    async def calculate_risk_metrics(
        self, 
        symbol: str, 
        exchange: str = "coinbase", 
        risk_free_rate: float = 0.02
    ) -> Dict[str, Any]:
        """Calculate comprehensive risk metrics
        
        Args:
            symbol: Trading symbol
            exchange: Exchange name
            risk_free_rate: Annual risk-free rate
            
        Returns:
            Risk metrics dictionary
        """
        try:
            df = self.data_manager.get_ohlcv(symbol, exchange)
            
            if df.empty:
                return {
                    'status': 'error',
                    'message': 'No data available for symbol'
                }
            
            # Calculate returns
            df['returns'] = df['close'].pct_change()
            returns = df['returns'].dropna()
            
            # Basic metrics
            mean_return = returns.mean()
            std_return = returns.std()
            
            # Annualized metrics (assuming daily data)
            annual_return = mean_return * 252
            annual_volatility = std_return * np.sqrt(252)
            
            # Sharpe ratio
            sharpe_ratio = (annual_return - risk_free_rate) / annual_volatility if annual_volatility > 0 else 0
            
            # Maximum drawdown
            cumulative = (1 + returns).cumprod()
            running_max = cumulative.expanding().max()
            drawdown = (cumulative - running_max) / running_max
            max_drawdown = drawdown.min()
            
            # Value at Risk (95% confidence)
            var_95 = np.percentile(returns, 5)
            
            # Conditional Value at Risk (CVaR)
            cvar_95 = returns[returns <= var_95].mean()
            
            # Skewness and Kurtosis
            skewness = returns.skew()
            kurtosis = returns.kurtosis()
            
            return {
                'symbol': symbol,
                'exchange': exchange,
                'risk_free_rate': risk_free_rate,
                'metrics': {
                    'annual_return': float(annual_return),
                    'annual_volatility': float(annual_volatility),
                    'sharpe_ratio': float(sharpe_ratio),
                    'max_drawdown': float(max_drawdown),
                    'var_95': float(var_95),
                    'cvar_95': float(cvar_95),
                    'skewness': float(skewness),
                    'kurtosis': float(kurtosis),
                    'mean_daily_return': float(mean_return),
                    'std_daily_return': float(std_return)
                }
            }
        except Exception as e:
            logger.error(f"Risk metrics calculation failed: {e}")
            raise
    
    async def perform_backtesting_analysis(self, strategy_config: Dict[str, Any]) -> Dict[str, Any]:
        """Perform backtesting analysis
        
        Args:
            strategy_config: Strategy configuration
            
        Returns:
            Backtest results
        """
        try:
            # This is a placeholder for more complex backtesting logic
            # In production, this would integrate with a backtesting engine
            
            symbol = strategy_config.get('symbol')
            exchange = strategy_config.get('exchange', 'coinbase')
            strategy_type = strategy_config.get('type', 'simple_ma_cross')
            
            df = self.data_manager.get_ohlcv(symbol, exchange)
            
            if df.empty:
                return {
                    'status': 'error',
                    'message': 'No data available for backtesting'
                }
            
            # Simple moving average crossover strategy as example
            if strategy_type == 'simple_ma_cross':
                fast_period = strategy_config.get('fast_period', 10)
                slow_period = strategy_config.get('slow_period', 20)
                
                df['ma_fast'] = df['close'].rolling(window=fast_period).mean()
                df['ma_slow'] = df['close'].rolling(window=slow_period).mean()
                
                # Generate signals
                df['signal'] = 0
                df.loc[df['ma_fast'] > df['ma_slow'], 'signal'] = 1
                df.loc[df['ma_fast'] < df['ma_slow'], 'signal'] = -1
                
                # Calculate returns
                df['returns'] = df['close'].pct_change()
                df['strategy_returns'] = df['signal'].shift(1) * df['returns']
                
                # Calculate metrics
                total_return = (1 + df['strategy_returns'].dropna()).prod() - 1
                sharpe = df['strategy_returns'].mean() / df['strategy_returns'].std() * np.sqrt(252)
                max_dd = self._calculate_max_drawdown(df['strategy_returns'].dropna())
                
                return {
                    'status': 'success',
                    'strategy': strategy_type,
                    'parameters': {
                        'fast_period': fast_period,
                        'slow_period': slow_period
                    },
                    'results': {
                        'total_return': float(total_return),
                        'sharpe_ratio': float(sharpe),
                        'max_drawdown': float(max_dd),
                        'num_trades': int(df['signal'].diff().abs().sum() / 2)
                    }
                }
            
            return {
                'status': 'error',
                'message': f'Unknown strategy type: {strategy_type}'
            }
            
        except Exception as e:
            logger.error(f"Backtesting failed: {e}")
            raise
    
    async def get_market_regime_analysis(self, symbols: List[str], exchange: str = "coinbase") -> Dict[str, Any]:
        """Analyze market regime for multiple symbols
        
        Args:
            symbols: List of symbols
            exchange: Exchange name
            
        Returns:
            Market regime analysis
        """
        try:
            regime_data = {}
            
            for symbol in symbols:
                df = self.data_manager.get_ohlcv(symbol, exchange)
                
                if df.empty:
                    regime_data[symbol] = {'status': 'no_data'}
                    continue
                
                # Calculate returns and volatility
                df['returns'] = df['close'].pct_change()
                
                # 20-day and 50-day moving averages
                df['ma20'] = df['close'].rolling(window=20).mean()
                df['ma50'] = df['close'].rolling(window=50).mean()
                
                # Current regime
                latest = df.iloc[-1]
                trend = 'bullish' if latest['close'] > latest['ma50'] else 'bearish'
                momentum = 'strong' if abs(latest['close'] - latest['ma20']) / latest['ma20'] > 0.05 else 'weak'
                
                # Volatility regime
                recent_vol = df['returns'].tail(20).std() * np.sqrt(252)
                vol_regime = 'high' if recent_vol > 0.5 else 'normal' if recent_vol > 0.2 else 'low'
                
                regime_data[symbol] = {
                    'trend': trend,
                    'momentum': momentum,
                    'volatility_regime': vol_regime,
                    'current_price': float(latest['close']),
                    'ma20': float(latest['ma20']) if not pd.isna(latest['ma20']) else None,
                    'ma50': float(latest['ma50']) if not pd.isna(latest['ma50']) else None,
                    'annualized_volatility': float(recent_vol)
                }
            
            return {
                'symbols': symbols,
                'exchange': exchange,
                'regime_analysis': regime_data
            }
            
        except Exception as e:
            logger.error(f"Market regime analysis failed: {e}")
            raise
    
    def _calculate_max_drawdown(self, returns: pd.Series) -> float:
        """Calculate maximum drawdown from returns series"""
        cumulative = (1 + returns).cumprod()
        running_max = cumulative.expanding().max()
        drawdown = (cumulative - running_max) / running_max
        return float(drawdown.min())
    
    def close(self):
        """Cleanup resources"""
        if self.data_manager:
            self.data_manager.close()

# The dependency function is now in core.container
# Import it from there when needed in route files
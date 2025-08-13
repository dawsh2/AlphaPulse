"""
Analysis Service - Statistical analysis and calculations
Business logic layer for market data analysis
"""
from typing import Dict, Any, Optional, List
import numpy as np
import pandas as pd
from data_manager import DataManager


class AnalysisService:
    """Service layer for statistical analysis operations"""
    
    def __init__(self, data_manager: DataManager = None):
        self.data_manager = data_manager or DataManager()
    
    def calculate_basic_statistics(self, symbol: str, exchange: str = "coinbase") -> Dict[str, float]:
        """Calculate basic statistics for a symbol"""
        try:
            return self.data_manager.calculate_statistics(symbol, exchange)
        except Exception as e:
            raise Exception(f"Statistics calculation failed for {symbol}: {str(e)}")
    
    def calculate_correlation_matrix(self, symbols: List[str], exchange: str = "coinbase") -> Dict[str, Any]:
        """Calculate correlation matrix for multiple symbols"""
        try:
            correlations = {}
            statistics = {}
            
            # Calculate pairwise correlations
            for i, symbol1 in enumerate(symbols):
                correlations[symbol1] = {}
                statistics[symbol1] = self.calculate_basic_statistics(symbol1, exchange)
                
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
                'symbols': symbols
            }
            
        except Exception as e:
            raise Exception(f"Correlation matrix calculation failed: {str(e)}")
    
    def calculate_rolling_statistics(self, symbol: str, window: int = 20, exchange: str = "coinbase") -> Dict[str, Any]:
        """Calculate rolling statistics for a symbol"""
        try:
            # Get OHLCV data with returns
            df = self.data_manager.get_returns(symbol, exchange)
            
            if df.empty:
                raise ValueError(f"No data available for {symbol}")
            
            # Calculate rolling statistics
            rolling_stats = {
                'rolling_mean': df['log_returns'].rolling(window).mean().tolist(),
                'rolling_std': df['log_returns'].rolling(window).std().tolist(),
                'rolling_sharpe': (df['log_returns'].rolling(window).mean() / df['log_returns'].rolling(window).std()).tolist(),
                'rolling_min': df['log_returns'].rolling(window).min().tolist(),
                'rolling_max': df['log_returns'].rolling(window).max().tolist(),
                'timestamps': df['timestamp'].tolist()
            }
            
            # Remove NaN values for JSON serialization
            for key in rolling_stats:
                if key != 'timestamps':
                    rolling_stats[key] = [x if pd.notna(x) else None for x in rolling_stats[key]]
            
            return {
                'symbol': symbol,
                'window': window,
                'data_points': len(df),
                'rolling_stats': rolling_stats
            }
            
        except Exception as e:
            raise Exception(f"Rolling statistics calculation failed: {str(e)}")
    
    def calculate_risk_metrics(self, symbol: str, exchange: str = "coinbase", risk_free_rate: float = 0.02) -> Dict[str, Any]:
        """Calculate comprehensive risk metrics"""
        try:
            df = self.data_manager.get_returns(symbol, exchange)
            
            if df.empty:
                raise ValueError(f"No data available for {symbol}")
            
            returns = df['log_returns'].dropna()
            
            if len(returns) == 0:
                raise ValueError(f"No valid returns data for {symbol}")
            
            # Basic statistics
            mean_return = returns.mean()
            volatility = returns.std()
            
            # Risk metrics
            var_95 = returns.quantile(0.05)  # Value at Risk (95%)
            var_99 = returns.quantile(0.01)  # Value at Risk (99%)
            
            # Expected Shortfall (Conditional VaR)
            es_95 = returns[returns <= var_95].mean()
            es_99 = returns[returns <= var_99].mean()
            
            # Maximum Drawdown calculation
            cumulative_returns = (1 + returns).cumprod()
            running_max = cumulative_returns.expanding().max()
            drawdown = (cumulative_returns - running_max) / running_max
            max_drawdown = drawdown.min()
            
            # Sharpe Ratio (annualized)
            excess_return = mean_return - (risk_free_rate / 365 / 24 / 60)  # Convert to per-minute
            sharpe_ratio = (excess_return / volatility) * np.sqrt(365 * 24 * 60) if volatility > 0 else 0
            
            # Sortino Ratio (downside deviation)
            downside_returns = returns[returns < 0]
            downside_deviation = downside_returns.std()
            sortino_ratio = (excess_return / downside_deviation) * np.sqrt(365 * 24 * 60) if downside_deviation > 0 else 0
            
            # Skewness and Kurtosis
            skewness = returns.skew()
            kurtosis = returns.kurtosis()
            
            return {
                'symbol': symbol,
                'data_points': len(returns),
                'mean_return_annualized': mean_return * 365 * 24 * 60,
                'volatility_annualized': volatility * np.sqrt(365 * 24 * 60),
                'sharpe_ratio': sharpe_ratio,
                'sortino_ratio': sortino_ratio,
                'var_95': var_95,
                'var_99': var_99,
                'expected_shortfall_95': es_95,
                'expected_shortfall_99': es_99,
                'max_drawdown': max_drawdown,
                'skewness': skewness,
                'kurtosis': kurtosis,
                'risk_free_rate': risk_free_rate
            }
            
        except Exception as e:
            raise Exception(f"Risk metrics calculation failed: {str(e)}")
    
    def perform_backtesting_analysis(self, strategy_config: Dict[str, Any]) -> Dict[str, Any]:
        """Perform backtesting analysis on a strategy"""
        try:
            # This is a placeholder for backtesting logic
            # In a real implementation, this would execute the strategy
            # against historical data and calculate performance metrics
            
            symbol = strategy_config.get('symbol', 'BTC/USD')
            
            # Get historical data
            df = self.data_manager.get_returns(symbol)
            
            if df.empty:
                raise ValueError(f"No data available for backtesting {symbol}")
            
            # Mock backtesting results for now
            # In practice, this would run the actual strategy logic
            returns = df['log_returns'].dropna()
            
            # Simulate strategy returns (random example)
            np.random.seed(42)  # For reproducible results
            strategy_returns = returns * (1 + np.random.normal(0, 0.1, len(returns)))
            
            # Calculate performance metrics
            total_return = strategy_returns.sum()
            annualized_return = total_return * 365 * 24 * 60 / len(returns)
            volatility = strategy_returns.std() * np.sqrt(365 * 24 * 60)
            sharpe_ratio = annualized_return / volatility if volatility > 0 else 0
            
            # Calculate drawdown
            cumulative_returns = (1 + strategy_returns).cumprod()
            running_max = cumulative_returns.expanding().max()
            drawdown = (cumulative_returns - running_max) / running_max
            max_drawdown = drawdown.min()
            
            return {
                'symbol': symbol,
                'strategy': strategy_config,
                'total_return': total_return,
                'annualized_return': annualized_return,
                'volatility': volatility,
                'sharpe_ratio': sharpe_ratio,
                'max_drawdown': max_drawdown,
                'total_trades': len(returns),
                'data_points': len(df),
                'backtest_period': {
                    'start': df['datetime'].min().isoformat() if not df.empty else None,
                    'end': df['datetime'].max().isoformat() if not df.empty else None
                }
            }
            
        except Exception as e:
            raise Exception(f"Backtesting analysis failed: {str(e)}")
    
    def get_market_regime_analysis(self, symbols: List[str], exchange: str = "coinbase") -> Dict[str, Any]:
        """Analyze market regimes across multiple symbols"""
        try:
            regime_data = {}
            
            for symbol in symbols:
                df = self.data_manager.get_returns(symbol, exchange)
                
                if df.empty:
                    continue
                
                returns = df['log_returns'].dropna()
                
                # Simple regime classification based on volatility and returns
                volatility = returns.rolling(20).std()
                mean_return = returns.rolling(20).mean()
                
                # Classify regimes
                high_vol = volatility > volatility.quantile(0.7)
                positive_trend = mean_return > 0
                
                regimes = []
                for i in range(len(returns)):
                    if pd.isna(volatility.iloc[i]) or pd.isna(mean_return.iloc[i]):
                        regimes.append('unknown')
                    elif high_vol.iloc[i] and positive_trend.iloc[i]:
                        regimes.append('bull_volatile')
                    elif high_vol.iloc[i] and not positive_trend.iloc[i]:
                        regimes.append('bear_volatile')
                    elif not high_vol.iloc[i] and positive_trend.iloc[i]:
                        regimes.append('bull_stable')
                    else:
                        regimes.append('bear_stable')
                
                # Calculate regime statistics
                regime_counts = pd.Series(regimes).value_counts()
                
                regime_data[symbol] = {
                    'regime_counts': regime_counts.to_dict(),
                    'current_regime': regimes[-1] if regimes else 'unknown',
                    'data_points': len(returns)
                }
            
            return {
                'symbols': symbols,
                'regime_analysis': regime_data,
                'regimes': ['bull_volatile', 'bear_volatile', 'bull_stable', 'bear_stable', 'unknown']
            }
            
        except Exception as e:
            raise Exception(f"Market regime analysis failed: {str(e)}")
    
    def close(self):
        """Close connections"""
        if hasattr(self.data_manager, 'close'):
            self.data_manager.close()
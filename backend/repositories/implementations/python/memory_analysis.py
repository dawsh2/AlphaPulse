"""
In-memory implementation of AnalysisRepository
Performs statistical calculations on market data
"""
from typing import Dict, Optional, Any
import numpy as np
import pandas as pd
import logging
from scipy import stats

from data_manager import DataManager

logger = logging.getLogger(__name__)


class MemoryAnalysisRepository:
    """
    In-memory implementation of AnalysisRepository protocol
    Performs calculations using pandas and numpy
    """
    
    def __init__(self, data_manager: Optional[DataManager] = None):
        """Initialize with DataManager for data access"""
        self.data_manager = data_manager or DataManager()
        logger.info("MemoryAnalysisRepository initialized")
    
    async def calculate_statistics(
        self,
        symbol: str,
        exchange: str,
        window: Optional[int] = None
    ) -> Dict[str, float]:
        """Calculate basic statistics for a symbol"""
        try:
            # Use existing DataManager method
            stats = self.data_manager.calculate_statistics(symbol, exchange)
            
            # Add window-specific calculations if requested
            if window:
                df = self.data_manager.get_ohlcv(symbol, exchange)
                if not df.empty and len(df) > window:
                    recent_df = df.tail(window)
                    returns = recent_df['close'].pct_change().dropna()
                    
                    stats.update({
                        f'mean_{window}d': float(returns.mean()),
                        f'std_{window}d': float(returns.std()),
                        f'sharpe_{window}d': float(returns.mean() / returns.std() * np.sqrt(252)) if returns.std() > 0 else 0
                    })
            
            return stats
        except Exception as e:
            logger.error(f"Failed to calculate statistics: {e}")
            return {}
    
    async def calculate_correlation(
        self,
        symbol1: str,
        symbol2: str,
        exchange: str,
        period: Optional[int] = None
    ) -> float:
        """Calculate correlation between two symbols"""
        try:
            # Use existing DataManager method
            correlation = self.data_manager.calculate_correlation(symbol1, symbol2, exchange)
            
            # If period specified, calculate for that specific period
            if period:
                df1 = self.data_manager.get_ohlcv(symbol1, exchange)
                df2 = self.data_manager.get_ohlcv(symbol2, exchange)
                
                if not df1.empty and not df2.empty:
                    # Align timestamps and get recent period
                    merged = pd.merge(
                        df1[['timestamp', 'close']].tail(period),
                        df2[['timestamp', 'close']].tail(period),
                        on='timestamp',
                        suffixes=('_1', '_2')
                    )
                    
                    if len(merged) > 1:
                        correlation = float(merged['close_1'].corr(merged['close_2']))
            
            return correlation
        except Exception as e:
            logger.error(f"Failed to calculate correlation: {e}")
            return 0.0
    
    async def calculate_volatility(
        self,
        symbol: str,
        exchange: str,
        window: int = 20
    ) -> float:
        """Calculate rolling volatility"""
        try:
            df = self.data_manager.get_ohlcv(symbol, exchange)
            
            if df.empty or len(df) < window:
                return 0.0
            
            # Calculate returns
            returns = df['close'].pct_change().dropna()
            
            # Calculate rolling volatility (annualized)
            volatility = returns.rolling(window=window).std().iloc[-1]
            annualized_vol = float(volatility * np.sqrt(252))
            
            return annualized_vol
        except Exception as e:
            logger.error(f"Failed to calculate volatility: {e}")
            return 0.0
    
    async def calculate_risk_metrics(
        self,
        symbol: str,
        exchange: str,
        risk_free_rate: float = 0.02
    ) -> Dict[str, float]:
        """Calculate risk metrics (Sharpe, Sortino, etc.)"""
        try:
            df = self.data_manager.get_ohlcv(symbol, exchange)
            
            if df.empty or len(df) < 2:
                return {}
            
            # Calculate returns
            returns = df['close'].pct_change().dropna()
            
            # Daily risk-free rate
            daily_rf = risk_free_rate / 252
            
            # Excess returns
            excess_returns = returns - daily_rf
            
            # Calculate metrics
            mean_return = returns.mean()
            std_return = returns.std()
            
            # Downside returns for Sortino
            downside_returns = returns[returns < 0]
            downside_std = downside_returns.std() if len(downside_returns) > 0 else std_return
            
            # Maximum drawdown
            cumulative = (1 + returns).cumprod()
            running_max = cumulative.expanding().max()
            drawdown = (cumulative - running_max) / running_max
            max_drawdown = drawdown.min()
            
            # Calmar ratio (annualized return / max drawdown)
            annual_return = mean_return * 252
            calmar = annual_return / abs(max_drawdown) if max_drawdown != 0 else 0
            
            metrics = {
                'daily_return': float(mean_return),
                'annual_return': float(annual_return),
                'volatility': float(std_return * np.sqrt(252)),
                'sharpe_ratio': float(excess_returns.mean() / std_return * np.sqrt(252)) if std_return > 0 else 0,
                'sortino_ratio': float(excess_returns.mean() / downside_std * np.sqrt(252)) if downside_std > 0 else 0,
                'max_drawdown': float(max_drawdown),
                'calmar_ratio': float(calmar),
                'skewness': float(returns.skew()),
                'kurtosis': float(returns.kurtosis()),
                'var_95': float(returns.quantile(0.05)),  # 95% Value at Risk
                'cvar_95': float(returns[returns <= returns.quantile(0.05)].mean())  # Conditional VaR
            }
            
            return metrics
        except Exception as e:
            logger.error(f"Failed to calculate risk metrics: {e}")
            return {}
    
    async def detect_regime(
        self,
        symbol: str,
        exchange: str
    ) -> Dict[str, Any]:
        """Detect market regime (trending, mean-reverting, etc.)"""
        try:
            df = self.data_manager.get_ohlcv(symbol, exchange)
            
            if df.empty or len(df) < 50:
                return {'regime': 'unknown', 'confidence': 0.0}
            
            # Calculate various indicators for regime detection
            returns = df['close'].pct_change().dropna()
            
            # Moving averages
            ma_short = df['close'].rolling(20).mean()
            ma_long = df['close'].rolling(50).mean()
            
            # Trend strength (ADX-like)
            price_range = df['high'] - df['low']
            avg_range = price_range.rolling(14).mean()
            trend_strength = abs(df['close'].iloc[-1] - df['close'].iloc[-20]) / (avg_range.iloc[-1] * 20)
            
            # Mean reversion test (Hurst exponent approximation)
            # Simplified version using variance ratio
            if len(returns) >= 100:
                var_short = returns.rolling(10).var().mean()
                var_long = returns.rolling(50).var().mean()
                variance_ratio = var_short / var_long if var_long > 0 else 1
            else:
                variance_ratio = 1
            
            # Volatility regime
            recent_vol = returns.tail(20).std()
            historical_vol = returns.std()
            vol_ratio = recent_vol / historical_vol if historical_vol > 0 else 1
            
            # Determine regime
            regime = 'unknown'
            confidence = 0.0
            
            if ma_short.iloc[-1] > ma_long.iloc[-1] and trend_strength > 0.5:
                regime = 'trending_up'
                confidence = min(trend_strength, 1.0)
            elif ma_short.iloc[-1] < ma_long.iloc[-1] and trend_strength > 0.5:
                regime = 'trending_down'
                confidence = min(trend_strength, 1.0)
            elif variance_ratio < 0.8:
                regime = 'mean_reverting'
                confidence = 1 - variance_ratio
            elif vol_ratio > 1.5:
                regime = 'high_volatility'
                confidence = min(vol_ratio / 2, 1.0)
            elif vol_ratio < 0.7:
                regime = 'low_volatility'
                confidence = 1 - vol_ratio
            else:
                regime = 'neutral'
                confidence = 0.5
            
            return {
                'regime': regime,
                'confidence': float(confidence),
                'indicators': {
                    'trend_strength': float(trend_strength),
                    'variance_ratio': float(variance_ratio),
                    'volatility_ratio': float(vol_ratio),
                    'ma_crossover': 'bullish' if ma_short.iloc[-1] > ma_long.iloc[-1] else 'bearish'
                }
            }
        except Exception as e:
            logger.error(f"Failed to detect regime: {e}")
            return {'regime': 'unknown', 'confidence': 0.0, 'error': str(e)}
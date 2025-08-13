"""
Unified schema definitions for AlphaPulse
All data models used across the application - JSON-serializable for Rust compatibility
"""
from pydantic import BaseModel, Field
from typing import Optional, List, Dict, Any, Union
from datetime import datetime
from enum import Enum
import pandas as pd


# Enums
class ExchangeEnum(str, Enum):
    coinbase = "coinbase"
    kraken = "kraken"
    binance = "binance"


class OrderSide(str, Enum):
    buy = "buy"
    sell = "sell"


class OrderType(str, Enum):
    market = "market"
    limit = "limit"


# Core Trading Types
class Trade(BaseModel):
    """Individual trade record"""
    timestamp: float
    price: float
    volume: float
    side: Optional[str] = None
    trade_id: Optional[str] = None


class OHLCVBar(BaseModel):
    """OHLCV candlestick bar"""
    timestamp: float
    open: float
    high: float
    low: float
    close: float
    volume: float


class OrderBookLevel(BaseModel):
    """Single level in orderbook"""
    price: float
    size: float


class OrderBookSnapshot(BaseModel):
    """Orderbook snapshot at specific time"""
    symbol: str
    exchange: str
    timestamp: float
    bids: List[OrderBookLevel]
    asks: List[OrderBookLevel]


# Analysis Types
class MarketStatistics(BaseModel):
    """Market statistics for a symbol"""
    symbol: str
    exchange: str
    mean_price: float
    volatility: float
    volume_avg: float
    high_24h: float
    low_24h: float
    price_change_24h: float


class RiskMetrics(BaseModel):
    """Risk metrics calculation"""
    symbol: str
    sharpe_ratio: float
    sortino_ratio: float
    max_drawdown: float
    var_95: float
    beta: Optional[float] = None


class MarketRegime(BaseModel):
    """Market regime detection result"""
    symbol: str
    regime: str  # trending, mean_reverting, volatile
    confidence: float
    detected_at: float


class DataSummary(BaseModel):
    """Summary of available data"""
    total_symbols: int
    total_exchanges: int
    total_records: int
    date_range: Dict[str, Any]
    symbols_by_exchange: Dict[str, List[str]]
    record_count_by_symbol: Dict[str, int]


# API Request/Response Models
class SaveDataRequest(BaseModel):
    """Request to save market data"""
    symbol: str
    exchange: str
    interval: str = "1m"
    candles: List[Dict[str, Any]]


class QueryRequest(BaseModel):
    """SQL query request"""
    query: str = Field(..., min_length=1, max_length=5000)


class QueryResult(BaseModel):
    """SQL query result"""
    columns: List[str]
    data: List[List[Any]]
    row_count: int
    execution_time: float


class CorrelationResponse(BaseModel):
    """Correlation analysis response"""
    symbol1: str
    symbol2: str
    exchange: str
    correlation: float
    data_points: int


class ApiResponse(BaseModel):
    """Generic API response wrapper"""
    status: str
    message: Optional[str] = None
    data: Optional[Any] = None


class AnalysisRequest(BaseModel):
    """Analysis request for multiple symbols"""
    symbols: List[str] = Field(..., min_items=1, max_items=20)
    exchange: ExchangeEnum = ExchangeEnum.coinbase


class RollingStatsRequest(BaseModel):
    """Rolling statistics request"""
    symbol: str
    exchange: str = "coinbase"
    window: int = Field(20, ge=1, le=500)


class RiskMetricsRequest(BaseModel):
    """Risk metrics calculation request"""
    symbol: str
    exchange: str = "coinbase"
    risk_free_rate: float = Field(0.02, ge=0, le=1)


class BacktestConfig(BaseModel):
    """Backtest configuration"""
    symbol: str
    exchange: str = "coinbase"
    start_time: Optional[datetime] = None
    end_time: Optional[datetime] = None
    initial_capital: float = Field(10000, gt=0)
    strategy_params: Dict[str, Any] = {}


class BacktestResult(BaseModel):
    """Backtest results"""
    config: BacktestConfig
    total_return: float
    sharpe_ratio: float
    max_drawdown: float
    total_trades: int
    win_rate: float
    profit_factor: float


class RegimeAnalysis(BaseModel):
    """Market regime analysis"""
    symbol: str
    exchange: str
    current_regime: MarketRegime
    regime_history: List[MarketRegime]


class StatisticsResult(BaseModel):
    """Statistics calculation result"""
    symbol: str
    exchange: str
    statistics: MarketStatistics
    calculated_at: datetime


# Data Conversion Functions
def dataframe_to_trades(df: pd.DataFrame) -> List[Trade]:
    """Convert pandas DataFrame to Trade objects"""
    trades = []
    for _, row in df.iterrows():
        trades.append(Trade(
            timestamp=float(row.get('timestamp', row.name.timestamp() if hasattr(row.name, 'timestamp') else 0)),
            price=float(row.get('price', row.get('close', 0))),
            volume=float(row.get('volume', row.get('size', 0))),
            side=row.get('side'),
            trade_id=row.get('trade_id')
        ))
    return trades


def dataframe_to_ohlcv(df: pd.DataFrame) -> List[OHLCVBar]:
    """Convert pandas DataFrame to OHLCV bars"""
    bars = []
    for _, row in df.iterrows():
        bars.append(OHLCVBar(
            timestamp=float(row.get('timestamp', row.name.timestamp() if hasattr(row.name, 'timestamp') else 0)),
            open=float(row.get('open', 0)),
            high=float(row.get('high', 0)),
            low=float(row.get('low', 0)),
            close=float(row.get('close', 0)),
            volume=float(row.get('volume', 0))
        ))
    return bars


def ohlcv_to_dataframe(bars: List[OHLCVBar]) -> pd.DataFrame:
    """Convert OHLCV bars to pandas DataFrame"""
    if not bars:
        return pd.DataFrame()
    
    data = []
    for bar in bars:
        data.append({
            'timestamp': bar.timestamp,
            'open': bar.open,
            'high': bar.high,
            'low': bar.low,
            'close': bar.close,
            'volume': bar.volume
        })
    
    df = pd.DataFrame(data)
    df['timestamp'] = pd.to_datetime(df['timestamp'], unit='s')
    df.set_index('timestamp', inplace=True)
    return df


def trades_to_dataframe(trades: List[Trade]) -> pd.DataFrame:
    """Convert trades to pandas DataFrame"""
    if not trades:
        return pd.DataFrame()
    
    data = []
    for trade in trades:
        data.append({
            'timestamp': trade.timestamp,
            'price': trade.price,
            'volume': trade.volume,
            'side': trade.side,
            'trade_id': trade.trade_id
        })
    
    df = pd.DataFrame(data)
    df['timestamp'] = pd.to_datetime(df['timestamp'], unit='s')
    df.set_index('timestamp', inplace=True)
    return df
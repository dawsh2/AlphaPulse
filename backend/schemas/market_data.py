"""
Market Data Models - Pydantic schemas for type safety
"""
from pydantic import BaseModel, Field, validator
from typing import List, Optional, Dict, Any
from datetime import datetime
from enum import Enum


class Exchange(str, Enum):
    """Supported exchanges"""
    COINBASE = "coinbase"
    ALPACA = "alpaca"
    BINANCE = "binance"


class Timeframe(str, Enum):
    """Supported timeframes"""
    MINUTE_1 = "1m"
    MINUTE_5 = "5m"
    MINUTE_15 = "15m"
    MINUTE_30 = "30m"
    HOUR_1 = "1h"
    HOUR_4 = "4h"
    DAY_1 = "1d"


class CandleData(BaseModel):
    """Single OHLCV candle data"""
    timestamp: int = Field(..., description="Unix timestamp in seconds")
    open: float = Field(..., gt=0, description="Opening price")
    high: float = Field(..., gt=0, description="Highest price")
    low: float = Field(..., gt=0, description="Lowest price")
    close: float = Field(..., gt=0, description="Closing price")
    volume: float = Field(..., ge=0, description="Volume traded")
    
    @validator('high')
    def high_must_be_highest(cls, v, values):
        """Validate that high is >= open, close, low"""
        if 'low' in values and v < values['low']:
            raise ValueError('High must be >= low')
        if 'open' in values and v < values['open']:
            raise ValueError('High must be >= open')
        if 'close' in values and v < values['close']:
            raise ValueError('High must be >= close')
        return v
    
    @validator('low')
    def low_must_be_lowest(cls, v, values):
        """Validate that low is <= open, close"""
        if 'open' in values and v > values['open']:
            raise ValueError('Low must be <= open')
        if 'close' in values and v > values['close']:
            raise ValueError('Low must be <= close')
        return v


class MarketDataRequest(BaseModel):
    """Request for market data"""
    symbol: str = Field(..., description="Trading pair (e.g., BTC/USD)")
    exchange: Exchange = Field(default=Exchange.COINBASE, description="Exchange name")
    interval: Timeframe = Field(default=Timeframe.MINUTE_1, description="Data interval")
    limit: Optional[int] = Field(default=100, ge=1, le=10000, description="Number of candles to fetch")
    start_time: Optional[int] = Field(None, description="Start timestamp (Unix seconds)")
    end_time: Optional[int] = Field(None, description="End timestamp (Unix seconds)")
    
    @validator('symbol')
    def validate_symbol_format(cls, v):
        """Validate symbol format"""
        if '/' not in v:
            raise ValueError('Symbol must be in format BASE/QUOTE (e.g., BTC/USD)')
        return v.upper()


class MarketDataset(BaseModel):
    """Complete market dataset"""
    symbol: str = Field(..., description="Trading pair")
    exchange: Exchange = Field(..., description="Exchange name")
    interval: Timeframe = Field(..., description="Data interval")
    candles: List[CandleData] = Field(..., description="OHLCV candle data")
    metadata: Optional[Dict[str, Any]] = Field(default_factory=dict, description="Additional metadata")
    
    @validator('candles')
    def candles_must_be_sorted(cls, v):
        """Ensure candles are sorted by timestamp"""
        if len(v) > 1:
            timestamps = [candle.timestamp for candle in v]
            if timestamps != sorted(timestamps):
                raise ValueError('Candles must be sorted by timestamp')
        return v


class DataSummary(BaseModel):
    """Summary of stored data"""
    total_bars: int = Field(..., ge=0, description="Total number of bars stored")
    symbols: List[Dict[str, Any]] = Field(..., description="Symbol information")
    
    class SymbolInfo(BaseModel):
        symbol: str
        exchange: str
        bar_count: int
        first_bar: datetime
        last_bar: datetime


class QueryRequest(BaseModel):
    """SQL query request"""
    query: str = Field(..., min_length=1, description="SQL query to execute")
    
    @validator('query')
    def validate_select_only(cls, v):
        """Only allow SELECT queries for security"""
        if not v.strip().upper().startswith('SELECT'):
            raise ValueError('Only SELECT queries are allowed')
        return v


class QueryResult(BaseModel):
    """SQL query result"""
    data: List[Dict[str, Any]] = Field(..., description="Query result data")
    rows: int = Field(..., ge=0, description="Number of rows returned")
    columns: List[str] = Field(..., description="Column names")


class CorrelationRequest(BaseModel):
    """Correlation analysis request"""
    symbols: List[str] = Field(..., min_items=2, description="Symbols to analyze")
    exchange: Exchange = Field(default=Exchange.COINBASE, description="Exchange name")
    
    @validator('symbols')
    def validate_symbols(cls, v):
        """Validate symbol formats"""
        validated = []
        for symbol in v:
            if '/' not in symbol:
                raise ValueError(f'Symbol {symbol} must be in format BASE/QUOTE')
            validated.append(symbol.upper())
        return validated


class StatisticsResult(BaseModel):
    """Statistical analysis result"""
    symbol: str
    exchange: str
    mean_return: Optional[float]
    volatility: Optional[float]
    skewness: Optional[float]
    kurtosis: Optional[float]
    min_return: Optional[float]
    max_return: Optional[float]
    total_bars: int
    annualized_volatility: Optional[float]
    sharpe_ratio: Optional[float]


class RiskMetrics(BaseModel):
    """Risk analysis metrics"""
    symbol: str
    data_points: int
    mean_return_annualized: float
    volatility_annualized: float
    sharpe_ratio: float
    sortino_ratio: float
    var_95: float = Field(..., description="Value at Risk (95%)")
    var_99: float = Field(..., description="Value at Risk (99%)")
    expected_shortfall_95: float = Field(..., description="Expected Shortfall (95%)")
    expected_shortfall_99: float = Field(..., description="Expected Shortfall (99%)")
    max_drawdown: float
    skewness: float
    kurtosis: float
    risk_free_rate: float


class BacktestConfig(BaseModel):
    """Backtesting configuration"""
    symbol: str = Field(..., description="Trading pair to backtest")
    exchange: Exchange = Field(default=Exchange.COINBASE)
    strategy_name: str = Field(..., description="Strategy identifier")
    parameters: Dict[str, Any] = Field(default_factory=dict, description="Strategy parameters")
    start_date: Optional[datetime] = Field(None, description="Backtest start date")
    end_date: Optional[datetime] = Field(None, description="Backtest end date")
    initial_capital: float = Field(default=10000.0, gt=0, description="Initial capital")
    commission: float = Field(default=0.001, ge=0, le=0.1, description="Commission rate")


class BacktestResult(BaseModel):
    """Backtesting result"""
    symbol: str
    strategy: Dict[str, Any]
    total_return: float
    annualized_return: float
    volatility: float
    sharpe_ratio: float
    max_drawdown: float
    total_trades: int
    data_points: int
    backtest_period: Dict[str, Optional[str]]


class MarketRegime(str, Enum):
    """Market regime classifications"""
    BULL_VOLATILE = "bull_volatile"
    BEAR_VOLATILE = "bear_volatile"  
    BULL_STABLE = "bull_stable"
    BEAR_STABLE = "bear_stable"
    UNKNOWN = "unknown"


class RegimeAnalysis(BaseModel):
    """Market regime analysis"""
    symbols: List[str]
    regime_analysis: Dict[str, Dict[str, Any]]
    regimes: List[MarketRegime]


class SaveDataRequest(BaseModel):
    """Request to save market data"""
    symbol: str = Field(..., description="Trading pair")
    exchange: str = Field(default="coinbase", description="Exchange name")
    candles: List[List[float]] = Field(..., description="Coinbase format candle data")
    interval: str = Field(default="1m", description="Data interval")
    
    @validator('candles')
    def validate_candle_format(cls, v):
        """Validate Coinbase candle format"""
        for candle in v:
            if len(candle) != 6:
                raise ValueError('Each candle must have 6 values: [timestamp, low, high, open, close, volume]')
            if not all(isinstance(x, (int, float)) for x in candle):
                raise ValueError('All candle values must be numeric')
        return v


class ApiResponse(BaseModel):
    """Generic API response wrapper"""
    status: str = Field(..., description="Response status")
    message: Optional[str] = Field(None, description="Response message")
    data: Optional[Any] = Field(None, description="Response data")
    error: Optional[str] = Field(None, description="Error message if status is error")


class CorrelationResponse(BaseModel):
    """Response for correlation analysis"""
    correlation: Optional[float] = Field(None, description="Correlation coefficient")
    symbol1_stats: Dict[str, Any] = Field(..., description="Statistics for first symbol")
    symbol2_stats: Dict[str, Any] = Field(..., description="Statistics for second symbol")


class AnalysisRequest(BaseModel):
    """Request for analysis operations"""
    symbols: List[str] = Field(..., min_items=1, description="Symbols to analyze")
    exchange: Exchange = Field(default=Exchange.COINBASE, description="Exchange name")
    
    @validator('symbols')
    def validate_symbols(cls, v):
        """Validate symbol formats"""
        validated = []
        for symbol in v:
            if '/' not in symbol and '-' not in symbol:
                raise ValueError(f'Symbol {symbol} must be in format BASE/QUOTE or BASE-QUOTE')
            # Normalize to our internal format (BASE/QUOTE)
            normalized = symbol.replace('-', '/').upper()
            validated.append(normalized)
        return validated


class RollingStatsRequest(BaseModel):
    """Request for rolling statistics"""
    symbol: str = Field(..., description="Symbol to analyze")
    window: int = Field(default=20, ge=5, le=200, description="Rolling window size")
    exchange: Exchange = Field(default=Exchange.COINBASE, description="Exchange name")
    
    @validator('symbol')
    def validate_symbol_format(cls, v):
        """Validate and normalize symbol format"""
        if '/' not in v and '-' not in v:
            raise ValueError('Symbol must be in format BASE/QUOTE or BASE-QUOTE')
        return v.replace('-', '/').upper()


class RiskMetricsRequest(BaseModel):
    """Request for risk metrics calculation"""
    symbol: str = Field(..., description="Symbol to analyze")
    exchange: Exchange = Field(default=Exchange.COINBASE, description="Exchange name")
    risk_free_rate: float = Field(default=0.02, ge=0, le=0.2, description="Risk-free rate for Sharpe ratio")
    
    @validator('symbol')
    def validate_symbol_format(cls, v):
        """Validate and normalize symbol format"""
        if '/' not in v and '-' not in v:
            raise ValueError('Symbol must be in format BASE/QUOTE or BASE-QUOTE')
        return v.replace('-', '/').upper()
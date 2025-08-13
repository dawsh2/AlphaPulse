"""
Data API Routes for FastAPI - Market data and analysis endpoints
"""
from fastapi import APIRouter, HTTPException, Depends, Query, Path as PathParam
from pydantic import BaseModel, Field
from typing import Optional, List, Dict, Any
from enum import Enum
import logging

from services.data_service import MarketDataService
from services.analysis_service import MarketAnalysisService
from core.container import get_market_data_service, get_market_analysis_service

# Setup logging
logger = logging.getLogger(__name__)

# Create router
router = APIRouter(
    prefix="/api",
    tags=["data", "analysis"],
    responses={404: {"description": "Not found"}},
)

# Enums
class ExchangeEnum(str, Enum):
    coinbase = "coinbase"
    kraken = "kraken"
    binance = "binance"

# Pydantic models
class SaveDataRequest(BaseModel):
    """Request to save market data"""
    symbol: str
    exchange: str
    interval: str = "1m"
    candles: List[Dict[str, Any]]

class QueryRequest(BaseModel):
    """SQL query request"""
    query: str = Field(..., min_length=1, max_length=5000)

class AnalysisRequest(BaseModel):
    """Analysis request for multiple symbols"""
    symbols: List[str] = Field(..., min_items=1, max_items=20)
    exchange: ExchangeEnum = ExchangeEnum.coinbase

class BacktestConfig(BaseModel):
    """Backtest configuration"""
    symbol: str
    exchange: str = "coinbase"
    type: str = "simple_ma_cross"
    fast_period: int = 10
    slow_period: int = 20
    start_date: Optional[str] = None
    end_date: Optional[str] = None

class MarketRegimeRequest(BaseModel):
    """Market regime analysis request"""
    symbols: List[str]
    exchange: str = "coinbase"

# ================================================================================
# Data Management Endpoints
# ================================================================================

@router.post("/market-data/save")
async def save_market_data(
    request: SaveDataRequest,
    service: MarketDataService = Depends(get_market_data_service)
):
    """
    Save market data to storage
    
    Args:
        request: Market data to save
        
    Returns:
        Save result with statistics
    """
    try:
        result = await service.save_market_data(
            request.symbol,
            request.exchange,
            request.candles,
            request.interval
        )
        
        if result['status'] == 'error':
            raise HTTPException(status_code=400, detail=result['message'])
        
        return result
    except Exception as e:
        logger.error(f"Error saving market data: {e}")
        raise HTTPException(status_code=500, detail=str(e))

@router.get("/crypto-data/{symbol}")
async def get_crypto_data(
    symbol: str = PathParam(..., description="Trading symbol (e.g., BTC-USD)"),
    exchange: str = Query("coinbase", description="Exchange name"),
    start_time: Optional[int] = Query(None, description="Start timestamp"),
    end_time: Optional[int] = Query(None, description="End timestamp"),
    limit: int = Query(10000, description="Max records to return"),
    service: MarketDataService = Depends(get_market_data_service)
):
    """
    Get OHLCV data for a symbol
    
    Args:
        symbol: Trading symbol
        exchange: Exchange name
        start_time: Start timestamp
        end_time: End timestamp
        limit: Max records
        
    Returns:
        OHLCV data
    """
    try:
        result = await service.get_ohlcv_data(symbol, exchange, start_time, end_time, limit)
        return result
    except Exception as e:
        logger.error(f"Error getting crypto data: {e}")
        raise HTTPException(status_code=500, detail=str(e))

@router.get("/data/summary")
async def get_data_summary(
    service: MarketDataService = Depends(get_market_data_service)
):
    """
    Get summary of all stored data
    
    Returns:
        Data summary statistics
    """
    try:
        summary = await service.get_data_summary()
        return summary
    except Exception as e:
        logger.error(f"Error getting data summary: {e}")
        raise HTTPException(status_code=500, detail=str(e))

@router.post("/data/query")
async def query_data(
    request: QueryRequest,
    service: MarketDataService = Depends(get_market_data_service)
):
    """
    Execute SQL query on DuckDB
    
    Args:
        request: SQL query
        
    Returns:
        Query results
    """
    try:
        result = await service.query_data(request.query)
        
        if result['status'] == 'error':
            raise HTTPException(status_code=400, detail=result['message'])
        
        return result
    except Exception as e:
        logger.error(f"Error executing query: {e}")
        raise HTTPException(status_code=500, detail=str(e))

@router.get("/data/correlation/{symbol1}/{symbol2}")
async def get_correlation(
    symbol1: str = PathParam(..., description="First symbol"),
    symbol2: str = PathParam(..., description="Second symbol"),
    exchange: str = Query("coinbase", description="Exchange name"),
    service: MarketDataService = Depends(get_market_data_service)
):
    """
    Get correlation between two symbols
    
    Args:
        symbol1: First symbol
        symbol2: Second symbol
        exchange: Exchange name
        
    Returns:
        Correlation data
    """
    try:
        result = await service.get_correlation(symbol1, symbol2, exchange)
        
        if result.get('status') == 'error':
            raise HTTPException(status_code=400, detail=result['message'])
        
        return result
    except Exception as e:
        logger.error(f"Error calculating correlation: {e}")
        raise HTTPException(status_code=500, detail=str(e))

@router.get("/catalog/list")
async def list_catalog_data(
    service: MarketDataService = Depends(get_market_data_service)
):
    """
    List all available data in catalog
    
    Returns:
        Catalog of available data
    """
    try:
        result = await service.list_catalog_data()
        return result
    except Exception as e:
        logger.error(f"Error listing catalog: {e}")
        raise HTTPException(status_code=500, detail=str(e))

# ================================================================================
# Proxy Endpoints for Exchange APIs
# ================================================================================

@router.get("/proxy/coinbase/{endpoint:path}")
async def proxy_coinbase(
    endpoint: str,
    service: MarketDataService = Depends(get_market_data_service)
):
    """
    Proxy requests to Coinbase API
    
    Args:
        endpoint: API endpoint path
        
    Returns:
        Coinbase API response
    """
    try:
        # Get query parameters from request
        import inspect
        frame = inspect.currentframe()
        # This is a simplified version - in production you'd get query params properly
        params = {}
        
        result = await service.proxy_coinbase_data(endpoint, params)
        
        if result['status'] == 'error':
            raise HTTPException(status_code=result['status_code'], detail=result['message'])
        
        return result['data']
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Error proxying Coinbase request: {e}")
        raise HTTPException(status_code=500, detail=str(e))

@router.get("/proxy/kraken/{endpoint:path}")
async def proxy_kraken(
    endpoint: str,
    service: MarketDataService = Depends(get_market_data_service)
):
    """
    Proxy requests to Kraken API
    
    Args:
        endpoint: API endpoint path
        
    Returns:
        Kraken API response
    """
    try:
        # Get query parameters from request
        params = {}
        
        result = await service.proxy_kraken_data(endpoint, params)
        
        if result['status'] == 'error':
            raise HTTPException(status_code=result['status_code'], detail=result['message'])
        
        return result['data']
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Error proxying Kraken request: {e}")
        raise HTTPException(status_code=500, detail=str(e))

# ================================================================================
# Analysis Endpoints
# ================================================================================

@router.get("/analysis/statistics/{symbol}")
async def get_symbol_statistics(
    symbol: str = PathParam(..., description="Trading symbol"),
    exchange: str = Query("coinbase", description="Exchange name"),
    service: MarketAnalysisService = Depends(get_market_analysis_service)
):
    """
    Get basic statistics for a symbol
    
    Args:
        symbol: Trading symbol
        exchange: Exchange name
        
    Returns:
        Statistical summary
    """
    try:
        # Convert URL format to internal format
        symbol = symbol.replace('-', '/')
        
        stats = await service.calculate_basic_statistics(symbol, exchange)
        return {
            'symbol': symbol,
            'exchange': exchange,
            'statistics': stats
        }
    except Exception as e:
        logger.error(f"Error calculating statistics: {e}")
        raise HTTPException(status_code=500, detail=str(e))

@router.post("/analysis/correlation-matrix")
async def get_correlation_matrix(
    request: AnalysisRequest,
    service: MarketAnalysisService = Depends(get_market_analysis_service)
):
    """
    Get correlation matrix for multiple symbols
    
    Args:
        request: List of symbols and exchange
        
    Returns:
        Correlation matrix
    """
    try:
        # Convert URL format to internal format
        symbols = [s.replace('-', '/') for s in request.symbols]
        
        result = await service.calculate_correlation_matrix(symbols, request.exchange.value)
        return result
    except Exception as e:
        logger.error(f"Error calculating correlation matrix: {e}")
        raise HTTPException(status_code=500, detail=str(e))

@router.get("/analysis/rolling-stats/{symbol}")
async def get_rolling_statistics(
    symbol: str = PathParam(..., description="Trading symbol"),
    exchange: str = Query("coinbase", description="Exchange name"),
    window: int = Query(20, description="Rolling window size"),
    service: MarketAnalysisService = Depends(get_market_analysis_service)
):
    """
    Get rolling statistics for a symbol
    
    Args:
        symbol: Trading symbol
        exchange: Exchange name
        window: Rolling window size
        
    Returns:
        Rolling statistics
    """
    try:
        # Convert URL format to internal format
        symbol = symbol.replace('-', '/')
        
        stats = await service.calculate_rolling_statistics(symbol, window, exchange)
        return stats
    except Exception as e:
        logger.error(f"Error calculating rolling statistics: {e}")
        raise HTTPException(status_code=500, detail=str(e))

@router.get("/analysis/risk-metrics/{symbol}")
async def get_risk_metrics(
    symbol: str = PathParam(..., description="Trading symbol"),
    exchange: str = Query("coinbase", description="Exchange name"),
    risk_free_rate: float = Query(0.02, description="Annual risk-free rate"),
    service: MarketAnalysisService = Depends(get_market_analysis_service)
):
    """
    Get comprehensive risk metrics for a symbol
    
    Args:
        symbol: Trading symbol
        exchange: Exchange name
        risk_free_rate: Annual risk-free rate
        
    Returns:
        Risk metrics
    """
    try:
        # Convert URL format to internal format
        symbol = symbol.replace('-', '/')
        
        metrics = await service.calculate_risk_metrics(symbol, exchange, risk_free_rate)
        return metrics
    except Exception as e:
        logger.error(f"Error calculating risk metrics: {e}")
        raise HTTPException(status_code=500, detail=str(e))

@router.post("/analysis/backtest")
async def run_backtest(
    config: BacktestConfig,
    service: MarketAnalysisService = Depends(get_market_analysis_service)
):
    """
    Run backtesting analysis
    
    Args:
        config: Backtest configuration
        
    Returns:
        Backtest results
    """
    try:
        result = await service.perform_backtesting_analysis(config.dict())
        
        if result.get('status') == 'error':
            raise HTTPException(status_code=400, detail=result['message'])
        
        return result
    except Exception as e:
        logger.error(f"Error running backtest: {e}")
        raise HTTPException(status_code=500, detail=str(e))

@router.post("/analysis/market-regime")
async def get_market_regime(
    request: MarketRegimeRequest,
    service: MarketAnalysisService = Depends(get_market_analysis_service)
):
    """
    Get market regime analysis
    
    Args:
        request: List of symbols and exchange
        
    Returns:
        Market regime analysis
    """
    try:
        # Convert URL format to internal format
        symbols = [s.replace('-', '/') for s in request.symbols]
        
        result = await service.get_market_regime_analysis(symbols, request.exchange)
        return result
    except Exception as e:
        logger.error(f"Error analyzing market regime: {e}")
        raise HTTPException(status_code=500, detail=str(e))

# Health check for data service
@router.get("/data/health")
async def data_health(
    data_service: MarketDataService = Depends(get_market_data_service),
    analysis_service: MarketAnalysisService = Depends(get_market_analysis_service)
):
    """Health check for data and analysis services"""
    return {
        "service": "data_and_analysis",
        "status": "healthy",
        "components": {
            "data_service": "active",
            "analysis_service": "active"
        }
    }
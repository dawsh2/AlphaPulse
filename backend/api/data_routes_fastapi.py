"""
Data API Routes for FastAPI - Market data and analysis endpoints
"""
from fastapi import APIRouter, HTTPException, Depends, Query, Path as PathParam
from pydantic import BaseModel, Field
from typing import Optional, List, Dict, Any
from enum import Enum
import logging
import os
from dotenv import load_dotenv
import sys

# Load environment variables
load_dotenv(verbose=True)
print(f"Python path: {sys.path}", file=sys.stderr)

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

class PoolAnalysisRequest(BaseModel):
    """Pool analysis request"""
    poolAddress: str = Field(..., min_length=40, max_length=42)
    tradeSize: Optional[float] = Field(default=1000.0, description="Trade size in USD")

class PoolAnalysisResponse(BaseModel):
    """Pool analysis response"""
    address: str
    symbol0: str
    symbol1: str
    reserve0: float
    reserve1: float
    price: float
    liquidity: float
    poolType: Optional[str] = "UNISWAP_V2"
    fee: Optional[float] = 0.003  # Fee percentage (0.003 = 0.3%)

class QuoteRequest(BaseModel):
    """Quote request for DEX swap"""
    tokenIn: str = Field(..., min_length=40, max_length=42)
    tokenOut: str = Field(..., min_length=40, max_length=42)
    amountIn: float = Field(..., gt=0)

class QuoteResponse(BaseModel):
    """Quote response with real execution price"""
    protocol: str
    amountIn: float
    amountOut: float
    price: float
    priceImpact: float
    fee: float
    success: bool
    error: Optional[str] = None

class BacktestRequest(BaseModel):
    """Backtest request configuration"""
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


class PairComparisonReport(BaseModel):
    """Pair comparison report between WebSocket and Dashboard"""
    websocket_pairs: List[str]
    dashboard_pairs: List[str]
    timestamp: str

@router.post("/pair-comparison-report")
async def save_pair_comparison_report(report: PairComparisonReport):
    """
    Save pair comparison report to disk for analysis
    """
    try:
        import json
        import os
        from datetime import datetime
        
        # Create reports directory if it doesn't exist
        reports_dir = os.path.join(os.path.dirname(__file__), '..', 'reports')
        os.makedirs(reports_dir, exist_ok=True)
        
        # Generate filename with timestamp
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        filename = f"pair_comparison_{timestamp}.json"
        filepath = os.path.join(reports_dir, filename)
        
        # Calculate comparison stats
        ws_set = set(report.websocket_pairs)
        dash_set = set(report.dashboard_pairs)
        
        comparison_data = {
            "timestamp": report.timestamp,
            "websocket_total": len(ws_set),
            "dashboard_total": len(dash_set),
            "common_pairs": len(ws_set.intersection(dash_set)),
            "websocket_only": list(ws_set - dash_set),
            "dashboard_only": list(dash_set - ws_set),
            "common_pairs_list": list(ws_set.intersection(dash_set)),
            "filtering_ratio": round((len(ws_set) - len(dash_set)) / len(ws_set) * 100, 2) if len(ws_set) > 0 else 0
        }
        
        # Write to file
        with open(filepath, 'w') as f:
            json.dump(comparison_data, f, indent=2)
        
        # Also append summary to main log
        summary_file = os.path.join(reports_dir, "pair_comparison_summary.log")
        with open(summary_file, 'a') as f:
            f.write(f"{report.timestamp}: WS={len(ws_set)}, Dash={len(dash_set)}, Filtered={comparison_data['filtering_ratio']}%\n")
        
        logger.info(f"Pair comparison report saved: {filename}")
        
        return {
            "status": "success",
            "filename": filename,
            "summary": comparison_data
        }
        
    except Exception as e:
        logger.error(f"Error saving pair comparison report: {e}")
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

# ================================================================================
# Pool Analysis Endpoint
# ================================================================================

@router.post("/pool-analysis", response_model=PoolAnalysisResponse)
async def analyze_pool(request: PoolAnalysisRequest):
    """
    Analyze DeFi pool liquidity and reserves with proper pool type detection
    
    Args:
        request: Pool analysis request with pool address
        
    Returns:
        Pool liquidity data including reserves, price, and symbols
    """
    import asyncio
    max_retries = 3
    retry_delay = 0.5
    
    for attempt in range(max_retries):
        try:
            import os
            from web3 import Web3
            
            logger.info(f"Pool analysis request for: {request.poolAddress} (attempt {attempt + 1}/{max_retries})")
            
            # Use Ankr RPC endpoint 
            ankr_key = os.getenv('ANKR_API_KEY', 'e6fac469b91ea8fd98406aca0820653ae6fe5c2400f44819450f6022dd2792e2')
            logger.info(f"Debug - ANKR_API_KEY loaded: {ankr_key[:10]}... (length: {len(ankr_key)})" if ankr_key else "Debug - No ANKR_API_KEY found")
            if ankr_key and len(ankr_key) > 10:
                rpc_url = f"https://rpc.ankr.com/polygon/{ankr_key}"
                logger.info(f"Using Ankr RPC with key (length: {len(ankr_key)})")
            else:
                rpc_url = "https://polygon.publicnode.com"
                logger.warning("No ANKR_API_KEY found, using free publicnode endpoint")
                
            # Add timeout to provider
            w3 = Web3(Web3.HTTPProvider(rpc_url, request_kwargs={'timeout': 10}))
            
            if not w3.is_connected():
                if attempt < max_retries - 1:
                    await asyncio.sleep(retry_delay)
                    continue
                raise HTTPException(status_code=503, detail="Unable to connect to Polygon network")
        
            # Detect pool type by checking interface
            pool_type = await detect_pool_type(w3, request.poolAddress)
            logger.info(f"Detected pool type: {pool_type} for {request.poolAddress}")
            
            # Uniswap V2 Pair ABI (minimal)
            pair_abi = [
                {
                "constant": True,
                "inputs": [],
                "name": "getReserves",
                "outputs": [
                    {"name": "_reserve0", "type": "uint112"},
                    {"name": "_reserve1", "type": "uint112"},
                    {"name": "_blockTimestampLast", "type": "uint32"}
                ],
                "type": "function"
            },
            {
                "constant": True,
                "inputs": [],
                "name": "token0",
                "outputs": [{"name": "", "type": "address"}],
                "type": "function"
            },
            {
                "constant": True,
                "inputs": [],
                "name": "token1",
                "outputs": [{"name": "", "type": "address"}],
                "type": "function"
            }
            ]
            
            # ERC20 ABI (minimal)
            erc20_abi = [
            {
                "constant": True,
                "inputs": [],
                "name": "symbol",
                "outputs": [{"name": "", "type": "string"}],
                "type": "function"
            },
            {
                "constant": True,
                "inputs": [],
                "name": "decimals",
                "outputs": [{"name": "", "type": "uint8"}],
                "type": "function"
            }
            ]
            
            # Validate and create contract instance
            try:
                pool_address = w3.to_checksum_address(request.poolAddress)
            except Exception as e:
                logger.error(f"Invalid pool address format: {request.poolAddress}")
                # Return default liquidity for invalid addresses
                return PoolAnalysisResponse(
                    address=request.poolAddress,
                    symbol0="UNKNOWN",
                    symbol1="UNKNOWN",
                    reserve0=50000.0,  # Default $50K reserves
                    reserve1=50000.0,
                    price=1.0,
                    liquidity=100000.0,  # Default $100K liquidity
                    poolType="UNKNOWN"
                )
            
            # Use appropriate ABI based on pool type
            if pool_type == 'UNISWAP_V3':
                # For V3, we need different approach
                return await analyze_v3_pool(w3, pool_address)
            elif pool_type == 'CURVE':
                # Curve has different mechanics
                return await analyze_curve_pool(w3, pool_address)
            
            # Default to V2 analysis
            contract = w3.eth.contract(address=pool_address, abi=pair_abi)
            
            # Get pool data with error handling
            try:
                reserves = contract.functions.getReserves().call()
                token0_addr = contract.functions.token0().call()
                token1_addr = contract.functions.token1().call()
            except Exception as e:
                logger.warning(f"Pool {request.poolAddress} doesn't support V2 interface: {e}")
                # Return default liquidity for incompatible pools
                return PoolAnalysisResponse(
                    address=request.poolAddress,
                    symbol0="TOKEN0",
                    symbol1="TOKEN1",
                    reserve0=50000.0,  # Default $50K reserves
                    reserve1=50000.0,
                    price=1.0,
                    liquidity=100000.0,  # Default $100K liquidity
                    poolType="UNKNOWN"
                )
            
            # Get token symbols and decimals
            token0_contract = w3.eth.contract(address=token0_addr, abi=erc20_abi)
            token1_contract = w3.eth.contract(address=token1_addr, abi=erc20_abi)
            
            try:
                symbol0 = token0_contract.functions.symbol().call()
                decimals0 = token0_contract.functions.decimals().call()
            except:
                symbol0 = "TOKEN0"
                decimals0 = 18
                
            try:
                symbol1 = token1_contract.functions.symbol().call()
                decimals1 = token1_contract.functions.decimals().call()
            except:
                symbol1 = "TOKEN1"
                decimals1 = 18
            
            # Convert reserves to human readable format
            reserve0 = float(reserves[0]) / (10 ** decimals0)
            reserve1 = float(reserves[1]) / (10 ** decimals1)
            
            # Calculate price (token1 per token0)
            price = reserve1 / reserve0 if reserve0 > 0 else 0
            
            # Calculate total liquidity (geometric mean * 2)
            liquidity = (reserve0 * reserve1) ** 0.5 * 2
            
            return PoolAnalysisResponse(
            address=request.poolAddress,
            symbol0=symbol0,
            symbol1=symbol1,
            reserve0=reserve0,
            reserve1=reserve1,
            price=price,
            liquidity=liquidity,
            fee=0.003  # V2 pools always have 0.3% fee
            )
            
        except Exception as e:
            if attempt < max_retries - 1:
                logger.warning(f"Pool analysis attempt {attempt + 1} failed: {e}, retrying...")
                await asyncio.sleep(retry_delay * (attempt + 1))  # Exponential backoff
                continue
            logger.error(f"Pool analysis error after {max_retries} attempts: {e}")
            raise HTTPException(status_code=500, detail=f"Failed to analyze pool after {max_retries} attempts: {str(e)}")

async def detect_pool_type(w3, pool_address: str) -> str:
    """
    Detect pool type by checking contract interface
    """
    try:
        pool_address = w3.to_checksum_address(pool_address)
        
        # Check for Uniswap V3 by looking for slot0() function
        v3_abi = [{
            "constant": True,
            "inputs": [],
            "name": "slot0",
            "outputs": [
                {"name": "sqrtPriceX96", "type": "uint160"},
                {"name": "tick", "type": "int24"},
                {"name": "observationIndex", "type": "uint16"},
                {"name": "observationCardinality", "type": "uint16"},
                {"name": "observationCardinalityNext", "type": "uint16"},
                {"name": "feeProtocol", "type": "uint8"},
                {"name": "unlocked", "type": "bool"}
            ],
            "type": "function"
        }]
        
        try:
            v3_contract = w3.eth.contract(address=pool_address, abi=v3_abi)
            slot0 = v3_contract.functions.slot0().call()
            if slot0:
                return "UNISWAP_V3"
        except:
            pass
        
        # Check for Curve by looking for A() function (amplification coefficient)
        curve_abi = [{
            "constant": True,
            "inputs": [],
            "name": "A",
            "outputs": [{"name": "", "type": "uint256"}],
            "type": "function"
        }]
        
        try:
            curve_contract = w3.eth.contract(address=pool_address, abi=curve_abi)
            a_coeff = curve_contract.functions.A().call()
            if a_coeff:
                return "CURVE"
        except:
            pass
        
        # Default to Uniswap V2
        return "UNISWAP_V2"
        
    except Exception as e:
        logger.warning(f"Failed to detect pool type: {e}")
        return "UNISWAP_V2"

async def analyze_v3_pool(w3, pool_address) -> PoolAnalysisResponse:
    """
    Analyze Uniswap V3 pool with tick data
    """
    # V3 specific ABI
    v3_abi = [
        {
            "constant": True,
            "inputs": [],
            "name": "slot0",
            "outputs": [
                {"name": "sqrtPriceX96", "type": "uint160"},
                {"name": "tick", "type": "int24"},
                {"name": "observationIndex", "type": "uint16"},
                {"name": "observationCardinality", "type": "uint16"},
                {"name": "observationCardinalityNext", "type": "uint16"},
                {"name": "feeProtocol", "type": "uint8"},
                {"name": "unlocked", "type": "bool"}
            ],
            "type": "function"
        },
        {
            "constant": True,
            "inputs": [],
            "name": "liquidity",
            "outputs": [{"name": "", "type": "uint128"}],
            "type": "function"
        },
        {
            "constant": True,
            "inputs": [],
            "name": "token0",
            "outputs": [{"name": "", "type": "address"}],
            "type": "function"
        },
        {
            "constant": True,
            "inputs": [],
            "name": "token1",
            "outputs": [{"name": "", "type": "address"}],
            "type": "function"
        },
        {
            "constant": True,
            "inputs": [],
            "name": "fee",
            "outputs": [{"name": "", "type": "uint24"}],
            "type": "function"
        }
    ]
    
    contract = w3.eth.contract(address=pool_address, abi=v3_abi)
    
    # Get pool state
    slot0 = contract.functions.slot0().call()
    liquidity = contract.functions.liquidity().call()
    token0_addr = contract.functions.token0().call()
    token1_addr = contract.functions.token1().call()
    fee = contract.functions.fee().call()
    
    # Calculate price from sqrtPriceX96
    sqrt_price_x96 = slot0[0]
    tick = slot0[1]
    
    # Price calculation for V3
    # sqrtPrice = sqrt(price) * 2^96
    # price = (sqrtPrice / 2^96)^2
    price_raw = (sqrt_price_x96 / (2 ** 96)) ** 2
    
    # Get token info
    erc20_abi = [
        {"constant": True, "inputs": [], "name": "symbol", "outputs": [{"name": "", "type": "string"}], "type": "function"},
        {"constant": True, "inputs": [], "name": "decimals", "outputs": [{"name": "", "type": "uint8"}], "type": "function"}
    ]
    
    token0_contract = w3.eth.contract(address=token0_addr, abi=erc20_abi)
    token1_contract = w3.eth.contract(address=token1_addr, abi=erc20_abi)
    
    try:
        symbol0 = token0_contract.functions.symbol().call()
        decimals0 = token0_contract.functions.decimals().call()
    except:
        symbol0 = "TOKEN0"
        decimals0 = 18
        
    try:
        symbol1 = token1_contract.functions.symbol().call()
        decimals1 = token1_contract.functions.decimals().call()
    except:
        symbol1 = "TOKEN1"
        decimals1 = 18
    
    # Adjust price for decimals
    price_adjusted = price_raw * (10 ** (decimals0 - decimals1))
    
    # For V3, we need to estimate liquidity around the current price
    # The liquidity value represents L, the virtual liquidity
    # We can estimate token amounts using the current price
    
    # Convert liquidity to human readable
    L = float(liquidity)
    sqrt_price = float(sqrt_price_x96) / (2 ** 96)
    
    # Calculate virtual reserves at current price
    # For V3: x * y = L^2 and price = y/x
    # So: x = L / sqrt(price) and y = L * sqrt(price)
    
    if L > 0 and sqrt_price > 0:
        # Virtual reserves in raw units
        virtual_x = L / sqrt_price
        virtual_y = L * sqrt_price
        
        # Convert to human readable
        reserve0 = virtual_x / (10 ** decimals0)
        reserve1 = virtual_y / (10 ** decimals1)
        
        # Estimate USD liquidity (assuming token1 is USD-like or we use the price)
        # This is a rough estimate for V3 pools
        if "USD" in symbol0.upper() or "DAI" in symbol0.upper() or "USDT" in symbol0.upper() or "USDC" in symbol0.upper():
            liquidity_usd = reserve0 * 2  # Double the USD reserve as estimate
        elif "USD" in symbol1.upper() or "DAI" in symbol1.upper() or "USDT" in symbol1.upper() or "USDC" in symbol1.upper():
            liquidity_usd = reserve1 * 2  # Double the USD reserve as estimate
        else:
            # Use token0 value in token1 terms
            liquidity_usd = (reserve0 * price_adjusted + reserve1) * 1.5  # 1.5x for concentrated liquidity
    else:
        # Fallback if liquidity is 0
        reserve0 = 100  # Default reserves
        reserve1 = 100 * price_adjusted
        liquidity_usd = 100000  # Default $100k
    
    # Get fee tier (in basis points / 10000)
    fee_percentage = fee / 1000000  # Convert to decimal (e.g., 3000 -> 0.003)
    
    logger.info(f"V3 pool {pool_address}: L={L:.0f}, sqrtPrice={sqrt_price:.6f}, tick={tick}, fee={fee_percentage:.2%}")
    logger.info(f"V3 virtual reserves: {reserve0:.6f} {symbol0}, {reserve1:.6f} {symbol1}")
    
    return PoolAnalysisResponse(
        address=str(pool_address),
        symbol0=symbol0,
        symbol1=symbol1,
        reserve0=reserve0,  # Virtual reserves for V3
        reserve1=reserve1,
        price=price_adjusted,
        liquidity=liquidity_usd,
        poolType="UNISWAP_V3",
        fee=fee_percentage  # Actual fee from the pool
    )

async def analyze_curve_pool(w3, pool_address) -> PoolAnalysisResponse:
    """
    Analyze Curve pool
    """
    # Simplified Curve analysis - would need specific ABI for each pool type
    return PoolAnalysisResponse(
        address=str(pool_address),
        symbol0="CURVE_TOKEN0",
        symbol1="CURVE_TOKEN1",
        reserve0=0,
        reserve1=0,
        price=1.0,
        liquidity=0
    )

# ================================================================================
# DEX Quote Endpoint
# ================================================================================

@router.post("/dex-quote", response_model=QuoteResponse)
async def get_dex_quote(request: QuoteRequest):
    """
    Get real execution quote from DEX quoter contracts
    
    Args:
        request: Quote request with token addresses and amount
        
    Returns:
        Real quote including price impact and fees
    """
    try:
        import os
        from web3 import Web3
        from .dex_quoter import DexQuoter
        
        # Connect to Polygon
        ankr_key = os.getenv('ANKR_API_KEY', '')
        if ankr_key and len(ankr_key) > 10:
            rpc_url = f"https://rpc.ankr.com/polygon/{ankr_key}"
        else:
            rpc_url = "https://polygon.publicnode.com"
            
        w3 = Web3(Web3.HTTPProvider(rpc_url))
        
        if not w3.is_connected():
            raise HTTPException(status_code=503, detail="Unable to connect to Polygon network")
        
        # Initialize quoter
        quoter = DexQuoter(w3)
        
        # Get token decimals first
        erc20_abi = [
            {"constant": True, "inputs": [], "name": "decimals", "outputs": [{"name": "", "type": "uint8"}], "type": "function"}
        ]
        
        token_in_contract = w3.eth.contract(address=w3.to_checksum_address(request.tokenIn), abi=erc20_abi)
        token_out_contract = w3.eth.contract(address=w3.to_checksum_address(request.tokenOut), abi=erc20_abi)
        
        try:
            decimals_in = token_in_contract.functions.decimals().call()
            decimals_out = token_out_contract.functions.decimals().call()
        except:
            decimals_in = 18
            decimals_out = 18
        
        # Convert amount to smallest unit
        amount_in_wei = int(request.amountIn * (10 ** decimals_in))
        
        # Get best quote
        quote = await quoter.get_best_quote(
            request.tokenIn,
            request.tokenOut,
            amount_in_wei
        )
        
        if quote["success"]:
            # Convert back to human readable
            amount_out_human = quote["amountOut"] / (10 ** decimals_out)
            
            # Calculate price impact
            spot_price = amount_out_human / request.amountIn
            effective_price = quote["price"] * (10 ** (decimals_in - decimals_out))
            price_impact = abs(effective_price - spot_price) / spot_price * 100 if spot_price > 0 else 0
            
            return QuoteResponse(
                protocol=quote["protocol"],
                amountIn=request.amountIn,
                amountOut=amount_out_human,
                price=effective_price,
                priceImpact=price_impact,
                fee=quote["fee"],
                success=True
            )
        else:
            return QuoteResponse(
                protocol="NONE",
                amountIn=request.amountIn,
                amountOut=0,
                price=0,
                priceImpact=0,
                fee=0,
                success=False,
                error=quote.get("error", "Quote failed")
            )
            
    except Exception as e:
        logger.error(f"DEX quote error: {e}")
        raise HTTPException(status_code=500, detail=f"Failed to get quote: {str(e)}")
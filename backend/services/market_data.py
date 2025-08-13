"""
Refactored Data Service Layer - Uses Repository Pattern
This service uses injected repositories instead of direct DataManager access
"""
from typing import Dict, Any, Optional, List
import logging
import requests

from repositories.protocols import MarketDataRepository
from core.schemas import Trade, OHLCVBar, ohlcv_to_dataframe

logger = logging.getLogger(__name__)


class MarketDataService:
    """Service layer for market data operations - uses repository pattern"""
    
    def __init__(self, repository: MarketDataRepository):
        """Initialize with injected repository"""
        self.repository = repository
        logger.info("MarketDataService initialized with repository")
    
    async def save_market_data(
        self, 
        symbol: str, 
        exchange: str, 
        candles: List[Dict], 
        interval: str = "1m"
    ) -> Dict[str, Any]:
        """Save market data to storage
        
        Args:
            symbol: Trading symbol
            exchange: Exchange name
            candles: List of OHLCV candles (raw dicts)
            interval: Time interval
            
        Returns:
            Save result with statistics
        """
        try:
            logger.info(f"Saving {len(candles)} candles for {symbol} from {exchange}")
            
            if not candles:
                return {
                    'status': 'error',
                    'message': 'No candles data provided'
                }
            
            # Convert raw candles to Trade objects
            trades = []
            for candle in candles:
                # Handle both list and dict format
                if isinstance(candle, list):
                    trades.append(Trade(
                        timestamp=float(candle[0]),
                        price=float(candle[4]),  # close price
                        volume=float(candle[5])
                    ))
                else:
                    trades.append(Trade(
                        timestamp=float(candle.get('timestamp', 0)),
                        price=float(candle.get('close', candle.get('price', 0))),
                        volume=float(candle.get('volume', 0))
                    ))
            
            # Save via repository
            result = await self.repository.save_trades(trades, symbol, exchange)
            
            return {
                'status': 'success' if result.get('success') else 'error',
                'message': f'Saved {len(trades)} trades for {symbol}',
                'symbol': symbol,
                'exchange': exchange,
                'interval': interval,
                'trade_count': len(trades),
                **result
            }
            
        except Exception as e:
            logger.error(f"Error saving market data: {e}")
            return {
                'status': 'error',
                'message': str(e)
            }
    
    async def get_ohlcv_data(
        self, 
        symbol: str, 
        exchange: str = "coinbase",
        start_time: Optional[int] = None,
        end_time: Optional[int] = None,
        limit: int = 10000
    ) -> Dict[str, Any]:
        """Get OHLCV data for a symbol
        
        Args:
            symbol: Trading symbol
            exchange: Exchange name
            start_time: Start timestamp
            end_time: End timestamp
            limit: Max records to return
            
        Returns:
            OHLCV data and metadata
        """
        try:
            # Convert URL format to internal format (e.g., BTC-USD -> BTC/USD)
            symbol = symbol.replace('-', '/')
            
            # Convert timestamps to datetime if provided
            from datetime import datetime
            start_dt = datetime.fromtimestamp(start_time) if start_time else None
            end_dt = datetime.fromtimestamp(end_time) if end_time else None
            
            # Get data from repository
            bars = await self.repository.get_ohlcv(
                symbol, exchange, "1m", start_dt, end_dt
            )
            
            if not bars:
                return {
                    'data': [],
                    'symbol': symbol,
                    'exchange': exchange,
                    'count': 0
                }
            
            # Limit results if specified
            if limit and len(bars) > limit:
                bars = bars[-limit:]  # Get last N bars
            
            # Convert to dict format for API response
            data = []
            for bar in bars:
                data.append({
                    'timestamp': int(bar.timestamp),
                    'open': float(bar.open),
                    'high': float(bar.high),
                    'low': float(bar.low),
                    'close': float(bar.close),
                    'volume': float(bar.volume)
                })
            
            return {
                'data': data,
                'symbol': symbol,
                'exchange': exchange,
                'count': len(data)
            }
            
        except Exception as e:
            logger.error(f"Error getting OHLCV data: {e}")
            raise
    
    async def get_data_summary(self) -> Dict[str, Any]:
        """Get summary of all stored data
        
        Returns:
            Summary statistics of available data
        """
        try:
            summary = await self.repository.get_data_summary()
            
            # Convert DataSummary to dict for API response
            return {
                'total_symbols': summary.total_symbols,
                'total_exchanges': summary.total_exchanges,
                'total_records': summary.total_records,
                'date_range': summary.date_range,
                'symbols_by_exchange': summary.symbols_by_exchange,
                'record_count_by_symbol': summary.record_count_by_symbol
            }
        except Exception as e:
            logger.error(f"Error getting data summary: {e}")
            raise
    
    async def get_trades(
        self,
        symbol: str,
        exchange: str = "coinbase",
        start_time: Optional[int] = None,
        end_time: Optional[int] = None,
        limit: Optional[int] = None
    ) -> List[Trade]:
        """Get trade data for a symbol
        
        Args:
            symbol: Trading symbol
            exchange: Exchange name
            start_time: Start timestamp
            end_time: End timestamp
            limit: Max trades to return
            
        Returns:
            List of Trade objects
        """
        try:
            # Convert URL format to internal format
            symbol = symbol.replace('-', '/')
            
            # Convert timestamps to datetime if provided
            from datetime import datetime
            start_dt = datetime.fromtimestamp(start_time) if start_time else None
            end_dt = datetime.fromtimestamp(end_time) if end_time else None
            
            # Get trades from repository
            trades = await self.repository.get_trades(
                symbol, exchange, start_dt, end_dt, limit
            )
            
            return trades
        except Exception as e:
            logger.error(f"Error getting trades: {e}")
            raise
    
    async def get_correlation(
        self, 
        symbol1: str, 
        symbol2: str, 
        exchange: str = "coinbase"
    ) -> Dict[str, Any]:
        """Calculate correlation between two symbols
        
        Args:
            symbol1: First symbol
            symbol2: Second symbol
            exchange: Exchange name
            
        Returns:
            Correlation data
        """
        try:
            # Convert URL format to internal format
            symbol1 = symbol1.replace('-', '/')
            symbol2 = symbol2.replace('-', '/')
            
            # Get trades for both symbols
            trades1 = await self.repository.get_trades(symbol1, exchange)
            trades2 = await self.repository.get_trades(symbol2, exchange)
            
            if not trades1 or not trades2:
                return {
                    'status': 'error',
                    'message': 'Insufficient data for correlation calculation'
                }
            
            # Convert to price series and calculate correlation
            import pandas as pd
            import numpy as np
            
            # Create DataFrames with timestamps as index
            df1 = pd.DataFrame([
                {'timestamp': t.timestamp, 'price': t.price} 
                for t in trades1
            ]).set_index('timestamp')
            
            df2 = pd.DataFrame([
                {'timestamp': t.timestamp, 'price': t.price} 
                for t in trades2
            ]).set_index('timestamp')
            
            # Merge and calculate correlation
            merged = df1.join(df2, lsuffix='_1', rsuffix='_2', how='inner')
            
            if len(merged) < 2:
                return {
                    'status': 'error',
                    'message': 'Not enough overlapping data points'
                }
            
            correlation = merged['price_1'].corr(merged['price_2'])
            
            return {
                'symbol1': symbol1,
                'symbol2': symbol2,
                'exchange': exchange,
                'correlation': float(correlation),
                'data_points': len(merged)
            }
            
        except Exception as e:
            logger.error(f"Error calculating correlation: {e}")
            raise
    
    async def list_catalog_data(self) -> Dict[str, Any]:
        """List all available data in the catalog
        
        Returns:
            Catalog of available data
        """
        try:
            summary = await self.repository.get_data_summary()
            
            # Format as catalog
            catalog = []
            for exchange, symbols in summary.symbols_by_exchange.items():
                for symbol in symbols:
                    catalog.append({
                        'symbol': symbol,
                        'exchange': exchange,
                        'record_count': summary.record_count_by_symbol.get(symbol, 0)
                    })
            
            return {
                'status': 'success',
                'catalog': catalog,
                'total_items': len(catalog)
            }
        except Exception as e:
            logger.error(f"Error listing catalog: {e}")
            raise
    
    # Keep proxy methods for backward compatibility
    async def proxy_coinbase_data(self, endpoint: str, params: Dict[str, Any]) -> Dict[str, Any]:
        """Proxy requests to Coinbase API"""
        try:
            base_url = "https://api.exchange.coinbase.com"
            url = f"{base_url}/{endpoint}"
            
            response = requests.get(url, params=params, timeout=10)
            
            if response.status_code == 200:
                return {
                    'status': 'success',
                    'data': response.json(),
                    'status_code': response.status_code
                }
            else:
                return {
                    'status': 'error',
                    'message': f"Coinbase API error: {response.status_code}",
                    'status_code': response.status_code
                }
                
        except requests.exceptions.RequestException as e:
            logger.error(f"Error proxying Coinbase request: {e}")
            return {
                'status': 'error',
                'message': str(e),
                'status_code': 500
            }
    
    async def proxy_kraken_data(self, endpoint: str, params: Dict[str, Any]) -> Dict[str, Any]:
        """Proxy requests to Kraken API"""
        try:
            base_url = "https://api.kraken.com/0/public"
            url = f"{base_url}/{endpoint}"
            
            response = requests.get(url, params=params, timeout=10)
            
            if response.status_code == 200:
                data = response.json()
                if data.get('error'):
                    return {
                        'status': 'error',
                        'message': f"Kraken API error: {data['error']}",
                        'status_code': 400
                    }
                return {
                    'status': 'success',
                    'data': data.get('result', data),
                    'status_code': response.status_code
                }
            else:
                return {
                    'status': 'error',
                    'message': f"Kraken API error: {response.status_code}",
                    'status_code': response.status_code
                }
                
        except requests.exceptions.RequestException as e:
            logger.error(f"Error proxying Kraken request: {e}")
            return {
                'status': 'error',
                'message': str(e),
                'status_code': 500
            }
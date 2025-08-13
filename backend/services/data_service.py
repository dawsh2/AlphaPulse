"""
Data Service Layer - Business logic for market data operations
Handles data storage, retrieval, and analysis
"""
from typing import Dict, Any, Optional, List
import logging
import requests
from pathlib import Path

from data_manager import DataManager

logger = logging.getLogger(__name__)

class MarketDataService:
    """Service layer for market data operations"""
    
    def __init__(self, data_manager: DataManager = None):
        self.data_manager = data_manager or DataManager()
    
    async def save_market_data(self, symbol: str, exchange: str, candles: List[Dict], interval: str = "1m") -> Dict[str, Any]:
        """Save market data to storage
        
        Args:
            symbol: Trading symbol
            exchange: Exchange name
            candles: List of OHLCV candles
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
            
            # Save to Parquet/DuckDB
            save_result = self.data_manager.save_coinbase_data(candles, symbol, exchange)
            logger.info(f"Saved to Parquet: {save_result}")
            
            return {
                'status': 'success',
                'message': f'Saved {save_result.get("bars_saved", 0)} candles for {symbol}',
                'symbol': symbol,
                'exchange': exchange,
                'interval': interval,
                'candle_count': save_result.get('bars_saved', 0),
                'parquet_path': save_result.get('parquet_path', ''),
                'date_range': save_result.get('date_range', {})
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
            
            df = self.data_manager.get_ohlcv(symbol, exchange, start_time, end_time)
            
            if df.empty:
                return {
                    'data': [],
                    'symbol': symbol,
                    'exchange': exchange,
                    'count': 0
                }
            
            # Limit results if specified
            if limit and len(df) > limit:
                df = df.tail(limit)
            
            # Convert DataFrame to list of dicts
            data = []
            for _, row in df.iterrows():
                data.append({
                    'timestamp': int(row['timestamp']),
                    'open': float(row['open']),
                    'high': float(row['high']),
                    'low': float(row['low']),
                    'close': float(row['close']),
                    'volume': float(row['volume'])
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
            summary = self.data_manager.get_data_summary()
            return summary
        except Exception as e:
            logger.error(f"Error getting data summary: {e}")
            raise
    
    async def query_data(self, query: str) -> Dict[str, Any]:
        """Execute SQL query on DuckDB
        
        Args:
            query: SQL query string
            
        Returns:
            Query results
        """
        try:
            result = self.data_manager.query(query)
            return {
                'status': 'success',
                'result': result,
                'row_count': len(result) if isinstance(result, list) else 0
            }
        except Exception as e:
            logger.error(f"Error executing query: {e}")
            return {
                'status': 'error',
                'message': str(e)
            }
    
    async def get_correlation(self, symbol1: str, symbol2: str, exchange: str = "coinbase") -> Dict[str, Any]:
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
            
            # Get data for both symbols
            df1 = self.data_manager.get_ohlcv(symbol1, exchange)
            df2 = self.data_manager.get_ohlcv(symbol2, exchange)
            
            if df1.empty or df2.empty:
                return {
                    'status': 'error',
                    'message': 'Insufficient data for correlation calculation'
                }
            
            # Merge on timestamp and calculate correlation
            merged = df1[['timestamp', 'close']].merge(
                df2[['timestamp', 'close']], 
                on='timestamp', 
                suffixes=('_1', '_2')
            )
            
            if len(merged) < 2:
                return {
                    'status': 'error',
                    'message': 'Not enough overlapping data points'
                }
            
            correlation = merged['close_1'].corr(merged['close_2'])
            
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
            catalog = self.data_manager.list_available_data()
            return {
                'status': 'success',
                'catalog': catalog
            }
        except Exception as e:
            logger.error(f"Error listing catalog: {e}")
            raise
    
    async def proxy_coinbase_data(self, endpoint: str, params: Dict[str, Any]) -> Dict[str, Any]:
        """Proxy requests to Coinbase API
        
        Args:
            endpoint: API endpoint
            params: Query parameters
            
        Returns:
            API response
        """
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
        """Proxy requests to Kraken API
        
        Args:
            endpoint: API endpoint
            params: Query parameters
            
        Returns:
            API response
        """
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
    
    def close(self):
        """Cleanup resources"""
        if self.data_manager:
            self.data_manager.close()

# The dependency function is now in core.container
# Import it from there when needed in route files
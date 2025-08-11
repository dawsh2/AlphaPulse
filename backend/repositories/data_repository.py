"""
Data Repository - Data access layer abstraction
Provides clean interface to data storage operations
"""
from typing import Dict, Any, Optional, List
from abc import ABC, abstractmethod
import pandas as pd
from data_manager import DataManager


class DataRepositoryInterface(ABC):
    """Abstract interface for data repository operations"""
    
    @abstractmethod
    def save_market_data(self, data: List[List[float]], symbol: str, exchange: str) -> Dict[str, Any]:
        """Save market data to storage"""
        pass
    
    @abstractmethod
    def get_ohlcv(self, symbol: str, exchange: str, start_time: Optional[int] = None, end_time: Optional[int] = None) -> pd.DataFrame:
        """Get OHLCV data from storage"""
        pass
    
    @abstractmethod
    def get_summary(self) -> Dict[str, Any]:
        """Get summary of stored data"""
        pass
    
    @abstractmethod
    def execute_query(self, query: str) -> pd.DataFrame:
        """Execute SQL query on data"""
        pass
    
    @abstractmethod
    def calculate_statistics(self, symbol: str, exchange: str) -> Dict[str, float]:
        """Calculate basic statistics"""
        pass
    
    @abstractmethod
    def calculate_correlation(self, symbol1: str, symbol2: str, exchange: str) -> float:
        """Calculate correlation between symbols"""
        pass


class ParquetDataRepository(DataRepositoryInterface):
    """Parquet + DuckDB implementation of data repository"""
    
    def __init__(self, data_manager: DataManager = None):
        self.data_manager = data_manager or DataManager()
    
    def save_market_data(self, data: List[List[float]], symbol: str, exchange: str = "coinbase") -> Dict[str, Any]:
        """Save market data to Parquet storage"""
        try:
            return self.data_manager.save_coinbase_data(data, symbol, exchange)
        except Exception as e:
            raise Exception(f"Failed to save market data: {str(e)}")
    
    def get_ohlcv(self, symbol: str, exchange: str = "coinbase", start_time: Optional[int] = None, end_time: Optional[int] = None) -> pd.DataFrame:
        """Get OHLCV data from storage"""
        try:
            return self.data_manager.get_ohlcv(symbol, exchange, start_time, end_time)
        except Exception as e:
            raise Exception(f"Failed to get OHLCV data: {str(e)}")
    
    def get_returns(self, symbol: str, exchange: str = "coinbase") -> pd.DataFrame:
        """Get OHLCV data with returns calculated"""
        try:
            return self.data_manager.get_returns(symbol, exchange)
        except Exception as e:
            raise Exception(f"Failed to get returns data: {str(e)}")
    
    def get_summary(self) -> Dict[str, Any]:
        """Get summary of all stored data"""
        try:
            return self.data_manager.get_summary()
        except Exception as e:
            raise Exception(f"Failed to get data summary: {str(e)}")
    
    def execute_query(self, query: str) -> pd.DataFrame:
        """Execute SQL query on DuckDB"""
        try:
            return self.data_manager.query(query)
        except Exception as e:
            raise Exception(f"Query execution failed: {str(e)}")
    
    def calculate_statistics(self, symbol: str, exchange: str = "coinbase") -> Dict[str, float]:
        """Calculate basic statistics for symbol"""
        try:
            return self.data_manager.calculate_statistics(symbol, exchange)
        except Exception as e:
            raise Exception(f"Statistics calculation failed: {str(e)}")
    
    def calculate_correlation(self, symbol1: str, symbol2: str, exchange: str = "coinbase") -> float:
        """Calculate correlation between two symbols"""
        try:
            return self.data_manager.calculate_correlation(symbol1, symbol2, exchange)
        except Exception as e:
            raise Exception(f"Correlation calculation failed: {str(e)}")
    
    def get_metadata(self) -> pd.DataFrame:
        """Get metadata for all stored datasets"""
        try:
            return self.data_manager.get_metadata()
        except Exception as e:
            raise Exception(f"Failed to get metadata: {str(e)}")
    
    def export_to_csv(self, symbol: str, exchange: str = "coinbase", output_path: Optional[str] = None):
        """Export data to CSV format"""
        try:
            return self.data_manager.export_to_csv(symbol, exchange, output_path)
        except Exception as e:
            raise Exception(f"CSV export failed: {str(e)}")
    
    def close(self):
        """Close database connections"""
        if hasattr(self.data_manager, 'close'):
            self.data_manager.close()


class CacheDataRepository:
    """In-memory cache layer for frequently accessed data"""
    
    def __init__(self, underlying_repo: DataRepositoryInterface):
        self.underlying_repo = underlying_repo
        self.cache = {}
        self.cache_ttl = 300  # 5 minutes TTL
    
    def _get_cache_key(self, method_name: str, *args, **kwargs) -> str:
        """Generate cache key for method call"""
        key_parts = [method_name] + list(map(str, args))
        for k, v in sorted(kwargs.items()):
            key_parts.append(f"{k}:{v}")
        return ":".join(key_parts)
    
    def _is_cache_valid(self, cache_entry) -> bool:
        """Check if cache entry is still valid"""
        import time
        return time.time() - cache_entry['timestamp'] < self.cache_ttl
    
    def get_ohlcv(self, symbol: str, exchange: str = "coinbase", start_time: Optional[int] = None, end_time: Optional[int] = None) -> pd.DataFrame:
        """Get OHLCV data with caching"""
        cache_key = self._get_cache_key('get_ohlcv', symbol, exchange, start_time, end_time)
        
        if cache_key in self.cache and self._is_cache_valid(self.cache[cache_key]):
            return self.cache[cache_key]['data'].copy()
        
        # Fetch from underlying repository
        data = self.underlying_repo.get_ohlcv(symbol, exchange, start_time, end_time)
        
        # Cache the result
        import time
        self.cache[cache_key] = {
            'data': data.copy(),
            'timestamp': time.time()
        }
        
        return data
    
    def calculate_statistics(self, symbol: str, exchange: str = "coinbase") -> Dict[str, float]:
        """Get statistics with caching"""
        cache_key = self._get_cache_key('calculate_statistics', symbol, exchange)
        
        if cache_key in self.cache and self._is_cache_valid(self.cache[cache_key]):
            return self.cache[cache_key]['data']
        
        # Fetch from underlying repository
        stats = self.underlying_repo.calculate_statistics(symbol, exchange)
        
        # Cache the result
        import time
        self.cache[cache_key] = {
            'data': stats,
            'timestamp': time.time()
        }
        
        return stats
    
    def clear_cache(self):
        """Clear all cached data"""
        self.cache.clear()
    
    def __getattr__(self, name):
        """Delegate all other methods to underlying repository"""
        return getattr(self.underlying_repo, name)


# Factory function to create appropriate repository
def create_data_repository(repo_type: str = "parquet", use_cache: bool = True) -> DataRepositoryInterface:
    """Factory function to create data repository instances"""
    
    if repo_type == "parquet":
        repo = ParquetDataRepository()
    else:
        raise ValueError(f"Unsupported repository type: {repo_type}")
    
    if use_cache:
        repo = CacheDataRepository(repo)
    
    return repo
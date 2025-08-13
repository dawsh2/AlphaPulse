"""
Test Protocol Compliance
Ensures all repository implementations correctly follow the protocols
"""
import pytest
import inspect
from typing import get_type_hints

from repositories.protocols import (
    MarketDataRepository, AnalysisRepository, CacheRepository
)
from repositories.implementations.python.duckdb_market import DuckDBMarketRepository
from repositories.implementations.python.memory_analysis import MemoryAnalysisRepository
from repositories.implementations.python.redis_cache import RedisCacheRepository


class TestProtocolCompliance:
    """Test that implementations follow protocols correctly"""
    
    def test_market_repository_compliance(self):
        """Test DuckDBMarketRepository implements MarketDataRepository protocol"""
        repo = DuckDBMarketRepository()
        
        # Runtime check using @runtime_checkable
        assert isinstance(repo, MarketDataRepository), \
            "DuckDBMarketRepository does not implement MarketDataRepository protocol"
        
        # Check all required methods exist
        protocol_methods = [
            'save_trades', 'get_trades', 'get_ohlcv', 
            'get_orderbook_snapshot', 'get_symbols', 'get_data_summary'
        ]
        
        for method_name in protocol_methods:
            assert hasattr(repo, method_name), \
                f"Missing method: {method_name}"
            
            method = getattr(repo, method_name)
            assert callable(method), \
                f"{method_name} is not callable"
            
            # Check if async
            assert inspect.iscoroutinefunction(method), \
                f"{method_name} must be async"
    
    def test_analysis_repository_compliance(self):
        """Test MemoryAnalysisRepository implements AnalysisRepository protocol"""
        repo = MemoryAnalysisRepository()
        
        # Runtime check
        assert isinstance(repo, AnalysisRepository), \
            "MemoryAnalysisRepository does not implement AnalysisRepository protocol"
        
        # Check all required methods exist
        protocol_methods = [
            'calculate_statistics', 'calculate_correlation', 
            'calculate_volatility', 'calculate_risk_metrics', 'detect_regime'
        ]
        
        for method_name in protocol_methods:
            assert hasattr(repo, method_name), \
                f"Missing method: {method_name}"
            
            method = getattr(repo, method_name)
            assert callable(method), \
                f"{method_name} is not callable"
            
            assert inspect.iscoroutinefunction(method), \
                f"{method_name} must be async"
    
    def test_cache_repository_compliance(self):
        """Test RedisCacheRepository implements CacheRepository protocol"""
        # Initialize with fallback to memory (no Redis required)
        repo = RedisCacheRepository(host='invalid', port=9999)
        
        # Runtime check
        assert isinstance(repo, CacheRepository), \
            "RedisCacheRepository does not implement CacheRepository protocol"
        
        # Check all required methods exist
        protocol_methods = [
            'get', 'set', 'delete', 'exists', 'clear_pattern'
        ]
        
        for method_name in protocol_methods:
            assert hasattr(repo, method_name), \
                f"Missing method: {method_name}"
            
            method = getattr(repo, method_name)
            assert callable(method), \
                f"{method_name} is not callable"
            
            assert inspect.iscoroutinefunction(method), \
                f"{method_name} must be async"
    
    @pytest.mark.asyncio
    async def test_method_signatures_match(self):
        """Test that method signatures match protocol definitions"""
        from core.schemas import Trade, OHLCVBar
        
        # Test that return types are correct
        repo = DuckDBMarketRepository()
        
        # These should return the correct types
        trades = await repo.get_trades("BTC/USD", "coinbase")
        assert isinstance(trades, list), "get_trades must return a list"
        
        ohlcv = await repo.get_ohlcv("BTC/USD", "coinbase")
        assert isinstance(ohlcv, list), "get_ohlcv must return a list"
        
        # Test that we can create a mock that follows protocol
        class MockMarketRepo:
            async def save_trades(self, trades, symbol, exchange):
                return {'success': True}
            
            async def get_trades(self, symbol, exchange, start_time=None, end_time=None, limit=None):
                return []
            
            async def get_ohlcv(self, symbol, exchange, interval="1m", start_time=None, end_time=None):
                return []
            
            async def get_orderbook_snapshot(self, symbol, exchange, timestamp=None):
                from core.schemas import OrderBookSnapshot
                return OrderBookSnapshot(
                    symbol=symbol, exchange=exchange, 
                    timestamp=0, bids=[], asks=[]
                )
            
            async def get_symbols(self, exchange):
                return []
            
            async def get_data_summary(self):
                from core.schemas import DataSummary
                return DataSummary(
                    total_symbols=0, total_exchanges=0, total_records=0,
                    date_range={}, symbols_by_exchange={}, 
                    record_count_by_symbol={}
                )
        
        mock = MockMarketRepo()
        assert isinstance(mock, MarketDataRepository), \
            "Mock implementation should be recognized as MarketDataRepository"
    
    def test_protocol_documentation(self):
        """Test that protocols have proper documentation"""
        # Check that protocols have docstrings
        assert MarketDataRepository.__doc__ is not None
        assert "Rust trait" in MarketDataRepository.__doc__
        
        assert AnalysisRepository.__doc__ is not None
        assert "Rust trait" in AnalysisRepository.__doc__
        
        assert CacheRepository.__doc__ is not None
        assert "Rust trait" in CacheRepository.__doc__
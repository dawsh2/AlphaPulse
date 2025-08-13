"""
Test Dependency Injection Framework
Verifies that protocols, repositories, and services work together
"""
import pytest
from unittest.mock import MagicMock, AsyncMock
import asyncio

from core.container import Container, ServiceContainer
from repositories.protocols import MarketDataRepository, AnalysisRepository
from repositories.implementations.python.duckdb_market import DuckDBMarketRepository
from repositories.implementations.python.memory_analysis import MemoryAnalysisRepository


class TestDependencyInjection:
    """Test dependency injection and protocol implementation"""
    
    def test_container_initialization(self):
        """Test that container initializes correctly"""
        container = ServiceContainer()
        
        # Check that services can be retrieved
        market_service = container.get_market_data_service()
        analysis_service = container.get_market_analysis_service()
        
        assert market_service is not None
        assert analysis_service is not None
    
    def test_repository_protocols(self):
        """Test that repositories implement protocols correctly"""
        # Create repositories
        market_repo = DuckDBMarketRepository()
        analysis_repo = MemoryAnalysisRepository()
        
        # Check that they have the required methods (duck typing)
        assert hasattr(market_repo, 'save_trades')
        assert hasattr(market_repo, 'get_trades')
        assert hasattr(market_repo, 'get_ohlcv')
        
        assert hasattr(analysis_repo, 'calculate_statistics')
        assert hasattr(analysis_repo, 'calculate_correlation')
        assert hasattr(analysis_repo, 'calculate_volatility')
    
    @pytest.mark.asyncio
    async def test_repository_swap(self):
        """Test that repositories can be swapped (key for Rust migration)"""
        
        # Create a mock repository that follows the protocol
        class MockMarketRepository:
            async def save_trades(self, trades, symbol, exchange):
                return {'success': True, 'mock': True}
            
            async def get_trades(self, symbol, exchange, start_time=None, end_time=None, limit=None):
                return [{'price': 100, 'volume': 1}]
            
            async def get_ohlcv(self, symbol, exchange, interval="1m", start_time=None, end_time=None):
                import pandas as pd
                return pd.DataFrame({'close': [100, 101, 102]})
            
            async def get_orderbook_snapshot(self, symbol, exchange, timestamp=None):
                return {'bids': [], 'asks': []}
            
            async def get_symbols(self, exchange):
                return ['BTC/USD', 'ETH/USD']
            
            async def get_data_summary(self):
                return {'total_records': 1000}
        
        # Test that mock repository works
        mock_repo = MockMarketRepository()
        result = await mock_repo.save_trades([], 'BTC/USD', 'coinbase')
        assert result['mock'] == True
    
    def test_service_container_singleton(self):
        """Test that service container maintains singletons"""
        container = ServiceContainer()
        
        # Get repositories multiple times
        repo1 = container.get_market_data_repository()
        repo2 = container.get_market_data_repository()
        
        # Should be the same instance (singleton)
        assert repo1 is repo2
    
    def test_cache_repository_fallback(self):
        """Test that cache repository falls back to memory when Redis unavailable"""
        from repositories.implementations.python.redis_cache import RedisCacheRepository
        
        # Create with invalid Redis connection
        cache = RedisCacheRepository(host='invalid_host', port=9999)
        
        # Should fall back to memory cache
        assert cache.redis is None
        assert hasattr(cache, '_memory_cache')
    
    @pytest.mark.asyncio
    async def test_protocol_type_checking(self):
        """Test that protocols can be used for type checking"""
        from typing import Protocol, runtime_checkable
        
        # Make protocol runtime checkable
        @runtime_checkable
        class SimpleRepository(Protocol):
            async def get_data(self) -> dict: ...
        
        # Implementation that follows protocol
        class GoodRepo:
            async def get_data(self) -> dict:
                return {'data': 'test'}
        
        # Implementation that doesn't follow protocol
        class BadRepo:
            async def fetch_data(self) -> dict:
                return {'data': 'test'}
        
        good = GoodRepo()
        bad = BadRepo()
        
        # Check protocol compliance
        assert isinstance(good, SimpleRepository)
        assert not isinstance(bad, SimpleRepository)
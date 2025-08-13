"""
Dependency Injection Container
Manages service dependencies and enables easy swapping of implementations
"""
from dependency_injector import containers, providers
import os
import logging

from repositories.implementations.python.duckdb_market import DuckDBMarketRepository
from repositories.implementations.python.memory_analysis import MemoryAnalysisRepository
from repositories.implementations.python.redis_cache import RedisCacheRepository
from repositories.implementations.rust.market_data import RustMarketDataRepository

from services.data_service import MarketDataService
from services.analysis_service import MarketAnalysisService

from data_manager import DataManager

logger = logging.getLogger(__name__)


class Container(containers.DeclarativeContainer):
    """
    Main dependency injection container
    Configures all services and their dependencies
    """
    
    # Configuration
    config = providers.Configuration()
    
    # Core components
    data_manager = providers.Singleton(
        DataManager
    )
    
    # Repository implementations
    market_data_repository = providers.Singleton(
        DuckDBMarketRepository,
        data_manager=data_manager
    )
    
    analysis_repository = providers.Singleton(
        MemoryAnalysisRepository,
        data_manager=data_manager
    )
    
    cache_repository = providers.Singleton(
        RedisCacheRepository,
        host=providers.Configuration().redis.host,
        port=providers.Configuration().redis.port,
        db=providers.Configuration().redis.db,
        password=providers.Configuration().redis.password
    )
    
    # Services with injected repositories
    market_data_service = providers.Factory(
        MarketDataService,
        data_manager=data_manager  # Keep backward compatibility for now
        # TODO: Switch to repository injection:
        # repository=market_data_repository
    )
    
    market_analysis_service = providers.Factory(
        MarketAnalysisService,
        data_manager=data_manager  # Keep backward compatibility for now
        # TODO: Switch to repository injection:
        # repository=analysis_repository
    )


class ServiceContainer:
    """
    Service container with configuration based on environment
    """
    
    def __init__(self):
        self.container = Container()
        self._configure()
    
    def _configure(self):
        """Configure container based on environment"""
        env = os.getenv('APP_ENV', 'development')
        
        # Redis configuration
        self.container.config.redis.host.from_env('REDIS_HOST', default='localhost')
        self.container.config.redis.port.from_env('REDIS_PORT', default=6379)
        self.container.config.redis.db.from_env('REDIS_DB', default=0)
        self.container.config.redis.password.from_env('REDIS_PASSWORD', default=None)
        
        # Rust service configuration
        use_rust = os.getenv('USE_RUST_SERVICES', 'false').lower() == 'true'
        rust_api_url = os.getenv('RUST_API_URL', 'http://localhost:3001')
        
        if use_rust:
            logger.info(f"Rust services enabled - switching to RustMarketDataRepository at {rust_api_url}")
            self.container.market_data_repository.override(
                providers.Singleton(RustMarketDataRepository, rust_api_url=rust_api_url)
            )
        else:
            logger.info("Using Python DuckDB repository for market data")
        
        logger.info(f"ServiceContainer configured for {env} environment")
    
    def get_market_data_service(self) -> MarketDataService:
        """Get market data service with dependencies"""
        return self.container.market_data_service()
    
    def get_market_analysis_service(self) -> MarketAnalysisService:
        """Get market analysis service with dependencies"""
        return self.container.market_analysis_service()
    
    def get_market_data_repository(self):
        """Get market data repository"""
        return self.container.market_data_repository()
    
    def get_analysis_repository(self):
        """Get analysis repository"""
        return self.container.analysis_repository()
    
    def get_cache_repository(self):
        """Get cache repository"""
        return self.container.cache_repository()


# Global service container instance
service_container = ServiceContainer()


# FastAPI dependency functions
def get_market_data_service() -> MarketDataService:
    """FastAPI dependency for market data service"""
    return service_container.get_market_data_service()


def get_market_analysis_service() -> MarketAnalysisService:
    """FastAPI dependency for market analysis service"""
    return service_container.get_market_analysis_service()


def get_market_data_repository():
    """FastAPI dependency for market data repository"""
    return service_container.get_market_data_repository()


def get_analysis_repository():
    """FastAPI dependency for analysis repository"""
    return service_container.get_analysis_repository()


def get_cache_repository():
    """FastAPI dependency for cache repository"""
    return service_container.get_cache_repository()
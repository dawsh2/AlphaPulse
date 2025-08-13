"""
Repository Protocol Definitions
These protocols define the interfaces that both Python and Rust implementations must follow.
They map directly to Rust traits, enabling seamless integration between Python and Rust services.

IMPORTANT: These protocols must NOT use Python-specific types (pandas, numpy).
All types must be JSON-serializable for Rust compatibility.
"""
from typing import Protocol, List, Dict, Any, Optional, runtime_checkable
from datetime import datetime

# Import JSON-serializable types
from core.schemas import (
    Trade, OHLCVBar, OrderBookSnapshot, 
    MarketStatistics, RiskMetrics, MarketRegime, DataSummary
)


@runtime_checkable
class MarketDataRepository(Protocol):
    """
    Protocol for market data access
    Maps to Rust trait: MarketDataRepository
    """
    
    async def save_trades(
        self, 
        trades: List[Trade], 
        symbol: str, 
        exchange: str
    ) -> Dict[str, Any]:
        """Save trade data to storage"""
        ...
    
    async def get_trades(
        self,
        symbol: str,
        exchange: str,
        start_time: Optional[datetime] = None,
        end_time: Optional[datetime] = None,
        limit: Optional[int] = None
    ) -> List[Trade]:
        """Retrieve trade data from storage"""
        ...
    
    async def get_ohlcv(
        self,
        symbol: str,
        exchange: str,
        interval: str = "1m",
        start_time: Optional[datetime] = None,
        end_time: Optional[datetime] = None
    ) -> List[OHLCVBar]:
        """Get OHLCV candlestick data - returns JSON-serializable bars"""
        ...
    
    async def get_orderbook_snapshot(
        self,
        symbol: str,
        exchange: str,
        timestamp: Optional[datetime] = None
    ) -> OrderBookSnapshot:
        """Get orderbook snapshot at specific time"""
        ...
    
    async def get_symbols(self, exchange: str) -> List[str]:
        """Get list of available symbols for an exchange"""
        ...
    
    async def get_data_summary(self) -> DataSummary:
        """Get summary statistics of stored data"""
        ...


@runtime_checkable
class AnalysisRepository(Protocol):
    """
    Protocol for market analysis operations
    Maps to Rust trait: AnalysisRepository
    """
    
    async def calculate_statistics(
        self,
        symbol: str,
        exchange: str,
        window: Optional[int] = None
    ) -> MarketStatistics:
        """Calculate basic statistics for a symbol"""
        ...
    
    async def calculate_correlation(
        self,
        symbol1: str,
        symbol2: str,
        exchange: str,
        period: Optional[int] = None
    ) -> float:
        """Calculate correlation between two symbols"""
        ...
    
    async def calculate_volatility(
        self,
        symbol: str,
        exchange: str,
        window: int = 20
    ) -> float:
        """Calculate rolling volatility"""
        ...
    
    async def calculate_risk_metrics(
        self,
        symbol: str,
        exchange: str,
        risk_free_rate: float = 0.02
    ) -> RiskMetrics:
        """Calculate risk metrics (Sharpe, Sortino, etc.)"""
        ...
    
    async def detect_regime(
        self,
        symbol: str,
        exchange: str
    ) -> MarketRegime:
        """Detect market regime (trending, mean-reverting, etc.)"""
        ...


@runtime_checkable
class CacheRepository(Protocol):
    """
    Protocol for caching operations
    Maps to Rust trait: CacheRepository
    """
    
    async def get(self, key: str) -> Optional[Any]:
        """Get value from cache"""
        ...
    
    async def set(
        self,
        key: str,
        value: Any,
        ttl: Optional[int] = None
    ) -> bool:
        """Set value in cache with optional TTL"""
        ...
    
    async def delete(self, key: str) -> bool:
        """Delete key from cache"""
        ...
    
    async def exists(self, key: str) -> bool:
        """Check if key exists in cache"""
        ...
    
    async def clear_pattern(self, pattern: str) -> int:
        """Clear all keys matching pattern"""
        ...


class EventRepository(Protocol):
    """
    Protocol for event streaming
    Maps to Rust trait: EventRepository
    """
    
    async def publish(
        self,
        stream: str,
        event: Dict[str, Any]
    ) -> str:
        """Publish event to stream, returns event ID"""
        ...
    
    async def subscribe(
        self,
        stream: str,
        group: Optional[str] = None,
        consumer: Optional[str] = None
    ) -> Any:  # Returns async iterator
        """Subscribe to event stream"""
        ...
    
    async def acknowledge(
        self,
        stream: str,
        group: str,
        message_id: str
    ) -> bool:
        """Acknowledge message processing"""
        ...
    
    async def get_pending(
        self,
        stream: str,
        group: str,
        consumer: str
    ) -> List[Dict[str, Any]]:
        """Get pending messages for consumer"""
        ...


class OrderRepository(Protocol):
    """
    Protocol for order management
    Maps to Rust trait: OrderRepository
    """
    
    async def create_order(
        self,
        order: Dict[str, Any]
    ) -> str:
        """Create new order, returns order ID"""
        ...
    
    async def get_order(
        self,
        order_id: str
    ) -> Optional[Dict[str, Any]]:
        """Get order by ID"""
        ...
    
    async def update_order(
        self,
        order_id: str,
        updates: Dict[str, Any]
    ) -> bool:
        """Update existing order"""
        ...
    
    async def cancel_order(
        self,
        order_id: str
    ) -> bool:
        """Cancel order"""
        ...
    
    async def get_orders(
        self,
        user_id: Optional[str] = None,
        symbol: Optional[str] = None,
        status: Optional[str] = None,
        limit: int = 100
    ) -> List[Dict[str, Any]]:
        """Get orders with filters"""
        ...


class StrategyRepository(Protocol):
    """
    Protocol for strategy storage and retrieval
    Maps to Rust trait: StrategyRepository
    """
    
    async def save_strategy(
        self,
        strategy: Dict[str, Any]
    ) -> str:
        """Save strategy, returns strategy ID"""
        ...
    
    async def get_strategy(
        self,
        strategy_id: str
    ) -> Optional[Dict[str, Any]]:
        """Get strategy by ID"""
        ...
    
    async def list_strategies(
        self,
        user_id: Optional[str] = None,
        tags: Optional[List[str]] = None
    ) -> List[Dict[str, Any]]:
        """List strategies with filters"""
        ...
    
    async def update_strategy(
        self,
        strategy_id: str,
        updates: Dict[str, Any]
    ) -> bool:
        """Update strategy"""
        ...
    
    async def delete_strategy(
        self,
        strategy_id: str
    ) -> bool:
        """Delete strategy"""
        ...


class BacktestRepository(Protocol):
    """
    Protocol for backtest results storage
    Maps to Rust trait: BacktestRepository
    """
    
    async def save_backtest(
        self,
        backtest: Dict[str, Any]
    ) -> str:
        """Save backtest results, returns backtest ID"""
        ...
    
    async def get_backtest(
        self,
        backtest_id: str
    ) -> Optional[Dict[str, Any]]:
        """Get backtest by ID"""
        ...
    
    async def list_backtests(
        self,
        strategy_id: Optional[str] = None,
        user_id: Optional[str] = None
    ) -> List[Dict[str, Any]]:
        """List backtests with filters"""
        ...
    
    async def compare_backtests(
        self,
        backtest_ids: List[str]
    ) -> Dict[str, Any]:
        """Compare multiple backtest results"""
        ...
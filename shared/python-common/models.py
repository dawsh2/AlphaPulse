"""
Shared data models used across services

These are domain models that multiple services need to understand.
Keep them simple and focused on data structure, not business logic.
"""

from dataclasses import dataclass
from typing import Optional, List, Dict
from datetime import datetime
from enum import Enum


class Exchange(Enum):
    """Supported exchanges"""
    COINBASE = "coinbase"
    KRAKEN = "kraken"
    BINANCE_US = "binance_us"
    ALPACA = "alpaca"


class OrderSide(Enum):
    """Order side"""
    BUY = "buy"
    SELL = "sell"


@dataclass
class Trade:
    """Trade data model shared across services"""
    timestamp: float
    symbol: str
    exchange: str
    price: float
    volume: float
    side: str
    trade_id: str
    
    def to_dict(self) -> Dict:
        return {
            "timestamp": self.timestamp,
            "symbol": self.symbol,
            "exchange": self.exchange,
            "price": self.price,
            "volume": self.volume,
            "side": self.side,
            "trade_id": self.trade_id
        }


@dataclass
class OrderBookLevel:
    """Single level in an order book"""
    price: float
    size: float
    
    
@dataclass
class OrderBook:
    """Order book snapshot"""
    timestamp: float
    symbol: str
    exchange: str
    bids: List[OrderBookLevel]
    asks: List[OrderBookLevel]
    sequence: Optional[int] = None
    
    @property
    def best_bid(self) -> Optional[float]:
        return self.bids[0].price if self.bids else None
    
    @property
    def best_ask(self) -> Optional[float]:
        return self.asks[0].price if self.asks else None
    
    @property
    def spread(self) -> Optional[float]:
        if self.best_bid and self.best_ask:
            return self.best_ask - self.best_bid
        return None


@dataclass
class MarketDataRequest:
    """Request for market data"""
    symbol: str
    exchange: Exchange
    start_time: datetime
    end_time: datetime
    data_type: str = "trades"  # trades, orderbook, both
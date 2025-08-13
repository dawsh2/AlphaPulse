"""
DuckDB implementation of MarketDataRepository
Provides high-performance analytical queries on market data
"""
from typing import List, Dict, Any, Optional
from datetime import datetime
import pandas as pd
import logging
from pathlib import Path
import asyncio

from data_manager import DataManager
from core.schemas import (
    Trade, OHLCVBar, OrderBookSnapshot, OrderBookLevel, DataSummary,
    dataframe_to_ohlcv, ohlcv_to_dataframe
)

logger = logging.getLogger(__name__)


class DuckDBMarketRepository:
    """
    DuckDB implementation of MarketDataRepository protocol
    This will be replaced by RustMarketRepository in the future
    """
    
    def __init__(self, data_manager: Optional[DataManager] = None):
        """Initialize with DataManager for backward compatibility"""
        self.data_manager = data_manager or DataManager()
        logger.info("DuckDBMarketRepository initialized")
    
    async def save_trades(
        self, 
        trades: List[Trade], 
        symbol: str, 
        exchange: str
    ) -> Dict[str, Any]:
        """Save trade data to DuckDB/Parquet"""
        try:
            # Convert Trade objects to format expected by DataManager
            candles = []
            for trade in trades:
                candle = [
                    trade.timestamp,
                    trade.price,  # open
                    trade.price,  # high
                    trade.price,  # low
                    trade.price,  # close
                    trade.volume
                ]
                candles.append(candle)
            
            # DataManager is sync, so run in thread pool
            result = await asyncio.to_thread(
                self.data_manager.save_coinbase_data, 
                candles, symbol, exchange
            )
            
            return {
                'success': True,
                'symbol': symbol,
                'exchange': exchange,
                'trades_saved': len(trades),
                **result
            }
        except Exception as e:
            logger.error(f"Failed to save trades: {e}")
            return {
                'success': False,
                'error': str(e)
            }
    
    async def get_trades(
        self,
        symbol: str,
        exchange: str,
        start_time: Optional[datetime] = None,
        end_time: Optional[datetime] = None,
        limit: Optional[int] = None
    ) -> List[Trade]:
        """Retrieve trade data from DuckDB"""
        try:
            # Get OHLCV data using thread pool for sync call
            df = await asyncio.to_thread(
                self.data_manager.get_ohlcv,
                symbol, 
                exchange,
                start_time.timestamp() if start_time else None,
                end_time.timestamp() if end_time else None
            )
            
            if df.empty:
                return []
            
            # Limit results if specified
            if limit:
                df = df.head(limit)
            
            # Convert to Trade objects
            trades = []
            for _, row in df.iterrows():
                trades.append(Trade(
                    timestamp=float(row['timestamp']),
                    price=float(row['close']),
                    volume=float(row['volume'])
                ))
            
            return trades
        except Exception as e:
            logger.error(f"Failed to get trades: {e}")
            return []
    
    async def get_ohlcv(
        self,
        symbol: str,
        exchange: str,
        interval: str = "1m",
        start_time: Optional[datetime] = None,
        end_time: Optional[datetime] = None
    ) -> List[OHLCVBar]:
        """Get OHLCV candlestick data"""
        try:
            # Get DataFrame using thread pool
            df = await asyncio.to_thread(
                self.data_manager.get_ohlcv,
                symbol,
                exchange,
                start_time.timestamp() if start_time else None,
                end_time.timestamp() if end_time else None
            )
            
            if df.empty:
                return []
            
            # Convert DataFrame to OHLCVBar objects
            return dataframe_to_ohlcv(df)
        except Exception as e:
            logger.error(f"Failed to get OHLCV: {e}")
            return []
    
    async def get_orderbook_snapshot(
        self,
        symbol: str,
        exchange: str,
        timestamp: Optional[datetime] = None
    ) -> OrderBookSnapshot:
        """Get orderbook snapshot - not yet implemented"""
        # TODO: Implement when orderbook data is available
        return OrderBookSnapshot(
            symbol=symbol,
            exchange=exchange,
            timestamp=timestamp.timestamp() if timestamp else 0,
            bids=[],
            asks=[]
        )
    
    async def get_symbols(self, exchange: str) -> List[str]:
        """Get list of available symbols for an exchange"""
        try:
            summary = self.data_manager.get_summary()
            
            # Extract unique symbols for the exchange
            symbols = set()
            for item in summary.get('data', []):
                if item.get('exchange') == exchange:
                    symbols.add(item.get('symbol'))
            
            return sorted(list(symbols))
        except Exception as e:
            logger.error(f"Failed to get symbols: {e}")
            return []
    
    async def get_data_summary(self) -> DataSummary:
        """Get summary statistics of stored data"""
        try:
            summary_dict = await asyncio.to_thread(self.data_manager.get_summary)
            
            # Convert to DataSummary object
            symbols_by_exchange = {}
            record_count_by_symbol = {}
            
            for item in summary_dict.get('data', []):
                exchange = item.get('exchange', 'unknown')
                symbol = item.get('symbol', 'unknown')
                
                if exchange not in symbols_by_exchange:
                    symbols_by_exchange[exchange] = []
                symbols_by_exchange[exchange].append(symbol)
                
                record_count_by_symbol[symbol] = item.get('count', 0)
            
            return DataSummary(
                total_symbols=len(set(record_count_by_symbol.keys())),
                total_exchanges=len(symbols_by_exchange),
                total_records=sum(record_count_by_symbol.values()),
                date_range=summary_dict.get('date_range', {}),
                symbols_by_exchange=symbols_by_exchange,
                record_count_by_symbol=record_count_by_symbol
            )
        except Exception as e:
            logger.error(f"Failed to get data summary: {e}")
            return DataSummary(
                total_symbols=0,
                total_exchanges=0,
                total_records=0,
                date_range={},
                symbols_by_exchange={},
                record_count_by_symbol={}
            )
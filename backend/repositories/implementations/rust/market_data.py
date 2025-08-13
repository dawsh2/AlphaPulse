"""
Rust-backed MarketDataRepository implementation
Communicates with AlphaPulse Rust API server via HTTP
"""
from typing import List, Dict, Any, Optional
from datetime import datetime
import logging
import aiohttp
import asyncio

from core.schemas import (
    Trade, OHLCVBar, OrderBookSnapshot, DataSummary
)

logger = logging.getLogger(__name__)


class RustMarketDataRepository:
    """
    MarketDataRepository implementation that calls Rust API server
    Provides seamless integration between Python services and Rust collectors
    """
    
    def __init__(self, rust_api_url: str = "http://localhost:3001"):
        self.rust_api_url = rust_api_url.rstrip('/')
        self.session: Optional[aiohttp.ClientSession] = None
        logger.info(f"RustMarketDataRepository initialized with URL: {rust_api_url}")
    
    async def _get_session(self) -> aiohttp.ClientSession:
        """Get or create HTTP session"""
        if self.session is None or self.session.closed:
            timeout = aiohttp.ClientTimeout(total=30)
            self.session = aiohttp.ClientSession(timeout=timeout)
        return self.session
    
    async def _make_request(self, method: str, endpoint: str, params: Optional[Dict] = None) -> Dict[str, Any]:
        """Make HTTP request to Rust API server"""
        url = f"{self.rust_api_url}{endpoint}"
        session = await self._get_session()
        
        try:
            async with session.request(method, url, params=params) as response:
                if response.status == 200:
                    return await response.json()
                else:
                    error_text = await response.text()
                    raise Exception(f"Rust API error {response.status}: {error_text}")
        except aiohttp.ClientError as e:
            logger.error(f"HTTP request failed: {e}")
            raise Exception(f"Failed to connect to Rust API: {e}")
    
    async def save_trades(
        self, 
        trades: List[Trade], 
        symbol: str, 
        exchange: str
    ) -> Dict[str, Any]:
        """
        Save trades - Not implemented for Rust collector (it writes directly to Redis)
        This method exists for interface compatibility
        """
        logger.warning("save_trades called on RustMarketDataRepository - trades are written directly by Rust collectors")
        return {
            'success': True,
            'message': 'Rust collectors write trades directly to Redis Streams',
            'count': len(trades)
        }
    
    async def get_trades(
        self,
        symbol: str,
        exchange: str,
        start_time: Optional[datetime] = None,
        end_time: Optional[datetime] = None,
        limit: Optional[int] = None
    ) -> List[Trade]:
        """Retrieve trade data from Rust API server"""
        # Convert symbol to API format (BTC/USD -> BTC-USD for URL)
        api_symbol = symbol.replace('/', '-')
        endpoint = f"/trades/{api_symbol}"
        
        params = {'exchange': exchange}
        if start_time:
            params['start_time'] = start_time.timestamp()
        if end_time:
            params['end_time'] = end_time.timestamp()
        if limit:
            params['limit'] = limit
        
        try:
            response = await self._make_request('GET', endpoint, params)
            
            # Convert response data to Trade objects
            trades = []
            for trade_data in response.get('data', []):
                trade = Trade(
                    timestamp=trade_data['timestamp'],
                    price=trade_data['price'],
                    volume=trade_data['volume'],
                    side=trade_data.get('side'),
                    trade_id=trade_data.get('trade_id')
                )
                trades.append(trade)
            
            logger.info(f"Retrieved {len(trades)} trades for {symbol} from {exchange}")
            return trades
            
        except Exception as e:
            logger.error(f"Failed to get trades for {symbol}: {e}")
            raise
    
    async def get_ohlcv(
        self,
        symbol: str,
        exchange: str,
        interval: str = "1m",
        start_time: Optional[datetime] = None,
        end_time: Optional[datetime] = None
    ) -> List[OHLCVBar]:
        """Get OHLCV candlestick data from Rust API server"""
        api_symbol = symbol.replace('/', '-')
        endpoint = f"/ohlcv/{api_symbol}"
        
        params = {
            'exchange': exchange,
            'interval': interval
        }
        if start_time:
            params['start_time'] = start_time.timestamp()
        if end_time:
            params['end_time'] = end_time.timestamp()
        
        try:
            response = await self._make_request('GET', endpoint, params)
            
            # Convert response data to OHLCVBar objects
            bars = []
            for bar_data in response.get('data', []):
                bar = OHLCVBar(
                    timestamp=bar_data['timestamp'],
                    open=bar_data['open'],
                    high=bar_data['high'],
                    low=bar_data['low'],
                    close=bar_data['close'],
                    volume=bar_data['volume']
                )
                bars.append(bar)
            
            logger.info(f"Retrieved {len(bars)} OHLCV bars for {symbol} ({interval})")
            return bars
            
        except Exception as e:
            logger.error(f"Failed to get OHLCV for {symbol}: {e}")
            raise
    
    async def get_orderbook_snapshot(
        self,
        symbol: str,
        exchange: str,
        timestamp: Optional[datetime] = None
    ) -> OrderBookSnapshot:
        """
        Get orderbook snapshot - Not implemented in Phase 1
        Returns empty snapshot for interface compatibility
        """
        logger.warning("get_orderbook_snapshot not implemented in Phase 1 Rust collector")
        return OrderBookSnapshot(
            symbol=symbol,
            exchange=exchange,
            timestamp=timestamp.timestamp() if timestamp else 0.0,
            bids=[],
            asks=[]
        )
    
    async def get_symbols(self, exchange: str) -> List[str]:
        """Get list of available symbols for an exchange"""
        endpoint = f"/symbols/{exchange}"
        
        try:
            response = await self._make_request('GET', endpoint)
            symbols = response.get('symbols', [])
            
            logger.info(f"Retrieved {len(symbols)} symbols for {exchange}")
            return symbols
            
        except Exception as e:
            logger.error(f"Failed to get symbols for {exchange}: {e}")
            raise
    
    async def get_data_summary(self) -> DataSummary:
        """Get summary statistics of stored data"""
        endpoint = "/summary"
        
        try:
            response = await self._make_request('GET', endpoint)
            
            summary = DataSummary(
                total_symbols=response['total_symbols'],
                total_exchanges=response['total_exchanges'],
                total_records=response['total_records'],
                date_range=response['date_range'],
                symbols_by_exchange=response['symbols_by_exchange'],
                record_count_by_symbol=response['record_count_by_symbol']
            )
            
            logger.info(f"Retrieved data summary: {summary.total_symbols} symbols, {summary.total_exchanges} exchanges")
            return summary
            
        except Exception as e:
            logger.error(f"Failed to get data summary: {e}")
            raise
    
    async def health_check(self) -> Dict[str, Any]:
        """Check health of Rust API server"""
        endpoint = "/health"
        
        try:
            response = await self._make_request('GET', endpoint)
            logger.info("Rust API server health check passed")
            return response
            
        except Exception as e:
            logger.error(f"Rust API server health check failed: {e}")
            raise
    
    async def close(self):
        """Close HTTP session"""
        if self.session and not self.session.closed:
            await self.session.close()
            logger.info("Closed HTTP session to Rust API server")
    
    def __del__(self):
        """Cleanup on garbage collection"""
        if self.session and not self.session.closed:
            # Schedule session cleanup
            try:
                loop = asyncio.get_event_loop()
                if loop.is_running():
                    loop.create_task(self.close())
            except Exception:
                pass  # Ignore cleanup errors during shutdown
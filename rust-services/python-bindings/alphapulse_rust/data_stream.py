"""
High-level data streaming interface for AlphaPulse Python bindings.

Provides convenient async iterators and stream processors for real-time market data.
"""

import asyncio
import time
from typing import List, Dict, Optional, Callable, AsyncIterator, Any
from dataclasses import dataclass
import logging

from . import (
    PySharedMemoryReader,
    PyOrderBookDeltaReader, 
    PyTrade,
    PyOrderBookDelta,
    PyOrderBookReconstructor,
    PyArbitrageDetector
)

logger = logging.getLogger(__name__)

@dataclass
class StreamConfig:
    """Configuration for data streams"""
    yield_interval_us: int = 100  # Micro-sleep between reads (event-driven, not polling)
    buffer_size: int = 10000
    enable_metrics: bool = True
    max_latency_us: int = 100  # Alert if latency exceeds this
    batch_timeout_ms: int = 1   # Maximum wait time for batching when no data available
    
class DataStream:
    """
    High-level interface for streaming market data with minimal latency overhead.
    
    Provides async iteration over trades and orderbook deltas with automatic
    orderbook reconstruction and arbitrage detection.
    """
    
    def __init__(self, config: Optional[StreamConfig] = None):
        self.config = config or StreamConfig()
        self.trade_readers: Dict[str, PySharedMemoryReader] = {}
        self.delta_readers: Dict[str, PyOrderBookDeltaReader] = {}
        self.orderbook_reconstructor = PyOrderBookReconstructor()
        self.arbitrage_detector = PyArbitrageDetector(min_profit_bps=1.0, min_volume=0.1)
        self.metrics = {
            "trades_processed": 0,
            "deltas_processed": 0,
            "arbitrage_opportunities": 0,
            "avg_latency_us": 0.0,
            "last_update": time.time(),
        }
        self._running = False
        
    def add_trade_stream(self, exchange: str, path: str, reader_id: int) -> 'DataStream':
        """Add a trade data stream for an exchange"""
        try:
            reader = PySharedMemoryReader(path, reader_id)
            self.trade_readers[exchange] = reader
            logger.info(f"Added trade stream for {exchange}: {path}")
        except Exception as e:
            logger.error(f"Failed to add trade stream for {exchange}: {e}")
            raise
        return self
        
    def add_delta_stream(self, exchange: str, path: str, reader_id: int) -> 'DataStream':
        """Add an orderbook delta stream for an exchange"""
        try:
            reader = PyOrderBookDeltaReader(path, reader_id)
            self.delta_readers[exchange] = reader
            logger.info(f"Added delta stream for {exchange}: {path}")
        except Exception as e:
            logger.error(f"Failed to add delta stream for {exchange}: {e}")
            raise
        return self
        
    async def stream_trades(self) -> AsyncIterator[List[PyTrade]]:
        """
        Event-driven async iterator for real-time trade data from all exchanges.
        
        Yields batches of trades immediately when available, with sub-millisecond latency.
        """
        yield_interval = self.config.yield_interval_us / 1_000_000  # Convert to seconds
        
        while self._running:
            start_time = time.perf_counter()
            all_trades = []
            has_data = False
            
            # Read from all exchanges immediately (non-blocking)
            for exchange, reader in self.trade_readers.items():
                try:
                    trades = reader.read_trades()
                    if trades:
                        all_trades.extend(trades)
                        self.metrics["trades_processed"] += len(trades)
                        logger.debug(f"Read {len(trades)} trades from {exchange}")
                        has_data = True
                except Exception as e:
                    logger.warning(f"Error reading trades from {exchange}: {e}")
            
            # Update latency metrics and yield if we have data
            if all_trades:
                latency_us = (time.perf_counter() - start_time) * 1_000_000
                self._update_latency_metric(latency_us)
                
                if latency_us > self.config.max_latency_us:
                    logger.warning(f"High latency detected: {latency_us:.1f}μs")
                
                yield all_trades
            
            # Brief yield to prevent CPU spinning (event-driven, not polling)
            if not has_data:
                await asyncio.sleep(yield_interval)
            
    async def stream_deltas(self) -> AsyncIterator[List[PyOrderBookDelta]]:
        """
        Event-driven async iterator for real-time orderbook deltas from all exchanges.
        
        Automatically updates orderbook reconstructor and detects arbitrage opportunities.
        """
        yield_interval = self.config.yield_interval_us / 1_000_000  # Convert to seconds
        
        while self._running:
            start_time = time.perf_counter()
            all_deltas = []
            has_data = False
            
            for exchange, reader in self.delta_readers.items():
                try:
                    deltas = reader.read_deltas()
                    if deltas:
                        all_deltas.extend(deltas)
                        self.metrics["deltas_processed"] += len(deltas)
                        has_data = True
                        
                        # Update orderbook reconstructor
                        for delta in deltas:
                            self.orderbook_reconstructor.apply_delta(delta)
                            
                            # Update arbitrage detector
                            orderbook = self.orderbook_reconstructor.get_orderbook(
                                delta.exchange, delta.symbol
                            )
                            if orderbook:
                                self.arbitrage_detector.update_orderbook(orderbook)
                        
                        logger.debug(f"Read {len(deltas)} deltas from {exchange}")
                except Exception as e:
                    logger.warning(f"Error reading deltas from {exchange}: {e}")
            
            if all_deltas:
                # Update latency metrics
                latency_us = (time.perf_counter() - start_time) * 1_000_000
                self._update_latency_metric(latency_us)
                
                # Yield immediately if we have data
            if all_deltas:
                yield all_deltas
            
            # Brief yield to prevent CPU spinning (event-driven, not polling)
            if not has_data:
                await asyncio.sleep(yield_interval)
            
    async def stream_arbitrage_opportunities(self, symbols: List[str]) -> AsyncIterator[List[Dict[str, Any]]]:
        """
        Async iterator for cross-exchange arbitrage opportunities.
        
        Args:
            symbols: List of symbols to monitor for arbitrage (e.g., ["BTC/USD", "ETH/USD"])
        """
        yield_interval = self.config.yield_interval_us / 1_000_000 * 10  # Slightly less frequent than raw data
        
        while self._running:
            all_opportunities = []
            
            for symbol in symbols:
                try:
                    opportunities = self.arbitrage_detector.detect_opportunities(symbol)
                    if opportunities:
                        all_opportunities.extend(opportunities)
                        self.metrics["arbitrage_opportunities"] += len(opportunities)
                        logger.info(f"Found {len(opportunities)} arbitrage opportunities for {symbol}")
                except Exception as e:
                    logger.warning(f"Error detecting arbitrage for {symbol}: {e}")
            
            if all_opportunities:
                yield all_opportunities
            
            # Brief yield for arbitrage detection (event-driven)
            await asyncio.sleep(yield_interval)
            
    def get_orderbook(self, exchange: str, symbol: str):
        """Get current orderbook state for a symbol"""
        return self.orderbook_reconstructor.get_orderbook(exchange, symbol)
        
    def get_metrics(self) -> Dict[str, Any]:
        """Get current streaming metrics"""
        self.metrics["last_update"] = time.time()
        return self.metrics.copy()
        
    def _update_latency_metric(self, latency_us: float):
        """Update average latency metric with exponential moving average"""
        alpha = 0.1  # Smoothing factor
        if self.metrics["avg_latency_us"] == 0.0:
            self.metrics["avg_latency_us"] = latency_us
        else:
            self.metrics["avg_latency_us"] = (
                alpha * latency_us + (1 - alpha) * self.metrics["avg_latency_us"]
            )
            
    async def start(self):
        """Start the data stream"""
        self._running = True
        logger.info("DataStream started")
        
    async def stop(self):
        """Stop the data stream"""
        self._running = False
        logger.info("DataStream stopped")
        
    def __aenter__(self):
        """Async context manager entry"""
        return self
        
    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Async context manager exit"""
        await self.stop()

# Convenience functions for common use cases

async def monitor_real_time_trades(
    exchanges: Dict[str, str], 
    callback: Callable[[List[PyTrade]], None],
    config: Optional[StreamConfig] = None
):
    """
    Monitor real-time trades from multiple exchanges.
    
    Args:
        exchanges: Dict mapping exchange name to shared memory path
        callback: Function to call with each batch of trades
        config: Stream configuration
    """
    stream = DataStream(config)
    
    # Add all exchange streams
    for exchange, path in exchanges.items():
        stream.add_trade_stream(exchange, path, reader_id=1)
    
    await stream.start()
    
    try:
        async for trades in stream.stream_trades():
            callback(trades)
    finally:
        await stream.stop()

async def monitor_arbitrage_opportunities(
    delta_streams: Dict[str, str],
    symbols: List[str],
    min_profit_bps: float = 1.0,
    callback: Optional[Callable[[List[Dict[str, Any]]], None]] = None,
    config: Optional[StreamConfig] = None
):
    """
    Monitor cross-exchange arbitrage opportunities.
    
    Args:
        delta_streams: Dict mapping exchange name to delta shared memory path
        symbols: List of symbols to monitor
        min_profit_bps: Minimum profit in basis points
        callback: Function to call with arbitrage opportunities
        config: Stream configuration
    """
    stream = DataStream(config)
    stream.arbitrage_detector = PyArbitrageDetector(min_profit_bps, min_volume=0.1)
    
    # Add all delta streams
    for exchange, path in delta_streams.items():
        stream.add_delta_stream(exchange, path, reader_id=1)
    
    await stream.start()
    
    try:
        # Start delta processing in background
        delta_task = asyncio.create_task(stream._process_deltas())
        
        # Monitor arbitrage opportunities
        async for opportunities in stream.stream_arbitrage_opportunities(symbols):
            if callback:
                callback(opportunities)
            else:
                for opp in opportunities:
                    logger.info(f"Arbitrage: Buy {opp['symbol']} on {opp['buy_exchange']} "
                              f"at {opp['buy_price']}, sell on {opp['sell_exchange']} "
                              f"at {opp['sell_price']} (profit: {opp['profit_bps']:.1f} bps)")
    finally:
        await stream.stop()
        
async def _process_deltas(stream: DataStream):
    """Background task to process deltas for arbitrage detection"""
    async for deltas in stream.stream_deltas():
        pass  # Processing happens automatically in stream_deltas
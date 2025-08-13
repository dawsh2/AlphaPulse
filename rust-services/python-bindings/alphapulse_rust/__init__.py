"""
AlphaPulse Rust Python Bindings

Ultra-low latency market data access for Python applications, research, and trading strategies.
Provides sub-10μs shared memory access to real-time market data from multiple exchanges.

Key Features:
- Sub-10μs shared memory operations
- 99.975% bandwidth reduction through delta compression
- Real-time orderbook reconstruction
- Cross-exchange arbitrage detection
- NumPy integration for efficient data analysis

Example Usage:
    ```python
    import alphapulse_rust as ap
    
    # Ultra-fast trade data access
    reader = ap.PySharedMemoryReader("/tmp/alphapulse_shm/coinbase_trades", reader_id=1)
    trades = reader.read_trades()
    
    # Real-time orderbook deltas
    delta_reader = ap.PyOrderBookDeltaReader("/tmp/alphapulse_shm/coinbase_orderbook_deltas", reader_id=1)
    deltas = delta_reader.read_deltas()
    
    # Orderbook reconstruction
    reconstructor = ap.PyOrderBookReconstructor()
    for delta in deltas:
        reconstructor.apply_delta(delta)
    
    orderbook = reconstructor.get_orderbook("coinbase", "BTC/USD")
    print(f"Best bid: {orderbook.get_best_bid()}")
    print(f"Best ask: {orderbook.get_best_ask()}")
    print(f"Spread: {orderbook.get_spread()}")
    
    # Cross-exchange arbitrage detection
    arbitrage = ap.PyArbitrageDetector(min_profit_bps=1.0, min_volume=0.1)
    arbitrage.update_orderbook(orderbook)
    opportunities = arbitrage.detect_opportunities("BTC/USD")
    ```
"""

from .alphapulse_rust import (
    # Core data types
    PyTrade,
    PyPriceLevel, 
    PyOrderBookDelta,
    PyOrderBook,
    
    # Shared memory readers
    PySharedMemoryReader,
    PyOrderBookDeltaReader,
    
    # Analysis utilities
    PyOrderBookReconstructor,
    PyArbitrageDetector,
    
    # Performance utilities
    benchmark_shared_memory_latency,
    
    # Metadata
    __version__,
    __author__,
)

# Convenience imports for common patterns
from .data_stream import DataStream
from .jupyter_utils import JupyterDisplay
from .pandas_integration import to_pandas, from_pandas

__all__ = [
    # Core classes
    "PyTrade",
    "PyPriceLevel", 
    "PyOrderBookDelta",
    "PyOrderBook",
    "PySharedMemoryReader",
    "PyOrderBookDeltaReader",
    "PyOrderBookReconstructor",
    "PyArbitrageDetector",
    
    # Utility classes
    "DataStream",
    "JupyterDisplay",
    
    # Functions
    "benchmark_shared_memory_latency",
    "to_pandas",
    "from_pandas",
    
    # Metadata
    "__version__",
    "__author__",
]

# Performance information
PERFORMANCE_INFO = {
    "shared_memory_latency_us": "<10",
    "delta_compression_ratio": 0.99975,
    "supported_exchanges": ["coinbase", "kraken", "binance"],
    "max_throughput_msgs_per_sec": 100000,
    "python_overhead_us": "<10",
}
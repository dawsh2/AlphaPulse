# AlphaPulse Python Bindings

Ultra-low latency Python access to AlphaPulse's Rust shared memory infrastructure. Enables sub-10μs market data access for research, analysis, and trading strategy development.

## 🎯 Features

- **Sub-10μs Latency**: Direct shared memory access from Python
- **99.975% Bandwidth Reduction**: Delta compression for efficient data streaming
- **Multi-Exchange Support**: Coinbase, Kraken, Binance.US with standardized interfaces
- **Real-time Analysis**: Live orderbook reconstruction and arbitrage detection
- **Jupyter Integration**: Interactive widgets for market data visualization
- **Pandas Integration**: Seamless DataFrame conversion for data analysis
- **Async Support**: Non-blocking data streams with asyncio

## 🚀 Quick Start

### Installation

```bash
# Install from source (development)
cd python-bindings
pip install -e .

# Install with optional dependencies
pip install -e ".[jupyter,analysis]"
```

### Basic Usage

```python
import alphapulse_rust as ap

# Ultra-fast trade data access
reader = ap.PySharedMemoryReader("/tmp/alphapulse_shm/coinbase_trades", reader_id=1)
trades = reader.read_trades()

print(f"Read {len(trades)} trades")
for trade in trades[:5]:
    print(f"{trade.symbol}: ${trade.price:.2f} x {trade.volume:.4f}")
```

### Real-time Orderbook Deltas

```python
# Read orderbook deltas with 99.975% compression
delta_reader = ap.PyOrderBookDeltaReader("/tmp/alphapulse_shm/coinbase_orderbook_deltas", reader_id=1)
deltas = delta_reader.read_deltas()

# Reconstruct full orderbook from deltas
reconstructor = ap.PyOrderBookReconstructor()
for delta in deltas:
    reconstructor.apply_delta(delta)
    
orderbook = reconstructor.get_orderbook("coinbase", "BTC/USD")
if orderbook:
    print(f"Best bid: ${orderbook.get_best_bid():.2f}")
    print(f"Best ask: ${orderbook.get_best_ask():.2f}")
    print(f"Spread: {orderbook.get_spread():.4f}")
```

### Cross-Exchange Arbitrage Detection

```python
# Real-time arbitrage detection
arbitrage = ap.PyArbitrageDetector(min_profit_bps=1.0, min_volume=0.1)

# Update with orderbooks from multiple exchanges
for exchange in ["coinbase", "kraken", "binance"]:
    orderbook = get_orderbook(exchange, "BTC/USD")  # Your orderbook source
    arbitrage.update_orderbook(orderbook)

# Detect opportunities
opportunities = arbitrage.detect_opportunities("BTC/USD")
for opp in opportunities:
    print(f"Arbitrage: Buy on {opp['buy_exchange']} at ${opp['buy_price']:.2f}, "
          f"sell on {opp['sell_exchange']} at ${opp['sell_price']:.2f} "
          f"(profit: {opp['profit_bps']:.1f} bps)")
```

## 📊 High-Level Data Streaming

### Async Data Streams

```python
import asyncio
from alphapulse_rust import DataStream, StreamConfig

async def main():
    # Configure ultra-low latency streaming
    config = StreamConfig(
        yield_interval_us=100,  # 100μs yield interval (event-driven, not polling)
        max_latency_us=100,     # Alert if latency > 100μs
        enable_metrics=True
    )
    
    # Create data stream
    stream = DataStream(config)
    
    # Add exchange data sources
    stream.add_trade_stream("coinbase", "/tmp/alphapulse_shm/coinbase_trades", reader_id=1)
    stream.add_delta_stream("coinbase", "/tmp/alphapulse_shm/coinbase_orderbook_deltas", reader_id=1)
    stream.add_delta_stream("kraken", "/tmp/alphapulse_shm/kraken_orderbook_deltas", reader_id=2)
    
    await stream.start()
    
    # Stream real-time trades
    async for trades in stream.stream_trades():
        if trades:
            print(f"Received {len(trades)} trades")
            
    # Stream arbitrage opportunities
    async for opportunities in stream.stream_arbitrage_opportunities(["BTC/USD", "ETH/USD"]):
        for opp in opportunities:
            print(f"Arbitrage opportunity: {opp['profit_bps']:.2f} bps")

asyncio.run(main())
```

### Convenience Functions

```python
from alphapulse_rust import monitor_real_time_trades, monitor_arbitrage_opportunities

# Simple trade monitoring
def handle_trades(trades):
    print(f"Got {len(trades)} trades")

await monitor_real_time_trades({
    "coinbase": "/tmp/alphapulse_shm/coinbase_trades",
    "kraken": "/tmp/alphapulse_shm/kraken_trades",
}, handle_trades)

# Simple arbitrage monitoring
await monitor_arbitrage_opportunities({
    "coinbase": "/tmp/alphapulse_shm/coinbase_orderbook_deltas",
    "kraken": "/tmp/alphapulse_shm/kraken_orderbook_deltas",
}, ["BTC/USD", "ETH/USD"], min_profit_bps=1.0)
```

## 📈 Jupyter Notebook Integration

### Interactive Widgets

```python
from alphapulse_rust.jupyter_utils import (
    quick_trade_monitor,
    quick_orderbook_viewer,
    quick_arbitrage_monitor,
    quick_performance_dashboard
)

# Real-time trade monitoring widget
trade_widget = quick_trade_monitor(
    exchanges=["coinbase", "kraken", "binance"],
    symbols=["BTC/USD", "ETH/USD"]
)
display(trade_widget)

# Live orderbook depth visualization
orderbook_widget = quick_orderbook_viewer("coinbase", "BTC/USD")
display(orderbook_widget)

# Arbitrage opportunity monitor
arbitrage_widget = quick_arbitrage_monitor(["BTC/USD", "ETH/USD"])
display(arbitrage_widget)

# System performance dashboard
performance_widget = quick_performance_dashboard()
display(performance_widget)
```

### Advanced Jupyter Usage

```python
from alphapulse_rust.jupyter_utils import JupyterDisplay

# Create comprehensive display
display = JupyterDisplay()

# Create custom monitoring setup
trade_monitor = display.create_trade_monitor(
    exchanges=["coinbase", "kraken"], 
    symbols=["BTC/USD", "ETH/USD"]
)

# Update displays with real-time data
async def update_displays():
    stream = DataStream()
    # ... setup stream ...
    
    async for trades in stream.stream_trades():
        display.update_trades(trades)
        
    async for deltas in stream.stream_deltas():
        for delta in deltas:
            orderbook = reconstructor.get_orderbook(delta.exchange, delta.symbol)
            if orderbook:
                display.update_orderbook(orderbook)
```

## 🐼 Pandas Integration

### DataFrame Conversion

```python
from alphapulse_rust.pandas_integration import to_pandas, analyze_trades, calculate_ohlcv

# Convert trades to DataFrame
trades = reader.read_trades()
df = to_pandas(trades)

print(df.head())
```

```
                            symbol exchange    price  volume side trade_id
timestamp                                                                 
2024-01-15 10:30:00.123  BTC/USD  coinbase  45250.50  0.1234  buy      None
2024-01-15 10:30:00.456  BTC/USD  coinbase  45251.00  0.0567  sell     None
...
```

### Market Data Analysis

```python
# Comprehensive trade analysis
analysis = analyze_trades(df)
print(f"Total trades: {analysis['total_trades']}")
print(f"Average price: ${analysis['price_stats']['mean']:.2f}")
print(f"Total volume: {analysis['volume_stats']['total']:.4f}")

# Calculate OHLCV bars
ohlcv = calculate_ohlcv(df, timeframe='1T')  # 1-minute bars
print(ohlcv.head())

# Volume Weighted Average Price
vwap = calculate_vwap(df, timeframe='5T')  # 5-minute VWAP
print(vwap.head())

# Detect price anomalies
anomalies = detect_price_anomalies(df, std_threshold=3.0)
print(f"Found {len(anomalies)} price anomalies")
```

### Spread Analysis

```python
from alphapulse_rust.pandas_integration import calculate_spread_statistics

# Analyze spreads across exchanges
orderbooks = [reconstructor.get_orderbook(ex, "BTC/USD") for ex in exchanges]
spread_stats = calculate_spread_statistics([ob for ob in orderbooks if ob])

print("Spread Statistics:")
print(spread_stats.groupby('exchange')['spread_bps'].agg(['mean', 'median', 'std']))
```

## ⚡ Performance Benchmarking

### Latency Testing

```python
import alphapulse_rust as ap

# Benchmark shared memory latency
avg_latency_us = ap.benchmark_shared_memory_latency(
    "/tmp/alphapulse_shm/coinbase_trades", 
    iterations=10000
)
print(f"Average shared memory latency: {avg_latency_us:.2f} μs")

# Performance monitoring
stream = DataStream()
# ... setup and run stream ...

metrics = stream.get_metrics()
print(f"Average processing latency: {metrics['avg_latency_us']:.2f} μs")
print(f"Trades processed: {metrics['trades_processed']:,}")
print(f"Deltas processed: {metrics['deltas_processed']:,}")
```

## 🔧 Configuration

### Stream Configuration

```python
from alphapulse_rust import StreamConfig

config = StreamConfig(
    yield_interval_us=100,     # Ultra-fast 100μs yields (event-driven)
    buffer_size=10000,         # Large buffer for high throughput
    enable_metrics=True,       # Enable performance tracking
    max_latency_us=100         # Alert threshold for latency
)
```

### Display Configuration

```python
from alphapulse_rust.jupyter_utils import DisplayConfig

config = DisplayConfig(
    max_points=1000,           # Maximum data points to display
    update_interval_ms=100,    # Widget update frequency
    auto_scroll=True,          # Auto-scroll time series
    show_volume=True,          # Show volume charts
    show_spread=True           # Show spread information
)
```

## 📚 API Reference

### Core Classes

- **`PySharedMemoryReader`**: Ultra-fast trade data reader
- **`PyOrderBookDeltaReader`**: Orderbook delta stream reader  
- **`PyOrderBookReconstructor`**: Full orderbook reconstruction from deltas
- **`PyArbitrageDetector`**: Cross-exchange arbitrage detection
- **`DataStream`**: High-level async data streaming interface

### Data Types

- **`PyTrade`**: Individual trade data
- **`PyOrderBookDelta`**: Orderbook change information
- **`PyOrderBook`**: Full orderbook state
- **`PyPriceLevel`**: Individual price level change

### Utility Modules

- **`data_stream`**: High-level streaming interfaces
- **`jupyter_utils`**: Interactive Jupyter widgets
- **`pandas_integration`**: DataFrame conversion and analysis

## 🎯 Performance Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Shared Memory Latency** | <10μs | Time to read from shared memory |
| **Python Overhead** | <10μs | Additional latency from Python bindings |
| **Delta Compression** | >99% | Bandwidth reduction vs full orderbook |
| **Throughput** | 100k+ msgs/sec | Messages processed per second |
| **Memory Usage** | <100MB | Peak memory usage for typical workload |

## 🔍 Examples

See the `examples/` directory for complete examples:

- **`basic_usage.py`**: Simple shared memory reading
- **`real_time_analysis.py`**: Live market data analysis
- **`arbitrage_detection.py`**: Cross-exchange arbitrage monitoring
- **`jupyter_research.ipynb`**: Jupyter notebook with interactive widgets
- **`strategy_backtest.py`**: Historical strategy backtesting
- **`performance_benchmark.py`**: Latency and throughput testing

## 🐛 Troubleshooting

### Common Issues

**Import Error**: `ModuleNotFoundError: No module named 'alphapulse_rust'`
```bash
# Ensure you're in the python-bindings directory and install in development mode
pip install -e .
```

**Shared Memory Error**: `Failed to open shared memory`
```bash
# Ensure Rust collectors are running and shared memory paths exist
ls -la /tmp/alphapulse_shm/
```

**Permission Error**: `Permission denied accessing shared memory`
```bash
# Check shared memory permissions
sudo chmod 666 /tmp/alphapulse_shm/*
```

### Performance Optimization

- Use `reader_id` values that don't conflict with other readers
- Set `yield_interval_us=100` for maximum event-driven responsiveness
- Use `buffer_size` appropriate for your data volume
- Monitor `avg_latency_us` metrics to detect performance issues

## 📄 License

MIT License - see [LICENSE](../LICENSE) for details.

---

**🚀 Built for ultra-low latency trading and research**
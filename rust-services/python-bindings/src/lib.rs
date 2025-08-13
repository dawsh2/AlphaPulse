// AlphaPulse Python Bindings - Ultra-Low Latency Market Data Access
// 
// This module provides Python access to Rust's shared memory infrastructure,
// enabling sub-10μs market data access from Python applications, Jupyter notebooks,
// and trading strategies.

use pyo3::prelude::*;
use pyo3::types::{PyList, PyDict};
use pyo3_asyncio::tokio::future_into_py;
use alphapulse_common::{
    Trade, OrderBookDelta, OrderBookSnapshot, PriceLevel, DeltaAction,
    shared_memory::{SharedMemoryReader, OrderBookDeltaReader, SharedTrade, SharedOrderBookDelta}
};
use std::sync::Arc;
use tokio::sync::Mutex;
use numpy::{PyArray1, PyArray2};
use chrono::{DateTime, Utc};

/// Python wrapper for Trade data
#[pyclass]
#[derive(Clone)]
pub struct PyTrade {
    #[pyo3(get)]
    pub timestamp: f64,
    #[pyo3(get)]
    pub symbol: String,
    #[pyo3(get)]
    pub exchange: String,
    #[pyo3(get)]
    pub price: f64,
    #[pyo3(get)]
    pub volume: f64,
    #[pyo3(get)]
    pub side: Option<String>,
    #[pyo3(get)]
    pub trade_id: Option<String>,
}

#[pymethods]
impl PyTrade {
    #[new]
    fn new(
        timestamp: f64,
        symbol: String,
        exchange: String,
        price: f64,
        volume: f64,
        side: Option<String>,
        trade_id: Option<String>,
    ) -> Self {
        Self {
            timestamp,
            symbol,
            exchange,
            price,
            volume,
            side,
            trade_id,
        }
    }
    
    fn __repr__(&self) -> String {
        format!(
            "PyTrade(symbol='{}', exchange='{}', price={}, volume={}, timestamp={})",
            self.symbol, self.exchange, self.price, self.volume, self.timestamp
        )
    }
    
    fn to_dict(&self) -> PyResult<PyObject> {
        Python::with_gil(|py| {
            let dict = PyDict::new(py);
            dict.set_item("timestamp", self.timestamp)?;
            dict.set_item("symbol", &self.symbol)?;
            dict.set_item("exchange", &self.exchange)?;
            dict.set_item("price", self.price)?;
            dict.set_item("volume", self.volume)?;
            dict.set_item("side", &self.side)?;
            dict.set_item("trade_id", &self.trade_id)?;
            Ok(dict.into())
        })
    }
}

impl From<&SharedTrade> for PyTrade {
    fn from(shared_trade: &SharedTrade) -> Self {
        let symbol = std::str::from_utf8(&shared_trade.symbol)
            .unwrap_or("UNKNOWN")
            .trim_end_matches('\0')
            .to_string();
            
        let exchange = std::str::from_utf8(&shared_trade.exchange)
            .unwrap_or("UNKNOWN")
            .trim_end_matches('\0')
            .to_string();
            
        let trade_id = if shared_trade.trade_id[0] != 0 {
            Some(std::str::from_utf8(&shared_trade.trade_id)
                .unwrap_or("")
                .trim_end_matches('\0')
                .to_string())
        } else {
            None
        };
        
        let side = match shared_trade.side {
            0 => Some("buy".to_string()),
            1 => Some("sell".to_string()),
            _ => None,
        };
        
        Self {
            timestamp: shared_trade.timestamp_ns as f64 / 1_000_000_000.0,
            symbol,
            exchange,
            price: shared_trade.price,
            volume: shared_trade.volume,
            side,
            trade_id,
        }
    }
}

/// Python wrapper for OrderBook Price Level
#[pyclass]
#[derive(Clone)]
pub struct PyPriceLevel {
    #[pyo3(get)]
    pub price: f64,
    #[pyo3(get)]
    pub volume: f64,
    #[pyo3(get)]
    pub action: String, // "add", "update", "remove"
}

#[pymethods]
impl PyPriceLevel {
    #[new]
    fn new(price: f64, volume: f64, action: String) -> Self {
        Self { price, volume, action }
    }
    
    fn __repr__(&self) -> String {
        format!("PyPriceLevel(price={}, volume={}, action='{}')", 
                self.price, self.volume, self.action)
    }
}

impl From<&PriceLevel> for PyPriceLevel {
    fn from(level: &PriceLevel) -> Self {
        let action = match level.action {
            DeltaAction::Add => "add",
            DeltaAction::Update => "update", 
            DeltaAction::Remove => "remove",
        };
        
        Self {
            price: level.price,
            volume: level.volume,
            action: action.to_string(),
        }
    }
}

/// Python wrapper for OrderBook Delta
#[pyclass]
#[derive(Clone)]
pub struct PyOrderBookDelta {
    #[pyo3(get)]
    pub timestamp: f64,
    #[pyo3(get)]
    pub symbol: String,
    #[pyo3(get)]
    pub exchange: String,
    #[pyo3(get)]
    pub version: u64,
    #[pyo3(get)]
    pub prev_version: u64,
    #[pyo3(get)]
    pub bid_changes: Vec<PyPriceLevel>,
    #[pyo3(get)]
    pub ask_changes: Vec<PyPriceLevel>,
}

#[pymethods]
impl PyOrderBookDelta {
    fn __repr__(&self) -> String {
        format!(
            "PyOrderBookDelta(symbol='{}', exchange='{}', bid_changes={}, ask_changes={}, version={})",
            self.symbol, self.exchange, self.bid_changes.len(), self.ask_changes.len(), self.version
        )
    }
    
    fn compression_ratio(&self) -> f64 {
        // Estimate compression ratio vs full orderbook
        let delta_size = (self.bid_changes.len() + self.ask_changes.len()) as f64;
        let estimated_full_size = 100.0; // Typical orderbook has ~100 levels
        
        if estimated_full_size > 0.0 {
            1.0 - (delta_size / estimated_full_size)
        } else {
            0.0
        }
    }
    
    fn to_dict(&self) -> PyResult<PyObject> {
        Python::with_gil(|py| {
            let dict = PyDict::new(py);
            dict.set_item("timestamp", self.timestamp)?;
            dict.set_item("symbol", &self.symbol)?;
            dict.set_item("exchange", &self.exchange)?;
            dict.set_item("version", self.version)?;
            dict.set_item("prev_version", self.prev_version)?;
            
            let bid_changes: Vec<PyObject> = self.bid_changes
                .iter()
                .map(|level| level.clone().into_py(py))
                .collect();
            dict.set_item("bid_changes", bid_changes)?;
            
            let ask_changes: Vec<PyObject> = self.ask_changes
                .iter()
                .map(|level| level.clone().into_py(py))
                .collect();
            dict.set_item("ask_changes", ask_changes)?;
            
            Ok(dict.into())
        })
    }
}

impl From<&OrderBookDelta> for PyOrderBookDelta {
    fn from(delta: &OrderBookDelta) -> Self {
        let bid_changes = delta.bid_changes
            .iter()
            .map(PyPriceLevel::from)
            .collect();
            
        let ask_changes = delta.ask_changes
            .iter()
            .map(PyPriceLevel::from)
            .collect();
        
        Self {
            timestamp: delta.timestamp,
            symbol: delta.symbol.clone(),
            exchange: delta.exchange.clone(),
            version: delta.version,
            prev_version: delta.prev_version,
            bid_changes,
            ask_changes,
        }
    }
}

/// Ultra-fast shared memory reader for trade data
#[pyclass]
pub struct PySharedMemoryReader {
    reader: Arc<Mutex<SharedMemoryReader>>,
    last_sequence: u64,
}

#[pymethods]
impl PySharedMemoryReader {
    #[new]
    fn new(path: &str, reader_id: usize) -> PyResult<Self> {
        let reader = SharedMemoryReader::open(path, reader_id)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to open shared memory: {}", e)
            ))?;
        
        Ok(Self {
            reader: Arc::new(Mutex::new(reader)),
            last_sequence: 0,
        })
    }
    
    /// Read all new trades since last call (non-blocking)
    fn read_trades(&mut self, py: Python) -> PyResult<Vec<PyTrade>> {
        let reader = self.reader.clone();
        
        py.allow_threads(|| {
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    format!("Failed to create async runtime: {}", e)
                ))?;
            
            rt.block_on(async {
                let mut reader_guard = reader.lock().await;
                
                match reader_guard.read_trades() {
                    Ok(trades) => {
                        let py_trades: Vec<PyTrade> = trades
                            .iter()
                            .map(PyTrade::from)
                            .collect();
                        Ok(py_trades)
                    }
                    Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                        format!("Failed to read trades: {}", e)
                    ))
                }
            })
        })
    }
    
    /// Get memory statistics
    fn get_stats(&self, py: Python) -> PyResult<PyObject> {
        let reader = self.reader.clone();
        
        py.allow_threads(|| {
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    format!("Failed to create async runtime: {}", e)
                ))?;
            
            rt.block_on(async {
                let reader_guard = reader.lock().await;
                
                Python::with_gil(|py| {
                    let dict = PyDict::new(py);
                    dict.set_item("capacity", reader_guard.capacity())?;
                    dict.set_item("available", reader_guard.available())?;
                    dict.set_item("reader_id", reader_guard.reader_id())?;
                    Ok(dict.into())
                })
            })
        })
    }
}

/// Ultra-fast orderbook delta reader
#[pyclass]
pub struct PyOrderBookDeltaReader {
    reader: Arc<Mutex<OrderBookDeltaReader>>,
}

#[pymethods]
impl PyOrderBookDeltaReader {
    #[new]
    fn new(path: &str, reader_id: usize) -> PyResult<Self> {
        let reader = OrderBookDeltaReader::open(path, reader_id)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to open orderbook delta reader: {}", e)
            ))?;
        
        Ok(Self {
            reader: Arc::new(Mutex::new(reader)),
        })
    }
    
    /// Read all new deltas since last call
    fn read_deltas(&mut self, py: Python) -> PyResult<Vec<PyOrderBookDelta>> {
        let reader = self.reader.clone();
        
        py.allow_threads(|| {
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    format!("Failed to create async runtime: {}", e)
                ))?;
            
            rt.block_on(async {
                let mut reader_guard = reader.lock().await;
                
                match reader_guard.read_deltas() {
                    Ok(deltas) => {
                        // Convert SharedOrderBookDelta to OrderBookDelta then to PyOrderBookDelta
                        let py_deltas: Vec<PyOrderBookDelta> = deltas
                            .iter()
                            .filter_map(|shared_delta| {
                                // Convert shared delta to regular delta format
                                // This is a simplified conversion - in practice you'd fully reconstruct
                                let symbol = std::str::from_utf8(&shared_delta.symbol)
                                    .unwrap_or("UNKNOWN")
                                    .trim_end_matches('\0');
                                let exchange = std::str::from_utf8(&shared_delta.exchange)
                                    .unwrap_or("UNKNOWN")
                                    .trim_end_matches('\0');
                                
                                // Extract changes from shared delta
                                let mut bid_changes = Vec::new();
                                let mut ask_changes = Vec::new();
                                
                                for i in 0..shared_delta.change_count as usize {
                                    if i < shared_delta.changes.len() {
                                        let change = &shared_delta.changes[i];
                                        let py_level = PyPriceLevel {
                                            price: change.price,
                                            volume: change.volume,
                                            action: match change.action {
                                                0 => "add".to_string(),
                                                1 => "update".to_string(),
                                                2 => "remove".to_string(),
                                                _ => "unknown".to_string(),
                                            }
                                        };
                                        
                                        if change.is_ask != 0 {
                                            ask_changes.push(py_level);
                                        } else {
                                            bid_changes.push(py_level);
                                        }
                                    }
                                }
                                
                                Some(PyOrderBookDelta {
                                    timestamp: shared_delta.timestamp_ns as f64 / 1_000_000_000.0,
                                    symbol: symbol.to_string(),
                                    exchange: exchange.to_string(),
                                    version: shared_delta.version,
                                    prev_version: shared_delta.prev_version,
                                    bid_changes,
                                    ask_changes,
                                })
                            })
                            .collect();
                        Ok(py_deltas)
                    }
                    Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                        format!("Failed to read deltas: {}", e)
                    ))
                }
            })
        })
    }
}

/// OrderBook reconstructor for full orderbook from deltas
#[pyclass]
pub struct PyOrderBookReconstructor {
    orderbooks: std::collections::HashMap<String, PyOrderBook>,
}

#[pyclass]
#[derive(Clone)]
pub struct PyOrderBook {
    #[pyo3(get)]
    pub symbol: String,
    #[pyo3(get)]
    pub exchange: String,
    #[pyo3(get)]
    pub timestamp: f64,
    #[pyo3(get)]
    pub version: u64,
    pub bids: std::collections::BTreeMap<u64, f64>, // price_key -> volume
    pub asks: std::collections::BTreeMap<u64, f64>, // price_key -> volume
}

#[pymethods]
impl PyOrderBook {
    fn get_bids(&self, py: Python) -> PyResult<PyObject> {
        let bids: Vec<[f64; 2]> = self.bids
            .iter()
            .rev() // Highest prices first
            .map(|(price_key, volume)| [*price_key as f64 / 100000.0, *volume])
            .collect();
        
        let array = PyArray2::from_vec2(py, &[bids])?;
        Ok(array.into_py(py))
    }
    
    fn get_asks(&self, py: Python) -> PyResult<PyObject> {
        let asks: Vec<[f64; 2]> = self.asks
            .iter()
            .map(|(price_key, volume)| [*price_key as f64 / 100000.0, *volume])
            .collect();
        
        let array = PyArray2::from_vec2(py, &[asks])?;
        Ok(array.into_py(py))
    }
    
    fn get_best_bid(&self) -> Option<f64> {
        self.bids.iter().next_back().map(|(price_key, _)| *price_key as f64 / 100000.0)
    }
    
    fn get_best_ask(&self) -> Option<f64> {
        self.asks.iter().next().map(|(price_key, _)| *price_key as f64 / 100000.0)
    }
    
    fn get_spread(&self) -> Option<f64> {
        match (self.get_best_ask(), self.get_best_bid()) {
            (Some(ask), Some(bid)) => Some(ask - bid),
            _ => None,
        }
    }
}

#[pymethods]
impl PyOrderBookReconstructor {
    #[new]
    fn new() -> Self {
        Self {
            orderbooks: std::collections::HashMap::new(),
        }
    }
    
    /// Apply delta to reconstruct orderbook state
    fn apply_delta(&mut self, delta: &PyOrderBookDelta) {
        let key = format!("{}:{}", delta.exchange, delta.symbol);
        
        let orderbook = self.orderbooks.entry(key.clone()).or_insert_with(|| {
            PyOrderBook {
                symbol: delta.symbol.clone(),
                exchange: delta.exchange.clone(),
                timestamp: delta.timestamp,
                version: delta.version,
                bids: std::collections::BTreeMap::new(),
                asks: std::collections::BTreeMap::new(),
            }
        });
        
        // Update orderbook with delta changes
        orderbook.timestamp = delta.timestamp;
        orderbook.version = delta.version;
        
        // Apply bid changes
        for change in &delta.bid_changes {
            let price_key = (change.price * 100000.0) as u64;
            match change.action.as_str() {
                "add" | "update" => {
                    if change.volume > 0.0 {
                        orderbook.bids.insert(price_key, change.volume);
                    } else {
                        orderbook.bids.remove(&price_key);
                    }
                }
                "remove" => {
                    orderbook.bids.remove(&price_key);
                }
                _ => {}
            }
        }
        
        // Apply ask changes
        for change in &delta.ask_changes {
            let price_key = (change.price * 100000.0) as u64;
            match change.action.as_str() {
                "add" | "update" => {
                    if change.volume > 0.0 {
                        orderbook.asks.insert(price_key, change.volume);
                    } else {
                        orderbook.asks.remove(&price_key);
                    }
                }
                "remove" => {
                    orderbook.asks.remove(&price_key);
                }
                _ => {}
            }
        }
    }
    
    /// Get current orderbook state
    fn get_orderbook(&self, exchange: &str, symbol: &str) -> Option<PyOrderBook> {
        let key = format!("{}:{}", exchange, symbol);
        self.orderbooks.get(&key).cloned()
    }
    
    /// Get all available symbols
    fn get_symbols(&self) -> Vec<String> {
        self.orderbooks.keys().cloned().collect()
    }
}

/// Cross-exchange arbitrage detector
#[pyclass]
pub struct PyArbitrageDetector {
    orderbooks: std::collections::HashMap<String, PyOrderBook>,
    min_profit_bps: f64,
    min_volume: f64,
}

#[pymethods]
impl PyArbitrageDetector {
    #[new]
    fn new(min_profit_bps: f64, min_volume: f64) -> Self {
        Self {
            orderbooks: std::collections::HashMap::new(),
            min_profit_bps,
            min_volume,
        }
    }
    
    /// Update orderbook state
    fn update_orderbook(&mut self, orderbook: PyOrderBook) {
        let key = format!("{}:{}", orderbook.exchange, orderbook.symbol);
        self.orderbooks.insert(key, orderbook);
    }
    
    /// Detect arbitrage opportunities
    fn detect_opportunities(&self, symbol: &str) -> Vec<PyObject> {
        let mut opportunities = Vec::new();
        
        // Find all exchanges with this symbol
        let exchanges: Vec<(&String, &PyOrderBook)> = self.orderbooks
            .iter()
            .filter(|(key, _)| key.ends_with(&format!(":{}", symbol)))
            .collect();
        
        // Compare all exchange pairs
        for (i, (key1, book1)) in exchanges.iter().enumerate() {
            for (key2, book2) in exchanges.iter().skip(i + 1) {
                if let (Some(ask1), Some(bid2)) = (book1.get_best_ask(), book2.get_best_bid()) {
                    if bid2 > ask1 {
                        let profit_bps = ((bid2 - ask1) / ask1) * 10000.0;
                        if profit_bps >= self.min_profit_bps {
                            Python::with_gil(|py| {
                                let dict = PyDict::new(py);
                                dict.set_item("symbol", symbol).unwrap();
                                dict.set_item("buy_exchange", book1.exchange.clone()).unwrap();
                                dict.set_item("sell_exchange", book2.exchange.clone()).unwrap();
                                dict.set_item("buy_price", ask1).unwrap();
                                dict.set_item("sell_price", bid2).unwrap();
                                dict.set_item("profit_bps", profit_bps).unwrap();
                                dict.set_item("timestamp", book1.timestamp.max(book2.timestamp)).unwrap();
                                opportunities.push(dict.into());
                            });
                        }
                    }
                }
                
                // Check the reverse direction
                if let (Some(ask2), Some(bid1)) = (book2.get_best_ask(), book1.get_best_bid()) {
                    if bid1 > ask2 {
                        let profit_bps = ((bid1 - ask2) / ask2) * 10000.0;
                        if profit_bps >= self.min_profit_bps {
                            Python::with_gil(|py| {
                                let dict = PyDict::new(py);
                                dict.set_item("symbol", symbol).unwrap();
                                dict.set_item("buy_exchange", book2.exchange.clone()).unwrap();
                                dict.set_item("sell_exchange", book1.exchange.clone()).unwrap();
                                dict.set_item("buy_price", ask2).unwrap();
                                dict.set_item("sell_price", bid1).unwrap();
                                dict.set_item("profit_bps", profit_bps).unwrap();
                                dict.set_item("timestamp", book1.timestamp.max(book2.timestamp)).unwrap();
                                opportunities.push(dict.into());
                            });
                        }
                    }
                }
            }
        }
        
        opportunities
    }
}

/// Performance testing utilities
#[pyfunction]
fn benchmark_shared_memory_latency(path: &str, iterations: usize) -> PyResult<f64> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Failed to create async runtime: {}", e)
        ))?;
    
    rt.block_on(async {
        let mut reader = SharedMemoryReader::open(path, 999)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to open shared memory: {}", e)
            ))?;
        
        let start = std::time::Instant::now();
        
        for _ in 0..iterations {
            let _ = reader.read_trades().unwrap_or_default();
        }
        
        let elapsed = start.elapsed();
        let avg_latency_us = elapsed.as_nanos() as f64 / iterations as f64 / 1000.0;
        
        Ok(avg_latency_us)
    })
}

/// Module initialization
#[pymodule]
fn alphapulse_rust(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyTrade>()?;
    m.add_class::<PyPriceLevel>()?;
    m.add_class::<PyOrderBookDelta>()?;
    m.add_class::<PySharedMemoryReader>()?;
    m.add_class::<PyOrderBookDeltaReader>()?;
    m.add_class::<PyOrderBook>()?;
    m.add_class::<PyOrderBookReconstructor>()?;
    m.add_class::<PyArbitrageDetector>()?;
    m.add_function(wrap_pyfunction!(benchmark_shared_memory_latency, m)?)?;
    
    // Add version info
    m.add("__version__", "0.1.0")?;
    m.add("__author__", "AlphaPulse Team")?;
    
    Ok(())
}
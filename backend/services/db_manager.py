#!/usr/bin/env python3
"""
Database Manager for Streaming Data
Handles concurrent access to DuckDB with connection pooling and WAL mode
"""

import duckdb
import threading
import time
import queue
import logging
from pathlib import Path
from typing import Dict, Any, List, Optional
import pandas as pd

logger = logging.getLogger(__name__)

class DuckDBManager:
    """Thread-safe DuckDB manager for streaming data"""
    
    def __init__(self, db_path: str = '../market_data/market_data.duckdb'):
        self.db_path = Path(db_path)
        self.write_queue = queue.Queue()
        self.running = True
        self._lock = threading.Lock()
        
        # Ensure directory exists
        self.db_path.parent.mkdir(parents=True, exist_ok=True)
        
        # Initialize database with WAL mode and optimizations
        self._init_database()
        
        # Start write thread
        self.write_thread = threading.Thread(target=self._write_worker, daemon=True)
        self.write_thread.start()
        
        logger.info(f"DuckDB Manager initialized: {self.db_path}")
    
    def _init_database(self):
        """Initialize database with optimizations for streaming data"""
        try:
            with duckdb.connect(str(self.db_path)) as conn:
                # Optimize for streaming workloads
                conn.execute("SET memory_limit='1GB'")
                conn.execute("SET threads=4")
                conn.execute("SET enable_progress_bar=false")
                
                # Create trades table if not exists
                conn.execute("""
                    CREATE TABLE IF NOT EXISTS trades (
                        timestamp DOUBLE,
                        datetime TIMESTAMP,
                        symbol VARCHAR,
                        exchange VARCHAR,
                        price DOUBLE,
                        size DOUBLE,
                        side VARCHAR,
                        trade_id VARCHAR,
                        created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                        PRIMARY KEY (exchange, trade_id)
                    )
                """)
                
                # Create indexes for common queries
                conn.execute("""
                    CREATE INDEX IF NOT EXISTS idx_trades_exchange_timestamp 
                    ON trades (exchange, timestamp)
                """)
                
                conn.execute("""
                    CREATE INDEX IF NOT EXISTS idx_trades_symbol_timestamp 
                    ON trades (symbol, timestamp)
                """)
                
                # Create OHLCV table optimized for time-series
                conn.execute("""
                    CREATE TABLE IF NOT EXISTS ohlcv (
                        datetime TIMESTAMP,
                        exchange VARCHAR,
                        symbol VARCHAR,
                        open DOUBLE,
                        high DOUBLE,
                        low DOUBLE,
                        close DOUBLE,
                        volume DOUBLE,
                        trade_count INTEGER DEFAULT 0,
                        created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                        PRIMARY KEY (exchange, symbol, datetime)
                    )
                """)
                
                logger.info("Database initialized with optimizations")
                
        except Exception as e:
            logger.error(f"Failed to initialize database: {e}")
            raise
    
    def queue_trades(self, trades: List[Dict[str, Any]], table: str = 'trades'):
        """Queue trades for async insertion"""
        if not trades:
            return
            
        self.write_queue.put({
            'type': 'insert',
            'table': table,
            'data': trades
        })
    
    def queue_ohlcv(self, ohlcv_data: List[Dict[str, Any]]):
        """Queue OHLCV data for async insertion"""
        if not ohlcv_data:
            return
            
        self.write_queue.put({
            'type': 'insert',
            'table': 'ohlcv',
            'data': ohlcv_data
        })
    
    def _write_worker(self):
        """Background thread that processes write queue"""
        logger.info("Database write worker started")
        
        while self.running:
            try:
                # Block for up to 1 second waiting for writes
                operation = self.write_queue.get(timeout=1.0)
                
                if operation['type'] == 'insert':
                    self._execute_insert(operation['table'], operation['data'])
                
                self.write_queue.task_done()
                
            except queue.Empty:
                continue
            except Exception as e:
                logger.error(f"Write worker error: {e}")
    
    def _execute_insert(self, table: str, data: List[Dict[str, Any]]):
        """Execute insert with retry logic"""
        max_retries = 3
        retry_delay = 0.1
        
        for attempt in range(max_retries):
            try:
                with duckdb.connect(str(self.db_path)) as conn:
                    df = pd.DataFrame(data)
                    
                    if table == 'trades':
                        conn.execute("INSERT OR IGNORE INTO trades SELECT * FROM df")
                    elif table == 'ohlcv':
                        conn.execute("INSERT OR REPLACE INTO ohlcv SELECT * FROM df")
                    
                    logger.debug(f"Inserted {len(data)} records into {table}")
                    return
                    
            except Exception as e:
                if "database is locked" in str(e).lower() or "conflicting lock" in str(e).lower():
                    if attempt < max_retries - 1:
                        time.sleep(retry_delay * (2 ** attempt))  # Exponential backoff
                        continue
                
                logger.error(f"Failed to insert into {table}: {e}")
                break
    
    def get_read_connection(self) -> duckdb.DuckDBPyConnection:
        """Get a read-only connection for queries"""
        return duckdb.connect(str(self.db_path), read_only=True)
    
    def execute_query(self, query: str, params: Optional[tuple] = None):
        """Execute read query with connection management"""
        try:
            with self.get_read_connection() as conn:
                if params:
                    return conn.execute(query, params).fetchall()
                else:
                    return conn.execute(query).fetchall()
        except Exception as e:
            logger.error(f"Query execution failed: {e}")
            raise
    
    def get_trade_stats(self) -> Dict[str, Any]:
        """Get current trade statistics"""
        try:
            query = """
            SELECT 
                exchange,
                COUNT(*) as total_trades,
                COUNT(DISTINCT DATE_TRUNC('day', datetime)) as days_of_data,
                MIN(datetime) as first_trade,
                MAX(datetime) as last_trade,
                COUNT(CASE WHEN datetime >= CURRENT_TIMESTAMP - INTERVAL '1 hour' THEN 1 END) as last_hour_trades
            FROM trades
            GROUP BY exchange
            ORDER BY exchange
            """
            
            results = self.execute_query(query)
            return {row[0]: {
                'total_trades': row[1],
                'days_of_data': row[2], 
                'first_trade': row[3],
                'last_trade': row[4],
                'last_hour_trades': row[5]
            } for row in results}
            
        except Exception as e:
            logger.error(f"Failed to get trade stats: {e}")
            return {}
    
    def close(self):
        """Shutdown the manager"""
        logger.info("Shutting down database manager...")
        self.running = False
        
        # Wait for queue to empty
        self.write_queue.join()
        
        # Wait for write thread to finish
        if self.write_thread.is_alive():
            self.write_thread.join(timeout=5.0)
        
        logger.info("Database manager shutdown complete")

# Global instance
_db_manager = None
_manager_lock = threading.Lock()

def get_db_manager() -> DuckDBManager:
    """Get singleton database manager"""
    global _db_manager
    
    if _db_manager is None:
        with _manager_lock:
            if _db_manager is None:
                _db_manager = DuckDBManager()
    
    return _db_manager

def shutdown_db_manager():
    """Shutdown the global database manager"""
    global _db_manager
    
    if _db_manager is not None:
        with _manager_lock:
            if _db_manager is not None:
                _db_manager.close()
                _db_manager = None
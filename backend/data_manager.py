"""
Data Manager with Parquet + DuckDB
Handles data storage, retrieval, and analysis in a backend-agnostic format
"""

import duckdb
import pandas as pd
import numpy as np
from pathlib import Path
from datetime import datetime, timezone
from typing import List, Dict, Any, Optional, Tuple
import json
import pyarrow as pa
import pyarrow.parquet as pq

class DataManager:
    """
    Manages market data using Parquet files and DuckDB for fast queries
    """
    
    def __init__(self, data_dir: str = "market_data"):
        self.data_dir = Path(data_dir)
        self.data_dir.mkdir(exist_ok=True)
        
        # Create directory structure
        self.parquet_dir = self.data_dir / "parquet"
        self.parquet_dir.mkdir(exist_ok=True)
        
        # Initialize DuckDB connection (in-memory with persistence)
        self.db_path = self.data_dir / "market_data.duckdb"
        self.conn = duckdb.connect(str(self.db_path))
        
        # Create tables if they don't exist
        self._init_database()
    
    def _init_database(self):
        """Initialize DuckDB tables and views"""
        
        # Create main OHLCV table
        self.conn.execute("""
            CREATE TABLE IF NOT EXISTS ohlcv (
                symbol VARCHAR,
                exchange VARCHAR,
                timestamp BIGINT,
                datetime TIMESTAMP,
                open DOUBLE,
                high DOUBLE,
                low DOUBLE,
                close DOUBLE,
                volume DOUBLE,
                PRIMARY KEY (symbol, exchange, timestamp)
            )
        """)
        
        # Create metadata table
        self.conn.execute("""
            CREATE TABLE IF NOT EXISTS metadata (
                symbol VARCHAR,
                exchange VARCHAR,
                first_timestamp BIGINT,
                last_timestamp BIGINT,
                total_bars INTEGER,
                last_updated TIMESTAMP,
                PRIMARY KEY (symbol, exchange)
            )
        """)
        
        # Create a view for easy analysis
        self.conn.execute("""
            CREATE OR REPLACE VIEW ohlcv_with_returns AS
            SELECT 
                *,
                (close - LAG(close) OVER (PARTITION BY symbol ORDER BY timestamp)) / LAG(close) OVER (PARTITION BY symbol ORDER BY timestamp) as returns,
                LN(close / LAG(close) OVER (PARTITION BY symbol ORDER BY timestamp)) as log_returns
            FROM ohlcv
        """)
    
    def save_coinbase_data(self, 
                          coinbase_data: List[List[float]], 
                          symbol: str,
                          exchange: str = "coinbase") -> Dict[str, Any]:
        """
        Save Coinbase JSON data to Parquet and DuckDB
        
        Args:
            coinbase_data: List of [timestamp, low, high, open, close, volume]
            symbol: Trading pair (e.g., "BTC/USD")
            exchange: Exchange name
        
        Returns:
            Dictionary with save statistics
        """
        
        if not coinbase_data:
            return {"success": False, "error": "No data provided"}
        
        # Convert to DataFrame
        df = pd.DataFrame(coinbase_data, columns=['timestamp', 'low', 'high', 'open', 'close', 'volume'])
        
        # Add symbol and exchange
        df['symbol'] = symbol
        df['exchange'] = exchange
        
        # Convert timestamp to datetime
        df['datetime'] = pd.to_datetime(df['timestamp'], unit='s')
        
        # Reorder columns to standard OHLCV format
        df = df[['symbol', 'exchange', 'timestamp', 'datetime', 'open', 'high', 'low', 'close', 'volume']]
        
        # Sort by timestamp
        df = df.sort_values('timestamp')
        
        # Save to Parquet (partitioned by symbol and date)
        parquet_path = self._save_to_parquet(df, symbol, exchange)
        
        # Insert/update in DuckDB
        rows_affected = self._upsert_to_duckdb(df)
        
        # Update metadata
        self._update_metadata(symbol, exchange, df)
        
        return {
            "success": True,
            "symbol": symbol,
            "exchange": exchange,
            "bars_saved": len(df),
            "rows_affected": rows_affected,
            "parquet_path": str(parquet_path),
            "date_range": {
                "start": df['datetime'].min().isoformat(),
                "end": df['datetime'].max().isoformat()
            }
        }
    
    def _save_to_parquet(self, df: pd.DataFrame, symbol: str, exchange: str) -> Path:
        """Save DataFrame to Parquet file"""
        
        # Create directory structure: parquet/exchange/symbol/
        symbol_dir = self.parquet_dir / exchange / symbol.replace('/', '_')
        symbol_dir.mkdir(parents=True, exist_ok=True)
        
        # Group by date and save daily files
        df['date'] = pd.to_datetime(df['timestamp'], unit='s').dt.date
        
        paths = []
        for date, group in df.groupby('date'):
            # File naming: YYYYMMDD.parquet
            file_name = f"{date.strftime('%Y%m%d')}.parquet"
            file_path = symbol_dir / file_name
            
            # Remove date column before saving
            group = group.drop(columns=['date'])
            
            # Save with compression
            group.to_parquet(
                file_path,
                engine='pyarrow',
                compression='snappy',
                index=False
            )
            paths.append(file_path)
        
        return paths[0] if paths else symbol_dir
    
    def _upsert_to_duckdb(self, df: pd.DataFrame) -> int:
        """Insert or update data in DuckDB"""
        
        # Create temporary table from DataFrame
        self.conn.register('temp_df', df)
        
        # Upsert using INSERT OR REPLACE
        result = self.conn.execute("""
            INSERT OR REPLACE INTO ohlcv 
            SELECT symbol, exchange, timestamp, datetime, open, high, low, close, volume
            FROM temp_df
        """)
        
        # Get number of affected rows
        rows_affected = result.fetchone()[0] if result else len(df)
        
        # Unregister temporary table
        self.conn.unregister('temp_df')
        
        return rows_affected
    
    def _update_metadata(self, symbol: str, exchange: str, df: pd.DataFrame):
        """Update metadata table"""
        
        self.conn.execute("""
            INSERT OR REPLACE INTO metadata 
            VALUES (?, ?, ?, ?, ?, ?)
        """, [
            symbol,
            exchange,
            int(df['timestamp'].min()),
            int(df['timestamp'].max()),
            len(df),
            datetime.now()
        ])
    
    def query(self, query: str) -> pd.DataFrame:
        """Execute SQL query and return DataFrame"""
        return self.conn.execute(query).df()
    
    def get_ohlcv(self, 
                  symbol: str, 
                  exchange: Optional[str] = None,
                  start_time: Optional[int] = None,
                  end_time: Optional[int] = None) -> pd.DataFrame:
        """
        Get OHLCV data from DuckDB
        
        Args:
            symbol: Trading pair
            exchange: Optional exchange filter
            start_time: Optional start timestamp (Unix seconds)
            end_time: Optional end timestamp (Unix seconds)
        
        Returns:
            DataFrame with OHLCV data
        """
        
        query = "SELECT * FROM ohlcv WHERE symbol = ?"
        params = [symbol]
        
        if exchange:
            query += " AND exchange = ?"
            params.append(exchange)
        
        if start_time:
            query += " AND timestamp >= ?"
            params.append(start_time)
        
        if end_time:
            query += " AND timestamp <= ?"
            params.append(end_time)
        
        query += " ORDER BY timestamp"
        
        return self.conn.execute(query, params).df()
    
    def get_returns(self, symbol: str, exchange: Optional[str] = None) -> pd.DataFrame:
        """Get OHLCV data with returns calculated"""
        
        query = "SELECT * FROM ohlcv_with_returns WHERE symbol = ?"
        params = [symbol]
        
        if exchange:
            query += " AND exchange = ?"
            params.append(exchange)
        
        query += " ORDER BY timestamp"
        
        return self.conn.execute(query, params).df()
    
    def calculate_correlation(self, symbol1: str, symbol2: str, exchange: str = "coinbase") -> float:
        """Calculate correlation between two symbols"""
        
        result = self.conn.execute("""
            WITH aligned_data AS (
                SELECT 
                    a.timestamp,
                    a.log_returns as returns1,
                    b.log_returns as returns2
                FROM ohlcv_with_returns a
                JOIN ohlcv_with_returns b ON a.timestamp = b.timestamp
                WHERE a.symbol = ? AND a.exchange = ?
                AND b.symbol = ? AND b.exchange = ?
                AND a.log_returns IS NOT NULL 
                AND b.log_returns IS NOT NULL
            )
            SELECT CORR(returns1, returns2) as correlation
            FROM aligned_data
        """, [symbol1, exchange, symbol2, exchange]).fetchone()
        
        return result[0] if result else None
    
    def calculate_statistics(self, symbol: str, exchange: str = "coinbase") -> Dict[str, float]:
        """Calculate basic statistics for a symbol"""
        
        result = self.conn.execute("""
            WITH stats AS (
                SELECT 
                    log_returns
                FROM ohlcv_with_returns
                WHERE symbol = ? AND exchange = ?
                AND log_returns IS NOT NULL
            )
            SELECT 
                AVG(log_returns) as mean_return,
                STDDEV(log_returns) as volatility,
                SKEWNESS(log_returns) as skewness,
                KURTOSIS(log_returns) as kurtosis,
                MIN(log_returns) as min_return,
                MAX(log_returns) as max_return,
                COUNT(*) as total_bars
            FROM stats
        """, [symbol, exchange]).fetchone()
        
        if result:
            return {
                "mean_return": result[0],
                "volatility": result[1],
                "skewness": result[2],
                "kurtosis": result[3],
                "min_return": result[4],
                "max_return": result[5],
                "total_bars": result[6],
                "annualized_volatility": result[1] * np.sqrt(365 * 24 * 60) if result[1] else None,
                "sharpe_ratio": (result[0] / result[1]) * np.sqrt(365 * 24 * 60) if result[1] else None
            }
        return {}
    
    def export_to_csv(self, symbol: str, exchange: str = "coinbase", output_path: Optional[str] = None) -> Path:
        """Export data to CSV"""
        
        df = self.get_returns(symbol, exchange)
        
        if output_path is None:
            output_path = self.data_dir / f"{symbol.replace('/', '_')}_{exchange}_{datetime.now().strftime('%Y%m%d_%H%M%S')}.csv"
        
        df.to_csv(output_path, index=False)
        return Path(output_path)
    
    def get_metadata(self) -> pd.DataFrame:
        """Get all metadata"""
        return self.conn.execute("SELECT * FROM metadata ORDER BY symbol, exchange").df()
    
    def get_summary(self) -> Dict[str, Any]:
        """Get summary of all stored data"""
        
        total_bars = self.conn.execute("SELECT COUNT(*) FROM ohlcv").fetchone()[0]
        symbols = self.conn.execute("SELECT DISTINCT symbol, exchange FROM ohlcv").df()
        
        summary = {
            "total_bars": total_bars,
            "symbols": []
        }
        
        for _, row in symbols.iterrows():
            symbol_info = self.conn.execute("""
                SELECT 
                    COUNT(*) as bar_count,
                    MIN(datetime) as first_bar,
                    MAX(datetime) as last_bar
                FROM ohlcv
                WHERE symbol = ? AND exchange = ?
            """, [row['symbol'], row['exchange']]).fetchone()
            
            summary["symbols"].append({
                "symbol": row['symbol'],
                "exchange": row['exchange'],
                "bar_count": symbol_info[0],
                "first_bar": symbol_info[1],
                "last_bar": symbol_info[2]
            })
        
        return summary
    
    def get_data_summary(self) -> Dict[str, Any]:
        """Alias for get_summary() for API compatibility"""
        return self.get_summary()
    
    def list_available_data(self) -> Dict[str, Any]:
        """List all available data in the catalog"""
        try:
            # Get list of parquet files
            parquet_files = []
            for exchange_dir in self.parquet_dir.iterdir():
                if exchange_dir.is_dir():
                    for symbol_dir in exchange_dir.iterdir():
                        if symbol_dir.is_dir():
                            files = list(symbol_dir.glob("*.parquet"))
                            parquet_files.extend([{
                                'exchange': exchange_dir.name,
                                'symbol': symbol_dir.name,
                                'file': f.name,
                                'size_mb': f.stat().st_size / (1024 * 1024)
                            } for f in files])
            
            # Get database summary
            db_summary = self.get_summary()
            
            return {
                'parquet_files': parquet_files,
                'database': db_summary,
                'total_files': len(parquet_files)
            }
        except Exception as e:
            return {
                'error': str(e),
                'parquet_files': [],
                'database': {'total_bars': 0, 'symbols': []},
                'total_files': 0
            }
    
    def close(self):
        """Close database connection"""
        if self.conn:
            self.conn.close()


# Example usage
if __name__ == "__main__":
    # Initialize manager
    dm = DataManager()
    
    # Example Coinbase data
    sample_data = [
        [1754867100, 119100.3, 119207, 119207, 119123.27, 11.94899356],
        [1754867040, 119180.64, 119218.75, 119199.99, 119211.25, 3.80621851],
    ]
    
    # Save data
    result = dm.save_coinbase_data(sample_data, "BTC/USD")
    print(f"Save result: {json.dumps(result, indent=2)}")
    
    # Query data
    df = dm.get_ohlcv("BTC/USD")
    print(f"\nQueried {len(df)} bars")
    
    # Calculate statistics
    stats = dm.calculate_statistics("BTC/USD")
    print(f"\nStatistics: {json.dumps(stats, indent=2)}")
    
    # Get summary
    summary = dm.get_summary()
    print(f"\nSummary: {json.dumps(summary, indent=2, default=str)}")
    
    dm.close()
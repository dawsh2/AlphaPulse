"""
Pandas integration for AlphaPulse Python bindings.

Provides seamless conversion between AlphaPulse data types and pandas DataFrames
for efficient data analysis and research.
"""

import time
from typing import List, Dict, Any, Optional, Union
import logging

try:
    import pandas as pd
    import numpy as np
    HAS_PANDAS = True
except ImportError:
    HAS_PANDAS = False

from . import PyTrade, PyOrderBookDelta, PyOrderBook, PyPriceLevel

logger = logging.getLogger(__name__)

def to_pandas(data: Union[List[PyTrade], List[PyOrderBookDelta], List[Dict[str, Any]]]) -> pd.DataFrame:
    """
    Convert AlphaPulse data types to pandas DataFrame.
    
    Args:
        data: List of PyTrade, PyOrderBookDelta objects, or dictionaries
        
    Returns:
        pandas DataFrame with appropriate columns and types
        
    Raises:
        ImportError: If pandas is not installed
        ValueError: If data type is not supported
    """
    if not HAS_PANDAS:
        raise ImportError("pandas is required for this function. Install with: pip install pandas")
    
    if not data:
        return pd.DataFrame()
    
    # Determine data type and convert accordingly
    first_item = data[0]
    
    if isinstance(first_item, PyTrade):
        return _trades_to_pandas(data)
    elif isinstance(first_item, PyOrderBookDelta):
        return _deltas_to_pandas(data)
    elif isinstance(first_item, dict):
        return pd.DataFrame(data)
    else:
        raise ValueError(f"Unsupported data type: {type(first_item)}")

def _trades_to_pandas(trades: List[PyTrade]) -> pd.DataFrame:
    """Convert trade data to pandas DataFrame"""
    records = []
    
    for trade in trades:
        records.append({
            'timestamp': pd.to_datetime(trade.timestamp, unit='s'),
            'symbol': trade.symbol,
            'exchange': trade.exchange,
            'price': trade.price,
            'volume': trade.volume,
            'side': trade.side,
            'trade_id': trade.trade_id,
        })
    
    df = pd.DataFrame(records)
    
    # Set appropriate data types
    df['timestamp'] = pd.to_datetime(df['timestamp'])
    df['price'] = pd.to_numeric(df['price'], errors='coerce')
    df['volume'] = pd.to_numeric(df['volume'], errors='coerce')
    
    # Set timestamp as index for time series analysis
    df.set_index('timestamp', inplace=True)
    
    return df

def _deltas_to_pandas(deltas: List[PyOrderBookDelta]) -> pd.DataFrame:
    """Convert orderbook delta data to pandas DataFrame"""
    records = []
    
    for delta in deltas:
        base_record = {
            'timestamp': pd.to_datetime(delta.timestamp, unit='s'),
            'symbol': delta.symbol,
            'exchange': delta.exchange,
            'version': delta.version,
            'prev_version': delta.prev_version,
        }
        
        # Add bid changes
        for i, change in enumerate(delta.bid_changes):
            record = base_record.copy()
            record.update({
                'side': 'bid',
                'price': change.price,
                'volume': change.volume,
                'action': change.action,
                'change_index': i,
            })
            records.append(record)
        
        # Add ask changes
        for i, change in enumerate(delta.ask_changes):
            record = base_record.copy()
            record.update({
                'side': 'ask',
                'price': change.price,
                'volume': change.volume,
                'action': change.action,
                'change_index': i,
            })
            records.append(record)
    
    df = pd.DataFrame(records)
    
    if not df.empty:
        # Set appropriate data types
        df['timestamp'] = pd.to_datetime(df['timestamp'])
        df['price'] = pd.to_numeric(df['price'], errors='coerce')
        df['volume'] = pd.to_numeric(df['volume'], errors='coerce')
        df['version'] = pd.to_numeric(df['version'], errors='coerce')
        df['prev_version'] = pd.to_numeric(df['prev_version'], errors='coerce')
        
        # Set timestamp as index
        df.set_index('timestamp', inplace=True)
    
    return df

def from_pandas(df: pd.DataFrame, data_type: str = 'trade') -> List[Union[PyTrade, Dict[str, Any]]]:
    """
    Convert pandas DataFrame back to AlphaPulse data types.
    
    Args:
        df: pandas DataFrame with appropriate columns
        data_type: Type of data ('trade' or 'dict')
        
    Returns:
        List of PyTrade objects or dictionaries
    """
    if not HAS_PANDAS:
        raise ImportError("pandas is required for this function. Install with: pip install pandas")
    
    if data_type == 'trade':
        return _pandas_to_trades(df)
    elif data_type == 'dict':
        return df.to_dict('records')
    else:
        raise ValueError(f"Unsupported data_type: {data_type}")

def _pandas_to_trades(df: pd.DataFrame) -> List[PyTrade]:
    """Convert pandas DataFrame to PyTrade objects"""
    trades = []
    
    # Ensure timestamp is in the right format
    if 'timestamp' in df.columns:
        timestamp_col = 'timestamp'
    elif df.index.name == 'timestamp':
        df = df.reset_index()
        timestamp_col = 'timestamp'
    else:
        raise ValueError("DataFrame must have a 'timestamp' column or index")
    
    for _, row in df.iterrows():
        # Convert pandas Timestamp to Unix timestamp
        if isinstance(row[timestamp_col], pd.Timestamp):
            timestamp = row[timestamp_col].timestamp()
        else:
            timestamp = float(row[timestamp_col])
        
        trade = PyTrade(
            timestamp=timestamp,
            symbol=str(row.get('symbol', '')),
            exchange=str(row.get('exchange', '')),
            price=float(row.get('price', 0.0)),
            volume=float(row.get('volume', 0.0)),
            side=row.get('side'),
            trade_id=row.get('trade_id'),
        )
        trades.append(trade)
    
    return trades

def analyze_trades(trades_df: pd.DataFrame) -> Dict[str, Any]:
    """
    Perform comprehensive analysis on trade data.
    
    Args:
        trades_df: DataFrame with trade data
        
    Returns:
        Dictionary with analysis results
    """
    if not HAS_PANDAS:
        raise ImportError("pandas is required for this function")
    
    if trades_df.empty:
        return {}
    
    analysis = {}
    
    # Basic statistics
    analysis['total_trades'] = len(trades_df)
    analysis['unique_symbols'] = trades_df['symbol'].nunique()
    analysis['unique_exchanges'] = trades_df['exchange'].nunique()
    
    # Price statistics
    analysis['price_stats'] = {
        'mean': trades_df['price'].mean(),
        'median': trades_df['price'].median(),
        'std': trades_df['price'].std(),
        'min': trades_df['price'].min(),
        'max': trades_df['price'].max(),
    }
    
    # Volume statistics
    analysis['volume_stats'] = {
        'total': trades_df['volume'].sum(),
        'mean': trades_df['volume'].mean(),
        'median': trades_df['volume'].median(),
        'std': trades_df['volume'].std(),
    }
    
    # Time-based analysis
    if isinstance(trades_df.index, pd.DatetimeIndex):
        time_span = trades_df.index.max() - trades_df.index.min()
        analysis['time_span_seconds'] = time_span.total_seconds()
        analysis['trades_per_second'] = len(trades_df) / time_span.total_seconds() if time_span.total_seconds() > 0 else 0
    
    # Exchange and symbol breakdown
    analysis['by_exchange'] = trades_df.groupby('exchange').agg({
        'price': ['count', 'mean'],
        'volume': 'sum'
    }).to_dict()
    
    analysis['by_symbol'] = trades_df.groupby('symbol').agg({
        'price': ['count', 'mean'],
        'volume': 'sum'
    }).to_dict()
    
    return analysis

def calculate_ohlcv(trades_df: pd.DataFrame, timeframe: str = '1T') -> pd.DataFrame:
    """
    Calculate OHLCV bars from trade data.
    
    Args:
        trades_df: DataFrame with trade data (must have timestamp index)
        timeframe: Pandas frequency string (e.g., '1T' for 1 minute, '5T' for 5 minutes)
        
    Returns:
        DataFrame with OHLCV data
    """
    if not HAS_PANDAS:
        raise ImportError("pandas is required for this function")
    
    if trades_df.empty:
        return pd.DataFrame()
    
    # Ensure we have a datetime index
    if not isinstance(trades_df.index, pd.DatetimeIndex):
        raise ValueError("DataFrame must have a DatetimeIndex")
    
    # Group by timeframe and calculate OHLCV
    ohlcv = trades_df.groupby([
        pd.Grouper(freq=timeframe),
        'symbol',
        'exchange'
    ]).agg({
        'price': ['first', 'max', 'min', 'last', 'count'],
        'volume': 'sum'
    }).round(8)
    
    # Flatten column names
    ohlcv.columns = ['open', 'high', 'low', 'close', 'trades', 'volume']
    
    # Reset index to make symbol and exchange regular columns
    ohlcv = ohlcv.reset_index()
    
    # Remove periods with no trades
    ohlcv = ohlcv.dropna()
    
    return ohlcv

def calculate_vwap(trades_df: pd.DataFrame, timeframe: str = '1T') -> pd.DataFrame:
    """
    Calculate Volume Weighted Average Price (VWAP).
    
    Args:
        trades_df: DataFrame with trade data
        timeframe: Pandas frequency string
        
    Returns:
        DataFrame with VWAP data
    """
    if not HAS_PANDAS:
        raise ImportError("pandas is required for this function")
    
    if trades_df.empty:
        return pd.DataFrame()
    
    # Calculate value (price * volume) for VWAP
    trades_df = trades_df.copy()
    trades_df['value'] = trades_df['price'] * trades_df['volume']
    
    # Group by timeframe and calculate VWAP
    vwap = trades_df.groupby([
        pd.Grouper(freq=timeframe),
        'symbol', 
        'exchange'
    ]).agg({
        'value': 'sum',
        'volume': 'sum',
        'price': ['count', 'mean']
    })
    
    # Calculate VWAP
    vwap['vwap'] = vwap[('value', 'sum')] / vwap[('volume', 'sum')]
    
    # Clean up columns
    vwap = vwap.reset_index()
    vwap.columns = ['timestamp', 'symbol', 'exchange', 'total_value', 'total_volume', 'trade_count', 'avg_price', 'vwap']
    
    return vwap[['timestamp', 'symbol', 'exchange', 'vwap', 'total_volume', 'trade_count']]

def detect_price_anomalies(trades_df: pd.DataFrame, std_threshold: float = 3.0) -> pd.DataFrame:
    """
    Detect price anomalies using statistical methods.
    
    Args:
        trades_df: DataFrame with trade data
        std_threshold: Number of standard deviations to consider anomalous
        
    Returns:
        DataFrame with anomalous trades
    """
    if not HAS_PANDAS:
        raise ImportError("pandas is required for this function")
    
    if trades_df.empty:
        return pd.DataFrame()
    
    anomalies = []
    
    # Detect anomalies by symbol and exchange
    for (symbol, exchange), group in trades_df.groupby(['symbol', 'exchange']):
        if len(group) < 10:  # Need enough data for meaningful statistics
            continue
            
        mean_price = group['price'].mean()
        std_price = group['price'].std()
        
        # Calculate z-scores
        group = group.copy()
        group['z_score'] = np.abs((group['price'] - mean_price) / std_price)
        
        # Find anomalies
        anomalous = group[group['z_score'] > std_threshold].copy()
        anomalous['anomaly_type'] = 'price_outlier'
        anomalous['expected_price'] = mean_price
        anomalous['price_deviation'] = anomalous['price'] - mean_price
        
        anomalies.append(anomalous)
    
    if anomalies:
        return pd.concat(anomalies, ignore_index=True)
    else:
        return pd.DataFrame()

def calculate_spread_statistics(orderbook_data: List[PyOrderBook]) -> pd.DataFrame:
    """
    Calculate spread statistics from orderbook data.
    
    Args:
        orderbook_data: List of PyOrderBook objects
        
    Returns:
        DataFrame with spread statistics
    """
    if not HAS_PANDAS:
        raise ImportError("pandas is required for this function")
    
    if not orderbook_data:
        return pd.DataFrame()
    
    records = []
    
    for book in orderbook_data:
        best_bid = book.get_best_bid()
        best_ask = book.get_best_ask()
        spread = book.get_spread()
        
        if best_bid and best_ask and spread:
            spread_bps = (spread / best_ask) * 10000  # Convert to basis points
            
            records.append({
                'timestamp': pd.to_datetime(book.timestamp, unit='s'),
                'symbol': book.symbol,
                'exchange': book.exchange,
                'best_bid': best_bid,
                'best_ask': best_ask,
                'spread': spread,
                'spread_bps': spread_bps,
                'mid_price': (best_bid + best_ask) / 2,
            })
    
    df = pd.DataFrame(records)
    
    if not df.empty:
        df.set_index('timestamp', inplace=True)
    
    return df

def resample_data(df: pd.DataFrame, timeframe: str, agg_funcs: Optional[Dict[str, str]] = None) -> pd.DataFrame:
    """
    Resample data to different timeframes.
    
    Args:
        df: DataFrame with datetime index
        timeframe: Target timeframe (e.g., '1S', '1T', '1H')
        agg_funcs: Dictionary mapping column names to aggregation functions
        
    Returns:
        Resampled DataFrame
    """
    if not HAS_PANDAS:
        raise ImportError("pandas is required for this function")
    
    if df.empty or not isinstance(df.index, pd.DatetimeIndex):
        return df
    
    # Default aggregation functions
    if agg_funcs is None:
        agg_funcs = {
            'price': 'last',
            'volume': 'sum',
            'spread': 'mean',
            'spread_bps': 'mean',
        }
    
    # Apply resampling with specified aggregation functions
    resampled = df.resample(timeframe).agg({
        col: func for col, func in agg_funcs.items() if col in df.columns
    })
    
    # Drop periods with no data
    resampled = resampled.dropna(how='all')
    
    return resampled

# Convenience functions for common operations

def quick_analysis(trades: List[PyTrade]) -> Dict[str, Any]:
    """Quick analysis of trade data"""
    if not trades:
        return {}
    
    df = to_pandas(trades)
    return analyze_trades(df)

def quick_ohlcv(trades: List[PyTrade], timeframe: str = '1T') -> pd.DataFrame:
    """Quick OHLCV calculation from trades"""
    if not trades:
        return pd.DataFrame()
    
    df = to_pandas(trades)
    return calculate_ohlcv(df, timeframe)

def quick_vwap(trades: List[PyTrade], timeframe: str = '1T') -> pd.DataFrame:
    """Quick VWAP calculation from trades"""
    if not trades:
        return pd.DataFrame()
    
    df = to_pandas(trades)
    return calculate_vwap(df, timeframe)
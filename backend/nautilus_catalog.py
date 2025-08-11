"""
NautilusTrader Data Catalog Integration
Converts Coinbase data to NautilusTrader format and manages the data catalog
"""

import pandas as pd
import numpy as np
from pathlib import Path
from datetime import datetime, timezone
import pyarrow as pa
import pyarrow.parquet as pq
from typing import List, Dict, Any, Optional
import json

class NautilusCatalog:
    """
    Manages market data in NautilusTrader catalog format
    """
    
    def __init__(self, catalog_path: str = "catalog"):
        self.catalog_path = Path(catalog_path)
        self.catalog_path.mkdir(exist_ok=True)
        
        # Create standard catalog structure
        (self.catalog_path / "data").mkdir(exist_ok=True)
        (self.catalog_path / "metadata").mkdir(exist_ok=True)
        
    def coinbase_to_nautilus_bar(self, 
                                  coinbase_data: List[List[float]], 
                                  symbol: str,
                                  exchange: str = "COINBASE") -> pd.DataFrame:
        """
        Convert Coinbase JSON array format to NautilusTrader Bar format
        
        Coinbase format: [timestamp, low, high, open, close, volume]
        NautilusTrader Bar format: DataFrame with specific columns
        """
        
        # Create DataFrame from Coinbase data
        df = pd.DataFrame(coinbase_data, columns=['timestamp', 'low', 'high', 'open', 'close', 'volume'])
        
        # Convert timestamp to nanoseconds (NautilusTrader uses ns precision)
        df['ts_event'] = pd.to_datetime(df['timestamp'], unit='s').astype(np.int64)
        df['ts_init'] = df['ts_event']  # Same as event time for historical data
        
        # Create bar_type identifier
        instrument_id = f"{symbol.replace('/', '-')}.{exchange}"
        bar_type = f"{instrument_id}-1-MINUTE-LAST-EXTERNAL"
        
        # Build NautilusTrader Bar DataFrame
        nautilus_df = pd.DataFrame({
            'bar_type': bar_type,
            'instrument_id': instrument_id,
            'open': df['open'].astype(np.float64),
            'high': df['high'].astype(np.float64),
            'low': df['low'].astype(np.float64),
            'close': df['close'].astype(np.float64),
            'volume': df['volume'].astype(np.float64),
            'ts_event': df['ts_event'],
            'ts_init': df['ts_init']
        })
        
        # Sort by timestamp
        nautilus_df = nautilus_df.sort_values('ts_event')
        
        return nautilus_df
    
    def save_bars_to_catalog(self, 
                            bars_df: pd.DataFrame, 
                            symbol: str,
                            date: Optional[str] = None) -> Path:
        """
        Save bars to catalog in Parquet format
        
        Catalog structure:
        catalog/
          data/
            bars/
              BTC-USD/
                2025-08-10.parquet
        """
        
        # Create bars directory structure
        bars_path = self.catalog_path / "data" / "bars" / symbol.replace('/', '-')
        bars_path.mkdir(parents=True, exist_ok=True)
        
        # Use provided date or extract from data
        if date is None:
            date = pd.to_datetime(bars_df['ts_event'].iloc[0], unit='ns').strftime('%Y-%m-%d')
        
        # File path
        file_path = bars_path / f"{date}.parquet"
        
        # Save to Parquet with compression
        bars_df.to_parquet(
            file_path,
            engine='pyarrow',
            compression='snappy',
            index=False
        )
        
        # Update metadata
        self._update_metadata(symbol, date, len(bars_df))
        
        return file_path
    
    def load_bars_from_catalog(self, 
                              symbol: str, 
                              start_date: Optional[str] = None,
                              end_date: Optional[str] = None) -> pd.DataFrame:
        """
        Load bars from catalog
        """
        bars_path = self.catalog_path / "data" / "bars" / symbol.replace('/', '-')
        
        if not bars_path.exists():
            return pd.DataFrame()
        
        # Get all parquet files
        parquet_files = sorted(bars_path.glob("*.parquet"))
        
        # Filter by date range if provided
        if start_date or end_date:
            filtered_files = []
            for file in parquet_files:
                file_date = file.stem  # e.g., "2025-08-10"
                if start_date and file_date < start_date:
                    continue
                if end_date and file_date > end_date:
                    continue
                filtered_files.append(file)
            parquet_files = filtered_files
        
        # Load and concatenate all files
        dfs = []
        for file in parquet_files:
            df = pd.read_parquet(file)
            dfs.append(df)
        
        if dfs:
            return pd.concat(dfs, ignore_index=True).sort_values('ts_event')
        return pd.DataFrame()
    
    def _update_metadata(self, symbol: str, date: str, bar_count: int):
        """
        Update catalog metadata
        """
        metadata_file = self.catalog_path / "metadata" / "catalog.json"
        
        # Load existing metadata
        if metadata_file.exists():
            with open(metadata_file, 'r') as f:
                metadata = json.load(f)
        else:
            metadata = {"symbols": {}}
        
        # Update symbol metadata
        symbol_key = symbol.replace('/', '-')
        if symbol_key not in metadata["symbols"]:
            metadata["symbols"][symbol_key] = {
                "dates": {},
                "total_bars": 0,
                "first_date": date,
                "last_date": date
            }
        
        symbol_meta = metadata["symbols"][symbol_key]
        symbol_meta["dates"][date] = bar_count
        symbol_meta["total_bars"] = sum(symbol_meta["dates"].values())
        symbol_meta["first_date"] = min(symbol_meta["dates"].keys())
        symbol_meta["last_date"] = max(symbol_meta["dates"].keys())
        
        # Save metadata
        metadata_file.parent.mkdir(exist_ok=True)
        with open(metadata_file, 'w') as f:
            json.dump(metadata, f, indent=2)
    
    def get_catalog_info(self) -> Dict[str, Any]:
        """
        Get catalog information
        """
        metadata_file = self.catalog_path / "metadata" / "catalog.json"
        
        if metadata_file.exists():
            with open(metadata_file, 'r') as f:
                return json.load(f)
        return {"symbols": {}}
    
    def calculate_returns(self, bars_df: pd.DataFrame) -> pd.DataFrame:
        """
        Calculate returns and add statistical columns
        """
        df = bars_df.copy()
        
        # Calculate returns
        df['returns'] = df['close'].pct_change()
        df['log_returns'] = np.log(df['close'] / df['close'].shift(1))
        
        # Calculate rolling statistics
        df['volatility_20'] = df['returns'].rolling(20).std()
        df['volatility_60'] = df['returns'].rolling(60).std()
        
        # Volume-weighted average price (VWAP)
        df['vwap'] = (df['volume'] * (df['high'] + df['low'] + df['close']) / 3).cumsum() / df['volume'].cumsum()
        
        return df
    
    def export_for_analysis(self, symbol: str, format: str = 'parquet') -> Path:
        """
        Export data in analysis-ready format
        """
        # Load all data for symbol
        df = self.load_bars_from_catalog(symbol)
        
        if df.empty:
            raise ValueError(f"No data found for {symbol}")
        
        # Add analysis columns
        df = self.calculate_returns(df)
        
        # Convert timestamps to datetime for readability
        df['datetime'] = pd.to_datetime(df['ts_event'], unit='ns')
        
        # Export path
        export_path = self.catalog_path / "exports"
        export_path.mkdir(exist_ok=True)
        
        timestamp = datetime.now().strftime('%Y%m%d_%H%M%S')
        
        if format == 'parquet':
            file_path = export_path / f"{symbol.replace('/', '-')}_{timestamp}.parquet"
            df.to_parquet(file_path, index=False)
        elif format == 'csv':
            file_path = export_path / f"{symbol.replace('/', '-')}_{timestamp}.csv"
            df.to_csv(file_path, index=False)
        else:
            raise ValueError(f"Unsupported format: {format}")
        
        return file_path


# Example usage
if __name__ == "__main__":
    catalog = NautilusCatalog()
    
    # Example: Process Coinbase data
    sample_coinbase_data = [
        [1754867100, 119100.3, 119207, 119207, 119123.27, 11.94899356],
        [1754867040, 119180.64, 119218.75, 119199.99, 119211.25, 3.80621851],
    ]
    
    # Convert to NautilusTrader format
    bars_df = catalog.coinbase_to_nautilus_bar(sample_coinbase_data, "BTC/USD")
    
    # Save to catalog
    file_path = catalog.save_bars_to_catalog(bars_df, "BTC/USD")
    print(f"Saved to: {file_path}")
    
    # Get catalog info
    info = catalog.get_catalog_info()
    print(f"Catalog info: {json.dumps(info, indent=2)}")
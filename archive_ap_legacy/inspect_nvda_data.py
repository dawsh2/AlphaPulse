#!/usr/bin/env python3
"""
Inspect the NVDA data to understand the single-price bar issue.
"""

from pathlib import Path
import pandas as pd
from nautilus_trader.persistence.catalog import ParquetDataCatalog
from nautilus_trader.model.data import Bar

def main():
    # Load data
    catalog_path = Path.cwd() / "catalog"
    catalog = ParquetDataCatalog(catalog_path)
    
    # Query data
    bar_type_str = "NVDA.ALPACA-1-MINUTE-LAST-EXTERNAL"
    bars = catalog.query(
        data_cls=Bar,
        identifiers=[bar_type_str],
    )
    
    print(f"Total bars: {len(bars)}")
    
    # Analyze the bars
    single_price_count = 0
    examples = []
    
    for i, bar in enumerate(bars):
        if bar.is_single_price():
            single_price_count += 1
            if len(examples) < 10:  # Collect first 10 examples
                examples.append((i, bar))
    
    print(f"\nSingle-price bars: {single_price_count} ({single_price_count/len(bars)*100:.1f}%)")
    
    # Show examples
    print("\nFirst 10 single-price bars:")
    for idx, bar in examples:
        print(f"  Bar #{idx}: {bar}")
    
    # Check if it's a data pattern
    print("\nAnalyzing bar price ranges...")
    
    # Convert to dataframe for easier analysis
    data = []
    for bar in bars[:100]:  # First 100 bars
        data.append({
            'open': float(bar.open),
            'high': float(bar.high),
            'low': float(bar.low),
            'close': float(bar.close),
            'volume': int(bar.volume),
            'is_single': bar.is_single_price(),
            'range': float(bar.high - bar.low)
        })
    
    df = pd.DataFrame(data)
    
    print("\nFirst 20 bars:")
    print(df.head(20))
    
    print("\nPrice statistics:")
    print(f"Average range (high-low): ${df['range'].mean():.4f}")
    print(f"Max range: ${df['range'].max():.4f}")
    print(f"Min range: ${df['range'].min():.4f}")
    
    # Check actual bar data structure
    print("\nRaw bar inspection (first non-single-price bar):")
    for bar in bars[:50]:
        if not bar.is_single_price():
            print(f"  Open: {bar.open} (type: {type(bar.open)})")
            print(f"  High: {bar.high} (type: {type(bar.high)})")
            print(f"  Low: {bar.low} (type: {type(bar.low)})")
            print(f"  Close: {bar.close} (type: {type(bar.close)})")
            print(f"  Volume: {bar.volume}")
            print(f"  is_single_price(): {bar.is_single_price()}")
            break
    
    # Let's also check the raw parquet data
    print("\nChecking raw parquet data...")
    import pyarrow.parquet as pq
    
    # Find the actual parquet file
    parquet_files = list((catalog_path / "data" / "bar").glob("*.parquet"))
    if parquet_files:
        print(f"\nFound parquet file: {parquet_files[0]}")
        table = pq.read_table(parquet_files[0])
        df_raw = table.to_pandas()
        print("\nRaw parquet columns:", df_raw.columns.tolist())
        print("\nFirst 5 rows of raw data:")
        print(df_raw.head())
        
        # Check for data issues
        print(f"\nUnique open prices in first 100 rows: {df_raw['open'][:100].nunique()}")
        print(f"Unique high prices in first 100 rows: {df_raw['high'][:100].nunique()}")
        print(f"Unique low prices in first 100 rows: {df_raw['low'][:100].nunique()}")
        print(f"Unique close prices in first 100 rows: {df_raw['close'][:100].nunique()}")


if __name__ == "__main__":
    main()
#!/usr/bin/env python3
"""
Analyze when and why single-price bars occur in our NVDA data.
"""

from pathlib import Path
from datetime import datetime
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
    
    # Analyze single-price bars by time of day
    single_price_bars = []
    
    for bar in bars:
        if bar.is_single_price():
            timestamp = datetime.fromtimestamp(bar.ts_event / 1e9)
            single_price_bars.append({
                'timestamp': timestamp,
                'hour': timestamp.hour,
                'minute': timestamp.minute,
                'price': float(bar.close),
                'volume': int(bar.volume),
                'date': timestamp.date(),
                'time_str': timestamp.strftime('%Y-%m-%d %H:%M')
            })
    
    df = pd.DataFrame(single_price_bars)
    
    if len(df) > 0:
        print(f"\nSingle-price bars: {len(df)} ({len(df)/len(bars)*100:.1f}%)")
        
        # Analyze by hour
        print("\nSingle-price bars by hour of day:")
        hour_counts = df.groupby('hour').size().sort_index()
        for hour, count in hour_counts.items():
            print(f"  {hour:02d}:00 - {count} bars")
        
        # Check if they're mostly pre-market or after-hours
        regular_hours = df[(df['hour'] >= 9) & (df['hour'] < 16)]
        pre_market = df[(df['hour'] >= 4) & (df['hour'] < 9)]
        after_hours = df[(df['hour'] >= 16) | (df['hour'] < 4)]
        
        print(f"\nSingle-price bars by market session:")
        print(f"  Pre-market (4:00-9:30):  {len(pre_market)} bars")
        print(f"  Regular (9:30-16:00):    {len(regular_hours)} bars")
        print(f"  After-hours (16:00-4:00): {len(after_hours)} bars")
        
        # Show volume distribution
        print(f"\nVolume statistics for single-price bars:")
        print(f"  Mean volume: {df['volume'].mean():.0f}")
        print(f"  Median volume: {df['volume'].median():.0f}")
        print(f"  Min volume: {df['volume'].min()}")
        print(f"  Max volume: {df['volume'].max()}")
        
        # Show some examples
        print(f"\nFirst 10 single-price bars:")
        for _, row in df.head(10).iterrows():
            print(f"  {row['time_str']} - Price: ${row['price']:.2f}, Volume: {row['volume']:,}")
        
        # Check if these are actually valid market conditions
        print(f"\nAnalyzing market context...")
        
        # Look at bars around single-price bars
        examples_analyzed = 0
        for i, bar in enumerate(bars):
            if bar.is_single_price() and examples_analyzed < 3:
                print(f"\nExample {examples_analyzed + 1}: Bar at {datetime.fromtimestamp(bar.ts_event / 1e9).strftime('%H:%M')}")
                
                # Show previous bar
                if i > 0:
                    prev_bar = bars[i-1]
                    print(f"  Previous bar: O={prev_bar.open}, H={prev_bar.high}, L={prev_bar.low}, C={prev_bar.close}, V={prev_bar.volume}")
                
                # Show current bar
                print(f"  Current bar:  O={bar.open}, H={bar.high}, L={bar.low}, C={bar.close}, V={bar.volume}")
                
                # Show next bar
                if i < len(bars) - 1:
                    next_bar = bars[i+1]
                    print(f"  Next bar:     O={next_bar.open}, H={next_bar.high}, L={next_bar.low}, C={next_bar.close}, V={next_bar.volume}")
                
                examples_analyzed += 1
    else:
        print("No single-price bars found!")


if __name__ == "__main__":
    main()
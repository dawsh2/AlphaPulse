#!/usr/bin/env python3
"""
Explore the ParquetDataCatalog to see what data we have.
"""

from pathlib import Path
from datetime import datetime
from nautilus_trader.persistence.catalog import ParquetDataCatalog
from nautilus_trader.model.data import Bar

# Set up catalog
catalog_path = Path.cwd() / "catalog"
catalog = ParquetDataCatalog(catalog_path)

print("📊 Catalog Overview")
print(f"Path: {catalog_path}")
print(f"Exists: {catalog_path.exists()}")

# List instruments
print("\n🎯 Instruments in Catalog:")
instruments = list(catalog.instruments())
for inst in instruments:
    print(f"  - {inst}")

# Try to query bars without date filter
print("\n🔍 Querying all bars (no date filter):")
try:
    all_bars = catalog.query(
        data_cls=Bar,
        identifiers=["NVDA.ALPACA-1-MINUTE-LAST-EXTERNAL"],
    )
    print(f"Found {len(all_bars)} bars total")
    
    if all_bars:
        print(f"\nFirst 3 bars:")
        for i, bar in enumerate(all_bars[:3]):
            ts = datetime.fromtimestamp(bar.ts_event / 1e9)
            print(f"  {i+1}. {ts}: O={bar.open} H={bar.high} L={bar.low} C={bar.close} V={bar.volume}")
        
        print(f"\nLast 3 bars:")
        for i, bar in enumerate(all_bars[-3:]):
            ts = datetime.fromtimestamp(bar.ts_event / 1e9)
            print(f"  {i+1}. {ts}: O={bar.open} H={bar.high} L={bar.low} C={bar.close} V={bar.volume}")
        
except Exception as e:
    print(f"Error querying: {e}")

# Show actual file contents summary
print("\n📄 Parquet Files:")
data_path = catalog_path / "data"
if data_path.exists():
    for file in data_path.rglob("*.parquet"):
        size_kb = file.stat().st_size / 1024
        print(f"  - {file.relative_to(catalog_path)} ({size_kb:.1f} KB)")
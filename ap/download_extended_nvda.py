#!/usr/bin/env python3
"""
Download extended NVDA data using our existing Alpaca adapter.
"""

import os
import asyncio
from datetime import datetime, timedelta
from pathlib import Path

# Setup path for our adapter
import sys
sys.path.append(str(Path(__file__).parent))

from nautilus_adapters.alpaca.config import AlpacaDataClientConfig
from nautilus_adapters.alpaca.data import AlpacaDataClient
from nautilus_trader.persistence.catalog import ParquetDataCatalog
from nautilus_trader.model.identifiers import Symbol, Venue
from nautilus_trader.model.data import Bar


async def main():
    """Download extended NVDA data."""
    
    # Configuration
    config = AlpacaDataClientConfig(
        api_key=os.getenv("ALPACA_API_KEY"),
        api_secret=os.getenv("ALPACA_API_SECRET"),
        base_url="https://data.alpaca.markets",  # Use data URL for historical
    )
    
    # Create client
    client = AlpacaDataClient(config=config)
    
    # Setup catalog
    catalog_path = Path.cwd() / "catalog"
    catalog_path.mkdir(exist_ok=True)
    catalog = ParquetDataCatalog(catalog_path)
    
    # Date range - 6 months back from today
    end_date = datetime.now()
    start_date = end_date - timedelta(days=180)  # 6 months
    
    print(f"\nDownloading NVDA data...")
    print(f"From: {start_date.date()}")
    print(f"To: {end_date.date()}")
    print(f"This may take a few minutes...\n")
    
    try:
        # Connect client
        await client.connect()
        
        # Download using our adapter method
        bars = await client._download_alpaca_bars(
            symbol="NVDA",
            start=start_date,
            end=end_date,
            timeframe="1Min"
        )
        
        print(f"\nDownloaded {len(bars):,} bars")
        
        if bars:
            # Store in catalog
            catalog.write_data(bars)
            print(f"Data saved to: {catalog_path}")
            
            # Show date range
            first_date = datetime.fromtimestamp(bars[0].ts_event / 1e9)
            last_date = datetime.fromtimestamp(bars[-1].ts_event / 1e9)
            
            print(f"\nActual date range:")
            print(f"First bar: {first_date}")
            print(f"Last bar: {last_date}")
            print(f"Total days: {(last_date - first_date).days}")
            
            # Count trading days
            trading_days = set()
            for bar in bars:
                date = datetime.fromtimestamp(bar.ts_event / 1e9).date()
                trading_days.add(date)
            
            print(f"Trading days: {len(trading_days)}")
            print(f"Average bars per day: {len(bars) / len(trading_days):.0f}")
        
        # Disconnect
        await client.disconnect()
        
    except Exception as e:
        print(f"Error: {e}")
        import traceback
        traceback.print_exc()
    
    # Now let's verify what we have in the catalog
    print("\n" + "="*60)
    print("CATALOG CONTENTS")
    print("="*60)
    
    # Query the catalog
    bar_type_str = "NVDA.ALPACA-1-MINUTE-LAST-EXTERNAL"
    stored_bars = catalog.query(
        data_cls=Bar,
        identifiers=[bar_type_str],
    )
    
    if stored_bars:
        print(f"\nTotal bars in catalog: {len(stored_bars):,}")
        
        # Show date range
        first_date = datetime.fromtimestamp(stored_bars[0].ts_event / 1e9)
        last_date = datetime.fromtimestamp(stored_bars[-1].ts_event / 1e9)
        
        print(f"Date range: {first_date.date()} to {last_date.date()}")
        print(f"Ready for backtesting!")
        
        print("\n" + "="*60)
        print("NEXT STEPS")
        print("="*60)
        print("\n1. Re-run parameter optimization with more data:")
        print("   python optimize_ema_cross.py")
        print("\n2. Try walk-forward analysis:")
        print("   python optimize_advanced.py")
        print("\n3. Test different strategies from nt_reference/examples/")
    else:
        print("No data found in catalog!")


if __name__ == "__main__":
    # Check environment
    if not os.getenv("ALPACA_API_KEY"):
        print("Please set ALPACA_API_KEY and ALPACA_API_SECRET environment variables")
        exit(1)
    
    asyncio.run(main())
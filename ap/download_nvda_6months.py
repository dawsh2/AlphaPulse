#!/usr/bin/env python3
"""
Download 6 months of NVDA data using Alpaca API directly.
"""

import os
import asyncio
import aiohttp
import pandas as pd
from datetime import datetime, timedelta
from pathlib import Path

from nautilus_trader.persistence.catalog import ParquetDataCatalog
from nautilus_trader.model.data import Bar, BarType
from nautilus_trader.model.identifiers import Symbol, Venue, InstrumentId
from nautilus_trader.model.objects import Price, Quantity
from nautilus_trader.core.datetime import dt_to_unix_nanos


async def download_alpaca_bars(symbol: str, start: datetime, end: datetime):
    """Download bars directly from Alpaca API."""
    
    api_key = os.getenv("ALPACA_API_KEY")
    api_secret = os.getenv("ALPACA_API_SECRET")
    
    if not api_key or not api_secret:
        raise ValueError("Please set ALPACA_API_KEY and ALPACA_API_SECRET")
    
    base_url = "https://data.alpaca.markets/v2/stocks"
    
    # Alpaca pagination limit
    page_limit = 10000
    
    all_bars = []
    page_token = None
    
    async with aiohttp.ClientSession() as session:
        while True:
            # Build URL
            url = f"{base_url}/{symbol}/bars"
            
            # Parameters
            params = {
                "start": start.isoformat() + "Z",
                "end": end.isoformat() + "Z",
                "timeframe": "1Min",
                "limit": page_limit,
                "adjustment": "raw",
                "feed": "sip",
            }
            
            if page_token:
                params["page_token"] = page_token
            
            # Headers
            headers = {
                "APCA-API-KEY-ID": api_key,
                "APCA-API-SECRET-KEY": api_secret,
            }
            
            print(f"Fetching page... {len(all_bars)} bars downloaded so far")
            
            async with session.get(url, headers=headers, params=params) as response:
                if response.status != 200:
                    text = await response.text()
                    raise Exception(f"API error {response.status}: {text}")
                
                data = await response.json()
                
                bars_data = data.get("bars", [])
                if not bars_data:
                    break
                
                # Convert to NautilusTrader bars
                venue = Venue("ALPACA")
                instrument_id = InstrumentId(Symbol(symbol), venue)
                bar_type = BarType.from_str(f"{symbol}.ALPACA-1-MINUTE-LAST-EXTERNAL")
                
                for bar_data in bars_data:
                    bar = Bar(
                        bar_type=bar_type,
                        open=Price.from_str(str(bar_data["o"])),
                        high=Price.from_str(str(bar_data["h"])),
                        low=Price.from_str(str(bar_data["l"])),
                        close=Price.from_str(str(bar_data["c"])),
                        volume=Quantity.from_int(bar_data["v"]),
                        ts_event=dt_to_unix_nanos(pd.Timestamp(bar_data["t"])),
                        ts_init=dt_to_unix_nanos(pd.Timestamp(bar_data["t"])),
                    )
                    all_bars.append(bar)
                
                # Check for next page
                page_token = data.get("next_page_token")
                if not page_token:
                    break
    
    return all_bars


async def main():
    """Download and store 6 months of NVDA data."""
    
    print("="*60)
    print("DOWNLOAD 6 MONTHS OF NVDA DATA")
    print("="*60)
    
    # Date range - end date should be today or last trading day
    end_date = datetime.now()
    if end_date.hour < 9:  # Before market open
        end_date = end_date - timedelta(days=1)
    
    # Go back 6 months from end date
    start_date = end_date - timedelta(days=180)  # 6 months
    
    print(f"\nDate range:")
    print(f"Start: {start_date.date()}")
    print(f"End: {end_date.date()}")
    
    try:
        # Download bars
        print(f"\nDownloading NVDA bars...")
        bars = await download_alpaca_bars("NVDA", start_date, end_date)
        
        print(f"\nTotal bars downloaded: {len(bars):,}")
        
        if bars:
            # Store in catalog
            catalog_path = Path.cwd() / "catalog"
            catalog_path.mkdir(exist_ok=True)
            catalog = ParquetDataCatalog(catalog_path)
            
            # Write data
            catalog.write_data(bars)
            print(f"Data saved to: {catalog_path}")
            
            # Show statistics
            first_date = datetime.fromtimestamp(bars[0].ts_event / 1e9)
            last_date = datetime.fromtimestamp(bars[-1].ts_event / 1e9)
            
            print(f"\nData statistics:")
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
            
            # Price range
            prices = [float(bar.close) for bar in bars]
            print(f"\nPrice range:")
            print(f"Min: ${min(prices):.2f}")
            print(f"Max: ${max(prices):.2f}")
            print(f"Current: ${prices[-1]:.2f}")
        
    except Exception as e:
        print(f"\nError: {e}")
        import traceback
        traceback.print_exc()
        return
    
    # Verify catalog
    print("\n" + "="*60)
    print("VERIFYING CATALOG")
    print("="*60)
    
    catalog_path = Path.cwd() / "catalog"
    catalog = ParquetDataCatalog(catalog_path)
    
    bar_type_str = "NVDA.ALPACA-1-MINUTE-LAST-EXTERNAL"
    stored_bars = catalog.query(
        data_cls=Bar,
        identifiers=[bar_type_str],
    )
    
    print(f"\nTotal bars in catalog: {len(stored_bars):,}")
    
    if stored_bars:
        first_date = datetime.fromtimestamp(stored_bars[0].ts_event / 1e9)
        last_date = datetime.fromtimestamp(stored_bars[-1].ts_event / 1e9)
        
        print(f"Date range: {first_date.date()} to {last_date.date()}")
        print(f"Days covered: {(last_date - first_date).days}")
        
        print("\n" + "="*60)
        print("READY FOR OPTIMIZATION!")
        print("="*60)
        print("\nNext steps:")
        print("1. Run parameter optimization: python optimize_ema_cross.py")
        print("2. Try walk-forward analysis: python optimize_advanced.py")
        print("3. Test on different timeframes (5min, 15min, etc.)")


if __name__ == "__main__":
    asyncio.run(main())
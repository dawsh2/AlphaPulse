#!/usr/bin/env python3
"""
Download historical data using the Alpaca adapter.
This demonstrates the production-ready interface.
"""

import asyncio
import os
from datetime import datetime, timedelta
from pathlib import Path

# Import the Alpaca adapter
import sys
sys.path.insert(0, '/Users/daws/alphapulse/ap')
from nautilus_adapters.alpaca import AlpacaDataClient, AlpacaDataClientConfig
from nautilus_trader.cache.cache import Cache
from nautilus_trader.common.component import LiveClock, MessageBus
from nautilus_trader.common.providers import InstrumentProvider
from nautilus_trader.model.identifiers import ClientId, TraderId, Venue
from nautilus_trader.persistence.catalog import ParquetDataCatalog


async def main():
    """Download historical data using the Alpaca adapter."""
    
    # Set up catalog
    catalog_path = Path.home() / ".nautilus" / "catalog"
    catalog = ParquetDataCatalog(catalog_path)
    
    # Configure data client (reads from env vars automatically)
    config = AlpacaDataClientConfig(
        api_key=os.getenv("ALPACA_API_KEY"),
        api_secret=os.getenv("ALPACA_API_SECRET"),
    )
    
    # Set up minimal NT components
    loop = asyncio.get_event_loop()
    clock = LiveClock()
    venue = Venue("ALPACA")
    
    msgbus = MessageBus(
        trader_id=TraderId("DOWNLOADER"),
        clock=clock,
    )
    
    cache = Cache()
    instrument_provider = InstrumentProvider()
    
    # Create data client
    client = AlpacaDataClient(
        loop=loop,
        client_id=ClientId("ALPACA"),
        venue=venue,
        msgbus=msgbus,
        cache=cache,
        clock=clock,
        instrument_provider=instrument_provider,
        config=config,
    )
    
    # Connect
    await client.connect()
    
    # Download data
    end_date = datetime.now()
    start_date = end_date - timedelta(days=30)
    
    results = await client.download_and_store(
        symbols=["NVDA", "AAPL", "MSFT", "GOOGL", "AMZN"],
        start=start_date,
        end=end_date,
        bar_interval="1-MINUTE",
        catalog=catalog,
    )
    
    # Show results
    print("\n📊 Download Results:")
    for symbol, count in results.items():
        if count > 0:
            print(f"  ✅ {symbol}: {count:,} bars")
        elif count == 0:
            print(f"  ⚠️  {symbol}: No data")
        else:
            print(f"  ❌ {symbol}: Error")
    
    # Disconnect
    await client.disconnect()
    
    print(f"\n✅ Data stored in: {catalog_path}")


if __name__ == "__main__":
    # Ensure environment variables are set
    if not os.getenv("ALPACA_API_KEY"):
        print("❌ Please set ALPACA_API_KEY and ALPACA_API_SECRET environment variables")
        exit(1)
        
    asyncio.run(main())
#!/usr/bin/env python3
"""
Download more historical data from Alpaca for better backtesting.
"""

import asyncio
from datetime import datetime, timedelta
from pathlib import Path

from nautilus_trader.persistence.catalog import ParquetDataCatalog

# Import our Alpaca adapter
import sys
sys.path.append(str(Path(__file__).parent))
from nautilus_adapters.alpaca.data import AlpacaDataClient
from nautilus_adapters.alpaca.config import AlpacaDataClientConfig


async def download_extended_data(symbol: str, months: int = 6):
    """Download extended historical data."""
    
    # Calculate date range
    end_date = datetime.now()
    start_date = end_date - timedelta(days=months * 30)
    
    print(f"\nDownloading {months} months of {symbol} data...")
    print(f"From: {start_date.date()}")
    print(f"To: {end_date.date()}")
    
    # Configure data client
    import os
    config = AlpacaDataClientConfig(
        api_key=os.getenv("ALPACA_API_KEY"),
        api_secret=os.getenv("ALPACA_API_SECRET"),
        base_url=os.getenv("ALPACA_BASE_URL", "https://paper-api.alpaca.markets")
    )
    client = AlpacaDataClient(config=config)
    
    # Download the data
    try:
        # Connect the client first
        await client.connect()
        
        # Create instrument
        from nautilus_trader.model.identifiers import Symbol, Venue, InstrumentId
        from nautilus_trader.model.instruments import Equity
        from nautilus_trader.model.currencies import USD
        from nautilus_trader.model.objects import Price, Quantity
        
        venue = Venue("ALPACA")
        instrument_id = InstrumentId(Symbol(symbol), venue)
        
        instrument = Equity(
            instrument_id=instrument_id,
            raw_symbol=Symbol(symbol),
            currency=USD,
            price_precision=2,
            price_increment=Price(0.01, 2),
            lot_size=Quantity.from_int(1),
            isin=None,
            ts_event=0,
            ts_init=0,
        )
        
        # Request bars
        bar_type = BarType.from_str(f"{symbol}.ALPACA-1-MINUTE-LAST-EXTERNAL")
        
        from nautilus_trader.core.datetime import dt_to_unix_nanos
        start_ns = dt_to_unix_nanos(pd.Timestamp(start_date))
        end_ns = dt_to_unix_nanos(pd.Timestamp(end_date))
        
        # Use the Alpaca-specific method
        bars = await client._download_alpaca_bars(
            symbol=symbol,
            start=start_date,
            end=end_date,
            timeframe="1Min"
        )
        
        print(f"\nDownloaded {len(bars):,} bars")
        
        if bars:
            # Save to catalog
            catalog_path = Path.home() / ".nautilus" / "catalog"
            catalog = ParquetDataCatalog(catalog_path)
            
            await client.download_and_store(
                catalog=catalog,
                symbols=[symbol],
                start=start_date,
                end=end_date,
                bar_size="1Min"
            )
            
            print(f"Data saved to: {catalog_path}")
            
            # Show date range
            first_bar = bars[0]
            last_bar = bars[-1]
            first_date = datetime.fromtimestamp(first_bar.ts_event / 1e9)
            last_date = datetime.fromtimestamp(last_bar.ts_event / 1e9)
            
            print(f"\nActual date range in data:")
            print(f"First bar: {first_date}")
            print(f"Last bar: {last_date}")
            print(f"Total days: {(last_date - first_date).days}")
            
            # Calculate some statistics
            daily_bars = {}
            for bar in bars:
                date = datetime.fromtimestamp(bar.ts_event / 1e9).date()
                daily_bars[date] = daily_bars.get(date, 0) + 1
            
            print(f"\nTrading days: {len(daily_bars)}")
            print(f"Average bars per day: {len(bars) / len(daily_bars):.0f}")
            
            return bars
        
    except Exception as e:
        print(f"Error downloading data: {e}")
        return None


async def download_multiple_symbols(symbols: list, months: int = 6):
    """Download data for multiple symbols."""
    
    print(f"\nDownloading {months} months of data for: {', '.join(symbols)}")
    
    import os
    config = AlpacaDataClientConfig(
        api_key=os.getenv("ALPACA_API_KEY"),
        api_secret=os.getenv("ALPACA_API_SECRET"),
        base_url=os.getenv("ALPACA_BASE_URL", "https://paper-api.alpaca.markets")
    )
    client = AlpacaDataClient(config=config)
    
    # Calculate date range
    end_date = datetime.now()
    start_date = end_date - timedelta(days=months * 30)
    
    # Set up catalog
    catalog_path = Path.home() / ".nautilus" / "catalog"
    catalog = ParquetDataCatalog(catalog_path)
    
    # Download all symbols
    await client.download_and_store(
        catalog=catalog,
        symbols=symbols,
        start=start_date,
        end=end_date,
        bar_size="1Min"
    )
    
    print(f"\nData saved to: {catalog_path}")
    
    # Show summary
    for symbol in symbols:
        bar_type_str = f"{symbol}.ALPACA-1-MINUTE-LAST-EXTERNAL"
        bars = catalog.query(
            data_cls=Bar,
            identifiers=[bar_type_str],
        )
        if bars:
            print(f"\n{symbol}: {len(bars):,} bars")


async def main():
    """Main function."""
    
    print("="*60)
    print("DOWNLOAD EXTENDED HISTORICAL DATA")
    print("="*60)
    
    # Download 6 months of NVDA data
    await download_extended_data("NVDA", months=6)
    
    # Optionally download more symbols for diversification
    print("\n" + "="*60)
    print("DOWNLOAD MULTIPLE SYMBOLS (Optional)")
    print("="*60)
    
    # Popular liquid stocks for testing
    symbols = ["NVDA", "AAPL", "TSLA", "SPY", "QQQ", "AMD", "MSFT"]
    
    response = input(f"\nDownload 6 months for {len(symbols)} symbols? (y/n): ")
    if response.lower() == 'y':
        await download_multiple_symbols(symbols, months=6)
    
    print("\n" + "="*60)
    print("RECOMMENDATIONS")
    print("="*60)
    print("\n1. Re-run optimization with more data:")
    print("   python optimize_ema_cross.py")
    print("\n2. Test on multiple symbols to avoid overfitting")
    print("\n3. Use walk-forward analysis:")
    print("   python optimize_advanced.py")
    print("\n4. Consider different timeframes:")
    print("   - 5-minute bars for less noise")
    print("   - Daily bars for longer-term trends")
    

if __name__ == "__main__":
    import os
    
    # Check for API keys
    if not os.getenv("ALPACA_API_KEY"):
        print("Please set ALPACA_API_KEY environment variable")
        exit(1)
    
    asyncio.run(main())
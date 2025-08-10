#!/usr/bin/env python3
# -------------------------------------------------------------------------------------------------
#  Copyright (C) 2015-2025 Nautech Systems Pty Ltd. All rights reserved.
#  https://nautechsystems.io
#
#  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
#  You may not use this file except in compliance with the License.
#  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
#
#  Unless required by applicable law or agreed to in writing, software
#  distributed under the License is distributed on an "AS IS" BASIS,
#  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#  See the License for the specific language governing permissions and
#  limitations under the License.
# -------------------------------------------------------------------------------------------------
"""
Example of backtesting with Alpaca historical data.

This example demonstrates:
1. Downloading historical data from Alpaca
2. Running a backtest with the data
3. Analyzing results
"""

import asyncio
import os
from datetime import datetime, timedelta
from decimal import Decimal

import pandas as pd

from nautilus_trader.backtest.engine import BacktestEngine
from nautilus_trader.backtest.engine import BacktestEngineConfig
from nautilus_trader.config import LoggingConfig
from nautilus_trader.examples.strategies.ema_cross import EMACross
from nautilus_trader.examples.strategies.ema_cross import EMACrossConfig
from nautilus_trader.model.currencies import USD
from nautilus_trader.model.data import BarType
from nautilus_trader.model.enums import AccountType
from nautilus_trader.model.enums import OmsType
from nautilus_trader.model.identifiers import InstrumentId
from nautilus_trader.model.identifiers import Symbol
from nautilus_trader.model.identifiers import Venue
from nautilus_trader.model.objects import Money
from nautilus_trader.model.objects import Price
from nautilus_trader.model.objects import Quantity
from nautilus_trader.model.instruments import Equity

# Import our Alpaca adapter
from nautilus_trader.adapters.alpaca.data import AlpacaDataClient
from nautilus_trader.adapters.alpaca.config import AlpacaDataClientConfig


# Configuration
ALPACA_API_KEY = os.getenv("ALPACA_API_KEY")
ALPACA_API_SECRET = os.getenv("ALPACA_API_SECRET")
ALPACA_BASE_URL = "https://paper-api.alpaca.markets"


async def download_alpaca_data(symbol: str, start: datetime, end: datetime):
    """
    Download historical bar data from Alpaca.
    
    Parameters
    ----------
    symbol : str
        The stock symbol to download.
    start : datetime
        Start date for historical data.
    end : datetime
        End date for historical data.
        
    Returns
    -------
    list[Bar]
        List of bars downloaded from Alpaca.
    
    """
    # Create venue and instrument ID
    venue = Venue("ALPACA")
    instrument_id = InstrumentId(Symbol(symbol), venue)
    bar_type = BarType.from_str(f"{symbol}.ALPACA-1-MINUTE-LAST-EXTERNAL")
    
    # Create a minimal trading node just for data download
    from nautilus_trader.cache.cache import Cache
    from nautilus_trader.common.component import LiveClock
    from nautilus_trader.common.component import MessageBus
    from nautilus_trader.common.component import TestClock
    from nautilus_trader.common.providers import InstrumentProvider
    from nautilus_trader.model.identifiers import ClientId
    from nautilus_trader.model.identifiers import TraderId
    from nautilus_trader.portfolio.portfolio import Portfolio
    
    # Set up components
    loop = asyncio.get_event_loop()
    clock = LiveClock()
    trader_id = TraderId("TRADER-001")
    
    # Create message bus
    msgbus = MessageBus(
        trader_id=trader_id,
        clock=clock,
    )
    
    # Create cache
    cache = Cache()
    instrument = Equity(
        instrument_id=instrument_id,
        raw_symbol=Symbol(symbol),
        currency=USD,
        price_precision=2,
        price_increment=Price(0.01, 2),
        lot_size=Quantity.from_int(1),
        isin=None,
        ts_event=clock.timestamp_ns(),
        ts_init=clock.timestamp_ns(),
    )
    cache.add_instrument(instrument)
    
    # Create instrument provider
    instrument_provider = InstrumentProvider()
    instrument_provider.add(instrument)
    
    # Create data client
    config = AlpacaDataClientConfig(
        api_key=ALPACA_API_KEY,
        api_secret=ALPACA_API_SECRET,
        base_url=ALPACA_BASE_URL,
        update_instruments_on_start=False,
    )
    
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
    
    # Connect client
    client.connect()
    # Wait a moment for connection
    await asyncio.sleep(0.5)
    
    print(f"Client session: {client._session}")
    
    # Create request for historical bars
    from nautilus_trader.data.messages import RequestBars
    from nautilus_trader.core.uuid import UUID4
    
    bars_received = []
    
    def handle_bars(bar_type, bars, partial, correlation_id, start, end, params):
        """Handle received bars."""
        bars_received.extend(bars)
    
    # Monkey patch the handler
    client._handle_bars = handle_bars
    
    # Request bars
    request = RequestBars(
        bar_type=bar_type,
        start=start,
        end=end,
        limit=0,  # No limit
        client_id=None,
        venue=venue,
        callback=lambda x: None,
        request_id=UUID4(),
        ts_init=clock.timestamp_ns(),
        params=None,
    )
    
    print(f"Requesting bars from {start} to {end}")
    await client._request_bars(request)
    
    # Disconnect
    client.disconnect()
    await asyncio.sleep(0.1)
    
    print(f"Received {len(bars_received)} bars")
    
    return bars_received


async def main():
    """Run the Alpaca backtest example."""
    
    # Download historical data
    print("Downloading historical data from Alpaca...")
    
    # Use dates from 2024 to ensure we get data
    end_date = datetime(2024, 11, 1)  # November 1, 2024
    start_date = end_date - timedelta(days=30)  # October 2, 2024
    
    bars = await download_alpaca_data("NVDA", start_date, end_date)
    
    if not bars:
        print("No data downloaded!")
        return
    
    print(f"Downloaded {len(bars)} bars")
    
    # Configure backtest engine
    config = BacktestEngineConfig(
        trader_id="BACKTESTER-001",
        logging=LoggingConfig(log_level="INFO"),
    )
    
    # Create backtest engine
    engine = BacktestEngine(config=config)
    
    # Add venue
    ALPACA = Venue("ALPACA")
    engine.add_venue(
        venue=ALPACA,
        oms_type=OmsType.NETTING,
        account_type=AccountType.MARGIN,
        base_currency=USD,
        starting_balances=[Money(100_000, USD)],
    )
    
    # Add instrument
    instrument_id = InstrumentId(Symbol("NVDA"), ALPACA)
    instrument = Equity(
        instrument_id=instrument_id,
        raw_symbol=Symbol("NVDA"),
        currency=USD,
        price_precision=2,
        price_increment=Price(0.01, 2),
        lot_size=Quantity.from_int(1),
        isin=None,
        ts_event=0,
        ts_init=0,
    )
    engine.add_instrument(instrument)
    
    # Add data
    engine.add_data(bars)
    
    # Configure strategy
    bar_type = BarType.from_str("NVDA.ALPACA-1-MINUTE-LAST-EXTERNAL")
    
    strategy_config = EMACrossConfig(
        instrument_id=instrument_id,
        bar_type=bar_type,
        fast_ema_period=10,
        slow_ema_period=20,
        trade_size=Decimal(100),  # Trade 100 shares
    )
    
    # Add strategy
    strategy = EMACross(config=strategy_config)
    engine.add_strategy(strategy)
    
    # Run backtest
    print("\nRunning backtest...")
    engine.run()
    
    # Print results
    print("\nBacktest complete!")
    print("\nAccount Report:")
    print(engine.trader.generate_account_report(ALPACA))
    
    print("\nPositions Report:")
    print(engine.trader.generate_positions_report())


if __name__ == "__main__":
    # Ensure we have API credentials
    if not ALPACA_API_KEY or not ALPACA_API_SECRET:
        print("Please set ALPACA_API_KEY and ALPACA_API_SECRET environment variables")
        sys.exit(1)
    
    asyncio.run(main())
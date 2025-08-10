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

import asyncio
from datetime import datetime
from decimal import Decimal
from typing import Any

import aiohttp
import msgspec
import pandas as pd
from websockets import connect

from nautilus_trader.cache.cache import Cache
from nautilus_trader.common.component import LiveClock
from nautilus_trader.common.component import MessageBus
from nautilus_trader.common.providers import InstrumentProvider
from nautilus_trader.core.datetime import millis_to_nanos
from nautilus_trader.core.datetime import secs_to_nanos
from nautilus_trader.core.uuid import UUID4
from nautilus_trader.data.messages import RequestBars
from nautilus_trader.data.messages import RequestQuoteTicks
from nautilus_trader.data.messages import RequestTradeTicks
from nautilus_trader.data.messages import SubscribeBars
from nautilus_trader.data.messages import SubscribeQuoteTicks
from nautilus_trader.data.messages import SubscribeTradeTicks
from nautilus_trader.data.messages import UnsubscribeBars
from nautilus_trader.data.messages import UnsubscribeQuoteTicks
from nautilus_trader.data.messages import UnsubscribeTradeTicks
from nautilus_trader.live.data_client import LiveMarketDataClient
from nautilus_trader.model.data import Bar
from nautilus_trader.model.data import BarAggregation
from nautilus_trader.model.data import BarSpecification
from nautilus_trader.model.data import BarType
from nautilus_trader.model.data import QuoteTick
from nautilus_trader.model.data import TradeTick
from nautilus_trader.model.enums import AggressorSide
from nautilus_trader.model.enums import PriceType
from nautilus_trader.persistence.catalog import ParquetDataCatalog
from nautilus_trader.model.identifiers import ClientId
from nautilus_trader.model.identifiers import InstrumentId
from nautilus_trader.model.identifiers import Symbol
from nautilus_trader.model.identifiers import TraderId
from nautilus_trader.model.identifiers import Venue
from nautilus_trader.model.currencies import USD
from nautilus_trader.model.instruments import Equity
from nautilus_trader.model.objects import Price
from nautilus_trader.model.objects import Quantity

from .config import AlpacaDataClientConfig


class AlpacaDataClient(LiveMarketDataClient):
    """
    Provides a market data client for Alpaca.
    
    This client handles both historical data requests and real-time data streaming
    via websockets.
    """

    def __init__(
        self,
        loop: asyncio.AbstractEventLoop,
        client_id: ClientId,
        venue: Venue,
        msgbus: MessageBus,
        cache: Cache,
        clock: LiveClock,
        instrument_provider: InstrumentProvider,
        config: AlpacaDataClientConfig,
    ) -> None:
        super().__init__(
            loop=loop,
            client_id=client_id,
            venue=venue,
            msgbus=msgbus,
            cache=cache,
            clock=clock,
            instrument_provider=instrument_provider,
        )

        self._config = config
        self._base_url = config.base_url
        self._ws_base_url = config.ws_base_url
        self._api_key = config.api_key
        self._api_secret = config.api_secret
        
        # HTTP session for REST API
        self._session: aiohttp.ClientSession | None = None
        
        # WebSocket connections
        self._ws_quotes = None
        self._ws_trades = None
        self._ws_bars = None
        
        # Subscription tracking
        self._subscribed_quotes: set[InstrumentId] = set()
        self._subscribed_trades: set[InstrumentId] = set()
        self._subscribed_bars: set[BarType] = set()
        
        # Tasks
        self._ws_tasks: set[asyncio.Task] = set()

    async def _connect(self) -> None:
        """Connect to Alpaca data services."""
        self._log.info("Connecting to Alpaca data services")
        
        # Create HTTP session
        self._session = aiohttp.ClientSession()
        
        # Update instruments on start if configured
        if self._config.update_instruments_on_start:
            await self._update_instruments()
        
        self._log.info("Connected to Alpaca data services")

    async def _disconnect(self) -> None:
        """Disconnect from Alpaca data services."""
        self._log.info("Disconnecting from Alpaca data services")
        
        # Cancel WebSocket tasks
        for task in self._ws_tasks:
            task.cancel()
        
        # Close WebSocket connections
        if self._ws_quotes:
            await self._ws_quotes.close()
        if self._ws_trades:
            await self._ws_trades.close()
        if self._ws_bars:
            await self._ws_bars.close()
        
        # Close HTTP session
        if self._session:
            await self._session.close()
        
        self._log.info("Disconnected from Alpaca data services")

    def reset(self) -> None:
        """Reset the client."""
        self._subscribed_quotes.clear()
        self._subscribed_trades.clear()
        self._subscribed_bars.clear()
        self._ws_tasks.clear()

    def dispose(self) -> None:
        """Dispose of the client."""
        if self._session and not self._session.closed:
            self._loop.create_task(self._session.close())

    # -- SUBSCRIPTIONS ----------------------------------------------------------------------------

    async def _subscribe_quote_ticks(self, command: SubscribeQuoteTicks) -> None:
        """Subscribe to quote tick data."""
        instrument_id = command.instrument_id
        
        if instrument_id in self._subscribed_quotes:
            self._log.warning(f"Already subscribed to quotes for {instrument_id}")
            return
        
        self._subscribed_quotes.add(instrument_id)
        
        # Connect to quotes WebSocket if not connected
        if not self._ws_quotes:
            await self._connect_quotes_ws()
        
        # Send subscription message
        symbol = instrument_id.symbol.value
        sub_msg = {
            "action": "subscribe",
            "quotes": [symbol]
        }
        await self._ws_quotes.send(msgspec.json.encode(sub_msg))
        
        self._log.info(f"Subscribed to quotes for {instrument_id}")

    async def _subscribe_trade_ticks(self, command: SubscribeTradeTicks) -> None:
        """Subscribe to trade tick data."""
        instrument_id = command.instrument_id
        
        if instrument_id in self._subscribed_trades:
            self._log.warning(f"Already subscribed to trades for {instrument_id}")
            return
        
        self._subscribed_trades.add(instrument_id)
        
        # Connect to trades WebSocket if not connected
        if not self._ws_trades:
            await self._connect_trades_ws()
        
        # Send subscription message
        symbol = instrument_id.symbol.value
        sub_msg = {
            "action": "subscribe",
            "trades": [symbol]
        }
        await self._ws_trades.send(msgspec.json.encode(sub_msg))
        
        self._log.info(f"Subscribed to trades for {instrument_id}")

    async def _subscribe_bars(self, command: SubscribeBars) -> None:
        """Subscribe to bar data."""
        bar_type = command.bar_type
        
        if bar_type in self._subscribed_bars:
            self._log.warning(f"Already subscribed to bars for {bar_type}")
            return
        
        # Alpaca only supports certain bar intervals
        if bar_type.spec.aggregation != BarAggregation.MINUTE:
            self._log.error(f"Alpaca only supports minute bars, got {bar_type.spec.aggregation}")
            return
        
        self._subscribed_bars.add(bar_type)
        
        # Connect to bars WebSocket if not connected
        if not self._ws_bars:
            await self._connect_bars_ws()
        
        # Send subscription message
        symbol = bar_type.instrument_id.symbol.value
        sub_msg = {
            "action": "subscribe",
            "bars": [symbol]
        }
        await self._ws_bars.send(msgspec.json.encode(sub_msg))
        
        self._log.info(f"Subscribed to bars for {bar_type}")

    async def _unsubscribe_quote_ticks(self, command: UnsubscribeQuoteTicks) -> None:
        """Unsubscribe from quote tick data."""
        instrument_id = command.instrument_id
        
        if instrument_id not in self._subscribed_quotes:
            self._log.warning(f"Not subscribed to quotes for {instrument_id}")
            return
        
        self._subscribed_quotes.remove(instrument_id)
        
        # Send unsubscription message
        if self._ws_quotes:
            symbol = instrument_id.symbol.value
            unsub_msg = {
                "action": "unsubscribe",
                "quotes": [symbol]
            }
            await self._ws_quotes.send(msgspec.json.encode(unsub_msg))
        
        self._log.info(f"Unsubscribed from quotes for {instrument_id}")

    async def _unsubscribe_trade_ticks(self, command: UnsubscribeTradeTicks) -> None:
        """Unsubscribe from trade tick data."""
        instrument_id = command.instrument_id
        
        if instrument_id not in self._subscribed_trades:
            self._log.warning(f"Not subscribed to trades for {instrument_id}")
            return
        
        self._subscribed_trades.remove(instrument_id)
        
        # Send unsubscription message
        if self._ws_trades:
            symbol = instrument_id.symbol.value
            unsub_msg = {
                "action": "unsubscribe",
                "trades": [symbol]
            }
            await self._ws_trades.send(msgspec.json.encode(unsub_msg))
        
        self._log.info(f"Unsubscribed from trades for {instrument_id}")

    async def _unsubscribe_bars(self, command: UnsubscribeBars) -> None:
        """Unsubscribe from bar data."""
        bar_type = command.bar_type
        
        if bar_type not in self._subscribed_bars:
            self._log.warning(f"Not subscribed to bars for {bar_type}")
            return
        
        self._subscribed_bars.remove(bar_type)
        
        # Send unsubscription message
        if self._ws_bars:
            symbol = bar_type.instrument_id.symbol.value
            unsub_msg = {
                "action": "unsubscribe",
                "bars": [symbol]
            }
            await self._ws_bars.send(msgspec.json.encode(unsub_msg))
        
        self._log.info(f"Unsubscribed from bars for {bar_type}")

    # -- REQUESTS ---------------------------------------------------------------------------------

    async def _request_quote_ticks(self, request: RequestQuoteTicks) -> None:
        """Request historical quote tick data."""
        instrument_id = request.instrument_id
        start = request.start
        end = request.end
        
        # Convert timestamps to RFC3339 format with Z suffix
        start_str = pd.Timestamp(start).isoformat() + "Z"
        end_str = pd.Timestamp(end).isoformat() + "Z" if end else None
        
        # Make API request
        url = f"https://data.alpaca.markets/v2/stocks/{instrument_id.symbol.value}/quotes"
        params = {
            "start": start_str,
            "limit": 10000,  # Max allowed by Alpaca
        }
        if end_str:
            params["end"] = end_str
        
        headers = {
            "APCA-API-KEY-ID": self._api_key,
            "APCA-API-SECRET-KEY": self._api_secret,
        }
        
        quotes = []
        page_token = None
        
        while True:
            if page_token:
                params["page_token"] = page_token
            
            async with self._session.get(url, params=params, headers=headers) as response:
                data = await response.json()
                
                if "quotes" not in data:
                    self._log.error(f"No quotes data in response: {data}")
                    break
                
                # Convert Alpaca quotes to QuoteTicks
                for quote in data["quotes"]:
                    tick = self._parse_quote_tick(quote, instrument_id)
                    quotes.append(tick)
                
                # Check for next page
                page_token = data.get("next_page_token")
                if not page_token:
                    break
        
        # Send quotes to engine
        self._handle_quote_ticks(instrument_id, quotes, request.id)
        
        self._log.info(f"Requested {len(quotes)} quote ticks for {instrument_id}")

    async def _request_trade_ticks(self, request: RequestTradeTicks) -> None:
        """Request historical trade tick data."""
        instrument_id = request.instrument_id
        start = request.start
        end = request.end
        
        # Convert timestamps to RFC3339 format with Z suffix
        start_str = pd.Timestamp(start).isoformat() + "Z"
        end_str = pd.Timestamp(end).isoformat() + "Z" if end else None
        
        # Make API request
        url = f"https://data.alpaca.markets/v2/stocks/{instrument_id.symbol.value}/trades"
        params = {
            "start": start_str,
            "limit": 10000,  # Max allowed by Alpaca
        }
        if end_str:
            params["end"] = end_str
        
        headers = {
            "APCA-API-KEY-ID": self._api_key,
            "APCA-API-SECRET-KEY": self._api_secret,
        }
        
        trades = []
        page_token = None
        
        while True:
            if page_token:
                params["page_token"] = page_token
            
            async with self._session.get(url, params=params, headers=headers) as response:
                data = await response.json()
                
                if "trades" not in data:
                    self._log.error(f"No trades data in response: {data}")
                    break
                
                # Convert Alpaca trades to TradeTicks
                for trade in data["trades"]:
                    tick = self._parse_trade_tick(trade, instrument_id)
                    trades.append(tick)
                
                # Check for next page
                page_token = data.get("next_page_token")
                if not page_token:
                    break
        
        # Send trades to engine
        self._handle_trade_ticks(instrument_id, trades, request.id)
        
        self._log.info(f"Requested {len(trades)} trade ticks for {instrument_id}")

    async def _request_bars(self, request: RequestBars) -> None:
        """Request historical bar data."""
        print(f"DEBUG: _request_bars called")
        print(f"DEBUG: Session exists: {self._session is not None}")
        
        bar_type = request.bar_type
        start = request.start
        end = request.end
        
        # Convert timestamps to RFC3339 format with Z suffix
        start_str = pd.Timestamp(start).isoformat() + "Z"
        end_str = pd.Timestamp(end).isoformat() + "Z" if end else None
        
        # Determine timeframe from bar spec
        timeframe = self._get_alpaca_timeframe(bar_type.spec)
        
        # Make API request
        url = f"https://data.alpaca.markets/v2/stocks/{bar_type.instrument_id.symbol.value}/bars"
        params = {
            "start": start_str,
            "timeframe": timeframe,
            "limit": 10000,  # Max allowed by Alpaca
        }
        if end_str:
            params["end"] = end_str
        
        headers = {
            "APCA-API-KEY-ID": self._api_key,
            "APCA-API-SECRET-KEY": self._api_secret,
        }
        
        self._log.info(f"Requesting bars from Alpaca: {url} with params: {params}")
        
        bars = []
        page_token = None
        
        print(f"DEBUG: About to make API request to {url}")
        
        while True:
            if page_token:
                params["page_token"] = page_token
            
            print(f"DEBUG: Making request with params: {params}")
            async with self._session.get(url, params=params, headers=headers) as response:
                print(f"DEBUG: Got response status: {response.status}")
                if response.status != 200:
                    error_text = await response.text()
                    self._log.error(f"Alpaca API error {response.status}: {error_text}")
                    print(f"DEBUG: Error text: {error_text}")
                    break
                    
                data = await response.json()
                self._log.debug(f"Alpaca response: {data}")
                print(f"DEBUG: Got response with keys: {data.keys()}")
                
                if "bars" not in data:
                    self._log.error(f"No bars data in response: {data}")
                    print(f"DEBUG: No bars in response!")
                    break
                
                # Convert Alpaca bars to NautilusTrader bars
                bars_data = data.get("bars", {})
                print(f"DEBUG: bars_data type: {type(bars_data)}, length: {len(bars_data) if isinstance(bars_data, list) else 'dict'}")
                
                # Alpaca returns bars as a dict with symbol as key
                if isinstance(bars_data, dict) and bar_type.instrument_id.symbol.value in bars_data:
                    symbol_bars = bars_data[bar_type.instrument_id.symbol.value]
                    print(f"DEBUG: Found {len(symbol_bars)} bars for {bar_type.instrument_id.symbol.value}")
                    for bar_data in symbol_bars:
                        bar = self._parse_bar(bar_data, bar_type)
                        bars.append(bar)
                elif isinstance(bars_data, list):
                    for bar_data in bars_data:
                        bar = self._parse_bar(bar_data, bar_type)
                        bars.append(bar)
                
                # Check for next page
                page_token = data.get("next_page_token")
                if not page_token:
                    break
        
        # Send bars to engine
        self._handle_bars(
            bar_type, 
            bars, 
            None,  # No partial bar
            request.id,
            request.start,
            request.end,
            request.params,
        )
        
        self._log.info(f"Requested {len(bars)} bars for {bar_type}")

    # -- WEBSOCKET CONNECTIONS --------------------------------------------------------------------

    async def _connect_quotes_ws(self) -> None:
        """Connect to Alpaca quotes WebSocket."""
        url = f"{self._ws_base_url}/v2/sip"
        self._ws_quotes = await connect(url)
        
        # Authenticate
        auth_msg = {
            "action": "auth",
            "key": self._api_key,
            "secret": self._api_secret
        }
        await self._ws_quotes.send(msgspec.json.encode(auth_msg))
        
        # Start listening task
        task = self._loop.create_task(self._listen_quotes_ws())
        self._ws_tasks.add(task)

    async def _connect_trades_ws(self) -> None:
        """Connect to Alpaca trades WebSocket."""
        url = f"{self._ws_base_url}/v2/sip"
        self._ws_trades = await connect(url)
        
        # Authenticate
        auth_msg = {
            "action": "auth",
            "key": self._api_key,
            "secret": self._api_secret
        }
        await self._ws_trades.send(msgspec.json.encode(auth_msg))
        
        # Start listening task
        task = self._loop.create_task(self._listen_trades_ws())
        self._ws_tasks.add(task)

    async def _connect_bars_ws(self) -> None:
        """Connect to Alpaca bars WebSocket."""
        url = f"{self._ws_base_url}/v2/sip"
        self._ws_bars = await connect(url)
        
        # Authenticate
        auth_msg = {
            "action": "auth",
            "key": self._api_key,
            "secret": self._api_secret
        }
        await self._ws_bars.send(msgspec.json.encode(auth_msg))
        
        # Start listening task
        task = self._loop.create_task(self._listen_bars_ws())
        self._ws_tasks.add(task)

    async def _listen_quotes_ws(self) -> None:
        """Listen for quote messages from WebSocket."""
        try:
            async for message in self._ws_quotes:
                data = msgspec.json.decode(message)
                
                if isinstance(data, list):
                    for item in data:
                        if item.get("T") == "q":  # Quote message
                            await self._handle_quote_ws(item)
        except Exception as e:
            self._log.error(f"Error in quotes WebSocket: {e}")

    async def _listen_trades_ws(self) -> None:
        """Listen for trade messages from WebSocket."""
        try:
            async for message in self._ws_trades:
                data = msgspec.json.decode(message)
                
                if isinstance(data, list):
                    for item in data:
                        if item.get("T") == "t":  # Trade message
                            await self._handle_trade_ws(item)
        except Exception as e:
            self._log.error(f"Error in trades WebSocket: {e}")

    async def _listen_bars_ws(self) -> None:
        """Listen for bar messages from WebSocket."""
        try:
            async for message in self._ws_bars:
                data = msgspec.json.decode(message)
                
                if isinstance(data, list):
                    for item in data:
                        if item.get("T") == "b":  # Bar message
                            await self._handle_bar_ws(item)
        except Exception as e:
            self._log.error(f"Error in bars WebSocket: {e}")

    # -- MESSAGE HANDLERS -------------------------------------------------------------------------

    async def _handle_quote_ws(self, data: dict) -> None:
        """Handle quote message from WebSocket."""
        symbol = data["S"]
        instrument_id = InstrumentId(Symbol(symbol), self._venue)
        
        # Check if we're subscribed
        if instrument_id not in self._subscribed_quotes:
            return
        
        tick = self._parse_quote_tick(data, instrument_id)
        self._handle_quote_tick(tick)

    async def _handle_trade_ws(self, data: dict) -> None:
        """Handle trade message from WebSocket."""
        symbol = data["S"]
        instrument_id = InstrumentId(Symbol(symbol), self._venue)
        
        # Check if we're subscribed
        if instrument_id not in self._subscribed_trades:
            return
        
        tick = self._parse_trade_tick(data, instrument_id)
        self._handle_trade_tick(tick)

    async def _handle_bar_ws(self, data: dict) -> None:
        """Handle bar message from WebSocket."""
        symbol = data["S"]
        
        # Find matching bar type
        bar_type = None
        for bt in self._subscribed_bars:
            if bt.instrument_id.symbol.value == symbol:
                bar_type = bt
                break
        
        if not bar_type:
            return
        
        bar = self._parse_bar(data, bar_type)
        self._handle_bar(bar_type, bar)

    # -- PARSING ----------------------------------------------------------------------------------

    def _parse_quote_tick(self, data: dict, instrument_id: InstrumentId) -> QuoteTick:
        """Parse Alpaca quote data to QuoteTick."""
        # Get instrument for tick size
        instrument = self._cache.instrument(instrument_id)
        tick_size = instrument.price_increment if instrument else 0.01
        
        return QuoteTick(
            instrument_id=instrument_id,
            bid_price=Price(float(data.get("bp", 0)), precision=Price.from_str(str(tick_size)).precision),
            ask_price=Price(float(data.get("ap", 0)), precision=Price.from_str(str(tick_size)).precision),
            bid_size=Quantity.from_int(int(data.get("bs", 0))),
            ask_size=Quantity.from_int(int(data.get("as", 0))),
            ts_event=millis_to_nanos(int(pd.Timestamp(data["t"]).timestamp() * 1000)),
            ts_init=self._clock.timestamp_ns(),
        )

    def _parse_trade_tick(self, data: dict, instrument_id: InstrumentId) -> TradeTick:
        """Parse Alpaca trade data to TradeTick."""
        # Get instrument for tick size
        instrument = self._cache.instrument(instrument_id)
        tick_size = instrument.price_increment if instrument else 0.01
        
        return TradeTick(
            instrument_id=instrument_id,
            price=Price(float(data["p"]), precision=Price.from_str(str(tick_size)).precision),
            size=Quantity.from_int(int(data["s"])),
            aggressor_side=AggressorSide.UNKNOWN,  # Alpaca doesn't provide this
            trade_id=str(data.get("i", "")),
            ts_event=millis_to_nanos(int(pd.Timestamp(data["t"]).timestamp() * 1000)),
            ts_init=self._clock.timestamp_ns(),
        )

    def _parse_bar(self, data: dict, bar_type: BarType) -> Bar:
        """Parse Alpaca bar data to Bar."""
        # Get instrument for tick size
        instrument = self._cache.instrument(bar_type.instrument_id)
        tick_size = instrument.price_increment if instrument else 0.01
        precision = Price.from_str(str(tick_size)).precision
        
        return Bar(
            bar_type=bar_type,
            open=Price(float(data["o"]), precision=precision),
            high=Price(float(data["h"]), precision=precision),
            low=Price(float(data["l"]), precision=precision),
            close=Price(float(data["c"]), precision=precision),
            volume=Quantity.from_int(int(data["v"])),
            ts_event=millis_to_nanos(int(pd.Timestamp(data["t"]).timestamp() * 1000)),
            ts_init=self._clock.timestamp_ns(),
        )

    def _get_alpaca_timeframe(self, spec: BarSpecification) -> str:
        """Convert bar specification to Alpaca timeframe string."""
        if spec.aggregation == BarAggregation.MINUTE:
            return f"{spec.step}Min"
        elif spec.aggregation == BarAggregation.HOUR:
            return f"{spec.step}Hour"
        elif spec.aggregation == BarAggregation.DAY:
            return f"{spec.step}Day"
        else:
            raise ValueError(f"Unsupported bar aggregation: {spec.aggregation}")

    async def _update_instruments(self) -> None:
        """Update instruments from Alpaca."""
        url = f"{self._base_url}/v2/assets"
        headers = {
            "APCA-API-KEY-ID": self._api_key,
            "APCA-API-SECRET-KEY": self._api_secret,
        }
        
        async with self._session.get(url, headers=headers) as response:
            assets = await response.json()
        
        instruments = []
        for asset in assets:
            if asset["status"] == "active" and asset["tradable"]:
                instrument = self._parse_instrument(asset)
                instruments.append(instrument)
        
        # Add instruments to cache
        for instrument in instruments:
            self._cache.add_instrument(instrument)
        
        self._log.info(f"Updated {len(instruments)} instruments from Alpaca")

    def _parse_instrument(self, asset: dict) -> Equity:
        """Parse Alpaca asset to instrument."""
        symbol = Symbol(asset["symbol"])
        
        return Equity(
            instrument_id=InstrumentId(symbol, self._venue),
            raw_symbol=symbol,
            currency=USD,
            price_precision=2,  # Most US equities
            price_increment=Decimal("0.01"),
            multiplier=Quantity.from_int(1),
            lot_size=Quantity.from_int(1),
            isin=None,
            ts_event=self._clock.timestamp_ns(),
            ts_init=self._clock.timestamp_ns(),
        )
    
    # -- ENHANCED METHODS FOR PRODUCTION USE ------------------------------------------------------
    
    async def download_and_store(
        self,
        symbols: list[str],
        start: datetime,
        end: datetime,
        bar_interval: str = "1-MINUTE",
        catalog: ParquetDataCatalog = None,
    ) -> dict[str, int]:
        """
        Download historical data for multiple symbols and store directly in catalog.
        
        Parameters
        ----------
        symbols : list[str]
            List of symbols to download
        start : datetime
            Start time for historical data
        end : datetime
            End time for historical data  
        bar_interval : str
            Bar interval (1-MINUTE, 5-MINUTE, 1-HOUR, 1-DAY)
        catalog : ParquetDataCatalog
            Catalog to store data
            
        Returns
        -------
        dict[str, int]
            Dictionary of symbol -> number of bars downloaded
        """
        if not self.is_connected:
            raise RuntimeError("Client not connected. Call connect() first.")
            
        if not catalog:
            raise ValueError("Catalog parameter is required.")
            
        results = {}
        retry_attempts = 3
        retry_delay = 1.0
        rate_limit_delay = 0.2  # 200ms between requests
        
        for symbol in symbols:
            self._log.info(f"Downloading {symbol} from {start} to {end}")
            
            try:
                # Download with retry logic
                bars = await self._download_bars_with_retry(
                    symbol=symbol,
                    start=start,
                    end=end,
                    bar_interval=bar_interval,
                    retry_attempts=retry_attempts,
                    retry_delay=retry_delay,
                )
                
                if bars:
                    # Store instrument if not exists
                    instrument = self._get_or_create_instrument(symbol)
                    if instrument.id not in [inst.id for inst in catalog.instruments()]:
                        catalog.write_data([instrument])
                    
                    # Store bars with proper timestamps
                    first_ts = bars[0].ts_event
                    last_ts = bars[-1].ts_event
                    
                    catalog.write_data(
                        bars,
                        start=first_ts,
                        end=last_ts,
                    )
                    
                    results[symbol] = len(bars)
                    self._log.info(f"Stored {len(bars)} bars for {symbol}")
                else:
                    results[symbol] = 0
                    self._log.warning(f"No bars received for {symbol}")
                    
            except Exception as e:
                self._log.error(f"Failed to download {symbol}: {e}")
                results[symbol] = -1  # Indicate error
                
            # Rate limiting
            if symbol != symbols[-1]:  # Not the last symbol
                await asyncio.sleep(rate_limit_delay)
                
        # Consolidate catalog for performance
        try:
            self._log.info("Consolidating catalog...")
            catalog.consolidate_catalog()
        except Exception as e:
            self._log.warning(f"Could not consolidate catalog: {e}")
            
        return results
    
    async def _download_bars_with_retry(
        self,
        symbol: str,
        start: datetime,
        end: datetime,
        bar_interval: str,
        retry_attempts: int = 3,
        retry_delay: float = 1.0,
    ) -> list:
        """Download bars with retry logic for robustness."""
        bar_type = BarType.from_str(f"{symbol}.{self._venue}-{bar_interval}-LAST-EXTERNAL")
        
        for attempt in range(retry_attempts):
            try:
                bars_received = []
                
                # Temporarily override handler
                original_handler = self._handle_bars
                
                def collect_bars(bar_type, bars, partial, correlation_id, start, end, params):
                    bars_received.extend(bars)
                
                self._handle_bars = collect_bars
                
                # Create request
                request = RequestBars(
                    bar_type=bar_type,
                    start=start,
                    end=end,
                    limit=0,
                    client_id=None,
                    venue=self._venue,
                    callback=lambda x: None,
                    request_id=UUID4(),
                    ts_init=self._clock.timestamp_ns(),
                    params=None,
                )
                
                # Make request
                await self._request_bars(request)
                
                # Restore handler
                self._handle_bars = original_handler
                
                if bars_received:
                    return bars_received
                    
            except Exception as e:
                self._log.warning(f"Attempt {attempt + 1} failed for {symbol}: {e}")
                if attempt < retry_attempts - 1:
                    await asyncio.sleep(retry_delay * (attempt + 1))
                else:
                    raise
                    
        return []
    
    def _get_or_create_instrument(self, symbol: str):
        """Get instrument from cache or create new one."""
        instrument_id = InstrumentId(Symbol(symbol), self._venue)
        
        # Check cache first
        instrument = self._cache.instrument(instrument_id)
        if instrument:
            return instrument
            
        # Create new instrument
        instrument = Equity(
            instrument_id=instrument_id,
            raw_symbol=Symbol(symbol),
            currency=USD,
            price_precision=2,
            price_increment=Decimal("0.01"),
            lot_size=Quantity.from_int(1),
            isin=None,
            ts_event=self._clock.timestamp_ns(),
            ts_init=self._clock.timestamp_ns(),
        )
        
        self._cache.add_instrument(instrument)
        return instrument
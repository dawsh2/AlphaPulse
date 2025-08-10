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
from decimal import Decimal
from typing import Any

import aiohttp
import msgspec
import pandas as pd
from websockets import connect

from nautilus_trader.cache.cache import Cache
from nautilus_trader.common.component import LiveClock
from nautilus_trader.common.component import MessageBus
from nautilus_trader.core.datetime import millis_to_nanos
from nautilus_trader.core.uuid import UUID4
from nautilus_trader.execution.messages import CancelAllOrders
from nautilus_trader.execution.messages import CancelOrder
from nautilus_trader.execution.messages import GenerateFillReports
from nautilus_trader.execution.messages import GenerateOrderStatusReport
from nautilus_trader.execution.messages import GenerateOrderStatusReports
from nautilus_trader.execution.messages import GeneratePositionStatusReports
from nautilus_trader.execution.messages import ModifyOrder
from nautilus_trader.execution.messages import SubmitOrder
from nautilus_trader.execution.reports import FillReport
from nautilus_trader.execution.reports import OrderStatusReport
from nautilus_trader.execution.reports import PositionStatusReport
from nautilus_trader.live.execution_client import LiveExecutionClient
from nautilus_trader.model.currencies import USD
from nautilus_trader.model.enums import AccountType
from nautilus_trader.model.enums import LiquiditySide
from nautilus_trader.model.enums import OmsType
from nautilus_trader.model.enums import OrderSide
from nautilus_trader.model.enums import OrderStatus
from nautilus_trader.model.enums import OrderType
from nautilus_trader.model.enums import PositionSide
from nautilus_trader.model.enums import TimeInForce
from nautilus_trader.model.events import AccountState
from nautilus_trader.model.events import OrderAccepted
from nautilus_trader.model.events import OrderCanceled
from nautilus_trader.model.events import OrderFilled
from nautilus_trader.model.events import OrderRejected
from nautilus_trader.model.events import OrderSubmitted
from nautilus_trader.model.events import OrderUpdated
from nautilus_trader.model.identifiers import AccountId
from nautilus_trader.model.identifiers import ClientId
from nautilus_trader.model.identifiers import ClientOrderId
from nautilus_trader.model.identifiers import InstrumentId
from nautilus_trader.model.identifiers import PositionId
from nautilus_trader.model.identifiers import Symbol
from nautilus_trader.model.identifiers import TraderId
from nautilus_trader.model.identifiers import TradeId
from nautilus_trader.model.identifiers import Venue
from nautilus_trader.model.identifiers import VenueOrderId
from nautilus_trader.model.objects import AccountBalance
from nautilus_trader.model.objects import Money
from nautilus_trader.model.objects import Price
from nautilus_trader.model.objects import Quantity
from nautilus_trader.model.orders import LimitOrder
from nautilus_trader.model.orders import MarketOrder
from nautilus_trader.model.orders import Order
from nautilus_trader.model.orders import StopMarketOrder

from .config import AlpacaExecClientConfig


class AlpacaExecutionClient(LiveExecutionClient):
    """
    Provides an execution client for Alpaca.
    
    This client handles order submission, modification, cancellation and
    account/position queries via the Alpaca API.
    """

    def __init__(
        self,
        loop: asyncio.AbstractEventLoop,
        client_id: ClientId,
        venue: Venue,
        msgbus: MessageBus,
        cache: Cache,
        clock: LiveClock,
        config: AlpacaExecClientConfig,
    ) -> None:
        account_type = AccountType.CASH if config.account_type == "CASH" else AccountType.MARGIN
        
        super().__init__(
            loop=loop,
            client_id=client_id,
            venue=venue,
            oms_type=OmsType.NETTING,  # Alpaca uses netting
            account_type=account_type,
            base_currency=USD,
            msgbus=msgbus,
            cache=cache,
            clock=clock,
        )

        self._config = config
        self._base_url = config.base_url
        self._api_key = config.api_key
        self._api_secret = config.api_secret
        
        # HTTP session for REST API
        self._session: aiohttp.ClientSession | None = None
        
        # WebSocket for order updates
        self._ws_orders = None
        self._ws_task: asyncio.Task | None = None
        
        # Order tracking
        self._client_order_ids: dict[str, ClientOrderId] = {}  # Alpaca ID -> Client ID
        self._venue_order_ids: dict[ClientOrderId, str] = {}  # Client ID -> Alpaca ID

    async def _connect(self) -> None:
        """Connect to Alpaca execution services."""
        self._log.info("Connecting to Alpaca execution services")
        
        # Create HTTP session
        self._session = aiohttp.ClientSession()
        
        # Get account info
        await self._update_account_state()
        
        # Connect to order updates WebSocket
        await self._connect_orders_ws()
        
        # Update instruments on start if configured
        if self._config.update_instruments_on_start:
            await self._update_instruments()
        
        self._log.info("Connected to Alpaca execution services")

    async def _disconnect(self) -> None:
        """Disconnect from Alpaca execution services."""
        self._log.info("Disconnecting from Alpaca execution services")
        
        # Cancel WebSocket task
        if self._ws_task:
            self._ws_task.cancel()
        
        # Close WebSocket connection
        if self._ws_orders:
            await self._ws_orders.close()
        
        # Close HTTP session
        if self._session:
            await self._session.close()
        
        self._log.info("Disconnected from Alpaca execution services")

    def reset(self) -> None:
        """Reset the client."""
        self._client_order_ids.clear()
        self._venue_order_ids.clear()

    def dispose(self) -> None:
        """Dispose of the client."""
        if self._session and not self._session.closed:
            self._loop.create_task(self._session.close())

    # -- EXECUTION REPORTS ------------------------------------------------------------------------

    async def generate_order_status_report(
        self,
        command: GenerateOrderStatusReport,
    ) -> OrderStatusReport | None:
        """Generate order status report for a single order."""
        client_order_id = command.client_order_id
        
        # Get Alpaca order ID
        alpaca_order_id = self._venue_order_ids.get(client_order_id)
        if not alpaca_order_id:
            self._log.warning(f"Cannot find Alpaca order ID for {client_order_id}")
            return None
        
        # Fetch order from API
        url = f"{self._base_url}/v2/orders/{alpaca_order_id}"
        headers = self._get_headers()
        
        async with self._session.get(url, headers=headers) as response:
            if response.status != 200:
                self._log.error(f"Failed to get order {alpaca_order_id}: {await response.text()}")
                return None
            
            order_data = await response.json()
        
        # Convert to order status report
        return self._parse_order_status_report(order_data)

    async def generate_order_status_reports(
        self,
        command: GenerateOrderStatusReports,
    ) -> list[OrderStatusReport]:
        """Generate order status reports for all orders."""
        # Fetch all orders from API
        url = f"{self._base_url}/v2/orders"
        headers = self._get_headers()
        params = {"status": "all", "limit": 500}
        
        reports = []
        
        async with self._session.get(url, headers=headers, params=params) as response:
            if response.status != 200:
                self._log.error(f"Failed to get orders: {await response.text()}")
                return reports
            
            orders_data = await response.json()
        
        # Convert to order status reports
        for order_data in orders_data:
            report = self._parse_order_status_report(order_data)
            if report:
                reports.append(report)
        
        return reports

    async def generate_fill_reports(
        self,
        command: GenerateFillReports,
    ) -> list[FillReport]:
        """Generate fill reports."""
        # Alpaca doesn't have a separate fills endpoint
        # Fills are included in order data
        reports = []
        
        # Get order status reports
        order_reports = await self.generate_order_status_reports(
            GenerateOrderStatusReports(
                trader_id=command.trader_id,
                client_id=command.client_id,
                venue=command.venue,
                account_id=command.account_id,
                correlation_id=command.correlation_id,
                ts_init=command.ts_init,
            )
        )
        
        # Extract fills from orders
        for order_report in order_reports:
            if order_report.filled_qty > 0:
                # Create fill report from order data
                fill_report = FillReport(
                    client_order_id=order_report.client_order_id,
                    venue_order_id=order_report.venue_order_id,
                    trade_id=TradeId(f"{order_report.venue_order_id}_1"),  # Alpaca doesn't provide trade IDs
                    order_side=order_report.order_side,
                    last_px=order_report.avg_px or order_report.price,
                    last_qty=order_report.filled_qty,
                    liquidity_side=LiquiditySide.UNKNOWN,
                    report_id=UUID4(),
                    account_id=order_report.account_id,
                    instrument_id=order_report.instrument_id,
                    venue=order_report.venue,
                    ts_event=order_report.ts_last,
                    ts_init=self._clock.timestamp_ns(),
                )
                reports.append(fill_report)
        
        return reports

    async def generate_position_status_reports(
        self,
        command: GeneratePositionStatusReports,
    ) -> list[PositionStatusReport]:
        """Generate position status reports."""
        # Fetch positions from API
        url = f"{self._base_url}/v2/positions"
        headers = self._get_headers()
        
        reports = []
        
        async with self._session.get(url, headers=headers) as response:
            if response.status != 200:
                self._log.error(f"Failed to get positions: {await response.text()}")
                return reports
            
            positions_data = await response.json()
        
        # Convert to position status reports
        for position_data in positions_data:
            report = self._parse_position_status_report(position_data)
            if report:
                reports.append(report)
        
        return reports

    # -- COMMAND HANDLERS -------------------------------------------------------------------------

    async def _submit_order(self, command: SubmitOrder) -> None:
        """Submit order to Alpaca."""
        order = command.order
        
        # Prepare order data
        order_data = {
            "symbol": order.instrument_id.symbol.value,
            "qty": str(order.quantity),
            "side": "buy" if order.side == OrderSide.BUY else "sell",
            "time_in_force": self._get_alpaca_tif(order.time_in_force),
            "client_order_id": str(order.client_order_id),
        }
        
        # Set order type specific fields
        if isinstance(order, MarketOrder):
            order_data["type"] = "market"
        elif isinstance(order, LimitOrder):
            order_data["type"] = "limit"
            order_data["limit_price"] = str(order.price)
        elif isinstance(order, StopMarketOrder):
            order_data["type"] = "stop"
            order_data["stop_price"] = str(order.trigger_price)
        else:
            self._log.error(f"Unsupported order type: {type(order)}")
            self.generate_order_rejected(
                order,
                reason="Unsupported order type",
                ts_event=self._clock.timestamp_ns(),
            )
            return
        
        # Submit order
        url = f"{self._base_url}/v2/orders"
        headers = self._get_headers()
        
        async with self._session.post(url, headers=headers, json=order_data) as response:
            response_data = await response.json()
            
            if response.status != 200:
                self._log.error(f"Order submission failed: {response_data}")
                self.generate_order_rejected(
                    order,
                    reason=response_data.get("message", "Unknown error"),
                    ts_event=self._clock.timestamp_ns(),
                )
                return
            
            # Store order ID mappings
            alpaca_order_id = response_data["id"]
            self._client_order_ids[alpaca_order_id] = order.client_order_id
            self._venue_order_ids[order.client_order_id] = alpaca_order_id
            
            # Generate order submitted event
            self.generate_order_submitted(
                order.client_order_id,
                ts_event=self._clock.timestamp_ns(),
            )
            
            # Generate order accepted event
            venue_order_id = VenueOrderId(alpaca_order_id)
            self.generate_order_accepted(
                order.client_order_id,
                venue_order_id,
                ts_event=self._clock.timestamp_ns(),
            )

    async def _modify_order(self, command: ModifyOrder) -> None:
        """Modify order on Alpaca."""
        # Get Alpaca order ID
        alpaca_order_id = self._venue_order_ids.get(command.client_order_id)
        if not alpaca_order_id:
            self._log.error(f"Cannot find Alpaca order ID for {command.client_order_id}")
            return
        
        # Prepare modification data
        data = {}
        if command.quantity:
            data["qty"] = str(command.quantity)
        if command.price:
            data["limit_price"] = str(command.price)
        
        # Send modification request
        url = f"{self._base_url}/v2/orders/{alpaca_order_id}"
        headers = self._get_headers()
        
        async with self._session.patch(url, headers=headers, json=data) as response:
            if response.status != 200:
                self._log.error(f"Order modification failed: {await response.text()}")
                return
            
            # Generate order updated event
            self.generate_order_updated(
                command.client_order_id,
                command.quantity,
                command.price,
                None,  # Trigger price
                ts_event=self._clock.timestamp_ns(),
                reconciliation=False,
            )

    async def _cancel_order(self, command: CancelOrder) -> None:
        """Cancel order on Alpaca."""
        # Get Alpaca order ID
        alpaca_order_id = self._venue_order_ids.get(command.client_order_id)
        if not alpaca_order_id:
            self._log.error(f"Cannot find Alpaca order ID for {command.client_order_id}")
            return
        
        # Send cancellation request
        url = f"{self._base_url}/v2/orders/{alpaca_order_id}"
        headers = self._get_headers()
        
        async with self._session.delete(url, headers=headers) as response:
            if response.status not in (200, 204):
                self._log.error(f"Order cancellation failed: {await response.text()}")
                return

    async def _cancel_all_orders(self, command: CancelAllOrders) -> None:
        """Cancel all orders on Alpaca."""
        # Send cancellation request
        url = f"{self._base_url}/v2/orders"
        headers = self._get_headers()
        
        async with self._session.delete(url, headers=headers) as response:
            if response.status not in (200, 204, 207):  # 207 is multi-status
                self._log.error(f"Cancel all orders failed: {await response.text()}")

    # -- WEBSOCKET CONNECTION ---------------------------------------------------------------------

    async def _connect_orders_ws(self) -> None:
        """Connect to Alpaca orders WebSocket."""
        url = f"wss://paper-api.alpaca.markets/stream"  # Use paper or live URL based on config
        if "paper" not in self._base_url:
            url = "wss://api.alpaca.markets/stream"
        
        self._ws_orders = await connect(url)
        
        # Authenticate
        auth_msg = {
            "action": "auth",
            "key": self._api_key,
            "secret": self._api_secret
        }
        await self._ws_orders.send(msgspec.json.encode(auth_msg))
        
        # Subscribe to trade updates
        sub_msg = {
            "action": "listen",
            "data": {
                "streams": ["trade_updates"]
            }
        }
        await self._ws_orders.send(msgspec.json.encode(sub_msg))
        
        # Start listening task
        self._ws_task = self._loop.create_task(self._listen_orders_ws())

    async def _listen_orders_ws(self) -> None:
        """Listen for order updates from WebSocket."""
        try:
            async for message in self._ws_orders:
                data = msgspec.json.decode(message)
                
                if data.get("stream") == "trade_updates":
                    await self._handle_trade_update(data["data"])
        except Exception as e:
            self._log.error(f"Error in orders WebSocket: {e}")

    async def _handle_trade_update(self, data: dict) -> None:
        """Handle trade update from WebSocket."""
        event_type = data["event"]
        order_data = data["order"]
        
        # Get client order ID
        alpaca_order_id = order_data["id"]
        client_order_id = self._client_order_ids.get(alpaca_order_id)
        
        if not client_order_id:
            # Try to parse from client_order_id field
            if order_data.get("client_order_id"):
                client_order_id = ClientOrderId(order_data["client_order_id"])
                self._client_order_ids[alpaca_order_id] = client_order_id
                self._venue_order_ids[client_order_id] = alpaca_order_id
            else:
                self._log.warning(f"Unknown order ID: {alpaca_order_id}")
                return
        
        venue_order_id = VenueOrderId(alpaca_order_id)
        
        # Handle different event types
        if event_type == "accepted":
            self.generate_order_accepted(
                client_order_id,
                venue_order_id,
                ts_event=self._parse_timestamp(data["timestamp"]),
            )
        
        elif event_type == "fill" or event_type == "partial_fill":
            # Parse fill details
            fill_price = Price.from_str(order_data["filled_avg_price"])
            fill_qty = Quantity.from_str(order_data["filled_qty"])
            
            # Generate fill event
            self.generate_order_filled(
                client_order_id,
                venue_order_id,
                venue_position_id=None,  # Alpaca doesn't provide position IDs
                trade_id=TradeId(f"{alpaca_order_id}_{data['timestamp']}"),
                order_side=OrderSide.BUY if order_data["side"] == "buy" else OrderSide.SELL,
                order_type=self._parse_order_type(order_data["type"]),
                last_qty=fill_qty,
                last_px=fill_price,
                quote_currency=USD,
                commission=Money(0, USD),  # Alpaca commission is separate
                liquidity_side=LiquiditySide.UNKNOWN,
                ts_event=self._parse_timestamp(data["timestamp"]),
            )
        
        elif event_type == "canceled":
            self.generate_order_canceled(
                client_order_id,
                venue_order_id,
                ts_event=self._parse_timestamp(data["timestamp"]),
            )
        
        elif event_type == "rejected":
            # This shouldn't happen for already accepted orders
            self._log.warning(f"Order rejected after acceptance: {order_data}")

    # -- HELPERS ----------------------------------------------------------------------------------

    def _get_headers(self) -> dict:
        """Get headers for API requests."""
        return {
            "APCA-API-KEY-ID": self._api_key,
            "APCA-API-SECRET-KEY": self._api_secret,
        }

    def _get_alpaca_tif(self, tif: TimeInForce) -> str:
        """Convert TimeInForce to Alpaca format."""
        mapping = {
            TimeInForce.DAY: "day",
            TimeInForce.GTC: "gtc",
            TimeInForce.IOC: "ioc",
            TimeInForce.FOK: "fok",
        }
        return mapping.get(tif, "day")

    def _parse_order_type(self, alpaca_type: str) -> OrderType:
        """Parse Alpaca order type to OrderType."""
        mapping = {
            "market": OrderType.MARKET,
            "limit": OrderType.LIMIT,
            "stop": OrderType.STOP_MARKET,
            "stop_limit": OrderType.STOP_LIMIT,
        }
        return mapping.get(alpaca_type, OrderType.MARKET)

    def _parse_order_status(self, alpaca_status: str) -> OrderStatus:
        """Parse Alpaca order status to OrderStatus."""
        mapping = {
            "new": OrderStatus.SUBMITTED,
            "accepted": OrderStatus.ACCEPTED,
            "partially_filled": OrderStatus.PARTIALLY_FILLED,
            "filled": OrderStatus.FILLED,
            "canceled": OrderStatus.CANCELED,
            "expired": OrderStatus.EXPIRED,
            "rejected": OrderStatus.REJECTED,
            "pending_new": OrderStatus.SUBMITTED,
            "pending_cancel": OrderStatus.PENDING_CANCEL,
        }
        return mapping.get(alpaca_status, OrderStatus.SUBMITTED)

    def _parse_timestamp(self, timestamp: str) -> int:
        """Parse timestamp string to nanoseconds."""
        return millis_to_nanos(int(pd.Timestamp(timestamp).timestamp() * 1000))

    def _parse_order_status_report(self, order_data: dict) -> OrderStatusReport | None:
        """Parse Alpaca order data to OrderStatusReport."""
        # Get client order ID
        alpaca_order_id = order_data["id"]
        client_order_id = self._client_order_ids.get(alpaca_order_id)
        
        if not client_order_id and order_data.get("client_order_id"):
            client_order_id = ClientOrderId(order_data["client_order_id"])
        
        if not client_order_id:
            return None
        
        instrument_id = InstrumentId(Symbol(order_data["symbol"]), self._venue)
        
        return OrderStatusReport(
            account_id=self.account_id,
            instrument_id=instrument_id,
            venue=self._venue,
            client_order_id=client_order_id,
            venue_order_id=VenueOrderId(alpaca_order_id),
            order_side=OrderSide.BUY if order_data["side"] == "buy" else OrderSide.SELL,
            order_type=self._parse_order_type(order_data["type"]),
            time_in_force=TimeInForce[order_data["time_in_force"].upper()],
            order_status=self._parse_order_status(order_data["status"]),
            quantity=Quantity.from_str(order_data["qty"]),
            filled_qty=Quantity.from_str(order_data["filled_qty"]),
            price=Price.from_str(order_data.get("limit_price", "0")) if order_data.get("limit_price") else None,
            avg_px=Price.from_str(order_data["filled_avg_price"]) if order_data.get("filled_avg_price") else None,
            report_id=UUID4(),
            ts_accepted=self._parse_timestamp(order_data["created_at"]),
            ts_last=self._parse_timestamp(order_data["updated_at"]),
            ts_init=self._clock.timestamp_ns(),
        )

    def _parse_position_status_report(self, position_data: dict) -> PositionStatusReport:
        """Parse Alpaca position data to PositionStatusReport."""
        instrument_id = InstrumentId(Symbol(position_data["symbol"]), self._venue)
        
        return PositionStatusReport(
            account_id=self.account_id,
            instrument_id=instrument_id,
            venue=self._venue,
            position_side=PositionSide.LONG if position_data["side"] == "long" else PositionSide.SHORT,
            quantity=Quantity.from_str(position_data["qty"]),
            report_id=UUID4(),
            ts_last=self._clock.timestamp_ns(),
            ts_init=self._clock.timestamp_ns(),
        )

    async def _update_account_state(self) -> None:
        """Update account state from Alpaca."""
        # Get account info
        url = f"{self._base_url}/v2/account"
        headers = self._get_headers()
        
        async with self._session.get(url, headers=headers) as response:
            if response.status != 200:
                self._log.error(f"Failed to get account info: {await response.text()}")
                return
            
            account_data = await response.json()
        
        # Create account state event
        balances = [
            AccountBalance(
                total=Money(Decimal(account_data["cash"]), USD),
                locked=Money(Decimal(account_data["cash"]) - Decimal(account_data["buying_power"]), USD),
                free=Money(Decimal(account_data["buying_power"]), USD),
            )
        ]
        
        account_state = AccountState(
            account_id=self.account_id,
            account_type=self._account_type,
            base_currency=USD,
            reported=True,
            balances=balances,
            margins=[],
            info=account_data,
            event_id=UUID4(),
            ts_event=self._clock.timestamp_ns(),
            ts_init=self._clock.timestamp_ns(),
        )
        
        self._msgbus.send(endpoint="ExecEngine.process", msg=account_state)

    async def _update_instruments(self) -> None:
        """Update instruments from Alpaca."""
        # This would be similar to the data client implementation
        # but included here for completeness
        pass
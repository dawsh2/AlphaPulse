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
Example of live trading with Alpaca using NautilusTrader.

This example demonstrates:
1. Setting up Alpaca data and execution clients
2. Running a simple EMA cross strategy
3. Handling real-time market data
4. Executing trades through Alpaca
"""

import asyncio
import os
from decimal import Decimal

from nautilus_trader.config import LiveDataEngineConfig
from nautilus_trader.config import LiveExecEngineConfig
from nautilus_trader.config import LiveRiskEngineConfig
from nautilus_trader.config import LoggingConfig
from nautilus_trader.config import TradingNodeConfig
from nautilus_trader.examples.strategies.ema_cross import EMACross
from nautilus_trader.examples.strategies.ema_cross import EMACrossConfig
from nautilus_trader.live.node import TradingNode
from nautilus_trader.model.data import BarType
from nautilus_trader.model.identifiers import InstrumentId
from nautilus_trader.model.identifiers import Symbol
from nautilus_trader.model.identifiers import Venue

# Import our Alpaca adapter
import sys
sys.path.append(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from nautilus_trader.adapters.alpaca import AlpacaDataClientConfig
from nautilus_trader.adapters.alpaca import AlpacaExecClientConfig


# Configuration
ALPACA_API_KEY = os.getenv("ALPACA_API_KEY")
ALPACA_API_SECRET = os.getenv("ALPACA_API_SECRET")
ALPACA_BASE_URL = os.getenv("ALPACA_BASE_URL", "https://paper-api.alpaca.markets")


async def main():
    """Run the Alpaca live trading example."""
    
    # Define venue
    venue = Venue("ALPACA")
    
    # Configure the trading node
    config = TradingNodeConfig(
        trader_id="TRADER-001",
        logging=LoggingConfig(log_level="INFO"),
        data_engine=LiveDataEngineConfig(
            time_bars_build_with_no_updates=True,
            time_bars_timestamp_on_close=True,
            validate_data_sequence=True,
        ),
        exec_engine=LiveExecEngineConfig(
            reconciliation=True,
            reconciliation_lookback_mins=1440,  # 24 hours
        ),
        risk_engine=LiveRiskEngineConfig(
            bypass=False,  # Use risk checks in live trading
            max_order_submit_rate="10/00:00:01",  # 10 orders per second
            max_position_modify_rate="5/00:00:01",  # 5 modifications per second
        ),
        data_clients={
            "ALPACA": AlpacaDataClientConfig(
                api_key=ALPACA_API_KEY,
                api_secret=ALPACA_API_SECRET,
                base_url=ALPACA_BASE_URL,
                update_instruments_on_start=True,
            ),
        },
        exec_clients={
            "ALPACA": AlpacaExecClientConfig(
                api_key=ALPACA_API_KEY,
                api_secret=ALPACA_API_SECRET,
                base_url=ALPACA_BASE_URL,
                account_type="MARGIN",
                update_instruments_on_start=True,
            ),
        },
    )
    
    # Create the trading node
    node = TradingNode(config=config)
    
    # Configure strategy
    instrument_id = InstrumentId(Symbol("AAPL"), venue)
    bar_type = BarType.from_str("AAPL.ALPACA-1-MINUTE-LAST-EXTERNAL")
    
    strategy_config = EMACrossConfig(
        instrument_id=instrument_id,
        bar_type=bar_type,
        fast_ema_period=10,
        slow_ema_period=20,
        trade_size=Decimal(100),  # Trade 100 shares
        request_bars=True,  # Request historical bars on start
    )
    
    # Add strategy to the node
    strategy = EMACross(config=strategy_config)
    node.trader.add_strategy(strategy)
    
    # Start the trading node
    try:
        await node.run_async()
    except KeyboardInterrupt:
        print("Received keyboard interrupt, shutting down...")
    finally:
        await node.stop_async()
        await node.dispose_async()


if __name__ == "__main__":
    asyncio.run(main())
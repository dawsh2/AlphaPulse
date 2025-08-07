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

from nautilus_trader.config import LiveDataClientConfig
from nautilus_trader.config import LiveExecClientConfig


class AlpacaDataClientConfig(LiveDataClientConfig, frozen=True, kw_only=True):
    """
    Configuration for ``AlpacaDataClient`` instances.

    Parameters
    ----------
    api_key : str
        The Alpaca API key.
    api_secret : str
        The Alpaca API secret.
    base_url : str, optional
        The Alpaca base URL (paper or live trading).
        Default is paper trading URL.
    ws_base_url : str, optional
        The Alpaca websocket base URL.
    update_instruments_on_start : bool, default True
        Whether to update instruments on start.
    
    """

    api_key: str
    api_secret: str
    base_url: str = "https://paper-api.alpaca.markets"
    ws_base_url: str = "wss://stream.data.alpaca.markets"
    update_instruments_on_start: bool = True


class AlpacaExecClientConfig(LiveExecClientConfig, frozen=True, kw_only=True):
    """
    Configuration for ``AlpacaExecutionClient`` instances.

    Parameters
    ----------
    api_key : str
        The Alpaca API key.
    api_secret : str
        The Alpaca API secret.
    base_url : str, optional
        The Alpaca base URL (paper or live trading).
        Default is paper trading URL.
    account_type : str, optional
        The account type (CASH or MARGIN).
    update_instruments_on_start : bool, default True
        Whether to update instruments on start.
    
    """

    api_key: str
    api_secret: str
    base_url: str = "https://paper-api.alpaca.markets"
    account_type: str = "MARGIN"
    update_instruments_on_start: bool = True
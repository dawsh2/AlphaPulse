"""
Alpaca API client using OS environment variables and pure requests
"""
import requests
import json
from datetime import datetime, timedelta
from config import Config

class AlpacaAPIClient:
    """Alpaca API client using API keys from OS environment variables."""
    
    def __init__(self):
        self.base_url = Config.ALPACA_BASE_URL
        self.headers = Config.get_alpaca_headers()
    
    def _make_request(self, method, endpoint, data=None, params=None):
        """Make authenticated request to Alpaca API."""
        url = f"{self.base_url.rstrip('/')}/{endpoint.lstrip('/')}"
        
        try:
            if method.upper() == 'GET':
                response = requests.get(url, headers=self.headers, params=params)
            elif method.upper() == 'POST':
                response = requests.post(url, headers=self.headers, json=data)
            elif method.upper() == 'DELETE':
                response = requests.delete(url, headers=self.headers)
            else:
                raise ValueError(f"Unsupported HTTP method: {method}")
            
            if response.status_code in [200, 201]:
                return response.json()
            else:
                print(f"Alpaca API request failed: {response.status_code} - {response.text}")
                return None
                
        except Exception as e:
            print(f"Error making Alpaca API request: {str(e)}")
            return None
    
    def get_account(self):
        """Get account information."""
        return self._make_request('GET', '/v2/account')
    
    def get_positions(self):
        """Get current positions."""
        positions_data = self._make_request('GET', '/v2/positions')
        if not positions_data:
            return []
        
        # Convert to objects similar to alpaca-trade-api format
        positions = []
        for pos in positions_data:
            position = AlpacaPosition(pos)
            positions.append(position)
        return positions
    
    def get_orders(self, status='all', limit=50):
        """Get orders."""
        params = {'status': status, 'limit': limit}
        orders_data = self._make_request('GET', '/v2/orders', params=params)
        if not orders_data:
            return []
        
        # Convert to objects similar to alpaca-trade-api format
        orders = []
        for order in orders_data:
            order_obj = AlpacaOrder(order)
            orders.append(order_obj)
        return orders
    
    def submit_order(self, symbol, qty, side, type='market', time_in_force='day'):
        """Submit a trading order."""
        order_data = {
            'symbol': symbol,
            'qty': qty,
            'side': side,
            'type': type,
            'time_in_force': time_in_force
        }
        
        result = self._make_request('POST', '/v2/orders', order_data)
        if result:
            return AlpacaOrder(result)
        return None
    
    def get_bars(self, symbol, timeframe='1Min', start=None, end=None, limit=1000):
        """Get historical bars."""
        try:
            if not start:
                start = datetime.now() - timedelta(days=1)
            if not end:
                end = datetime.now()
            
            # Format dates for Alpaca API
            start_str = start.strftime('%Y-%m-%dT%H:%M:%SZ')
            end_str = end.strftime('%Y-%m-%dT%H:%M:%SZ')
            
            params = {
                'symbols': symbol,
                'timeframe': timeframe,
                'start': start_str,
                'end': end_str,
                'limit': limit
            }
            
            # Use the market data API endpoint
            market_data_url = f"https://data.alpaca.markets/v2/stocks/bars"
            
            # Use market data headers (same as trading API for now)
            response = requests.get(market_data_url, headers=self.headers, params=params)
            
            if response.status_code == 200:
                result = response.json()
                if 'bars' in result and symbol in result['bars']:
                    bars = []
                    for bar_data in result['bars'][symbol]:
                        bar = AlpacaBar(bar_data)
                        bars.append(bar)
                    return bars
            else:
                print(f"Market data request failed: {response.status_code} - {response.text}")
            
            return []
            
        except Exception as e:
            print(f"Error getting bars: {str(e)}")
            return []
    
    def is_market_open(self):
        """Check if market is currently open."""
        try:
            clock_data = self._make_request('GET', '/v2/clock')
            if clock_data:
                return clock_data.get('is_open', False)
            return False
        except Exception as e:
            print(f"Error checking market status: {str(e)}")
            return False
    
    def test_connection(self):
        """Test the API connection."""
        try:
            account = self.get_account()
            if account:
                return {
                    'status': 'success',
                    'account_type': 'paper' if 'paper' in self.base_url else 'live',
                    'buying_power': account.get('buying_power'),
                    'cash': account.get('cash')
                }
            else:
                return {'status': 'error', 'message': 'Failed to get account info'}
        except Exception as e:
            return {'status': 'error', 'message': str(e)}

class AlpacaPosition:
    """Wrapper for position data to match alpaca-trade-api interface."""
    
    def __init__(self, data):
        self.symbol = data.get('symbol')
        self.qty = data.get('qty')
        self.market_value = data.get('market_value')
        self.unrealized_pl = data.get('unrealized_pl')
        self.unrealized_plpc = data.get('unrealized_plpc')
        self.current_price = data.get('current_price')
        self.avg_entry_price = data.get('avg_entry_price')

class AlpacaOrder:
    """Wrapper for order data to match alpaca-trade-api interface."""
    
    def __init__(self, data):
        self.id = data.get('id')
        self.symbol = data.get('symbol')
        self.qty = data.get('qty')
        self.side = data.get('side')
        self.order_type = data.get('type')
        self.status = data.get('status')
        self.filled_qty = data.get('filled_qty')
        self.filled_avg_price = data.get('filled_avg_price')
        self.created_at = data.get('created_at')

class AlpacaBar:
    """Wrapper for bar data to match alpaca-trade-api interface."""
    
    def __init__(self, data):
        self.timestamp = data.get('t')
        self.open = data.get('o')
        self.high = data.get('h')
        self.low = data.get('l')
        self.close = data.get('c')
        self.volume = data.get('v')
        self.vwap = data.get('vw')  # Volume weighted average price

def create_alpaca_client():
    """Create Alpaca client using OS environment variables."""
    if not Config.ALPACA_API_KEY or not Config.ALPACA_SECRET_KEY:
        return None
    
    return AlpacaAPIClient()

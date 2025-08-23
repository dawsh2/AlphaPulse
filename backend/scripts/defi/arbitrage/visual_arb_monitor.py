#!/usr/bin/env python3
"""Visual Real-time Arbitrage Monitor - Terminal UI like htop
Shows live pools, prices, spreads, and opportunities in real-time
"""

import asyncio
import websockets
import json
import struct
import time
import curses
import threading
from collections import defaultdict, deque
from web3 import Web3
from eth_account import Account
from decimal import Decimal, getcontext
import os
import redis
from datetime import datetime

# High precision for calculations
getcontext().prec = 78

# Connections
w3 = Web3(Web3.HTTPProvider('https://polygon.publicnode.com'))
try:
    r = redis.Redis(host='localhost', port=6379, decode_responses=True, socket_timeout=1)
    r.ping()  # Test connection
except:
    r = None

# Socket paths
UNIX_SOCKET_PATH = "/tmp/alphapulse/polygon.sock"
WS_URL = "wss://polygon.publicnode.com"

# Enhanced token mapping
TOKENS = {
    '0x2791bca1f2de4661ed88a30c99a7a9449aa84174': 'USDC.e',
    '0x3c499c542cef5e3811e1192ce70d8cc03d5c3359': 'USDC',
    '0xc2132d05d31c914a87c6611c10748aeb04b58e8f': 'USDT',
    '0x0d500b1d8e8ef31e21c99d1db9a6444d3adf1270': 'WPOL',
    '0x8f3cf7ad23cd3cadbd9735aff958023239c6a063': 'DAI',
    '0x7ceb23fd6bc0add59e62ac25578270cff1b9f619': 'WETH',
    '0x1bfd67037b42cf73acf2047067bd4f2c47d9bfd6': 'WBTC',
    '0xeb51d9a39ad5eef215dc0bf39a8821ff804a0f01': 'LGNS',
    '0x2e67812d0171e509a21e0c4c2dc11348c812ea00': 'Happy',
    '0x692597b009d13c4049a947cab2239b7d6517875f': 'Pnut',
    '0xdf7837de1f2fa4631d716cf2502f8b230f1dcc32': 'TEL',
    '0xa8b1e0764f85f53dfe21760e8af5bf6cc4504582': 'oAUTO',
    '0x90bb609649e0451e5ad952683d64bd2d37182d6b': 'DDAO',
    '0x8793fbc3baa83c04b99670b5be49c7e35ed68c50': 'NSFW',
    '0x718658312ae3ced8b1d23b6b00de7b7c8a820d8c': 'BTTP',
    '0xb2e52ef941eb1a9da9e1e1a8a81d0d6e96043e5e': 'USDL',
    '0x4e830ff67b4bd3ce33c6e06ff7d9d7ff8b4f705e': 'BAE',
    '0xeac274c4b4e5a70cda4e99c7e4e26a4fb2c34e3b': 'PANA',
    '0x172370d5cd63279efa6d502dab29171933a610af': 'CRV',
    '0xa3fa99a148fa48d14ed51d610c367c61876997f1': 'MANA',
    '0x6bb45ceaf03755b1913ddd4a55c6dba1c4475ced': 'AS',
    '0x6bb45ceaf03755b1913ddd4a55c6dba1c4475ced': 'GLD',
    '0x1796ae0b0fa4862485106a0de9b654efe301d0b2': 'PLT',
    '0x8a1d2e5b9e2b8e6e4a7e7b9d9d9d9d9d9d9d9d9d': 'IXT',
    '0x65559aa14915a70190438ef90104769e5e890a00': 'MEN',
}

# Global state for the visual monitor
class VisualMonitorState:
    def __init__(self):
        self.pools = {}
        self.opportunities = deque(maxlen=20)  # Keep last 20 opportunities
        self.stats = {
            'total_pools': 0,
            'total_trades': 0,
            'total_opportunities': 0,
            'gas_price_gwei': 0,
            'matic_price': 0.25,
            'uptime': time.time(),
            'connections': {'websocket': False, 'unix_socket': False}
        }
        self.recent_trades = deque(maxlen=10)
        self.price_updates = deque(maxlen=100)
        self.lock = threading.Lock()

# Global state instance
state = VisualMonitorState()

class VisualArbitrageMonitor:
    def __init__(self):
        self.pool_details = {}
        self.executing = set()
        
        # Load private key if available
        self.account = None
        private_key = os.getenv('PRIVATE_KEY')
        if private_key:
            self.account = Account.from_key(private_key)

    def get_gas_cost(self, pool_types=None):
        """Calculate gas cost"""
        gas_price_wei = state.stats['gas_price_gwei'] * 1e9
        gas_units = 280000  # Default
        
        if pool_types:
            base_gas = {'V2': 140000, 'V3': 200000, 'STABLE': 160000}
            if all(p == 'V2' for p in pool_types):
                gas_units = base_gas['V2'] * len(pool_types)
            elif all(p == 'V3' for p in pool_types):
                gas_units = base_gas['V3'] * len(pool_types)
        
        gas_cost_matic = (gas_price_wei * gas_units) / 10**18
        return gas_cost_matic * state.stats['matic_price']

    async def connect_websocket(self):
        """WebSocket connection with visual updates"""
        subscription = {
            "jsonrpc": "2.0", "id": 1, "method": "eth_subscribe",
            "params": ["logs", {"topics": ["0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1"]}]
        }
        
        try:
            async with websockets.connect(WS_URL) as ws:
                state.stats['connections']['websocket'] = True
                await ws.send(json.dumps(subscription))
                await ws.recv()  # Subscription confirmation
                
                while True:
                    try:
                        message = await ws.recv()
                        data = json.loads(message)
                        if 'params' in data and 'result' in data['params']:
                            await self.process_sync_event(data['params']['result'])
                    except Exception as e:
                        pass
                        
        except Exception:
            state.stats['connections']['websocket'] = False

    async def connect_unix_socket(self):
        """Unix socket connection with visual updates"""
        try:
            reader, writer = await asyncio.open_unix_connection(UNIX_SOCKET_PATH)
            state.stats['connections']['unix_socket'] = True
            
            while True:
                try:
                    header = await reader.readexactly(8)
                    if not header:
                        break
                    
                    msg_type = header[2]
                    msg_length = struct.unpack('<H', header[4:6])[0]
                    sequence = struct.unpack('<H', header[6:8])[0]
                    
                    payload_length = msg_length - 8
                    if payload_length > 0:
                        payload = await reader.readexactly(payload_length)
                    else:
                        payload = b''
                    
                    if msg_type == 0x01:  # TRADE
                        await self.process_trade_message(payload)
                        
                except asyncio.IncompleteReadError:
                    break
                except Exception:
                    await asyncio.sleep(0.1)
                    
        except:
            state.stats['connections']['unix_socket'] = False

    async def process_trade_message(self, payload):
        """Process trade with visual updates"""
        if len(payload) >= 36:
            try:
                exchange = struct.unpack('<I', payload[0:4])[0]
                instrument = struct.unpack('<Q', payload[4:12])[0]
                price = struct.unpack('<Q', payload[12:20])[0]
                volume = struct.unpack('<Q', payload[20:28])[0]
                
                price_float = price / 1e8
                volume_float = volume / 1e8
                
                with state.lock:
                    state.stats['total_trades'] += 1
                    
                    # Update MATIC price if reasonable
                    if 0.1 < price_float < 2.0:
                        state.stats['matic_price'] = price_float
                    
                    # Add to recent trades
                    state.recent_trades.append({
                        'instrument': f"{instrument:016x}",
                        'price': price_float,
                        'volume': volume_float,
                        'time': datetime.now().strftime("%H:%M:%S")
                    })
                    
                    # Store price update
                    state.price_updates.append({
                        'instrument': instrument,
                        'price': price_float,
                        'timestamp': time.time()
                    })
                    
            except Exception:
                pass

    async def process_sync_event(self, event):
        """Process sync event with pool discovery"""
        pool_address = event['address'].lower()
        data = event['data']
        
        if len(data) >= 130:
            reserve0 = int(data[2:66], 16)
            reserve1 = int(data[66:130], 16)
            
            if pool_address not in self.pool_details:
                await self.fetch_pool_info(pool_address, reserve0, reserve1)
            
            pool_info = self.pool_details.get(pool_address)
            if pool_info:
                with state.lock:
                    # Update pool data
                    r0 = reserve0 / (10 ** pool_info.get('decimals0', 18))
                    r1 = reserve1 / (10 ** pool_info.get('decimals1', 18))
                    price = r1 / r0 if r0 > 0 else 0
                    
                    pool_key = pool_address[:10]
                    state.pools[pool_key] = {
                        'address': pool_address,
                        'pair': f"{pool_info.get('symbol0', 'UNK')}/{pool_info.get('symbol1', 'UNK')}",
                        'price': price,
                        'reserve0': r0,
                        'reserve1': r1,
                        'type': 'V2',
                        'last_update': datetime.now().strftime("%H:%M:%S"),
                        'volume_24h': r0 * price + r1  # Approximate TVL
                    }
                    
                    state.stats['total_pools'] = len(state.pools)
                    
                    # Check for arbitrage opportunities
                    await self.check_arbitrage_visual(pool_key)

    async def fetch_pool_info(self, pool_address, reserve0, reserve1):
        """Fetch pool info for visual display"""
        try:
            pool = Web3.to_checksum_address(pool_address)
            abi = json.loads('[{"inputs":[],"name":"token0","outputs":[{"name":"","type":"address"}],"type":"function"},{"inputs":[],"name":"token1","outputs":[{"name":"","type":"address"}],"type":"function"}]')
            contract = w3.eth.contract(address=pool, abi=abi)
            
            token0 = contract.functions.token0().call().lower()
            token1 = contract.functions.token1().call().lower()
            
            # Get symbols
            symbol0 = TOKENS.get(token0, token0[:6])
            symbol1 = TOKENS.get(token1, token1[:6])
            
            self.pool_details[pool_address] = {
                'token0': token0, 'token1': token1,
                'symbol0': symbol0, 'symbol1': symbol1,
                'decimals0': 18, 'decimals1': 18  # Default
            }
            
        except Exception:
            pass

    async def check_arbitrage_visual(self, pool_key):
        """Check arbitrage with visual feedback"""
        current_pool = state.pools.get(pool_key)
        if not current_pool:
            return
            
        # Find similar pools for arbitrage
        current_pair = current_pool['pair']
        similar_pools = []
        
        with state.lock:
            for key, pool in state.pools.items():
                if key != pool_key and pool['pair'] == current_pair:
                    similar_pools.append((key, pool))
        
        # Check for opportunities
        for other_key, other_pool in similar_pools:
            price1 = current_pool['price']
            price2 = other_pool['price']
            
            if price1 > 0 and price2 > 0:
                spread = abs(price1 - price2) / min(price1, price2)
                
                if spread > 0.005:  # 0.5% minimum spread
                    gas_cost = self.get_gas_cost(['V2', 'V2'])
                    
                    # Estimate profit (simplified)
                    trade_size = min(current_pool['reserve0'], other_pool['reserve0']) * 0.01
                    gross_profit = trade_size * spread * 0.7  # Rough calculation
                    net_profit = gross_profit - gas_cost
                    
                    if net_profit > 0.01:  # $0.01 minimum
                        opportunity = {
                            'pair': current_pair,
                            'spread_pct': spread * 100,
                            'trade_size_usd': trade_size,
                            'net_profit_usd': net_profit,
                            'pool1': pool_key,
                            'pool2': other_key,
                            'timestamp': datetime.now().strftime("%H:%M:%S")
                        }
                        
                        with state.lock:
                            state.opportunities.append(opportunity)
                            state.stats['total_opportunities'] += 1

    async def run_monitor(self):
        """Run the monitoring tasks"""
        # Update gas price
        try:
            gas_price = w3.eth.gas_price
            state.stats['gas_price_gwei'] = gas_price / 1e9
        except:
            state.stats['gas_price_gwei'] = 30
        
        # Start monitoring tasks
        tasks = [
            asyncio.create_task(self.connect_websocket()),
            asyncio.create_task(self.connect_unix_socket()),
        ]
        
        await asyncio.gather(*tasks, return_exceptions=True)

def draw_visual_monitor(stdscr):
    """Draw the visual monitor interface"""
    curses.curs_set(0)  # Hide cursor
    stdscr.nodelay(1)   # Non-blocking input
    stdscr.timeout(100) # Refresh every 100ms
    
    # Colors
    curses.start_color()
    curses.init_pair(1, curses.COLOR_GREEN, curses.COLOR_BLACK)   # Profit
    curses.init_pair(2, curses.COLOR_RED, curses.COLOR_BLACK)     # Loss
    curses.init_pair(3, curses.COLOR_YELLOW, curses.COLOR_BLACK)  # Warning
    curses.init_pair(4, curses.COLOR_CYAN, curses.COLOR_BLACK)    # Info
    curses.init_pair(5, curses.COLOR_MAGENTA, curses.COLOR_BLACK) # Header
    
    while True:
        try:
            stdscr.clear()
            height, width = stdscr.getmaxyx()
            
            # Header
            uptime = int(time.time() - state.stats['uptime'])
            header = f"⚡ ARBITRAGE MONITOR | Uptime: {uptime//60}m{uptime%60}s | Gas: {state.stats['gas_price_gwei']:.1f} Gwei | MATIC: ${state.stats['matic_price']:.4f}"
            stdscr.addstr(0, 0, header[:width-1], curses.color_pair(5) | curses.A_BOLD)
            
            # Connections status
            ws_status = "🟢 WS" if state.stats['connections']['websocket'] else "🔴 WS"
            unix_status = "🟢 UNIX" if state.stats['connections']['unix_socket'] else "🔴 UNIX"
            wallet_status = f"💰 {monitor.account.address[:10]}..." if monitor.account else "💰 No wallet"
            
            conn_line = f"{ws_status} | {unix_status} | {wallet_status}"
            stdscr.addstr(1, 0, conn_line[:width-1], curses.color_pair(4))
            
            # Stats
            stats_line = f"📊 Pools: {state.stats['total_pools']} | Trades: {state.stats['total_trades']} | Opportunities: {state.stats['total_opportunities']}"
            stdscr.addstr(2, 0, stats_line[:width-1], curses.color_pair(4))
            
            # Opportunities section
            stdscr.addstr(4, 0, "💰 ARBITRAGE OPPORTUNITIES (Last 10):", curses.color_pair(5) | curses.A_BOLD)
            
            row = 5
            with state.lock:
                opportunities = list(state.opportunities)[-10:]  # Last 10
            
            if opportunities:
                # Headers
                stdscr.addstr(row, 0, f"{'Time':<8} {'Pair':<15} {'Spread%':<8} {'Profit$':<10} {'Pools':<20}", curses.A_BOLD)
                row += 1
                
                for opp in opportunities:
                    if row >= height - 15:  # Leave space for other sections
                        break
                    
                    time_str = opp['timestamp']
                    pair_str = opp['pair'][:14]
                    spread_str = f"{opp['spread_pct']:.2f}%"
                    profit_str = f"${opp['net_profit_usd']:.3f}"
                    pools_str = f"{opp['pool1']} -> {opp['pool2']}"
                    
                    # Color based on profit
                    color = curses.color_pair(1) if opp['net_profit_usd'] > 0 else curses.color_pair(2)
                    
                    line = f"{time_str:<8} {pair_str:<15} {spread_str:<8} {profit_str:<10} {pools_str:<20}"
                    stdscr.addstr(row, 0, line[:width-1], color)
                    row += 1
            else:
                stdscr.addstr(row, 0, "   No opportunities detected yet...", curses.color_pair(3))
                row += 2
            
            # Recent trades section
            stdscr.addstr(row + 1, 0, "📈 RECENT TRADES:", curses.color_pair(5) | curses.A_BOLD)
            row += 2
            
            with state.lock:
                recent_trades = list(state.recent_trades)[-5:]  # Last 5
            
            if recent_trades:
                stdscr.addstr(row, 0, f"{'Time':<8} {'Instrument':<18} {'Price':<12} {'Volume':<12}", curses.A_BOLD)
                row += 1
                
                for trade in recent_trades:
                    if row >= height - 8:
                        break
                    
                    line = f"{trade['time']:<8} {trade['instrument'][:17]:<18} ${trade['price']:<11.6f} {trade['volume']:<12.2f}"
                    stdscr.addstr(row, 0, line[:width-1], curses.color_pair(4))
                    row += 1
            
            # Top pools section
            stdscr.addstr(row + 1, 0, "🏊 TOP POOLS BY TVL:", curses.color_pair(5) | curses.A_BOLD)
            row += 2
            
            with state.lock:
                pools = list(state.pools.values())
            
            # Sort by volume
            pools.sort(key=lambda x: x.get('volume_24h', 0), reverse=True)
            
            if pools:
                stdscr.addstr(row, 0, f"{'Pair':<15} {'Price':<12} {'TVL$':<12} {'Updated':<8}", curses.A_BOLD)
                row += 1
                
                for pool in pools[:min(8, len(pools))]:  # Top 8 pools
                    if row >= height - 2:
                        break
                    
                    pair_str = pool['pair'][:14]
                    price_str = f"${pool['price']:.6f}"
                    tvl_str = f"${pool.get('volume_24h', 0):.0f}"
                    updated_str = pool['last_update']
                    
                    line = f"{pair_str:<15} {price_str:<12} {tvl_str:<12} {updated_str:<8}"
                    stdscr.addstr(row, 0, line[:width-1], curses.color_pair(4))
                    row += 1
            
            # Footer
            footer = "Press 'q' to quit | Updates every 100ms"
            stdscr.addstr(height-1, 0, footer[:width-1], curses.color_pair(3))
            
            stdscr.refresh()
            
            # Check for quit
            key = stdscr.getch()
            if key == ord('q') or key == ord('Q'):
                break
                
        except Exception as e:
            # In case of any drawing error, just continue
            pass

def main():
    global monitor
    monitor = VisualArbitrageMonitor()
    
    # Start the monitor in a separate thread
    def run_monitor():
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        loop.run_until_complete(monitor.run_monitor())
    
    monitor_thread = threading.Thread(target=run_monitor, daemon=True)
    monitor_thread.start()
    
    # Start the visual interface
    try:
        curses.wrapper(draw_visual_monitor)
    except KeyboardInterrupt:
        pass
    except Exception as e:
        print(f"Visual interface error: {e}")
        print("Falling back to simple output...")
        
        # Fallback to simple text output
        try:
            while True:
                time.sleep(5)
                print(f"\n📊 Stats: {state.stats['total_pools']} pools, {state.stats['total_trades']} trades, {state.stats['total_opportunities']} opportunities")
                
                with state.lock:
                    if state.opportunities:
                        latest = list(state.opportunities)[-1]
                        print(f"💰 Latest opportunity: {latest['pair']} - {latest['spread_pct']:.2f}% spread, ${latest['net_profit_usd']:.3f} profit")
                
        except KeyboardInterrupt:
            pass

if __name__ == "__main__":
    main()
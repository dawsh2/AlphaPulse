#!/usr/bin/env python3
"""
Simple Real-time Arbitrage Monitor
Uses sophisticated arbitrage detection from scripts/arb with the existing data pipeline
"""

import socket
import struct
import time
import os
import sys
from collections import defaultdict
from decimal import Decimal, getcontext
from web3 import Web3
import json

# High precision calculations
getcontext().prec = 78

# Web3 connection
w3 = Web3(Web3.HTTPProvider('https://polygon.publicnode.com'))

# Load environment variables
from dotenv import load_dotenv
load_dotenv('../.env')

# Socket path - connects to relay server
RELAY_SOCKET_PATH = "/tmp/alphapulse/relay.sock"

# Known token pairs we want to monitor (from the proven arb script)
MONITORED_POOLS = [
    ("0x6e7a5fafcec6bb1e78bae2a1f0b612012bf14827", "QuickSwap", "WMATIC", "USDC"),
    ("0xa374094527e1673a86de625aa59517c5de346d32", "Uniswap", "WMATIC", "USDC"),
    ("0x2cf7252e74036d1da831d11089d326296e64a728", "QuickSwap", "USDC", "USDT"),
]

# ABIs from arb script
ERC20_ABI = json.loads('[{"inputs":[],"name":"symbol","outputs":[{"name":"","type":"string"}],"type":"function"},{"inputs":[],"name":"decimals","outputs":[{"name":"","type":"uint8"}],"type":"function"}]')
V2_ABI = json.loads('[{"inputs":[],"name":"getReserves","outputs":[{"name":"","type":"uint112"},{"name":"","type":"uint112"},{"name":"","type":"uint32"}],"type":"function"},{"inputs":[],"name":"token0","outputs":[{"name":"","type":"address"}],"type":"function"},{"inputs":[],"name":"token1","outputs":[{"name":"","type":"address"}],"type":"function"}]')

class SimpleArbitrageMonitor:
    def __init__(self):
        self.pool_prices = {}
        self.pool_details = {}
        self.message_count = 0
        self.opportunities_found = 0
        self.last_opportunity_time = 0
        
        print("🤖 Simple Real-time Arbitrage Monitor")
        print("=" * 60)
        
        # Load pool details on startup
        print("⚠️  Skipping pool loading for now - focusing on data pipeline")
        # self.load_pool_details()
        
    def load_pool_details(self):
        """Load detailed information about monitored pools"""
        print(f"📊 Loading {len(MONITORED_POOLS)} monitored pools...")
        
        for pool_addr, dex, token_a, token_b in MONITORED_POOLS:
            try:
                addr = Web3.to_checksum_address(pool_addr)
                pool = w3.eth.contract(address=addr, abi=V2_ABI)
                
                t0, t1 = pool.functions.token0().call(), pool.functions.token1().call()
                reserves = pool.functions.getReserves().call()
                
                # Get token info
                t0c = w3.eth.contract(address=Web3.to_checksum_address(t0), abi=ERC20_ABI)
                t1c = w3.eth.contract(address=Web3.to_checksum_address(t1), abi=ERC20_ABI)
                
                s0, s1 = t0c.functions.symbol().call(), t1c.functions.symbol().call()
                d0, d1 = t0c.functions.decimals().call(), t1c.functions.decimals().call()
                
                r0_raw, r1_raw = reserves[0], reserves[1]
                r0, r1 = r0_raw / (10**d0), r1_raw / (10**d1)
                price = r1/r0 if r0 > 0 else 0
                
                self.pool_details[pool_addr.lower()] = {
                    'address': pool_addr,
                    'dex': dex,
                    'token0': t0.lower(),
                    'token1': t1.lower(),
                    'symbol0': s0,
                    'symbol1': s1,
                    'decimals0': d0,
                    'decimals1': d1,
                    'reserve0_raw': r0_raw,
                    'reserve1_raw': r1_raw,
                    'reserve0': r0,
                    'reserve1': r1,
                    'price': price,
                    'fee_bps': 30,  # 0.3% for most DEXs
                    'type': 'V2'
                }
                
                print(f"✅ {dex} {s0}/{s1} @ {price:.6f} (reserves: {r0:.2f}/{r1:.2f})")
                
            except Exception as e:
                print(f"❌ Failed to load {pool_addr}: {e}")
    
    def calc_v2_output(self, amount_in, reserve_in, reserve_out, fee_bps=30):
        """Exact V2 calculation from arb script"""
        amount_in = int(amount_in)
        reserve_in = int(reserve_in)
        reserve_out = int(reserve_out)
        
        amount_with_fee = amount_in * (10000 - fee_bps)
        numerator = amount_with_fee * reserve_out
        denominator = reserve_in * 10000 + amount_with_fee
        
        return numerator // denominator if denominator > 0 else 0
    
    def get_gas_cost(self):
        """Get gas cost in USD"""
        try:
            gas_price_wei = w3.eth.gas_price
            gas_units = 280000  # Conservative estimate for V2 arbitrage
            gas_cost_matic = (gas_price_wei * gas_units) / 10**18
            matic_price = 0.40  # Conservative estimate
            return gas_cost_matic * matic_price
        except:
            return 2.0  # Conservative fallback
    
    def check_arbitrage_opportunities(self):
        """Check for arbitrage using proven logic from arb script"""
        current_time = time.time()
        
        # Only check every 5 seconds to avoid spam
        if current_time - self.last_opportunity_time < 5:
            return
            
        # Group pools by token pair
        pairs = defaultdict(list)
        
        for pool_addr, pool_data in self.pool_details.items():
            if pool_addr in self.pool_prices:
                # Update pool data with current price
                current_price = self.pool_prices[pool_addr]
                pool_data['current_price'] = current_price
                
                # Group by token pair
                pair_key = tuple(sorted([pool_data['token0'], pool_data['token1']]))
                pairs[pair_key].append(pool_data)
        
        # Check each pair for arbitrage
        for pair_key, pair_pools in pairs.items():
            if len(pair_pools) < 2:
                continue
                
            # Find best spread
            for i, p1 in enumerate(pair_pools):
                for j, p2 in enumerate(pair_pools):
                    if i >= j:
                        continue
                    
                    # Use current prices if available, otherwise reserves
                    price1 = p1.get('current_price', p1['price'])
                    price2 = p2.get('current_price', p2['price'])
                    
                    if price1 <= 0 or price2 <= 0:
                        continue
                    
                    spread = abs(price1 - price2) / min(price1, price2)
                    total_fees = (p1['fee_bps'] + p2['fee_bps']) / 10000
                    
                    if spread > total_fees + 0.001:  # 0.1% minimum net profit
                        # Determine buy/sell pools
                        if price1 < price2:
                            buy_pool, sell_pool = p1, p2
                        else:
                            buy_pool, sell_pool = p2, p1
                        
                        # Calculate optimal trade size using binary search (simplified)
                        max_input = min(
                            buy_pool['reserve1_raw'] * 0.05,  # 5% max of reserves
                            sell_pool['reserve0_raw'] * 0.05
                        )
                        
                        # Test a range of trade sizes
                        best_profit = 0
                        best_input = 0
                        
                        for input_mult in [0.01, 0.05, 0.1, 0.2]:  # Test different sizes
                            input_amount = int(max_input * input_mult)
                            if input_amount < 1000000:  # Minimum $1 trade (6 decimals)
                                continue
                            
                            # Calculate arbitrage profit
                            t0_out = self.calc_v2_output(
                                input_amount, 
                                buy_pool['reserve1_raw'], 
                                buy_pool['reserve0_raw'], 
                                buy_pool['fee_bps']
                            )
                            
                            t1_out = self.calc_v2_output(
                                t0_out,
                                sell_pool['reserve0_raw'],
                                sell_pool['reserve1_raw'],
                                sell_pool['fee_bps']
                            )
                            
                            profit_raw = t1_out - input_amount
                            if profit_raw > best_profit:
                                best_profit = profit_raw
                                best_input = input_amount
                        
                        if best_profit > 0:
                            profit_usd = best_profit / (10**buy_pool['decimals1'])
                            input_usd = best_input / (10**buy_pool['decimals1'])
                            gas_cost = self.get_gas_cost()
                            net_profit = profit_usd - gas_cost
                            
                            if net_profit > 0.01:  # Minimum $0.01 net profit
                                self.opportunities_found += 1
                                
                                print(f"\n💰 ARBITRAGE OPPORTUNITY #{self.opportunities_found}")
                                print(f"   Pair: {buy_pool['symbol0']}/{buy_pool['symbol1']}")
                                print(f"   Buy from: {buy_pool['dex']} ({buy_pool['address'][:8]}...)")
                                print(f"   Sell to: {sell_pool['dex']} ({sell_pool['address'][:8]}...)")
                                print(f"   Spread: {spread*100:.3f}%")
                                print(f"   Input: ${input_usd:.2f}")
                                print(f"   Gross profit: ${profit_usd:.4f}")
                                print(f"   Gas cost: ${gas_cost:.4f}")
                                print(f"   Net profit: ${net_profit:.4f}")
                                print(f"   ROI: {(net_profit/input_usd)*100:.3f}%")
        
        self.last_opportunity_time = current_time
    
    def process_trade_data(self, data):
        """Process incoming trade data and update pool prices"""
        try:
            # Extract symbol hash and price from trade message
            # This would need to be adapted based on the actual message format
            # For now, simulate with dummy data
            pass
        except Exception as e:
            print(f"Error processing trade: {e}")
    
    def connect_and_listen(self):
        """Connect to relay server and listen for trade data"""
        print(f"🔌 Connecting to relay server at {RELAY_SOCKET_PATH}")
        
        try:
            sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            sock.connect(RELAY_SOCKET_PATH)
            print("✅ Connected to relay server")
            
            buffer = b""
            
            while True:
                try:
                    data = sock.recv(4096)
                    if not data:
                        print("Connection closed by server")
                        break
                    
                    buffer += data
                    self.message_count += 1
                    
                    # For now, just count messages and check opportunities periodically
                    if self.message_count % 100 == 0:
                        print(f"📊 Processed {self.message_count} messages")
                        self.check_arbitrage_opportunities()
                        
                except KeyboardInterrupt:
                    print("\\n🛑 Stopping monitor...")
                    break
                except Exception as e:
                    print(f"Error reading data: {e}")
                    break
                    
        except FileNotFoundError:
            print(f"❌ Relay socket not found at {RELAY_SOCKET_PATH}")
            print("Make sure the relay server is running")
        except Exception as e:
            print(f"❌ Connection failed: {e}")
        finally:
            try:
                sock.close()
            except:
                pass

def main():
    monitor = SimpleArbitrageMonitor()
    
    try:
        monitor.connect_and_listen()
    except KeyboardInterrupt:
        print("\\n\\n🛑 Monitor stopped")
        print(f"Total messages processed: {monitor.message_count}")
        print(f"Opportunities found: {monitor.opportunities_found}")

if __name__ == "__main__":
    main()
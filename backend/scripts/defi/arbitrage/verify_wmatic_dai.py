#!/usr/bin/env python3
"""
Analyze WMATIC/DAI Arbitrage Opportunity
Includes detailed tick analysis for V3 pools
"""

import json
import sys
from web3 import Web3
from decimal import Decimal, getcontext
import os
from dotenv import load_dotenv
import math

# Set high precision
getcontext().prec = 50

# Load environment
load_dotenv('../.env')
ANKR_KEY = os.getenv('ANKR_API_KEY', '')
RPC_URL = f"https://rpc.ankr.com/polygon/{ANKR_KEY}" if ANKR_KEY else "https://polygon.publicnode.com"
w3 = Web3(Web3.HTTPProvider(RPC_URL))

# Pool addresses (checksummed)
BUY_POOL = "0xd32f3139A214034A0f9777c87eE0a064c1FF6AE2"
SELL_POOL = "0x0f663c16Dd7C65cF87eDB9229464cA77aEea536b"

# Token addresses
WMATIC = "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270"
DAI = "0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063"

# ERC20 ABI
ERC20_ABI = json.loads('''[
    {"inputs": [], "name": "symbol", "outputs": [{"name": "", "type": "string"}], "type": "function"},
    {"inputs": [], "name": "decimals", "outputs": [{"name": "", "type": "uint8"}], "type": "function"},
    {"inputs": [{"name": "account", "type": "address"}], "name": "balanceOf", "outputs": [{"name": "", "type": "uint256"}], "type": "function"}
]''')

# V3 Pool ABI
V3_POOL_ABI = json.loads('''[
    {
        "inputs": [],
        "name": "slot0",
        "outputs": [
            {"name": "sqrtPriceX96", "type": "uint160"},
            {"name": "tick", "type": "int24"},
            {"name": "observationIndex", "type": "uint16"},
            {"name": "observationCardinality", "type": "uint16"},
            {"name": "observationCardinalityNext", "type": "uint16"},
            {"name": "feeProtocol", "type": "uint8"},
            {"name": "unlocked", "type": "bool"}
        ],
        "type": "function"
    },
    {"inputs": [], "name": "liquidity", "outputs": [{"name": "", "type": "uint128"}], "type": "function"},
    {"inputs": [], "name": "fee", "outputs": [{"name": "", "type": "uint24"}], "type": "function"},
    {"inputs": [], "name": "token0", "outputs": [{"name": "", "type": "address"}], "type": "function"},
    {"inputs": [], "name": "token1", "outputs": [{"name": "", "type": "address"}], "type": "function"},
    {"inputs": [], "name": "tickSpacing", "outputs": [{"name": "", "type": "int24"}], "type": "function"}
]''')

# V2 Pool ABI
V2_POOL_ABI = json.loads('''[
    {
        "inputs": [],
        "name": "getReserves",
        "outputs": [
            {"name": "_reserve0", "type": "uint112"},
            {"name": "_reserve1", "type": "uint112"},
            {"name": "_blockTimestampLast", "type": "uint32"}
        ],
        "type": "function"
    },
    {"inputs": [], "name": "token0", "outputs": [{"name": "", "type": "address"}], "type": "function"},
    {"inputs": [], "name": "token1", "outputs": [{"name": "", "type": "address"}], "type": "function"}
]''')

def tick_to_price(tick):
    """Convert tick to actual price"""
    return 1.0001 ** tick

def sqrt_price_x96_to_price(sqrt_price_x96, decimals0, decimals1):
    """Convert sqrtPriceX96 to human readable price"""
    price = (float(sqrt_price_x96) / (2**96)) ** 2
    # Adjust for decimals
    decimal_adjustment = 10 ** (decimals1 - decimals0)
    return price * decimal_adjustment

def analyze_v3_pool(pool_address, name):
    """Detailed V3 pool analysis"""
    print(f"\n{'='*70}")
    print(f"{name}: {pool_address}")
    print('='*70)
    
    try:
        pool = w3.eth.contract(address=Web3.to_checksum_address(pool_address), abi=V3_POOL_ABI)
        
        # Get basic info
        token0 = pool.functions.token0().call()
        token1 = pool.functions.token1().call()
        fee = pool.functions.fee().call()
        tick_spacing = pool.functions.tickSpacing().call()
        
        # Get slot0 data
        slot0 = pool.functions.slot0().call()
        sqrt_price_x96 = slot0[0]
        current_tick = slot0[1]
        
        # Get liquidity
        liquidity = pool.functions.liquidity().call()
        
        # Get token info
        token0_contract = w3.eth.contract(address=Web3.to_checksum_address(token0), abi=ERC20_ABI)
        token1_contract = w3.eth.contract(address=Web3.to_checksum_address(token1), abi=ERC20_ABI)
        
        symbol0 = token0_contract.functions.symbol().call()
        symbol1 = token1_contract.functions.symbol().call()
        decimals0 = token0_contract.functions.decimals().call()
        decimals1 = token1_contract.functions.decimals().call()
        
        # Get pool balances
        balance0 = token0_contract.functions.balanceOf(pool_address).call()
        balance1 = token1_contract.functions.balanceOf(pool_address).call()
        
        balance0_human = balance0 / (10 ** decimals0)
        balance1_human = balance1 / (10 ** decimals1)
        
        # Calculate prices
        tick_price = tick_to_price(current_tick)
        actual_price = sqrt_price_x96_to_price(sqrt_price_x96, decimals0, decimals1)
        
        print("✅ Uniswap V3 Pool")
        print(f"\n📊 Pool Configuration:")
        print(f"  Token0: {symbol0} ({token0[:10]}...)")
        print(f"  Token1: {symbol1} ({token1[:10]}...)")
        print(f"  Fee Tier: {fee/10000:.2f}% ({fee} bps)")
        print(f"  Tick Spacing: {tick_spacing}")
        
        print(f"\n💧 Liquidity:")
        print(f"  Active Liquidity: {liquidity:,}")
        print(f"  Pool Balance {symbol0}: {balance0_human:,.6f}")
        print(f"  Pool Balance {symbol1}: {balance1_human:,.6f}")
        print(f"  Pool Value (est): ${(balance1_human * 2):,.2f}") # Assuming DAI = $1
        
        print(f"\n📈 Price Information:")
        print(f"  Current Tick: {current_tick}")
        print(f"  Tick Price (1.0001^tick): {tick_price:.10f}")
        print(f"  Actual Price ({symbol1}/{symbol0}): {actual_price:.10f}")
        print(f"  Price ({symbol0}/{symbol1}): {1/actual_price:.10f}")
        
        # Tick analysis
        print(f"\n🎯 Tick Analysis:")
        if current_tick > 0:
            print(f"  Tick is POSITIVE: {symbol1} is worth more than {symbol0}")
        elif current_tick < 0:
            print(f"  Tick is NEGATIVE: {symbol0} is worth more than {symbol1}")
        else:
            print(f"  Tick is ZERO: Tokens are at 1:1 ratio")
        
        tick_percentage = (tick_price - 1) * 100
        print(f"  Price deviation from 1:1 = {tick_percentage:.4f}%")
        
        # Liquidity depth analysis
        if liquidity > 0:
            # Rough estimate of max trade size before significant slippage
            estimated_max_trade = min(balance0_human * 0.01, balance1_human * 0.01)
            print(f"\n💰 Liquidity Depth:")
            print(f"  Estimated max trade (1% of pool): ${estimated_max_trade:,.2f}")
            
        return {
            'type': 'V3',
            'token0': token0,
            'token1': token1,
            'symbol0': symbol0,
            'symbol1': symbol1,
            'tick': current_tick,
            'liquidity': liquidity,
            'price': actual_price,
            'balance0': balance0_human,
            'balance1': balance1_human,
            'fee': fee/10000
        }
        
    except Exception as e:
        print(f"❌ Error analyzing V3 pool: {e}")
        return None

def analyze_v2_pool(pool_address, name):
    """Analyze V2 pool"""
    print(f"\n{'='*70}")
    print(f"{name}: {pool_address}")
    print('='*70)
    
    try:
        pool = w3.eth.contract(address=Web3.to_checksum_address(pool_address), abi=V2_POOL_ABI)
        
        token0 = pool.functions.token0().call()
        token1 = pool.functions.token1().call()
        reserves = pool.functions.getReserves().call()
        
        # Get token info
        token0_contract = w3.eth.contract(address=Web3.to_checksum_address(token0), abi=ERC20_ABI)
        token1_contract = w3.eth.contract(address=Web3.to_checksum_address(token1), abi=ERC20_ABI)
        
        symbol0 = token0_contract.functions.symbol().call()
        symbol1 = token1_contract.functions.symbol().call()
        decimals0 = token0_contract.functions.decimals().call()
        decimals1 = token1_contract.functions.decimals().call()
        
        reserve0_human = reserves[0] / (10 ** decimals0)
        reserve1_human = reserves[1] / (10 ** decimals1)
        
        price = reserve1_human / reserve0_human if reserve0_human > 0 else 0
        
        print("✅ Uniswap V2 Pool")
        print(f"\n📊 Pool Info:")
        print(f"  Token0: {symbol0} ({token0[:10]}...)")
        print(f"  Token1: {symbol1} ({token1[:10]}...)")
        print(f"  Reserve {symbol0}: {reserve0_human:,.6f}")
        print(f"  Reserve {symbol1}: {reserve1_human:,.6f}")
        print(f"  Price ({symbol1}/{symbol0}): {price:.10f}")
        print(f"  Pool Value (est): ${(reserve1_human * 2):,.2f}")
        
        return {
            'type': 'V2',
            'token0': token0,
            'token1': token1,
            'symbol0': symbol0,
            'symbol1': symbol1,
            'reserve0': reserve0_human,
            'reserve1': reserve1_human,
            'price': price,
            'fee': 0.3
        }
        
    except Exception as e:
        print(f"❌ Not a V2 pool or error: {e}")
        return None

def identify_pool_type(pool_address):
    """Quick check of pool type"""
    try:
        pool = w3.eth.contract(address=Web3.to_checksum_address(pool_address), abi=V3_POOL_ABI)
        pool.functions.slot0().call()
        return 'V3'
    except:
        try:
            pool = w3.eth.contract(address=Web3.to_checksum_address(pool_address), abi=V2_POOL_ABI)
            pool.functions.getReserves().call()
            return 'V2'
        except:
            return 'Unknown'

def main():
    print("="*70)
    print("WMATIC/DAI ARBITRAGE ANALYSIS")
    print("="*70)
    print(f"\n✅ Connected to Polygon (Block: {w3.eth.block_number})")
    
    # Quick type check
    buy_type = identify_pool_type(BUY_POOL)
    sell_type = identify_pool_type(SELL_POOL)
    
    print(f"\n🔍 Pool Types Detected:")
    print(f"  Buy Pool: {buy_type}")
    print(f"  Sell Pool: {sell_type}")
    
    # Analyze pools
    buy_pool = None
    sell_pool = None
    
    if buy_type == 'V3':
        buy_pool = analyze_v3_pool(BUY_POOL, "BUY POOL (V3)")
    elif buy_type == 'V2':
        buy_pool = analyze_v2_pool(BUY_POOL, "BUY POOL (V2)")
    
    if sell_type == 'V3':
        sell_pool = analyze_v3_pool(SELL_POOL, "SELL POOL (V3)")
    elif sell_type == 'V2':
        sell_pool = analyze_v2_pool(SELL_POOL, "SELL POOL (V2)")
    
    # Arbitrage analysis
    print("\n" + "="*70)
    print("ARBITRAGE ANALYSIS")
    print("="*70)
    
    if buy_pool and sell_pool:
        # Check if it's actually WMATIC/DAI
        wmatic_address = WMATIC.lower()
        dai_address = DAI.lower()
        
        buy_has_pair = (wmatic_address in [buy_pool['token0'], buy_pool['token1']] and 
                        dai_address in [buy_pool['token0'], buy_pool['token1']])
        sell_has_pair = (wmatic_address in [sell_pool['token0'], sell_pool['token1']] and 
                         dai_address in [sell_pool['token0'], sell_pool['token1']])
        
        if not buy_has_pair or not sell_has_pair:
            print("⚠️  WARNING: Pools don't contain expected WMATIC/DAI pair!")
            
        # Get prices in consistent format (DAI per WMATIC)
        if buy_pool['token0'] == wmatic_address:
            buy_price = buy_pool['price']  # DAI/WMATIC
        else:
            buy_price = 1 / buy_pool['price']  # Convert to DAI/WMATIC
            
        if sell_pool['token0'] == wmatic_address:
            sell_price = sell_pool['price']  # DAI/WMATIC
        else:
            sell_price = 1 / sell_pool['price']  # Convert to DAI/WMATIC
        
        price_diff_pct = ((sell_price - buy_price) / buy_price) * 100
        
        print(f"\n💹 Price Comparison (DAI per WMATIC):")
        print(f"  Buy Pool Price: {buy_price:.6f} DAI/WMATIC")
        print(f"  Sell Pool Price: {sell_price:.6f} DAI/WMATIC")
        print(f"  Price Difference: {price_diff_pct:.4f}%")
        
        # Liquidity analysis
        print(f"\n💧 Liquidity Analysis:")
        
        if buy_pool['type'] == 'V3':
            print(f"  Buy Pool: Active liquidity {buy_pool['liquidity']:,}")
            if buy_pool['liquidity'] < 1000000:
                print("    ⚠️  LOW LIQUIDITY - High slippage expected")
        else:
            buy_value = buy_pool.get('reserve1', 0) * 2  # DAI reserves * 2
            print(f"  Buy Pool Value: ${buy_value:,.2f}")
            if buy_value < 1000:
                print("    ⚠️  LOW LIQUIDITY - High slippage expected")
        
        if sell_pool['type'] == 'V3':
            print(f"  Sell Pool: Active liquidity {sell_pool['liquidity']:,}")
            if sell_pool['liquidity'] < 1000000:
                print("    ⚠️  LOW LIQUIDITY - High slippage expected")
        else:
            sell_value = sell_pool.get('reserve1', 0) * 2
            print(f"  Sell Pool Value: ${sell_value:,.2f}")
            if sell_value < 1000:
                print("    ⚠️  LOW LIQUIDITY - High slippage expected")
        
        # Fee analysis
        total_fees = buy_pool['fee'] + sell_pool['fee']
        print(f"\n💸 Fee Analysis:")
        print(f"  Buy Pool Fee: {buy_pool['fee']:.2f}%")
        print(f"  Sell Pool Fee: {sell_pool['fee']:.2f}%")
        print(f"  Total Fees: {total_fees:.2f}%")
        print(f"  Minimum profit needed: {total_fees:.2f}%")
        
        # Verdict
        print(f"\n🎯 VERDICT:")
        if price_diff_pct > total_fees + 0.1:  # 0.1% for gas
            print(f"  ✅ Potentially profitable (spread {price_diff_pct:.2f}% > fees {total_fees:.2f}%)")
        else:
            print(f"  ❌ Not profitable (spread {price_diff_pct:.2f}% < fees {total_fees:.2f}%)")
            
        # Why small trade size
        print(f"\n📉 Small Trade Size ($0.239) Explained:")
        print("  The tiny trade size indicates one or both pools have:")
        print("  1. Extremely low liquidity")
        print("  2. Concentrated liquidity far from current price (V3)")
        print("  3. Nearly depleted reserves (V2)")

if __name__ == "__main__":
    main()
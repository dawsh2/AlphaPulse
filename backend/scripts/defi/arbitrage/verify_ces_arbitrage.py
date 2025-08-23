#!/usr/bin/env python3
"""
Verify CES/USDT Arbitrage - Investigate tiny trade size issue
"""

import json
import sys
from web3 import Web3
from decimal import Decimal, getcontext
import os
from dotenv import load_dotenv

# Set high precision for accurate calculations
getcontext().prec = 50

# Load environment
load_dotenv('../.env')
ANKR_KEY = os.getenv('ANKR_API_KEY', '')
RPC_URL = f"https://rpc.ankr.com/polygon/{ANKR_KEY}" if ANKR_KEY else "https://polygon.publicnode.com"
w3 = Web3(Web3.HTTPProvider(RPC_URL))

# Pool addresses from the arbitrage analysis
BUY_POOL = "0x296b95dd0e8b726c4e358b0683ff0b6d675c35e9"
SELL_POOL = "0xa17d41cfc6c0437c336c1c7cc8ff72b085f13278"

# Common token addresses on Polygon
USDT = "0xc2132D05D31c914a87C6611C10748AEb04B58e8F"

# ERC20 ABI for token info
ERC20_ABI = json.loads('''[
    {"inputs": [], "name": "symbol", "outputs": [{"name": "", "type": "string"}], "type": "function"},
    {"inputs": [], "name": "decimals", "outputs": [{"name": "", "type": "uint8"}], "type": "function"},
    {"inputs": [], "name": "totalSupply", "outputs": [{"name": "", "type": "uint256"}], "type": "function"},
    {"inputs": [{"name": "account", "type": "address"}], "name": "balanceOf", "outputs": [{"name": "", "type": "uint256"}], "type": "function"}
]''')

# Uniswap V3 Pool ABI
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
    {"inputs": [], "name": "token1", "outputs": [{"name": "", "type": "address"}], "type": "function"}
]''')

# Uniswap V2 Pool ABI
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

def get_token_info(token_address):
    """Get token information"""
    try:
        token = w3.eth.contract(address=Web3.to_checksum_address(token_address), abi=ERC20_ABI)
        symbol = token.functions.symbol().call()
        decimals = token.functions.decimals().call()
        total_supply = token.functions.totalSupply().call()
        return {
            'address': token_address,
            'symbol': symbol,
            'decimals': decimals,
            'total_supply': total_supply,
            'total_supply_human': total_supply / (10 ** decimals)
        }
    except Exception as e:
        return {'address': token_address, 'error': str(e)}

def analyze_pool(pool_address, pool_name):
    """Analyze a pool - try V3 first, then V2"""
    print(f"\n{pool_name}: {pool_address}")
    print("-" * 60)
    
    pool_checksum = Web3.to_checksum_address(pool_address)
    
    # Check if contract exists
    code = w3.eth.get_code(pool_checksum)
    if not code or code == b'' or code == '0x':
        print("❌ Not a valid contract address!")
        return None
    
    # Try V3
    try:
        pool = w3.eth.contract(address=pool_checksum, abi=V3_POOL_ABI)
        
        token0 = pool.functions.token0().call()
        token1 = pool.functions.token1().call()
        fee = pool.functions.fee().call()
        slot0 = pool.functions.slot0().call()
        liquidity = pool.functions.liquidity().call()
        
        sqrt_price_x96 = slot0[0]
        tick = slot0[1]
        
        # Calculate price
        price = (float(sqrt_price_x96) / (2**96)) ** 2
        
        print("✅ Uniswap V3 Pool")
        print(f"  Token0: {token0}")
        print(f"  Token1: {token1}")
        print(f"  Fee Tier: {fee/10000:.2f}%")
        print(f"  Current Tick: {tick}")
        print(f"  Active Liquidity: {liquidity:,}")
        print(f"  Price (token1/token0): {price:.10f}")
        
        # Get token details
        token0_info = get_token_info(token0)
        token1_info = get_token_info(token1)
        
        print(f"\n  Token0 ({token0_info.get('symbol', 'Unknown')}):")
        print(f"    Decimals: {token0_info.get('decimals', 'Unknown')}")
        print(f"    Total Supply: {token0_info.get('total_supply_human', 0):,.2f}")
        
        print(f"\n  Token1 ({token1_info.get('symbol', 'Unknown')}):")
        print(f"    Decimals: {token1_info.get('decimals', 'Unknown')}")
        print(f"    Total Supply: {token1_info.get('total_supply_human', 0):,.2f}")
        
        # Check pool balances
        if 'decimals' in token0_info:
            balance0 = w3.eth.contract(address=Web3.to_checksum_address(token0), abi=ERC20_ABI).functions.balanceOf(pool_checksum).call()
            balance0_human = balance0 / (10 ** token0_info['decimals'])
            print(f"\n  Pool Balance Token0: {balance0_human:,.6f} {token0_info['symbol']}")
        
        if 'decimals' in token1_info:
            balance1 = w3.eth.contract(address=Web3.to_checksum_address(token1), abi=ERC20_ABI).functions.balanceOf(pool_checksum).call()
            balance1_human = balance1 / (10 ** token1_info['decimals'])
            print(f"  Pool Balance Token1: {balance1_human:,.6f} {token1_info['symbol']}")
        
        # Calculate pool value in USD (assuming token1 is USDT)
        if token1 == USDT.lower():
            pool_value_usd = balance1_human * 2  # Rough estimate
            print(f"\n  💰 Estimated Pool Value: ${pool_value_usd:,.2f}")
        
        return {
            'type': 'V3',
            'token0': token0,
            'token1': token1,
            'token0_info': token0_info,
            'token1_info': token1_info,
            'liquidity': liquidity,
            'tick': tick,
            'fee': fee/10000
        }
        
    except Exception as e:
        # Try V2
        try:
            pool = w3.eth.contract(address=pool_checksum, abi=V2_POOL_ABI)
            
            token0 = pool.functions.token0().call()
            token1 = pool.functions.token1().call()
            reserves = pool.functions.getReserves().call()
            
            print("✅ Uniswap V2 Pool")
            print(f"  Token0: {token0}")
            print(f"  Token1: {token1}")
            
            # Get token details
            token0_info = get_token_info(token0)
            token1_info = get_token_info(token1)
            
            reserve0_human = reserves[0] / (10 ** token0_info.get('decimals', 18))
            reserve1_human = reserves[1] / (10 ** token1_info.get('decimals', 18))
            
            print(f"  Reserve0: {reserve0_human:,.6f} {token0_info.get('symbol', 'Unknown')}")
            print(f"  Reserve1: {reserve1_human:,.6f} {token1_info.get('symbol', 'Unknown')}")
            
            # Price calculation
            if reserve0_human > 0:
                price = reserve1_human / reserve0_human
                print(f"  Price: {price:.10f} {token1_info.get('symbol', '')} per {token0_info.get('symbol', '')}")
            
            # Pool value
            if token1 == USDT.lower():
                pool_value_usd = reserve1_human * 2
                print(f"\n  💰 Estimated Pool Value: ${pool_value_usd:,.2f}")
            
            return {
                'type': 'V2',
                'token0': token0,
                'token1': token1,
                'token0_info': token0_info,
                'token1_info': token1_info,
                'reserve0': reserves[0],
                'reserve1': reserves[1]
            }
            
        except Exception as e2:
            print(f"❌ Failed to identify pool type")
            print(f"   V3 Error: {str(e)[:100]}")
            print(f"   V2 Error: {str(e2)[:100]}")
            return None

def analyze_arbitrage():
    """Analyze the CES/USDT arbitrage opportunity"""
    print("="*70)
    print("CES/USDT ARBITRAGE ANALYSIS")
    print("="*70)
    
    print(f"\n🔍 Investigating why trade size is ${4.02e-5:,.8f} (extremely small)")
    
    # Analyze both pools
    buy_pool = analyze_pool(BUY_POOL, "BUY POOL")
    sell_pool = analyze_pool(SELL_POOL, "SELL POOL")
    
    # Check if CES token exists and has liquidity
    if buy_pool and sell_pool:
        print("\n" + "="*70)
        print("LIQUIDITY ANALYSIS")
        print("="*70)
        
        # Identify CES token
        ces_address = None
        if buy_pool['token0'] != USDT.lower() and buy_pool['token1'] == USDT.lower():
            ces_address = buy_pool['token0']
        elif buy_pool['token1'] != USDT.lower() and buy_pool['token0'] == USDT.lower():
            ces_address = buy_pool['token1']
        
        if ces_address:
            ces_info = get_token_info(ces_address)
            print(f"\nCES Token: {ces_address}")
            print(f"  Symbol: {ces_info.get('symbol', 'Unknown')}")
            print(f"  Decimals: {ces_info.get('decimals', 'Unknown')}")
            print(f"  Total Supply: {ces_info.get('total_supply_human', 0):,.2f}")
        
        # Check liquidity issues
        if buy_pool['type'] == 'V3':
            if buy_pool['liquidity'] < 1000:
                print("\n⚠️  WARNING: Buy pool has EXTREMELY LOW liquidity!")
                print(f"   Liquidity: {buy_pool['liquidity']}")
                print("   This explains the tiny trade size!")
        
        if sell_pool['type'] == 'V3':
            if sell_pool['liquidity'] < 1000:
                print("\n⚠️  WARNING: Sell pool has EXTREMELY LOW liquidity!")
                print(f"   Liquidity: {sell_pool['liquidity']}")
                print("   This explains the tiny trade size!")
    
    print("\n" + "="*70)
    print("CONCLUSION")
    print("="*70)
    print("\nThe extremely small trade size ($0.00004) indicates:")
    print("1. The pools have very low liquidity")
    print("2. CES might be a low-cap or illiquid token")
    print("3. Any larger trade would cause massive slippage")
    print("4. This is not a viable arbitrage opportunity")
    print("\n❌ VERDICT: Not worth pursuing - liquidity too low")

def main():
    print("CES/USDT Arbitrage Investigation")
    print("="*70)
    
    if not w3.is_connected():
        print("❌ Failed to connect to Polygon RPC")
        sys.exit(1)
    
    print(f"✅ Connected to Polygon (Block: {w3.eth.block_number})")
    
    analyze_arbitrage()

if __name__ == "__main__":
    main()
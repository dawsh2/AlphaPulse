#!/usr/bin/env python3
"""
Verify Arbitrage Opportunity - Supports both Uniswap V2 and V3 pools
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

# Token addresses on Polygon
USDC_E = "0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359"  # USDC.e (native)
USDT = "0xc2132D05D31c914a87C6611C10748AEb04B58e8F"    # USDT

# Pool addresses from the arbitrage analysis
BUY_POOL = "0x1cb770fc7c7367c6ad5e88d32072b7f4bf881304"
SELL_POOL = "0x498d5cdcc5667b21210b49442bf2d8792527194d"

# Uniswap V3 Pool ABI (minimal)
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
    {
        "inputs": [],
        "name": "liquidity",
        "outputs": [{"name": "", "type": "uint128"}],
        "type": "function"
    },
    {
        "inputs": [],
        "name": "fee",
        "outputs": [{"name": "", "type": "uint24"}],
        "type": "function"
    },
    {
        "inputs": [],
        "name": "token0",
        "outputs": [{"name": "", "type": "address"}],
        "type": "function"
    },
    {
        "inputs": [],
        "name": "token1",
        "outputs": [{"name": "", "type": "address"}],
        "type": "function"
    }
]''')

# Uniswap V2 Pool ABI (minimal)
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
    {
        "inputs": [],
        "name": "token0",
        "outputs": [{"name": "", "type": "address"}],
        "type": "function"
    },
    {
        "inputs": [],
        "name": "token1",
        "outputs": [{"name": "", "type": "address"}],
        "type": "function"
    }
]''')

def identify_pool_type(pool_address):
    """Identify if pool is V2 or V3 by checking for characteristic functions"""
    pool_checksum = Web3.to_checksum_address(pool_address)
    
    # Check for V3 slot0 function
    try:
        v3_pool = w3.eth.contract(address=pool_checksum, abi=V3_POOL_ABI)
        slot0 = v3_pool.functions.slot0().call()
        return "V3"
    except:
        pass
    
    # Check for V2 getReserves function
    try:
        v2_pool = w3.eth.contract(address=pool_checksum, abi=V2_POOL_ABI)
        reserves = v2_pool.functions.getReserves().call()
        return "V2"
    except:
        pass
    
    return "Unknown"

def get_v3_pool_info(pool_address):
    """Get Uniswap V3 pool information"""
    try:
        pool = w3.eth.contract(address=Web3.to_checksum_address(pool_address), abi=V3_POOL_ABI)
        
        token0 = pool.functions.token0().call()
        token1 = pool.functions.token1().call()
        fee = pool.functions.fee().call()  # Fee in 0.01% units (e.g., 3000 = 0.3%)
        slot0 = pool.functions.slot0().call()
        liquidity = pool.functions.liquidity().call()
        
        sqrt_price_x96 = slot0[0]
        
        # Calculate price from sqrtPriceX96
        # price = (sqrtPriceX96 / 2^96)^2
        price_ratio = (Decimal(sqrt_price_x96) / Decimal(2**96)) ** 2
        
        return {
            'address': pool_address,
            'type': 'V3',
            'token0': token0.lower(),
            'token1': token1.lower(),
            'fee': fee / 10000,  # Convert to percentage
            'sqrt_price_x96': sqrt_price_x96,
            'liquidity': liquidity,
            'price_ratio': float(price_ratio),
            'tick': slot0[1]
        }
    except Exception as e:
        print(f"  ⚠️  Error reading V3 pool {pool_address}: {e}")
        return None

def get_v2_pool_info(pool_address):
    """Get Uniswap V2 pool information"""
    try:
        pool = w3.eth.contract(address=Web3.to_checksum_address(pool_address), abi=V2_POOL_ABI)
        
        token0 = pool.functions.token0().call()
        token1 = pool.functions.token1().call()
        reserves = pool.functions.getReserves().call()
        
        return {
            'address': pool_address,
            'type': 'V2',
            'token0': token0.lower(),
            'token1': token1.lower(),
            'reserve0': reserves[0],
            'reserve1': reserves[1],
            'fee': 0.3  # Standard 0.3% for V2
        }
    except Exception as e:
        print(f"  ⚠️  Error reading V2 pool {pool_address}: {e}")
        return None

def estimate_v3_swap(pool_info, amount_in, zero_for_one):
    """
    Rough estimation of V3 swap output
    This is a simplified calculation - real V3 swaps are much more complex
    """
    fee_pct = pool_info['fee'] / 100
    amount_after_fee = amount_in * (1 - fee_pct)
    
    # Very rough approximation using current price
    # In reality, V3 swaps traverse multiple ticks and liquidity positions
    if zero_for_one:
        # Swapping token0 for token1
        output = amount_after_fee * pool_info['price_ratio']
    else:
        # Swapping token1 for token0
        output = amount_after_fee / pool_info['price_ratio']
    
    # Apply estimated slippage based on liquidity
    # This is a very rough approximation
    liquidity_factor = min(1.0, pool_info['liquidity'] / 10**15)
    slippage_multiplier = 0.99 * liquidity_factor  # More liquidity = less slippage
    
    return output * slippage_multiplier

def calculate_v2_swap(reserve_in, reserve_out, amount_in, fee_pct=0.3):
    """Calculate V2 swap output using constant product formula"""
    amount_in = Decimal(str(amount_in))
    reserve_in = Decimal(str(reserve_in))
    reserve_out = Decimal(str(reserve_out))
    
    amount_with_fee = amount_in * (100 - fee_pct) / 100
    numerator = amount_with_fee * reserve_out
    denominator = reserve_in + amount_with_fee
    
    return float(numerator / denominator)

def analyze_pools():
    """Analyze the arbitrage pools"""
    print("\n" + "="*60)
    print("Pool Analysis")
    print("="*60)
    
    # Identify pool types
    print(f"\nBuy Pool: {BUY_POOL}")
    buy_type = identify_pool_type(BUY_POOL)
    print(f"  Type: {buy_type}")
    
    print(f"\nSell Pool: {SELL_POOL}")
    sell_type = identify_pool_type(SELL_POOL)
    print(f"  Type: {sell_type}")
    
    # Get pool info based on type
    buy_pool = None
    sell_pool = None
    
    if buy_type == "V3":
        buy_pool = get_v3_pool_info(BUY_POOL)
    elif buy_type == "V2":
        buy_pool = get_v2_pool_info(BUY_POOL)
    
    if sell_type == "V3":
        sell_pool = get_v3_pool_info(SELL_POOL)
    elif sell_type == "V2":
        sell_pool = get_v2_pool_info(SELL_POOL)
    
    if buy_pool:
        print(f"\nBuy Pool Details:")
        print(f"  Token0: {buy_pool['token0'][:10]}...")
        print(f"  Token1: {buy_pool['token1'][:10]}...")
        print(f"  Fee: {buy_pool.get('fee', 0.3)}%")
        if buy_pool['type'] == 'V3':
            print(f"  Liquidity: {buy_pool['liquidity']}")
            print(f"  Current Tick: {buy_pool['tick']}")
    
    if sell_pool:
        print(f"\nSell Pool Details:")
        print(f"  Token0: {sell_pool['token0'][:10]}...")
        print(f"  Token1: {sell_pool['token1'][:10]}...")
        print(f"  Fee: {sell_pool.get('fee', 0.3)}%")
        if sell_pool['type'] == 'V3':
            print(f"  Liquidity: {sell_pool['liquidity']}")
            print(f"  Current Tick: {sell_pool['tick']}")
    
    # Determine if tokens match
    if buy_pool and sell_pool:
        tokens_buy = {buy_pool['token0'], buy_pool['token1']}
        tokens_sell = {sell_pool['token0'], sell_pool['token1']}
        
        if USDC_E.lower() in tokens_buy and USDT.lower() in tokens_buy:
            print(f"\n✅ Buy pool contains USDC.e and USDT")
        else:
            print(f"\n⚠️  Buy pool tokens don't match expected USDC.e/USDT")
            
        if USDC_E.lower() in tokens_sell and USDT.lower() in tokens_sell:
            print(f"✅ Sell pool contains USDC.e and USDT")
        else:
            print(f"⚠️  Sell pool tokens don't match expected USDC.e/USDT")
    
    return buy_pool, sell_pool

def simulate_arbitrage(buy_pool, sell_pool, trade_size_usd):
    """Simulate the arbitrage trade"""
    print(f"\n" + "="*60)
    print(f"Arbitrage Simulation: ${trade_size_usd:,.2f}")
    print("="*60)
    
    if not buy_pool or not sell_pool:
        print("❌ Cannot simulate - pool data unavailable")
        return None
    
    # Convert USD to token units (6 decimals for both)
    amount_usdt = trade_size_usd * 10**6
    
    print(f"\n📊 Theoretical Analysis:")
    print(f"  Note: V3 calculations are rough estimates")
    print(f"  Actual execution would differ significantly")
    
    # Step 1: Buy USDC.e with USDT
    if buy_pool['type'] == 'V2':
        # Determine reserves
        if buy_pool['token0'] == USDT.lower():
            usdt_reserve = buy_pool['reserve0']
            usdc_reserve = buy_pool['reserve1']
        else:
            usdt_reserve = buy_pool['reserve1']
            usdc_reserve = buy_pool['reserve0']
        
        usdc_out = calculate_v2_swap(usdt_reserve, usdc_reserve, amount_usdt, buy_pool['fee'])
    else:  # V3
        zero_for_one = buy_pool['token0'] == USDT.lower()
        usdc_out = estimate_v3_swap(buy_pool, amount_usdt, zero_for_one)
    
    print(f"\nStep 1 - Buy USDC.e:")
    print(f"  Input: {amount_usdt/10**6:,.2f} USDT")
    print(f"  Est. Output: {usdc_out/10**6:,.2f} USDC.e")
    
    # Step 2: Sell USDC.e for USDT
    if sell_pool['type'] == 'V2':
        if sell_pool['token0'] == USDC_E.lower():
            usdc_reserve = sell_pool['reserve0']
            usdt_reserve = sell_pool['reserve1']
        else:
            usdc_reserve = sell_pool['reserve1']
            usdt_reserve = sell_pool['reserve0']
        
        usdt_out = calculate_v2_swap(usdc_reserve, usdt_reserve, usdc_out, sell_pool['fee'])
    else:  # V3
        zero_for_one = sell_pool['token0'] == USDC_E.lower()
        usdt_out = estimate_v3_swap(sell_pool, usdc_out, zero_for_one)
    
    print(f"\nStep 2 - Sell USDC.e:")
    print(f"  Input: {usdc_out/10**6:,.2f} USDC.e")
    print(f"  Est. Output: {usdt_out/10**6:,.2f} USDT")
    
    # Calculate profit
    gross_profit = (usdt_out - amount_usdt) / 10**6
    profit_pct = ((usdt_out / amount_usdt) - 1) * 100
    
    # Gas estimate
    gas_cost = 0.02  # Rough estimate for Polygon
    net_profit = gross_profit - gas_cost
    
    print(f"\n💰 Estimated Results:")
    print(f"  Gross Profit: ${gross_profit:,.2f} ({profit_pct:.2f}%)")
    print(f"  Gas Cost: ${gas_cost:.2f}")
    print(f"  Net Profit: ${net_profit:,.2f}")
    
    if net_profit > 0:
        print(f"\n✅ Potentially PROFITABLE")
    else:
        print(f"\n❌ Likely UNPROFITABLE")
    
    return net_profit

def main():
    print("Advanced Arbitrage Verification (V2 & V3 Support)")
    print("="*60)
    
    if not w3.is_connected():
        print("❌ Failed to connect to Polygon RPC")
        sys.exit(1)
    
    print(f"✅ Connected to Polygon (Block: {w3.eth.block_number})")
    
    # Analyze pools
    buy_pool, sell_pool = analyze_pools()
    
    # Simulate trades
    if buy_pool and sell_pool:
        trade_sizes = [2683, 5366, 10733]
        
        for size in trade_sizes:
            simulate_arbitrage(buy_pool, sell_pool, size)
    
    print(f"\n" + "="*60)
    print("⚠️  DISCLAIMER:")
    print("  - V3 calculations are rough approximations")
    print("  - Real execution requires complex tick math")
    print("  - Always simulate on-chain before executing")
    print("  - MEV bots may front-run profitable opportunities")

if __name__ == "__main__":
    main()
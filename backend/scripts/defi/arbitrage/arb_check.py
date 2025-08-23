#!/usr/bin/env python3
"""
Quick Arbitrage Checker - Just pass two pool addresses
Usage: python3 arb_check.py <pool1> <pool2>

Automatically determines buy/sell direction and calculates precise profit
"""

import json
import sys
from web3 import Web3
from decimal import Decimal, getcontext
import os
from dotenv import load_dotenv

# Set high precision
getcontext().prec = 50

# Load environment
load_dotenv('../.env')
ANKR_KEY = os.getenv('ANKR_API_KEY', '')
RPC_URL = f"https://rpc.ankr.com/polygon/{ANKR_KEY}" if ANKR_KEY else "https://polygon.publicnode.com"
w3 = Web3(Web3.HTTPProvider(RPC_URL))

# ABIs
ERC20_ABI = json.loads('[{"inputs":[],"name":"symbol","outputs":[{"name":"","type":"string"}],"type":"function"},{"inputs":[],"name":"decimals","outputs":[{"name":"","type":"uint8"}],"type":"function"},{"inputs":[{"name":"account","type":"address"}],"name":"balanceOf","outputs":[{"name":"","type":"uint256"}],"type":"function"}]')

V3_ABI = json.loads('[{"inputs":[],"name":"slot0","outputs":[{"name":"sqrtPriceX96","type":"uint160"},{"name":"tick","type":"int24"},{"name":"observationIndex","type":"uint16"},{"name":"observationCardinality","type":"uint16"},{"name":"observationCardinalityNext","type":"uint16"},{"name":"feeProtocol","type":"uint8"},{"name":"unlocked","type":"bool"}],"type":"function"},{"inputs":[],"name":"liquidity","outputs":[{"name":"","type":"uint128"}],"type":"function"},{"inputs":[],"name":"fee","outputs":[{"name":"","type":"uint24"}],"type":"function"},{"inputs":[],"name":"token0","outputs":[{"name":"","type":"address"}],"type":"function"},{"inputs":[],"name":"token1","outputs":[{"name":"","type":"address"}],"type":"function"}]')

V2_ABI = json.loads('[{"inputs":[],"name":"getReserves","outputs":[{"name":"_reserve0","type":"uint112"},{"name":"_reserve1","type":"uint112"},{"name":"_blockTimestampLast","type":"uint32"}],"type":"function"},{"inputs":[],"name":"token0","outputs":[{"name":"","type":"address"}],"type":"function"},{"inputs":[],"name":"token1","outputs":[{"name":"","type":"address"}],"type":"function"}]')

def get_pool_info(address):
    """Get pool info for V2 or V3"""
    pool_addr = Web3.to_checksum_address(address)
    
    # Try V3
    try:
        pool = w3.eth.contract(address=pool_addr, abi=V3_ABI)
        token0 = pool.functions.token0().call()
        token1 = pool.functions.token1().call()
        fee = pool.functions.fee().call() / 10000
        slot0 = pool.functions.slot0().call()
        liquidity = pool.functions.liquidity().call()
        
        # Get token info
        t0 = w3.eth.contract(address=Web3.to_checksum_address(token0), abi=ERC20_ABI)
        t1 = w3.eth.contract(address=Web3.to_checksum_address(token1), abi=ERC20_ABI)
        
        symbol0 = t0.functions.symbol().call()
        symbol1 = t1.functions.symbol().call()
        decimals0 = t0.functions.decimals().call()
        decimals1 = t1.functions.decimals().call()
        
        # Get balances
        balance0 = t0.functions.balanceOf(pool_addr).call() / (10**decimals0)
        balance1 = t1.functions.balanceOf(pool_addr).call() / (10**decimals1)
        
        # Calculate price from sqrtPriceX96
        sqrt_price_x96 = slot0[0]
        price_raw = (float(sqrt_price_x96) / (2**96)) ** 2
        price = price_raw * (10**(decimals1-decimals0))
        
        return {
            'type': 'V3',
            'address': address,
            'token0': token0,
            'token1': token1,
            'symbol0': symbol0,
            'symbol1': symbol1,
            'decimals0': decimals0,
            'decimals1': decimals1,
            'fee': fee,
            'liquidity': liquidity,
            'balance0': balance0,
            'balance1': balance1,
            'price': price,  # token1 per token0
            'tick': slot0[1],
            'sqrt_price_x96': sqrt_price_x96
        }
    except:
        pass
    
    # Try V2
    try:
        pool = w3.eth.contract(address=pool_addr, abi=V2_ABI)
        token0 = pool.functions.token0().call()
        token1 = pool.functions.token1().call()
        reserves = pool.functions.getReserves().call()
        
        # Get token info
        t0 = w3.eth.contract(address=Web3.to_checksum_address(token0), abi=ERC20_ABI)
        t1 = w3.eth.contract(address=Web3.to_checksum_address(token1), abi=ERC20_ABI)
        
        symbol0 = t0.functions.symbol().call()
        symbol1 = t1.functions.symbol().call()
        decimals0 = t0.functions.decimals().call()
        decimals1 = t1.functions.decimals().call()
        
        reserve0 = reserves[0] / (10**decimals0)
        reserve1 = reserves[1] / (10**decimals1)
        
        price = reserve1 / reserve0 if reserve0 > 0 else 0
        
        return {
            'type': 'V2',
            'address': address,
            'token0': token0,
            'token1': token1,
            'symbol0': symbol0,
            'symbol1': symbol1,
            'decimals0': decimals0,
            'decimals1': decimals1,
            'fee': 0.3,
            'reserve0': reserve0,
            'reserve1': reserve1,
            'price': price  # token1 per token0
        }
    except Exception as e:
        return None

def calculate_v2_output(amount_in, reserve_in, reserve_out, fee_percent=0.3):
    """Exact V2 AMM calculation"""
    amount_in = Decimal(str(amount_in))
    reserve_in = Decimal(str(reserve_in))
    reserve_out = Decimal(str(reserve_out))
    fee_multiplier = Decimal(str(1 - fee_percent/100))
    
    amount_with_fee = amount_in * fee_multiplier
    numerator = amount_with_fee * reserve_out
    denominator = reserve_in + amount_with_fee
    
    return float(numerator / denominator)

def calculate_v3_output_simple(amount_in, liquidity, sqrt_price_x96, fee_percent, zero_for_one, decimals0, decimals1):
    """
    Simplified V3 calculation assuming single tick
    For precise calculation, we'd need to traverse ticks
    """
    amount_in = Decimal(str(amount_in))
    fee_multiplier = Decimal(str(1 - fee_percent/100))
    amount_after_fee = amount_in * fee_multiplier
    
    # Convert sqrt_price to actual price
    sqrt_price = Decimal(sqrt_price_x96) / Decimal(2**96)
    
    if zero_for_one:
        # Selling token0 for token1
        # Simplified: assume constant liquidity in range
        delta_sqrt_price = amount_after_fee / Decimal(liquidity)
        new_sqrt_price = sqrt_price - delta_sqrt_price
        
        # Calculate output
        output = Decimal(liquidity) * (sqrt_price - new_sqrt_price)
        
        # Adjust for decimals
        output = output * Decimal(10**(decimals1-decimals0))
    else:
        # Selling token1 for token0
        delta_sqrt_price = amount_after_fee / Decimal(liquidity)
        new_sqrt_price = sqrt_price + delta_sqrt_price
        
        output = Decimal(liquidity) / sqrt_price - Decimal(liquidity) / new_sqrt_price
        
        # Adjust for decimals
        output = output * Decimal(10**(decimals0-decimals1))
    
    return abs(float(output))

def find_optimal_trade_v2(pool1, pool2, is_token0):
    """
    Find optimal trade size for V2 pools using calculus
    The optimal point is where marginal revenue = marginal cost
    """
    if is_token0:
        # Trading token0: pool1 sells token0, pool2 buys token0
        r1_in = pool1['reserve0']
        r1_out = pool1['reserve1'] 
        r2_in = pool2['reserve1']
        r2_out = pool2['reserve0']
    else:
        # Trading token1: pool1 sells token1, pool2 buys token1
        r1_in = pool1['reserve1']
        r1_out = pool1['reserve0']
        r2_in = pool2['reserve0']
        r2_out = pool2['reserve1']
    
    fee1 = 1 - pool1['fee']/100
    fee2 = 1 - pool2['fee']/100
    
    # Binary search for optimal trade size
    low = 0.001
    high = min(r1_in * 0.3, r2_out * 0.3)  # Max 30% of smaller pool
    
    best_profit = -float('inf')
    best_size = 0
    
    for _ in range(50):  # 50 iterations for precision
        mid = (low + high) / 2
        
        # Calculate profit at this trade size
        out1 = calculate_v2_output(mid, r1_in, r1_out, pool1['fee'])
        out2 = calculate_v2_output(out1, r2_in, r2_out, pool2['fee'])
        
        profit = out2 - mid
        
        if profit > best_profit:
            best_profit = profit
            best_size = mid
        
        # Test slightly higher and lower
        test_high = mid * 1.01
        out1_h = calculate_v2_output(test_high, r1_in, r1_out, pool1['fee'])
        out2_h = calculate_v2_output(out1_h, r2_in, r2_out, pool2['fee'])
        profit_h = out2_h - test_high
        
        if profit_h > profit:
            low = mid
        else:
            high = mid
        
        if high - low < 0.001:
            break
    
    return best_size, best_profit

def analyze_arbitrage(pool1_addr, pool2_addr):
    """Main arbitrage analysis"""
    print("\n" + "="*70)
    print("ARBITRAGE QUICK CHECK")
    print("="*70)
    
    # Get pool info
    pool1 = get_pool_info(pool1_addr)
    pool2 = get_pool_info(pool2_addr)
    
    if not pool1 or not pool2:
        print("❌ Failed to load one or both pools")
        return
    
    # Print pool summaries
    print(f"\n📊 Pool 1 ({pool1['type']}): {pool1['symbol0']}/{pool1['symbol1']}")
    print(f"   Price: {pool1['price']:.8f} {pool1['symbol1']}/{pool1['symbol0']}")
    print(f"   Fee: {pool1['fee']:.2f}%")
    if pool1['type'] == 'V2':
        print(f"   Reserves: {pool1['reserve0']:.2f} / {pool1['reserve1']:.2f}")
    else:
        print(f"   Liquidity: {pool1['liquidity']:,}")
        print(f"   Balances: {pool1['balance0']:.2f} / {pool1['balance1']:.2f}")
    
    print(f"\n📊 Pool 2 ({pool2['type']}): {pool2['symbol0']}/{pool2['symbol1']}")
    print(f"   Price: {pool2['price']:.8f} {pool2['symbol1']}/{pool2['symbol0']}")
    print(f"   Fee: {pool2['fee']:.2f}%")
    if pool2['type'] == 'V2':
        print(f"   Reserves: {pool2['reserve0']:.2f} / {pool2['reserve1']:.2f}")
    else:
        print(f"   Liquidity: {pool2['liquidity']:,}")
        print(f"   Balances: {pool2['balance0']:.2f} / {pool2['balance1']:.2f}")
    
    # Check if same pair
    if not (pool1['token0'] == pool2['token0'] and pool1['token1'] == pool2['token1']):
        print(f"\n⚠️  WARNING: Pools have different token pairs!")
        print(f"   Pool1: {pool1['symbol0']}/{pool1['symbol1']}")
        print(f"   Pool2: {pool2['symbol0']}/{pool2['symbol1']}")
        return
    
    # Determine arbitrage direction
    price_diff = abs(pool1['price'] - pool2['price'])
    price_diff_pct = (price_diff / min(pool1['price'], pool2['price'])) * 100
    
    print(f"\n💹 Price Analysis:")
    print(f"   Price difference: {price_diff_pct:.4f}%")
    print(f"   Total fees: {pool1['fee'] + pool2['fee']:.2f}%")
    
    if price_diff_pct < 0.01:
        print(f"   ❌ Prices too similar - no arbitrage opportunity")
        return
    
    # Determine direction - we want to buy where it's cheap and sell where it's expensive
    # Price is token1/token0, so:
    # - Higher price means token0 is expensive (more token1 per token0)
    # - Lower price means token0 is cheap (less token1 per token0)
    
    if pool1['price'] < pool2['price']:
        # Pool1 has cheaper token0 (lower token1/token0 ratio)
        # Buy token0 from Pool1, sell to Pool2
        buy_pool = pool1
        sell_pool = pool2
        direction = f"Buy {pool1['symbol0']} from Pool1, sell to Pool2"
        trade_token = 'token0'
    else:
        # Pool2 has cheaper token0
        buy_pool = pool2
        sell_pool = pool1  
        direction = f"Buy {pool1['symbol0']} from Pool2, sell to Pool1"
        trade_token = 'token0'
    
    print(f"\n🎯 Arbitrage Direction:")
    print(f"   {direction}")
    print(f"   Buy at: {buy_pool['price']:.8f}")
    print(f"   Sell at: {sell_pool['price']:.8f}")
    
    # Calculate optimal trade for V2 pools
    if pool1['type'] == 'V2' and pool2['type'] == 'V2':
        # For V2, we can calculate the optimal trade size precisely
        # We're buying token0 where it's cheap and selling where it's expensive
        
        # Start with token1, buy token0, sell token0 for token1
        # Optimal size calculation using binary search
        low = 0.01
        high = min(buy_pool['reserve1'] * 0.1, sell_pool['reserve0'] * 0.1)  # Max 10% of smaller side
        
        best_profit = -float('inf')
        best_size = 0
        best_path = {}
        
        for _ in range(100):  # More iterations for precision
            mid = (low + high) / 2
            
            # Path: token1 -> token0 -> token1
            token1_in = mid
            token0_out = calculate_v2_output(token1_in, buy_pool['reserve1'], buy_pool['reserve0'], buy_pool['fee'])
            token1_final = calculate_v2_output(token0_out, sell_pool['reserve0'], sell_pool['reserve1'], sell_pool['fee'])
            
            profit = token1_final - token1_in
            
            if profit > best_profit:
                best_profit = profit
                best_size = token1_in
                best_path = {
                    'token1_in': token1_in,
                    'token0_out': token0_out,
                    'token1_final': token1_final
                }
            
            # Check derivative to find direction
            test_higher = mid * 1.001
            t0_h = calculate_v2_output(test_higher, buy_pool['reserve1'], buy_pool['reserve0'], buy_pool['fee'])
            t1_h = calculate_v2_output(t0_h, sell_pool['reserve0'], sell_pool['reserve1'], sell_pool['fee'])
            profit_h = t1_h - test_higher
            
            if profit_h > profit:
                low = mid
            else:
                high = mid
            
            if high - low < 0.0001:
                break
        
        if best_path:
            profit = best_path['token1_final'] - best_path['token1_in']
            profit_pct = (profit / best_path['token1_in']) * 100
            
            print(f"\n💰 Optimal Trade (Mathematical Precision):")
            print(f"   Input: {best_path['token1_in']:.6f} {buy_pool['symbol1']}")
            print(f"   → Get: {best_path['token0_out']:.6f} {buy_pool['symbol0']}")
            print(f"   → Sell for: {best_path['token1_final']:.6f} {sell_pool['symbol1']}")
            print(f"   Gross Profit: {profit:.6f} {buy_pool['symbol1']} ({profit_pct:.2f}%)")
            
            # Gas cost estimate
            gas_cost_usd = 0.004  # Typical for 2 swaps on Polygon
            
            # Assuming token1 is a stablecoin or calculate USD value
            profit_usd = profit  # Simplified - assumes token1 ≈ $1
            net_profit_usd = profit_usd - gas_cost_usd
            
            print(f"\n📊 Profitability:")
            print(f"   Gross: ${profit_usd:.6f}")
            print(f"   Gas: ${gas_cost_usd:.6f}")
            print(f"   Net: ${net_profit_usd:.6f}")
            
            if net_profit_usd > 0:
                print(f"   ✅ PROFITABLE!")
            else:
                print(f"   ❌ Not profitable after gas")
    
    else:
        print(f"\n⚠️  V3 pools require complex tick math for precise calculation")
        print(f"   Using estimation only...")
        
        # Simple estimation for mixed or V3 pools
        test_amount = 100  # Test with $100
        
        if buy_pool['type'] == 'V2':
            out1 = calculate_v2_output(test_amount, buy_pool['reserve1'], buy_pool['reserve0'], buy_pool['fee'])
        else:
            # Rough V3 estimate
            out1 = test_amount / buy_pool['price'] * (1 - buy_pool['fee']/100)
        
        if sell_pool['type'] == 'V2':
            out2 = calculate_v2_output(out1, sell_pool['reserve0'], sell_pool['reserve1'], sell_pool['fee'])
        else:
            # Rough V3 estimate
            out2 = out1 * sell_pool['price'] * (1 - sell_pool['fee']/100)
        
        profit = out2 - test_amount
        profit_pct = (profit / test_amount) * 100
        
        print(f"\n💰 Rough Estimate (${test_amount} test):")
        print(f"   Return: ${out2:.2f}")
        print(f"   Profit: ${profit:.2f} ({profit_pct:.2f}%)")
        
        if profit > 0.01:
            print(f"   ✅ Potentially profitable")
        else:
            print(f"   ❌ Not profitable")

def main():
    if len(sys.argv) != 3:
        print("Usage: python3 arb_check.py <pool1_address> <pool2_address>")
        print("\nExample:")
        print("python3 arb_check.py 0x123... 0x456...")
        sys.exit(1)
    
    pool1 = sys.argv[1]
    pool2 = sys.argv[2]
    
    # Auto-checksum addresses
    try:
        pool1 = Web3.to_checksum_address(pool1)
        pool2 = Web3.to_checksum_address(pool2)
    except:
        print("❌ Invalid Ethereum addresses provided")
        sys.exit(1)
    
    if not w3.is_connected():
        print("❌ Failed to connect to Polygon RPC")
        sys.exit(1)
    
    print(f"Connected to Polygon (Block: {w3.eth.block_number})")
    
    analyze_arbitrage(pool1, pool2)

if __name__ == "__main__":
    main()
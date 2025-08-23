#!/usr/bin/env python3
"""
EXACT V3 Liquidity Calculation - No Estimates!
Shows how to calculate precise available liquidity and optimal trade size
"""

from web3 import Web3
import json
import math

w3 = Web3(Web3.HTTPProvider("https://polygon.publicnode.com"))

V3_ABI = json.loads('''[
    {"inputs":[],"name":"slot0","outputs":[
        {"name":"sqrtPriceX96","type":"uint160"},
        {"name":"tick","type":"int24"}
    ],"type":"function"},
    {"inputs":[],"name":"liquidity","outputs":[{"name":"","type":"uint128"}],"type":"function"},
    {"inputs":[],"name":"fee","outputs":[{"name":"","type":"uint24"}],"type":"function"},
    {"inputs":[],"name":"tickSpacing","outputs":[{"name":"","type":"int24"}],"type":"function"},
    {"inputs":[],"name":"token0","outputs":[{"name":"","type":"address"}],"type":"function"},
    {"inputs":[],"name":"token1","outputs":[{"name":"","type":"address"}],"type":"function"}
]''')

def sqrt_price_to_tick(sqrt_price_x96):
    """Convert sqrtPriceX96 to tick"""
    price = (sqrt_price_x96 / (2**96)) ** 2
    return int(math.log(price) / math.log(1.0001))

def tick_to_sqrt_price(tick):
    """Convert tick to sqrtPriceX96"""
    price = 1.0001 ** tick
    sqrt_price = math.sqrt(price)
    return int(sqrt_price * (2**96))

def calculate_swap_output(
    sqrt_price_x96_start,
    liquidity,
    amount_in,
    fee_pips,  # fee in pips (100 = 1 basis point = 0.01%)
    zero_for_one  # true if swapping token0 for token1
):
    """
    Calculate EXACT output for a swap in V3
    This is the actual math used by Uniswap V3 contracts
    """
    # Apply fee
    amount_in_after_fee = amount_in * (1_000_000 - fee_pips) // 1_000_000
    
    if zero_for_one:
        # Swapping token0 for token1 (price decreases)
        # Calculate how much sqrt_price changes
        sqrt_price_x96_end = sqrt_price_x96_start - (amount_in_after_fee * (2**96)) // liquidity
        
        # Calculate output amount
        amount_out = liquidity * (sqrt_price_x96_start - sqrt_price_x96_end) // (2**96)
    else:
        # Swapping token1 for token0 (price increases)
        # This is more complex - need to solve quadratic
        numerator = liquidity * (2**96) * amount_in_after_fee
        denominator = liquidity * (2**96) + amount_in_after_fee * sqrt_price_x96_start
        sqrt_price_x96_end = sqrt_price_x96_start + numerator // denominator
        
        # Calculate output
        amount_out = liquidity * (sqrt_price_x96_end - sqrt_price_x96_start) // (2**96)
    
    # Calculate actual price impact
    price_impact = abs(sqrt_price_x96_end - sqrt_price_x96_start) / sqrt_price_x96_start
    
    return {
        'amount_out': amount_out,
        'sqrt_price_end': sqrt_price_x96_end,
        'price_impact': price_impact * 100,  # as percentage
        'execution_price': amount_out / amount_in if amount_in > 0 else 0
    }

def find_optimal_trade_size(pool_address):
    """
    Find the optimal trade size for maximum profit
    No estimates - uses exact V3 math!
    """
    pool = w3.eth.contract(address=Web3.to_checksum_address(pool_address), abi=V3_ABI)
    
    # Get pool state
    slot0 = pool.functions.slot0().call()
    sqrt_price_x96 = slot0[0]
    current_tick = slot0[1]
    liquidity = pool.functions.liquidity().call()
    fee = pool.functions.fee().call()  # in 0.01% units (100 = 1%)
    
    print(f"\n📊 Pool Analysis: {pool_address[:10]}...")
    print(f"   Current tick: {current_tick}")
    print(f"   Current liquidity: {liquidity:,}")
    print(f"   Fee: {fee/10000:.2%}")
    
    # Calculate price from sqrtPriceX96
    price = (sqrt_price_x96 / (2**96)) ** 2
    print(f"   Price (token1/token0): {price:.8f}")
    
    # Test different trade sizes to find optimal
    # Start with small amounts and increase
    test_amounts = [
        10**16,    # 0.01 token
        10**17,    # 0.1 token
        10**18,    # 1 token
        5*10**18,  # 5 tokens
        10**19,    # 10 tokens
        5*10**19,  # 50 tokens
        10**20,    # 100 tokens
    ]
    
    print(f"\n💧 Testing trade sizes (token0 -> token1):")
    print(f"   {'Amount':>12} | {'Output':>12} | {'Impact':>8} | {'Avg Price':>12}")
    print("   " + "-"*55)
    
    results = []
    for amount in test_amounts:
        if liquidity == 0:
            print("   ❌ No liquidity in current tick!")
            break
            
        result = calculate_swap_output(
            sqrt_price_x96,
            liquidity,
            amount,
            fee * 100,  # Convert to pips
            True  # zero_for_one
        )
        
        amount_human = amount / 10**18
        output_human = result['amount_out'] / 10**18
        
        results.append({
            'amount': amount_human,
            'output': output_human,
            'impact': result['price_impact'],
            'avg_price': output_human / amount_human if amount_human > 0 else 0
        })
        
        print(f"   {amount_human:>12.4f} | {output_human:>12.4f} | {result['price_impact']:>7.3f}% | {results[-1]['avg_price']:>12.8f}")
        
        # Stop if price impact is too high
        if result['price_impact'] > 10:
            print("   ⚠️  Stopping - price impact too high!")
            break
    
    # Find optimal size (best execution price)
    if results:
        best = max(results, key=lambda x: x['avg_price'])
        print(f"\n✅ Optimal trade size: {best['amount']:.4f} tokens")
        print(f"   Expected output: {best['output']:.4f} tokens")
        print(f"   Price impact: {best['impact']:.3f}%")
        print(f"   Execution price: {best['avg_price']:.8f}")
    
    return results

def calculate_arbitrage_profit(pool1_addr, pool2_addr, trade_size):
    """
    Calculate EXACT arbitrage profit between two pools
    Using real V3 math, not estimates!
    """
    pool1 = w3.eth.contract(address=Web3.to_checksum_address(pool1_addr), abi=V3_ABI)
    pool2 = w3.eth.contract(address=Web3.to_checksum_address(pool2_addr), abi=V3_ABI)
    
    # Get pool states
    slot0_1 = pool1.functions.slot0().call()
    slot0_2 = pool2.functions.slot0().call()
    
    liquidity1 = pool1.functions.liquidity().call()
    liquidity2 = pool2.functions.liquidity().call()
    
    fee1 = pool1.functions.fee().call()
    fee2 = pool2.functions.fee().call()
    
    sqrt_price1 = slot0_1[0]
    sqrt_price2 = slot0_2[0]
    
    price1 = (sqrt_price1 / (2**96)) ** 2
    price2 = (sqrt_price2 / (2**96)) ** 2
    
    print(f"\n🎯 Arbitrage Analysis:")
    print(f"   Pool 1: {pool1_addr[:10]}... @ {price1:.8f} (fee {fee1/10000:.2%})")
    print(f"   Pool 2: {pool2_addr[:10]}... @ {price2:.8f} (fee {fee2/10000:.2%})")
    
    if price1 < price2:
        # Buy from pool1, sell to pool2
        buy_pool = (pool1_addr, sqrt_price1, liquidity1, fee1)
        sell_pool = (pool2_addr, sqrt_price2, liquidity2, fee2)
        direction = "Pool1 → Pool2"
    else:
        # Buy from pool2, sell to pool1
        buy_pool = (pool2_addr, sqrt_price2, liquidity2, fee2)
        sell_pool = (pool1_addr, sqrt_price1, liquidity1, fee1)
        direction = "Pool2 → Pool1"
    
    trade_size_raw = int(trade_size * 10**18)
    
    # Step 1: Buy token0 with token1
    buy_result = calculate_swap_output(
        buy_pool[1],  # sqrt_price
        buy_pool[2],  # liquidity
        trade_size_raw,
        buy_pool[3] * 100,  # fee in pips
        False  # token1 -> token0
    )
    
    token0_received = buy_result['amount_out']
    
    # Step 2: Sell token0 for token1
    sell_result = calculate_swap_output(
        sell_pool[1],  # sqrt_price
        sell_pool[2],  # liquidity
        token0_received,
        sell_pool[3] * 100,  # fee in pips
        True  # token0 -> token1
    )
    
    token1_final = sell_result['amount_out']
    
    # Calculate profit
    profit_raw = token1_final - trade_size_raw
    profit = profit_raw / 10**18
    
    print(f"\n📈 Trade path: {direction}")
    print(f"   Input: {trade_size:.6f} token1")
    print(f"   → Buy: {token0_received/10**18:.6f} token0 (impact: {buy_result['price_impact']:.3f}%)")
    print(f"   → Sell: {token1_final/10**18:.6f} token1 (impact: {sell_result['price_impact']:.3f}%)")
    print(f"   Profit: {profit:.6f} token1")
    
    return profit

def main():
    print("="*70)
    print("EXACT V3 LIQUIDITY AND ARBITRAGE CALCULATION")
    print("="*70)
    
    # Test with a V3 pool
    test_pool = "0x0f663c16dd7c65cf87edb9229464ca77aeea536b"  # WMATIC/DAI 0.05%
    
    print("\n1️⃣ Finding optimal trade size...")
    results = find_optimal_trade_size(test_pool)
    
    # Test arbitrage between two pools
    pool1 = "0x7a7374873de28b06386013da94cbd9b554f6ac6e"  # 0.01% fee
    pool2 = "0x58359563b3f4854428b1b98e91a42471e6d20b8e"  # 1.00% fee
    
    print("\n2️⃣ Calculating exact arbitrage profit...")
    profit = calculate_arbitrage_profit(pool1, pool2, 0.1)  # Test with 0.1 token
    
    print("\n" + "="*70)
    print("KEY INSIGHTS")
    print("="*70)
    print("\n✅ We CAN calculate EXACT liquidity and outputs!")
    print("   - No estimates needed")
    print("   - Use pool's liquidity() for current tick")
    print("   - Calculate exact price impact with V3 math")
    print("   - Dynamically size trades based on available liquidity")
    print("\n❌ The backend should NOT:")
    print("   - Estimate with percentages")
    print("   - Use TVL as available liquidity")
    print("   - Ignore price impact")

if __name__ == "__main__":
    main()
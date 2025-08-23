#!/usr/bin/env python3
"""
Mathematical calculation of optimal arbitrage trade size
No binary search needed - pure math!
"""

import math
from web3 import Web3

def calculate_optimal_v2_arbitrage(r1_in, r1_out, r2_in, r2_out, fee1=997, fee2=997):
    """
    Calculate the EXACT optimal trade size for V2 arbitrage
    
    For arbitrage between two Uniswap V2 pools:
    Pool 1: Buy token0 with token1 (reserves: r1_in token1, r1_out token0)
    Pool 2: Sell token0 for token1 (reserves: r2_in token0, r2_out token1)
    
    The profit function is:
    profit(x) = output_from_pool2(output_from_pool1(x)) - x
    
    Taking the derivative and setting to zero gives us the optimal x.
    
    Args:
        r1_in: Pool 1 input token reserves (token1)
        r1_out: Pool 1 output token reserves (token0)
        r2_in: Pool 2 input token reserves (token0) 
        r2_out: Pool 2 output token reserves (token1)
        fee1: Fee multiplier for pool 1 (997 for 0.3% fee)
        fee2: Fee multiplier for pool 2 (997 for 0.3% fee)
    
    Returns:
        Optimal trade amount in token1
    """
    
    # The closed-form solution for optimal trade size
    # Derived from setting d(profit)/dx = 0
    
    # Numerator: sqrt(r1_in * r1_out * r2_in * r2_out * fee1 * fee2) - (r1_in * r2_out * 1000)
    # Denominator: fee1 * fee2
    
    numerator = math.sqrt(r1_in * r1_out * r2_in * r2_out * fee1 * fee2) - (r1_in * r2_out * 1000)
    denominator = fee1 * fee2 / 1000
    
    x_optimal = numerator / denominator
    
    # x must be positive and less than available reserves
    if x_optimal < 0:
        return 0
    
    # Don't trade more than a reasonable portion of the pool
    max_trade = min(r1_in * 0.1, r2_out * 0.1)  # Max 10% of smaller pool
    
    return min(x_optimal, max_trade)

def calculate_arbitrage_profit(x, r1_in, r1_out, r2_in, r2_out, fee1=997, fee2=997):
    """
    Calculate the actual profit for a given trade size
    """
    if x <= 0:
        return 0
    
    # Step 1: Buy token0 from pool1 with x token1
    # output = (x * fee1 * r1_out) / (1000 * r1_in + x * fee1)
    token0_received = (x * fee1 * r1_out) / (1000 * r1_in + x * fee1)
    
    # Step 2: Sell token0 to pool2 for token1
    # output = (token0_received * fee2 * r2_out) / (1000 * r2_in + token0_received * fee2)
    token1_received = (token0_received * fee2 * r2_out) / (1000 * r2_in + token0_received * fee2)
    
    # Profit is what we get back minus what we put in
    profit = token1_received - x
    
    return profit

def find_optimal_with_gas(r1_in, r1_out, r2_in, r2_out, gas_cost_in_token1, fee1=997, fee2=997):
    """
    Find optimal trade size considering gas costs
    The optimal point shifts when we account for fixed gas costs
    """
    
    # First find the unconstrained optimal
    x_optimal_no_gas = calculate_optimal_v2_arbitrage(r1_in, r1_out, r2_in, r2_out, fee1, fee2)
    
    # If no arbitrage without gas, definitely none with gas
    if x_optimal_no_gas <= 0:
        return 0, -gas_cost_in_token1
    
    # The profit function with gas is: profit(x) - gas_cost
    # This shifts the optimal point slightly, but for most cases
    # the unconstrained optimal is still very close
    
    profit_at_optimal = calculate_arbitrage_profit(x_optimal_no_gas, r1_in, r1_out, r2_in, r2_out, fee1, fee2)
    
    # Check if profitable after gas
    if profit_at_optimal <= gas_cost_in_token1:
        # Not profitable at optimal size
        # Could solve for break-even point, but usually not worth it
        return 0, profit_at_optimal - gas_cost_in_token1
    
    return x_optimal_no_gas, profit_at_optimal - gas_cost_in_token1

def demo():
    """
    Demonstrate the mathematical calculation vs binary search
    """
    print("="*70)
    print("OPTIMAL ARBITRAGE - MATHEMATICAL SOLUTION")
    print("="*70)
    
    # Example reserves (in raw units)
    # Pool 1: 1000 USDC, 1500 WETH (price: 1500 USDC/WETH)
    # Pool 2: 1600 WETH, 2500 USDC (price: 1562.5 USDC/WETH)
    
    r1_in = 1000 * 10**6    # 1000 USDC (6 decimals)
    r1_out = 1500 * 10**18  # 1500 WETH (18 decimals)
    r2_in = 1600 * 10**18   # 1600 WETH
    r2_out = 2500 * 10**6   # 2500 USDC
    
    print("\n📊 Pool Setup:")
    print(f"   Pool 1: {r1_in/10**6:.2f} USDC, {r1_out/10**18:.2f} WETH")
    print(f"   Pool 1 price: {(r1_in/10**6)/(r1_out/10**18):.2f} USDC/WETH")
    print(f"   Pool 2: {r2_out/10**6:.2f} USDC, {r2_in/10**18:.2f} WETH")
    print(f"   Pool 2 price: {(r2_out/10**6)/(r2_in/10**18):.2f} USDC/WETH")
    
    # Calculate optimal trade size mathematically
    optimal_x = calculate_optimal_v2_arbitrage(r1_in, r1_out, r2_in, r2_out)
    optimal_profit = calculate_arbitrage_profit(optimal_x, r1_in, r1_out, r2_in, r2_out)
    
    print(f"\n✨ MATHEMATICAL SOLUTION:")
    print(f"   Optimal trade size: {optimal_x/10**6:.6f} USDC")
    print(f"   Maximum profit: {optimal_profit/10**6:.6f} USDC")
    
    # Compare with binary search approach
    print(f"\n🔍 BINARY SEARCH COMPARISON:")
    low = 0.01 * 10**6
    high = min(r1_in * 0.1, r2_out * 0.1)
    
    for i in range(20):
        mid = (low + high) / 2
        profit = calculate_arbitrage_profit(mid, r1_in, r1_out, r2_in, r2_out)
        
        # Test gradient
        profit_higher = calculate_arbitrage_profit(mid * 1.001, r1_in, r1_out, r2_in, r2_out)
        
        if profit_higher > profit:
            low = mid
        else:
            high = mid
        
        if i == 19:  # Last iteration
            print(f"   Binary search result: {mid/10**6:.6f} USDC")
            print(f"   Binary search profit: {profit/10**6:.6f} USDC")
    
    print(f"\n📐 Accuracy comparison:")
    print(f"   Math vs Binary difference: {abs(optimal_x - mid)/10**6:.9f} USDC")
    print(f"   Math is exact, binary search is approximate!")
    
    # With gas costs
    gas_cost_usd = 0.003
    gas_cost_in_usdc = gas_cost_usd * 10**6  # Convert to USDC units
    
    optimal_with_gas, net_profit = find_optimal_with_gas(
        r1_in, r1_out, r2_in, r2_out, gas_cost_in_usdc
    )
    
    print(f"\n⛽ With gas costs (${gas_cost_usd}):")
    if optimal_with_gas > 0:
        print(f"   Optimal trade: {optimal_with_gas/10**6:.6f} USDC")
        print(f"   Net profit: {net_profit/10**6:.6f} USDC")
    else:
        print(f"   Not profitable after gas!")
    
    print("\n" + "="*70)
    print("KEY INSIGHTS")
    print("="*70)
    print("\n✅ For V2 pools, optimal trade size is DETERMINISTIC:")
    print("   x* = sqrt(r1_in * r1_out * r2_in * r2_out * f1 * f2) - (r1_in * r2_out)")
    print("        --------------------------------------------------------")
    print("                           (f1 * f2) / 1000")
    print("\n✅ No binary search needed for V2-V2 arbitrage!")
    print("\n⚠️  For V3 pools, it's more complex due to:")
    print("   - Concentrated liquidity in tick ranges")
    print("   - Need to traverse multiple ticks")
    print("   - Non-linear price impact")
    print("   → Binary search or numerical methods needed for V3")

if __name__ == "__main__":
    demo()
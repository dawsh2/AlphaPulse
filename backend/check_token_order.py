#!/usr/bin/env python3
"""
Token Order Checker
===================
Checks the actual token ordering in POL/USDC pool to fix price calculation.
"""

def compare_addresses(addr1, addr2):
    """Compare two Ethereum addresses"""
    # Remove 0x prefix and convert to lowercase
    a1 = addr1.lower().replace('0x', '')
    a2 = addr2.lower().replace('0x', '')
    
    if a1 < a2:
        return -1
    elif a1 > a2:
        return 1
    else:
        return 0

def main():
    print("🔍 TOKEN ORDER ANALYSIS")
    print("=" * 40)
    
    # Token addresses on Polygon
    pol_address = "0x455e53CBB86018Ac2B8092FdCd39d8444aFFC3F6"  # POL token
    usdc_address = "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174"  # USDC token
    
    print(f"POL address:  {pol_address}")
    print(f"USDC address: {usdc_address}")
    
    comparison = compare_addresses(pol_address, usdc_address)
    
    if comparison < 0:
        print(f"\n📊 TOKEN ORDERING:")
        print(f"  token0 = POL  ({pol_address})")
        print(f"  token1 = USDC ({usdc_address})")
        token0 = "POL"
        token1 = "USDC"
    else:
        print(f"\n📊 TOKEN ORDERING:")
        print(f"  token0 = USDC ({usdc_address})")
        print(f"  token1 = POL  ({pol_address})")
        token0 = "USDC"
        token1 = "POL"
    
    print(f"\n🔍 SWAP ANALYSIS with correct ordering:")
    
    # Let me re-parse the exact hex data:
    # "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000052080000000000000000000000000000000000000000000000000000ee5e36d040d720000000000000000000000000000000000000000000000000000000000000000"
    
    hex_data = "00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000052080000000000000000000000000000000000000000000000000000ee5e36d040d720000000000000000000000000000000000000000000000000000000000000000"
    
    amount0_in_raw = int(hex_data[0:64], 16)
    amount1_in_raw = int(hex_data[64:128], 16) 
    amount0_out_raw = int(hex_data[128:192], 16)
    amount1_out_raw = int(hex_data[192:256], 16)
    
    print(f"  Re-parsed amounts:")
    print(f"    amount0_in_raw = {amount0_in_raw}")
    print(f"    amount1_in_raw = {amount1_in_raw}")
    print(f"    amount0_out_raw = {amount0_out_raw}")
    print(f"    amount1_out_raw = {amount1_out_raw}")
    
    print(f"  amount0_in_raw = {amount0_in_raw} ({token0})")
    print(f"  amount1_in_raw = {amount1_in_raw} ({token1})")  
    print(f"  amount0_out_raw = {amount0_out_raw} ({token0})")
    print(f"  amount1_out_raw = {amount1_out_raw} ({token1})")
    
    # Apply correct decimals based on actual token assignment
    if token0 == "POL":
        decimals0 = 18  # POL
        decimals1 = 6   # USDC
    else:
        decimals0 = 6   # USDC
        decimals1 = 18  # POL
        
    amount0_in = amount0_in_raw / (10 ** decimals0)
    amount1_in = amount1_in_raw / (10 ** decimals1)
    amount0_out = amount0_out_raw / (10 ** decimals0)
    amount1_out = amount1_out_raw / (10 ** decimals1)
    
    print(f"\n📊 DECIMAL ADJUSTED:")
    print(f"  {token0}_in:  {amount0_in:.6f}")
    print(f"  {token1}_in:  {amount1_in:.6f}")
    print(f"  {token0}_out: {amount0_out:.6f}")
    print(f"  {token1}_out: {amount1_out:.6f}")
    
    # Calculate POL price correctly
    if token0 == "POL":
        # POL is token0, USDC is token1
        if amount1_in > 0 and amount0_out > 0:
            price = amount1_in / amount0_out  # USDC per POL
            direction = "USDC → POL"
        elif amount0_in > 0 and amount1_out > 0:
            price = amount1_out / amount0_in  # USDC per POL
            direction = "POL → USDC"
        else:
            print("❓ Unclear direction")
            return
    else:
        # USDC is token0, POL is token1  
        if amount0_in > 0 and amount1_out > 0:
            price = amount0_in / amount1_out  # USDC per POL
            direction = "USDC → POL"
        elif amount1_in > 0 and amount0_out > 0:
            price = amount1_in / amount0_out  # USDC per POL (this should be wrong)
            direction = "POL → USDC"
        else:
            print("❓ Unclear direction")
            return
            
    print(f"\n💰 PRICE CALCULATION:")
    print(f"  Direction: {direction}")
    print(f"  POL price: ${price:.6f}")
    
    expected = 0.23
    if abs(price - expected) < 0.05:
        print(f"  ✅ CORRECT! (expected ~${expected})")
    else:
        factor = price / expected
        print(f"  ❌ WRONG! Off by {factor:.1f}x (expected ~${expected})")

if __name__ == "__main__":
    main()
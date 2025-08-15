#!/usr/bin/env python3
"""
Price Calculation Debug
=======================
Debug the exact price calculation logic with real swap data.
"""

def debug_price_calculation():
    """Debug the price calculation with real values"""
    print("🔍 PRICE CALCULATION DEBUG")
    print("=" * 40)
    
    # Real swap data from hex: "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000052080000000000000000000000000000000000000000000000000000ee5e36d040d720000000000000000000000000000000000000000000000000000000000000000"
    hex_data = "00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000052080000000000000000000000000000000000000000000000000000ee5e36d040d720000000000000000000000000000000000000000000000000000000000000000"
    
    amount0_in_raw = int(hex_data[0:64], 16)
    amount1_in_raw = int(hex_data[64:128], 16) 
    amount0_out_raw = int(hex_data[128:192], 16)
    amount1_out_raw = int(hex_data[192:256], 16)
    
    print(f"HEX PARSING CHECK:")
    print(f"  Hex segments:")
    print(f"    amount0_in:  {hex_data[0:64]} = {amount0_in_raw}")
    print(f"    amount1_in:  {hex_data[64:128]} = {amount1_in_raw}")
    print(f"    amount0_out: {hex_data[128:192]} = {amount0_out_raw}")
    print(f"    amount1_out: {hex_data[192:256]} = {amount1_out_raw}")
    print()
    
    print(f"RAW AMOUNTS:")
    print(f"  token0 (USDC) in:  {amount0_in_raw}")
    print(f"  token1 (POL) in:   {amount1_in_raw}")
    print(f"  token0 (USDC) out: {amount0_out_raw}")
    print(f"  token1 (POL) out:  {amount1_out_raw}")
    
    # Apply decimals correctly
    usdc_decimals = 6
    pol_decimals = 18
    
    amount0_in = amount0_in_raw / (10 ** usdc_decimals)  # USDC
    amount1_in = amount1_in_raw / (10 ** pol_decimals)   # POL
    amount0_out = amount0_out_raw / (10 ** usdc_decimals) # USDC
    amount1_out = amount1_out_raw / (10 ** pol_decimals)  # POL
    
    print(f"\nDECIMAL ADJUSTED:")
    print(f"  USDC in:  {amount0_in:.6f}")
    print(f"  POL in:   {amount1_in:.18f} (full precision)")
    print(f"  USDC out: {amount0_out:.6f}")
    print(f"  POL out:  {amount1_out:.6f}")
    
    print(f"\nTHE PROBLEM:")
    print(f"  336000 POL raw / 10^18 = {amount1_in}")
    print(f"  This is 0.000336 POL, which rounds to 0.000000 when displayed to 6 decimals")
    print(f"  But 4193414623268210 USDC raw / 10^6 = {amount0_out} USDC")
    print(f"  So we have: 0.000336 POL bought for 4,193,414,623 USDC")
    
    # Simulate the Rust logic
    token0 = "USDC"
    token1 = "POL"
    
    # Step 1: Determine base/quote and inversion
    def is_quote_currency(token):
        return token in ["USDC", "USDT", "DAI", "USD"]
    
    if is_quote_currency(token1):
        # token1 is quote (like USDC), token0 is base (like POL)
        base_token = token0
        quote_token = token1
        invert_price = False
    elif is_quote_currency(token0):
        # token0 is quote, token1 is base - need to invert
        base_token = token1
        quote_token = token0
        invert_price = True
    else:
        # Neither is clear quote, use token1 as quote by default
        base_token = token0
        quote_token = token1
        invert_price = False
    
    print(f"\nLOGIC ANALYSIS:")
    print(f"  base_token: {base_token}")
    print(f"  quote_token: {quote_token}")
    print(f"  invert_price: {invert_price}")
    
    # Step 2: Calculate raw price
    if amount0_in > 0.0 and amount1_out > 0.0:
        # Buying token1 with token0: price = amount1_out / amount0_in
        raw_price = amount1_out / amount0_in
        direction = f"{token0} → {token1}"
    elif amount1_in > 0.0 and amount0_out > 0.0:
        # Buying token0 with token1: price = amount1_in / amount0_out
        raw_price = amount1_in / amount0_out
        direction = f"{token1} → {token0}"
    else:
        raw_price = 0.0
        direction = "unclear"
    
    print(f"\nRAW PRICE CALCULATION:")
    print(f"  Direction: {direction}")
    print(f"  Calculation: {amount1_in:.6f} / {amount0_out:.6f} = {raw_price:.6f}")
    
    # Step 3: Apply inversion
    if invert_price:
        final_price = 1.0 / raw_price if raw_price > 0.0 else 0.0
    else:
        final_price = raw_price
        
    print(f"\nFINAL PRICE:")
    print(f"  After inversion: {final_price:.6f}")
    print(f"  Expected POL price: ~0.23")
    
    # The problem is revealed!
    print(f"\n🔍 PROBLEM ANALYSIS:")
    print(f"  raw_price = {amount1_in:.6f} POL / {amount0_out:.6f} USDC")
    print(f"  raw_price = {raw_price:.6f} POL per USDC")
    print(f"  To get USDC per POL, we need: 1/raw_price = {1/raw_price:.6f}")
    
    # Correct calculation
    correct_price = amount0_out / amount1_in  # USDC per POL
    print(f"\n✅ CORRECT CALCULATION:")
    print(f"  POL price = {amount0_out:.6f} USDC / {amount1_in:.6f} POL")
    print(f"  POL price = ${correct_price:.6f} per POL")

if __name__ == "__main__":
    debug_price_calculation()
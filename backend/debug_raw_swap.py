#!/usr/bin/env python3
"""
Raw Swap Data Decoder
======================
Decodes the actual raw swap data from live events to understand price calculation issue.
"""

def decode_swap_data(hex_data):
    """Decode Uniswap V2 swap event data"""
    # Remove 0x prefix if present
    data = hex_data.replace('0x', '')
    
    # Each uint256 is 64 hex characters (32 bytes)
    # Swap event: (uint256 amount0In, uint256 amount1In, uint256 amount0Out, uint256 amount1Out)
    
    if len(data) < 256:
        print(f"❌ Data too short: {len(data)} chars, need 256")
        return None
        
    amount0_in_raw = int(data[0:64], 16)
    amount1_in_raw = int(data[64:128], 16) 
    amount0_out_raw = int(data[128:192], 16)
    amount1_out_raw = int(data[192:256], 16)
    
    return {
        'amount0_in_raw': amount0_in_raw,
        'amount1_in_raw': amount1_in_raw, 
        'amount0_out_raw': amount0_out_raw,
        'amount1_out_raw': amount1_out_raw
    }

def analyze_pol_swap(amounts):
    """Analyze POL/USDC swap amounts"""
    print("📊 RAW AMOUNTS:")
    print(f"  amount0_in_raw:  {amounts['amount0_in_raw']:,}")
    print(f"  amount1_in_raw:  {amounts['amount1_in_raw']:,}")
    print(f"  amount0_out_raw: {amounts['amount0_out_raw']:,}")
    print(f"  amount1_out_raw: {amounts['amount1_out_raw']:,}")
    
    # Apply decimals: POL=18, USDC=6
    pol_decimals = 18
    usdc_decimals = 6
    
    amount0_in = amounts['amount0_in_raw'] / (10 ** pol_decimals)
    amount1_in = amounts['amount1_in_raw'] / (10 ** usdc_decimals)
    amount0_out = amounts['amount0_out_raw'] / (10 ** pol_decimals)
    amount1_out = amounts['amount1_out_raw'] / (10 ** usdc_decimals)
    
    print("\n📊 DECIMAL ADJUSTED:")
    print(f"  POL_in:  {amount0_in:.6f}")
    print(f"  USDC_in: {amount1_in:.6f}")
    print(f"  POL_out: {amount0_out:.6f}")
    print(f"  USDC_out: {amount1_out:.6f}")
    
    # Determine swap direction and calculate price
    if amount0_in > 0 and amount1_out > 0:
        # Selling POL for USDC
        price = amount1_out / amount0_in
        direction = "POL → USDC"
        print(f"\n💰 DIRECTION: {direction}")
        print(f"  Price = {amount1_out:.6f} USDC / {amount0_in:.6f} POL = ${price:.6f} per POL")
        
    elif amount1_in > 0 and amount0_out > 0:
        # Buying POL with USDC  
        price = amount1_in / amount0_out
        direction = "USDC → POL"
        print(f"\n💰 DIRECTION: {direction}")
        print(f"  Price = {amount1_in:.6f} USDC / {amount0_out:.6f} POL = ${price:.6f} per POL")
        
    else:
        print("\n❓ UNCLEAR SWAP DIRECTION")
        return None
        
    # Validate against expected POL price
    expected_pol_price = 0.23
    print(f"\n🎯 VALIDATION:")
    print(f"  Expected POL price: ~${expected_pol_price:.2f}")
    print(f"  Calculated price:   ${price:.6f}")
    
    if abs(price - expected_pol_price) < 0.05:
        print("  ✅ CORRECT!")
    else:
        factor = price / expected_pol_price
        print(f"  ❌ WRONG! Off by {factor:.1f}x")
        if factor > 100:
            print("  💡 Likely decimal/inversion issue")
        
    return price

if __name__ == "__main__":
    print("🔍 POL SWAP DATA DECODER")
    print("=" * 40)
    
    # Real swap data from logs  
    test_cases = [
        "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000052080000000000000000000000000000000000000000000000000000ee5e36d040d720000000000000000000000000000000000000000000000000000000000000000",
        "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000036b000000000000000000000000000000000000000000000000000009ee979e00f6320000000000000000000000000000000000000000000000000000000000000000"
    ]
    
    for i, hex_data in enumerate(test_cases, 1):
        print(f"\n📋 TEST CASE {i}:")
        print(f"Raw data: {hex_data[:50]}...")
        
        amounts = decode_swap_data(hex_data)
        if amounts:
            analyze_pol_swap(amounts)
            
        print("\n" + "=" * 50)
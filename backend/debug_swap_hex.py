#!/usr/bin/env python3
"""
Debug the exact hex parsing from the problematic swap
"""

# The problematic swap data
hex_data = "0x000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000063893e8000000000000000000000000000000000000000000000000121396b879a673490000000000000000000000000000000000000000000000000000000000000000"

# Remove 0x prefix if present
data = hex_data.replace('0x', '')

# Parse the 4 amounts (each 64 hex chars = 32 bytes)
amount0_in_raw = int(data[0:64], 16)
amount1_in_raw = int(data[64:128], 16)
amount0_out_raw = int(data[128:192], 16)
amount1_out_raw = int(data[192:256], 16)

print("🔍 HEX PARSING DEBUG")
print("=" * 40)
print(f"Raw hex data: {hex_data[:80]}...")
print()
print("Parsed amounts:")
print(f"  amount0_in:  {data[0:64]} = {amount0_in_raw:,}")
print(f"  amount1_in:  {data[64:128]} = {amount1_in_raw:,}")
print(f"  amount0_out: {data[128:192]} = {amount0_out_raw:,}")
print(f"  amount1_out: {data[192:256]} = {amount1_out_raw:,}")
print()

# Apply decimals (token0=USDC with 6 decimals, token1=POL with 18 decimals)
usdc_decimals = 6
pol_decimals = 18

amount0_in = amount0_in_raw / (10 ** usdc_decimals)  # USDC in
amount1_in = amount1_in_raw / (10 ** pol_decimals)   # POL in
amount0_out = amount0_out_raw / (10 ** usdc_decimals) # USDC out
amount1_out = amount1_out_raw / (10 ** pol_decimals)  # POL out

print("Decimal adjusted:")
print(f"  USDC in:  {amount0_in:.6f}")
print(f"  POL in:   {amount1_in:.18f}")
print(f"  USDC out: {amount0_out:.6f}")
print(f"  POL out:  {amount1_out:.6f}")
print()

# What's the actual swap here?
if amount1_in > 0 and amount0_out > 0:
    # Swapping POL for USDC
    print("Swap direction: POL → USDC")
    print(f"  Swapping {amount1_in:.18f} POL")
    print(f"  Receiving {amount0_out:.6f} USDC")
    
    price = amount0_out / amount1_in
    print(f"  Price: ${price:.2f} per POL")
    print()
    print("❌ PROBLEM: This says 0.0000001 POL is worth 1.3 trillion USDC!")
    print("   This is clearly corrupted data from the blockchain")
elif amount0_in > 0 and amount1_out > 0:
    # Swapping USDC for POL
    print("Swap direction: USDC → POL")
    print(f"  Swapping {amount0_in:.6f} USDC")
    print(f"  Receiving {amount1_out:.18f} POL")
    
    price = amount0_in / amount1_out
    print(f"  Price: ${price:.6f} per POL")
#!/usr/bin/env python3
"""
Analyze the raw swap amounts to understand what's happening
"""

# Sample raw swap amounts from logs
samples = [
    # token0_in_raw, token1_in_raw, token0_out_raw, token1_out_raw
    (20021767500419825664, 0, 0, 1594904687),  # Looks like 20 trillion USDC in?!
    (0, 300000000, 3743512536621823488, 0),      # 0.0000003 POL for 3.7 million USDC?
    (0, 9500000000, 118544475741846929408, 0),   # 0.0095 POL for 118 million USDC?
    (17900195102610092032, 0, 0, 1425908212),    # 17 trillion USDC for 0.0014 POL?
]

print("ANALYSIS OF RAW SWAP AMOUNTS")
print("=" * 60)

for i, (a0_in, a1_in, a0_out, a1_out) in enumerate(samples, 1):
    print(f"\nSample {i}:")
    print(f"  Raw: token0_in={a0_in:,}, token1_in={a1_in:,}")
    print(f"       token0_out={a0_out:,}, token1_out={a1_out:,}")
    
    # Apply decimals (USDC=6, POL=18)
    usdc_in = a0_in / 10**6
    pol_in = a1_in / 10**18
    usdc_out = a0_out / 10**6
    pol_out = a1_out / 10**18
    
    print(f"  Decimal adjusted (if USDC/POL):")
    print(f"    USDC in: ${usdc_in:,.2f}")
    print(f"    POL in: {pol_in:.9f}")
    print(f"    USDC out: ${usdc_out:,.2f}")
    print(f"    POL out: {pol_out:.9f}")
    
    # What if we got the decimals wrong?
    # What if these are actually wei values (10^18)?
    usdc_in_wei = a0_in / 10**18
    pol_in_wei = a1_in / 10**18
    usdc_out_wei = a0_out / 10**18
    pol_out_wei = a1_out / 10**18
    
    print(f"  If everything was 18 decimals:")
    print(f"    Token0 in: {usdc_in_wei:.6f}")
    print(f"    Token1 in: {pol_in_wei:.9f}")
    print(f"    Token0 out: {usdc_out_wei:.6f}")
    print(f"    Token1 out: {pol_out_wei:.9f}")
    
    # Check if these look like realistic values
    if usdc_in_wei < 100000 and pol_out_wei < 100000:
        if pol_in_wei > 0 and usdc_out_wei > 0:
            price = usdc_out_wei / pol_in_wei
            print(f"    -> Price if 18 decimals: ${price:.4f} per POL")
        elif usdc_in_wei > 0 and pol_out_wei > 0:
            price = usdc_in_wei / pol_out_wei
            print(f"    -> Price if 18 decimals: ${price:.4f} per POL")

print("\n" + "=" * 60)
print("HYPOTHESIS:")
print("The huge numbers suggest either:")
print("1. We're parsing the wrong event or wrong fields")
print("2. The pool token decimals are different than expected")
print("3. These are wrapped/synthetic tokens with different decimals")
print("4. The pool address mappings are incorrect")
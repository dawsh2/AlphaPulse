#!/usr/bin/env python3

# Test script to verify decimal handling for DEX price calculations

# Token addresses on Polygon
TOKENS = {
    "WMATIC": "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270",
    "USDC": "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174",
    "USDT": "0xc2132D05D31c914a87C6611C10748AEb04B58e8F",
    "WETH": "0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619",
    "DAI": "0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063",
    "WBTC": "0x1BFD67037B42Cf73acF2047067bd4F2C47D9BfD6",
    "LINK": "0x53E0bca35eC356BD5ddDFebbD1Fc0fD03FaBad39",
    "AAVE": "0xD6DF932A45C0f255f85145f286eA0b292B21C90B",
}

# Token decimals
DECIMALS = {
    "USDC": 6,
    "USDT": 6,
    "WBTC": 8,
    "WMATIC": 18,
    "WETH": 18,
    "DAI": 18,
    "LINK": 18,
    "AAVE": 18,
}

def check_token_order(token0, token1):
    """Check which token comes first in AMM pool ordering"""
    addr0 = TOKENS[token0].lower()
    addr1 = TOKENS[token1].lower()
    
    if addr0 < addr1:
        return token0, token1, False  # No swap needed
    else:
        return token1, token0, True   # Swap needed

def calculate_price(token0, token1, reserve0_raw, reserve1_raw):
    """Calculate price with proper decimal adjustment"""
    # Determine token order
    actual_token0, actual_token1, swapped = check_token_order(token0, token1)
    
    if swapped:
        # Tokens were swapped, so swap reserves too
        reserve0_raw, reserve1_raw = reserve1_raw, reserve0_raw
        
    # Get decimals
    decimals0 = DECIMALS[actual_token0]
    decimals1 = DECIMALS[actual_token1]
    
    # Adjust for decimals
    reserve0 = reserve0_raw / (10 ** decimals0)
    reserve1 = reserve1_raw / (10 ** decimals1)
    
    # Calculate price (token1 per token0)
    if swapped:
        # We swapped, so need to return price in original order
        price = reserve0 / reserve1  # Inverted because we swapped
    else:
        price = reserve1 / reserve0
        
    return price, reserve0, reserve1, actual_token0, actual_token1, swapped

# Test cases
print("Testing DEX Price Calculations with Decimal Handling")
print("=" * 60)

# Example: WETH/USDC pool
# Let's say the pool has 1000 WETH and 4,600,000 USDC
# Raw reserves would be:
# - WETH: 1000 * 10^18 = 1000000000000000000000
# - USDC: 4600000 * 10^6 = 4600000000000

print("\n1. WETH/USDC Pool:")
weth_raw = 1000 * (10 ** 18)
usdc_raw = 4600000 * (10 ** 6)

price, r0, r1, t0, t1, swapped = calculate_price("WETH", "USDC", weth_raw, usdc_raw)
print(f"   Raw reserves: WETH={weth_raw:.0f}, USDC={usdc_raw:.0f}")
print(f"   Token order: {t0}/{t1} (swapped={swapped})")
print(f"   Adjusted reserves: {t0}={r0:.2f}, {t1}={r1:.2f}")
print(f"   Price: 1 WETH = ${price:.2f} USDC")

# Example: DAI/USDC (stablecoin pair)
print("\n2. DAI/USDC Pool:")
dai_raw = 1000000 * (10 ** 18)
usdc_raw2 = 999000 * (10 ** 6)

price2, r0_2, r1_2, t0_2, t1_2, swapped2 = calculate_price("DAI", "USDC", dai_raw, usdc_raw2)
print(f"   Raw reserves: DAI={dai_raw:.0f}, USDC={usdc_raw2:.0f}")
print(f"   Token order: {t0_2}/{t1_2} (swapped={swapped2})")
print(f"   Adjusted reserves: {t0_2}={r0_2:.2f}, {t1_2}={r1_2:.2f}")
print(f"   Price: 1 DAI = ${price2:.4f} USDC")

# Example: WBTC/USDC
print("\n3. WBTC/USDC Pool:")
wbtc_raw = 10 * (10 ** 8)
usdc_raw3 = 1180000 * (10 ** 6)

price3, r0_3, r1_3, t0_3, t1_3, swapped3 = calculate_price("WBTC", "USDC", wbtc_raw, usdc_raw3)
print(f"   Raw reserves: WBTC={wbtc_raw:.0f}, USDC={usdc_raw3:.0f}")
print(f"   Token order: {t0_3}/{t1_3} (swapped={swapped3})")
print(f"   Adjusted reserves: {t0_3}={r0_3:.6f}, {t1_3}={r1_3:.2f}")
print(f"   Price: 1 WBTC = ${price3:.2f} USDC")

print("\n" + "=" * 60)
print("Token Address Ordering (for AMM pools):")
for token, addr in sorted(TOKENS.items(), key=lambda x: x[1].lower()):
    print(f"   {token}: {addr}")
#!/usr/bin/env python3
"""Fix scanner to handle stable pools correctly"""

print("SCANNER VULNERABILITY: Stable Pool False Positives")
print("="*60)

print("\n🔴 THE PROBLEM:")
print("1. Stable pools (Curve, Solidly, Dystopia) use different math")
print("2. Scanner uses: price = reserve1/reserve0")
print("3. Stable pools use: x³y + xy³ = k (StableSwap invariant)")
print("4. This causes MASSIVE discrepancies")

print("\n📊 EXAMPLE:")
print("Dystopia pool showed:")
print("  Spot price: $0.0817 (reserves ratio)")
print("  Actual swap: $0.245 (3x worse!)")
print("  Scanner showed +$31 profit")
print("  Reality: -$32 LOSS")

print("\n" + "="*60)
print("REQUIRED FIXES:")

fixes = """
1. **Detect Stable Pools:**
   - Check for stable() function
   - Check factory address against known stable factories
   - Check for getAmountOut() function

2. **Use Correct Pricing:**
   - For stable pools, call getAmountOut() directly
   - Never use reserves ratio for stable pools
   - Implement StableSwap math if no helper function

3. **Add Pool Type Detection:**
   - Uniswap V2: Standard x*y=k
   - Stable pools: x³y + xy³ = k
   - Concentrated liquidity: Use quoter contract

4. **Verify with Simulation:**
   - Always simulate the actual swap path
   - Compare expected vs actual output
   - Flag discrepancies > 1%
"""

print(fixes)

print("\n" + "="*60)
print("IMPLEMENTATION:")

code_fix = '''
def detect_pool_type(pool_address):
    """Detect if pool is stable or volatile"""
    try:
        # Check for stable() function (Solidly/Dystopia)
        stable_abi = [{"inputs":[],"name":"stable","outputs":[{"type":"bool"}],"type":"function"}]
        contract = w3.eth.contract(address=pool_address, abi=stable_abi)
        is_stable = contract.functions.stable().call()
        return "stable" if is_stable else "volatile"
    except:
        # Check for known stable pool factories
        factory = get_factory(pool_address)
        stable_factories = [
            "0x1d21Db6cde1b18c7E47B0F7F42f4b3F68b9beeC9",  # Dystopia
            # Add Curve, Saddle, etc.
        ]
        if factory in stable_factories:
            return "stable"
    return "volatile"

def get_swap_quote(pool_address, token_in, amount_in, pool_type):
    """Get accurate swap quote based on pool type"""
    if pool_type == "stable":
        # Use getAmountOut() for stable pools
        try:
            abi = [{"inputs":[{"name":"amountIn","type":"uint256"},{"name":"tokenIn","type":"address"}],"name":"getAmountOut","outputs":[{"type":"uint256"}],"type":"function"}]
            contract = w3.eth.contract(address=pool_address, abi=abi)
            return contract.functions.getAmountOut(amount_in, token_in).call()
        except:
            # Implement stable swap math
            return calculate_stable_swap_output(amount_in, reserves)
    else:
        # Standard UniV2 math
        return (amount_in * 997 * reserve_out) // (reserve_in * 1000 + amount_in * 997)
'''

print(code_fix)

print("\n" + "="*60)
print("STABLE SWAP MATH:")

stable_math = '''
def calculate_stable_swap_output(dx, x, y, A=85):
    """
    StableSwap invariant: An²(x+y) + xy = An²D + D³/(4n²xy)
    Where A is amplification coefficient (typically 10-100)
    """
    # Simplified for 2-token pool
    D = x + y  # Simplified, actual is more complex
    
    # Newton's method to find new y
    y_new = y
    for _ in range(255):
        y_prev = y_new
        
        # f(y) = y² + (b - D)y - c = 0
        b = x + dx + D/A - D
        c = D³/(4*A*(x+dx))
        
        # Newton iteration
        y_new = (y_new² + c) / (2*y_new + b - D)
        
        if abs(y_new - y_prev) < 1:
            break
    
    return y - y_new  # Amount out
'''

print(stable_math)

print("\n" + "="*60)
print("TESTING:")
print("Verify the fix works:")

test_code = '''
# Test on Dystopia pool
dystopia = "0x380615f37993b5a96adf3d443b6e0ac50a211998"
pool_type = detect_pool_type(dystopia)
print(f"Pool type: {pool_type}")  # Should be "stable"

# Compare pricing methods
reserves = get_reserves(dystopia)
spot_price = reserves[1] / reserves[0]  # Wrong for stable!
actual_output = get_swap_quote(dystopia, usdc_old, 10*10**6, "stable")
actual_price = 10*10**6 / actual_output

print(f"Spot price: ${spot_price:.4f}")       # ~$0.082
print(f"Actual price: ${actual_price:.4f}")   # ~$0.245
print(f"Discrepancy: {actual_price/spot_price:.1f}x")  # ~3x!
'''

print(test_code)

print("\n" + "="*60)
print("SUMMARY:")
print("✅ Detect stable pools before calculating prices")
print("✅ Use getAmountOut() or stable swap math")
print("✅ Never trust spot prices for stable pools")
print("✅ Always simulate the full swap path")
print("❌ Current scanner is vulnerable to stable pool false positives")
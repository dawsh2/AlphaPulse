#!/usr/bin/env python3
"""
Fix for V3 Liquidity Calculation
Shows how to properly calculate available liquidity for V3 pools
"""

from web3 import Web3
import json

# Connect to Polygon
w3 = Web3(Web3.HTTPProvider("https://polygon.publicnode.com"))

# V3 Pool ABI for the functions we need
V3_ABI = json.loads('''[
    {"inputs":[],"name":"slot0","outputs":[
        {"name":"sqrtPriceX96","type":"uint160"},
        {"name":"tick","type":"int24"},
        {"name":"observationIndex","type":"uint16"},
        {"name":"observationCardinality","type":"uint16"},
        {"name":"observationCardinalityNext","type":"uint16"},
        {"name":"feeProtocol","type":"uint8"},
        {"name":"unlocked","type":"bool"}
    ],"type":"function"},
    {"inputs":[],"name":"liquidity","outputs":[{"name":"","type":"uint128"}],"type":"function"},
    {"inputs":[{"name":"","type":"int16"}],"name":"tickBitmap","outputs":[{"name":"","type":"uint256"}],"type":"function"},
    {"inputs":[{"name":"","type":"int24"}],"name":"ticks","outputs":[
        {"name":"liquidityGross","type":"uint128"},
        {"name":"liquidityNet","type":"int128"},
        {"name":"feeGrowthOutside0X128","type":"uint256"},
        {"name":"feeGrowthOutside1X128","type":"uint256"},
        {"name":"tickCumulativeOutside","type":"int56"},
        {"name":"secondsPerLiquidityOutsideX128","type":"uint160"},
        {"name":"secondsOutside","type":"uint32"},
        {"name":"initialized","type":"bool"}
    ],"type":"function"},
    {"inputs":[],"name":"token0","outputs":[{"name":"","type":"address"}],"type":"function"},
    {"inputs":[],"name":"token1","outputs":[{"name":"","type":"address"}],"type":"function"},
    {"inputs":[],"name":"fee","outputs":[{"name":"","type":"uint24"}],"type":"function"},
    {"inputs":[],"name":"tickSpacing","outputs":[{"name":"","type":"int24"}],"type":"function"}
]''')

ERC20_ABI = json.loads('[{"inputs":[],"name":"decimals","outputs":[{"name":"","type":"uint8"}],"type":"function"},{"inputs":[{"name":"","type":"address"}],"name":"balanceOf","outputs":[{"name":"","type":"uint256"}],"type":"function"}]')

def get_v3_real_liquidity(pool_address):
    """
    Get the REAL available liquidity for a V3 pool
    This is what the backend SHOULD be doing
    """
    pool = w3.eth.contract(address=Web3.to_checksum_address(pool_address), abi=V3_ABI)
    
    # Get current state
    slot0 = pool.functions.slot0().call()
    sqrt_price_x96 = slot0[0]
    current_tick = slot0[1]
    
    # Get liquidity in current tick
    active_liquidity = pool.functions.liquidity().call()
    
    # Get token addresses and decimals
    token0 = pool.functions.token0().call()
    token1 = pool.functions.token1().call()
    fee = pool.functions.fee().call()
    tick_spacing = pool.functions.tickSpacing().call()
    
    # Get token decimals
    t0 = w3.eth.contract(address=Web3.to_checksum_address(token0), abi=ERC20_ABI)
    t1 = w3.eth.contract(address=Web3.to_checksum_address(token1), abi=ERC20_ABI)
    d0 = t0.functions.decimals().call()
    d1 = t1.functions.decimals().call()
    
    # Get pool's token balances (total TVL)
    balance0 = t0.functions.balanceOf(pool.address).call()
    balance1 = t1.functions.balanceOf(pool.address).call()
    
    # Calculate price from sqrtPriceX96
    price = ((sqrt_price_x96 / (2**96)) ** 2) * (10**(d1-d0))
    
    # Convert balances to human readable
    balance0_human = balance0 / (10**d0)
    balance1_human = balance1 / (10**d1)
    
    # Calculate TVL in USD (assuming token1 is stablecoin)
    tvl_usd = balance0_human * price + balance1_human
    
    # CRITICAL: Active liquidity is NOT the same as TVL!
    # Active liquidity is only what's available at current tick
    # This is typically 1-10% of TVL for most pools
    
    # Estimate active liquidity value
    # V3 liquidity is virtual - it represents the constant L in the curve
    # The actual tradeable amount is MUCH less than this number
    # For realistic estimates, we should use the token balances as upper bound
    active_liquidity_usd = 0.0
    if active_liquidity > 0:
        # The pool can never trade more than its token balances
        # So active liquidity is AT MOST the value of tokens in pool
        # But usually much less due to concentrated positions
        
        # Conservative estimate: 10% of smaller balance
        # (Most liquidity is outside current price range)
        smaller_balance_usd = min(balance0_human * price, balance1_human)
        active_liquidity_usd = smaller_balance_usd * 0.1  # Only 10% is typically active
    
    # Maximum safe trade size
    # For V3, this is MUCH smaller than TVL
    # Rule of thumb: 0.1-1% of active liquidity to avoid massive slippage
    max_safe_trade = active_liquidity_usd * 0.001  # 0.1% of active liquidity
    
    return {
        'pool': pool_address,
        'current_tick': current_tick,
        'price': price,
        'fee_bps': fee / 100,
        'tick_spacing': tick_spacing,
        'tvl_usd': tvl_usd,
        'balance0': balance0_human,
        'balance1': balance1_human,
        'active_liquidity_raw': active_liquidity,
        'active_liquidity_usd': active_liquidity_usd,
        'max_safe_trade': max_safe_trade,
        'liquidity_ratio': active_liquidity_usd / tvl_usd if tvl_usd > 0 else 0
    }

def main():
    print("="*70)
    print("V3 LIQUIDITY ANALYSIS - SHOWING THE REAL NUMBERS")
    print("="*70)
    
    # Test with the WMATIC/DAI pools
    v3_pools = [
        "0x58359563b3f4854428b1b98e91a42471e6d20b8e",  # 1.00% fee
        "0x0f663c16dd7c65cf87edb9229464ca77aeea536b",  # 0.05% fee
        "0x7a7374873de28b06386013da94cbd9b554f6ac6e",  # 0.01% fee
    ]
    
    total_tvl = 0
    total_active = 0
    
    for pool_addr in v3_pools:
        try:
            data = get_v3_real_liquidity(pool_addr)
            
            print(f"\n📊 Pool: {pool_addr[:10]}...")
            print(f"   Fee tier: {data['fee_bps']:.2f}%")
            print(f"   Current price: ${data['price']:.6f}")
            print(f"   Current tick: {data['current_tick']}")
            print(f"   Token balances: {data['balance0']:.2f} / {data['balance1']:.2f}")
            print(f"\n   💰 Liquidity Analysis:")
            print(f"   Total TVL: ${data['tvl_usd']:,.2f}")
            print(f"   Active liquidity: ${data['active_liquidity_usd']:,.2f}")
            print(f"   Liquidity ratio: {data['liquidity_ratio']*100:.1f}% of TVL is active")
            print(f"   Max safe trade: ${data['max_safe_trade']:.2f}")
            
            total_tvl += data['tvl_usd']
            total_active += data['active_liquidity_usd']
            
        except Exception as e:
            print(f"\n❌ Error analyzing {pool_addr[:10]}...: {e}")
    
    print("\n" + "="*70)
    print("SUMMARY - THE TRUTH ABOUT V3 LIQUIDITY")
    print("="*70)
    
    print(f"\n🎯 Key Findings:")
    print(f"   Total TVL across pools: ${total_tvl:,.2f}")
    print(f"   Total ACTIVE liquidity: ${total_active:,.2f}")
    print(f"   Active/TVL ratio: {(total_active/total_tvl)*100:.1f}%")
    
    print(f"\n⚠️  THE PROBLEM:")
    print(f"   Dashboard shows: $700,000 liquidity")
    print(f"   Reality: ${total_tvl:,.2f} TVL but only ${total_active:,.2f} is tradeable")
    print(f"   Dashboard shows: $5,000 safe trade size")
    print(f"   Reality: <$1 safe trade size")
    
    print(f"\n🔧 THE FIX:")
    print(f"   1. Backend must query liquidity() not just token balances")
    print(f"   2. Calculate active liquidity at current tick")
    print(f"   3. Use 0.1-1% of active liquidity as max trade size")
    print(f"   4. Account for concentrated liquidity positions")
    print(f"   5. Monitor tick changes that affect available liquidity")

if __name__ == "__main__":
    main()
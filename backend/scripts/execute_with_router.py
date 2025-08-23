#!/usr/bin/env python3
"""Execute arbitrage using the correct router automatically"""

from web3 import Web3
import json
import sys

w3 = Web3(Web3.HTTPProvider('https://polygon.publicnode.com'))

# Load router registry
try:
    with open('router_registry.json', 'r') as f:
        router_registry = json.load(f)
except:
    print("❌ Router registry not found. Run setup_routers.py first")
    exit(1)

def get_router_for_pool(pool_address):
    """Get router info for a pool"""
    if pool_address in router_registry:
        return router_registry[pool_address]
    else:
        # Auto-detect
        from setup_routers import detect_pool_router
        router, dex, pool_type = detect_pool_router(pool_address)
        return {'router': router, 'dex': dex, 'type': pool_type}

def generate_execution_plan(buy_pool, sell_pool, amount_usdc):
    """Generate execution plan for arbitrage"""
    
    buy_info = get_router_for_pool(buy_pool)
    sell_info = get_router_for_pool(sell_pool)
    
    print("="*60)
    print("ARBITRAGE EXECUTION PLAN")
    print("="*60)
    
    print(f"\n1️⃣  BUY STEP:")
    print(f"   Pool: {buy_pool}")
    print(f"   DEX: {buy_info['dex']}")
    print(f"   Type: {buy_info['type']}")
    print(f"   Router: {buy_info['router']}")
    print(f"   Action: Swap {amount_usdc} USDC -> WPOL")
    
    print(f"\n2️⃣  SELL STEP:")
    print(f"   Pool: {sell_pool}")
    print(f"   DEX: {sell_info['dex']}")
    print(f"   Type: {sell_info['type']}")
    print(f"   Router: {sell_info['router']}")
    print(f"   Action: Swap WPOL -> USDC")
    
    # Generate contract parameters
    print(f"\n3️⃣  CONTRACT PARAMETERS:")
    
    router_types = {
        ('QuickSwap', 'V2'): 0,
        ('SushiSwap', 'V2'): 1,
        ('UniswapV3', 'V3'): 2,
        ('Dystopia-Stable', 'Stable'): 3,
        ('Dystopia', 'V2'): 0,  # Use QuickSwap router for regular Dystopia
    }
    
    buy_router_type = router_types.get((buy_info['dex'], buy_info['type']), 0)
    sell_router_type = router_types.get((sell_info['dex'], sell_info['type']), 0)
    
    print(f"   buyRouterType: {buy_router_type}")
    print(f"   sellRouterType: {sell_router_type}")
    
    # Generate execution code
    print(f"\n4️⃣  EXECUTION CODE:")
    print(f"""
    // Deploy UniversalArbitrage.sol first, then call:
    contract.executeArbitrage(
        0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174,  // USDC (tokenA)
        0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270,  // WPOL (tokenB)
        {int(amount_usdc * 10**6)},  // amountIn ({amount_usdc} USDC)
        {buy_pool},  // buyPool
        {buy_router_type},  // buyRouterType
        {sell_pool},  // sellPool
        {sell_router_type},  // sellRouterType
        3000  // v3Fee (0.3% if V3, ignored otherwise)
    );
    """)
    
    # Check for known issues
    print(f"\n⚠️  WARNINGS:")
    if buy_info['type'] == 'Stable' or sell_info['type'] == 'Stable':
        print("   - Stable pool detected: Ensure stable math is correct")
    if buy_info['type'] == 'V3' or sell_info['type'] == 'V3':
        print("   - V3 pool detected: May need to adjust fee tier")
    if buy_info['dex'] == 'Unknown' or sell_info['dex'] == 'Unknown':
        print("   - Unknown DEX detected: Router may not work correctly")
    
    return buy_info, sell_info

if __name__ == "__main__":
    if len(sys.argv) < 3:
        print("Usage: python3 execute_with_router.py <buy_pool> <sell_pool> [amount_usdc]")
        print("\nExample:")
        print("python3 execute_with_router.py 0x380615f37993b5a96adf3d443b6e0ac50a211998 0x6d9e8dbb2779853db00418d4dcf96f3987cfc9d2 10")
        exit(1)
    
    buy_pool = sys.argv[1]
    sell_pool = sys.argv[2]
    amount = float(sys.argv[3]) if len(sys.argv) > 3 else 10.0
    
    # Generate execution plan
    buy_info, sell_info = generate_execution_plan(buy_pool, sell_pool, amount)
    
    print("\n" + "="*60)
    print("Ready to execute!")
    print("1. Deploy UniversalArbitrage.sol")
    print("2. Fund it with USDC")
    print("3. Call executeArbitrage() with parameters above")
    print("4. Use MEV protection (high gas or Flashbots)")
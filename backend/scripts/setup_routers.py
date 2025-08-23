#!/usr/bin/env python3
"""Automatically detect and setup routers for all pools"""

from web3 import Web3
import json

w3 = Web3(Web3.HTTPProvider('https://polygon.publicnode.com'))

# Known routers and factories
ROUTERS = {
    'QuickSwap': '0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff',
    'SushiSwap': '0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506',
    'UniswapV3': '0xE592427A0AEce92De3Edee1F18E0157C05861564',
    'Dystopia': '0xbE75Dd16D029c6B32B7aD57A0FD9C1c20Dd2862e',
    'Curve': '0x74dC1C4ec10abE9F5C8A3EabF1A90b97cDc3Ead8',
    'Balancer': '0xBA12222222228d8Ba445958a75a0704d566BF2C8',
}

FACTORIES = {
    '0x5757371414417b8C6CAad45bAeF941aBc7d3Ab32': ('QuickSwap', 'V2'),
    '0xc35DADB65012eC5796536bD9864eD8773aBc74C4': ('SushiSwap', 'V2'),
    '0x1F98431c8aD98523631AE4a59f267346ea31F984': ('UniswapV3', 'V3'),
    '0x1d21Db6cde1b18c7E47B0F7F42f4b3F68b9beeC9': ('Dystopia', 'Stable/V2'),
}

def detect_pool_router(pool_address):
    """Detect which router to use for a pool"""
    pool = Web3.to_checksum_address(pool_address)
    
    # Try to get factory
    factory_abi = json.loads('[{"inputs":[],"name":"factory","outputs":[{"name":"","type":"address"}],"type":"function"}]')
    try:
        contract = w3.eth.contract(address=pool, abi=factory_abi)
        factory = contract.functions.factory().call()
        
        if factory.lower() in [f.lower() for f in FACTORIES]:
            dex_name, pool_type = FACTORIES[factory]
            
            # Check if Dystopia pool is stable
            if dex_name == 'Dystopia':
                try:
                    stable_abi = json.loads('[{"inputs":[],"name":"stable","outputs":[{"name":"","type":"bool"}],"type":"function"}]')
                    stable_contract = w3.eth.contract(address=pool, abi=stable_abi)
                    is_stable = stable_contract.functions.stable().call()
                    if is_stable:
                        return ROUTERS['Dystopia'], 'Dystopia-Stable', 'Stable'
                except:
                    pass
            
            return ROUTERS[dex_name], dex_name, pool_type
    except:
        pass
    
    # Try V3 detection
    v3_abi = json.loads('[{"inputs":[],"name":"fee","outputs":[{"name":"","type":"uint24"}],"type":"function"}]')
    try:
        contract = w3.eth.contract(address=pool, abi=v3_abi)
        fee = contract.functions.fee().call()
        if fee in [100, 500, 3000, 10000]:  # Valid V3 fees
            return ROUTERS['UniswapV3'], 'UniswapV3', 'V3'
    except:
        pass
    
    # Default to QuickSwap
    return ROUTERS['QuickSwap'], 'Unknown', 'V2'

def create_router_mapping(pools):
    """Create router mapping for multiple pools"""
    router_map = {}
    
    for pool in pools:
        router, dex, pool_type = detect_pool_router(pool)
        router_map[pool] = {
            'router': router,
            'dex': dex,
            'type': pool_type
        }
        print(f"Pool {pool[:10]}... -> {dex} ({pool_type}) via {router[:10]}...")
    
    return router_map

def generate_execution_contract(pool1, pool2, router_map):
    """Generate a custom execution contract for specific pools"""
    
    r1 = router_map.get(pool1, {})
    r2 = router_map.get(pool2, {})
    
    contract_template = f"""
// Auto-generated arbitrage contract for:
// Pool1: {pool1} ({r1.get('dex', 'Unknown')})
// Pool2: {pool2} ({r2.get('dex', 'Unknown')})

pragma solidity ^0.8.0;

contract ArbitrageExecution {{
    address constant POOL1 = {pool1};
    address constant POOL2 = {pool2};
    address constant ROUTER1 = {r1.get('router', '0x0')};
    address constant ROUTER2 = {r2.get('router', '0x0')};
    
    function execute(uint256 amount) external {{
        // Route through appropriate routers
        {generate_swap_code(r1.get('type', 'V2'), 'ROUTER1', 'POOL1')}
        {generate_swap_code(r2.get('type', 'V2'), 'ROUTER2', 'POOL2')}
    }}
}}
"""
    return contract_template

def generate_swap_code(pool_type, router_var, pool_var):
    """Generate swap code based on pool type"""
    if pool_type == 'V2':
        return f"""
        // V2 swap via {router_var}
        IUniswapV2Router({router_var}).swapExactTokensForTokens(
            amountIn, 0, path, address(this), block.timestamp
        );"""
    elif pool_type == 'V3':
        return f"""
        // V3 swap via {router_var}
        ISwapRouter({router_var}).exactInputSingle(
            ISwapRouter.ExactInputSingleParams({{
                tokenIn: tokenIn,
                tokenOut: tokenOut,
                fee: fee,
                recipient: address(this),
                deadline: block.timestamp,
                amountIn: amountIn,
                amountOutMinimum: 0,
                sqrtPriceLimitX96: 0
            }})
        );"""
    elif pool_type == 'Stable':
        return f"""
        // Stable swap via Dystopia router
        IDystopiaRouter({router_var}).swapExactTokensForTokens(
            amountIn, 0, routes, address(this), block.timestamp
        );"""
    else:
        return "// Unknown pool type - manual implementation needed"

# Example usage
if __name__ == "__main__":
    print("Router Detection System")
    print("="*60)
    
    # Test pools from previous examples
    test_pools = [
        '0x6d9e8dbb2779853db00418d4dcf96f3987cfc9d2',  # QuickSwap WPOL/USDC
        '0x380615f37993b5a96adf3d443b6e0ac50a211998',  # Dystopia WPOL/USDC
        '0xec15624fbb314eb05baad4ca49b7904c0cb6b645',  # V3 pool
        '0x6e7a5fafcec6bb1e78bae2a1f0b612012bf14827',  # V2 pool
    ]
    
    print("\nDetecting routers for pools...")
    router_map = create_router_mapping(test_pools)
    
    print("\n" + "="*60)
    print("Router Mapping Summary:")
    for pool, info in router_map.items():
        print(f"\n{pool}")
        print(f"  DEX: {info['dex']}")
        print(f"  Type: {info['type']}")
        print(f"  Router: {info['router']}")
    
    # Save to JSON for easy access
    import json
    with open('router_registry.json', 'w') as f:
        json.dump(router_map, f, indent=2)
    
    print("\n✅ Router registry saved to router_registry.json")
    
    # Generate example execution contract
    print("\n" + "="*60)
    print("Example Execution Contract:")
    if len(test_pools) >= 2:
        contract_code = generate_execution_contract(test_pools[0], test_pools[1], router_map)
        print(contract_code[:500] + "...")
        
        with open('ArbitrageExecution.sol', 'w') as f:
            f.write(contract_code)
        print("\n✅ Example contract saved to ArbitrageExecution.sol")
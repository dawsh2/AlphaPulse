#!/usr/bin/env python3
"""Fast arbitrage scanner using Redis cache for pool data"""

import redis
import json
import time
from web3 import Web3
import sys
from concurrent.futures import ThreadPoolExecutor, as_completed

# Redis connection
r = redis.Redis(host='localhost', port=6379, decode_responses=True)

# Web3 connection
w3 = Web3(Web3.HTTPProvider('https://polygon.publicnode.com'))

# Constants
USDC = '0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174'
WPOL = '0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270'
CACHE_TTL = 30  # Cache for 30 seconds

# Known DEX factories
FACTORIES = {
    '0x5757371414417b8C6CAad45bAeF941aBc7d3Ab32': 'QuickSwap',
    '0xc35DADB65012eC5796536bD9864eD8773aBc74C4': 'SushiSwap',
    '0x1F98431c8aD98523631AE4a59f267346ea31F984': 'UniswapV3',
}

def get_pool_data(pool_address):
    """Get pool data from cache or fetch if needed"""
    cache_key = f"pool:{pool_address.lower()}"
    
    # Check cache first
    cached = r.get(cache_key)
    if cached:
        return json.loads(cached)
    
    # Fetch from chain
    try:
        pool = Web3.to_checksum_address(pool_address)
        
        # Get token addresses
        token_abi = json.loads('[{"inputs":[],"name":"token0","outputs":[{"name":"","type":"address"}],"type":"function"},{"inputs":[],"name":"token1","outputs":[{"name":"","type":"address"}],"type":"function"}]')
        contract = w3.eth.contract(address=pool, abi=token_abi)
        token0 = contract.functions.token0().call()
        token1 = contract.functions.token1().call()
        
        # Check if USDC/WPOL pair
        is_usdc_wpol = (token0.lower() in [USDC.lower(), WPOL.lower()] and 
                        token1.lower() in [USDC.lower(), WPOL.lower()])
        
        if not is_usdc_wpol:
            return None
            
        # Get reserves
        reserves_abi = json.loads('[{"inputs":[],"name":"getReserves","outputs":[{"name":"reserve0","type":"uint112"},{"name":"reserve1","type":"uint112"},{"name":"blockTimestampLast","type":"uint32"}],"type":"function"}]')
        contract = w3.eth.contract(address=pool, abi=reserves_abi)
        reserves = contract.functions.getReserves().call()
        
        # Determine which reserve is USDC
        if token0.lower() == USDC.lower():
            usdc_reserve = reserves[0] / 10**6
            wpol_reserve = reserves[1] / 10**18
        else:
            usdc_reserve = reserves[1] / 10**6
            wpol_reserve = reserves[0] / 10**18
            
        pool_data = {
            'address': pool_address.lower(),
            'token0': token0,
            'token1': token1,
            'usdc_reserve': usdc_reserve,
            'wpol_reserve': wpol_reserve,
            'price': usdc_reserve / wpol_reserve if wpol_reserve > 0 else 0,
            'timestamp': time.time()
        }
        
        # Cache it
        r.setex(cache_key, CACHE_TTL, json.dumps(pool_data))
        
        return pool_data
        
    except Exception as e:
        return None

def find_arbitrage_opportunities():
    """Find all profitable arbitrage opportunities"""
    print("🔍 Scanning for arbitrage opportunities...")
    start_time = time.time()
    
    # Get all pools (from a predefined list or discovery)
    # For now, using some known pools
    known_pools = [
        '0x4db1087154cd5b33fa275a88b183619f1a6f6614',
        '0x9b08288c3be4f62bbf8d1c20ac9c5e6f9467d8b7',
        '0xbf68e75977f3823618725cc3f6ae4c59498593f0',
        '0x55ff76bffc3cdd9d5fdbbc2ece4528ecce45047e',
        '0xfd5c2eda66f7cfcfcbf87b6b1d6c395853e323a9',
        '0x3bfcb475e528f54246f1847ec0e7b53dd88bda4e',
        '0x604229c960e5cacf2aaeac8be68ac07ba9df81c3',
        '0x781067ef296e5c4a4203f81c593274824b7c185d',
        '0x65d43b64e3b31965cd5ea367d4c2b94c03084797',
    ]
    
    # Fetch pool data in parallel
    pool_data = {}
    with ThreadPoolExecutor(max_workers=10) as executor:
        futures = {executor.submit(get_pool_data, pool): pool for pool in known_pools}
        
        for future in as_completed(futures):
            pool = futures[future]
            data = future.result()
            if data and data['price'] > 0:
                pool_data[pool] = data
    
    print(f"✅ Loaded {len(pool_data)} USDC/WPOL pools in {time.time() - start_time:.2f}s")
    
    # Find arbitrage opportunities
    opportunities = []
    pools = list(pool_data.values())
    
    for i, pool1 in enumerate(pools):
        for pool2 in pools[i+1:]:
            price1 = pool1['price']
            price2 = pool2['price']
            
            if price1 <= 0 or price2 <= 0:
                continue
                
            # Calculate spread
            spread = abs(price1 - price2) / min(price1, price2) * 100
            
            if spread > 0.1:  # 0.1% minimum
                # Determine buy/sell direction
                if price1 < price2:
                    buy_pool = pool1
                    sell_pool = pool2
                else:
                    buy_pool = pool2
                    sell_pool = pool1
                
                # Calculate optimal trade size
                # Simplified: use 1% of smaller pool's liquidity
                min_liquidity = min(
                    buy_pool['usdc_reserve'],
                    sell_pool['usdc_reserve']
                )
                trade_size = min(min_liquidity * 0.01, 1000)  # Cap at $1000
                
                # Calculate profit (simplified, not accounting for fees/slippage)
                profit = trade_size * spread / 100 * 0.9  # Assume 10% costs
                
                if profit > 0.001:  # $0.001 minimum
                    opportunities.append({
                        'buy': buy_pool['address'],
                        'sell': sell_pool['address'],
                        'spread': spread,
                        'size': trade_size,
                        'profit': profit,
                        'buy_price': buy_pool['price'],
                        'sell_price': sell_pool['price']
                    })
    
    # Sort by profit
    opportunities.sort(key=lambda x: x['profit'], reverse=True)
    
    # Store in Redis
    r.delete('arb:opportunities')
    for opp in opportunities:
        r.rpush('arb:opportunities', json.dumps(opp))
    r.expire('arb:opportunities', 60)
    
    return opportunities

def main():
    """Main function"""
    opportunities = find_arbitrage_opportunities()
    
    print("\n" + "="*70)
    print("PROFITABLE ARBITRAGE OPPORTUNITIES (CACHED)")
    print("="*70)
    
    if opportunities:
        print(f"\n✅ Found {len(opportunities)} profitable opportunities:\n")
        
        for opp in opportunities[:10]:  # Show top 10
            print(f"Buy:  {opp['buy']}")
            print(f"Sell: {opp['sell']}")
            print(f"Net Profit: ${opp['profit']:.4f} (spread: {opp['spread']:.3f}%, size: ${opp['size']:.2f})")
            print()
    else:
        print("\n❌ No profitable opportunities found")
    
    print(f"💾 Results cached in Redis (key: 'arb:opportunities')")
    print("🚀 Access from any process instantly!")

if __name__ == "__main__":
    main()
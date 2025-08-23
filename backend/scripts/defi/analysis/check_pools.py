#!/usr/bin/env python3
from web3 import Web3
import json

w3 = Web3(Web3.HTTPProvider('https://polygon.publicnode.com'))

buy_pool = '0xd1c77e3b20760b15dc0387d07da3748745b27a5f'
sell_pool = '0x234f4264b907677356cc82c451dc5a073ba7c44c'

print("Checking pools...")

# Check both pools
for name, pool in [("Buy", buy_pool), ("Sell", sell_pool)]:
    print(f"\n{name} Pool: {pool}")
    
    # Check if contract exists
    code = w3.eth.get_code(Web3.to_checksum_address(pool))
    if code == b'':
        print(f"  ❌ No contract at this address!")
        continue
    else:
        print(f"  ✅ Contract exists")
    
    # Try to get token addresses
    try:
        token_abi = json.loads('[{"inputs":[],"name":"token0","outputs":[{"name":"","type":"address"}],"type":"function"},{"inputs":[],"name":"token1","outputs":[{"name":"","type":"address"}],"type":"function"}]')
        contract = w3.eth.contract(address=Web3.to_checksum_address(pool), abi=token_abi)
        token0 = contract.functions.token0().call()
        token1 = contract.functions.token1().call()
        print(f"  Token0: {token0}")
        print(f"  Token1: {token1}")
        
        # Check if USDC/WPOL pair
        USDC = '0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174'
        WPOL = '0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270'
        
        if (token0.lower() == USDC.lower() or token1.lower() == USDC.lower()) and \
           (token0.lower() == WPOL.lower() or token1.lower() == WPOL.lower()):
            print(f"  ✅ USDC/WPOL pair confirmed")
        else:
            print(f"  ⚠️ Not USDC/WPOL pair")
            
    except Exception as e:
        print(f"  Error getting tokens: {e}")
    
    # Check reserves
    try:
        reserves_abi = json.loads('[{"inputs":[],"name":"getReserves","outputs":[{"name":"reserve0","type":"uint112"},{"name":"reserve1","type":"uint112"},{"name":"blockTimestampLast","type":"uint32"}],"type":"function"}]')
        contract = w3.eth.contract(address=Web3.to_checksum_address(pool), abi=reserves_abi)
        reserves = contract.functions.getReserves().call()
        print(f"  Reserve0: {reserves[0]}")
        print(f"  Reserve1: {reserves[1]}")
        
        if reserves[0] == 0 or reserves[1] == 0:
            print(f"  ⚠️ Pool has no liquidity!")
    except:
        # Try V3 liquidity
        try:
            liq_abi = json.loads('[{"inputs":[],"name":"liquidity","outputs":[{"name":"","type":"uint128"}],"type":"function"}]')
            contract = w3.eth.contract(address=Web3.to_checksum_address(pool), abi=liq_abi)
            liquidity = contract.functions.liquidity().call()
            print(f"  V3 Liquidity: {liquidity}")
            if liquidity == 0:
                print(f"  ⚠️ V3 pool has no liquidity!")
        except:
            print(f"  Could not get reserves/liquidity")

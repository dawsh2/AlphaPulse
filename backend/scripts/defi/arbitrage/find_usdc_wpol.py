#!/usr/bin/env python3
from web3 import Web3
import json

w3 = Web3(Web3.HTTPProvider('https://polygon.publicnode.com'))

USDC = '0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174'
WPOL = '0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270'

# Check one of the "profitable" pools
pool = '0x4db1087154cd5b33fa275a88b183619f1a6f6614'

token_abi = json.loads('[{"inputs":[],"name":"token0","outputs":[{"name":"","type":"address"}],"type":"function"},{"inputs":[],"name":"token1","outputs":[{"name":"","type":"address"}],"type":"function"}]')
contract = w3.eth.contract(address=Web3.to_checksum_address(pool), abi=token_abi)

token0 = contract.functions.token0().call()
token1 = contract.functions.token1().call()

print(f"Pool: {pool}")
print(f"Token0: {token0}")
print(f"Token1: {token1}")
print(f"Is USDC? {token0.lower() == USDC.lower() or token1.lower() == USDC.lower()}")
print(f"Is WPOL? {token0.lower() == WPOL.lower() or token1.lower() == WPOL.lower()}")

# Get some real USDC/WPOL pools
print("\n🔍 Finding real USDC/WPOL pools...")

# QuickSwap V2 factory
factory = '0x5757371414417b8C6CAad45bAeF941aBc7d3Ab32'
factory_abi = json.loads('[{"inputs":[{"name":"tokenA","type":"address"},{"name":"tokenB","type":"address"}],"name":"getPair","outputs":[{"name":"pair","type":"address"}],"type":"function"}]')
factory_contract = w3.eth.contract(address=Web3.to_checksum_address(factory), abi=factory_abi)

pair = factory_contract.functions.getPair(
    Web3.to_checksum_address(USDC),
    Web3.to_checksum_address(WPOL)
).call()

if pair != '0x0000000000000000000000000000000000000000':
    print(f"✅ QuickSwap USDC/WPOL: {pair}")
    
    # Check reserves
    reserves_abi = json.loads('[{"inputs":[],"name":"getReserves","outputs":[{"name":"reserve0","type":"uint112"},{"name":"reserve1","type":"uint112"},{"name":"blockTimestampLast","type":"uint32"}],"type":"function"}]')
    pair_contract = w3.eth.contract(address=Web3.to_checksum_address(pair), abi=reserves_abi)
    reserves = pair_contract.functions.getReserves().call()
    print(f"   Reserves: {reserves[0]/10**6:.2f} / {reserves[1]/10**18:.2f}")

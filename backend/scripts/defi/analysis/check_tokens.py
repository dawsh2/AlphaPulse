#!/usr/bin/env python3
from web3 import Web3
import json

w3 = Web3(Web3.HTTPProvider('https://polygon.publicnode.com'))

# Token addresses
tokens = {
    '0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174': 'USDC.e (bridged)',
    '0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359': 'USDC (native)',
    '0xc2132D05D31c914a87C6611C10748AEb04B58e8F': 'USDT',
    '0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270': 'WMATIC/WPOL',
}

print("Token Addresses on Polygon:")
for addr, name in tokens.items():
    print(f"{name}: {addr}")

print("\nThe scanner is finding WPOL/USDT arbitrage, not WPOL/USDC!")
print("USDT address: 0xc2132D05D31c914a87C6611C10748AEb04B58e8F")

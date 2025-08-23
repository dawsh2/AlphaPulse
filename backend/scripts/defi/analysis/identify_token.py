#!/usr/bin/env python3
from web3 import Web3
import json

w3 = Web3(Web3.HTTPProvider('https://polygon.publicnode.com'))

# Mystery token
token = '0xB178f7A15fA2349B4495d805Bce46ad5c6231415'

erc20_abi = json.loads('[{"inputs":[],"name":"symbol","outputs":[{"name":"","type":"string"}],"type":"function"},{"inputs":[],"name":"name","outputs":[{"name":"","type":"string"}],"type":"function"},{"inputs":[],"name":"decimals","outputs":[{"name":"","type":"uint8"}],"type":"function"}]')

contract = w3.eth.contract(address=Web3.to_checksum_address(token), abi=erc20_abi)

name = contract.functions.name().call()
symbol = contract.functions.symbol().call()
decimals = contract.functions.decimals().call()

print(f"Token: {token}")
print(f"Name: {name}")
print(f"Symbol: {symbol}")
print(f"Decimals: {decimals}")

print("\nSo the pools are WPOL/{symbol}, not WPOL/USDC!")
print("The scanner found arbitrage between two WPOL/{symbol} pools with different prices.")
print("\nTo execute this, you'd need to:")
print(f"1. Swap USDC → {symbol}")
print(f"2. Execute the WPOL/{symbol} arbitrage")
print(f"3. Swap {symbol} → USDC")

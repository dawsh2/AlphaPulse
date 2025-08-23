#!/usr/bin/env python3
from web3 import Web3
import json

w3 = Web3(Web3.HTTPProvider('https://polygon.publicnode.com'))

pool = '0x21988c9cfd08db3b5793c2c6782271dc94749251'
v3_abi = json.loads('[{"inputs":[],"name":"fee","outputs":[{"name":"","type":"uint24"}],"type":"function"}]')

try:
    contract = w3.eth.contract(address=Web3.to_checksum_address(pool), abi=v3_abi)
    fee = contract.functions.fee().call()
    print(f"V3 Pool Fee: {fee} ({fee/10000}%)")
    if fee == 100:
        print("This is a 0.01% fee tier pool")
    elif fee == 500:
        print("This is a 0.05% fee tier pool")
    elif fee == 3000:
        print("This is a 0.3% fee tier pool")
    elif fee == 10000:
        print("This is a 1% fee tier pool")
except Exception as e:
    print(f"Error: {e}")

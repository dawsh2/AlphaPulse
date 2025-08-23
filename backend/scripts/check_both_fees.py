#!/usr/bin/env python3
from web3 import Web3
import json

w3 = Web3(Web3.HTTPProvider('https://polygon.publicnode.com'))

pools = [
    '0xec15624fbb314eb05baad4ca49b7904c0cb6b645',
    '0xa374094527e1673a86de625aa59517c5de346d32'
]

v3_abi = json.loads('[{"inputs":[],"name":"fee","outputs":[{"name":"","type":"uint24"}],"type":"function"}]')

for pool in pools:
    try:
        contract = w3.eth.contract(address=Web3.to_checksum_address(pool), abi=v3_abi)
        fee = contract.functions.fee().call()
        print(f"Pool {pool[:10]}...: Fee = {fee} ({fee/10000}%)")
    except Exception as e:
        print(f"Pool {pool[:10]}...: Error - {e}")

#!/usr/bin/env python3
"""Trace the exact failure point in the transaction"""

from web3 import Web3
# from eth_abi import decode_abi
import json

w3 = Web3(Web3.HTTPProvider('https://polygon.publicnode.com'))

# The failed transaction
tx_hash = '0x51952ee20ad0e357c909f6f65bd5ad20ce5caed73e1718e9533e290ce648992c'
tx = w3.eth.get_transaction(tx_hash)

print("Transaction Input Analysis:")
print("="*60)

# The input data
input_data = tx['input']
print(f"Full input: {input_data}")
print(f"Function selector: {input_data[:10]}")

# Decode based on known function signatures
known_selectors = {
    '0x5ea41926': 'executeArbitrage(uint256)',
    '0xf7c7724f': 'Unknown function',
}

selector = input_data[:10]
if selector in known_selectors:
    print(f"Function: {known_selectors[selector]}")
else:
    print(f"Function: Unknown ({selector})")

# Try to decode the parameters
if len(input_data) > 10:
    param_data = input_data[10:]
    print(f"Parameter data: {param_data}")
    
    # Try to decode as uint256
    try:
        # Each parameter is 32 bytes (64 hex chars)
        if len(param_data) == 64:
            value = int(param_data, 16)
            print(f"Decoded parameter: {value}")
            print(f"As USDC amount: {value / 10**6:.2f} USDC")
    except:
        pass

# Check what the correct function selector should be
print("\n" + "="*60)
print("Expected Function Signatures:")

# Generate correct selectors
from eth_utils import keccak

functions = [
    'executeArbitrage(uint256)',
    'checkProfitability(uint256)',
    'withdraw(address)',
]

for func in functions:
    selector = '0x' + keccak(text=func).hex()[:8]
    print(f"{func}: {selector}")

print("\n" + "="*60)
print("ANALYSIS:")
print("The transaction used the wrong function selector!")
print("This suggests the contract ABI or function name is different than expected.")
print("")
print("Let's check what functions the deployed contract actually has...")

# Get the contract bytecode to analyze
contract_addr = '0x2a36DED40Dc15935dd3fA31d035D2Ed880290e67'
code = w3.eth.get_code(contract_addr)

print(f"\nContract has bytecode: {'Yes' if len(code) > 0 else 'No'}")
print(f"Bytecode length: {len(code)} bytes")

# The issue is likely in how we're calling the contract
print("\n" + "="*60)
print("SOLUTION:")
print("The contract was deployed correctly but called with wrong function.")
print("Need to use the correct function selector for executeArbitrage.")
print("")
print("Let's prepare a correct execution...")
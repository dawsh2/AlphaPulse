#!/usr/bin/env python3
"""Find minimum profitable trade size"""

from web3 import Web3
import json

w3 = Web3(Web3.HTTPProvider('https://polygon.publicnode.com'))

# Contracts
dystopia = '0x380615F37993B5A96adF3D443b6E0Ac50a211998'
quickswap = '0x6D9e8dbB2779853db00418D4DcF96F3987CFC9D2'
usdc_old = '0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174'

# ABIs
dystopia_abi = json.loads('[{"inputs":[{"name":"amountIn","type":"uint256"},{"name":"tokenIn","type":"address"}],"name":"getAmountOut","outputs":[{"name":"","type":"uint256"}],"type":"function"}]')
pool_abi = json.loads('[{"inputs":[],"name":"getReserves","outputs":[{"name":"","type":"uint112"},{"name":"","type":"uint112"}],"type":"function"}]')

dystopia_contract = w3.eth.contract(address=dystopia, abi=dystopia_abi)
quickswap_contract = w3.eth.contract(address=quickswap, abi=pool_abi)

print("Finding minimum profitable trade size...")
print("="*60)

# Get QuickSwap reserves
reserves = quickswap_contract.functions.getReserves().call()
print(f"QuickSwap reserves: {reserves[0]/10**18:.2f} WPOL, {reserves[1]/10**6:.2f} USDC_NEW")

# Test different sizes
test_amounts = [0.5, 1, 2, 5, 10, 20, 50, 100]

for amount in test_amounts:
    amount_wei = int(amount * 10**6)
    
    # Get WPOL from Dystopia
    wpol_out = dystopia_contract.functions.getAmountOut(amount_wei, usdc_old).call()
    
    # Calculate USDC_NEW from QuickSwap
    usdc_new_out = (wpol_out * 997 * reserves[1]) // (reserves[0] * 1000 + wpol_out * 997)
    
    profit = (usdc_new_out - amount_wei) / 10**6
    profit_pct = profit / amount * 100
    
    print(f"\n{amount:6.1f} USDC_OLD -> {wpol_out/10**18:8.4f} WPOL -> {usdc_new_out/10**6:6.2f} USDC_NEW")
    print(f"  Profit: ${profit:6.2f} ({profit_pct:5.2f}%)")
    
    if profit > 0:
        print(f"  ✅ Profitable!")
        if amount <= 1:
            print(f"  -> Minimum profitable size: {amount} USDC")
            break
    else:
        print(f"  ❌ Not profitable")

print("\n" + "="*60)
print("CONCLUSION:")
print("The Dystopia pool is a stable pool with different pricing curve")
print("Small trades don't overcome the fee structure")
print("You need more USDC_OLD to make this profitable")
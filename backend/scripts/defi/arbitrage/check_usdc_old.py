#!/usr/bin/env python3
from web3 import Web3
import json
import os
from eth_account import Account

w3 = Web3(Web3.HTTPProvider('https://polygon.publicnode.com'))

# USDC addresses
USDC_OLD = '0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174'  # USDC.e (bridged)
USDC_NEW = '0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359'  # Native USDC

private_key = os.getenv('PRIVATE_KEY')
if not private_key:
    print("Please set PRIVATE_KEY environment variable")
    # Check a default address if no private key
    check_address = input("Enter wallet address to check: ")
else:
    account = Account.from_key(private_key)
    check_address = account.address
    print(f"Checking wallet: {check_address}")

# Check both USDC balances
erc20_abi = json.loads('[{"inputs":[{"name":"account","type":"address"}],"name":"balanceOf","outputs":[{"name":"","type":"uint256"}],"type":"function"},{"inputs":[],"name":"symbol","outputs":[{"name":"","type":"string"}],"type":"function"},{"inputs":[],"name":"name","outputs":[{"name":"","type":"string"}],"type":"function"}]')

print("\n" + "="*60)
print("USDC BALANCES ON POLYGON")
print("="*60)

# Check USDC.e (OLD - Bridged USDC)
usdc_old = w3.eth.contract(address=Web3.to_checksum_address(USDC_OLD), abi=erc20_abi)
balance_old = usdc_old.functions.balanceOf(Web3.to_checksum_address(check_address)).call()
name_old = usdc_old.functions.name().call()
symbol_old = usdc_old.functions.symbol().call()

print(f"\n🔵 USDC.e (Bridged/OLD)")
print(f"   Contract: {USDC_OLD}")
print(f"   Name: {name_old}")
print(f"   Symbol: {symbol_old}")
print(f"   Balance: {balance_old / 10**6:.6f} USDC.e")
print(f"   Raw: {balance_old}")

# Check native USDC (NEW)
usdc_new = w3.eth.contract(address=Web3.to_checksum_address(USDC_NEW), abi=erc20_abi)
balance_new = usdc_new.functions.balanceOf(Web3.to_checksum_address(check_address)).call()
name_new = usdc_new.functions.name().call()
symbol_new = usdc_new.functions.symbol().call()

print(f"\n🟢 USDC (Native/NEW)")
print(f"   Contract: {USDC_NEW}")
print(f"   Name: {name_new}")
print(f"   Symbol: {symbol_new}")
print(f"   Balance: {balance_new / 10**6:.6f} USDC")
print(f"   Raw: {balance_new}")

# Check MATIC balance too
matic_balance = w3.eth.get_balance(Web3.to_checksum_address(check_address))
print(f"\n⛽ MATIC")
print(f"   Balance: {Web3.from_wei(matic_balance, 'ether')} MATIC")

print("\n" + "="*60)
print("SUMMARY")
print("="*60)
total_usd = (balance_old / 10**6) + (balance_new / 10**6)
print(f"Total USD value: ${total_usd:.2f}")

if balance_old > 0:
    print(f"\n✅ You have {balance_old / 10**6:.6f} USDC.e (OLD)")
    print("This is the bridged USDC from Ethereum")
    print("Most Polygon DEXs still use this version")
else:
    print("\n❌ No USDC.e (OLD) balance found")
    
if balance_new > 0:
    print(f"\n✅ You have {balance_new / 10**6:.6f} native USDC (NEW)")
    print("This is the new native USDC on Polygon")
    print("Some newer pools use this version")
else:
    print("\n❌ No native USDC (NEW) balance found")

# Show how to swap between them if needed
if balance_old > 0 or balance_new > 0:
    print("\n" + "="*60)
    print("SWAP BETWEEN USDC VERSIONS")
    print("="*60)
    print("You can swap 1:1 between USDC.e and native USDC at:")
    print("• Uniswap V3: https://app.uniswap.org/swap")
    print("• QuickSwap: https://quickswap.exchange")
    print("• Or use the cross-USDC arbitrage we identified earlier!")

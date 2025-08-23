#!/usr/bin/env python3
"""Deploy SimplestArbitrage contract and execute the trade"""

from web3 import Web3
from eth_account import Account
import json
import os
import time
from dotenv import load_dotenv
from solcx import compile_source, install_solc, set_solc_version

# Set Solidity compiler version
try:
    set_solc_version('0.8.19')
except:
    install_solc('0.8.19')
    set_solc_version('0.8.19')

# Load environment
load_dotenv('/Users/daws/alphapulse/backend/services/capital_arb_bot/.env')
private_key = os.getenv('PRIVATE_KEY')

if not private_key:
    print("ERROR: No PRIVATE_KEY found in .env")
    exit(1)

w3 = Web3(Web3.HTTPProvider('https://polygon.publicnode.com'))
account = Account.from_key(private_key)
address = account.address

print(f"Account: {address}")
balance = w3.eth.get_balance(address)
print(f"Balance: {Web3.from_wei(balance, 'ether'):.4f} MATIC")

# Check current opportunity
print("\n" + "="*60)
print("Checking current arbitrage opportunity...")

pool_abi = json.loads('[{"inputs":[],"name":"getReserves","outputs":[{"name":"","type":"uint112"},{"name":"","type":"uint112"}],"type":"function"}]')

buy_pool = '0x380615f37993b5a96adf3d443b6e0ac50a211998'
sell_pool = '0x6d9e8dbb2779853db00418d4dcf96f3987cfc9d2'

buy_contract = w3.eth.contract(address=Web3.to_checksum_address(buy_pool), abi=pool_abi)
sell_contract = w3.eth.contract(address=Web3.to_checksum_address(sell_pool), abi=pool_abi)

buy_reserves = buy_contract.functions.getReserves().call()
sell_reserves = sell_contract.functions.getReserves().call()

wpol_price_buy = buy_reserves[1] / buy_reserves[0] * 10**12
wpol_price_sell = sell_reserves[1] / sell_reserves[0] * 10**12

print(f"Buy pool: WPOL @ ${wpol_price_buy:.6f}")
print(f"Sell pool: WPOL @ ${wpol_price_sell:.6f}")
print(f"Spread: {(wpol_price_sell/wpol_price_buy - 1)*100:.2f}%")

# Use 4 USDC (keeping 0.82 as buffer)
test_amount = 4 * 10**6  # 4 USDC
fee = 0.997
wpol_out = (test_amount * fee * buy_reserves[0]) // (buy_reserves[1] + test_amount * fee)
usdc_new_out = (wpol_out * fee * sell_reserves[1]) // (sell_reserves[0] + wpol_out * fee)
profit = (usdc_new_out - test_amount) / 10**6

print(f"\nSimulation with {test_amount/10**6:.0f} USDC_OLD:")
print(f"  Expected output: {usdc_new_out/10**6:.2f} USDC_NEW")
print(f"  Profit: ${profit:.2f}")

if profit < 0.5:
    print("\n❌ Profit too small, not worth gas costs")
    exit(1)

# Check USDC_OLD balance
print("\n" + "="*60)
print("Checking USDC_OLD balance...")

usdc_old = '0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174'
usdc_abi = json.loads('[{"inputs":[{"name":"account","type":"address"}],"name":"balanceOf","outputs":[{"name":"","type":"uint256"}],"type":"function"},{"inputs":[{"name":"spender","type":"address"},{"name":"amount","type":"uint256"}],"name":"approve","outputs":[{"name":"","type":"bool"}],"type":"function"},{"inputs":[{"name":"to","type":"address"},{"name":"amount","type":"uint256"}],"name":"transfer","outputs":[{"name":"","type":"bool"}],"type":"function"}]')

usdc_contract = w3.eth.contract(address=Web3.to_checksum_address(usdc_old), abi=usdc_abi)
usdc_balance = usdc_contract.functions.balanceOf(address).call()

print(f"Your USDC_OLD balance: {usdc_balance/10**6:.2f}")

if usdc_balance < test_amount:
    print("\n⚠️  You need USDC_OLD to execute this arbitrage")
    print("Options:")
    print("1. Buy USDC_OLD on any DEX")
    print("2. Bridge USDC from Ethereum")
    print("3. Swap some MATIC for USDC_OLD")
    
    # Offer to swap MATIC for USDC
    print("\n" + "="*60)
    print("Option: Swap MATIC for USDC_OLD")
    
    # QuickSwap router
    router = '0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff'
    matic_price = 0.5  # Approximate
    matic_needed = 25 / matic_price  # For 25 USDC worth
    
    print(f"Would need ~{matic_needed:.2f} MATIC to get 25 USDC")
    print("Execute swap? (Would need separate transaction)")
    exit(0)

# Compile contract
print("\n" + "="*60)
print("Compiling contract...")

with open('SimplestArbitrage.sol', 'r') as f:
    contract_source = f.read()

compiled = compile_source(contract_source, output_values=['abi', 'bin'])
contract_id, contract_interface = compiled.popitem()

bytecode = contract_interface['bin']
abi = contract_interface['abi']

print(f"Contract compiled successfully")
print(f"Bytecode size: {len(bytecode)} bytes")

# Deploy contract
print("\n" + "="*60)
print("Deploying contract...")

Contract = w3.eth.contract(abi=abi, bytecode=bytecode)
nonce = w3.eth.get_transaction_count(address)
gas_price = w3.eth.gas_price

tx = Contract.constructor().build_transaction({
    'from': address,
    'nonce': nonce,
    'gas': 1000000,
    'gasPrice': gas_price,
})

signed_tx = account.sign_transaction(tx)
tx_hash = w3.eth.send_raw_transaction(signed_tx.raw_transaction)

print(f"Deploy tx: {tx_hash.hex()}")
print("Waiting for confirmation...")

receipt = w3.eth.wait_for_transaction_receipt(tx_hash)
contract_address = receipt.contractAddress

print(f"✅ Contract deployed at: {contract_address}")
print(f"Gas used: {receipt.gasUsed:,}")

# Get contract instance
arb_contract = w3.eth.contract(address=contract_address, abi=abi)

# Send USDC_OLD to contract
print("\n" + "="*60)
print(f"Sending {test_amount/10**6} USDC_OLD to contract...")

nonce = w3.eth.get_transaction_count(address)
tx = usdc_contract.functions.transfer(
    contract_address,
    test_amount
).build_transaction({
    'from': address,
    'nonce': nonce,
    'gas': 100000,
    'gasPrice': gas_price,
})

signed_tx = account.sign_transaction(tx)
tx_hash = w3.eth.send_raw_transaction(signed_tx.raw_transaction)

print(f"Transfer tx: {tx_hash.hex()}")
receipt = w3.eth.wait_for_transaction_receipt(tx_hash)

if receipt.status == 1:
    print("✅ USDC transferred successfully")
else:
    print("❌ Transfer failed")
    exit(1)

# Execute arbitrage
print("\n" + "="*60)
print("Executing arbitrage...")

# Check USDC_NEW balance before
usdc_new = '0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359'
usdc_new_contract = w3.eth.contract(address=Web3.to_checksum_address(usdc_new), abi=usdc_abi)
before_balance = usdc_new_contract.functions.balanceOf(address).call()

nonce = w3.eth.get_transaction_count(address)
tx = arb_contract.functions.executeArbitrage().build_transaction({
    'from': address,
    'nonce': nonce,
    'gas': 300000,
    'gasPrice': gas_price,
})

signed_tx = account.sign_transaction(tx)
tx_hash = w3.eth.send_raw_transaction(signed_tx.raw_transaction)

print(f"Arbitrage tx: {tx_hash.hex()}")
print("Waiting for confirmation...")

receipt = w3.eth.wait_for_transaction_receipt(tx_hash)

if receipt.status == 1:
    print("✅ Arbitrage executed successfully!")
    
    # Check profit
    after_balance = usdc_new_contract.functions.balanceOf(address).call()
    profit_received = (after_balance - before_balance) / 10**6
    
    print(f"\n💰 RESULTS:")
    print(f"   Invested: {test_amount/10**6:.2f} USDC_OLD")
    print(f"   Received: {profit_received:.2f} USDC_NEW")
    print(f"   Profit: ${profit_received - test_amount/10**6:.2f}")
    print(f"   Gas used: {receipt.gasUsed:,}")
    
    # Calculate ROI
    gas_cost = receipt.gasUsed * gas_price / 10**18 * 0.5  # MATIC price
    net_profit = profit_received - test_amount/10**6 - gas_cost
    roi = (net_profit / (test_amount/10**6)) * 100
    
    print(f"   Gas cost: ${gas_cost:.2f}")
    print(f"   Net profit: ${net_profit:.2f}")
    print(f"   ROI: {roi:.1f}%")
else:
    print("❌ Arbitrage failed")
    print(f"Check tx: https://polygonscan.com/tx/{tx_hash.hex()}")
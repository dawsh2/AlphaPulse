#!/usr/bin/env python3
"""Deploy Flash Arbitrage contract for capital-free execution"""

from web3 import Web3
import json
import os
from eth_account import Account
from solcx import compile_source, install_solc

# Install solc if needed
try:
    install_solc('0.8.19')
except:
    pass

w3 = Web3(Web3.HTTPProvider('https://polygon.publicnode.com'))

private_key = os.getenv('PRIVATE_KEY')
if not private_key:
    print("❌ Please set PRIVATE_KEY environment variable")
    exit(1)

account = Account.from_key(private_key)
print(f"🔑 Deploying from: {account.address}")

# Check balance
balance = w3.eth.get_balance(account.address)
print(f"💰 MATIC Balance: {Web3.from_wei(balance, 'ether')} MATIC")

if balance < Web3.to_wei(0.01, 'ether'):
    print("❌ Insufficient MATIC for deployment. Need at least 0.01 MATIC")
    exit(1)

# Read contract
with open('FlashArbitrage.sol', 'r') as f:
    contract_source = f.read()

# Compile
print("📦 Compiling contract...")
compiled = compile_source(contract_source, output_values=['abi', 'bin'])
contract_id, contract_interface = compiled.popitem()

bytecode = contract_interface['bin']
abi = contract_interface['abi']

# Deploy
Contract = w3.eth.contract(abi=abi, bytecode=bytecode)

print("🔨 Deploying Flash Arbitrage contract...")
nonce = w3.eth.get_transaction_count(account.address)
gas_price = int(w3.eth.gas_price * 1.5)

transaction = Contract.constructor().build_transaction({
    'from': account.address,
    'nonce': nonce,
    'gasPrice': gas_price,
    'gas': 3000000,
    'chainId': 137
})

signed = account.sign_transaction(transaction)
tx_hash = w3.eth.send_raw_transaction(signed.rawTransaction)
print(f"📝 TX: {tx_hash.hex()}")

print("⏳ Waiting for confirmation...")
receipt = w3.eth.wait_for_transaction_receipt(tx_hash)

if receipt.status == 1:
    contract_address = receipt.contractAddress
    print(f"✅ Flash Arbitrage deployed at: {contract_address}")
    
    # Save deployment
    deployment_info = {
        'address': contract_address,
        'abi': abi,
        'owner': account.address,
        'tx_hash': tx_hash.hex()
    }
    
    with open('flash_arb_deployment.json', 'w') as f:
        json.dump(deployment_info, f, indent=2)
    
    print("💾 Saved to flash_arb_deployment.json")
    print("\n✨ Now you can execute arbitrage with NO CAPITAL!")
else:
    print("❌ Deployment failed!")

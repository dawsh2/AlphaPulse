#!/usr/bin/env python3
"""Deploy UniversalArbitrage contract to Polygon"""

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

# Connect to Polygon
w3 = Web3(Web3.HTTPProvider('https://polygon.publicnode.com'))

# Load private key
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

# Read contract source
with open('UniversalArbitrage.sol', 'r') as f:
    contract_source = f.read()

# Compile contract
print("📦 Compiling contract...")
compiled = compile_source(contract_source, output_values=['abi', 'bin'])
contract_id, contract_interface = compiled.popitem()

# Get bytecode and ABI
bytecode = contract_interface['bin']
abi = contract_interface['abi']

# Create contract instance
Contract = w3.eth.contract(abi=abi, bytecode=bytecode)

# Build transaction
print("🔨 Building deployment transaction...")
nonce = w3.eth.get_transaction_count(account.address)
gas_price = w3.eth.gas_price

# Estimate gas
transaction = Contract.constructor().build_transaction({
    'from': account.address,
    'nonce': nonce,
    'gasPrice': int(gas_price * 1.5),  # 50% higher for priority
    'chainId': 137
})

# Estimate gas
try:
    gas_estimate = w3.eth.estimate_gas(transaction)
    transaction['gas'] = int(gas_estimate * 1.2)  # 20% buffer
except Exception as e:
    print(f"⚠️ Gas estimation failed, using default: {e}")
    transaction['gas'] = 3000000

print(f"⛽ Gas estimate: {transaction['gas']}")
print(f"💵 Gas price: {Web3.from_wei(transaction['gasPrice'], 'gwei')} gwei")
print(f"💸 Estimated cost: {Web3.from_wei(transaction['gas'] * transaction['gasPrice'], 'ether')} MATIC")

# Sign and send transaction
print("📤 Deploying contract...")
signed = account.sign_transaction(transaction)
tx_hash = w3.eth.send_raw_transaction(signed.rawTransaction)
print(f"📝 Transaction hash: {tx_hash.hex()}")

# Wait for confirmation
print("⏳ Waiting for confirmation...")
receipt = w3.eth.wait_for_transaction_receipt(tx_hash)

if receipt.status == 1:
    contract_address = receipt.contractAddress
    print(f"✅ Contract deployed at: {contract_address}")
    
    # Save deployment info
    deployment_info = {
        'address': contract_address,
        'abi': abi,
        'owner': account.address,
        'tx_hash': tx_hash.hex(),
        'block': receipt.blockNumber
    }
    
    with open('universal_arbitrage_deployment.json', 'w') as f:
        json.dump(deployment_info, f, indent=2)
    
    print("💾 Deployment info saved to universal_arbitrage_deployment.json")
    
    # Display next steps
    print("\n" + "="*60)
    print("NEXT STEPS:")
    print("1. Fund the contract with USDC")
    print("2. Execute arbitrage using executeArbitrageWithFees()")
    print(f"3. Contract address: {contract_address}")
    
else:
    print("❌ Deployment failed!")
    print(f"Receipt: {receipt}")
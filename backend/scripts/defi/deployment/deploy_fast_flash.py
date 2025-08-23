#!/usr/bin/env python3
"""Deploy FastFlashArb.sol contract to Polygon"""

from web3 import Web3
import json
import os
from eth_account import Account
from solcx import compile_source, install_solc

# Ensure solc is installed
try:
    install_solc('0.8.19')
except:
    pass

w3 = Web3(Web3.HTTPProvider('https://polygon.publicnode.com'))

# Check for private key
private_key = os.getenv('PRIVATE_KEY')
if not private_key:
    print("❌ Set PRIVATE_KEY environment variable first")
    print("export PRIVATE_KEY='your_private_key_here'")
    exit(1)

account = Account.from_key(private_key)
print(f"🔑 Deploying from: {account.address}")

# Check MATIC balance
balance = w3.eth.get_balance(account.address)
matic_balance = Web3.from_wei(balance, 'ether')
print(f"💰 MATIC Balance: {matic_balance} MATIC")

if balance < Web3.to_wei(0.01, 'ether'):
    print("❌ Need at least 0.01 MATIC for deployment")
    exit(1)

# Read contract source
print("📄 Reading FastFlashArb.sol...")
with open('FastFlashArb.sol', 'r') as f:
    contract_source = f.read()

# Compile contract
print("🔨 Compiling contract...")
try:
    compiled = compile_source(contract_source, output_values=['abi', 'bin'])
    contract_id, contract_interface = compiled.popitem()
    
    bytecode = contract_interface['bin']
    abi = contract_interface['abi']
except Exception as e:
    print(f"❌ Compilation failed: {e}")
    print("\n💡 Alternative: Use Remix IDE")
    print("1. Go to https://remix.ethereum.org")
    print("2. Create new file: FastFlashArb.sol")
    print("3. Paste the contract code")
    print("4. Compile with Solidity 0.8.19")
    print("5. Deploy to Polygon using MetaMask")
    print("6. Save the contract address")
    exit(1)

# Deploy contract
print("📤 Deploying to Polygon...")
Contract = w3.eth.contract(abi=abi, bytecode=bytecode)

# Build deployment transaction
nonce = w3.eth.get_transaction_count(account.address)
gas_price = int(w3.eth.gas_price * 1.5)  # 50% higher for priority

transaction = Contract.constructor().build_transaction({
    'from': account.address,
    'nonce': nonce,
    'gasPrice': gas_price,
    'gas': 2000000,
    'chainId': 137
})

print(f"⛽ Gas price: {Web3.from_wei(gas_price, 'gwei')} gwei")
print(f"💸 Estimated cost: ~{Web3.from_wei(2000000 * gas_price, 'ether')} MATIC")

# Sign and send
signed = account.sign_transaction(transaction)
tx_hash = w3.eth.send_raw_transaction(signed.raw_transaction)
print(f"📝 TX Hash: {tx_hash.hex()}")

# Wait for confirmation
print("⏳ Waiting for confirmation...")
receipt = w3.eth.wait_for_transaction_receipt(tx_hash)

if receipt.status == 1:
    contract_address = receipt.contractAddress
    print(f"\n✅ SUCCESS! Contract deployed at: {contract_address}")
    
    # Save deployment info
    deployment = {
        'address': contract_address,
        'abi': abi,
        'owner': account.address,
        'tx': tx_hash.hex(),
        'block': receipt.blockNumber
    }
    
    with open('fast_flash_deployment.json', 'w') as f:
        json.dump(deployment, f, indent=2)
    
    print(f"💾 Deployment saved to fast_flash_deployment.json")
    print(f"\n🎯 View on PolygonScan:")
    print(f"   https://polygonscan.com/address/{contract_address}")
    
    print(f"\n🚀 HOW TO USE:")
    print(f"1. Find arbitrage opportunity (spread > 0.5%)")
    print(f"2. Call execute() with:")
    print(f"   - tokenBorrow: Token to flash loan (USDT/USDC)")
    print(f"   - tokenMiddle: Middle token (WPOL)")
    print(f"   - borrowAmount: Amount to borrow (with decimals)")
    print(f"   - router1: Router for buy swap")
    print(f"   - router2: Router for sell swap")
    print(f"   - minProfit: Minimum profit required")
    print(f"\n💡 Example for USDT/WPOL arbitrage:")
    print(f"   tokenBorrow: 0xc2132D05D31c914a87C6611C10748AEb04B58e8F (USDT)")
    print(f"   tokenMiddle: 0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270 (WPOL)")
    print(f"   borrowAmount: 1000000000 (1000 USDT with 6 decimals)")
    print(f"   router1: 0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff (QuickSwap)")
    print(f"   router2: 0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff (QuickSwap)")
    print(f"   minProfit: 1000000 (1 USDT minimum profit)")
    
else:
    print(f"❌ Deployment failed!")
    print(f"Check TX: https://polygonscan.com/tx/{tx_hash.hex()}")
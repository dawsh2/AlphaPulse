#!/usr/bin/env python3
"""Deploy and execute Dystopia-compatible arbitrage"""

from web3 import Web3
from eth_account import Account
import json
import os
from dotenv import load_dotenv
from solcx import compile_source, set_solc_version

# Set compiler
try:
    set_solc_version('0.8.19')
except:
    from solcx import install_solc
    install_solc('0.8.19')
    set_solc_version('0.8.19')

# Load environment
load_dotenv('/Users/daws/alphapulse/backend/services/capital_arb_bot/.env')
private_key = os.getenv('PRIVATE_KEY')

w3 = Web3(Web3.HTTPProvider('https://polygon.publicnode.com'))
account = Account.from_key(private_key)
address = account.address

print(f"Account: {address}")
balance = w3.eth.get_balance(address)
print(f"MATIC Balance: {Web3.from_wei(balance, 'ether'):.4f}")

# Check USDC_OLD balance
usdc_old = '0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174'
usdc_abi = json.loads('[{"inputs":[{"name":"account","type":"address"}],"name":"balanceOf","outputs":[{"name":"","type":"uint256"}],"type":"function"},{"inputs":[{"name":"to","type":"address"},{"name":"amount","type":"uint256"}],"name":"transfer","outputs":[{"name":"","type":"bool"}],"type":"function"}]')
usdc_contract = w3.eth.contract(address=Web3.to_checksum_address(usdc_old), abi=usdc_abi)
usdc_balance = usdc_contract.functions.balanceOf(address).call()
print(f"USDC_OLD Balance: {usdc_balance/10**6:.2f}")

if usdc_balance < 0.5*10**6:
    print("❌ Not enough USDC_OLD")
    exit(1)

# Check opportunity
print("\n" + "="*60)
print("Checking arbitrage opportunity...")

dystopia = '0x380615F37993B5A96adF3D443b6E0Ac50a211998'
dystopia_abi = json.loads('[{"inputs":[{"name":"amountIn","type":"uint256"},{"name":"tokenIn","type":"address"}],"name":"getAmountOut","outputs":[{"name":"","type":"uint256"}],"type":"function"}]')
dystopia_contract = w3.eth.contract(address=dystopia, abi=dystopia_abi)

# Use remaining balance (keeping 0.02 for gas)
trade_amount = int(usdc_balance * 0.95)  # Use 95% of balance
wpol_out = dystopia_contract.functions.getAmountOut(trade_amount, usdc_old).call()

print(f"Trade size: {trade_amount/10**6:.2f} USDC_OLD")
print(f"Expected WPOL: {wpol_out/10**18:.4f}")

# Calculate final output
quickswap = '0x6D9e8dbB2779853db00418D4DcF96F3987CFC9D2'
pool_abi = json.loads('[{"inputs":[],"name":"getReserves","outputs":[{"name":"","type":"uint112"},{"name":"","type":"uint112"}],"type":"function"}]')
quickswap_contract = w3.eth.contract(address=quickswap, abi=pool_abi)
reserves = quickswap_contract.functions.getReserves().call()
usdc_new_out = (wpol_out * 997 * reserves[1]) // (reserves[0] * 1000 + wpol_out * 997)

print(f"Expected USDC_NEW: {usdc_new_out/10**6:.2f}")
print(f"Expected Profit: ${(usdc_new_out - trade_amount)/10**6:.2f}")

if usdc_new_out <= trade_amount:
    print("❌ Not profitable")
    exit(1)

# Compile contract
print("\n" + "="*60)
print("Compiling contract...")

with open('DystopiaArbitrageFixed.sol', 'r') as f:
    contract_source = f.read()

compiled = compile_source(contract_source, output_values=['abi', 'bin'])
contract_id, contract_interface = compiled.popitem()
bytecode = contract_interface['bin']
abi = contract_interface['abi']

# Deploy
print("Deploying contract...")
Contract = w3.eth.contract(abi=abi, bytecode=bytecode)
nonce = w3.eth.get_transaction_count(address)
# MEV Protection: Use higher gas to get included quickly
# Or use Flashbots/private mempool
gas_price = w3.eth.gas_price * 5  # 5x to outbid MEV bots

tx = Contract.constructor().build_transaction({
    'from': address,
    'nonce': nonce,
    'gas': 1500000,
    'gasPrice': gas_price,
})

signed_tx = account.sign_transaction(tx)
tx_hash = w3.eth.send_raw_transaction(signed_tx.raw_transaction)
print(f"Deploy tx: {tx_hash.hex()}")
receipt = w3.eth.wait_for_transaction_receipt(tx_hash)

if receipt.status != 1:
    print("❌ Deployment failed")
    exit(1)

contract_address = receipt.contractAddress
print(f"✅ Contract deployed at: {contract_address}")

# Transfer USDC to contract
print("\n" + "="*60)
print(f"Transferring {trade_amount/10**6:.2f} USDC_OLD to contract...")

nonce = w3.eth.get_transaction_count(address)
tx = usdc_contract.functions.transfer(contract_address, trade_amount).build_transaction({
    'from': address,
    'nonce': nonce,
    'gas': 100000,
    'gasPrice': gas_price,
})

signed_tx = account.sign_transaction(tx)
tx_hash = w3.eth.send_raw_transaction(signed_tx.raw_transaction)
print(f"Transfer tx: {tx_hash.hex()}")
receipt = w3.eth.wait_for_transaction_receipt(tx_hash)

if receipt.status != 1:
    print("❌ Transfer failed")
    exit(1)

# Execute arbitrage
print("\n" + "="*60)
print("Executing arbitrage...")

arb_contract = w3.eth.contract(address=contract_address, abi=abi)

# Check USDC_NEW balance before
usdc_new = '0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359'
usdc_new_contract = w3.eth.contract(address=Web3.to_checksum_address(usdc_new), abi=usdc_abi)
before_balance = usdc_new_contract.functions.balanceOf(address).call()

nonce = w3.eth.get_transaction_count(address)
tx = arb_contract.functions.executeArbitrage().build_transaction({
    'from': address,
    'nonce': nonce,
    'gas': 400000,
    'gasPrice': gas_price,
})

signed_tx = account.sign_transaction(tx)
tx_hash = w3.eth.send_raw_transaction(signed_tx.raw_transaction)
print(f"Arbitrage tx: {tx_hash.hex()}")
receipt = w3.eth.wait_for_transaction_receipt(tx_hash)

if receipt.status == 1:
    print("✅ Arbitrage executed!")
    
    after_balance = usdc_new_contract.functions.balanceOf(address).call()
    profit_received = (after_balance - before_balance) / 10**6
    
    print(f"\n💰 RESULTS:")
    print(f"   Invested: {trade_amount/10**6:.2f} USDC_OLD")
    print(f"   Received: {profit_received:.2f} USDC_NEW")
    print(f"   Gross Profit: ${profit_received - trade_amount/10**6:.2f}")
    
    gas_used = receipt.gasUsed
    gas_cost = gas_used * gas_price / 10**18 * 0.5  # Estimate MATIC price
    print(f"   Gas cost: ${gas_cost:.2f}")
    print(f"   Net Profit: ${profit_received - trade_amount/10**6 - gas_cost:.2f}")
else:
    print("❌ Arbitrage failed")
    print(f"Check: https://polygonscan.com/tx/{tx_hash.hex()}")
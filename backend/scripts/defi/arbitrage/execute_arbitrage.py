#!/usr/bin/env python3
"""Execute the arbitrage opportunity"""

from web3 import Web3
import json
import os
from eth_account import Account

# Connect to Polygon
w3 = Web3(Web3.HTTPProvider('https://polygon.publicnode.com'))

# Configuration
USDC = '0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174'
WPOL = '0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270'

# Arbitrage parameters
BUY_POOL = '0xec15624fbb314eb05baad4ca49b7904c0cb6b645'
SELL_POOL = '0xa374094527e1673a86de625aa59517c5de346d32'
AMOUNT_USDC = 43.49
BUY_FEE = 500  # 0.05%
SELL_FEE = 500  # 0.05%

print("="*60)
print("ARBITRAGE EXECUTION SCRIPT")
print("="*60)
print(f"\n📊 Opportunity Details:")
print(f"   Buy Pool:  {BUY_POOL}")
print(f"   Sell Pool: {SELL_POOL}")
print(f"   Amount:    {AMOUNT_USDC} USDC")
print(f"   Expected Profit: ~$0.0112")

# Load deployment info if exists
try:
    with open('universal_arbitrage_deployment.json', 'r') as f:
        deployment = json.load(f)
        contract_address = deployment['address']
        abi = deployment['abi']
        print(f"\n✅ Found deployed contract at: {contract_address}")
except:
    print("\n❌ No deployment found. Please run deploy_universal_arb.py first")
    print("\nTo deploy the contract:")
    print("1. Export your private key: export PRIVATE_KEY='your_private_key_here'")
    print("2. Run: python3 deploy_universal_arb.py")
    exit(1)

# Check private key
private_key = os.getenv('PRIVATE_KEY')
if not private_key:
    print("\n❌ PRIVATE_KEY not set")
    print("\nTo execute arbitrage:")
    print("1. Export your private key: export PRIVATE_KEY='your_private_key_here'")
    print("2. Make sure you have at least 43.5 USDC in your wallet")
    print("3. Run this script again")
    exit(1)

account = Account.from_key(private_key)
print(f"\n🔑 Executing from: {account.address}")

# Check balances
matic_balance = w3.eth.get_balance(account.address)
print(f"💰 MATIC Balance: {Web3.from_wei(matic_balance, 'ether')} MATIC")

# Check USDC balance
usdc_abi = json.loads('[{"inputs":[{"name":"account","type":"address"}],"name":"balanceOf","outputs":[{"name":"","type":"uint256"}],"type":"function"}]')
usdc_contract = w3.eth.contract(address=Web3.to_checksum_address(USDC), abi=usdc_abi)
usdc_balance = usdc_contract.functions.balanceOf(account.address).call()
usdc_balance_human = usdc_balance / 10**6
print(f"💵 USDC Balance: {usdc_balance_human} USDC")

if usdc_balance_human < AMOUNT_USDC:
    print(f"\n❌ Insufficient USDC. Need {AMOUNT_USDC} USDC, have {usdc_balance_human} USDC")
    exit(1)

# Create contract instance
contract = w3.eth.contract(address=Web3.to_checksum_address(contract_address), abi=abi)

print("\n" + "="*60)
print("EXECUTION PLAN:")
print("="*60)
print("\n1️⃣  Fund the contract with USDC")
print("2️⃣  Execute arbitrage with MEV protection")
print("3️⃣  Withdraw profits")

print("\n⚠️  IMPORTANT: This will execute a real transaction on Polygon mainnet!")
response = input("\nProceed with execution? (yes/no): ")

if response.lower() != 'yes':
    print("❌ Execution cancelled")
    exit(0)

# Step 1: Transfer USDC to contract
print("\n📤 Step 1: Transferring USDC to contract...")
amount_wei = int(AMOUNT_USDC * 10**6)

transfer_abi = json.loads('[{"inputs":[{"name":"to","type":"address"},{"name":"amount","type":"uint256"}],"name":"transfer","outputs":[{"name":"","type":"bool"}],"type":"function"}]')
usdc_transfer = w3.eth.contract(address=Web3.to_checksum_address(USDC), abi=transfer_abi)

nonce = w3.eth.get_transaction_count(account.address)
gas_price = int(w3.eth.gas_price * 2)  # 2x for priority

transfer_tx = usdc_transfer.functions.transfer(
    contract_address,
    amount_wei
).build_transaction({
    'from': account.address,
    'nonce': nonce,
    'gasPrice': gas_price,
    'gas': 100000,
    'chainId': 137
})

signed_transfer = account.sign_transaction(transfer_tx)
transfer_hash = w3.eth.send_raw_transaction(signed_transfer.rawTransaction)
print(f"📝 Transfer TX: {transfer_hash.hex()}")

print("⏳ Waiting for transfer confirmation...")
transfer_receipt = w3.eth.wait_for_transaction_receipt(transfer_hash)

if transfer_receipt.status != 1:
    print("❌ Transfer failed!")
    exit(1)

print("✅ USDC transferred to contract")

# Step 2: Execute arbitrage
print("\n📤 Step 2: Executing arbitrage with MEV protection...")

# Use executeArbitrageWithFees function
arb_tx = contract.functions.executeArbitrageWithFees(
    Web3.to_checksum_address(USDC),     # tokenA
    Web3.to_checksum_address(WPOL),     # tokenB
    amount_wei,                          # amountIn
    Web3.to_checksum_address(BUY_POOL), # buyPool
    2,                                   # buyRouterType (V3_UNISWAP)
    BUY_FEE,                            # buyV3Fee
    Web3.to_checksum_address(SELL_POOL),# sellPool
    2,                                   # sellRouterType (V3_UNISWAP)
    SELL_FEE                            # sellV3Fee
).build_transaction({
    'from': account.address,
    'nonce': nonce + 1,
    'gasPrice': int(gas_price * 1.5),  # Extra high for MEV protection
    'gas': 500000,
    'chainId': 137
})

signed_arb = account.sign_transaction(arb_tx)
arb_hash = w3.eth.send_raw_transaction(signed_arb.rawTransaction)
print(f"📝 Arbitrage TX: {arb_hash.hex()}")

print("⏳ Waiting for arbitrage confirmation...")
arb_receipt = w3.eth.wait_for_transaction_receipt(arb_hash)

if arb_receipt.status == 1:
    print("✅ Arbitrage executed successfully!")
    
    # Check final balance
    final_usdc = usdc_contract.functions.balanceOf(account.address).call()
    final_usdc_human = final_usdc / 10**6
    profit = final_usdc_human - (usdc_balance_human - AMOUNT_USDC)
    
    print(f"\n💰 RESULTS:")
    print(f"   Initial USDC: {usdc_balance_human}")
    print(f"   Final USDC:   {final_usdc_human}")
    print(f"   Net Profit:   ${profit:.4f}")
    
    # Calculate gas costs
    gas_used = transfer_receipt.gasUsed + arb_receipt.gasUsed
    gas_cost_matic = Web3.from_wei(gas_used * gas_price, 'ether')
    print(f"   Gas Used:     {gas_used}")
    print(f"   Gas Cost:     {gas_cost_matic} MATIC")
    
else:
    print("❌ Arbitrage failed!")
    print(f"Receipt: {arb_receipt}")
    
    # Try to withdraw stuck USDC
    print("\n🔄 Attempting to withdraw stuck USDC...")
    withdraw_tx = contract.functions.withdraw(
        Web3.to_checksum_address(USDC)
    ).build_transaction({
        'from': account.address,
        'nonce': w3.eth.get_transaction_count(account.address),
        'gasPrice': gas_price,
        'gas': 100000,
        'chainId': 137
    })
    
    signed_withdraw = account.sign_transaction(withdraw_tx)
    withdraw_hash = w3.eth.send_raw_transaction(signed_withdraw.rawTransaction)
    print(f"📝 Withdraw TX: {withdraw_hash.hex()}")
    
    withdraw_receipt = w3.eth.wait_for_transaction_receipt(withdraw_hash)
    if withdraw_receipt.status == 1:
        print("✅ USDC withdrawn from contract")
    else:
        print("❌ Withdrawal also failed")

print("\n" + "="*60)
print("Execution complete!")
print("Check on PolygonScan: https://polygonscan.com/address/" + account.address)

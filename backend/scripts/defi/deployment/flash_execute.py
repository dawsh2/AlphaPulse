#!/usr/bin/env python3
"""Execute arbitrage using flash loans - NO CAPITAL NEEDED!"""

import sys
from web3 import Web3
import json
import os
from eth_account import Account

if len(sys.argv) < 3:
    print("Usage: ./flash_execute.py <buy_pool> <sell_pool> [amount]")
    exit(1)

w3 = Web3(Web3.HTTPProvider('https://polygon.publicnode.com'))

buy_pool = sys.argv[1]
sell_pool = sys.argv[2]
amount = float(sys.argv[3]) if len(sys.argv) > 3 else 100.0

# Load flash arb contract
try:
    with open('flash_arb_deployment.json', 'r') as f:
        deployment = json.load(f)
        contract_address = deployment['address']
        abi = deployment['abi']
except:
    print("❌ Deploy FlashArbitrage.sol first!")
    print("Run: python3 deploy_flash_arb.py")
    exit(1)

private_key = os.getenv('PRIVATE_KEY')
if not private_key:
    print("❌ Set PRIVATE_KEY")
    exit(1)

account = Account.from_key(private_key)
contract = w3.eth.contract(address=Web3.to_checksum_address(contract_address), abi=abi)

# Detect pool types
def detect_pool(pool):
    try:
        v3_abi = json.loads('[{"inputs":[],"name":"fee","outputs":[{"name":"","type":"uint24"}],"type":"function"}]')
        c = w3.eth.contract(address=Web3.to_checksum_address(pool), abi=v3_abi)
        fee = c.functions.fee().call()
        return 1, fee  # UniV3
    except:
        return 0, 3000  # V2 (QuickSwap)

buy_router, buy_fee = detect_pool(buy_pool)
sell_router, sell_fee = detect_pool(sell_pool)

print(f"🎯 Flash Loan Arbitrage")
print(f"   Buy:  {buy_pool[:10]}... (Router: {buy_router}, Fee: {buy_fee})")
print(f"   Sell: {sell_pool[:10]}... (Router: {sell_router}, Fee: {sell_fee})")
print(f"   Amount: ${amount} (borrowed from Balancer)")
print(f"   Your capital needed: $0.00")

# Build params
params = (
    Web3.to_checksum_address(buy_pool),
    Web3.to_checksum_address(sell_pool),
    buy_router,
    sell_router,
    buy_fee,
    sell_fee,
    int(amount * 10**6),  # USDC has 6 decimals
    int(0.1 * 10**6)  # Min profit $0.10
)

# Execute
print("\n⚡ Executing with flash loan...")
nonce = w3.eth.get_transaction_count(account.address)
gas_price = int(w3.eth.gas_price * 2)

tx = contract.functions.executeArbitrage(params).build_transaction({
    'from': account.address,
    'nonce': nonce,
    'gasPrice': gas_price,
    'gas': 800000,
    'chainId': 137
})

signed = account.sign_transaction(tx)
tx_hash = w3.eth.send_raw_transaction(signed.rawTransaction)
print(f"📤 TX: {tx_hash.hex()}")

print("⏳ Waiting...")
receipt = w3.eth.wait_for_transaction_receipt(tx_hash, timeout=60)

if receipt.status == 1:
    print("✅ SUCCESS! Check your wallet for profit!")
    print(f"Gas used: {receipt.gasUsed}")
    print(f"View on PolygonScan: https://polygonscan.com/tx/{tx_hash.hex()}")
else:
    print("❌ Transaction failed - likely not profitable after fees")
    print("The flash loan automatically reverts if not profitable")
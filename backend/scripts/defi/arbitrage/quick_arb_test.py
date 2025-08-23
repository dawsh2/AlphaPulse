#!/usr/bin/env python3
"""
Quick test of arbitrage using Anvil fork
No real money at risk - tests on a local fork of Polygon
"""

import subprocess
import time
import sys
from web3 import Web3

print("="*70)
print("ARBITRAGE TESTING ON FORKED POLYGON")
print("="*70)

print("\n📝 This will:")
print("1. Fork Polygon mainnet locally")
print("2. Give you test USDC")
print("3. Execute the arbitrage")
print("4. Show you the profit")
print("\n✅ NO REAL MONEY AT RISK - This is a simulation\n")

# Check if anvil is installed
import os
anvil_path = os.path.expanduser("~/.foundry/bin/anvil")
if not os.path.exists(anvil_path):
    print("❌ Anvil not installed. Install with:")
    print("   curl -L https://foundry.paradigm.xyz | bash")
    print("   foundryup")
    sys.exit(1)

print("🚀 Starting Anvil fork of Polygon...")
print("   (Press Ctrl+C to stop)\n")

# Start anvil in background
anvil_process = subprocess.Popen([
    anvil_path,
    "--fork-url", "https://polygon.publicnode.com",
    "--fork-block-number", "latest",
    "--port", "8545"
], stdout=subprocess.PIPE, stderr=subprocess.PIPE)

time.sleep(10)  # Wait for anvil to start and fork

# Connect to local fork
w3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))

if not w3.is_connected():
    print("❌ Failed to connect to Anvil")
    anvil_process.terminate()
    sys.exit(1)

print("✅ Connected to local fork\n")

# Use your actual account for testing on fork
# This is safe because it's a local fork - no real transactions
from eth_account import Account
your_private_key = "0x6f98892dfba083c63b2a695a56b83667753348fb441f730d37bd5498ad9662e5"
your_account = Account.from_key(your_private_key)
test_account = your_account.address
test_private_key = your_private_key

print(f"🔑 Test account: {test_account}")
print(f"   Balance: {w3.eth.get_balance(test_account) / 10**18:.2f} MATIC\n")

# USDC addresses
USDC_OLD = "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174"
USDC_NEW = "0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359"
WPOL = "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270"

# Give ourselves some USDC (old) - impersonate a whale
whale = "0xF977814e90dA44bFA03b6295A0616a897441aceC"  # Binance hot wallet
print(f"🐳 Impersonating whale: {whale}")

# Unlock the whale account
w3.provider.make_request("anvil_impersonateAccount", [whale])

# Check whale balance
import json
ERC20_ABI = json.loads('[{"inputs":[{"name":"","type":"address"}],"name":"balanceOf","outputs":[{"name":"","type":"uint256"}],"type":"function"},{"inputs":[{"name":"to","type":"address"},{"name":"amount","type":"uint256"}],"name":"transfer","outputs":[{"name":"","type":"bool"}],"type":"function"}]')

usdc_contract = w3.eth.contract(address=Web3.to_checksum_address(USDC_OLD), abi=ERC20_ABI)
whale_balance = usdc_contract.functions.balanceOf(whale).call()
print(f"   Whale USDC balance: ${whale_balance/10**6:,.0f}\n")

# Transfer 100 USDC to our test account
transfer_amount = 100 * 10**6  # 100 USDC

print(f"💸 Transferring 100 USDC to test account...")
tx_hash = usdc_contract.functions.transfer(
    test_account,
    transfer_amount
).transact({"from": whale})

w3.eth.wait_for_transaction_receipt(tx_hash)
test_balance = usdc_contract.functions.balanceOf(test_account).call()
print(f"   Test account USDC: ${test_balance/10**6:.2f}\n")

# Now execute the arbitrage
print("🎯 Executing arbitrage...")

# Router addresses
SUSHISWAP_ROUTER = "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506"

ROUTER_ABI = json.loads('''[
    {
        "inputs": [
            {"name": "amountIn", "type": "uint256"},
            {"name": "amountOutMin", "type": "uint256"},
            {"name": "path", "type": "address[]"},
            {"name": "to", "type": "address"},
            {"name": "deadline", "type": "uint256"}
        ],
        "name": "swapExactTokensForTokens",
        "outputs": [{"name": "amounts", "type": "uint256[]"}],
        "type": "function"
    },
    {
        "inputs": [
            {"name": "spender", "type": "address"},
            {"name": "amount", "type": "uint256"}
        ],
        "name": "approve",
        "outputs": [{"name": "", "type": "bool"}],
        "type": "function"
    }
]''')

router = w3.eth.contract(address=Web3.to_checksum_address(SUSHISWAP_ROUTER), abi=ROUTER_ABI)

# Approve router to spend our USDC
print("   1. Approving router...")
approve_tx = usdc_contract.functions.approve(
    SUSHISWAP_ROUTER,
    transfer_amount
).transact({"from": test_account})
w3.eth.wait_for_transaction_receipt(approve_tx)

# Execute swap 1: USDC -> WPOL (on cheap pool)
print("   2. Buying WPOL with USDC (cheap pool)...")
path1 = [USDC_OLD, WPOL]
deadline = int(time.time()) + 300

# Note: We're using the router which will find the best pool
swap1_tx = router.functions.swapExactTokensForTokens(
    transfer_amount,  # 100 USDC
    0,  # Accept any amount
    path1,
    test_account,
    deadline
).transact({"from": test_account, "gas": 500000})

receipt1 = w3.eth.wait_for_transaction_receipt(swap1_tx)

# Check WPOL balance
wpol_contract = w3.eth.contract(address=Web3.to_checksum_address(WPOL), abi=ERC20_ABI)
wpol_balance = wpol_contract.functions.balanceOf(test_account).call()
print(f"   Received: {wpol_balance/10**18:.4f} WPOL")

# Approve router to spend WPOL
print("   3. Approving WPOL...")
approve2_tx = wpol_contract.functions.approve(
    SUSHISWAP_ROUTER,
    wpol_balance
).transact({"from": test_account})
w3.eth.wait_for_transaction_receipt(approve2_tx)

# Execute swap 2: WPOL -> USDC.e (on expensive pool)
print("   4. Selling WPOL for USDC.e (expensive pool)...")
path2 = [WPOL, USDC_NEW]

swap2_tx = router.functions.swapExactTokensForTokens(
    wpol_balance,
    0,  # Accept any amount
    path2,
    test_account,
    deadline
).transact({"from": test_account, "gas": 500000})

receipt2 = w3.eth.wait_for_transaction_receipt(swap2_tx)

# Check final balance
usdc_new_contract = w3.eth.contract(address=Web3.to_checksum_address(USDC_NEW), abi=ERC20_ABI)
final_balance = usdc_new_contract.functions.balanceOf(test_account).call()

print(f"   Received: {final_balance/10**6:.4f} USDC.e\n")

# Calculate profit
profit = (final_balance/10**6) - 100
profit_pct = (profit / 100) * 100

print("="*70)
print("📊 RESULTS:")
print(f"   Started with: 100 USDC (old)")
print(f"   Ended with:   {final_balance/10**6:.4f} USDC.e (new)")
print(f"   Profit:       {profit:.4f} USDC ({profit_pct:.1f}%)")

if profit > 0:
    print("\n✅ ARBITRAGE SUCCESSFUL IN SIMULATION!")
    print("   To execute for real, you need:")
    print("   1. USDC (old contract) in your wallet")
    print("   2. Deploy the smart contract")
    print("   3. Execute through MEV-protected channel")
else:
    print("\n❌ Arbitrage not profitable")

print("\n🛑 Stopping local fork...")
anvil_process.terminate()
print("Done!")
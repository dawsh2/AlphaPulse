#!/usr/bin/env python3
"""
Execute the cross-USDC arbitrage RIGHT NOW
Uses your actual wallet on Polygon mainnet
"""

import sys
import time
from web3 import Web3
from eth_account import Account
import json

print("="*70)
print("CROSS-USDC ARBITRAGE EXECUTOR")
print("="*70)

# Your configuration
PRIVATE_KEY = "0x6f98892dfba083c63b2a695a56b83667753348fb441f730d37bd5498ad9662e5"
RPC_URL = "https://polygon.publicnode.com"

# Connect
w3 = Web3(Web3.HTTPProvider(RPC_URL))
account = Account.from_key(PRIVATE_KEY)

print(f"\n📍 Your wallet: {account.address}")

# Check balances
matic_balance = w3.eth.get_balance(account.address)
print(f"   MATIC: {matic_balance/10**18:.4f}")

# Contracts
USDC_OLD = Web3.to_checksum_address("0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174")
USDC_NEW = Web3.to_checksum_address("0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359")
WPOL = Web3.to_checksum_address("0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270")

ERC20_ABI = json.loads('[{"inputs":[{"name":"","type":"address"}],"name":"balanceOf","outputs":[{"name":"","type":"uint256"}],"type":"function"}]')

usdc_old_contract = w3.eth.contract(address=USDC_OLD, abi=ERC20_ABI)
usdc_new_contract = w3.eth.contract(address=USDC_NEW, abi=ERC20_ABI)

usdc_old_balance = usdc_old_contract.functions.balanceOf(account.address).call()
usdc_new_balance = usdc_new_contract.functions.balanceOf(account.address).call()

print(f"   USDC (old): {usdc_old_balance/10**6:.2f}")
print(f"   USDC.e (new): {usdc_new_balance/10**6:.2f}")

# Check current opportunity
print("\n🔍 Checking arbitrage opportunity...")

buy_pool = Web3.to_checksum_address("0x380615f37993b5a96adf3d443b6e0ac50a211998")
sell_pool = Web3.to_checksum_address("0x6d9e8dbb2779853db00418d4dcf96f3987cfc9d2")

V2_ABI = json.loads('[{"inputs":[],"name":"getReserves","outputs":[{"name":"","type":"uint112"},{"name":"","type":"uint112"}],"type":"function"}]')

buy_contract = w3.eth.contract(address=buy_pool, abi=V2_ABI)
sell_contract = w3.eth.contract(address=sell_pool, abi=V2_ABI)

buy_reserves = buy_contract.functions.getReserves().call()
sell_reserves = sell_contract.functions.getReserves().call()

# Calculate arbitrage
amount_usdc = min(10, usdc_old_balance/10**6)  # Use max 10 USDC or what we have
input_wei = int(amount_usdc * 10**6)
fee = 0.997

wpol_out = (input_wei * fee * buy_reserves[0]) // (buy_reserves[1] + input_wei * fee)
usdc_out = (wpol_out * fee * sell_reserves[1]) // (sell_reserves[0] + wpol_out * fee)

profit = (usdc_out - input_wei) / 10**6

print(f"\n💰 With {amount_usdc:.2f} USDC:")
print(f"   Expected USDC.e back: {usdc_out/10**6:.4f}")
print(f"   Profit: ${profit:.4f} ({profit/amount_usdc*100:.1f}%)")

if profit < 0.01:
    print("\n❌ Profit too small (< $0.01)")
    sys.exit(1)

if usdc_old_balance < input_wei:
    print(f"\n❌ You need {amount_usdc:.2f} USDC (old) but only have {usdc_old_balance/10**6:.2f}")
    print("\nTo get USDC (old):")
    print("1. Buy on an exchange and withdraw to Polygon")
    print("2. Or swap USDC.e to USDC on QuickSwap")
    sys.exit(1)

print("\n✅ READY TO EXECUTE!")
print(f"   Trading: {amount_usdc:.2f} USDC")
print(f"   Expected profit: ${profit:.4f}")

# Router addresses
SUSHISWAP_ROUTER = Web3.to_checksum_address("0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506")

print("\n⚠️  WARNING: This will execute REAL trades on mainnet!")
print("   - MEV bots may front-run this transaction")
print("   - Use at your own risk")
print("   - Consider using Flashbots or private mempool")

response = input("\n🚀 Execute NOW? (type 'yes' to confirm): ")
if response != 'yes':
    print("Cancelled")
    sys.exit(0)

print("\n🔥 EXECUTING ARBITRAGE...")

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
    }
]''')

APPROVE_ABI = json.loads('[{"inputs":[{"name":"spender","type":"address"},{"name":"amount","type":"uint256"}],"name":"approve","outputs":[{"name":"","type":"bool"}],"type":"function"}]')

router = w3.eth.contract(address=SUSHISWAP_ROUTER, abi=ROUTER_ABI)

# Step 1: Approve USDC spending
print("1. Approving USDC...")
usdc_old_full = w3.eth.contract(address=USDC_OLD, abi=APPROVE_ABI)

approve_tx = usdc_old_full.functions.approve(
    SUSHISWAP_ROUTER,
    input_wei
).build_transaction({
    'from': account.address,
    'nonce': w3.eth.get_transaction_count(account.address),
    'gas': 100000,
    'gasPrice': w3.eth.gas_price,
})

signed_approve = account.sign_transaction(approve_tx)
approve_hash = w3.eth.send_raw_transaction(signed_approve.rawTransaction)
print(f"   Approval tx: {approve_hash.hex()}")

# Wait for approval
receipt = w3.eth.wait_for_transaction_receipt(approve_hash)
if receipt.status != 1:
    print("❌ Approval failed!")
    sys.exit(1)

print("   ✅ Approved")

# Step 2: Execute swap USDC -> WPOL
print("2. Buying WPOL with USDC...")
path1 = [USDC_OLD, WPOL]
deadline = int(time.time()) + 300

swap1_tx = router.functions.swapExactTokensForTokens(
    input_wei,
    0,  # Accept any amount
    path1,
    account.address,
    deadline
).build_transaction({
    'from': account.address,
    'nonce': w3.eth.get_transaction_count(account.address),
    'gas': 300000,
    'gasPrice': w3.eth.gas_price,
})

signed_swap1 = account.sign_transaction(swap1_tx)
swap1_hash = w3.eth.send_raw_transaction(signed_swap1.rawTransaction)
print(f"   Swap 1 tx: {swap1_hash.hex()}")

receipt1 = w3.eth.wait_for_transaction_receipt(swap1_hash)
if receipt1.status != 1:
    print("❌ Swap 1 failed!")
    sys.exit(1)

# Check WPOL balance
wpol_contract = w3.eth.contract(address=WPOL, abi=ERC20_ABI)
wpol_balance = wpol_contract.functions.balanceOf(account.address).call()
print(f"   ✅ Received {wpol_balance/10**18:.4f} WPOL")

# Step 3: Approve WPOL spending
print("3. Approving WPOL...")
wpol_full = w3.eth.contract(address=WPOL, abi=APPROVE_ABI)

approve2_tx = wpol_full.functions.approve(
    SUSHISWAP_ROUTER,
    wpol_balance
).build_transaction({
    'from': account.address,
    'nonce': w3.eth.get_transaction_count(account.address),
    'gas': 100000,
    'gasPrice': w3.eth.gas_price,
})

signed_approve2 = account.sign_transaction(approve2_tx)
approve2_hash = w3.eth.send_raw_transaction(signed_approve2.rawTransaction)
receipt2 = w3.eth.wait_for_transaction_receipt(approve2_hash)

# Step 4: Execute swap WPOL -> USDC.e
print("4. Selling WPOL for USDC.e...")
path2 = [WPOL, USDC_NEW]

swap2_tx = router.functions.swapExactTokensForTokens(
    wpol_balance,
    0,  # Accept any amount
    path2,
    account.address,
    deadline
).build_transaction({
    'from': account.address,
    'nonce': w3.eth.get_transaction_count(account.address),
    'gas': 300000,
    'gasPrice': w3.eth.gas_price,
})

signed_swap2 = account.sign_transaction(swap2_tx)
swap2_hash = w3.eth.send_raw_transaction(signed_swap2.rawTransaction)
print(f"   Swap 2 tx: {swap2_hash.hex()}")

receipt3 = w3.eth.wait_for_transaction_receipt(swap2_hash)
if receipt3.status != 1:
    print("❌ Swap 2 failed!")
    sys.exit(1)

# Check final balance
final_balance = usdc_new_contract.functions.balanceOf(account.address).call()
profit_actual = (final_balance - usdc_new_balance) / 10**6 - amount_usdc

print("\n" + "="*70)
print("✅ ARBITRAGE COMPLETE!")
print(f"   Started with: {amount_usdc:.2f} USDC")
print(f"   Gained: {(final_balance - usdc_new_balance)/10**6:.4f} USDC.e")
print(f"   Net profit: ${profit_actual:.4f}")
print("="*70)
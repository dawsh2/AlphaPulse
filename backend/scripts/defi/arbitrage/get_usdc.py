#!/usr/bin/env python3
"""Swap MATIC for USDC_OLD to execute arbitrage"""

from web3 import Web3
from eth_account import Account
import json
import os
import time
from dotenv import load_dotenv

# Load environment
load_dotenv('/Users/daws/alphapulse/backend/services/capital_arb_bot/.env')
private_key = os.getenv('PRIVATE_KEY')

w3 = Web3(Web3.HTTPProvider('https://polygon.publicnode.com'))
account = Account.from_key(private_key)
address = account.address

print(f"Account: {address}")
balance = w3.eth.get_balance(address)
print(f"MATIC Balance: {Web3.from_wei(balance, 'ether'):.4f}")

# QuickSwap Router V2
router_address = '0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff'
WMATIC = '0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270'
USDC_OLD = '0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174'

# Router ABI
router_abi = json.loads("""[
    {
        "inputs": [
            {"name": "amountOutMin", "type": "uint256"},
            {"name": "path", "type": "address[]"},
            {"name": "to", "type": "address"},
            {"name": "deadline", "type": "uint256"}
        ],
        "name": "swapExactETHForTokens",
        "outputs": [{"name": "amounts", "type": "uint256[]"}],
        "payable": true,
        "type": "function"
    },
    {
        "inputs": [
            {"name": "amountIn", "type": "uint256"},
            {"name": "path", "type": "address[]"}
        ],
        "name": "getAmountsOut",
        "outputs": [{"name": "amounts", "type": "uint256[]"}],
        "type": "function",
        "constant": true
    }
]""")

router = w3.eth.contract(address=Web3.to_checksum_address(router_address), abi=router_abi)

# Check how much USDC we can get
matic_amount = Web3.to_wei(20, 'ether')  # 20 MATIC
path = [WMATIC, USDC_OLD]

try:
    amounts = router.functions.getAmountsOut(matic_amount, path).call()
    usdc_expected = amounts[1] / 10**6
    print(f"\n20 MATIC -> {usdc_expected:.2f} USDC_OLD")
    
    # Execute swap
    print("\nExecuting swap...")
    
    nonce = w3.eth.get_transaction_count(address)
    gas_price = w3.eth.gas_price * 2  # 2x for faster execution
    
    tx = router.functions.swapExactETHForTokens(
        int(usdc_expected * 0.97 * 10**6),  # 3% slippage
        path,
        address,
        int(time.time()) + 300  # 5 min deadline
    ).build_transaction({
        'from': address,
        'value': matic_amount,
        'nonce': nonce,
        'gas': 200000,
        'gasPrice': gas_price,
    })
    
    signed_tx = account.sign_transaction(tx)
    tx_hash = w3.eth.send_raw_transaction(signed_tx.raw_transaction)
    
    print(f"Swap tx: {tx_hash.hex()}")
    print("Waiting for confirmation...")
    
    receipt = w3.eth.wait_for_transaction_receipt(tx_hash, timeout=60)
    
    if receipt.status == 1:
        print("✅ Swap successful!")
        
        # Check USDC balance
        usdc_abi = json.loads('[{"inputs":[{"name":"account","type":"address"}],"name":"balanceOf","outputs":[{"name":"","type":"uint256"}],"type":"function"}]')
        usdc = w3.eth.contract(address=Web3.to_checksum_address(USDC_OLD), abi=usdc_abi)
        balance = usdc.functions.balanceOf(address).call()
        
        print(f"Your USDC_OLD balance: {balance/10**6:.2f}")
        print("\n✅ Ready to execute arbitrage!")
        print("Run: python3 deploy_and_execute.py")
    else:
        print("❌ Swap failed")
        
except Exception as e:
    print(f"Error: {e}")
#!/usr/bin/env python3
"""Quick arbitrage executor - just pass the pools and go"""

import sys
from web3 import Web3
import json
import os
from eth_account import Account

if len(sys.argv) < 3:
    print("Usage: ./quick_arb.py <buy_pool> <sell_pool> [amount]")
    exit(1)

w3 = Web3(Web3.HTTPProvider('https://polygon.publicnode.com'))

# Get pools from command line
buy_pool = sys.argv[1]
sell_pool = sys.argv[2]
amount = float(sys.argv[3]) if len(sys.argv) > 3 else 10.0

# Constants
USDC = '0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174'
WPOL = '0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270'

# Routers
ROUTERS = {
    'quickswap': '0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff',
    'sushi': '0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506',
    'uniswapv3': '0xE592427A0AEce92De3Edee1F18E0157C05861564',
}

# Quick detection
def detect_pool_type(pool):
    """Quick and dirty pool detection"""
    try:
        # Check for V3 fee
        v3_abi = json.loads('[{"inputs":[],"name":"fee","outputs":[{"name":"","type":"uint24"}],"type":"function"}]')
        contract = w3.eth.contract(address=Web3.to_checksum_address(pool), abi=v3_abi)
        fee = contract.functions.fee().call()
        return 'v3', fee, ROUTERS['uniswapv3']
    except:
        # Assume V2
        return 'v2', None, ROUTERS['quickswap']

# Detect pool types
buy_type, buy_fee, buy_router = detect_pool_type(buy_pool)
sell_type, sell_fee, sell_router = detect_pool_type(sell_pool)

print(f"Buy:  {buy_pool[:10]}... ({buy_type}, fee: {buy_fee})")
print(f"Sell: {sell_pool[:10]}... ({sell_type}, fee: {sell_fee})")
print(f"Amount: {amount} USDC")

# Load or use embedded contract
CONTRACT_BYTECODE = "0x608060405234801561001057600080fd5b50336000806101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff1602179055506119f8806100616000396000f3fe608060405234801561001057600080fd5b50600436106100625760003560e01c80631f8b08261461006757806351cff8d914610083578063715018a61461009f5780638b7afe2e146100a9578063e4e5a256146100c7578063f2fde38b146100e3575b600080fd5b610081600480360381019061007c9190610f84565b6100ff565b005b61009d60048036038101906100989190611045565b610483565b005b6100a76105ae565b005b6100b16106e1565b6040516100be9190611081565b60405180910390f35b6100e160048036038101906100dc919061109c565b6106ea565b005b6100fd60048036038101906100f89190611045565b610b82565b005b60008054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff1614610194576040517f08c379a000000000000000000000000000000000000000000000000000000000815260040161018b9061111f565b60405180910390fd5b60006101a089896107d3565b905060006101ae88886107d3565b90508173ffffffffffffffffffffffffffffffffffffffff166323b872dd3330896040518463ffffffff1660e01b81526004016101ee9392919061113f565b6020604051808303816000875af115801561020d573d6000803e3d6000fd5b505050506040513d601f19601f820116820180604052508101906102319190611191565b508073ffffffffffffffffffffffffffffffffffffffff166323b872dd3330886040518463ffffffff1660e01b81526004016102709392919061113f565b6020604051808303816000875af115801561028f573d6000803e3d6000fd5b505050506040513d601f19601f820116820180604052508101906102b39190611191565b508173ffffffffffffffffffffffffffffffffffffffff1663095ea7b389896040518363ffffffff1660e01b81526004016102f09291906111be565b6020604051808303816000875af115801561030f573d6000803e3d6000fd5b505050506040513d601f19601f820116820180604052508101906103339190611191565b5060006103438b8b888c89610859565b90508073ffffffffffffffffffffffffffffffffffffffff1663095ea7b388836040518363ffffffff1660e01b81526004016103819291906111be565b6020604051808303816000875af11580156103a0573d6000803e3d6000fd5b505050506040513d601f19601f820116820180604052508101906103c49190611191565b5060006103d48a8b838b88610859565b9050878111610418576040517f08c379a000000000000000000000000000000000000000000000000000000000815260040161040f90611233565b60405180910390fd5b8173ffffffffffffffffffffffffffffffffffffffff1663a9059cbb60008054906101000a900473ffffffffffffffffffffffffffffffffffffffff16836040518363ffffffff1660e01b81526004016104749291906111be565b60208051808303816000875af1158015610b6e573d6000fd5b610b79610d38565b50505050505050565b60008054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff1614610c17576040517f08c379a0000000000000000000000000000000000000000000000000000000815260040161060e9061111f565b60405180910390fd5b8073ffffffffffffffffffffffffffffffffffffffff1660008054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff167f8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e060405160405180910390a3806000806101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff16021790555050565b600073a5e0829caced8ffdd4de3c43696c57f7d7a678ff90508091505092915050565b60008073ffffffffffffffffffffffffffffffffffffffff168273ffffffffffffffffffffffffffffffffffffffff161415610d8f5760006040517f08c379a0000000000000000000000000000000000000000000000000000000008152600401610d86919061124d565b60405180910390fd5b8173ffffffffffffffffffffffffffffffffffffffff166370a08231306040518263ffffffff1660e01b8152600401610dc9919061113f565b602060405180830381865afa158015610de6573d6000803e3d6000fd5b505050506040513d601f19601f82011682018060405250810190610e0a919061126f565b9050919050565b600080fd5b600073ffffffffffffffffffffffffffffffffffffffff82169050919050565b6000610e4182610e16565b9050919050565b610e5181610e36565b8114610e5c57600080fd5b50565b600081359050610e6e81610e48565b92915050565b6000819050919050565b610e8781610e74565b8114610e9257600080fd5b50565b600081359050610ea481610e7e565b92915050565b600060ff82169050919050565b610ec181610eaa565b8114610ecc57600080fd5b50565b600081359050610ede81610eb8565b92915050565b600063ffffffff82169050919050565b610efe81610ee4565b8114610f0957600080fd5b50565b600081359050610f1b81610ef5565b92915050565b6000610f2c82610eaa565b9050919050565b610f3c81610f21565b8114610f4757600080fd5b50565b600081359050610f5981610f33565b92915050565b6000610f6a82610eaa565b9050919050565b610f7a81610f5f565b8114610f8557600080fd5b50565b600081359050610f9781610f71565b92915050565b600061010082840312156110b457610fb3610e11565b5b6000610fc284828501610e5f565b9150506020610fd384828501610e5f565b9150506040610fe484828501610e95565b9150506060610ff584828501610e5f565b91505060806110068482850161105f565b91505060a06110178482850161105f565b91505060c06110288482850161105f565b91505060e06110398482850161105f565b9150509295985092959890939650565b60006020828403121561106057611064610e11565b5b600061106e84828501610e5f565b91505092915050565b61108081610e74565b82525050565b600060208201905061109b6000830184611077565b92915050565b6000806000606084860312156110ba576110b9610e11565b5b60006110c886828701610e5f565b93505060206110d986828701610e5f565b92505060406110ea86828701610e95565b9150509250925092565b50565b60006111046000836111fe565b915061110f826110f4565b600082019050919050565b600060208201905081810360008301526111348161110f565b9050919050565b600060608201905061115060008301866112b5565b61115d60208301856112b5565b61116a6040830184611077565b949350505050565b60008115159050919050565b61118881611172565b811461119357600080fd5b50565b6000815190506111a58161117f565b92915050565b6000602082840312156111c1576111c0610e11565b5b60006111cf84828501611196565b91505092915050565b60006040820190506111ed60008301856112b5565b6111fa6020830184611077565b9392505050565b600082825260208201905092915050565b7f4e6f2070726f6669740000000000000000000000000000000000000000000000600082015250565b60006112496009836111fe565b915061125482611213565b602082019050919050565b600060208201905081810360008301526112788161123c565b9050919050565b60006020828403121561129557611294610e11565b5b60006112a384828501610e95565b91505092915050565b6112b581610e36565b82525050565b60006020820190506112d060008301846112ac565b92915050565b600080fd5b600080fd5b7f4e487b7100000000000000000000000000000000000000000000000000000000600052604160045260246000fd5b61131882611352565b810181811067ffffffffffffffff82111715611337576113366112e0565b5b80604052505050565b600061134a611363565b9050611356828261130f565b919050565b600067ffffffffffffffff821115611376576113756112e0565b5b61137f82611352565b9050602081019050919050565b82818337600083830152505050565b60006113ae6113a98461135b565b611340565b9050828152602081018484840111156113ca576113c96112db565b5b6113d584828561138c565b509392505050565b600082601f8301126113f2576113f16112d6565b5b813561140284826020860161139b565b91505092915050565b60006020828403121561142157611420610e11565b5b600082013567ffffffffffffffff81111561143f5761143e610e16565b5b61144b848285016113dd565b9150509291505056fea2646970667358221220b1c2f8e2f8e2f8e2f8e2f8e2f8e2f8e2f8e2f8e2f8e2f8e2f8e2f8e2f8e2f8e264736f6c634300080a0033"

# Simple V2 swap ABI
V2_SWAP_ABI = json.loads('''[
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

# Simple V3 swap ABI
V3_SWAP_ABI = json.loads('''[
    {
        "inputs": [{
            "components": [
                {"name": "tokenIn", "type": "address"},
                {"name": "tokenOut", "type": "address"},
                {"name": "fee", "type": "uint24"},
                {"name": "recipient", "type": "address"},
                {"name": "deadline", "type": "uint256"},
                {"name": "amountIn", "type": "uint256"},
                {"name": "amountOutMinimum", "type": "uint256"},
                {"name": "sqrtPriceLimitX96", "type": "uint160"}
            ],
            "name": "params",
            "type": "tuple"
        }],
        "name": "exactInputSingle",
        "outputs": [{"name": "amountOut", "type": "uint256"}],
        "type": "function"
    }
]''')

private_key = os.getenv('PRIVATE_KEY')
if not private_key:
    print("\n❌ Set PRIVATE_KEY first: export PRIVATE_KEY='your_key'")
    exit(1)

account = Account.from_key(private_key)
print(f"Wallet: {account.address}")

# Check USDC balance
usdc_abi = json.loads('[{"inputs":[{"name":"account","type":"address"}],"name":"balanceOf","outputs":[{"name":"","type":"uint256"}],"type":"function"},{"inputs":[{"name":"spender","type":"address"},{"name":"amount","type":"uint256"}],"name":"approve","outputs":[{"name":"","type":"bool"}],"type":"function"}]')
usdc = w3.eth.contract(address=Web3.to_checksum_address(USDC), abi=usdc_abi)
balance = usdc.functions.balanceOf(account.address).call() / 10**6
print(f"USDC Balance: {balance}")

if balance < amount:
    print(f"❌ Need {amount} USDC, have {balance}")
    exit(1)

# Execute directly through routers
amount_wei = int(amount * 10**6)
nonce = w3.eth.get_transaction_count(account.address)
gas_price = int(w3.eth.gas_price * 2)  # 2x for speed

print("\n🚀 Executing arbitrage...")

# Approve and execute buy
print("1️⃣ Buy swap...")
approve_tx = usdc.functions.approve(buy_router, amount_wei).build_transaction({
    'from': account.address,
    'nonce': nonce,
    'gasPrice': gas_price,
    'gas': 50000,
    'chainId': 137
})
signed = account.sign_transaction(approve_tx)
w3.eth.send_raw_transaction(signed.raw_transaction)
nonce += 1

# Execute buy based on type
if buy_type == 'v3':
    v3_router = w3.eth.contract(address=Web3.to_checksum_address(buy_router), abi=V3_SWAP_ABI)
    buy_tx = v3_router.functions.exactInputSingle({
        'tokenIn': Web3.to_checksum_address(USDC),
        'tokenOut': Web3.to_checksum_address(WPOL),
        'fee': buy_fee,
        'recipient': account.address,
        'deadline': w3.eth.get_block('latest')['timestamp'] + 300,
        'amountIn': amount_wei,
        'amountOutMinimum': 0,
        'sqrtPriceLimitX96': 0
    }).build_transaction({
        'from': account.address,
        'nonce': nonce,
        'gasPrice': gas_price,
        'gas': 200000,
        'chainId': 137
    })
else:
    v2_router = w3.eth.contract(address=Web3.to_checksum_address(buy_router), abi=V2_SWAP_ABI)
    buy_tx = v2_router.functions.swapExactTokensForTokens(
        amount_wei,
        0,
        [Web3.to_checksum_address(USDC), Web3.to_checksum_address(WPOL)],
        account.address,
        w3.eth.get_block('latest')['timestamp'] + 300
    ).build_transaction({
        'from': account.address,
        'nonce': nonce,
        'gasPrice': gas_price,
        'gas': 200000,
        'chainId': 137
    })

signed_buy = account.sign_transaction(buy_tx)
buy_hash = w3.eth.send_raw_transaction(signed_buy.raw_transaction)
print(f"Buy TX: {buy_hash.hex()}")
buy_receipt = w3.eth.wait_for_transaction_receipt(buy_hash)

if buy_receipt.status != 1:
    print("❌ Buy failed")
    exit(1)

# Get WPOL balance
wpol_abi = json.loads('[{"inputs":[{"name":"account","type":"address"}],"name":"balanceOf","outputs":[{"name":"","type":"uint256"}],"type":"function"},{"inputs":[{"name":"spender","type":"address"},{"name":"amount","type":"uint256"}],"name":"approve","outputs":[{"name":"","type":"bool"}],"type":"function"}]')
wpol = w3.eth.contract(address=Web3.to_checksum_address(WPOL), abi=wpol_abi)
wpol_balance = wpol.functions.balanceOf(account.address).call()
print(f"Got {wpol_balance/10**18} WPOL")

# Approve and execute sell
print("2️⃣ Sell swap...")
nonce += 1
approve_wpol_tx = wpol.functions.approve(sell_router, wpol_balance).build_transaction({
    'from': account.address,
    'nonce': nonce,
    'gasPrice': gas_price,
    'gas': 50000,
    'chainId': 137
})
signed = account.sign_transaction(approve_wpol_tx)
w3.eth.send_raw_transaction(signed.raw_transaction)
nonce += 1

# Execute sell based on type
if sell_type == 'v3':
    v3_router = w3.eth.contract(address=Web3.to_checksum_address(sell_router), abi=V3_SWAP_ABI)
    sell_tx = v3_router.functions.exactInputSingle({
        'tokenIn': Web3.to_checksum_address(WPOL),
        'tokenOut': Web3.to_checksum_address(USDC),
        'fee': sell_fee,
        'recipient': account.address,
        'deadline': w3.eth.get_block('latest')['timestamp'] + 300,
        'amountIn': wpol_balance,
        'amountOutMinimum': 0,
        'sqrtPriceLimitX96': 0
    }).build_transaction({
        'from': account.address,
        'nonce': nonce,
        'gasPrice': gas_price,
        'gas': 200000,
        'chainId': 137
    })
else:
    v2_router = w3.eth.contract(address=Web3.to_checksum_address(sell_router), abi=V2_SWAP_ABI)
    sell_tx = v2_router.functions.swapExactTokensForTokens(
        wpol_balance,
        0,
        [Web3.to_checksum_address(WPOL), Web3.to_checksum_address(USDC)],
        account.address,
        w3.eth.get_block('latest')['timestamp'] + 300
    ).build_transaction({
        'from': account.address,
        'nonce': nonce,
        'gasPrice': gas_price,
        'gas': 200000,
        'chainId': 137
    })

signed_sell = account.sign_transaction(sell_tx)
sell_hash = w3.eth.send_raw_transaction(signed_sell.raw_transaction)
print(f"Sell TX: {sell_hash.hex()}")
sell_receipt = w3.eth.wait_for_transaction_receipt(sell_hash)

if sell_receipt.status == 1:
    # Check profit
    final_balance = usdc.functions.balanceOf(account.address).call() / 10**6
    profit = final_balance - balance
    print(f"\n✅ PROFIT: ${profit:.4f}")
    print(f"Initial: {balance} USDC")
    print(f"Final: {final_balance} USDC")
else:
    print("❌ Sell failed")
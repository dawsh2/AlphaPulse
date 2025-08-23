#!/usr/bin/env python3
"""Analyze Dystopia pool to understand its swap interface"""

from web3 import Web3
import json

w3 = Web3(Web3.HTTPProvider('https://polygon.publicnode.com'))

dystopia_pool = Web3.to_checksum_address('0x380615f37993b5a96adf3d443b6e0ac50a211998')

print("Analyzing Dystopia Pool...")
print("="*60)

# Get bytecode to understand contract type
code = w3.eth.get_code(dystopia_pool)
print(f"Contract has bytecode: {len(code)} bytes")

# Try different swap function selectors
swap_selectors = {
    '0x022c0d9f': 'swap(uint256,uint256,address,bytes)',  # Uniswap V2
    '0x128acb08': 'swap(address,bool,int256,uint160,bytes)',  # Uniswap V3
    '0x8803dbee': 'swapTokensForExactTokens',  # Router style
    '0x38ed1739': 'swapExactTokensForTokens',  # Router style
    '0x6d9a640a': 'swap(uint256,uint256,address)',  # Simpler swap
    '0x098b9945': 'swap(address,uint256,uint256,bytes)',  # Alternative
    '0x8803c8a4': 'exchange(int128,int128,uint256,uint256)',  # Curve style
}

# Check first 4 bytes of transactions to this pool
print("\nChecking recent transactions to find swap method...")

# Get recent transactions
latest_block = w3.eth.block_number
txs = []

for block_num in range(latest_block - 50, latest_block):
    try:
        block = w3.eth.get_block(block_num, full_transactions=True)
        for tx in block.transactions:
            if tx.to and tx.to.lower() == dystopia_pool.lower():
                selector = tx.input[:10]
                if selector not in ['0x095ea7b3', '0xa9059cbb']:  # Skip approve/transfer
                    print(f"  Found tx with selector: {selector}")
                    if selector in swap_selectors:
                        print(f"    -> {swap_selectors[selector]}")
                    txs.append(tx)
                    if len(txs) >= 5:
                        break
    except:
        pass
    if len(txs) >= 5:
        break

# Check if it's a Solidly/Velodrome fork (common for Dystopia)
print("\n" + "="*60)
print("Testing Solidly/Velodrome interface...")

solidly_abi = json.loads("""[
    {
        "inputs": [{"name": "amount0Out", "type": "uint256"}, {"name": "amount1Out", "type": "uint256"}, {"name": "to", "type": "address"}, {"name": "data", "type": "bytes"}],
        "name": "swap",
        "outputs": [],
        "type": "function"
    },
    {
        "inputs": [],
        "name": "getReserves",
        "outputs": [{"name": "_reserve0", "type": "uint256"}, {"name": "_reserve1", "type": "uint256"}, {"name": "_blockTimestampLast", "type": "uint256"}],
        "type": "function"
    },
    {
        "inputs": [],
        "name": "stable",
        "outputs": [{"name": "", "type": "bool"}],
        "type": "function"
    },
    {
        "inputs": [{"name": "amountIn", "type": "uint256"}, {"name": "tokenIn", "type": "address"}],
        "name": "getAmountOut",
        "outputs": [{"name": "", "type": "uint256"}],
        "type": "function"
    }
]""")

contract = w3.eth.contract(address=Web3.to_checksum_address(dystopia_pool), abi=solidly_abi)

try:
    reserves = contract.functions.getReserves().call()
    print(f"✅ getReserves() works: {reserves[0]/10**18:.2f} WPOL, {reserves[1]/10**6:.2f} USDC")
    
    try:
        is_stable = contract.functions.stable().call()
        print(f"✅ stable() works: {is_stable}")
        print("   -> This confirms it's a Solidly/Velodrome fork!")
    except:
        print("❌ stable() doesn't exist - might be standard V2")
        
    # Test getAmountOut
    try:
        test_amount = 10 * 10**6  # 10 USDC
        usdc_addr = '0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174'
        wpol_out = contract.functions.getAmountOut(test_amount, usdc_addr).call()
        print(f"✅ getAmountOut() works: 10 USDC -> {wpol_out/10**18:.4f} WPOL")
    except Exception as e:
        print(f"❌ getAmountOut() failed: {e}")
        
except Exception as e:
    print(f"❌ Not a Solidly fork: {e}")

print("\n" + "="*60)
print("CONCLUSION:")
print("Dystopia uses Solidly/Velodrome interface with:")
print("1. swap(uint256 amount0Out, uint256 amount1Out, address to, bytes data)")
print("2. Same calculation but may use different fee structure")
print("3. Has stable() function to check if it's a stable pair")
print("4. May have getAmountOut() helper function")
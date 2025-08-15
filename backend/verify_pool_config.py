#!/usr/bin/env python3
"""
Verify the actual configuration of the problematic pool
Query the blockchain to get real token addresses and decimals
"""

import json
import requests
from web3 import Web3

# Alchemy API
ALCHEMY_URL = "https://polygon-mainnet.g.alchemy.com/v2/YIN6CJks2-fLUDgen4hAs"

# The problematic pool
POOL_ADDRESS = "0x882df4b0fb50a229c3b4124eb18c759911485bfb"

# Known token addresses on Polygon
KNOWN_TOKENS = {
    "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174": "USDC (bridged)",
    "0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359": "USDC (native)", 
    "0x455e53CBB86018Ac2B8092FdCd39d8444aFFC3F6": "POL",
    "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270": "WMATIC",
    "0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619": "WETH",
    "0xc2132D05D31c914a87C6611C10748AEb04B58e8F": "USDT",
}

def get_pool_tokens(pool_address):
    """Get token0 and token1 addresses from a Uniswap V2 style pool"""
    w3 = Web3(Web3.HTTPProvider(ALCHEMY_URL))
    
    # Convert to checksum address
    pool_address = w3.to_checksum_address(pool_address)
    
    # Function selectors
    TOKEN0_SELECTOR = "0x0dfe1681"  # token0()
    TOKEN1_SELECTOR = "0xd21220a7"  # token1()
    
    # Make the calls
    token0_result = w3.eth.call({
        'to': pool_address,
        'data': TOKEN0_SELECTOR
    })
    
    token1_result = w3.eth.call({
        'to': pool_address,
        'data': TOKEN1_SELECTOR
    })
    
    # Parse addresses (last 20 bytes of the 32-byte result)
    token0 = "0x" + token0_result.hex()[-40:]
    token1 = "0x" + token1_result.hex()[-40:]
    
    return token0, token1

def get_token_info(token_address):
    """Get token decimals and symbol"""
    w3 = Web3(Web3.HTTPProvider(ALCHEMY_URL))
    
    # Convert to checksum address
    token_address = w3.to_checksum_address(token_address)
    
    # Function selectors
    DECIMALS_SELECTOR = "0x313ce567"  # decimals()
    SYMBOL_SELECTOR = "0x95d89b41"     # symbol()
    
    try:
        # Get decimals
        decimals_result = w3.eth.call({
            'to': token_address,
            'data': DECIMALS_SELECTOR
        })
        decimals = int(decimals_result.hex(), 16)
        
        # Get symbol
        symbol_result = w3.eth.call({
            'to': token_address,
            'data': SYMBOL_SELECTOR
        })
        # Parse string from bytes (skip first 64 bytes which contain offset and length)
        symbol_hex = symbol_result.hex()
        if len(symbol_hex) > 128:
            symbol_bytes = bytes.fromhex(symbol_hex[128:])
            symbol = symbol_bytes.decode('utf-8').strip('\x00')
        else:
            symbol = "UNKNOWN"
            
        return decimals, symbol
    except Exception as e:
        print(f"Error getting token info: {e}")
        return None, None

def analyze_swap_with_correct_decimals(token0_decimals, token1_decimals):
    """Re-analyze the swap data with correct decimals"""
    # Sample problematic swap
    amount0_in_raw = 20021767500419825664
    amount1_in_raw = 0
    amount0_out_raw = 0
    amount1_out_raw = 1594904687
    
    print("\n📊 RE-ANALYZING SWAP WITH CORRECT DECIMALS:")
    print(f"  Raw amounts: ")
    print(f"    token0_in:  {amount0_in_raw:,}")
    print(f"    token1_in:  {amount1_in_raw:,}")
    print(f"    token0_out: {amount0_out_raw:,}")
    print(f"    token1_out: {amount1_out_raw:,}")
    
    # Apply correct decimals
    token0_in = amount0_in_raw / (10 ** token0_decimals)
    token1_in = amount1_in_raw / (10 ** token1_decimals)
    token0_out = amount0_out_raw / (10 ** token0_decimals)
    token1_out = amount1_out_raw / (10 ** token1_decimals)
    
    print(f"\n  With decimals ({token0_decimals}, {token1_decimals}):")
    print(f"    token0_in:  {token0_in:.6f}")
    print(f"    token1_in:  {token1_in:.6f}")
    print(f"    token0_out: {token0_out:.6f}")
    print(f"    token1_out: {token1_out:.6f}")
    
    # Calculate price
    if token0_in > 0 and token1_out > 0:
        price = token0_in / token1_out
        print(f"\n  Price: {price:.6f} token0 per token1")
    elif token1_in > 0 and token0_out > 0:
        price = token0_out / token1_in
        print(f"\n  Price: {price:.6f} token0 per token1")

def main():
    print("🔍 VERIFYING POOL CONFIGURATION")
    print("=" * 60)
    print(f"Pool address: {POOL_ADDRESS}")
    
    # Get pool tokens
    print("\n📍 Querying pool tokens...")
    token0, token1 = get_pool_tokens(POOL_ADDRESS)
    
    print(f"  Token0: {token0}")
    if token0 in KNOWN_TOKENS:
        print(f"    ✅ Identified as: {KNOWN_TOKENS[token0]}")
    else:
        print(f"    ❓ Unknown token")
    
    print(f"  Token1: {token1}")
    if token1 in KNOWN_TOKENS:
        print(f"    ✅ Identified as: {KNOWN_TOKENS[token1]}")
    else:
        print(f"    ❓ Unknown token")
    
    # Get token details
    print("\n📊 Getting token details...")
    
    decimals0, symbol0 = get_token_info(token0)
    if decimals0:
        print(f"  Token0 ({symbol0}): {decimals0} decimals")
    
    decimals1, symbol1 = get_token_info(token1)
    if decimals1:
        print(f"  Token1 ({symbol1}): {decimals1} decimals")
    
    # Analyze token ordering
    print("\n🔄 Token Ordering Analysis:")
    if token0.lower() < token1.lower():
        print(f"  ✅ Token0 < Token1 (correct Uniswap V2 ordering)")
    else:
        print(f"  ⚠️  Token0 > Token1 (unusual ordering!)")
    
    # Re-analyze swap with correct decimals
    if decimals0 and decimals1:
        analyze_swap_with_correct_decimals(decimals0, decimals1)
    
    print("\n" + "=" * 60)
    print("💡 FINDINGS:")
    if decimals0 != 6 and symbol0 == "USDC":
        print(f"  ⚠️  USDC has {decimals0} decimals, not 6 as assumed!")
    if decimals1 != 18 and symbol1 == "POL":
        print(f"  ⚠️  POL has {decimals1} decimals, not 18 as assumed!")
    
    print("\n🔧 RECOMMENDATION:")
    print("  The pool token configuration should be:")
    print(f"    Token0: {symbol0} ({decimals0} decimals)")
    print(f"    Token1: {symbol1} ({decimals1} decimals)")

if __name__ == "__main__":
    main()
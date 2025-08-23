#!/usr/bin/env python3
"""
Test V3 execution prices and find maximum profitable trade size
"""

import sys
from web3 import Web3
import json

# RPC setup
RPC_URL = "https://polygon-mainnet.public.blastapi.io"
w3 = Web3(Web3.HTTPProvider(RPC_URL))

# Uniswap V3 Quoter
QUOTER_ADDRESS = "0xb27308f9F90D607463bb33eA1BeBb41C27CE5AB6"
QUOTER_ABI = json.loads('[{"inputs":[{"name":"tokenIn","type":"address"},{"name":"tokenOut","type":"address"},{"name":"fee","type":"uint24"},{"name":"amountIn","type":"uint256"},{"name":"sqrtPriceLimitX96","type":"uint160"}],"name":"quoteExactInputSingle","outputs":[{"name":"amountOut","type":"uint256"}],"type":"function","stateMutability":"view"}]')

V3_ABI = json.loads('[{"inputs":[],"name":"slot0","outputs":[{"name":"sqrtPriceX96","type":"uint160"},{"name":"tick","type":"int24"}],"type":"function"},{"inputs":[],"name":"fee","outputs":[{"name":"","type":"uint24"}],"type":"function"},{"inputs":[],"name":"token0","outputs":[{"name":"","type":"address"}],"type":"function"},{"inputs":[],"name":"token1","outputs":[{"name":"","type":"address"}],"type":"function"}]')

V2_ABI = json.loads('[{"inputs":[],"name":"getReserves","outputs":[{"name":"","type":"uint112"},{"name":"","type":"uint112"},{"name":"","type":"uint32"}],"type":"function"},{"inputs":[],"name":"token0","outputs":[{"name":"","type":"address"}],"type":"function"},{"inputs":[],"name":"token1","outputs":[{"name":"","type":"address"}],"type":"function"}]')

ERC20_ABI = json.loads('[{"inputs":[],"name":"symbol","outputs":[{"name":"","type":"string"}],"type":"function"},{"inputs":[],"name":"decimals","outputs":[{"name":"","type":"uint8"}],"type":"function"}]')

def get_token_info(token_address):
    """Get token symbol and decimals"""
    token = w3.eth.contract(address=Web3.to_checksum_address(token_address), abi=ERC20_ABI)
    try:
        symbol = token.functions.symbol().call()
        decimals = token.functions.decimals().call()
        return symbol, decimals
    except:
        return "TOKEN", 18

def calc_v2_output(amount_in, reserve_in, reserve_out, fee_bps=30):
    """Calculate V2 output"""
    amount_with_fee = amount_in * (10000 - fee_bps)
    numerator = amount_with_fee * reserve_out
    denominator = reserve_in * 10000 + amount_with_fee
    return numerator // denominator

def test_v3_slippage_curve(pool_address):
    """Test V3 pool at different trade sizes to find profitable range"""
    pool = w3.eth.contract(address=Web3.to_checksum_address(pool_address), abi=V3_ABI)
    quoter = w3.eth.contract(address=Web3.to_checksum_address(QUOTER_ADDRESS), abi=QUOTER_ABI)
    
    # Get pool info
    token0 = pool.functions.token0().call()
    token1 = pool.functions.token1().call()
    fee = pool.functions.fee().call()
    slot0 = pool.functions.slot0().call()
    
    # Get token info
    sym0, dec0 = get_token_info(token0)
    sym1, dec1 = get_token_info(token1)
    
    # Calculate spot price
    sqrt_price = slot0[0] / (2**96)
    spot_price = sqrt_price ** 2 * (10**(dec0-dec1))
    
    print(f"\n📊 V3 Pool: {sym0}/{sym1}")
    print(f"Spot price: {spot_price:.6f} {sym1}/{sym0}")
    print(f"Fee tier: {fee/10000:.2f}%")
    
    # Test different trade sizes (in token0)
    if sym0 in ['WPOL', 'WMATIC']:
        test_amounts = [0.1, 0.5, 1, 2, 5, 10, 20, 50, 100, 200, 500]
    elif sym0 in ['WBTC']:
        test_amounts = [0.0001, 0.0005, 0.001, 0.002, 0.005, 0.01, 0.02, 0.05]
    elif sym0 in ['WETH']:
        test_amounts = [0.001, 0.005, 0.01, 0.02, 0.05, 0.1, 0.2, 0.5]
    else:
        # USDC/USDT
        test_amounts = [10, 50, 100, 200, 500, 1000, 2000, 5000, 10000]
    
    print(f"\n{'Amount':<12} {'Output':<12} {'Exec Price':<12} {'Slippage %':<10} {'Status'}")
    print("-" * 60)
    
    last_profitable = None
    for amount in test_amounts:
        amount_wei = int(amount * 10**dec0)
        
        try:
            # Get quote for token0 -> token1
            output_wei = quoter.functions.quoteExactInputSingle(
                Web3.to_checksum_address(token0),
                Web3.to_checksum_address(token1),
                fee,
                amount_wei,
                0
            ).call()
            
            output = output_wei / (10**dec1)
            exec_price = output / amount
            
            # Calculate slippage (including fee)
            fee_pct = fee / 1000000
            price_before_fee = exec_price / (1 - fee_pct)
            slippage = (spot_price - price_before_fee) / spot_price * 100
            
            # Check if profitable vs 0.3% V2 pool
            # Need > 0.3% + fee% spread to be profitable
            min_spread_needed = 0.3 + fee/10000
            status = "✅" if abs(slippage) < 0.5 else "⚠️" if abs(slippage) < 2 else "❌"
            
            print(f"{amount:<12.4f} {output:<12.4f} {exec_price:<12.6f} {slippage:<9.2f}% {status}")
            
            if abs(slippage) < 0.5:
                last_profitable = amount
            
        except Exception as e:
            print(f"{amount:<12.4f} Quote failed: {str(e)[:30]}")
            break
    
    if last_profitable:
        print(f"\n✅ Maximum profitable trade size: {last_profitable} {sym0}")
        # Convert to USD estimate
        if sym0 == 'WPOL':
            usd_value = last_profitable * 0.25
        elif sym0 == 'WBTC':
            usd_value = last_profitable * 100000
        elif sym0 == 'WETH':
            usd_value = last_profitable * 3800
        else:
            usd_value = last_profitable
        print(f"   Approximate USD value: ${usd_value:.2f}")
    else:
        print(f"\n❌ No profitable trade size found - liquidity too low")

def analyze_pools(pool_addresses):
    """Analyze multiple pools"""
    print("\n" + "="*70)
    print("V3 POOL EXECUTION ANALYSIS")
    print("="*70)
    
    v3_pools = []
    v2_pools = []
    
    for addr in pool_addresses:
        try:
            # Try V3
            pool = w3.eth.contract(address=Web3.to_checksum_address(addr), abi=V3_ABI)
            fee = pool.functions.fee().call()
            v3_pools.append(addr)
        except:
            # Try V2
            try:
                pool = w3.eth.contract(address=Web3.to_checksum_address(addr), abi=V2_ABI)
                reserves = pool.functions.getReserves().call()
                v2_pools.append(addr)
            except:
                print(f"❌ Failed to load pool: {addr}")
    
    print(f"\nFound {len(v3_pools)} V3 pools and {len(v2_pools)} V2 pools")
    
    # Test each V3 pool
    for pool_addr in v3_pools:
        try:
            test_v3_slippage_curve(pool_addr)
        except Exception as e:
            print(f"\n❌ Error testing {pool_addr}: {e}")
    
    print("\n" + "="*70)
    print("SUMMARY")
    print("="*70)
    print("V3 pools often have concentrated liquidity causing slippage.")
    print("Always test actual execution prices, not just spot prices!")
    print("Maximum profitable trade size is usually much smaller than liquidity suggests.")

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python3 test_v3_execution.py <pool1> [pool2] ...")
        print("\nExample:")
        print("  python3 test_v3_execution.py 0x4f28eef4dde2bfa0bdb95a7efe586c3654e6cf07")
        sys.exit(1)
    
    pool_addresses = sys.argv[1:]
    analyze_pools(pool_addresses)
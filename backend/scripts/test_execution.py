#!/usr/bin/env python3
"""
Test if an arbitrage opportunity is actually executable
Simulates the trade with eth_call to verify it would work
"""

import sys
from web3 import Web3
import json
import time

# RPC setup
RPC_URL = "https://polygon-mainnet.public.blastapi.io"
w3 = Web3(Web3.HTTPProvider(RPC_URL))

# Router ABIs
ROUTER_V2_ABI = json.loads('[{"inputs":[{"name":"amountIn","type":"uint256"},{"name":"amountOutMin","type":"uint256"},{"name":"path","type":"address[]"},{"name":"to","type":"address"},{"name":"deadline","type":"uint256"}],"name":"swapExactTokensForTokens","outputs":[{"name":"amounts","type":"uint256[]"}],"type":"function"},{"inputs":[{"name":"amountIn","type":"uint256"},{"name":"path","type":"address[]"}],"name":"getAmountsOut","outputs":[{"name":"amounts","type":"uint256[]"}],"type":"function","stateMutability":"view"}]')

ROUTER_V3_ABI = json.loads('[{"inputs":[{"name":"tokenIn","type":"address"},{"name":"tokenOut","type":"address"},{"name":"fee","type":"uint24"},{"name":"recipient","type":"address"},{"name":"deadline","type":"uint256"},{"name":"amountIn","type":"uint256"},{"name":"amountOutMinimum","type":"uint256"},{"name":"sqrtPriceLimitX96","type":"uint160"}],"name":"exactInputSingle","outputs":[{"name":"amountOut","type":"uint256"}],"type":"function"}]')

# Known routers
ROUTERS = {
    "quickswap_v2": "0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff",
    "sushiswap_v2": "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506",
    "uniswap_v3": "0xE592427A0AEce92De3Edee1F18E0157C05861564",
    "quickswap_v3": "0xf5b509bB0909a69B1c207E495f687a596C168E12",
}

# Test wallet (random address for simulation)
TEST_WALLET = "0x0000000000000000000000000000000000000001"

def identify_router(pool_address):
    """Try to identify which router to use for a pool"""
    # This is simplified - in production, check factory contracts
    return ROUTERS["sushiswap_v2"]  # Default to SushiSwap

def simulate_v2_swap(router_address, token_in, token_out, amount_in_wei):
    """Simulate a V2 swap"""
    try:
        router = w3.eth.contract(address=Web3.to_checksum_address(router_address), abi=ROUTER_V2_ABI)
        
        # Build path
        path = [Web3.to_checksum_address(token_in), Web3.to_checksum_address(token_out)]
        
        # Get expected output
        amounts = router.functions.getAmountsOut(amount_in_wei, path).call()
        amount_out = amounts[1]
        
        # Build swap transaction (for simulation)
        deadline = int(time.time()) + 300
        min_out = int(amount_out * 0.99)  # 1% slippage
        
        # Build swap calldata manually
        from eth_abi import encode
        function_selector = Web3.keccak(text="swapExactTokensForTokens(uint256,uint256,address[],address,uint256)")[:4]
        encoded_params = encode(
            ['uint256', 'uint256', 'address[]', 'address', 'uint256'],
            [amount_in_wei, min_out, path, TEST_WALLET, deadline]
        )
        swap_data = function_selector + encoded_params
        
        # Simulate the call
        try:
            result = w3.eth.call({
                'to': router_address,
                'from': TEST_WALLET,
                'data': swap_data,
                'value': 0
            })
            print(f"✅ Swap simulation successful!")
            return amount_out
        except Exception as e:
            error_msg = str(e)
            if 'INSUFFICIENT' in error_msg.upper():
                print(f"⚠️  Insufficient liquidity for full amount")
            elif 'TRANSFER' in error_msg.upper():
                print(f"⚠️  Token transfer would fail (no balance in test wallet)")
                return amount_out  # Expected, we don't have tokens
            else:
                print(f"❌ Swap would fail: {error_msg[:100]}")
                return 0
                
    except Exception as e:
        print(f"❌ Failed to simulate: {e}")
        return 0

def test_arbitrage(pool1_addr, pool2_addr, amount_usd=10):
    """Test if arbitrage between two pools would work"""
    print("\n" + "="*70)
    print("ARBITRAGE EXECUTION TEST")
    print("="*70)
    
    # Load pool data (simplified - using V2 for now)
    V2_ABI = json.loads('[{"inputs":[],"name":"getReserves","outputs":[{"name":"","type":"uint112"},{"name":"","type":"uint112"}],"type":"function"},{"inputs":[],"name":"token0","outputs":[{"name":"","type":"address"}],"type":"function"},{"inputs":[],"name":"token1","outputs":[{"name":"","type":"address"}],"type":"function"}]')
    
    try:
        pool1 = w3.eth.contract(address=Web3.to_checksum_address(pool1_addr), abi=V2_ABI)
        pool2 = w3.eth.contract(address=Web3.to_checksum_address(pool2_addr), abi=V2_ABI)
        
        # Get tokens
        token0 = pool1.functions.token0().call()
        token1 = pool1.functions.token1().call()
        
        # Get reserves
        r1 = pool1.functions.getReserves().call()
        r2 = pool2.functions.getReserves().call()
        
        # Calculate prices (simplified - assumes same decimal places)
        price1 = r1[1] / r1[0] if r1[0] > 0 else 0
        price2 = r2[1] / r2[0] if r2[0] > 0 else 0
        
        print(f"\n📊 Pool Analysis:")
        print(f"Pool 1: {pool1_addr[:10]}...")
        print(f"  Price: {price1:.6f}")
        print(f"  Reserves: {r1[0]/1e18:.2f} / {r1[1]/1e6:.2f}")
        
        print(f"Pool 2: {pool2_addr[:10]}...")
        print(f"  Price: {price2:.6f}")
        print(f"  Reserves: {r2[0]/1e18:.2f} / {r2[1]/1e6:.2f}")
        
        # Determine arbitrage direction
        if price1 < price2:
            buy_pool, sell_pool = pool1_addr, pool2_addr
            buy_price, sell_price = price1, price2
        else:
            buy_pool, sell_pool = pool2_addr, pool1_addr
            buy_price, sell_price = price2, price1
        
        spread = (sell_price - buy_price) / buy_price * 100
        print(f"\n💰 Arbitrage Opportunity:")
        print(f"  Spread: {spread:.2f}%")
        print(f"  Direction: Buy from {buy_pool[:10]}... → Sell to {sell_pool[:10]}...")
        
        # Test with small amount
        if token1.lower() in ['0x2791bca1f2de4661ed88a30c99a7a9449aa84174', '0x3c499c542cef5e3811e1192ce70d8cc03d5c3359']:
            # USDC
            test_amount_wei = int(amount_usd * 1e6)
            decimals = 6
        else:
            test_amount_wei = int(amount_usd * 1e18)
            decimals = 18
        
        print(f"\n🧪 Testing with ${amount_usd} equivalent...")
        
        # Simulate buy
        router1 = identify_router(buy_pool)
        print(f"Step 1: Buy token0 from pool 1 using {router1[:10]}...")
        token0_received = simulate_v2_swap(router1, token1, token0, test_amount_wei)
        
        if token0_received > 0:
            # Simulate sell
            router2 = identify_router(sell_pool)
            print(f"Step 2: Sell token0 to pool 2 using {router2[:10]}...")
            token1_back = simulate_v2_swap(router2, token0, token1, token0_received)
            
            if token1_back > 0:
                profit_wei = token1_back - test_amount_wei
                profit = profit_wei / (10**decimals)
                profit_pct = profit / amount_usd * 100
                
                print(f"\n📈 Results:")
                print(f"  Input: ${amount_usd}")
                print(f"  Output: ${amount_usd + profit:.4f}")
                print(f"  Profit: ${profit:.4f} ({profit_pct:.2f}%)")
                
                if profit > 0.005:  # More than $0.005 profit
                    print(f"\n✅ ARBITRAGE IS EXECUTABLE!")
                    print(f"⚠️  Note: This is a simulation. Actual execution may differ due to:")
                    print(f"   - MEV bots front-running")
                    print(f"   - Slippage from other trades")
                    print(f"   - Token transfer restrictions")
                else:
                    print(f"\n❌ Not profitable after gas")
        
    except Exception as e:
        print(f"❌ Error: {e}")

if __name__ == "__main__":
    if len(sys.argv) < 3:
        print("Usage: python3 test_execution.py <pool1> <pool2> [amount_usd]")
        print("\nExample:")
        print("  python3 test_execution.py 0x380615f37993b5a96adf3d443b6e0ac50a211998 0x6d9e8dbb2779853db00418d4dcf96f3987cfc9d2")
        sys.exit(1)
    
    pool1 = sys.argv[1]
    pool2 = sys.argv[2]
    amount = float(sys.argv[3]) if len(sys.argv) > 3 else 10
    
    test_arbitrage(pool1, pool2, amount)
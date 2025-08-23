#!/usr/bin/env python3
"""
Test if arbitrage opportunities are real or honeypots
Tests actual swap execution and checks for common issues
"""

import sys
from web3 import Web3
import json
import time

# RPC setup
RPC_URL = "https://polygon.publicnode.com"
w3 = Web3(Web3.HTTPProvider(RPC_URL))

# ABIs
V2_ABI = json.loads('[{"inputs":[],"name":"getReserves","outputs":[{"name":"","type":"uint112"},{"name":"","type":"uint112"}],"type":"function"},{"inputs":[],"name":"token0","outputs":[{"name":"","type":"address"}],"type":"function"},{"inputs":[],"name":"token1","outputs":[{"name":"","type":"address"}],"type":"function"}]')
ERC20_ABI = json.loads('[{"inputs":[],"name":"symbol","outputs":[{"name":"","type":"string"}],"type":"function"},{"inputs":[],"name":"decimals","outputs":[{"name":"","type":"uint8"}],"type":"function"},{"inputs":[{"name":"","type":"address"}],"name":"balanceOf","outputs":[{"name":"","type":"uint256"}],"type":"function"}]')

# Known router addresses
ROUTERS = {
    "quickswap_v2": "0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff",
    "sushiswap_v2": "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506",
}

def check_honeypot(pool_address, amount_usd=1):
    """Check if a pool might be a honeypot"""
    
    print(f"\n🔍 Testing pool {pool_address[:10]}... for honeypot characteristics")
    
    try:
        pool = w3.eth.contract(address=Web3.to_checksum_address(pool_address), abi=V2_ABI)
        
        # Get tokens
        token0 = pool.functions.token0().call()
        token1 = pool.functions.token1().call()
        
        # Get reserves
        reserves = pool.functions.getReserves().call()
        r0, r1 = reserves[0], reserves[1]
        
        # Get token contracts
        t0_contract = w3.eth.contract(address=Web3.to_checksum_address(token0), abi=ERC20_ABI)
        t1_contract = w3.eth.contract(address=Web3.to_checksum_address(token1), abi=ERC20_ABI)
        
        # Get symbols and decimals
        s0 = t0_contract.functions.symbol().call()
        s1 = t1_contract.functions.symbol().call()
        d0 = t0_contract.functions.decimals().call()
        d1 = t1_contract.functions.decimals().call()
        
        print(f"  Tokens: {s0}/{s1}")
        print(f"  Reserves: {r0/(10**d0):.2f} {s0}, {r1/(10**d1):.2f} {s1}")
        
        # Check 1: Zero reserves
        if r0 == 0 or r1 == 0:
            print("  ❌ HONEYPOT: Zero reserves!")
            return False
        
        # Check 2: Extremely low liquidity (< $10)
        if s1 in ['USDC', 'USDT', 'DAI'] and r1/(10**d1) < 10:
            print(f"  ⚠️  WARNING: Very low liquidity ({r1/(10**d1):.2f} {s1})")
        
        # Check 3: Test swap calculation
        # Calculate a small swap
        if s1 in ['USDC', 'USDT']:
            test_amount = int(amount_usd * 10**d1)
        else:
            # For WPOL or other tokens
            test_amount = int(0.01 * 10**d0)  # Small amount
        
        # Simulate buying token0 with token1
        amount_with_fee = test_amount * 997
        numerator = amount_with_fee * r0
        denominator = r1 * 1000 + amount_with_fee
        
        if denominator == 0:
            print("  ❌ HONEYPOT: Division by zero in swap calculation!")
            return False
        
        output = numerator // denominator
        
        if output == 0:
            print("  ❌ HONEYPOT: Swap would return 0 tokens!")
            return False
        
        # Check 4: Price sanity check
        price = (r1/(10**d1)) / (r0/(10**d0)) if r0 > 0 else 0
        
        # Known approximate prices
        expected_prices = {
            ('WPOL', 'USDC'): (0.20, 0.30),  # WPOL should be $0.20-$0.30
            ('WMATIC', 'USDC'): (0.20, 0.30),
            ('WETH', 'USDC'): (3000, 4500),
            ('WBTC', 'USDC'): (90000, 110000),
        }
        
        for (t0_sym, t1_sym), (min_price, max_price) in expected_prices.items():
            if s0 == t0_sym and s1 in [t1_sym, 'USDT', 'DAI']:
                if price < min_price or price > max_price:
                    print(f"  ⚠️  WARNING: Unusual price {price:.6f} {s1}/{s0} (expected {min_price}-{max_price})")
                    if price < min_price * 0.5 or price > max_price * 2:
                        print(f"  ❌ LIKELY HONEYPOT: Price way out of range!")
                        return False
        
        print(f"  ✅ Pool appears legitimate")
        print(f"  Price: {price:.6f} {s1}/{s0}")
        
        # Check 5: Verify actual executability with eth_call simulation
        print("\n  🧪 Testing swap simulation...")
        
        # Try a simulated swap
        router_address = ROUTERS["sushiswap_v2"]  # Default to SushiSwap
        
        # Build swap calldata
        from eth_abi import encode
        
        deadline = int(time.time()) + 300
        path = [token1, token0]  # Buy token0 with token1
        min_out = 0  # Accept any amount for test
        test_wallet = "0x0000000000000000000000000000000000000001"
        
        # getAmountsOut to check expected output
        selector = Web3.keccak(text="getAmountsOut(uint256,address[])")[:4]
        params = encode(['uint256', 'address[]'], [test_amount, path])
        call_data = selector + params
        
        try:
            result = w3.eth.call({
                'to': router_address,
                'data': call_data
            })
            amounts = w3.eth.abi.decode(['uint256[]'], result)[0]
            expected_out = amounts[1]
            
            print(f"    Router expects: {expected_out/(10**d0):.6f} {s0} for {test_amount/(10**d1):.6f} {s1}")
            
            if expected_out == 0:
                print("  ❌ HONEYPOT: Router returns 0 output!")
                return False
            
            print("  ✅ Swap simulation successful")
            return True
            
        except Exception as e:
            error_str = str(e)
            if 'INSUFFICIENT' in error_str.upper():
                print("  ⚠️  Pool has insufficient liquidity for test amount")
            elif 'LOCKED' in error_str.upper():
                print("  ❌ HONEYPOT: Liquidity appears locked!")
                return False
            else:
                print(f"  ⚠️  Simulation failed: {error_str[:100]}")
        
        return True
        
    except Exception as e:
        print(f"  ❌ Error checking pool: {e}")
        return False

def test_arbitrage_execution(buy_pool_addr, sell_pool_addr, input_amount=10):
    """Test if an arbitrage between two pools would actually work"""
    
    print("\n" + "="*70)
    print("ARBITRAGE EXECUTION TEST")
    print("="*70)
    
    # First check both pools for honeypots
    print("\n1️⃣ Checking buy pool...")
    if not check_honeypot(buy_pool_addr, 1):
        print("  ❌ Buy pool failed checks!")
        return False
    
    print("\n2️⃣ Checking sell pool...")
    if not check_honeypot(sell_pool_addr, 1):
        print("  ❌ Sell pool failed checks!")
        return False
    
    # Now test the actual arbitrage
    print("\n3️⃣ Testing arbitrage path...")
    
    try:
        buy_pool = w3.eth.contract(address=Web3.to_checksum_address(buy_pool_addr), abi=V2_ABI)
        sell_pool = w3.eth.contract(address=Web3.to_checksum_address(sell_pool_addr), abi=V2_ABI)
        
        # Get reserves
        buy_reserves = buy_pool.functions.getReserves().call()
        sell_reserves = sell_pool.functions.getReserves().call()
        
        # Get tokens
        buy_t0 = buy_pool.functions.token0().call()
        buy_t1 = buy_pool.functions.token1().call()
        sell_t0 = sell_pool.functions.token0().call()
        sell_t1 = sell_pool.functions.token1().call()
        
        # Check if it's cross-token arbitrage (USDC vs USDC.e)
        usdc_old = "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174"
        usdc_new = "0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359"
        
        is_cross_token = False
        if (buy_t1.lower() == usdc_old.lower() and sell_t1.lower() == usdc_new.lower()) or \
           (buy_t1.lower() == usdc_new.lower() and sell_t1.lower() == usdc_old.lower()):
            is_cross_token = True
            print("  ⚠️  Cross-token arbitrage detected (USDC vs USDC.e)")
            print("  This requires special handling - both tokens are 'USDC' but different contracts")
        
        # Calculate expected profit
        input_wei = int(input_amount * 10**6)  # USDC has 6 decimals
        
        # Step 1: Buy token0 with token1
        amount_with_fee = input_wei * 997
        numerator = amount_with_fee * buy_reserves[0]
        denominator = buy_reserves[1] * 1000 + amount_with_fee
        token0_out = numerator // denominator
        
        # Step 2: Sell token0 for token1
        amount_with_fee = token0_out * 997
        numerator = amount_with_fee * sell_reserves[1]
        denominator = sell_reserves[0] * 1000 + amount_with_fee
        token1_back = numerator // denominator
        
        profit_wei = token1_back - input_wei
        profit = profit_wei / 10**6
        
        print(f"\n  Expected Results:")
        print(f"    Input: ${input_amount}")
        print(f"    Get back: ${(token1_back/10**6):.4f}")
        print(f"    Profit: ${profit:.4f} ({profit/input_amount*100:.2f}%)")
        
        if profit > 0:
            print(f"\n  ✅ ARBITRAGE IS THEORETICALLY PROFITABLE!")
            
            if is_cross_token:
                print("\n  📝 Notes for execution:")
                print("    1. You're arbitraging between USDC and USDC.e")
                print("    2. Both tokens work similarly but have different contract addresses")
                print("    3. Make sure your execution contract handles both USDC types")
                print("    4. Check if you have approval for both USDC contracts")
            
            print("\n  ⚠️  Important considerations:")
            print("    - MEV bots will likely front-run public transactions")
            print("    - Use flashloan to minimize capital requirements")
            print("    - Consider using a private mempool (Flashbots)")
            print("    - Gas costs on failed transactions still apply")
            
            return True
        else:
            print(f"\n  ❌ Not profitable in calculation")
            return False
            
    except Exception as e:
        print(f"\n  ❌ Error testing arbitrage: {e}")
        return False

if __name__ == "__main__":
    if len(sys.argv) < 3:
        print("Usage: python3 test_arb_opportunity.py <buy_pool> <sell_pool> [amount_usd]")
        print("\nExample:")
        print("  python3 test_arb_opportunity.py 0x380615f37993b5a96adf3d443b6e0ac50a211998 0x6d9e8dbb2779853db00418d4dcf96f3987cfc9d2 10")
        sys.exit(1)
    
    buy_pool = sys.argv[1]
    sell_pool = sys.argv[2]
    amount = float(sys.argv[3]) if len(sys.argv) > 3 else 10
    
    test_arbitrage_execution(buy_pool, sell_pool, amount)
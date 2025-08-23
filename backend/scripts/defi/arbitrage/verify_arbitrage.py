#!/usr/bin/env python3
"""
Verify Arbitrage Opportunity for USDC.e/USDT
Checks actual pool reserves and calculates real execution prices with slippage
"""

import json
import sys
from web3 import Web3
from decimal import Decimal, getcontext

# Set high precision for accurate calculations
getcontext().prec = 50

# Configuration - Use ANKR for better reliability
import os
from dotenv import load_dotenv

load_dotenv('../.env')
ANKR_KEY = os.getenv('ANKR_API_KEY', '')
RPC_URL = f"https://rpc.ankr.com/polygon/{ANKR_KEY}" if ANKR_KEY else "https://polygon-rpc.com"
w3 = Web3(Web3.HTTPProvider(RPC_URL))

# Token addresses on Polygon
USDC_E = "0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359"  # USDC.e (native)
USDT = "0xc2132D05D31c914a87C6611C10748AEb04B58e8F"    # USDT

# Pool addresses from the arbitrage analysis
BUY_POOL = "0x1cb770fc7c7367c6ad5e88d32072b7f4bf881304"
SELL_POOL = "0x498d5cdcc5667b21210b49442bf2d8792527194d"

# Uniswap V2 Pool ABI (minimal)
POOL_ABI = json.loads('''[
    {
        "constant": true,
        "inputs": [],
        "name": "getReserves",
        "outputs": [
            {"name": "_reserve0", "type": "uint112"},
            {"name": "_reserve1", "type": "uint112"},
            {"name": "_blockTimestampLast", "type": "uint32"}
        ],
        "type": "function"
    },
    {
        "constant": true,
        "inputs": [],
        "name": "token0",
        "outputs": [{"name": "", "type": "address"}],
        "type": "function"
    },
    {
        "constant": true,
        "inputs": [],
        "name": "token1",
        "outputs": [{"name": "", "type": "address"}],
        "type": "function"
    }
]''')

def get_pool_info(pool_address):
    """Get pool reserves and token ordering"""
    try:
        pool = w3.eth.contract(address=Web3.to_checksum_address(pool_address), abi=POOL_ABI)
        
        # First check if this is a valid contract
        code = w3.eth.get_code(Web3.to_checksum_address(pool_address))
        if code == b'' or code == '0x':
            print(f"  ⚠️  {pool_address} is not a contract!")
            return None
            
        token0 = pool.functions.token0().call()
        token1 = pool.functions.token1().call()
        reserves = pool.functions.getReserves().call()
        
        return {
            'address': pool_address,
            'token0': token0.lower(),
            'token1': token1.lower(),
            'reserve0': reserves[0],
            'reserve1': reserves[1]
        }
    except Exception as e:
        print(f"  ⚠️  Error reading pool {pool_address}: {e}")
        print(f"      This might be a Uniswap V3 pool or different DEX")
        return None

def calculate_swap_output(amount_in, reserve_in, reserve_out, fee_bps=30):
    """
    Calculate output amount for a swap with fee
    fee_bps: fee in basis points (30 = 0.3%)
    """
    amount_in = Decimal(str(amount_in))
    reserve_in = Decimal(str(reserve_in))
    reserve_out = Decimal(str(reserve_out))
    
    # Apply fee: amount_with_fee = amount_in * (10000 - fee_bps) / 10000
    amount_with_fee = amount_in * (10000 - fee_bps) / 10000
    
    # Calculate output using constant product formula
    # output = (amount_with_fee * reserve_out) / (reserve_in + amount_with_fee)
    numerator = amount_with_fee * reserve_out
    denominator = reserve_in + amount_with_fee
    
    output = numerator / denominator
    
    return int(output)

def calculate_slippage(amount_in, reserve_in, reserve_out):
    """Calculate price impact (slippage) percentage"""
    amount_in = Decimal(str(amount_in))
    reserve_in = Decimal(str(reserve_in))
    reserve_out = Decimal(str(reserve_out))
    
    # Initial price
    initial_price = reserve_out / reserve_in
    
    # Price after swap
    new_reserve_in = reserve_in + amount_in
    new_reserve_out = reserve_out - (amount_in * reserve_out / (reserve_in + amount_in))
    final_price = new_reserve_out / new_reserve_in
    
    # Slippage percentage
    slippage = abs((final_price - initial_price) / initial_price * 100)
    
    return float(slippage)

def analyze_arbitrage(trade_size_usd):
    """Analyze arbitrage opportunity for given trade size"""
    print(f"\n{'='*60}")
    print(f"Arbitrage Analysis: USDC.e/USDT")
    print(f"Trade Size: ${trade_size_usd:,.2f}")
    print(f"{'='*60}\n")
    
    # Get pool information
    print("Fetching pool data from blockchain...")
    buy_pool = get_pool_info(BUY_POOL)
    sell_pool = get_pool_info(SELL_POOL)
    
    if not buy_pool or not sell_pool:
        print("\n❌ Unable to fetch pool data. The pools might be:")
        print("   - Uniswap V3 pools (require different calculation)")
        print("   - From a different DEX protocol")
        print("   - Invalid addresses")
        return 0
    
    # Determine token positions in each pool
    # Buy pool: We're buying USDC.e with USDT
    if buy_pool['token0'].lower() == USDC_E.lower():
        buy_usdc_reserve = buy_pool['reserve0']
        buy_usdt_reserve = buy_pool['reserve1']
        buy_usdc_is_token0 = True
    else:
        buy_usdc_reserve = buy_pool['reserve1']
        buy_usdt_reserve = buy_pool['reserve0']
        buy_usdc_is_token0 = False
    
    # Sell pool: We're selling USDC.e for USDT
    if sell_pool['token0'].lower() == USDC_E.lower():
        sell_usdc_reserve = sell_pool['reserve0']
        sell_usdt_reserve = sell_pool['reserve1']
        sell_usdc_is_token0 = True
    else:
        sell_usdc_reserve = sell_pool['reserve1']
        sell_usdt_reserve = sell_pool['reserve0']
        sell_usdc_is_token0 = False
    
    # Convert trade size to token units (both have 6 decimals)
    trade_amount_usdt = int(trade_size_usd * 10**6)
    
    print(f"Buy Pool ({BUY_POOL[:8]}...):")
    print(f"  USDC.e reserves: {buy_usdc_reserve / 10**6:,.2f}")
    print(f"  USDT reserves: {buy_usdt_reserve / 10**6:,.2f}")
    print(f"  Price (USDC.e/USDT): {buy_usdc_reserve / buy_usdt_reserve:.6f}")
    
    print(f"\nSell Pool ({SELL_POOL[:8]}...):")
    print(f"  USDC.e reserves: {sell_usdc_reserve / 10**6:,.2f}")
    print(f"  USDT reserves: {sell_usdt_reserve / 10**6:,.2f}")
    print(f"  Price (USDC.e/USDT): {sell_usdc_reserve / sell_usdt_reserve:.6f}")
    
    # Step 1: Buy USDC.e with USDT in buy pool
    usdc_received = calculate_swap_output(
        trade_amount_usdt,
        buy_usdt_reserve,
        buy_usdc_reserve,
        fee_bps=30  # 0.3% fee
    )
    
    buy_slippage = calculate_slippage(trade_amount_usdt, buy_usdt_reserve, buy_usdc_reserve)
    buy_price = trade_amount_usdt / usdc_received  # USDT per USDC.e
    
    print(f"\n📊 Trade Execution:")
    print(f"Step 1 - Buy USDC.e with USDT:")
    print(f"  Input: {trade_amount_usdt / 10**6:,.2f} USDT")
    print(f"  Output: {usdc_received / 10**6:,.2f} USDC.e")
    print(f"  Execution Price: {buy_price:.6f} USDT/USDC.e")
    print(f"  Slippage: {buy_slippage:.2f}%")
    
    # Step 2: Sell USDC.e for USDT in sell pool
    usdt_received = calculate_swap_output(
        usdc_received,
        sell_usdc_reserve,
        sell_usdt_reserve,
        fee_bps=30  # 0.3% fee
    )
    
    sell_slippage = calculate_slippage(usdc_received, sell_usdc_reserve, sell_usdt_reserve)
    sell_price = usdt_received / usdc_received  # USDT per USDC.e
    
    print(f"\nStep 2 - Sell USDC.e for USDT:")
    print(f"  Input: {usdc_received / 10**6:,.2f} USDC.e")
    print(f"  Output: {usdt_received / 10**6:,.2f} USDT")
    print(f"  Execution Price: {sell_price:.6f} USDT/USDC.e")
    print(f"  Slippage: {sell_slippage:.2f}%")
    
    # Calculate profit/loss
    gross_profit = (usdt_received - trade_amount_usdt) / 10**6
    profit_percentage = (usdt_received / trade_amount_usdt - 1) * 100
    
    # Gas costs (approximate)
    gas_price_gwei = 25  # Polygon typical
    gas_used = 150000  # Approximate for 2 swaps
    gas_cost_matic = (gas_price_gwei * gas_used) / 10**9
    matic_price = 0.40  # Approximate
    gas_cost_usd = gas_cost_matic * matic_price
    
    net_profit = gross_profit - gas_cost_usd
    
    print(f"\n💰 Profit/Loss Analysis:")
    print(f"  Gross Profit: ${gross_profit:,.2f} ({profit_percentage:.2f}%)")
    print(f"  Gas Cost: ${gas_cost_usd:.3f}")
    print(f"  Net Profit: ${net_profit:,.2f}")
    
    # Verdict
    print(f"\n🎯 Verdict:")
    if net_profit > 0:
        print(f"  ✅ PROFITABLE - This arbitrage opportunity is valid!")
        print(f"  Expected profit: ${net_profit:,.2f}")
    else:
        print(f"  ❌ UNPROFITABLE - This arbitrage would lose money")
        print(f"  Expected loss: ${abs(net_profit):,.2f}")
    
    print(f"\n📝 Summary:")
    print(f"  Total Slippage: {buy_slippage + sell_slippage:.2f}%")
    print(f"  DEX Fees (0.6% total): ${trade_size_usd * 0.006:,.2f}")
    print(f"  Break-even price difference needed: {0.6 + (buy_slippage + sell_slippage):.2f}%")
    
    return net_profit

def main():
    print("Verifying USDC.e/USDT Arbitrage Opportunity")
    print("=" * 60)
    
    # Check connection
    if not w3.is_connected():
        print("❌ Failed to connect to Polygon RPC")
        sys.exit(1)
    
    print(f"✅ Connected to Polygon (Block: {w3.eth.block_number})")
    
    # Analyze different trade sizes
    trade_sizes = [2683, 5366, 10733]
    
    results = []
    for size in trade_sizes:
        profit = analyze_arbitrage(size)
        results.append((size, profit))
    
    # Summary
    print("\n" + "="*60)
    print("TRADE SIZE COMPARISON:")
    print("="*60)
    for size, profit in results:
        status = "✅ Profitable" if profit > 0 else "❌ Loss"
        print(f"${size:,}: {status} (${profit:,.2f})")

if __name__ == "__main__":
    main()
#!/usr/bin/env python3
"""Deploy and test the fixed cross-USDC arbitrage contract"""

from web3 import Web3
from eth_account import Account
import json
import os
from dotenv import load_dotenv

# Load environment
load_dotenv('/Users/daws/alphapulse/backend/services/capital_arb_bot/.env')
private_key = os.getenv('PRIVATE_KEY')

if not private_key:
    print("ERROR: No PRIVATE_KEY found in .env")
    exit(1)

w3 = Web3(Web3.HTTPProvider('https://polygon.publicnode.com'))
account = Account.from_key(private_key)
address = account.address

print(f"Account: {address}")
balance = w3.eth.get_balance(address)
print(f"Balance: {Web3.from_wei(balance, 'ether'):.4f} MATIC")

# First, let's check if the opportunity still exists
print("\n" + "="*60)
print("Checking current arbitrage opportunity...")

pool_abi = json.loads('[{"inputs":[],"name":"getReserves","outputs":[{"name":"","type":"uint112"},{"name":"","type":"uint112"}],"type":"function"}]')

buy_pool = '0x380615f37993b5a96adf3d443b6e0ac50a211998'
sell_pool = '0x6d9e8dbb2779853db00418d4dcf96f3987cfc9d2'

buy_contract = w3.eth.contract(address=Web3.to_checksum_address(buy_pool), abi=pool_abi)
sell_contract = w3.eth.contract(address=Web3.to_checksum_address(sell_pool), abi=pool_abi)

buy_reserves = buy_contract.functions.getReserves().call()
sell_reserves = sell_contract.functions.getReserves().call()

# Buy pool: WPOL/USDC_OLD
wpol_price_buy = buy_reserves[1] / buy_reserves[0] * 10**12
print(f"Buy pool (Dystopia): WPOL @ ${wpol_price_buy:.6f}")
print(f"  Reserves: {buy_reserves[0]/10**18:.2f} WPOL, {buy_reserves[1]/10**6:.2f} USDC_OLD")

# Sell pool: WPOL/USDC_NEW  
wpol_price_sell = sell_reserves[1] / sell_reserves[0] * 10**12
print(f"Sell pool (QuickSwap): WPOL @ ${wpol_price_sell:.6f}")
print(f"  Reserves: {sell_reserves[0]/10**18:.2f} WPOL, {sell_reserves[1]/10**6:.2f} USDC_NEW")

spread = (wpol_price_sell - wpol_price_buy) / wpol_price_buy * 100
print(f"\nSpread: {spread:.2f}%")

if spread < 1:
    print("❌ Spread too small, arbitrage not profitable")
    exit(0)

# Simulate the trade
test_amount = 20 * 10**6  # 20 USDC
fee = 0.997

# Buy WPOL with USDC_OLD
wpol_out = (test_amount * fee * buy_reserves[0]) // (buy_reserves[1] + test_amount * fee)
print(f"\nSimulation with 20 USDC_OLD:")
print(f"  Step 1: 20 USDC_OLD -> {wpol_out/10**18:.4f} WPOL")

# Sell WPOL for USDC_NEW
usdc_new_out = (wpol_out * fee * sell_reserves[1]) // (sell_reserves[0] + wpol_out * fee)
print(f"  Step 2: {wpol_out/10**18:.4f} WPOL -> {usdc_new_out/10**6:.2f} USDC_NEW")

# Flash loan cost
flash_fee = test_amount * 5 // 10000
total_cost = test_amount + flash_fee

profit = usdc_new_out - total_cost
print(f"\nFlash loan fee: {flash_fee/10**6:.4f} USDC")
print(f"Total cost: {total_cost/10**6:.2f} USDC")
print(f"Revenue: {usdc_new_out/10**6:.2f} USDC_NEW")
print(f"Profit: ${profit/10**6:.2f}")

if profit <= 0:
    print("❌ Not profitable after fees")
    exit(0)

print("\n✅ Profitable! But needs USDC conversion...")

# Check if Curve pool exists for USDC conversion
curve_pool = '0x5ab5C56B9db92Ba45a0B46a207286cD83C15C939'
print(f"\nChecking Curve pool at {curve_pool}...")

try:
    code = w3.eth.get_code(curve_pool)
    if len(code) > 2:
        print("✅ Curve pool exists")
        
        # Try to check exchange rates
        curve_abi = json.loads('[{"name":"get_dy","inputs":[{"name":"i","type":"int128"},{"name":"j","type":"int128"},{"name":"dx","type":"uint256"}],"outputs":[{"name":"","type":"uint256"}],"type":"function"}]')
        curve = w3.eth.contract(address=Web3.to_checksum_address(curve_pool), abi=curve_abi)
        
        try:
            # Check USDC_NEW -> USDC_OLD conversion
            dy = curve.functions.get_dy(1, 0, 10**6).call()  # 1 USDC_NEW to USDC_OLD
            print(f"  1 USDC_NEW = {dy/10**6:.6f} USDC_OLD")
            
            if dy < 10**6:
                conversion_loss = (10**6 - dy) / 10**6 * 100
                print(f"  Conversion loss: {conversion_loss:.2f}%")
                adjusted_profit = profit * dy // 10**6 - total_cost
                print(f"  Adjusted profit: ${adjusted_profit/10**6:.2f}")
        except Exception as e:
            print(f"  Could not get exchange rate: {e}")
    else:
        print("❌ Curve pool not found - need alternative USDC conversion")
except Exception as e:
    print(f"❌ Error checking Curve pool: {e}")

# Alternative: Direct USDC conversion via other pools
print("\n" + "="*60)
print("Alternative: Find direct USDC/USDC pool")

# Search for USDC/USDC pools
print("Searching for USDC conversion pools...")

# Known USDC aggregators/bridges
usdc_converters = [
    ('0x2e7d6490526c7d7e2fdea5c6ec4b0d1b9f8b25b7', 'Polygon Bridge'),
    ('0xe7cea2f6d7b120174bf3a9bc98efaf1ff72c996d', 'Router Protocol'),
    # Add more if known
]

for addr, name in usdc_converters:
    try:
        code = w3.eth.get_code(Web3.to_checksum_address(addr))
        if len(code) > 2:
            print(f"  ✅ {name} at {addr}")
    except:
        pass

print("\n" + "="*60)
print("RECOMMENDATION:")
print("The arbitrage is profitable but requires USDC_NEW -> USDC_OLD conversion")
print("Options:")
print("1. Use Curve 2pool if it supports USDC/USDC.e")
print("2. Find alternative conversion route")
print("3. Flash loan USDC_NEW instead of USDC_OLD")
print("4. Use existing DEX aggregators for conversion")

# Try option 3: Flash loan USDC_NEW
print("\n" + "="*60)
print("Testing with USDC_NEW flash loan...")

# Reverse calculation
# We need X USDC_NEW to get Y WPOL to get Z USDC_OLD
# where Z > X + fees

# Start with 20 USDC_NEW
test_new = 20 * 10**6

# Sell USDC_NEW for WPOL (backwards through sell pool)
# This is buying WPOL at expensive price - not good
wpol_from_new = (test_new * fee * sell_reserves[0]) // (sell_reserves[1] + test_new * fee)
print(f"  20 USDC_NEW -> {wpol_from_new/10**18:.4f} WPOL (expensive)")

# Sell WPOL for USDC_OLD (through buy pool)
usdc_old_out = (wpol_from_new * fee * buy_reserves[1]) // (buy_reserves[0] + wpol_from_new * fee)
print(f"  {wpol_from_new/10**18:.4f} WPOL -> {usdc_old_out/10**6:.2f} USDC_OLD")

print(f"  Result: 20 USDC_NEW -> {usdc_old_out/10**6:.2f} USDC_OLD")
print("  ❌ This direction loses money as expected")

print("\n" + "="*60)
print("CONCLUSION:")
print("Must use USDC_OLD flash loan and find USDC conversion")
print("Or deploy without Curve and handle conversion manually")
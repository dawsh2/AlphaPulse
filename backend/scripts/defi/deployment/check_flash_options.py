#!/usr/bin/env python3
"""Check flash loan options for USDC_NEW and test the arbitrage flow"""

from web3 import Web3
import json

w3 = Web3(Web3.HTTPProvider('https://polygon.publicnode.com'))

USDC_OLD = '0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174'
USDC_NEW = '0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359'

# Aave V3 Pool
aave_pool = '0x794a61358D6845594F94dc1DB02A252b5b4814aD'

print("Checking Aave V3 Flash Loan Options:")
print("="*60)

# Check available liquidity
erc20_abi = json.loads('[{"inputs":[{"name":"account","type":"address"}],"name":"balanceOf","outputs":[{"name":"","type":"uint256"}],"type":"function"}]')

for name, addr in [("USDC_OLD", USDC_OLD), ("USDC_NEW", USDC_NEW)]:
    token = w3.eth.contract(address=Web3.to_checksum_address(addr), abi=erc20_abi)
    balance = token.functions.balanceOf(aave_pool).call()
    print(f"{name}: ${balance/10**6:,.0f} available in Aave")

print("\n" + "="*60)
print("Option 1: Flash loan USDC_NEW directly")
print("-"*40)

# The flow would be:
# 1. Flash loan USDC_NEW
# 2. Swap USDC_NEW -> WPOL on sell pool (but at expensive price!)
# 3. Swap WPOL -> USDC_OLD on buy pool (at cheap price)
# 4. Swap USDC_OLD -> USDC_NEW to repay
# This is backwards and won't work!

print("❌ Problem: This reverses the arbitrage direction")
print("   We'd be buying WPOL at $0.241 and selling at $0.082")
print("   This loses money!")

print("\n" + "="*60)
print("Option 2: Flash loan USDC_OLD, keep profit in USDC_NEW")
print("-"*40)

# The correct flow:
print("✅ This is the RIGHT approach:")
print("1. Flash loan 100 USDC_OLD from Aave")
print("2. Buy WPOL with USDC_OLD at $0.082 (cheap)")
print("3. Sell WPOL for USDC_NEW at $0.241 (expensive)")
print("4. Convert just enough USDC_NEW to USDC_OLD to repay loan")
print("5. Keep remaining USDC_NEW as profit")

# Calculate the numbers
loan_amount = 100 * 10**6  # 100 USDC_OLD
flash_fee = loan_amount * 5 // 10000  # 0.05%
total_debt = loan_amount + flash_fee

# Simulate the trade
buy_reserves = (2764.69 * 10**18, 226.02 * 10**6)  # WPOL, USDC_OLD
sell_reserves = (2194.00 * 10**18, 530.43 * 10**6)  # WPOL, USDC_NEW

fee = 0.997
wpol_out = (loan_amount * fee * buy_reserves[0]) // (buy_reserves[1] + loan_amount * fee)
usdc_new_out = (wpol_out * fee * sell_reserves[1]) // (sell_reserves[0] + wpol_out * fee)

print(f"\nWith 100 USDC_OLD flash loan:")
print(f"  Get: {usdc_new_out/10**6:.2f} USDC_NEW")
print(f"  Need to repay: {total_debt/10**6:.2f} USDC_OLD")
print(f"  Convert {total_debt/10**6:.2f} USDC_NEW to USDC_OLD")
print(f"  Keep profit: {(usdc_new_out - total_debt)/10**6:.2f} USDC_NEW")

print("\n" + "="*60)
print("BEST SOLUTION:")
print("Modify the contract to:")
print("1. Flash loan USDC_OLD (plenty of liquidity)")
print("2. Do the arbitrage")
print("3. Convert only what's needed for repayment")
print("4. Send profit to owner in USDC_NEW")
print("\nThis minimizes conversion costs and is simpler!")
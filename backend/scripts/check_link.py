#!/usr/bin/env python3
"""Quick check of LINK/WETH pools"""
import sys
sys.path.append('.')
from arb_multi import *

pools_link = [
    "0x70bf5ffcc6090a8d243fd05485ec4c084bd10ae5",  # $0.005733
    "0x436803355d26943dd0bc9826d39f9079199a890a",  # $0.005736
    "0x3db7148e24be957a6229404c3f7a5fdc948ae543",  # $0.005740
    "0x3e31ab7f37c048fc6574189135d108df80f0ea26",  # $0.005742
    "0x74d23f21f780ca26b47db16b0504f2e3832b9321",  # $0.005755
    "0x5ca6ca6c3709e1e6cfe74a50cf6b2b6ba2dadd67",  # $0.005757
    "0x3dc10d7bfb94eeb009203e84a653e5764f71771d",  # $0.006032
]

print("LINK/WETH POOL ANALYSIS")
print("="*60)

pools = []
for addr in pools_link:
    p = load_pool(addr)
    if p:
        pools.append(p)
        liq = p.get('liquidity_usd', 0)
        print(f"{p['type']} {p['symbol0']}/{p['symbol1']} @ {p['price']:.6f}")
        print(f"  Fee: {p['fee_bps']/100:.2f}%, Liquidity: ${liq:.2f}")

# Find best arb
if len(pools) >= 2:
    min_price = min(p['price'] for p in pools)
    max_price = max(p['price'] for p in pools)
    
    buy_pool = [p for p in pools if p['price'] == min_price][0]
    sell_pool = [p for p in pools if p['price'] == max_price][0]
    
    spread_pct = (max_price - min_price) / min_price * 100
    fees_pct = (buy_pool['fee_bps'] + sell_pool['fee_bps']) / 100
    
    print(f"\n{'='*60}")
    print("BEST ARBITRAGE OPPORTUNITY")
    print(f"Buy: {buy_pool['address'][:10]}... @ {buy_pool['price']:.6f}")
    print(f"Sell: {sell_pool['address'][:10]}... @ {sell_pool['price']:.6f}")
    print(f"Spread: {spread_pct:.3f}%")
    print(f"Fees: {fees_pct:.2f}%")
    print(f"Net spread: {spread_pct - fees_pct:.3f}%")
    
    # Check liquidity
    min_liq = min(buy_pool.get('liquidity_usd', 0), sell_pool.get('liquidity_usd', 0))
    print(f"\n💧 LIQUIDITY ANALYSIS:")
    print(f"Buy pool liquidity: ${buy_pool.get('liquidity_usd', 0):.2f}")
    print(f"Sell pool liquidity: ${sell_pool.get('liquidity_usd', 0):.2f}")
    print(f"Min liquidity: ${min_liq:.2f}")
    
    if min_liq > 0:
        max_trade = min_liq * 0.01
        gross_profit = max_trade * (spread_pct - fees_pct) / 100
        gas_cost = 0.003
        net_profit = gross_profit - gas_cost
        
        print(f"\n💰 PROFIT CALCULATION:")
        print(f"Max safe trade (1% of liquidity): ${max_trade:.2f}")
        print(f"Gross profit: ${gross_profit:.4f}")
        print(f"Gas cost: ${gas_cost:.4f}")
        print(f"Net profit: ${net_profit:.4f}")
        
        if net_profit > 0:
            print(f"\n✅ PROFITABLE: ${net_profit:.4f}")
        else:
            print(f"\n❌ LOSS: ${net_profit:.4f}")
    else:
        print("\n❌ NO LIQUIDITY - All pools show $0!")
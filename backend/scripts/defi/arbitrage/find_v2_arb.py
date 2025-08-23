#!/usr/bin/env python3
from web3 import Web3
import json

w3 = Web3(Web3.HTTPProvider('https://polygon.publicnode.com'))

# Analyze pools to find best standard V2 opportunities
pools_to_check = [
    '0x6d9e8dbb2779853db00418d4dcf96f3987cfc9d2',  # QuickSwap V2 WPOL/USDC.e
    '0x6e7a5fafcec6bb1e78bae2a1f0b612012bf14827',  # V2 pool
    '0x29a92b95be45d5bdd638b749798f0fee107fdbc7',  # V2 pool
    '0x019011032a7ac3a87ee885b6c08467ac46ad11cd',  # V2 pool
    '0x82404e05857e83ba35faa99824245cdf641845e3',  # V2 pool
    '0x60c088234180b36edcec7aa8aa23912bb6bed114',  # V2 pool
    '0x5e58e0ced3a272caeb8ba00f4a4c2805df6be495',  # V2 pool
    '0x8a4ceb4dffa238539c5d62ce424980e8fdb21ebc',  # V2 pool
]

V2_ABI = json.loads('[{"inputs":[],"name":"getReserves","outputs":[{"name":"","type":"uint112"},{"name":"","type":"uint112"}],"type":"function"},{"inputs":[],"name":"token0","outputs":[{"name":"","type":"address"}],"type":"function"},{"inputs":[],"name":"token1","outputs":[{"name":"","type":"address"}],"type":"function"},{"inputs":[],"name":"factory","outputs":[{"name":"","type":"address"}],"type":"function"}]')

print('Standard V2 Pool Analysis:')
print('='*50)

v2_pools = []
for addr in pools_to_check:
    try:
        pool = w3.eth.contract(address=Web3.to_checksum_address(addr), abi=V2_ABI)
        reserves = pool.functions.getReserves().call()
        token0 = pool.functions.token0().call()
        token1 = pool.functions.token1().call()
        factory = pool.functions.factory().call()
        
        # Check if it's WPOL/USDC pair
        wpol = '0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270'
        usdc_old = '0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174'
        usdc_new = '0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359'
        
        is_wpol_pair = False
        usdc_type = None
        
        if token0.lower() == wpol.lower():
            if token1.lower() == usdc_old.lower():
                is_wpol_pair = True
                usdc_type = 'USDC_OLD'
            elif token1.lower() == usdc_new.lower():
                is_wpol_pair = True
                usdc_type = 'USDC.e'
        
        if is_wpol_pair:
            price = reserves[1] / reserves[0] * 10**12  # Adjust for decimals
            
            # Known factories
            factory_name = 'Unknown'
            if factory.lower() == '0x5757371414417b8c6caad45baef941abc7d3ab32':
                factory_name = 'QuickSwap'
            elif factory.lower() == '0xc35dadb65012ec5796536bd9864ed8773abc74c4':
                factory_name = 'SushiSwap'
                
            v2_pools.append({
                'address': addr,
                'factory': factory_name,
                'usdc_type': usdc_type,
                'price': price,
                'reserves_wpol': reserves[0] / 10**18,
                'reserves_usdc': reserves[1] / 10**6
            })
            
            print(f'{addr[:10]}... ({factory_name}):')
            print(f'  WPOL/{usdc_type}: {price:.6f}')
            print(f'  Reserves: {reserves[0]/10**18:.0f} WPOL, {reserves[1]/10**6:.0f} USDC')
            
    except Exception as e:
        pass

# Find best arbitrage among V2 pools
print('\n' + '='*50)
print('Best V2 Arbitrage Opportunities:')

if len(v2_pools) >= 2:
    # Find cross-USDC opportunities
    old_pools = [p for p in v2_pools if p['usdc_type'] == 'USDC_OLD']
    new_pools = [p for p in v2_pools if p['usdc_type'] == 'USDC.e']
    
    if old_pools and new_pools:
        print('\n💎 CROSS-USDC ARBITRAGE OPPORTUNITIES:')
        
        for old_p in old_pools:
            for new_p in new_pools:
                spread = abs(new_p['price'] - old_p['price']) / min(old_p['price'], new_p['price']) * 100
                
                if spread > 0.6:  # More than fees
                    if old_p['price'] < new_p['price']:
                        buy_pool, sell_pool = old_p, new_p
                    else:
                        buy_pool, sell_pool = new_p, old_p
                    
                    print(f'\nOpportunity found:')
                    print(f'  Buy from: {buy_pool["address"][:10]}... ({buy_pool["factory"]})')
                    print(f'    Price: {buy_pool["price"]:.6f} {buy_pool["usdc_type"]}')
                    print(f'  Sell to: {sell_pool["address"][:10]}... ({sell_pool["factory"]})')  
                    print(f'    Price: {sell_pool["price"]:.6f} {sell_pool["usdc_type"]}')
                    print(f'  Spread: {spread:.2f}%')
                    
                    # Calculate profit on 100 USDC
                    test_amount = 100 * 10**6
                    fee = 0.997
                    
                    # Simulate trade
                    wpol_out_raw = (test_amount * fee * int(buy_pool['reserves_wpol'] * 10**18)) // (int(buy_pool['reserves_usdc'] * 10**6) + test_amount * fee)
                    usdc_back_raw = (wpol_out_raw * fee * int(sell_pool['reserves_usdc'] * 10**6)) // (int(sell_pool['reserves_wpol'] * 10**18) + wpol_out_raw * fee)
                    
                    profit = (usdc_back_raw - test_amount) / 10**6
                    print(f'  Expected profit on 100 USDC: ${profit:.2f}')
                    
                    if profit > 10:
                        print(f'  🚀 HIGHLY PROFITABLE! Execute with pools:')
                        print(f'     Buy: {buy_pool["address"]}')
                        print(f'     Sell: {sell_pool["address"]}')

# Also check the original profitable pair
print('\n' + '='*50)
print('Original Cross-USDC Opportunity:')
print('  Buy: 0x380615f37993b5a96adf3d443b6e0ac50a211998 (Unknown factory - likely Dystopia)')
print('  Sell: 0x6d9e8dbb2779853db00418d4dcf96f3987cfc9d2 (QuickSwap V2)')
print('  Issue: Buy pool uses different AMM interface')
print('  Solution: Need Dystopia-compatible contract or find alternative buy pool')
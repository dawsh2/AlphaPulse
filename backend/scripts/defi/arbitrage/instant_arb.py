#!/usr/bin/env python3
"""Instant arbitrage executor - monitors and executes in real-time"""

import subprocess
import re
from web3 import Web3
import json
import os
from eth_account import Account
import time
from concurrent.futures import ThreadPoolExecutor
import threading

# Configuration
MIN_PROFIT = 0.01  # $0.01 minimum
MAX_SPREAD = 5.0   # Max 5% spread (above this is usually fake)
SCAN_INTERVAL = 1  # Scan every second

w3 = Web3(Web3.HTTPProvider('https://polygon.publicnode.com'))

USDC = '0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174'
WPOL = '0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270'

class InstantArbitrage:
    def __init__(self):
        self.executing = set()  # Track executing trades to avoid duplicates
        self.account = None
        self.last_scan = 0
        
        # Check for private key
        private_key = os.getenv('PRIVATE_KEY')
        if private_key:
            self.account = Account.from_key(private_key)
            print(f"💰 Wallet: {self.account.address}")
            self.check_balance()
        else:
            print("⚠️  No PRIVATE_KEY set - running in monitor mode only")
            
    def check_balance(self):
        """Check USDC balance"""
        if not self.account:
            return 0
            
        abi = json.loads('[{"inputs":[{"name":"account","type":"address"}],"name":"balanceOf","outputs":[{"name":"","type":"uint256"}],"type":"function"}]')
        usdc = w3.eth.contract(address=Web3.to_checksum_address(USDC), abi=abi)
        balance = usdc.functions.balanceOf(self.account.address).call() / 10**6
        print(f"💵 USDC Balance: ${balance:.2f}")
        return balance
        
    def scan_opportunities(self):
        """Run scanner and parse output"""
        try:
            result = subprocess.run(['./arb'], capture_output=True, text=True, timeout=10)
            output = result.stdout
            
            # Parse opportunities
            opportunities = []
            lines = output.split('\n')
            
            for i, line in enumerate(lines):
                if 'Buy:' in line and i+2 < len(lines):
                    buy_match = re.search(r'0x[a-fA-F0-9]{40}', line)
                    sell_match = re.search(r'0x[a-fA-F0-9]{40}', lines[i+1]) if 'Sell:' in lines[i+1] else None
                    profit_match = re.search(r'\$([0-9.]+)', lines[i+2]) if 'Net Profit:' in lines[i+2] else None
                    spread_match = re.search(r'([0-9.]+)%', lines[i+2]) if 'spread:' in lines[i+2] else None
                    size_match = re.search(r'size: \$([0-9.]+)', lines[i+2]) if 'size:' in lines[i+2] else None
                    
                    if buy_match and sell_match and profit_match:
                        opp = {
                            'buy': buy_match.group(),
                            'sell': sell_match.group(),
                            'profit': float(profit_match.group(1)),
                            'spread': float(spread_match.group(1)) if spread_match else 0,
                            'size': float(size_match.group(1)) if size_match else 10
                        }
                        
                        # Filter out suspicious opportunities
                        if opp['profit'] >= MIN_PROFIT and opp['spread'] <= MAX_SPREAD:
                            opportunities.append(opp)
                            
            return opportunities
            
        except subprocess.TimeoutExpired:
            print("⏱️  Scanner timeout")
            return []
        except Exception as e:
            print(f"❌ Scan error: {e}")
            return []
            
    def execute_trade(self, opp):
        """Execute a single arbitrage trade"""
        key = f"{opp['buy']}-{opp['sell']}"
        
        # Skip if already executing
        if key in self.executing:
            return
            
        self.executing.add(key)
        
        try:
            print(f"\n🎯 Executing: ${opp['profit']:.4f} profit")
            print(f"   Buy:  {opp['buy'][:10]}...")
            print(f"   Sell: {opp['sell'][:10]}...")
            print(f"   Size: ${opp['size']:.2f}")
            
            if not self.account:
                print("   ⚠️  No wallet - simulation only")
                return
                
            # Quick execution using existing quick_arb script
            result = subprocess.run([
                './quick_arb.py',
                opp['buy'],
                opp['sell'],
                str(min(opp['size'], 100))  # Cap at $100
            ], capture_output=True, text=True, timeout=30)
            
            if 'PROFIT:' in result.stdout:
                profit_match = re.search(r'PROFIT: \$([0-9.]+)', result.stdout)
                if profit_match:
                    actual_profit = float(profit_match.group(1))
                    print(f"   ✅ SUCCESS! Profit: ${actual_profit:.4f}")
            elif 'failed' in result.stdout.lower():
                print(f"   ❌ Failed: Transaction reverted")
            else:
                print(f"   ⚠️  Unknown result")
                
        except subprocess.TimeoutExpired:
            print(f"   ⏱️  Execution timeout")
        except Exception as e:
            print(f"   ❌ Error: {e}")
        finally:
            self.executing.discard(key)
            
    def run_continuous(self):
        """Main loop - scan and execute continuously"""
        print("🚀 Instant Arbitrage Bot Starting...")
        print(f"   Min Profit: ${MIN_PROFIT}")
        print(f"   Max Spread: {MAX_SPREAD}%")
        print(f"   Scan Interval: {SCAN_INTERVAL}s")
        print("\n📡 Monitoring for opportunities...\n")
        
        executor = ThreadPoolExecutor(max_workers=5)
        
        while True:
            try:
                # Scan for opportunities
                opportunities = self.scan_opportunities()
                
                if opportunities:
                    print(f"\n💡 Found {len(opportunities)} opportunities!")
                    
                    # Sort by profit (highest first)
                    opportunities.sort(key=lambda x: x['profit'], reverse=True)
                    
                    # Execute top opportunities in parallel
                    for opp in opportunities[:3]:  # Execute top 3
                        executor.submit(self.execute_trade, opp)
                        
                    # Brief pause to avoid overwhelming
                    time.sleep(0.5)
                else:
                    print(".", end="", flush=True)
                    
                time.sleep(SCAN_INTERVAL)
                
            except KeyboardInterrupt:
                print("\n\n👋 Shutting down...")
                executor.shutdown(wait=False)
                break
            except Exception as e:
                print(f"\n❌ Error in main loop: {e}")
                time.sleep(5)
                
if __name__ == "__main__":
    bot = InstantArbitrage()
    bot.run_continuous()
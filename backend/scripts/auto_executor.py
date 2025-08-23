#!/usr/bin/env python3
"""Auto-executor: Monitors and executes profitable arbitrage automatically"""

import asyncio
import json
import os
from web3 import Web3
from eth_account import Account
import websockets
from datetime import datetime

# Configuration
MIN_PROFIT_USD = 0.50  # Minimum profit to execute
MAX_GAS_PRICE = 100  # Max gas price in gwei
CHECK_INTERVAL = 0.5  # Check every 500ms

w3 = Web3(Web3.HTTPProvider('https://polygon.publicnode.com'))

# Load contract ABI (would be from deployment)
FLASH_ARB_ABI = json.loads('''[
    {
        "inputs": [{
            "components": [
                {"name": "buyPool", "type": "address"},
                {"name": "sellPool", "type": "address"},
                {"name": "buyRouter", "type": "uint8"},
                {"name": "sellRouter", "type": "uint8"},
                {"name": "buyFee", "type": "uint24"},
                {"name": "sellFee", "type": "uint24"},
                {"name": "amount", "type": "uint256"},
                {"name": "minProfit", "type": "uint256"}
            ],
            "name": "params",
            "type": "tuple"
        }],
        "name": "executeArbitrage",
        "outputs": [],
        "type": "function"
    }
]''')

class ArbitrageBot:
    def __init__(self):
        self.private_key = os.getenv('PRIVATE_KEY')
        if not self.private_key:
            raise Exception("Set PRIVATE_KEY environment variable")
        
        self.account = Account.from_key(self.private_key)
        self.contract_address = None  # Set after deployment
        self.executing = False
        self.opportunities = {}
        
    def log(self, msg, level="INFO"):
        timestamp = datetime.now().strftime("%H:%M:%S.%f")[:-3]
        print(f"[{timestamp}] [{level}] {msg}")
        
    async def monitor_websocket(self):
        """Monitor DEX events via WebSocket for instant detection"""
        ws_url = "wss://polygon.publicnode.com"
        
        # Subscribe to Sync events from major DEXs
        subscription = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_subscribe",
            "params": ["logs", {
                "topics": [
                    "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1"  # Sync event
                ]
            }]
        }
        
        async with websockets.connect(ws_url) as ws:
            await ws.send(json.dumps(subscription))
            self.log("📡 Connected to WebSocket, monitoring swaps...")
            
            while True:
                try:
                    message = await ws.recv()
                    data = json.loads(message)
                    
                    if 'params' in data:
                        # New swap detected
                        await self.check_arbitrage_opportunity(data['params']['result'])
                        
                except Exception as e:
                    self.log(f"WebSocket error: {e}", "ERROR")
                    await asyncio.sleep(1)
                    
    async def check_arbitrage_opportunity(self, event):
        """Check if event creates arbitrage opportunity"""
        pool_address = event['address']
        
        # Quick profitability check (simplified)
        # In production, this would:
        # 1. Identify the tokens and amounts
        # 2. Check price against other DEXs
        # 3. Calculate profit after gas
        
        # For demo, simulate finding opportunity
        if pool_address not in self.opportunities:
            self.opportunities[pool_address] = {
                'count': 0,
                'last_check': datetime.now()
            }
        
        self.opportunities[pool_address]['count'] += 1
        
        # Simulate profitable opportunity every 100 events
        if self.opportunities[pool_address]['count'] % 100 == 0:
            await self.execute_opportunity({
                'buy_pool': pool_address,
                'sell_pool': '0xa374094527e1673a86de625aa59517c5de346d32',
                'profit': 1.5,
                'amount': 100
            })
            
    async def execute_opportunity(self, opp):
        """Execute profitable arbitrage atomically"""
        if self.executing:
            return  # Already executing another trade
            
        self.executing = True
        
        try:
            profit = opp['profit']
            
            if profit < MIN_PROFIT_USD:
                return
                
            gas_price = w3.eth.gas_price / 10**9
            if gas_price > MAX_GAS_PRICE:
                self.log(f"⛽ Gas too high: {gas_price} gwei", "WARN")
                return
                
            self.log(f"💰 PROFIT FOUND: ${profit:.2f}", "SUCCESS")
            self.log(f"   Buy:  {opp['buy_pool'][:10]}...")
            self.log(f"   Sell: {opp['sell_pool'][:10]}...")
            self.log(f"   Size: ${opp['amount']}")
            
            # Execute via flash loan contract
            if self.contract_address:
                contract = w3.eth.contract(
                    address=Web3.to_checksum_address(self.contract_address),
                    abi=FLASH_ARB_ABI
                )
                
                # Build transaction
                params = {
                    'buyPool': opp['buy_pool'],
                    'sellPool': opp['sell_pool'],
                    'buyRouter': 1,  # UniV3
                    'sellRouter': 1,  # UniV3
                    'buyFee': 500,
                    'sellFee': 500,
                    'amount': int(opp['amount'] * 10**6),
                    'minProfit': int(MIN_PROFIT_USD * 10**6)
                }
                
                tx = contract.functions.executeArbitrage(params).build_transaction({
                    'from': self.account.address,
                    'nonce': w3.eth.get_transaction_count(self.account.address),
                    'gasPrice': int(gas_price * 1.5 * 10**9),  # 50% higher for priority
                    'gas': 500000,
                    'chainId': 137
                })
                
                # Sign and send
                signed = self.account.sign_transaction(tx)
                tx_hash = w3.eth.send_raw_transaction(signed.rawTransaction)
                
                self.log(f"📤 Executed: {tx_hash.hex()}", "SUCCESS")
                
                # Don't wait for receipt (too slow)
                asyncio.create_task(self.check_result(tx_hash))
            else:
                self.log("⚠️ Contract not deployed, simulating...", "WARN")
                
        except Exception as e:
            self.log(f"Execution error: {e}", "ERROR")
        finally:
            self.executing = False
            
    async def check_result(self, tx_hash):
        """Check transaction result asynchronously"""
        try:
            receipt = await asyncio.get_event_loop().run_in_executor(
                None, w3.eth.wait_for_transaction_receipt, tx_hash
            )
            
            if receipt.status == 1:
                # Calculate actual profit from events
                self.log(f"✅ TX confirmed: {tx_hash.hex()}", "SUCCESS")
            else:
                self.log(f"❌ TX failed: {tx_hash.hex()}", "ERROR")
        except Exception as e:
            self.log(f"Result check error: {e}", "ERROR")
            
    async def scanner_monitor(self):
        """Monitor output from the ./arb scanner"""
        while True:
            try:
                # Run scanner and parse output
                proc = await asyncio.create_subprocess_exec(
                    './arb',
                    stdout=asyncio.subprocess.PIPE,
                    stderr=asyncio.subprocess.PIPE
                )
                
                stdout, _ = await proc.communicate()
                output = stdout.decode()
                
                # Parse for profitable trades
                if "Net Profit:" in output:
                    lines = output.split('\n')
                    for i, line in enumerate(lines):
                        if "Net Profit:" in line and "$" in line:
                            # Extract profit
                            profit_str = line.split('$')[1].split()[0]
                            profit = float(profit_str)
                            
                            if profit >= MIN_PROFIT_USD:
                                # Extract pool addresses
                                buy_line = lines[i-2] if i >= 2 else ""
                                sell_line = lines[i-1] if i >= 1 else ""
                                
                                if "Buy:" in buy_line and "Sell:" in sell_line:
                                    buy_pool = buy_line.split()[1]
                                    sell_pool = sell_line.split()[1]
                                    
                                    await self.execute_opportunity({
                                        'buy_pool': buy_pool,
                                        'sell_pool': sell_pool,
                                        'profit': profit,
                                        'amount': 100  # Would extract from output
                                    })
                                    
            except Exception as e:
                self.log(f"Scanner error: {e}", "ERROR")
                
            await asyncio.sleep(CHECK_INTERVAL)
            
    async def run(self):
        """Main bot loop"""
        self.log("🤖 Arbitrage Auto-Executor Starting...", "INFO")
        self.log(f"👛 Wallet: {self.account.address}", "INFO")
        self.log(f"💵 Min Profit: ${MIN_PROFIT_USD}", "INFO")
        self.log(f"⛽ Max Gas: {MAX_GAS_PRICE} gwei", "INFO")
        
        # Check if contract is deployed
        if os.path.exists('flash_arb_deployment.json'):
            with open('flash_arb_deployment.json', 'r') as f:
                deployment = json.load(f)
                self.contract_address = deployment['address']
                self.log(f"📄 Contract: {self.contract_address}", "INFO")
        else:
            self.log("⚠️ No flash loan contract deployed", "WARN")
            self.log("   Deploy FlashArbitrage.sol first for gas-free execution", "WARN")
        
        # Run monitoring tasks concurrently
        tasks = [
            asyncio.create_task(self.scanner_monitor()),
            # asyncio.create_task(self.monitor_websocket()),  # Uncomment for WebSocket
        ]
        
        self.log("🚀 Monitoring for opportunities...", "SUCCESS")
        
        try:
            await asyncio.gather(*tasks)
        except KeyboardInterrupt:
            self.log("Shutting down...", "INFO")
            
if __name__ == "__main__":
    bot = ArbitrageBot()
    asyncio.run(bot.run())
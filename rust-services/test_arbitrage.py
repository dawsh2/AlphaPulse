#!/usr/bin/env python3
"""
Cross-Exchange Arbitrage Detection Test

This script demonstrates the ultra-low latency arbitrage detection capabilities
of the multi-exchange delta streaming system.
"""

import asyncio
import json
import websocket
import time
from typing import Dict, Optional

class ArbitrageDetector:
    def __init__(self):
        self.orderbooks: Dict[str, Dict] = {}  # exchange -> symbol -> orderbook
        self.last_arbitrage_time = {}
        self.arbitrage_count = 0
        
    def update_orderbook(self, delta):
        """Update orderbook from delta and check for arbitrage opportunities"""
        exchange = delta['exchange']
        symbol = delta['symbol']
        
        # Initialize if needed
        if exchange not in self.orderbooks:
            self.orderbooks[exchange] = {}
        if symbol not in self.orderbooks[exchange]:
            self.orderbooks[exchange][symbol] = {'bids': {}, 'asks': {}}
            
        orderbook = self.orderbooks[exchange][symbol]
        
        # Apply delta changes
        for change in delta['changes']:
            price = change['price']
            volume = change['volume']
            side = change['side']
            action = change['action']
            
            if action == 'remove' or volume == 0:
                if side == 'bid' and price in orderbook['bids']:
                    del orderbook['bids'][price]
                elif side == 'ask' and price in orderbook['asks']:
                    del orderbook['asks'][price]
            else:
                if side == 'bid':
                    orderbook['bids'][price] = volume
                else:
                    orderbook['asks'][price] = volume
        
        # Check for arbitrage after update
        self.check_arbitrage(symbol)
    
    def check_arbitrage(self, symbol: str):
        """Check for arbitrage opportunities across exchanges for a symbol"""
        exchanges_with_data = []
        
        for exchange in self.orderbooks:
            if symbol in self.orderbooks[exchange]:
                orderbook = self.orderbooks[exchange][symbol]
                if orderbook['bids'] and orderbook['asks']:
                    best_bid = max(orderbook['bids'].keys())
                    best_ask = min(orderbook['asks'].keys())
                    exchanges_with_data.append({
                        'exchange': exchange,
                        'best_bid': best_bid,
                        'best_ask': best_ask,
                        'bid_volume': orderbook['bids'][best_bid],
                        'ask_volume': orderbook['asks'][best_ask]
                    })
        
        if len(exchanges_with_data) >= 2:
            # Find arbitrage opportunities
            for i, ex1 in enumerate(exchanges_with_data):
                for ex2 in exchanges_with_data[i+1:]:
                    # Check if we can buy on ex1 and sell on ex2
                    if ex1['best_ask'] < ex2['best_bid']:
                        spread = ex2['best_bid'] - ex1['best_ask']
                        spread_pct = (spread / ex1['best_ask']) * 100
                        
                        if spread_pct > 0.01:  # Minimum 0.01% spread
                            self.report_arbitrage(symbol, ex1, ex2, spread, spread_pct, 'buy_sell')
                    
                    # Check if we can buy on ex2 and sell on ex1  
                    if ex2['best_ask'] < ex1['best_bid']:
                        spread = ex1['best_bid'] - ex2['best_ask']
                        spread_pct = (spread / ex2['best_ask']) * 100
                        
                        if spread_pct > 0.01:  # Minimum 0.01% spread
                            self.report_arbitrage(symbol, ex2, ex1, spread, spread_pct, 'buy_sell')
    
    def report_arbitrage(self, symbol: str, buy_ex: dict, sell_ex: dict, spread: float, spread_pct: float, direction: str):
        """Report arbitrage opportunity"""
        now = time.time()
        key = f"{symbol}_{buy_ex['exchange']}_{sell_ex['exchange']}"
        
        # Rate limit: only report same opportunity every 5 seconds
        if key in self.last_arbitrage_time and now - self.last_arbitrage_time[key] < 5:
            return
            
        self.last_arbitrage_time[key] = now
        self.arbitrage_count += 1
        
        max_volume = min(buy_ex['ask_volume'], sell_ex['bid_volume'])
        potential_profit = spread * max_volume
        
        print(f"""
🚨 ARBITRAGE OPPORTUNITY #{self.arbitrage_count} 🚨
Symbol: {symbol}
Direction: Buy {buy_ex['exchange']} @ ${buy_ex['best_ask']:.2f} → Sell {sell_ex['exchange']} @ ${sell_ex['best_bid']:.2f}
Spread: ${spread:.2f} ({spread_pct:.3f}%)
Max Volume: {max_volume:.6f}
Potential Profit: ${potential_profit:.2f}
Timestamp: {time.strftime('%H:%M:%S.%f')[:-3]}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        """)

def on_message(ws, message):
    """Handle WebSocket messages from the delta stream"""
    try:
        data = json.loads(message)
        
        if data.get('type') == 'delta':
            delta = data.get('data')
            if delta:
                # Update arbitrage detector with new delta
                detector.update_orderbook(delta)
                
                # Print delta info
                print(f"📊 {delta['exchange'].upper()} {delta['symbol']}: "
                      f"{len(delta['changes'])} changes "
                      f"(v{delta['version']})")
                      
    except json.JSONDecodeError as e:
        print(f"❌ JSON decode error: {e}")
    except Exception as e:
        print(f"❌ Error processing message: {e}")

def on_error(ws, error):
    print(f"❌ WebSocket error: {error}")

def on_close(ws, close_status_code, close_msg):
    print("🔌 WebSocket connection closed")

def on_open(ws):
    print("🔗 Connected to WebSocket delta stream")
    print("🔍 Monitoring for cross-exchange arbitrage opportunities...")
    print("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")

def main():
    global detector
    detector = ArbitrageDetector()
    
    print("""
🏛️  AlphaPulse Multi-Exchange Arbitrage Detection Test
🚀 Ultra-Low Latency Cross-Exchange Monitoring

This test demonstrates real-time arbitrage detection across:
- Coinbase Pro
- Kraken  
- Binance.US

The system uses shared memory delta compression for sub-10μs latency.
    """)
    
    # Connect to the WebSocket delta stream
    websocket.enableTrace(False)
    ws = websocket.WebSocketApp(
        "ws://localhost:8765/ws",
        on_open=on_open,
        on_message=on_message,
        on_error=on_error,
        on_close=on_close
    )
    
    try:
        ws.run_forever()
    except KeyboardInterrupt:
        print(f"\n🛑 Stopping arbitrage detector... Found {detector.arbitrage_count} opportunities!")
        ws.close()

if __name__ == "__main__":
    main()
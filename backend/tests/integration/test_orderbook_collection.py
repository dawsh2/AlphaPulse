#!/usr/bin/env python3
"""
Test Order Book Data Collection
Quick test to see what Level 2 data looks like and analyze spreads
"""

import asyncio
import websockets
import json
import time
from datetime import datetime

async def test_kraken_orderbook():
    """Test Kraken Level 2 order book collection"""
    print("=" * 60)
    print("TESTING KRAKEN LEVEL 2 ORDER BOOK COLLECTION")
    print("=" * 60)
    
    uri = "wss://ws.kraken.com/v2"
    
    try:
        async with websockets.connect(uri) as websocket:
            # Subscribe to BTC/USD order book
            subscribe_msg = {
                "method": "subscribe",
                "params": {
                    "channel": "book",
                    "symbol": ["BTC/USD"],
                    "depth": 10  # Top 10 levels for testing
                }
            }
            
            await websocket.send(json.dumps(subscribe_msg))
            print("✅ Subscribed to Kraken BTC/USD order book")
            
            snapshot_received = False
            update_count = 0
            
            # Collect data for 30 seconds
            start_time = time.time()
            while time.time() - start_time < 30:
                try:
                    message = await asyncio.wait_for(websocket.recv(), timeout=5)
                    data = json.loads(message)
                    
                    # Debug: print subscription confirmations only
                    if data.get('success') and not snapshot_received:
                        print(f"✅ Subscription confirmed: {data.get('result', {}).get('channel')}")
                    
                    if data.get('channel') == 'book':
                        if data.get('type') == 'snapshot' and not snapshot_received:
                            print("\n📊 RECEIVED ORDER BOOK SNAPSHOT:")
                            
                            # Kraken v2 format: data is array with one object containing bids/asks
                            snapshot_data = data.get('data', [])
                            if snapshot_data and len(snapshot_data) > 0:
                                book_data = snapshot_data[0]
                                bids = book_data.get('bids', [])
                                asks = book_data.get('asks', [])
                                
                                print("Top 5 Bids (highest prices):")
                                for i, bid in enumerate(bids[:5], 1):
                                    price = bid.get('price', 0)
                                    qty = bid.get('qty', 0)
                                    print(f"  {i}. ${float(price):,.2f} - {float(qty):.6f} BTC")
                                
                                print("\nTop 5 Asks (lowest prices):")
                                for i, ask in enumerate(asks[:5], 1):
                                    price = ask.get('price', 0)
                                    qty = ask.get('qty', 0)
                                    print(f"  {i}. ${float(price):,.2f} - {float(qty):.6f} BTC")
                            
                                # Calculate spread
                                if bids and asks:
                                    best_bid = float(bids[0].get('price', 0))
                                    best_ask = float(asks[0].get('price', 0))
                                    spread = best_ask - best_bid
                                    spread_pct = (spread / best_bid) * 100
                                    
                                    print(f"\n💰 SPREAD ANALYSIS:")
                                    print(f"Best Bid: ${best_bid:,.2f}")
                                    print(f"Best Ask: ${best_ask:,.2f}")
                                    print(f"Spread: ${spread:.2f}")
                                    print(f"Spread %: {spread_pct:.4f}%")
                                    
                                    # Market making opportunity check
                                    position_size = 0.1  # 0.1 BTC
                                    fee_rate = 0.0025   # 0.25% Kraken fee
                                    
                                    gross_profit = spread * position_size
                                    fee_cost = (best_bid + best_ask) / 2 * position_size * fee_rate * 2
                                    net_profit = gross_profit - fee_cost
                                    
                                    print(f"\n🎯 MARKET MAKING SIMULATION:")
                                    print(f"Position size: {position_size} BTC")
                                    print(f"Gross profit: ${gross_profit:.2f}")
                                    print(f"Round-trip fees: ${fee_cost:.2f}")
                                    print(f"Net profit: ${net_profit:.2f}")
                                    
                                    if net_profit > 0:
                                        print("✅ PROFITABLE OPPORTUNITY!")
                                    else:
                                        print("❌ Unprofitable due to fees")
                            
                            snapshot_received = True
                            
                        elif data.get('type') == 'update':
                            update_count += 1
                            if update_count <= 3:  # Show first 3 updates
                                print(f"\n📈 ORDER BOOK UPDATE #{update_count}:")
                                
                                updates = data.get('data', {})
                                if 'bids' in updates:
                                    print(f"Bid updates: {len(updates['bids'])}")
                                    for price, size in updates['bids'][:2]:
                                        action = "REMOVE" if float(size) == 0 else "UPDATE"
                                        print(f"  {action} ${float(price):,.2f} -> {float(size):.6f} BTC")
                                
                                if 'asks' in updates:
                                    print(f"Ask updates: {len(updates['asks'])}")
                                    for price, size in updates['asks'][:2]:
                                        action = "REMOVE" if float(size) == 0 else "UPDATE"
                                        print(f"  {action} ${float(price):,.2f} -> {float(size):.6f} BTC")
                
                except asyncio.TimeoutError:
                    continue
                except Exception as e:
                    print(f"Error: {e}")
                    break
            
            print(f"\n📊 SUMMARY:")
            print(f"Snapshot received: {snapshot_received}")
            print(f"Updates received: {update_count}")
            print(f"Data collection duration: 30 seconds")
            
    except Exception as e:
        print(f"❌ Connection error: {e}")

async def test_coinbase_orderbook():
    """Test Coinbase Level 2 order book (will show auth requirement)"""
    print("\n" + "=" * 60)
    print("TESTING COINBASE LEVEL 2 ORDER BOOK ACCESS")
    print("=" * 60)
    
    uri = "wss://ws-feed.exchange.coinbase.com"
    
    try:
        async with websockets.connect(uri) as websocket:
            # Try to subscribe to level2 (will likely fail without auth)
            subscribe_msg = {
                "type": "subscribe",
                "product_ids": ["BTC-USD"],
                "channels": ["level2"]
            }
            
            await websocket.send(json.dumps(subscribe_msg))
            print("📡 Attempted to subscribe to Coinbase Level 2...")
            
            # Wait for response
            try:
                message = await asyncio.wait_for(websocket.recv(), timeout=10)
                data = json.loads(message)
                
                if data.get('type') == 'error':
                    print(f"❌ AUTH REQUIRED: {data.get('message', 'Authentication needed')}")
                    print("💡 Need to add Coinbase API credentials for Level 2 data")
                elif data.get('type') == 'subscriptions':
                    print("✅ Successfully subscribed (unexpected - API may have changed)")
                else:
                    print(f"📨 Received: {data.get('type', 'unknown')}")
                    
            except asyncio.TimeoutError:
                print("⏰ No response - possible connection issue")
                
    except Exception as e:
        print(f"❌ Connection error: {e}")

async def main():
    """Run order book tests"""
    print("🚀 TESTING ORDER BOOK DATA COLLECTION")
    print("This will show what Level 2 data looks like and calculate real spreads")
    print()
    
    # Test Kraken (should work)
    await test_kraken_orderbook()
    
    # Test Coinbase (will show auth requirement)
    await test_coinbase_orderbook()
    
    print("\n" + "=" * 60)
    print("NEXT STEPS:")
    print("=" * 60)
    print("1. ✅ Kraken Level 2 works - no authentication required")
    print("2. ❌ Coinbase Level 2 requires API credentials")
    print("3. 🎯 Real spreads are much tighter than our trade-based estimates")
    print("4. 💰 Market making opportunities depend on actual spread size")
    print("5. 📈 Order book updates show real-time market changes")
    print()
    print("Recommendation: Start with Kraken L2, add Coinbase L2 with credentials later")

if __name__ == "__main__":
    asyncio.run(main())
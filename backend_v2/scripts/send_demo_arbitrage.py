#!/usr/bin/env python3
"""Send demo arbitrage opportunities to the dashboard WebSocket."""

import asyncio
import json
import random
import time
import websockets

async def send_demo_data():
    """Send demo arbitrage opportunities to the dashboard."""
    uri = "ws://localhost:8080/ws"
    
    async with websockets.connect(uri) as websocket:
        print("✅ Connected to dashboard WebSocket")
        
        # Subscribe to channels
        subscribe_msg = {
            "msg_type": "subscribe",
            "channels": ["trades", "orderbook", "l2_updates", "status_updates"],
            "symbols": []
        }
        await websocket.send(json.dumps(subscribe_msg))
        print("📡 Subscribed to channels")
        
        pairs = ["WMATIC/USDC", "WETH/USDC", "WBTC/USDC", "LINK/USDC"]
        exchanges = ["QuickSwap", "SushiSwap", "Uniswap V3"]
        
        while True:
            # Generate demo arbitrage opportunity
            pair = random.choice(pairs)
            buy_exchange = random.choice(exchanges)
            sell_exchange = random.choice([e for e in exchanges if e != buy_exchange])
            
            base_price = random.uniform(0.5, 2.0) if "WMATIC" in pair else random.uniform(1000, 50000)
            spread_percent = random.uniform(0.1, 3.0)
            trade_size = random.uniform(1000, 10000)
            
            buy_price = base_price
            sell_price = base_price * (1 + spread_percent / 100)
            gross_profit = trade_size * (spread_percent / 100)
            
            gas_fee = random.uniform(5, 20)
            dex_fees = trade_size * 0.003  # 0.3% fee
            slippage = trade_size * random.uniform(0.001, 0.005)
            
            net_profit = gross_profit - gas_fee - dex_fees - slippage
            net_profit_percent = (net_profit / trade_size) * 100
            
            opportunity = {
                "msg_type": "arbitrage_opportunity",
                "id": f"demo-{time.time()}",
                "timestamp": int(time.time() * 1000),
                "pair": pair,
                "token_a": pair.split("/")[0],
                "token_b": pair.split("/")[1],
                "dex_buy": buy_exchange,
                "dex_sell": sell_exchange,
                "dex_buy_router": f"0x{random.randbytes(20).hex()}",
                "dex_sell_router": f"0x{random.randbytes(20).hex()}",
                "price_buy": buy_price,
                "price_sell": sell_price,
                "max_trade_size": trade_size,
                "estimated_profit": gross_profit,
                "profit_percent": spread_percent,
                "gas_fee_usd": gas_fee,
                "dex_fees_usd": dex_fees,
                "slippage_cost_usd": slippage,
                "net_profit_usd": net_profit,
                "net_profit_percent": net_profit_percent,
                "executable": net_profit > 10,
                "confidence_score": random.uniform(0.7, 0.95),
                "detected_at": int(time.time() * 1000)
            }
            
            await websocket.send(json.dumps(opportunity))
            print(f"📊 Sent arbitrage opportunity: {pair} ${net_profit:.2f} profit ({net_profit_percent:.2f}%)")
            
            # Also send some pool swaps
            for _ in range(random.randint(1, 3)):
                swap = {
                    "type": "pool_swap",
                    "pool_address": f"0x{random.randbytes(20).hex()}",
                    "token_in": random.choice(["WMATIC", "WETH", "USDC", "WBTC"]),
                    "token_out": random.choice(["USDC", "USDT", "DAI", "WETH"]),
                    "amount_in": {
                        "normalized": random.uniform(100, 10000),
                        "decimals": 18
                    },
                    "amount_out": {
                        "normalized": random.uniform(100, 10000),
                        "decimals": 6 if "USD" in pair else 18
                    },
                    "timestamp": int(time.time() * 1000)
                }
                await websocket.send(json.dumps(swap))
            
            # Wait before sending next opportunity
            await asyncio.sleep(random.uniform(2, 5))

if __name__ == "__main__":
    print("🚀 Starting demo arbitrage data generator")
    print("📡 Connecting to dashboard WebSocket on ws://localhost:8080/ws")
    
    try:
        asyncio.run(send_demo_data())
    except KeyboardInterrupt:
        print("\n👋 Stopping demo generator")
    except Exception as e:
        print(f"❌ Error: {e}")
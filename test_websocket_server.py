#!/usr/bin/env python3
"""
Test WebSocket server for dashboard validation
Provides sample arbitrage data with native precision
"""

import asyncio
import json
import random
import time
import websockets
import logging

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

# Sample token data with native precision
TOKENS = {
    0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270: {"symbol": "WMATIC", "decimals": 18},
    0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174: {"symbol": "USDC", "decimals": 6},
    0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619: {"symbol": "WETH", "decimals": 18},
    0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063: {"symbol": "DAI", "decimals": 18},
    0xc2132D05D31c914a87C6611C10748AEb04B58e8F: {"symbol": "USDT", "decimals": 6},
}

connected_clients = set()

def generate_pool_swap():
    """Generate a realistic pool swap with native precision"""
    token_ids = list(TOKENS.keys())
    token_in_id = random.choice(token_ids)
    token_out_id = random.choice([t for t in token_ids if t != token_in_id])
    
    token_in = TOKENS[token_in_id]
    token_out = TOKENS[token_out_id]
    
    # Generate realistic amounts based on token decimals
    if token_in["decimals"] == 18:
        amount_in_raw = random.randint(1000000000000000000, 10000000000000000000)  # 1-10 tokens
    else:  # 6 decimals
        amount_in_raw = random.randint(1000000, 10000000)  # 1-10 tokens
    
    if token_out["decimals"] == 18:
        amount_out_raw = random.randint(500000000000000000, 5000000000000000000)  # 0.5-5 tokens
    else:  # 6 decimals  
        amount_out_raw = random.randint(500000, 5000000)  # 0.5-5 tokens
    
    # Calculate normalized amounts for display
    amount_in_normalized = amount_in_raw / (10 ** token_in["decimals"])
    amount_out_normalized = amount_out_raw / (10 ** token_out["decimals"])
    
    return {
        "type": "pool_swap",
        "venue": 137,  # Polygon
        "venue_name": "Polygon",
        "pool_id": f"pool_0x{random.randint(0x1000, 0x9999):04x}",
        "token_in": f"0x{token_in_id:016x}",
        "token_out": f"0x{token_out_id:016x}",
        "token_in_symbol": token_in["symbol"],
        "token_out_symbol": token_out["symbol"],
        "amount_in": {
            "raw": amount_in_raw,
            "normalized": amount_in_normalized,
            "decimals": token_in["decimals"]
        },
        "amount_out": {
            "raw": amount_out_raw,
            "normalized": amount_out_normalized,
            "decimals": token_out["decimals"]
        },
        "sqrt_price_x96_after": random.randint(1000000, 9000000),
        "tick_after": random.randint(-887220, 887220),
        "liquidity_after": random.randint(1000000, 10000000),
        "timestamp": int(time.time() * 1000),
        "timestamp_iso": time.strftime("%Y-%m-%dT%H:%M:%S.%fZ"),
        "block_number": random.randint(50000000, 60000000)
    }

def generate_arbitrage_opportunity():
    """Generate a realistic arbitrage opportunity"""
    token_pairs = [
        ("WMATIC", "USDC"),
        ("WETH", "DAI"), 
        ("USDC", "USDT"),
        ("WMATIC", "WETH"),
        ("DAI", "USDC")
    ]
    
    token_in, token_out = random.choice(token_pairs)
    expected_profit = random.uniform(5.0, 150.0)
    spread_percentage = random.uniform(0.1, 2.5)
    required_capital = random.uniform(100.0, 5000.0)
    
    return {
        "id": f"arb_{int(time.time() * 1000)}_{random.randint(1000, 9999)}",
        "timestamp": int(time.time() * 1000),
        "pair": f"{token_in}/{token_out}",
        "token0Symbol": token_in,
        "token1Symbol": token_out,
        "buyPool": f"pool_0x{random.randint(0x1000, 0x9999):04x}",
        "sellPool": f"pool_0x{random.randint(0x1000, 0x9999):04x}",
        "buyExchange": random.choice(["Uniswap V2", "Uniswap V3", "QuickSwap", "SushiSwap"]),
        "sellExchange": random.choice(["Uniswap V2", "Uniswap V3", "QuickSwap", "SushiSwap"]),
        "buyPrice": random.uniform(0.5, 2000.0),
        "sellPrice": random.uniform(0.5, 2000.0),
        "tradeSize": required_capital,
        "grossProfit": expected_profit + random.uniform(10, 50),
        "profitPercent": spread_percentage,
        "gasFee": random.uniform(2.0, 15.0),
        "dexFees": random.uniform(1.0, 8.0),
        "slippageCost": random.uniform(0.5, 5.0),
        "totalFees": random.uniform(3.5, 28.0),
        "netProfit": expected_profit,
        "netProfitPercent": spread_percentage * 0.7,
        "executable": expected_profit > 10.0,
        "recommendation": "EXECUTE" if expected_profit > 25.0 else "MONITOR" if expected_profit > 10.0 else "SKIP",
        "confidence": random.uniform(0.7, 0.98)
    }

async def handle_client(websocket, path):
    """Handle a new WebSocket client connection"""
    connected_clients.add(websocket)
    client_address = websocket.remote_address
    logger.info(f"📱 Client connected from {client_address}")
    
    try:
        await websocket.wait_closed()
    except websockets.exceptions.ConnectionClosed:
        pass
    finally:
        connected_clients.discard(websocket)
        logger.info(f"📱 Client disconnected from {client_address}")

async def broadcast_data():
    """Continuously broadcast sample data to all connected clients"""
    while True:
        if connected_clients:
            # Generate pool swap data
            if random.random() < 0.7:  # 70% chance of pool swap
                swap_data = generate_pool_swap()
                message = json.dumps(swap_data)
                
                # Broadcast to all connected clients
                disconnected = set()
                for client in connected_clients.copy():
                    try:
                        await client.send(message)
                    except websockets.exceptions.ConnectionClosed:
                        disconnected.add(client)
                
                # Clean up disconnected clients
                for client in disconnected:
                    connected_clients.discard(client)
                
                logger.info(f"📊 Sent pool swap: {swap_data['token_in_symbol']} → {swap_data['token_out_symbol']} "
                           f"({swap_data['amount_in']['normalized']:.4f} → {swap_data['amount_out']['normalized']:.4f})")
            
            # Generate arbitrage opportunity  
            if random.random() < 0.3:  # 30% chance of arbitrage opportunity
                arb_data = generate_arbitrage_opportunity()
                message = json.dumps(arb_data)
                
                # Broadcast to all connected clients
                disconnected = set()
                for client in connected_clients.copy():
                    try:
                        await client.send(message)
                    except websockets.exceptions.ConnectionClosed:
                        disconnected.add(client)
                
                # Clean up disconnected clients
                for client in disconnected:
                    connected_clients.discard(client)
                
                logger.info(f"💰 Sent arbitrage opportunity: {arb_data['pair']} "
                           f"(${arb_data['netProfit']:.2f} profit, {arb_data['recommendation']})")
        
        # Wait before next broadcast
        await asyncio.sleep(random.uniform(1.0, 3.0))

async def main():
    """Start the WebSocket server"""
    logger.info("🚀 Starting test WebSocket server for dashboard validation")
    logger.info("   Provides sample Polygon DeFi data with native precision")
    logger.info("   Dashboard can connect at: ws://localhost:8765")
    
    # Start the WebSocket server
    server = await websockets.serve(handle_client, "localhost", 8765)
    logger.info("✅ WebSocket server listening on ws://localhost:8765")
    
    # Start broadcasting data
    broadcast_task = asyncio.create_task(broadcast_data())
    
    try:
        await server.wait_closed()
    except KeyboardInterrupt:
        logger.info("🛑 Shutting down WebSocket server")
        server.close()
        await server.wait_closed()
        broadcast_task.cancel()

if __name__ == "__main__":
    asyncio.run(main())
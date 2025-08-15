#!/usr/bin/env python3
"""
Monitor ALL Polygon DEX Swaps
==============================
Watch all swap events across major DEXes to see actual data
"""

import asyncio
import json
import websockets
from datetime import datetime

async def test_all_swaps():
    """Connect to Alchemy and monitor ALL swap events"""
    
    api_key = "YIN6CJks2-fLUDgen4hAs"
    ws_url = f"wss://polygon-mainnet.g.alchemy.com/v2/{api_key}"
    
    print(f"🔌 Connecting to Alchemy WebSocket...")
    print(f"   URL: {ws_url}")
    print()
    
    # Popular DEX pools on Polygon
    pools = [
        "0xa8c32a57c440e55022c7e3b0e39b5e0c00a3a05e",  # POL/USDC QuickSwap
        "0xcd353f79d9fade311fc3119b841e1f456b54e858",  # WETH/USDC QuickSwap
        "0x853ee4b2a13f8a742d64c8f088be7ba2131f670d",  # USDC/WETH QuickSwap
        "0x6e7a5fafcec6bb1e78bae2a1f0b612012bf14827",  # WMATIC/USDC QuickSwap
    ]
    
    # Subscribe to swap events from ALL these pools
    subscription = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_subscribe",
        "params": [
            "logs",
            {
                "address": pools[:2],  # Start with just 2 pools
                "topics": [
                    "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822"  # Swap event
                ]
            }
        ]
    }
    
    async with websockets.connect(ws_url) as ws:
        await ws.send(json.dumps(subscription))
        response = await ws.recv()
        print(f"📊 Subscription response: {response}")
        print()
        print("🎯 Monitoring swap events (capturing 10 events or 60 seconds)...")
        print("=" * 70)
        
        event_count = 0
        start_time = asyncio.get_event_loop().time()
        
        while event_count < 10 and (asyncio.get_event_loop().time() - start_time) < 60:
            try:
                message = await asyncio.wait_for(ws.recv(), timeout=5.0)
                data = json.loads(message)
                
                if "params" in data and "result" in data["params"]:
                    event_count += 1
                    result = data["params"]["result"]
                    
                    pool_addr = result.get("address", "unknown")
                    swap_data = result.get("data", "")
                    
                    # Identify pool
                    pool_name = "Unknown"
                    if "a8c32a57" in pool_addr:
                        pool_name = "POL/USDC"
                        token0, token1 = "USDC", "POL"
                        decimals0, decimals1 = 6, 18
                    elif "cd353f79" in pool_addr:
                        pool_name = "WETH/USDC"
                        token0, token1 = "WETH", "USDC"
                        decimals0, decimals1 = 18, 6
                    elif "853ee4b2" in pool_addr:
                        pool_name = "USDC/WETH"
                        token0, token1 = "USDC", "WETH"
                        decimals0, decimals1 = 6, 18
                    elif "6e7a5faf" in pool_addr:
                        pool_name = "WMATIC/USDC"
                        token0, token1 = "WMATIC", "USDC"
                        decimals0, decimals1 = 18, 6
                    
                    print(f"\n📦 EVENT #{event_count} - {pool_name} at {datetime.now().strftime('%H:%M:%S')}")
                    
                    # Parse swap data
                    if swap_data.startswith("0x"):
                        swap_data = swap_data[2:]
                    
                    if len(swap_data) >= 256:
                        amount0_in_raw = int(swap_data[0:64], 16)
                        amount1_in_raw = int(swap_data[64:128], 16)
                        amount0_out_raw = int(swap_data[128:192], 16)
                        amount1_out_raw = int(swap_data[192:256], 16)
                        
                        # Check for suspicious values
                        if amount0_in_raw > 10**30 or amount0_out_raw > 10**30:
                            print(f"   ⚠️ SUSPICIOUS AMOUNT0: in={amount0_in_raw}, out={amount0_out_raw}")
                        if amount1_in_raw > 10**30 or amount1_out_raw > 10**30:
                            print(f"   ⚠️ SUSPICIOUS AMOUNT1: in={amount1_in_raw}, out={amount1_out_raw}")
                        
                        # Apply decimals
                        amount0_in = amount0_in_raw / (10 ** decimals0)
                        amount1_in = amount1_in_raw / (10 ** decimals1)
                        amount0_out = amount0_out_raw / (10 ** decimals0)
                        amount1_out = amount1_out_raw / (10 ** decimals1)
                        
                        # Determine swap direction
                        if amount0_in > 0 and amount1_out > 0:
                            print(f"   💱 {amount0_in:.6f} {token0} → {amount1_out:.6f} {token1}")
                            if token0 == "USDC" and token1 == "POL":
                                price = amount0_in / amount1_out
                                print(f"   📈 POL price: ${price:.6f}")
                            elif token0 == "POL" and token1 == "USDC":
                                price = amount1_out / amount0_in
                                print(f"   📈 POL price: ${price:.6f}")
                        elif amount1_in > 0 and amount0_out > 0:
                            print(f"   💱 {amount1_in:.6f} {token1} → {amount0_out:.6f} {token0}")
                            if token1 == "POL" and token0 == "USDC":
                                price = amount0_out / amount1_in
                                print(f"   📈 POL price: ${price:.6f}")
                            elif token1 == "USDC" and token0 == "POL":
                                price = amount1_in / amount0_out
                                print(f"   📈 POL price: ${price:.6f}")
                        
                        # Check for unrealistic prices
                        if "POL" in [token0, token1]:
                            if amount0_in > 0 and amount1_out > 0:
                                ratio = amount0_in / amount1_out if amount1_out > 0 else 0
                            elif amount1_in > 0 and amount0_out > 0:
                                ratio = amount0_out / amount1_in if amount1_in > 0 else 0
                            else:
                                ratio = 0
                            
                            if ratio > 10 or ratio < 0.01:
                                print(f"   🚨 UNREALISTIC RATIO: {ratio:.9f}")
                                print(f"   🔍 Raw hex: {swap_data[:256]}")
                        
            except asyncio.TimeoutError:
                print(".", end="", flush=True)
                continue
        
        print(f"\n\n{'=' * 70}")
        print(f"📊 Captured {event_count} events in {asyncio.get_event_loop().time() - start_time:.1f} seconds")

if __name__ == "__main__":
    asyncio.run(test_all_swaps())
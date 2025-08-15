#!/usr/bin/env python3
"""
Test Raw WebSocket Data from Alchemy
=====================================
Connect directly to Alchemy and see what raw data we're getting
"""

import asyncio
import json
import websockets
from datetime import datetime

async def test_alchemy_websocket():
    """Connect to Alchemy and monitor raw swap events"""
    
    # The API key we found
    api_key = "YIN6CJks2-fLUDgen4hAs"
    ws_url = f"wss://polygon-mainnet.g.alchemy.com/v2/{api_key}"
    
    print(f"🔌 Connecting to Alchemy WebSocket...")
    print(f"   URL: {ws_url}")
    print()
    
    # POL/USDC pool on QuickSwap
    pol_usdc_pool = "0xa8c32a57c440e55022c7e3b0e39b5e0c00a3a05e"
    
    # Subscribe to swap events
    subscription = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_subscribe",
        "params": [
            "logs",
            {
                "address": pol_usdc_pool,
                "topics": [
                    "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822"  # Swap event signature
                ]
            }
        ]
    }
    
    events_captured = []
    
    async with websockets.connect(ws_url) as ws:
        # Send subscription
        await ws.send(json.dumps(subscription))
        response = await ws.recv()
        print(f"📊 Subscription response: {response}")
        print()
        print("🎯 Monitoring POL/USDC swaps (will capture 5 events)...")
        print("=" * 60)
        
        # Capture a few events
        event_count = 0
        while event_count < 5:
            try:
                message = await asyncio.wait_for(ws.recv(), timeout=30.0)
                data = json.loads(message)
                
                if "params" in data and "result" in data["params"]:
                    event_count += 1
                    result = data["params"]["result"]
                    
                    print(f"\n📦 RAW EVENT #{event_count} at {datetime.now().strftime('%H:%M:%S')}")
                    print(f"   Block: {result.get('blockNumber', 'unknown')}")
                    print(f"   Pool: {result.get('address', 'unknown')}")
                    
                    # Get the swap data
                    swap_data = result.get("data", "")
                    print(f"   Raw data: {swap_data[:80]}...")
                    
                    # Parse it
                    if swap_data.startswith("0x"):
                        swap_data = swap_data[2:]
                    
                    if len(swap_data) >= 256:
                        amount0_in = int(swap_data[0:64], 16)
                        amount1_in = int(swap_data[64:128], 16)
                        amount0_out = int(swap_data[128:192], 16)
                        amount1_out = int(swap_data[192:256], 16)
                        
                        print(f"\n   📊 PARSED AMOUNTS (raw):")
                        print(f"      USDC in:  {amount0_in:,}")
                        print(f"      POL in:   {amount1_in:,}")
                        print(f"      USDC out: {amount0_out:,}")
                        print(f"      POL out:  {amount1_out:,}")
                        
                        # Apply decimals
                        usdc_in = amount0_in / 1e6
                        pol_in = amount1_in / 1e18
                        usdc_out = amount0_out / 1e6
                        pol_out = amount1_out / 1e18
                        
                        print(f"\n   💰 DECIMAL ADJUSTED:")
                        print(f"      USDC in:  ${usdc_in:,.2f}")
                        print(f"      POL in:   {pol_in:,.6f}")
                        print(f"      USDC out: ${usdc_out:,.2f}")
                        print(f"      POL out:  {pol_out:,.6f}")
                        
                        # Calculate price
                        if pol_in > 0 and usdc_out > 0:
                            price = usdc_out / pol_in
                            print(f"\n   💱 SWAP: {pol_in:.6f} POL → ${usdc_out:.2f} USDC")
                            print(f"   📈 Price: ${price:.6f} per POL")
                        elif usdc_in > 0 and pol_out > 0:
                            price = usdc_in / pol_out
                            print(f"\n   💱 SWAP: ${usdc_in:.2f} USDC → {pol_out:.6f} POL")
                            print(f"   📈 Price: ${price:.6f} per POL")
                        
                        # Store for analysis
                        events_captured.append({
                            "raw_data": swap_data,
                            "usdc_in": amount0_in,
                            "pol_in": amount1_in,
                            "usdc_out": amount0_out,
                            "pol_out": amount1_out
                        })
                        
                        # Check for corruption
                        if amount0_in == 0 and amount1_in > 0 and amount1_in < 1000000:
                            # Very small POL amount
                            if amount0_out > 1e12:  # More than 1M USDC
                                print(f"\n   ⚠️ SUSPICIOUS: Tiny POL amount ({pol_in:.9f}) for huge USDC ({usdc_out:,.0f})")
                                print(f"   ⚠️ This looks like corrupted/malicious data!")
                        
            except asyncio.TimeoutError:
                print("\n⏱️ No events in 30 seconds, continuing...")
                break
    
    print("\n" + "=" * 60)
    print("📊 SUMMARY OF CAPTURED EVENTS:")
    for i, event in enumerate(events_captured, 1):
        pol_amount = max(event["pol_in"], event["pol_out"]) / 1e18
        usdc_amount = max(event["usdc_in"], event["usdc_out"]) / 1e6
        if pol_amount > 0:
            price = usdc_amount / pol_amount
            print(f"   Event {i}: ${price:.6f} per POL")

if __name__ == "__main__":
    asyncio.run(test_alchemy_websocket())
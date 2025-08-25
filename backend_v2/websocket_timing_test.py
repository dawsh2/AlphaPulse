#!/usr/bin/env python3
"""
Test to diagnose timing differences between Python and Rust WebSocket behavior
"""
import websockets
import json
import asyncio
import time

DEX_SIGNATURES = [
    "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822",  # Uniswap V2 Swap
    "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67",  # Uniswap V3 Swap  
    "0x4c209b5fc8ad50758f13e2e1088ba56a560dff690a1c6fef26394f4c03821c4f",  # Uniswap V2 Mint
    "0x7a53080ba414158be7ec69b987b5fb7d07dee101fe85488f0853ae16239d0bde",  # Uniswap V3 Mint
    "0xdccd412f0b1252819cb1fd330b93224ca42612892bb3f4f789976e6d81936496",  # Uniswap V2 Burn
    "0x0c396cd989a39f4459b5fa1aed6a9a8dcdbc45908acfd67e028cd568da98982c",  # Uniswap V3 Burn
    "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1"   # Uniswap V2 Sync
]

async def test_websocket_timing():
    """
    Test with exact same parameters as Rust: 60-second timeout, same subscription format
    """
    url = "wss://polygon.drpc.org"
    
    print("🧪 Testing WebSocket timing behavior exactly like Rust")
    print("📊 Parameters:")
    print("   - URL: wss://polygon.drpc.org")
    print("   - Timeout: 60 seconds per message (like Rust)")
    print("   - Subscription: eth_subscribe with logs filter")
    print()
    
    try:
        # Connect with no ping to match Rust behavior
        async with websockets.connect(url, ping_interval=None, ping_timeout=None) as websocket:
            print("✅ Connected!")
            
            # Create exact same subscription as Rust
            subscription = {
                "jsonrpc": "2.0",
                "id": 1,
                "method": "eth_subscribe",
                "params": [
                    "logs", 
                    {
                        "topics": [DEX_SIGNATURES]
                    }
                ]
            }
            
            print("📡 Sending subscription...")
            await websocket.send(json.dumps(subscription))
            
            # Wait for confirmation with 10s timeout
            try:
                response = await asyncio.wait_for(websocket.recv(), timeout=10.0)
                resp_data = json.loads(response)
                if 'result' in resp_data:
                    print(f"✅ Subscription confirmed: {resp_data['result']}")
                else:
                    print(f"❌ Subscription failed: {resp_data}")
                    return
            except asyncio.TimeoutError:
                print("❌ Subscription confirmation timeout!")
                return
                
            # Now test the exact same message loop pattern as Rust
            print("\n🔄 Testing Rust-like message loop...")
            print("📊 Using 60-second timeout per message (exactly like Rust)")
            
            iteration = 0
            total_messages = 0
            start_time = time.time()
            
            while iteration < 10:  # Test for 10 iterations  
                iteration += 1
                iteration_start = time.time()
                
                try:
                    # Use 60-second timeout exactly like Rust
                    message = await asyncio.wait_for(websocket.recv(), timeout=60.0)
                    total_messages += 1
                    
                    # Parse message
                    data = json.loads(message)
                    elapsed_since_iteration = time.time() - iteration_start
                    elapsed_total = time.time() - start_time
                    
                    if data.get('method') == 'eth_subscription':
                        params = data.get('params', {})
                        result = params.get('result', {})
                        topic0 = result.get('topics', ['N/A'])[0]
                        block_num = result.get('blockNumber', 'N/A')
                        
                        print(f"🎯 Iteration {iteration}: DEX Event {total_messages}")
                        print(f"   Time in iteration: {elapsed_since_iteration:.3f}s")
                        print(f"   Total elapsed: {elapsed_total:.3f}s")
                        print(f"   Topic0: {topic0}")
                        print(f"   Block: {block_num}")
                        print()
                        
                        # If we get messages quickly, note the timing
                        if elapsed_since_iteration < 1.0:
                            print(f"📈 Fast message! Received in {elapsed_since_iteration:.3f}s")
                            
                    elif 'id' in data:
                        print(f"📋 Confirmation message: {data}")
                    else:
                        print(f"📨 Other message: {data.get('method', 'unknown')}")
                        
                except asyncio.TimeoutError:
                    elapsed_iteration = time.time() - iteration_start
                    elapsed_total = time.time() - start_time
                    print(f"⏳ Iteration {iteration}: 60s timeout reached")
                    print(f"   Iteration time: {elapsed_iteration:.3f}s")
                    print(f"   Total elapsed: {elapsed_total:.3f}s")
                    print(f"   Messages so far: {total_messages}")
                    print()
                    
                    # This matches Rust behavior - continue on timeout
                    continue
                    
            final_elapsed = time.time() - start_time
            print(f"\n📊 Final Results:")
            print(f"   Test duration: {final_elapsed:.3f}s")
            print(f"   Total messages: {total_messages}")
            print(f"   Message rate: {total_messages / final_elapsed:.3f} msg/s")
            
            if total_messages > 0:
                print("✅ Messages received successfully!")
                print("❗ This proves the issue is in Rust WebSocket handling")
            else:
                print("❌ No messages received")
                print("❓ Need to investigate further")
                
    except Exception as e:
        print(f"❌ Error: {e}")

if __name__ == "__main__":
    asyncio.run(test_websocket_timing())
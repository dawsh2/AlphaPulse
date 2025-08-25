#!/usr/bin/env python3
"""
Comprehensive WebSocket debugging to identify the exact issue with Rust collector
"""
import websockets
import json
import asyncio
import time

# The exact signatures from our Rust code
DEX_SIGNATURES = [
    "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822",  # Uniswap V2 Swap
    "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67",  # Uniswap V3 Swap  
    "0x4c209b5fc8ad50758f13e2e1088ba56a560dff690a1c6fef26394f4c03821c4f",  # Uniswap V2 Mint
    "0x7a53080ba414158be7ec69b987b5fb7d07dee101fe85488f0853ae16239d0bde",  # Uniswap V3 Mint
    "0xdccd412f0b1252819cb1fd330b93224ca42612892bb3f4f789976e6d81936496",  # Uniswap V2 Burn
    "0x0c396cd989a39f4459b5fa1aed6a9a8dcdbc45908acfd67e028cd568da98982c",  # Uniswap V3 Burn
    "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1"   # Uniswap V2 Sync
]

async def debug_websocket_behavior():
    """
    Test the exact same behavior as our Rust code to identify discrepancies
    """
    url = "wss://polygon.drpc.org"
    
    print(f"🔌 Connecting to {url}")
    
    try:
        # Use same timeout as Rust code (60 seconds)
        async with websockets.connect(url, ping_interval=None, ping_timeout=None) as websocket:
            print("✅ Connected!")
            
            # Create the exact same subscription as Rust code
            subscription = {
                "jsonrpc": "2.0",
                "id": 1,
                "method": "eth_subscribe",
                "params": [
                    "logs", 
                    {
                        "topics": [DEX_SIGNATURES]  # Array of topic0 filters - SAME AS RUST
                    }
                ]
            }
            
            print(f"📡 Sending subscription (exactly like Rust)...")
            print(f"📋 Subscription JSON: {json.dumps(subscription, indent=2)}")
            
            await websocket.send(json.dumps(subscription))
            print("✅ Subscription sent")
            
            # Wait for subscription confirmation with timeout
            print("⏳ Waiting for subscription confirmation...")
            try:
                response = await asyncio.wait_for(websocket.recv(), timeout=10.0)
                print(f"📋 Subscription response: {response}")
                
                resp_data = json.loads(response)
                if 'result' in resp_data:
                    subscription_id = resp_data['result']
                    print(f"✅ Subscription confirmed with ID: {subscription_id}")
                else:
                    print(f"❌ Subscription failed: {resp_data}")
                    return
                    
            except asyncio.TimeoutError:
                print("❌ Subscription confirmation timeout!")
                return
                
            # Now simulate the exact same message receive loop as Rust
            print("\n🔄 Starting message receive loop (simulating Rust behavior)...")
            print("📊 Will run for 120 seconds with 60-second timeout per message")
            
            message_count = 0
            timeout_count = 0
            start_time = time.time()
            
            for iteration in range(120):  # 120 iterations of 1-second checks
                try:
                    # Use 60-second timeout like Rust (but check every second for debugging)
                    message = await asyncio.wait_for(websocket.recv(), timeout=1.0)
                    message_count += 1
                    
                    try:
                        data = json.loads(message)
                        
                        # Check if this is a subscription notification
                        if data.get('method') == 'eth_subscription':
                            params = data.get('params', {})
                            result = params.get('result', {})
                            
                            topics = result.get('topics', [])
                            topic0 = topics[0] if topics else 'N/A'
                            block_num = result.get('blockNumber', 'N/A')
                            address = result.get('address', 'N/A')
                            
                            print(f"🎯 DEX Event {message_count}:")
                            print(f"   Topic0: {topic0}")
                            print(f"   Block: {block_num}")  
                            print(f"   Address: {address}")
                            print(f"   Time since start: {time.time() - start_time:.1f}s")
                            
                            if message_count >= 5:
                                print("✅ Received enough events to confirm working!")
                                break
                                
                        elif 'id' in data and data['id'] == 1:
                            # This is the subscription confirmation (already handled above)
                            pass
                        else:
                            print(f"📨 Other message type: {data.get('method', 'unknown')}")
                            
                    except json.JSONDecodeError:
                        print(f"⚠️  Non-JSON message: {message[:100]}...")
                        
                except asyncio.TimeoutError:
                    timeout_count += 1
                    elapsed = time.time() - start_time
                    if iteration % 10 == 0:  # Print every 10 seconds
                        print(f"⏳ {elapsed:.1f}s elapsed - {message_count} messages, {timeout_count} timeouts")
                        
            final_elapsed = time.time() - start_time
            print(f"\n📊 Final Results:")
            print(f"   Total runtime: {final_elapsed:.1f} seconds")
            print(f"   Messages received: {message_count}")
            print(f"   Timeouts: {timeout_count}")
            print(f"   Message rate: {message_count / final_elapsed:.2f} msg/s")
            
            if message_count > 0:
                print("✅ WebSocket is receiving DEX events correctly!")
                print("❓ Issue is definitely in Rust WebSocket handling code")
            else:
                print("❌ No messages received - same issue as Rust")
                
    except Exception as e:
        print(f"❌ Error: {e}")

if __name__ == "__main__":
    asyncio.run(debug_websocket_behavior())
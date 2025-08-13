#!/usr/bin/env python3
"""Test WebSocket client to verify orderbook delta streaming"""
import asyncio
import websockets
import json
import signal
import sys

# Global flag for graceful shutdown
running = True

def signal_handler(sig, frame):
    global running
    print("\n🛑 Received interrupt signal, shutting down...")
    running = False

signal.signal(signal.SIGINT, signal_handler)

async def test_delta_websocket():
    """Test WebSocket connection to receive orderbook deltas"""
    print("🧪 Testing OrderBook Delta WebSocket Streaming")
    print("=" * 50)
    
    uri = "ws://localhost:8765/ws"
    
    try:
        print(f"📡 Connecting to {uri}...")
        async with websockets.connect(uri) as websocket:
            print("✅ Connected to WebSocket server")
            
            # Subscribe to orderbook deltas
            subscription = {
                "type": "subscribe",
                "channels": ["orderbook_delta", "trade"]
            }
            await websocket.send(json.dumps(subscription))
            print("📬 Sent subscription request")
            
            # Track message counts
            trade_count = 0
            orderbook_count = 0
            delta_count = 0
            
            print("\n📊 Listening for messages...")
            print("Type Ctrl+C to stop\n")
            
            while running:
                try:
                    # Receive message with timeout
                    message = await asyncio.wait_for(
                        websocket.recv(), 
                        timeout=1.0
                    )
                    
                    try:
                        data = json.loads(message)
                        msg_type = data.get("type", "unknown")
                        
                        if msg_type == "trade":
                            trade_count += 1
                            trade_data = data.get("data", {})
                            print(f"💰 Trade #{trade_count}: {trade_data.get('symbol')} ${trade_data.get('price'):.2f} ({trade_data.get('exchange')})")
                            
                        elif msg_type == "orderbook":
                            orderbook_count += 1
                            orderbook_data = data.get("data", {})
                            bids = len(orderbook_data.get("bids", []))
                            asks = len(orderbook_data.get("asks", []))
                            print(f"📖 OrderBook #{orderbook_count}: {orderbook_data.get('symbol')} - {bids} bids, {asks} asks")
                            
                        elif msg_type == "orderbook_delta":
                            delta_count += 1
                            delta_data = data.get("data", {})
                            changes = len(delta_data.get("changes", []))
                            version = delta_data.get("version", 0)
                            
                            # Show first few changes for analysis
                            changes_summary = []
                            for i, change in enumerate(delta_data.get("changes", [])[:3]):
                                side = change.get("side", "?")
                                price = change.get("price", 0)
                                volume = change.get("volume", 0)
                                action = change.get("action", "?")
                                changes_summary.append(f"{side} ${price:.2f}→{volume:.4f} ({action})")
                            
                            changes_str = ", ".join(changes_summary)
                            if len(delta_data.get("changes", [])) > 3:
                                changes_str += f", +{len(delta_data.get('changes', [])) - 3} more"
                            
                            print(f"🚀 Delta #{delta_count}: {delta_data.get('symbol')} v{version} - {changes} changes [{changes_str}]")
                            
                        else:
                            print(f"❓ Unknown message type: {msg_type}")
                            
                    except json.JSONDecodeError:
                        print(f"❌ Failed to parse message: {message[:100]}...")
                        
                except asyncio.TimeoutError:
                    # No message received, continue listening
                    continue
                except websockets.exceptions.ConnectionClosed:
                    print("🔌 WebSocket connection closed by server")
                    break
                    
    except ConnectionRefusedError:
        print(f"❌ Failed to connect to {uri}")
        print("   Make sure the WebSocket server is running on port 8765")
        return
    except Exception as e:
        print(f"❌ WebSocket error: {e}")
        return
    
    print(f"\n📈 Session Summary:")
    print(f"   Trades received: {trade_count}")
    print(f"   OrderBooks received: {orderbook_count}")
    print(f"   Deltas received: {delta_count}")
    print("✅ Delta WebSocket test complete!")

if __name__ == "__main__":
    try:
        asyncio.run(test_delta_websocket())
    except KeyboardInterrupt:
        print("\n🛑 Test interrupted by user")
        sys.exit(0)
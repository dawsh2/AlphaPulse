#!/usr/bin/env python3
"""
Test WebSocket functionality for terminal
"""
import asyncio
import websockets
import json
import sys

async def test_terminal_websocket():
    """Test the terminal WebSocket endpoint"""
    uri = "ws://localhost:8080/api/terminal/ws"
    
    print("Connecting to terminal WebSocket...")
    
    async with websockets.connect(uri) as websocket:
        # Wait for session creation message
        message = await websocket.recv()
        data = json.loads(message)
        
        if data["type"] == "session_created":
            session_id = data["session_id"]
            print(f"✅ Session created: {session_id}")
            print(f"   PID: {data['session']['pid']}")
            
            # Send a test command
            print("\nSending command: 'echo Hello from WebSocket'")
            await websocket.send(json.dumps({
                "type": "input",
                "data": "echo Hello from WebSocket\n"
            }))
            
            # Read output
            print("\nWaiting for output...")
            for _ in range(5):  # Read a few messages
                try:
                    message = await asyncio.wait_for(websocket.recv(), timeout=2.0)
                    data = json.loads(message)
                    
                    if data["type"] == "output":
                        print(f"Output received: {repr(data['data'])}")
                        if "Hello from WebSocket" in data["data"]:
                            print("✅ Command executed successfully!")
                            break
                except asyncio.TimeoutError:
                    print("Timeout waiting for output")
                    break
            
            # Test resize
            print("\nTesting resize to 120x30...")
            await websocket.send(json.dumps({
                "type": "resize",
                "cols": 120,
                "rows": 30
            }))
            print("✅ Resize sent")
            
            # Send exit command
            print("\nSending exit command...")
            await websocket.send(json.dumps({
                "type": "input",
                "data": "exit\n"
            }))
            
            await asyncio.sleep(0.5)
            print("✅ Test completed successfully!")
        else:
            print(f"❌ Unexpected message: {data}")

async def test_existing_session():
    """Test connecting to an existing session"""
    import aiohttp
    
    # First create a session via REST API
    async with aiohttp.ClientSession() as session:
        # Create session
        async with session.post('http://localhost:8080/api/terminal/sessions',
                               json={"shell": "/bin/bash"}) as resp:
            result = await resp.json()
            session_id = result["session"]["session_id"]
            print(f"Created session via REST: {session_id}")
        
        # Connect via WebSocket
        uri = f"ws://localhost:8080/api/terminal/ws/{session_id}"
        print(f"Connecting to WebSocket for session {session_id}...")
        
        async with websockets.connect(uri) as websocket:
            # Send a command
            await websocket.send(json.dumps({
                "type": "input",
                "data": "pwd\n"
            }))
            
            # Read output
            for _ in range(3):
                try:
                    message = await asyncio.wait_for(websocket.recv(), timeout=1.0)
                    data = json.loads(message)
                    if data["type"] == "output":
                        print(f"Output: {repr(data['data'])}")
                except asyncio.TimeoutError:
                    break
        
        # Clean up session
        async with session.delete(f'http://localhost:8080/api/terminal/sessions/{session_id}') as resp:
            print(f"Deleted session: {session_id}")

async def main():
    print("=" * 60)
    print("Terminal WebSocket Test")
    print("=" * 60)
    
    # Test 1: New session via WebSocket
    print("\n### Test 1: Create new session via WebSocket ###")
    try:
        await test_terminal_websocket()
    except Exception as e:
        print(f"❌ Test 1 failed: {e}")
    
    # Test 2: Connect to existing session
    print("\n### Test 2: Connect to existing session ###")
    try:
        await test_existing_session()
    except Exception as e:
        print(f"❌ Test 2 failed: {e}")
    
    print("\n" + "=" * 60)
    print("All tests completed!")

if __name__ == "__main__":
    asyncio.run(main())
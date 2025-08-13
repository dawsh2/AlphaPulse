#!/usr/bin/env python3
"""
Comprehensive test suite for FastAPI migration
Tests all migrated endpoints to ensure functionality
"""
import asyncio
import aiohttp
import json
import websockets
from typing import Dict, Any
import sys

BASE_URL = "http://localhost:8080"
WS_URL = "ws://localhost:8080"

class Colors:
    """ANSI color codes for terminal output"""
    GREEN = '\033[92m'
    RED = '\033[91m'
    YELLOW = '\033[93m'
    BLUE = '\033[94m'
    ENDC = '\033[0m'
    BOLD = '\033[1m'

def print_header(text: str):
    """Print a formatted header"""
    print(f"\n{Colors.BOLD}{Colors.BLUE}{'=' * 60}{Colors.ENDC}")
    print(f"{Colors.BOLD}{Colors.BLUE}{text}{Colors.ENDC}")
    print(f"{Colors.BOLD}{Colors.BLUE}{'=' * 60}{Colors.ENDC}")

def print_test(name: str, success: bool, details: str = ""):
    """Print test result"""
    if success:
        status = f"{Colors.GREEN}✅ PASS{Colors.ENDC}"
    else:
        status = f"{Colors.RED}❌ FAIL{Colors.ENDC}"
    
    print(f"  {status} {name}")
    if details:
        print(f"      {Colors.YELLOW}{details}{Colors.ENDC}")

async def check_health(session: aiohttp.ClientSession) -> bool:
    """Test health check endpoint"""
    try:
        async with session.get(f"{BASE_URL}/api/health") as resp:
            data = await resp.json()
            return resp.status == 200 and data.get("status") == "ok"
    except Exception as e:
        print(f"      Error: {e}")
        return False

async def check_auth_endpoints(session: aiohttp.ClientSession) -> Dict[str, Any]:
    """Test authentication endpoints"""
    results = {}
    
    # Test demo login
    try:
        async with session.post(f"{BASE_URL}/api/auth/demo-login") as resp:
            if resp.status == 200:
                data = await resp.json()
                results['demo_login'] = True
                results['token'] = data.get('access_token')
                results['user_id'] = data.get('user_id')
            else:
                results['demo_login'] = False
    except Exception as e:
        results['demo_login'] = False
        results['error'] = str(e)
    
    # Test /me endpoint with token
    if results.get('token'):
        headers = {'Authorization': f"Bearer {results['token']}"}
        try:
            async with session.get(f"{BASE_URL}/api/auth/me", headers=headers) as resp:
                results['auth_me'] = resp.status == 200
        except:
            results['auth_me'] = False
    
    return results

async def check_notebook_endpoints(session: aiohttp.ClientSession) -> Dict[str, bool]:
    """Test notebook/Jupyter endpoints"""
    results = {}
    
    # Test list kernels
    try:
        async with session.get(f"{BASE_URL}/api/notebook/kernels") as resp:
            results['list_kernels'] = resp.status == 200
    except:
        results['list_kernels'] = False
    
    # Test list templates
    try:
        async with session.get(f"{BASE_URL}/api/notebook/templates") as resp:
            results['list_templates'] = resp.status == 200
    except:
        results['list_templates'] = False
    
    # Test execute code
    try:
        payload = {"code": "print(1 + 1)"}
        async with session.post(f"{BASE_URL}/api/notebook/execute", json=payload) as resp:
            if resp.status == 200:
                data = await resp.json()
                results['execute_code'] = data.get('status') == 'ok'
            else:
                results['execute_code'] = False
    except:
        results['execute_code'] = False
    
    return results

async def check_workspace_endpoints(session: aiohttp.ClientSession) -> Dict[str, bool]:
    """Test workspace file management endpoints"""
    results = {}
    
    # Test health
    try:
        async with session.get(f"{BASE_URL}/api/workspace/health") as resp:
            results['health'] = resp.status == 200
    except:
        results['health'] = False
    
    # Test list files
    try:
        async with session.get(f"{BASE_URL}/api/workspace/files") as resp:
            results['list_files'] = resp.status == 200
    except:
        results['list_files'] = False
    
    # Test file CRUD
    test_file = "test_api.txt"
    test_content = "Hello from API test"
    
    # Create file
    try:
        payload = {"content": test_content}
        async with session.put(f"{BASE_URL}/api/workspace/file/{test_file}", json=payload) as resp:
            results['create_file'] = resp.status == 200
    except:
        results['create_file'] = False
    
    # Read file
    try:
        async with session.get(f"{BASE_URL}/api/workspace/file/{test_file}") as resp:
            if resp.status == 200:
                data = await resp.json()
                results['read_file'] = data.get('content') == test_content
            else:
                results['read_file'] = False
    except:
        results['read_file'] = False
    
    # Delete file
    try:
        async with session.delete(f"{BASE_URL}/api/workspace/file/{test_file}") as resp:
            results['delete_file'] = resp.status == 200
    except:
        results['delete_file'] = False
    
    return results

async def check_terminal_endpoints(session: aiohttp.ClientSession) -> Dict[str, bool]:
    """Test terminal session endpoints"""
    results = {}
    
    # Test health
    try:
        async with session.get(f"{BASE_URL}/api/terminal/health") as resp:
            results['health'] = resp.status == 200
    except:
        results['health'] = False
    
    # Test create session
    session_id = None
    try:
        payload = {"shell": "/bin/bash"}
        async with session.post(f"{BASE_URL}/api/terminal/sessions", json=payload) as resp:
            if resp.status == 200:
                data = await resp.json()
                session_id = data.get('session', {}).get('session_id')
                results['create_session'] = bool(session_id)
            else:
                results['create_session'] = False
    except:
        results['create_session'] = False
    
    # Test list sessions
    try:
        async with session.get(f"{BASE_URL}/api/terminal/sessions") as resp:
            results['list_sessions'] = resp.status == 200
    except:
        results['list_sessions'] = False
    
    # Test delete session
    if session_id:
        try:
            async with session.delete(f"{BASE_URL}/api/terminal/sessions/{session_id}") as resp:
                results['delete_session'] = resp.status == 200
        except:
            results['delete_session'] = False
    
    # Test WebSocket
    try:
        async with websockets.connect(f"{WS_URL}/api/terminal/ws") as ws:
            # Wait for session creation
            msg = await asyncio.wait_for(ws.recv(), timeout=2)
            data = json.loads(msg)
            
            if data.get('type') == 'session_created':
                # Send test command
                await ws.send(json.dumps({
                    "type": "input",
                    "data": "echo test\n"
                }))
                
                # Read response
                msg = await asyncio.wait_for(ws.recv(), timeout=2)
                data = json.loads(msg)
                results['websocket'] = data.get('type') == 'output'
            else:
                results['websocket'] = False
    except:
        results['websocket'] = False
    
    return results

async def check_data_endpoints(session: aiohttp.ClientSession) -> Dict[str, bool]:
    """Test data and analysis endpoints"""
    results = {}
    
    # Test health
    try:
        async with session.get(f"{BASE_URL}/api/data/health") as resp:
            results['health'] = resp.status == 200
    except:
        results['health'] = False
    
    # Test data summary
    try:
        async with session.get(f"{BASE_URL}/api/data/summary") as resp:
            results['data_summary'] = resp.status == 200
    except:
        results['data_summary'] = False
    
    # Test crypto data
    try:
        async with session.get(f"{BASE_URL}/api/crypto-data/BTC-USD?limit=10") as resp:
            results['crypto_data'] = resp.status == 200
    except:
        results['crypto_data'] = False
    
    # Test statistics
    try:
        async with session.get(f"{BASE_URL}/api/analysis/statistics/BTC-USD") as resp:
            results['statistics'] = resp.status == 200
    except:
        results['statistics'] = False
    
    # Test risk metrics
    try:
        async with session.get(f"{BASE_URL}/api/analysis/risk-metrics/BTC-USD") as resp:
            results['risk_metrics'] = resp.status == 200
    except:
        results['risk_metrics'] = False
    
    return results

async def run_all_tests():
    """Run all tests"""
    print_header("FastAPI Migration Test Suite")
    
    total_tests = 0
    passed_tests = 0
    
    async with aiohttp.ClientSession() as session:
        # 1. Health Check
        print("\n📍 Testing Health Check...")
        result = await check_health(session)
        print_test("Health Check", result)
        total_tests += 1
        if result: passed_tests += 1
        
        # 2. Authentication
        print("\n🔐 Testing Authentication...")
        auth_results = await check_auth_endpoints(session)
        for name, success in auth_results.items():
            if name not in ['token', 'user_id', 'error']:
                print_test(f"Auth: {name}", success)
                total_tests += 1
                if success: passed_tests += 1
        
        # 3. Notebook/Jupyter
        print("\n📓 Testing Notebook/Jupyter...")
        notebook_results = await check_notebook_endpoints(session)
        for name, success in notebook_results.items():
            print_test(f"Notebook: {name}", success)
            total_tests += 1
            if success: passed_tests += 1
        
        # 4. Workspace
        print("\n📁 Testing Workspace...")
        workspace_results = await check_workspace_endpoints(session)
        for name, success in workspace_results.items():
            print_test(f"Workspace: {name}", success)
            total_tests += 1
            if success: passed_tests += 1
        
        # 5. Terminal
        print("\n💻 Testing Terminal...")
        terminal_results = await check_terminal_endpoints(session)
        for name, success in terminal_results.items():
            print_test(f"Terminal: {name}", success)
            total_tests += 1
            if success: passed_tests += 1
        
        # 6. Data & Analysis
        print("\n📊 Testing Data & Analysis...")
        data_results = await check_data_endpoints(session)
        for name, success in data_results.items():
            print_test(f"Data: {name}", success)
            total_tests += 1
            if success: passed_tests += 1
    
    # Summary
    print_header("Test Summary")
    success_rate = (passed_tests / total_tests * 100) if total_tests > 0 else 0
    
    if success_rate == 100:
        color = Colors.GREEN
        status = "ALL TESTS PASSED! 🎉"
    elif success_rate >= 80:
        color = Colors.YELLOW
        status = "Most tests passed"
    else:
        color = Colors.RED
        status = "Many tests failed"
    
    print(f"\n{color}{status}{Colors.ENDC}")
    print(f"Tests Run: {total_tests}")
    print(f"Tests Passed: {passed_tests}")
    print(f"Tests Failed: {total_tests - passed_tests}")
    print(f"Success Rate: {success_rate:.1f}%")
    
    if success_rate == 100:
        print(f"\n{Colors.GREEN}✅ FastAPI migration is complete and working!{Colors.ENDC}")
        print(f"{Colors.GREEN}   Ready to deprecate Flask application.{Colors.ENDC}")
    
    return success_rate == 100

async def main():
    """Main test runner"""
    try:
        # Check if server is running
        async with aiohttp.ClientSession() as session:
            try:
                async with session.get(f"{BASE_URL}/api/health", timeout=2) as resp:
                    if resp.status != 200:
                        print(f"{Colors.RED}❌ Server not responding properly{Colors.ENDC}")
                        print(f"   Make sure FastAPI is running on port 8080")
                        sys.exit(1)
            except:
                print(f"{Colors.RED}❌ Cannot connect to server at {BASE_URL}{Colors.ENDC}")
                print(f"   Run: python app_fastapi.py")
                sys.exit(1)
        
        # Run tests
        success = await run_all_tests()
        sys.exit(0 if success else 1)
        
    except KeyboardInterrupt:
        print(f"\n{Colors.YELLOW}Tests interrupted by user{Colors.ENDC}")
        sys.exit(1)
    except Exception as e:
        print(f"\n{Colors.RED}Test suite error: {e}{Colors.ENDC}")
        sys.exit(1)

if __name__ == "__main__":
    asyncio.run(main())
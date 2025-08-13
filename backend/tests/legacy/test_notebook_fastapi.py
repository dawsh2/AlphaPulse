#!/usr/bin/env python3
"""
Test script for FastAPI notebook endpoints
"""
import requests
import json
import time

BASE_URL = "http://localhost:8080"

def test_notebook_health():
    """Test notebook health endpoint"""
    response = requests.get(f"{BASE_URL}/api/notebook/health")
    print("Notebook Health Check:")
    print(json.dumps(response.json(), indent=2))
    assert response.status_code == 200
    print("✅ Health check passed\n")

def test_kernel_status():
    """Test kernel status"""
    response = requests.get(f"{BASE_URL}/api/notebook/status")
    print("Kernel Status:")
    print(json.dumps(response.json(), indent=2))
    assert response.status_code == 200
    print("✅ Status check passed\n")
    return response.json()

def test_execute_code():
    """Test code execution"""
    code = """
import numpy as np
import pandas as pd
print("Hello from Jupyter!")
print(f"NumPy version: {np.__version__}")
print(f"Pandas version: {pd.__version__}")
result = 2 + 2
print(f"2 + 2 = {result}")
"""
    
    response = requests.post(
        f"{BASE_URL}/api/notebook/execute",
        json={"code": code}
    )
    
    print("Code Execution Test:")
    print("Code sent:")
    print(code)
    print("\nResult:")
    print(json.dumps(response.json(), indent=2))
    assert response.status_code == 200
    assert response.json()["error"] is None
    print("✅ Code execution passed\n")

def test_templates():
    """Test getting templates"""
    response = requests.get(f"{BASE_URL}/api/notebook/templates")
    print("Available Templates:")
    print(json.dumps(response.json(), indent=2))
    assert response.status_code == 200
    print("✅ Templates list passed\n")
    
    # Test getting a specific template
    response = requests.get(f"{BASE_URL}/api/notebook/templates/arbitrage_basic")
    print("Arbitrage Basic Template (first few lines):")
    template = response.json()
    print(f"Title: {template.get('title', 'N/A')}")
    print(f"Description: {template.get('description', 'N/A')}")
    if 'code' in template:
        lines = template['code'].split('\n')[:5]
        print("Code preview:")
        for line in lines:
            print(f"  {line}")
    assert response.status_code == 200
    print("✅ Template retrieval passed\n")

def test_kernel_restart():
    """Test kernel restart"""
    response = requests.post(f"{BASE_URL}/api/notebook/restart")
    print("Kernel Restart:")
    print(json.dumps(response.json(), indent=2))
    assert response.status_code == 200
    print("✅ Kernel restart passed\n")

def test_kernel_cleanup():
    """Test kernel cleanup"""
    response = requests.post(f"{BASE_URL}/api/notebook/cleanup")
    print("Kernel Cleanup:")
    print(json.dumps(response.json(), indent=2))
    assert response.status_code == 200
    print("✅ Kernel cleanup passed\n")

if __name__ == "__main__":
    print("🧪 Testing FastAPI Notebook Endpoints...\n")
    print("=" * 50)
    
    try:
        # Test all endpoints
        test_notebook_health()
        
        # Check initial status
        initial_status = test_kernel_status()
        
        # Execute some code (this will start kernel if not running)
        test_execute_code()
        
        # Check status after execution
        test_kernel_status()
        
        # Test templates
        test_templates()
        
        # Test restart
        test_kernel_restart()
        
        # Test cleanup
        test_kernel_cleanup()
        
        # Check final status (should be stopped)
        final_status = test_kernel_status()
        
        print("=" * 50)
        print("✅ All notebook tests passed!")
        print("\nNotebook API successfully migrated to FastAPI")
        
    except AssertionError as e:
        print(f"❌ Test assertion failed: {e}")
    except Exception as e:
        print(f"❌ Test failed: {e}")
        print("\nMake sure the FastAPI server is running:")
        print("  python app_fastapi.py")
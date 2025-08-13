#!/usr/bin/env python3
"""
Test script to verify FastAPI server is working
"""
import requests
import json

BASE_URL = "http://localhost:8080"

def check_health():
    """Test health endpoint"""
    response = requests.get(f"{BASE_URL}/api/health")
    print("Health Check:")
    print(json.dumps(response.json(), indent=2))
    assert response.status_code == 200
    print("✅ Health check passed\n")

def check_demo_login():
    """Test demo login"""
    response = requests.post(f"{BASE_URL}/api/auth/demo-login")
    print("Demo Login:")
    data = response.json()
    print(json.dumps(data, indent=2))
    assert response.status_code == 200
    assert "access_token" in data
    print("✅ Demo login passed\n")
    return data["access_token"]

def check_auth_me(token):
    """Test authenticated endpoint"""
    headers = {"Authorization": f"Bearer {token}"}
    response = requests.get(f"{BASE_URL}/api/auth/me", headers=headers)
    print("Get Current User:")
    print(json.dumps(response.json(), indent=2))
    assert response.status_code == 200
    print("✅ Auth check passed\n")

def check_deprecated_endpoint():
    """Test deprecated endpoint warning"""
    response = requests.get(f"{BASE_URL}/api/market-data/BTC-USD")
    print("Deprecated Endpoint Test:")
    print(json.dumps(response.json(), indent=2))
    assert "warning" in response.json()
    print("✅ Deprecation warning working\n")

if __name__ == "__main__":
    print("🧪 Testing FastAPI Server...\n")
    print("=" * 50)
    
    try:
        check_health()
        token = check_demo_login()
        check_auth_me(token)
        check_deprecated_endpoint()
        
        print("=" * 50)
        print("✅ All tests passed!")
        print("\nFastAPI server is working correctly.")
        print("Access interactive docs at: http://localhost:8080/docs")
    except Exception as e:
        print(f"❌ Test failed: {e}")
        print("\nMake sure the FastAPI server is running:")
        print("  python app_fastapi.py")
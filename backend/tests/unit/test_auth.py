"""
Unit tests for authentication endpoints
"""
import pytest
from fastapi.testclient import TestClient

@pytest.mark.unit
class TestAuthentication:
    """Test authentication endpoints"""
    
    def test_health_check(self, client: TestClient):
        """Test health check endpoint"""
        response = client.get("/api/health")
        assert response.status_code == 200
        data = response.json()
        assert data["status"] == "ok"
        assert "version" in data
    
    def test_demo_login(self, client: TestClient):
        """Test demo login"""
        response = client.post("/api/auth/demo-login")
        assert response.status_code == 200
        data = response.json()
        assert "access_token" in data
        assert data["token_type"] == "bearer"
        assert data["username"] == "demo"
    
    def test_login_with_invalid_credentials(self, client: TestClient):
        """Test login with invalid credentials"""
        response = client.post("/api/auth/login", json={
            "email": "invalid@example.com",
            "password": "wrongpassword"
        })
        assert response.status_code == 401
    
    def test_get_current_user(self, client: TestClient, auth_headers: dict):
        """Test getting current user info"""
        response = client.get("/api/auth/me", headers=auth_headers)
        assert response.status_code == 200
        data = response.json()
        assert data["email"] == "demo@alphapulse.io"
        assert data["username"] == "demo"
    
    def test_get_current_user_without_token(self, client: TestClient):
        """Test getting current user without authentication"""
        response = client.get("/api/auth/me")
        assert response.status_code == 403  # Forbidden without token
    
    def test_get_current_user_with_invalid_token(self, client: TestClient):
        """Test getting current user with invalid token"""
        headers = {"Authorization": "Bearer invalid_token"}
        response = client.get("/api/auth/me", headers=headers)
        assert response.status_code == 401
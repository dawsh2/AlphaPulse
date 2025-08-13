"""
Pytest configuration and fixtures
Shared test utilities and setup
"""
import pytest
import asyncio
from typing import Generator, AsyncGenerator
import sys
import os
from pathlib import Path

# Add backend directory to path
backend_dir = Path(__file__).parent.parent
sys.path.insert(0, str(backend_dir))

from fastapi.testclient import TestClient
from httpx import AsyncClient
import aiohttp

# Import the FastAPI app
from app import app

# Test configuration
TEST_BASE_URL = "http://localhost:8080"
TEST_WS_URL = "ws://localhost:8080"

@pytest.fixture(scope="session")
def event_loop():
    """Create an event loop for the test session"""
    loop = asyncio.get_event_loop_policy().new_event_loop()
    yield loop
    loop.close()

@pytest.fixture
def client() -> Generator:
    """Create a test client for FastAPI app"""
    with TestClient(app) as c:
        yield c

@pytest.fixture
async def async_client() -> AsyncGenerator:
    """Create an async test client for FastAPI app"""
    async with AsyncClient(app=app, base_url="http://test") as ac:
        yield ac

@pytest.fixture
async def aiohttp_session() -> AsyncGenerator:
    """Create an aiohttp session for testing"""
    async with aiohttp.ClientSession() as session:
        yield session

@pytest.fixture
def demo_token(client: TestClient) -> str:
    """Get a demo authentication token"""
    response = client.post("/api/auth/demo-login")
    assert response.status_code == 200
    return response.json()["access_token"]

@pytest.fixture
def auth_headers(demo_token: str) -> dict:
    """Get authentication headers with token"""
    return {"Authorization": f"Bearer {demo_token}"}

@pytest.fixture
def sample_market_data() -> dict:
    """Sample market data for testing"""
    return {
        "symbol": "BTC/USD",
        "exchange": "coinbase",
        "interval": "1m",
        "candles": [
            {
                "timestamp": 1234567890,
                "open": 50000.0,
                "high": 50100.0,
                "low": 49900.0,
                "close": 50050.0,
                "volume": 10.5
            }
        ]
    }

@pytest.fixture
def sample_code() -> str:
    """Sample Python code for notebook execution"""
    return "print(1 + 1)"

@pytest.fixture
def sample_strategy_config() -> dict:
    """Sample strategy configuration for backtesting"""
    return {
        "symbol": "BTC/USD",
        "exchange": "coinbase",
        "type": "simple_ma_cross",
        "fast_period": 10,
        "slow_period": 20
    }

# Test markers
def pytest_configure(config):
    """Configure custom markers"""
    config.addinivalue_line(
        "markers", "unit: Unit tests"
    )
    config.addinivalue_line(
        "markers", "integration: Integration tests"
    )
    config.addinivalue_line(
        "markers", "e2e: End-to-end tests"
    )
    config.addinivalue_line(
        "markers", "slow: Slow tests"
    )
    config.addinivalue_line(
        "markers", "websocket: WebSocket tests"
    )
# AlphaPulse Test Suite

Comprehensive test suite for the AlphaPulse FastAPI backend.

## Directory Structure

```
tests/
├── __init__.py           # Test package initialization
├── conftest.py          # Pytest fixtures and configuration
├── unit/                # Unit tests (fast, isolated)
│   ├── test_auth.py    # Authentication tests
│   ├── test_notebook.py # Jupyter notebook tests
│   └── ...
├── integration/         # Integration tests
│   ├── test_data_analysis.py # Data & analysis tests
│   └── ...
├── e2e/                # End-to-end tests
│   └── test_complete_workflow.py
└── fixtures/           # Test data and fixtures
```

## Running Tests

### Quick Start

```bash
# Run all tests
./run_tests.sh

# Run specific test suites
./run_tests.sh unit        # Unit tests only
./run_tests.sh integration # Integration tests only
./run_tests.sh e2e         # End-to-end tests only

# Run with coverage
./run_tests.sh coverage
```

### Using Pytest Directly

```bash
# Run all tests
pytest

# Run with verbose output
pytest -v

# Run specific test file
pytest tests/unit/test_auth.py

# Run tests matching pattern
pytest -k "test_login"

# Run with coverage
pytest --cov=. --cov-report=html

# Run specific markers
pytest -m unit          # Unit tests only
pytest -m integration   # Integration tests only
pytest -m "not slow"    # Exclude slow tests
```

## Test Markers

- `@pytest.mark.unit` - Unit tests (fast, isolated)
- `@pytest.mark.integration` - Integration tests (may use database)
- `@pytest.mark.e2e` - End-to-end tests (full workflow)
- `@pytest.mark.slow` - Slow running tests
- `@pytest.mark.websocket` - Tests requiring WebSocket
- `@pytest.mark.skip_ci` - Skip in CI/CD pipeline

## Writing Tests

### Unit Test Example

```python
import pytest
from fastapi.testclient import TestClient

@pytest.mark.unit
def test_health_check(client: TestClient):
    """Test health check endpoint"""
    response = client.get("/api/health")
    assert response.status_code == 200
    assert response.json()["status"] == "ok"
```

### Integration Test Example

```python
@pytest.mark.integration
def test_data_workflow(client: TestClient):
    """Test complete data workflow"""
    # Save data
    response = client.post("/api/market-data/save", json=data)
    assert response.status_code == 200
    
    # Retrieve and analyze
    response = client.get("/api/analysis/statistics/BTC-USD")
    assert response.status_code == 200
```

### Async/WebSocket Test Example

```python
@pytest.mark.asyncio
@pytest.mark.websocket
async def test_terminal_websocket():
    """Test terminal WebSocket"""
    async with websockets.connect("ws://localhost:8080/api/terminal/ws") as ws:
        msg = await ws.recv()
        data = json.loads(msg)
        assert data["type"] == "session_created"
```

## Fixtures

Common fixtures are defined in `conftest.py`:

- `client` - FastAPI test client
- `async_client` - Async test client
- `demo_token` - Demo authentication token
- `auth_headers` - Headers with auth token
- `sample_market_data` - Sample market data
- `sample_code` - Sample Python code
- `sample_strategy_config` - Strategy config

## Coverage Reports

After running tests with coverage:

```bash
./run_tests.sh coverage
```

View the HTML coverage report:
```bash
open htmlcov/index.html
```

## Continuous Integration

Tests can be run in CI/CD pipelines:

```yaml
# GitHub Actions example
- name: Run tests
  run: |
    pip install -r requirements.txt
    pytest --cov=. --cov-report=xml
```

## Test Database

Tests use a separate test database to avoid affecting development data:
- Unit tests use in-memory SQLite
- Integration tests use test DuckDB instance
- E2E tests may create temporary data

## Troubleshooting

### Server not running
```bash
# Start FastAPI server
python app_fastapi.py
```

### Missing dependencies
```bash
pip install pytest pytest-asyncio pytest-cov aiohttp websockets
```

### Permission errors
```bash
chmod +x run_tests.sh
```

## Best Practices

1. **Write tests first** - TDD approach
2. **Keep tests isolated** - Each test should be independent
3. **Use fixtures** - Don't repeat setup code
4. **Test edge cases** - Not just happy paths
5. **Mock external services** - For unit tests
6. **Use meaningful names** - `test_login_with_invalid_credentials`
7. **Document complex tests** - Add docstrings
8. **Keep tests fast** - Mark slow tests appropriately

## Current Test Coverage

- ✅ Authentication (100%)
- ✅ Notebook/Jupyter (100%)
- ✅ Workspace (100%)
- ✅ Terminal (100%)
- ✅ Data & Analysis (100%)
- ⏳ WebSocket streaming (partial)
- ⏳ Error handling (partial)

## TODO

- [ ] Add performance benchmarks
- [ ] Add load testing with Locust
- [ ] Add mutation testing
- [ ] Add property-based testing with Hypothesis
- [ ] Setup CI/CD integration
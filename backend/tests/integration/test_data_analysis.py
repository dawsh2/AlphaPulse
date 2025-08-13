"""
Integration tests for data and analysis endpoints
"""
import pytest
from fastapi.testclient import TestClient
from typing import Dict, Any

@pytest.mark.integration
class TestDataAnalysis:
    """Test data and analysis endpoints"""
    
    def test_data_health(self, client: TestClient):
        """Test data service health"""
        response = client.get("/api/data/health")
        assert response.status_code == 200
        data = response.json()
        assert data["service"] == "data_and_analysis"
        assert data["status"] == "healthy"
    
    def test_save_market_data(self, client: TestClient, sample_market_data: dict):
        """Test saving market data"""
        response = client.post("/api/market-data/save", json=sample_market_data)
        assert response.status_code == 200
        data = response.json()
        assert data["status"] == "success"
        assert "symbol" in data
        assert data["symbol"] == sample_market_data["symbol"]
    
    def test_get_crypto_data(self, client: TestClient):
        """Test getting crypto OHLCV data"""
        response = client.get("/api/crypto-data/BTC-USD?limit=10")
        assert response.status_code == 200
        data = response.json()
        assert "data" in data
        assert "symbol" in data
        assert "count" in data
        assert isinstance(data["data"], list)
    
    def test_get_data_summary(self, client: TestClient):
        """Test getting data summary"""
        response = client.get("/api/data/summary")
        assert response.status_code == 200
        data = response.json()
        assert "total_bars" in data
        assert "symbols" in data
    
    def test_list_catalog(self, client: TestClient):
        """Test listing data catalog"""
        response = client.get("/api/catalog/list")
        assert response.status_code == 200
        data = response.json()
        assert "catalog" in data or "status" in data
    
    def test_get_statistics(self, client: TestClient):
        """Test getting symbol statistics"""
        response = client.get("/api/analysis/statistics/BTC-USD")
        assert response.status_code == 200
        data = response.json()
        assert data["symbol"] == "BTC/USD"
        assert "statistics" in data
        stats = data["statistics"]
        assert "mean_return" in stats
        assert "volatility" in stats
        assert "sharpe_ratio" in stats
    
    def test_get_risk_metrics(self, client: TestClient):
        """Test getting risk metrics"""
        response = client.get("/api/analysis/risk-metrics/BTC-USD?risk_free_rate=0.02")
        assert response.status_code == 200
        data = response.json()
        assert data["symbol"] == "BTC/USD"
        assert "metrics" in data
        metrics = data["metrics"]
        assert "sharpe_ratio" in metrics
        assert "max_drawdown" in metrics
        assert "var_95" in metrics
    
    def test_get_rolling_statistics(self, client: TestClient):
        """Test getting rolling statistics"""
        response = client.get("/api/analysis/rolling-stats/BTC-USD?window=20")
        assert response.status_code == 200
        data = response.json()
        assert data["symbol"] == "BTC/USD"
        assert data["window"] == 20
        assert "data" in data
    
    def test_correlation_between_symbols(self, client: TestClient):
        """Test correlation calculation between two symbols"""
        response = client.get("/api/data/correlation/BTC-USD/ETH-USD")
        assert response.status_code == 200
        data = response.json()
        assert "correlation" in data or "status" in data
    
    def test_correlation_matrix(self, client: TestClient):
        """Test correlation matrix for multiple symbols"""
        response = client.post("/api/analysis/correlation-matrix", json={
            "symbols": ["BTC-USD", "ETH-USD"],
            "exchange": "coinbase"
        })
        assert response.status_code == 200
        data = response.json()
        assert "correlations" in data or "error" in data
    
    def test_market_regime_analysis(self, client: TestClient):
        """Test market regime analysis"""
        response = client.post("/api/analysis/market-regime", json={
            "symbols": ["BTC-USD"],
            "exchange": "coinbase"
        })
        assert response.status_code == 200
        data = response.json()
        assert "regime_analysis" in data
    
    def test_run_backtest(self, client: TestClient, sample_strategy_config: dict):
        """Test running a backtest"""
        response = client.post("/api/analysis/backtest", json=sample_strategy_config)
        assert response.status_code == 200
        data = response.json()
        # Backtest might fail if no data, but endpoint should work
        assert "status" in data or "results" in data
    
    @pytest.mark.skip(reason="Query endpoint not yet implemented")
    def test_query_data(self, client: TestClient):
        """Test SQL query on DuckDB"""
        # TODO: Implement /api/data/query endpoint in FastAPI
        response = client.post("/api/data/query", json={
            "query": "SELECT COUNT(*) as count FROM ohlcv LIMIT 1"
        })
        assert response.status_code == 200
        data = response.json()
        assert "result" in data or "status" in data
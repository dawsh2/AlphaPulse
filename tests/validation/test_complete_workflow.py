"""
End-to-end tests for complete workflows
"""
import pytest
import asyncio
import json
import websockets
from fastapi.testclient import TestClient
import aiohttp

@pytest.mark.e2e
class TestCompleteWorkflows:
    """Test complete user workflows"""
    
    def test_authentication_workflow(self, client: TestClient):
        """Test complete authentication workflow"""
        # 1. Health check
        response = client.get("/api/health")
        assert response.status_code == 200
        
        # 2. Demo login
        response = client.post("/api/auth/demo-login")
        assert response.status_code == 200
        token = response.json()["access_token"]
        
        # 3. Use token to get user info
        headers = {"Authorization": f"Bearer {token}"}
        response = client.get("/api/auth/me", headers=headers)
        assert response.status_code == 200
        assert response.json()["username"] == "demo"
    
    def test_notebook_execution_workflow(self, client: TestClient):
        """Test complete notebook execution workflow"""
        # 1. Start kernel
        response = client.post("/api/notebook/kernels")
        assert response.status_code == 200
        
        # 2. Execute simple code
        response = client.post("/api/notebook/execute", json={
            "code": "x = 10\nprint(f'x = {x}')"
        })
        assert response.status_code == 200
        assert "x = 10" in response.json()["output"]
        
        # 3. Execute code using previous variable
        response = client.post("/api/notebook/execute", json={
            "code": "print(f'x * 2 = {x * 2}')"
        })
        assert response.status_code == 200
        assert "x * 2 = 20" in response.json()["output"]
        
        # 4. Stop kernel
        response = client.delete("/api/notebook/kernels/default")
        assert response.status_code == 200
    
    def test_workspace_file_workflow(self, client: TestClient):
        """Test complete workspace file management workflow"""
        test_file = "test_workflow.py"
        test_content = "# Test file\nprint('Hello, World!')"
        
        # 1. List files (should be empty or have some files)
        response = client.get("/api/workspace/files")
        assert response.status_code == 200
        initial_files = response.json()["files"]
        
        # 2. Create a new file
        response = client.put(f"/api/workspace/file/{test_file}", json={
            "content": test_content
        })
        assert response.status_code == 200
        assert response.json()["name"] == test_file
        
        # 3. Read the file back
        response = client.get(f"/api/workspace/file/{test_file}")
        assert response.status_code == 200
        assert response.json()["content"] == test_content
        
        # 4. Update the file
        updated_content = test_content + "\nprint('Updated!')"
        response = client.put(f"/api/workspace/file/{test_file}", json={
            "content": updated_content
        })
        assert response.status_code == 200
        
        # 5. Verify update
        response = client.get(f"/api/workspace/file/{test_file}")
        assert response.status_code == 200
        assert "Updated!" in response.json()["content"]
        
        # 6. Delete the file
        response = client.delete(f"/api/workspace/file/{test_file}")
        assert response.status_code == 200
        
        # 7. Verify deletion
        response = client.get(f"/api/workspace/file/{test_file}")
        assert response.status_code == 404
    
    def test_data_analysis_workflow(self, client: TestClient):
        """Test complete data analysis workflow"""
        # 1. Check data health
        response = client.get("/api/data/health")
        assert response.status_code == 200
        
        # 2. Get available data summary
        response = client.get("/api/data/summary")
        assert response.status_code == 200
        
        # 3. Get crypto data
        response = client.get("/api/crypto-data/BTC-USD?limit=100")
        assert response.status_code == 200
        data = response.json()
        
        # 4. If data exists, analyze it
        if data["count"] > 0:
            # Get statistics
            response = client.get("/api/analysis/statistics/BTC-USD")
            assert response.status_code == 200
            
            # Get risk metrics
            response = client.get("/api/analysis/risk-metrics/BTC-USD")
            assert response.status_code == 200
            
            # Get rolling stats
            response = client.get("/api/analysis/rolling-stats/BTC-USD?window=10")
            assert response.status_code == 200
    
    @pytest.mark.asyncio
    @pytest.mark.websocket
    @pytest.mark.skip(reason="Terminal WebSocket endpoint not yet implemented")
    async def test_terminal_websocket_workflow(self):
        """Test complete terminal WebSocket workflow"""
        # Connect to WebSocket
        uri = "ws://localhost:8080/api/terminal/ws"
        
        async with websockets.connect(uri) as ws:
            # 1. Wait for session creation
            msg = await asyncio.wait_for(ws.recv(), timeout=2)
            data = json.loads(msg)
            assert data["type"] == "session_created"
            session_id = data["session_id"]
            
            # 2. Send a command
            await ws.send(json.dumps({
                "type": "input",
                "data": "echo 'Testing terminal'\n"
            }))
            
            # 3. Read output
            output_received = False
            for _ in range(5):
                try:
                    msg = await asyncio.wait_for(ws.recv(), timeout=1)
                    data = json.loads(msg)
                    if data["type"] == "output" and "Testing terminal" in data["data"]:
                        output_received = True
                        break
                except asyncio.TimeoutError:
                    continue
            
            assert output_received, "Did not receive expected output"
            
            # 4. Test resize
            await ws.send(json.dumps({
                "type": "resize",
                "cols": 100,
                "rows": 40
            }))
            
            # 5. Send exit
            await ws.send(json.dumps({
                "type": "input",
                "data": "exit\n"
            }))
    
    def test_complete_trading_workflow(self, client: TestClient):
        """Test a complete trading analysis workflow"""
        # This would be a full workflow including:
        # 1. Authentication
        # 2. Data retrieval
        # 3. Analysis
        # 4. Strategy backtesting
        # 5. Results storage
        
        # 1. Authenticate
        response = client.post("/api/auth/demo-login")
        assert response.status_code == 200
        token = response.json()["access_token"]
        headers = {"Authorization": f"Bearer {token}"}
        
        # 2. Check available data
        response = client.get("/api/data/summary")
        assert response.status_code == 200
        
        # 3. Get market data
        response = client.get("/api/crypto-data/BTC-USD?limit=500")
        assert response.status_code == 200
        
        # 4. Run analysis
        if response.json()["count"] > 100:
            # Get statistics
            response = client.get("/api/analysis/statistics/BTC-USD", headers=headers)
            assert response.status_code == 200
            
            # Run backtest
            response = client.post("/api/analysis/backtest", json={
                "symbol": "BTC/USD",
                "exchange": "coinbase",
                "type": "simple_ma_cross",
                "fast_period": 5,
                "slow_period": 20
            }, headers=headers)
            assert response.status_code == 200
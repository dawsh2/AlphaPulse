"""
Unit tests for notebook/Jupyter endpoints
"""
import pytest
from fastapi.testclient import TestClient

@pytest.mark.unit
class TestNotebook:
    """Test notebook endpoints"""
    
    def test_list_kernels(self, client: TestClient):
        """Test listing Jupyter kernels"""
        response = client.get("/api/notebook/kernels")
        assert response.status_code == 200
        data = response.json()
        assert "kernels" in data
        assert isinstance(data["kernels"], list)
    
    def test_start_kernel(self, client: TestClient):
        """Test starting a new kernel"""
        response = client.post("/api/notebook/kernels")
        assert response.status_code == 200
        data = response.json()
        assert "kernel" in data
        assert data["kernel"]["status"] == "running"
    
    def test_stop_kernel(self, client: TestClient):
        """Test stopping a kernel"""
        # First start a kernel
        response = client.post("/api/notebook/kernels")
        assert response.status_code == 200
        
        # Then stop it
        response = client.delete("/api/notebook/kernels/default")
        assert response.status_code == 200
        data = response.json()
        assert data["status"] == "stopped"
    
    def test_execute_code(self, client: TestClient, sample_code: str):
        """Test executing code in Jupyter kernel"""
        response = client.post("/api/notebook/execute", json={
            "code": sample_code
        })
        assert response.status_code == 200
        data = response.json()
        assert data["status"] == "ok"
        assert data["output"] == "2\n"
        assert data["error"] is None
    
    def test_execute_code_with_error(self, client: TestClient):
        """Test executing code that produces an error"""
        response = client.post("/api/notebook/execute", json={
            "code": "1/0"  # Division by zero
        })
        assert response.status_code == 200
        data = response.json()
        assert data["status"] == "error"
        assert data["error"] is not None
        assert "ZeroDivisionError" in data["error"]
    
    def test_list_templates(self, client: TestClient):
        """Test listing notebook templates"""
        response = client.get("/api/notebook/templates")
        assert response.status_code == 200
        data = response.json()
        assert "templates" in data
        assert isinstance(data["templates"], list)
        if len(data["templates"]) > 0:
            template = data["templates"][0]
            assert "id" in template
            assert "title" in template
            assert "description" in template
    
    def test_get_template(self, client: TestClient):
        """Test getting a specific template"""
        # First list templates
        response = client.get("/api/notebook/templates")
        templates = response.json()["templates"]
        
        if len(templates) > 0:
            template_id = templates[0]["id"]
            response = client.get(f"/api/notebook/templates/{template_id}")
            assert response.status_code == 200
            data = response.json()
            assert "title" in data
            assert "cells" in data
    
    def test_notebook_health(self, client: TestClient):
        """Test notebook service health"""
        response = client.get("/api/notebook/health")
        assert response.status_code == 200
        data = response.json()
        assert data["service"] == "notebook"
        assert "kernel_running" in data
        assert "cleanup_thread_active" in data
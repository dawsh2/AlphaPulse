"""
Notebook API Routes for FastAPI - Handle Jupyter notebook execution requests
"""
from fastapi import APIRouter, HTTPException, Depends, BackgroundTasks
from pydantic import BaseModel
from typing import Optional, List, Dict, Any
import threading
import time
import logging
from analytics.jupyter_service import JupyterService
from analytics.template_service import (
    load_arbitrage_template, 
    load_tick_arbitrage_template, 
    load_trade_data_template, 
    load_convergence_template, 
    load_kraken_signal_template, 
    get_available_templates
)

# Setup logging
logger = logging.getLogger(__name__)

# Create router
router = APIRouter(
    prefix="/api/notebook",
    tags=["notebook"],
    responses={404: {"description": "Not found"}},
)

# Global Jupyter service instance (single kernel for now)
jupyter_service = None
cleanup_thread = None
service_lock = threading.Lock()

# Pydantic models for request/response
class ExecuteCodeRequest(BaseModel):
    code: str

class ExecuteCodeResponse(BaseModel):
    output: Optional[str]
    error: Optional[str]
    execution_count: Optional[int] = None
    status: str = "ok"  # Add status field for compatibility

class KernelStatus(BaseModel):
    status: str
    kernel: Optional[str]
    idle_seconds: Optional[int] = None
    idle_timeout: int = 300
    will_cleanup_at: Optional[int] = None

class StatusResponse(BaseModel):
    status: str
    message: Optional[str] = None
    error: Optional[str] = None

class TemplateInfo(BaseModel):
    id: str
    title: str  # Changed from 'name' to match actual data
    description: str
    category: Optional[str] = None  # Made optional since not all templates have it

class TemplatesResponse(BaseModel):
    templates: List[TemplateInfo]

# Helper functions
def get_jupyter_service() -> JupyterService:
    """Get or create the Jupyter service instance"""
    global jupyter_service
    
    with service_lock:
        if jupyter_service is None:
            jupyter_service = JupyterService()
            if not jupyter_service.start_kernel():
                raise HTTPException(status_code=500, detail="Failed to start Jupyter kernel")
            # Start cleanup thread if not running
            start_cleanup_thread()
        # Check if kernel is idle and cleanup if needed
        elif jupyter_service.is_idle(idle_timeout=300):  # 5 minutes idle timeout
            logger.info("Kernel has been idle for 5 minutes, restarting...")
            jupyter_service.shutdown_kernel()
            jupyter_service = JupyterService()
            if not jupyter_service.start_kernel():
                raise HTTPException(status_code=500, detail="Failed to restart idle kernel")
        
        return jupyter_service

def cleanup_idle_kernels():
    """Background thread to cleanup idle kernels
    Note: This runs in a separate thread, not async context, so time.sleep is appropriate
    """
    global jupyter_service
    while True:
        time.sleep(60)  # Check every minute - OK in thread context
        with service_lock:
            if jupyter_service and jupyter_service.is_idle(idle_timeout=300):
                logger.info("Auto-cleaning idle kernel...")
                jupyter_service.shutdown_kernel()
                jupyter_service = None

def start_cleanup_thread():
    """Start the cleanup thread if not already running"""
    global cleanup_thread
    if cleanup_thread is None or not cleanup_thread.is_alive():
        cleanup_thread = threading.Thread(target=cleanup_idle_kernels, daemon=True)
        cleanup_thread.start()
        logger.info("Started kernel cleanup thread")

# API Endpoints
@router.post("/execute", response_model=ExecuteCodeResponse)
async def execute_code(request: ExecuteCodeRequest):
    """
    Execute Python code in Jupyter kernel
    
    Request body:
    {
        "code": "print('hello')"
    }
    
    Response:
    {
        "output": "hello",
        "error": null,
        "execution_count": 1
    }
    """
    try:
        # Get Jupyter service
        service = get_jupyter_service()
        
        # Execute code
        result = service.execute_code(request.code)
        
        return ExecuteCodeResponse(
            output=result.get('output'),
            error=result.get('error'),
            execution_count=result.get('execution_count'),
            status="ok" if not result.get('error') else "error"
        )
        
    except HTTPException:
        raise
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))

@router.get("/status", response_model=KernelStatus)
async def kernel_status():
    """Check if kernel is running"""
    global jupyter_service
    
    try:
        with service_lock:
            if jupyter_service and jupyter_service.kernel_client:
                idle_time = None
                if jupyter_service.last_activity:
                    idle_time = int(time.time() - jupyter_service.last_activity)
                
                return KernelStatus(
                    status='running',
                    kernel='python3',
                    idle_seconds=idle_time,
                    idle_timeout=300,
                    will_cleanup_at=idle_time + (300 - idle_time) if idle_time else None
                )
            else:
                return KernelStatus(
                    status='stopped',
                    kernel=None
                )
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))

@router.post("/restart", response_model=StatusResponse)
async def restart_kernel():
    """Restart the Jupyter kernel"""
    global jupyter_service
    
    try:
        with service_lock:
            if jupyter_service:
                jupyter_service.shutdown_kernel()
            
            jupyter_service = JupyterService()
            if jupyter_service.start_kernel():
                return StatusResponse(status='restarted', message='Kernel restarted successfully')
            else:
                raise HTTPException(status_code=500, detail='Failed to restart kernel')
                
    except HTTPException:
        raise
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))

@router.post("/cleanup", response_model=StatusResponse)
async def cleanup_kernel():
    """Manually cleanup/shutdown the kernel to release resources"""
    global jupyter_service
    
    try:
        with service_lock:
            if jupyter_service:
                jupyter_service.shutdown_kernel()
                jupyter_service = None
                return StatusResponse(
                    status='cleaned', 
                    message='Kernel shutdown and resources released'
                )
            else:
                return StatusResponse(
                    status='not_running', 
                    message='No kernel to cleanup'
                )
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))

@router.get("/templates", response_model=TemplatesResponse)
async def get_templates():
    """Get available notebook templates"""
    try:
        templates = get_available_templates()
        return TemplatesResponse(templates=templates)
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))

@router.get("/templates/{template_id}")
async def get_template(template_id: str):
    """Get a specific template"""
    try:
        template_loaders = {
            'arbitrage_basic': load_arbitrage_template,
            'tick_arbitrage': load_tick_arbitrage_template,
            'trade_data_analysis': load_trade_data_template,
            'convergence_trading': load_convergence_template,
            'kraken_signal_trading': load_kraken_signal_template,
        }
        
        if template_id in template_loaders:
            template = template_loaders[template_id]()
            return template
        else:
            raise HTTPException(status_code=404, detail='Template not found')
            
    except HTTPException:
        raise
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))

# Kernel management endpoints
@router.get("/kernels")
async def list_kernels():
    """List available Jupyter kernels"""
    global jupyter_service
    
    with service_lock:
        if jupyter_service and jupyter_service.kernel_client:
            return {
                "kernels": [{
                    "id": "default",
                    "name": "python3",
                    "status": "running",
                    "last_activity": jupyter_service.last_execution_time
                }]
            }
        else:
            return {"kernels": []}

@router.post("/kernels")
async def start_kernel():
    """Start a new Jupyter kernel"""
    service = get_jupyter_service()
    
    return {
        "kernel": {
            "id": "default",
            "name": "python3",
            "status": "running"
        }
    }

@router.delete("/kernels/{kernel_id}")
async def stop_kernel(kernel_id: str):
    """Stop a Jupyter kernel"""
    global jupyter_service
    
    with service_lock:
        if jupyter_service:
            jupyter_service.shutdown_kernel()
            jupyter_service = None
    
    return {"status": "stopped", "kernel_id": kernel_id}

# Additional helper endpoints
@router.get("/health")
async def notebook_health():
    """Health check for notebook service"""
    global jupyter_service
    
    with service_lock:
        kernel_running = jupyter_service is not None and jupyter_service.kernel_client is not None
    
    return {
        "service": "notebook",
        "kernel_running": kernel_running,
        "cleanup_thread_active": cleanup_thread is not None and cleanup_thread.is_alive()
    }
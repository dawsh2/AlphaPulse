"""
Jupyter Service - Manages Jupyter kernel for notebook execution
"""
import asyncio
import json
import atexit
import weakref
from jupyter_client import KernelManager
from queue import Empty
import sys
import time


class JupyterService:
    """Service to manage Jupyter kernel and execute code"""
    
    # Class variable to track all instances
    _instances = weakref.WeakSet()
    
    def __init__(self):
        self.kernel_manager = None
        self.kernel_client = None
        self.last_activity = None
        # Track this instance
        JupyterService._instances.add(self)
    
    def _setup_readonly_environment(self):
        """Setup the notebook environment to use read-only DuckDB connections"""
        setup_code = '''
import duckdb
import os

# Override duckdb.connect to default to read_only=True for analysis notebooks
_original_connect = duckdb.connect

def safe_duckdb_connect(database=':memory:', read_only=None, **kwargs):
    """DuckDB connect wrapper that forces read-only mode for market data"""
    # Always force read_only=True for market data database
    if 'market_data.duckdb' in str(database):
        read_only = True
        print(f"🔒 Forcing read-only mode for market_data.duckdb")
    elif read_only is None:
        # Default to read-only for safety
        read_only = True
    return _original_connect(database, read_only=read_only, **kwargs)

# Replace the connect function globally
duckdb.connect = safe_duckdb_connect

print("📊 Notebook environment configured with read-only DuckDB connections")
'''
        
        try:
            self.execute_code(setup_code, timeout=5)
        except Exception as e:
            print(f"Warning: Could not setup read-only environment: {e}")
        
    def start_kernel(self):
        """Start a new Jupyter kernel"""
        try:
            # Create kernel manager
            self.kernel_manager = KernelManager(kernel_name='python3')
            self.kernel_manager.start_kernel()
            
            # Create client to communicate with kernel
            self.kernel_client = self.kernel_manager.client()
            self.kernel_client.start_channels()
            
            # Wait for kernel to be ready
            self.kernel_client.wait_for_ready(timeout=10)
            
            # Set initial activity time
            self.last_activity = time.time()
            
            # Setup notebook environment with read-only DuckDB connections
            self._setup_readonly_environment()
            
            print("✅ Jupyter kernel started successfully")
            return True
            
        except Exception as e:
            print(f"❌ Error starting kernel: {e}")
            return False
    
    def execute_code(self, code: str, timeout: int = 10):
        """Execute code in the kernel and return output"""
        if not self.kernel_client:
            return {"error": "Kernel not started"}
        
        # Update activity time
        self.last_activity = time.time()
        
        try:
            # Execute the code
            msg_id = self.kernel_client.execute(code)
            
            # Collect output
            output = []
            errors = []
            images = []
            
            # Wait for execution to complete
            while True:
                try:
                    # Get messages from the kernel
                    msg = self.kernel_client.get_iopub_msg(timeout=timeout)
                    
                    # Process different message types
                    if msg['msg_type'] == 'stream':
                        output.append(msg['content']['text'])
                    elif msg['msg_type'] == 'execute_result':
                        data = msg['content']['data']
                        output.append(data.get('text/plain', ''))
                        # Check for image data
                        if 'image/png' in data:
                            images.append(data['image/png'])
                    elif msg['msg_type'] == 'display_data':
                        data = msg['content']['data']
                        if 'image/png' in data:
                            images.append(data['image/png'])
                        if 'text/plain' in data:
                            output.append(data['text/plain'])
                    elif msg['msg_type'] == 'error':
                        errors.append('\n'.join(msg['content']['traceback']))
                    elif msg['msg_type'] == 'status':
                        if msg['content']['execution_state'] == 'idle':
                            break
                            
                except Empty:
                    break
                except Exception as e:
                    errors.append(f"Error getting message: {str(e)}")
                    break
            
            return {
                "output": ''.join(output) if output else None,
                "error": '\n'.join(errors) if errors else None,
                "images": images if images else None
            }
            
        except Exception as e:
            return {"error": f"Execution error: {str(e)}"}
    
    def shutdown_kernel(self):
        """Shutdown the kernel"""
        if self.kernel_manager:
            try:
                if self.kernel_client:
                    self.kernel_client.stop_channels()
                self.kernel_manager.shutdown_kernel(now=True)
                self.kernel_manager = None
                self.kernel_client = None
                print("Kernel shutdown complete")
            except Exception as e:
                print(f"Error shutting down kernel: {e}")
    
    def is_idle(self, idle_timeout: int = 300):
        """Check if kernel has been idle for more than idle_timeout seconds"""
        if self.last_activity is None:
            return False
        return (time.time() - self.last_activity) > idle_timeout
    
    @classmethod
    def cleanup_all_kernels(cls):
        """Cleanup all kernel instances"""
        for instance in list(cls._instances):
            instance.shutdown_kernel()


# Register cleanup on exit
atexit.register(JupyterService.cleanup_all_kernels)

# Simple test when run directly
if __name__ == "__main__":
    print("Starting Jupyter Service test...")
    
    service = JupyterService()
    
    # Start kernel
    if service.start_kernel():
        print("Testing kernel with simple calculation...")
        
        # Test 1: Simple math
        result = service.execute_code("1 + 1")
        print(f"Test 1 (1+1): {result}")
        
        # Test 2: Print statement
        result = service.execute_code("print('Hello from Jupyter')")
        print(f"Test 2 (print): {result}")
        
        # Test 3: Import
        result = service.execute_code("import sys; print(sys.version)")
        print(f"Test 3 (import): {result}")
        
        # Shutdown
        service.shutdown_kernel()
    else:
        print("Failed to start kernel")
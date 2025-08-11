"""
Jupyter Service - Manages Jupyter kernel for notebook execution
"""
import asyncio
import json
from jupyter_client import KernelManager
from queue import Empty
import sys


class JupyterService:
    """Service to manage Jupyter kernel and execute code"""
    
    def __init__(self):
        self.kernel_manager = None
        self.kernel_client = None
        
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
            
            print("✅ Jupyter kernel started successfully")
            return True
            
        except Exception as e:
            print(f"❌ Error starting kernel: {e}")
            return False
    
    def execute_code(self, code: str, timeout: int = 10):
        """Execute code in the kernel and return output"""
        if not self.kernel_client:
            return {"error": "Kernel not started"}
        
        try:
            # Execute the code
            msg_id = self.kernel_client.execute(code)
            
            # Collect output
            output = []
            errors = []
            
            # Wait for execution to complete
            while True:
                try:
                    # Get messages from the kernel
                    msg = self.kernel_client.get_iopub_msg(timeout=timeout)
                    
                    # Process different message types
                    if msg['msg_type'] == 'stream':
                        output.append(msg['content']['text'])
                    elif msg['msg_type'] == 'execute_result':
                        output.append(msg['content']['data'].get('text/plain', ''))
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
                "error": '\n'.join(errors) if errors else None
            }
            
        except Exception as e:
            return {"error": f"Execution error: {str(e)}"}
    
    def shutdown_kernel(self):
        """Shutdown the kernel"""
        if self.kernel_manager:
            self.kernel_client.stop_channels()
            self.kernel_manager.shutdown_kernel()
            print("Kernel shutdown complete")


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
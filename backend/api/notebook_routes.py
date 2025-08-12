"""
Notebook API Routes - Handle Jupyter notebook execution requests
"""
from flask import Blueprint, request, jsonify, make_response
from services.jupyter_service import JupyterService
from services.template_service import load_arbitrage_template, get_available_templates
import threading
import time

notebook_bp = Blueprint('notebook', __name__, url_prefix='/api/notebook')

# Global Jupyter service instance (single kernel for now)
jupyter_service = None
cleanup_thread = None


def get_jupyter_service():
    """Get or create the Jupyter service instance"""
    global jupyter_service
    if jupyter_service is None:
        jupyter_service = JupyterService()
        if not jupyter_service.start_kernel():
            raise Exception("Failed to start Jupyter kernel")
        # Start cleanup thread if not running
        start_cleanup_thread()
    # Check if kernel is idle and cleanup if needed
    elif jupyter_service.is_idle(idle_timeout=300):  # 5 minutes idle timeout
        print("Kernel has been idle for 5 minutes, restarting...")
        jupyter_service.shutdown_kernel()
        jupyter_service = JupyterService()
        if not jupyter_service.start_kernel():
            raise Exception("Failed to restart idle kernel")
    return jupyter_service


def cleanup_idle_kernels():
    """Background thread to cleanup idle kernels"""
    global jupyter_service
    while True:
        time.sleep(60)  # Check every minute
        if jupyter_service and jupyter_service.is_idle(idle_timeout=300):
            print("Auto-cleaning idle kernel...")
            jupyter_service.shutdown_kernel()
            jupyter_service = None


def start_cleanup_thread():
    """Start the cleanup thread if not already running"""
    global cleanup_thread
    if cleanup_thread is None or not cleanup_thread.is_alive():
        cleanup_thread = threading.Thread(target=cleanup_idle_kernels, daemon=True)
        cleanup_thread.start()
        print("Started kernel cleanup thread")


@notebook_bp.route('/execute', methods=['POST', 'OPTIONS'])
def execute_code():
    # Handle preflight OPTIONS request
    if request.method == 'OPTIONS':
        response = make_response('', 200)
        response.headers['Access-Control-Allow-Origin'] = '*'
        response.headers['Access-Control-Allow-Methods'] = 'POST, OPTIONS'
        response.headers['Access-Control-Allow-Headers'] = 'Content-Type'
        return response
    """
    Execute Python code in Jupyter kernel
    
    Request body:
    {
        "code": "print('hello')"
    }
    
    Response:
    {
        "output": "hello",
        "error": null
    }
    """
    try:
        data = request.get_json()
        
        if not data or 'code' not in data:
            return jsonify({
                'error': 'No code provided',
                'output': None
            }), 400
        
        code = data['code']
        
        # Get Jupyter service
        service = get_jupyter_service()
        
        # Execute code
        result = service.execute_code(code)
        
        # Create response with explicit CORS headers
        response = make_response(jsonify(result))
        response.headers['Access-Control-Allow-Origin'] = '*'
        response.headers['Access-Control-Allow-Methods'] = 'POST, OPTIONS'
        response.headers['Access-Control-Allow-Headers'] = 'Content-Type'
        return response
        
    except Exception as e:
        response = make_response(jsonify({
            'error': str(e),
            'output': None
        }), 500)
        response.headers['Access-Control-Allow-Origin'] = '*'
        response.headers['Access-Control-Allow-Methods'] = 'POST, OPTIONS'
        response.headers['Access-Control-Allow-Headers'] = 'Content-Type'
        return response


@notebook_bp.route('/status', methods=['GET'])
def kernel_status():
    """Check if kernel is running"""
    global jupyter_service
    try:
        if jupyter_service and jupyter_service.kernel_client:
            idle_time = None
            if jupyter_service.last_activity:
                idle_time = int(time.time() - jupyter_service.last_activity)
            
            return jsonify({
                'status': 'running',
                'kernel': 'python3',
                'idle_seconds': idle_time,
                'idle_timeout': 300,
                'will_cleanup_at': idle_time + (300 - idle_time) if idle_time else None
            })
        else:
            return jsonify({
                'status': 'stopped',
                'kernel': None
            })
    except Exception as e:
        return jsonify({
            'status': 'error',
            'error': str(e)
        }), 500


@notebook_bp.route('/restart', methods=['POST'])
def restart_kernel():
    """Restart the Jupyter kernel"""
    global jupyter_service
    try:
        if jupyter_service:
            jupyter_service.shutdown_kernel()
        
        jupyter_service = JupyterService()
        if jupyter_service.start_kernel():
            return jsonify({'status': 'restarted'})
        else:
            return jsonify({'error': 'Failed to restart kernel'}), 500
            
    except Exception as e:
        return jsonify({'error': str(e)}), 500


@notebook_bp.route('/cleanup', methods=['POST'])
def cleanup_kernel():
    """Manually cleanup/shutdown the kernel to release resources"""
    global jupyter_service
    try:
        if jupyter_service:
            jupyter_service.shutdown_kernel()
            jupyter_service = None
            return jsonify({'status': 'cleaned', 'message': 'Kernel shutdown and resources released'})
        else:
            return jsonify({'status': 'not_running', 'message': 'No kernel to cleanup'})
    except Exception as e:
        return jsonify({'error': str(e)}), 500


@notebook_bp.route('/templates', methods=['GET'])
def get_templates():
    """Get available notebook templates"""
    try:
        templates = get_available_templates()
        response = make_response(jsonify({'templates': templates}))
        response.headers['Access-Control-Allow-Origin'] = '*'
        return response
    except Exception as e:
        return jsonify({'error': str(e)}), 500


@notebook_bp.route('/templates/<template_id>', methods=['GET'])
def get_template(template_id):
    """Get a specific template"""
    try:
        if template_id == 'arbitrage_basic':
            template = load_arbitrage_template()
            response = make_response(jsonify(template))
            response.headers['Access-Control-Allow-Origin'] = '*'
            return response
        else:
            return jsonify({'error': 'Template not found'}), 404
    except Exception as e:
        return jsonify({'error': str(e)}), 500
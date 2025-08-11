"""
Notebook API Routes - Handle Jupyter notebook execution requests
"""
from flask import Blueprint, request, jsonify, make_response
from services.jupyter_service import JupyterService

notebook_bp = Blueprint('notebook', __name__, url_prefix='/api/notebook')

# Global Jupyter service instance (single kernel for now)
jupyter_service = None


def get_jupyter_service():
    """Get or create the Jupyter service instance"""
    global jupyter_service
    if jupyter_service is None:
        jupyter_service = JupyterService()
        if not jupyter_service.start_kernel():
            raise Exception("Failed to start Jupyter kernel")
    return jupyter_service


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
    try:
        service = get_jupyter_service()
        return jsonify({
            'status': 'running' if service.kernel_client else 'stopped',
            'kernel': 'python3'
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
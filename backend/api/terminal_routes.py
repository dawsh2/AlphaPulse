"""
Terminal API Routes - Secure command execution for the development environment
Provides controlled shell command execution with output streaming
"""
import os
import subprocess
import json
import uuid
import threading
import queue
import time
from pathlib import Path
from typing import Dict, Any, Optional
from flask import Blueprint, request, jsonify, make_response, Response

# Create blueprint
terminal_api = Blueprint('terminal_api', __name__)

# Configure workspace directory (same as workspace_routes)
WORKSPACE_ROOT = Path(__file__).parent.parent / "workspace"

# Active terminal sessions
terminal_sessions: Dict[str, Dict[str, Any]] = {}

# Security: Whitelist of allowed commands
ALLOWED_COMMANDS = {
    # Python and package management
    'python', 'python3', 'pip', 'pip3',
    # File operations
    'ls', 'dir', 'pwd', 'cd', 'cat', 'head', 'tail', 'grep', 'find',
    'echo', 'touch', 'mkdir', 'rm', 'cp', 'mv',
    # Git
    'git',
    # Data tools
    'jupyter', 'ipython',
    # System info
    'which', 'whoami', 'date', 'ps', 'top',
    # Text processing
    'awk', 'sed', 'sort', 'uniq', 'wc',
}

# Security: Forbidden command patterns
FORBIDDEN_PATTERNS = [
    'sudo', 'su', 'chmod', 'chown', 'shutdown', 'reboot',
    'systemctl', 'service', 'kill', 'pkill', 'killall',
    'curl', 'wget', 'nc', 'netcat', 'ssh', 'scp',
    'eval', 'exec', '__import__',
    '&&', '||', '|', '>', '<', '>>', '`', '$(',
]


def add_cors_headers(response):
    """Add CORS headers to response"""
    response.headers['Access-Control-Allow-Origin'] = 'http://localhost:5173'
    response.headers['Access-Control-Allow-Methods'] = 'GET, POST, OPTIONS'
    response.headers['Access-Control-Allow-Headers'] = 'Content-Type'
    return response


def is_command_safe(command: str) -> bool:
    """Check if a command is safe to execute"""
    # Split command to get the base command
    parts = command.strip().split()
    if not parts:
        return False
    
    base_command = parts[0]
    
    # Check if base command is in whitelist
    if base_command not in ALLOWED_COMMANDS:
        # Allow running Python files directly
        if base_command.endswith('.py') and os.path.exists(WORKSPACE_ROOT / base_command):
            parts[0] = 'python'
            parts.insert(1, base_command)
            return is_command_safe(' '.join(parts))
        return False
    
    # Check for forbidden patterns
    command_lower = command.lower()
    for pattern in FORBIDDEN_PATTERNS:
        if pattern in command_lower:
            return False
    
    return True


@terminal_api.route('/api/terminal/execute', methods=['POST', 'OPTIONS'])
def execute_command():
    """Execute a shell command in the workspace directory"""
    if request.method == 'OPTIONS':
        return add_cors_headers(make_response()), 200
    
    try:
        data = request.get_json()
        command = data.get('command', '').strip()
        session_id = data.get('session_id', str(uuid.uuid4()))
        
        if not command:
            return jsonify({'error': 'No command provided'}), 400
        
        # Security check
        if not is_command_safe(command):
            return jsonify({
                'error': f'Command not allowed: {command}',
                'output': f'Error: Command "{command}" is not allowed for security reasons.\n',
                'session_id': session_id
            }), 403
        
        # Special handling for 'cd' command
        if command.startswith('cd '):
            path = command[3:].strip()
            if path.startswith('/'):
                return jsonify({
                    'error': 'Absolute paths not allowed',
                    'output': 'Error: Cannot change to absolute path. Use relative paths only.\n',
                    'session_id': session_id
                }), 403
            
            # Store the working directory in session
            if session_id not in terminal_sessions:
                terminal_sessions[session_id] = {'cwd': str(WORKSPACE_ROOT)}
            
            new_path = Path(terminal_sessions[session_id]['cwd']) / path
            if new_path.exists() and new_path.is_dir():
                # Ensure new path is still within workspace
                try:
                    new_path = new_path.resolve()
                    if str(new_path).startswith(str(WORKSPACE_ROOT.resolve())):
                        terminal_sessions[session_id]['cwd'] = str(new_path)
                        return jsonify({
                            'output': '',
                            'session_id': session_id,
                            'cwd': str(new_path.relative_to(WORKSPACE_ROOT))
                        }), 200
                except:
                    pass
            
            return jsonify({
                'error': 'Invalid directory',
                'output': f'cd: {path}: No such file or directory\n',
                'session_id': session_id
            }), 400
        
        # Get working directory for session
        cwd = WORKSPACE_ROOT
        if session_id in terminal_sessions:
            session_cwd = terminal_sessions[session_id].get('cwd')
            if session_cwd:
                cwd = Path(session_cwd)
        
        # Execute the command
        try:
            # Prepend python3 for .py files if needed
            if command.endswith('.py') and not command.startswith('python'):
                command = f'python3 {command}'
            
            result = subprocess.run(
                command,
                shell=True,
                cwd=str(cwd),
                capture_output=True,
                text=True,
                timeout=30,  # 30 second timeout
                env={**os.environ, 'PYTHONPATH': str(WORKSPACE_ROOT)}
            )
            
            output = result.stdout
            if result.stderr:
                output += '\n' + result.stderr
            
            response = jsonify({
                'output': output,
                'exit_code': result.returncode,
                'session_id': session_id,
                'cwd': str(cwd.relative_to(WORKSPACE_ROOT)) if cwd != WORKSPACE_ROOT else '/'
            })
            return add_cors_headers(response), 200
            
        except subprocess.TimeoutExpired:
            return jsonify({
                'error': 'Command timed out',
                'output': 'Error: Command execution timed out after 30 seconds\n',
                'session_id': session_id
            }), 408
        except Exception as e:
            return jsonify({
                'error': str(e),
                'output': f'Error executing command: {str(e)}\n',
                'session_id': session_id
            }), 500
            
    except Exception as e:
        return jsonify({'error': str(e)}), 500


@terminal_api.route('/api/terminal/sessions', methods=['GET', 'OPTIONS'])
def get_sessions():
    """Get list of active terminal sessions"""
    if request.method == 'OPTIONS':
        return add_cors_headers(make_response()), 200
    
    sessions = []
    for session_id, session_data in terminal_sessions.items():
        sessions.append({
            'id': session_id,
            'cwd': session_data.get('cwd', str(WORKSPACE_ROOT))
        })
    
    response = jsonify({'sessions': sessions})
    return add_cors_headers(response), 200


@terminal_api.route('/api/terminal/session/<session_id>', methods=['DELETE', 'OPTIONS'])
def close_session(session_id):
    """Close a terminal session"""
    if request.method == 'OPTIONS':
        return add_cors_headers(make_response()), 200
    
    if session_id in terminal_sessions:
        del terminal_sessions[session_id]
    
    response = jsonify({'success': True})
    return add_cors_headers(response), 200


@terminal_api.route('/api/terminal/autocomplete', methods=['POST', 'OPTIONS'])
def autocomplete():
    """Provide command autocomplete suggestions"""
    if request.method == 'OPTIONS':
        return add_cors_headers(make_response()), 200
    
    try:
        data = request.get_json()
        partial_command = data.get('partial', '')
        session_id = data.get('session_id')
        
        suggestions = []
        
        # Get working directory
        cwd = WORKSPACE_ROOT
        if session_id and session_id in terminal_sessions:
            session_cwd = terminal_sessions[session_id].get('cwd')
            if session_cwd:
                cwd = Path(session_cwd)
        
        # Split command to determine context
        parts = partial_command.split()
        
        if len(parts) <= 1:
            # Suggest commands
            prefix = parts[0] if parts else ''
            suggestions = [cmd for cmd in ALLOWED_COMMANDS if cmd.startswith(prefix)]
            
            # Also suggest Python files in current directory
            if cwd.exists():
                for file in cwd.glob('*.py'):
                    if file.name.startswith(prefix):
                        suggestions.append(file.name)
        else:
            # Suggest files/directories
            base_command = parts[0]
            partial_path = parts[-1] if len(parts) > 1 else ''
            
            # Determine search directory
            if '/' in partial_path:
                dir_path = cwd / '/'.join(partial_path.split('/')[:-1])
                prefix = partial_path.split('/')[-1]
            else:
                dir_path = cwd
                prefix = partial_path
            
            if dir_path.exists() and dir_path.is_dir():
                for item in dir_path.iterdir():
                    if item.name.startswith(prefix):
                        name = item.name
                        if item.is_dir():
                            name += '/'
                        
                        # Build full suggestion
                        if '/' in partial_path:
                            suggestion = '/'.join(partial_path.split('/')[:-1]) + '/' + name
                        else:
                            suggestion = name
                        
                        suggestions.append(suggestion)
        
        response = jsonify({
            'suggestions': suggestions[:20]  # Limit to 20 suggestions
        })
        return add_cors_headers(response), 200
        
    except Exception as e:
        return jsonify({'error': str(e)}), 500
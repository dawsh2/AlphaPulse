"""
Workspace API Routes - File system operations for the development environment
Provides secure file operations within a sandboxed workspace directory
"""
import os
import json
import subprocess
import shutil
from pathlib import Path
from typing import Dict, Any, List, Optional
from flask import Blueprint, request, jsonify, make_response
from werkzeug.utils import secure_filename
from datetime import datetime

# Create blueprint
workspace_api = Blueprint('workspace_api', __name__)

# Configure workspace directory (sandboxed area for user files)
WORKSPACE_ROOT = Path(__file__).parent.parent / "workspace"
WORKSPACE_ROOT.mkdir(exist_ok=True)

# Security: Allowed file extensions for editing
ALLOWED_EXTENSIONS = {
    '.py', '.json', '.yaml', '.yml', '.txt', '.md', '.csv', 
    '.ipynb', '.html', '.css', '.js', '.ts', '.jsx', '.tsx',
    '.sh', '.env', '.gitignore', '.dockerfile'
}

# Security: Restricted directories (cannot access)
RESTRICTED_PATHS = {'__pycache__', '.git', 'node_modules', '.env.local'}


def add_cors_headers(response):
    """Add CORS headers to response"""
    response.headers['Access-Control-Allow-Origin'] = 'http://localhost:5173'
    response.headers['Access-Control-Allow-Methods'] = 'GET, POST, PUT, DELETE, OPTIONS'
    response.headers['Access-Control-Allow-Headers'] = 'Content-Type'
    return response


def is_safe_path(path: Path) -> bool:
    """Check if a path is safe to access"""
    try:
        # Resolve to absolute path and check if it's within workspace
        resolved = path.resolve()
        workspace_resolved = WORKSPACE_ROOT.resolve()
        
        # Check if path is within workspace
        if not str(resolved).startswith(str(workspace_resolved)):
            return False
        
        # Check for restricted directories
        for part in resolved.parts:
            if part in RESTRICTED_PATHS:
                return False
        
        return True
    except:
        return False


def get_file_info(file_path: Path) -> Dict[str, Any]:
    """Get file metadata"""
    stat = file_path.stat()
    return {
        'name': file_path.name,
        'path': str(file_path.relative_to(WORKSPACE_ROOT)),
        'type': 'folder' if file_path.is_dir() else 'file',
        'size': stat.st_size if file_path.is_file() else None,
        'modified': datetime.fromtimestamp(stat.st_mtime).isoformat(),
        'extension': file_path.suffix if file_path.is_file() else None
    }


@workspace_api.route('/api/workspace/files', methods=['GET', 'OPTIONS'])
def list_workspace_files():
    """List all files in the workspace directory"""
    if request.method == 'OPTIONS':
        return add_cors_headers(make_response()), 200
    
    try:
        path = request.args.get('path', '')
        target_path = WORKSPACE_ROOT / path
        
        if not is_safe_path(target_path):
            return jsonify({'error': 'Invalid path'}), 403
        
        if not target_path.exists():
            return jsonify({'error': 'Path not found'}), 404
        
        files = []
        if target_path.is_dir():
            for item in sorted(target_path.iterdir()):
                # Skip hidden files and restricted paths
                if item.name.startswith('.') and item.name not in ['.env', '.gitignore']:
                    continue
                if item.name in RESTRICTED_PATHS:
                    continue
                    
                file_info = get_file_info(item)
                
                # For directories, optionally include children count
                if item.is_dir():
                    try:
                        file_info['children_count'] = len(list(item.iterdir()))
                    except:
                        file_info['children_count'] = 0
                
                files.append(file_info)
        
        response = jsonify({
            'path': path,
            'files': files,
            'workspace_root': str(WORKSPACE_ROOT)
        })
        return add_cors_headers(response), 200
        
    except Exception as e:
        return jsonify({'error': str(e)}), 500


@workspace_api.route('/api/workspace/file/<path:filepath>', methods=['GET', 'OPTIONS'])
def get_file_content(filepath):
    """Get content of a specific file"""
    if request.method == 'OPTIONS':
        return add_cors_headers(make_response()), 200
    
    try:
        target_path = WORKSPACE_ROOT / filepath
        
        if not is_safe_path(target_path):
            return jsonify({'error': 'Invalid path'}), 403
        
        if not target_path.exists():
            return jsonify({'error': 'File not found'}), 404
        
        if target_path.is_dir():
            return jsonify({'error': 'Path is a directory'}), 400
        
        # Check file extension
        if target_path.suffix not in ALLOWED_EXTENSIONS:
            return jsonify({'error': f'File type {target_path.suffix} not allowed'}), 403
        
        # Read file content
        try:
            with open(target_path, 'r', encoding='utf-8') as f:
                content = f.read()
        except UnicodeDecodeError:
            # Try reading as binary for certain files
            with open(target_path, 'rb') as f:
                content = f.read().decode('utf-8', errors='replace')
        
        response = jsonify({
            'content': content,
            'path': filepath,
            'info': get_file_info(target_path)
        })
        return add_cors_headers(response), 200
        
    except Exception as e:
        return jsonify({'error': str(e)}), 500


@workspace_api.route('/api/workspace/file/<path:filepath>', methods=['PUT', 'OPTIONS'])
def update_file(filepath):
    """Update or create a file"""
    if request.method == 'OPTIONS':
        return add_cors_headers(make_response()), 200
    
    try:
        target_path = WORKSPACE_ROOT / filepath
        
        if not is_safe_path(target_path):
            return jsonify({'error': 'Invalid path'}), 403
        
        # Check file extension for new files
        if not target_path.exists() and target_path.suffix not in ALLOWED_EXTENSIONS:
            return jsonify({'error': f'File type {target_path.suffix} not allowed'}), 403
        
        data = request.get_json()
        content = data.get('content', '')
        
        # Create parent directories if needed
        target_path.parent.mkdir(parents=True, exist_ok=True)
        
        # Write file
        with open(target_path, 'w', encoding='utf-8') as f:
            f.write(content)
        
        response = jsonify({
            'success': True,
            'path': filepath,
            'info': get_file_info(target_path)
        })
        return add_cors_headers(response), 200
        
    except Exception as e:
        return jsonify({'error': str(e)}), 500


@workspace_api.route('/api/workspace/file/<path:filepath>', methods=['POST', 'OPTIONS'])
def create_file(filepath):
    """Create a new file or directory"""
    if request.method == 'OPTIONS':
        return add_cors_headers(make_response()), 200
    
    try:
        target_path = WORKSPACE_ROOT / filepath
        
        if not is_safe_path(target_path):
            return jsonify({'error': 'Invalid path'}), 403
        
        if target_path.exists():
            return jsonify({'error': 'File already exists'}), 409
        
        data = request.get_json()
        is_directory = data.get('is_directory', False)
        
        if is_directory:
            target_path.mkdir(parents=True, exist_ok=True)
        else:
            # Check file extension
            if target_path.suffix not in ALLOWED_EXTENSIONS:
                return jsonify({'error': f'File type {target_path.suffix} not allowed'}), 403
            
            # Create parent directories if needed
            target_path.parent.mkdir(parents=True, exist_ok=True)
            
            # Create empty file or with initial content
            content = data.get('content', '')
            with open(target_path, 'w', encoding='utf-8') as f:
                f.write(content)
        
        response = jsonify({
            'success': True,
            'path': filepath,
            'info': get_file_info(target_path)
        })
        return add_cors_headers(response), 200
        
    except Exception as e:
        return jsonify({'error': str(e)}), 500


@workspace_api.route('/api/workspace/file/<path:filepath>', methods=['DELETE', 'OPTIONS'])
def delete_file(filepath):
    """Delete a file or directory"""
    if request.method == 'OPTIONS':
        return add_cors_headers(make_response()), 200
    
    try:
        target_path = WORKSPACE_ROOT / filepath
        
        if not is_safe_path(target_path):
            return jsonify({'error': 'Invalid path'}), 403
        
        if not target_path.exists():
            return jsonify({'error': 'File not found'}), 404
        
        # Delete file or directory
        if target_path.is_dir():
            shutil.rmtree(target_path)
        else:
            target_path.unlink()
        
        response = jsonify({
            'success': True,
            'path': filepath
        })
        return add_cors_headers(response), 200
        
    except Exception as e:
        return jsonify({'error': str(e)}), 500


@workspace_api.route('/api/workspace/rename', methods=['POST', 'OPTIONS'])
def rename_file():
    """Rename a file or directory"""
    if request.method == 'OPTIONS':
        return add_cors_headers(make_response()), 200
    
    try:
        data = request.get_json()
        old_path = data.get('old_path')
        new_path = data.get('new_path')
        
        if not old_path or not new_path:
            return jsonify({'error': 'Missing paths'}), 400
        
        old_target = WORKSPACE_ROOT / old_path
        new_target = WORKSPACE_ROOT / new_path
        
        if not is_safe_path(old_target) or not is_safe_path(new_target):
            return jsonify({'error': 'Invalid path'}), 403
        
        if not old_target.exists():
            return jsonify({'error': 'Source file not found'}), 404
        
        if new_target.exists():
            return jsonify({'error': 'Destination already exists'}), 409
        
        # Rename/move the file
        old_target.rename(new_target)
        
        response = jsonify({
            'success': True,
            'old_path': old_path,
            'new_path': new_path,
            'info': get_file_info(new_target)
        })
        return add_cors_headers(response), 200
        
    except Exception as e:
        return jsonify({'error': str(e)}), 500


# Initialize workspace with sample files if empty
def init_workspace():
    """Initialize workspace with sample files if it's empty"""
    if not any(WORKSPACE_ROOT.iterdir()):
        # Create sample structure
        readme_path = WORKSPACE_ROOT / "README.md"
        readme_path.write_text("""# AlphaPulse Workspace

Welcome to your AlphaPulse development workspace!

This is your personal workspace for developing and testing trading strategies.

## Getting Started

1. Create new strategy files in the `strategies/` folder
2. Use the terminal to run backtests
3. Save your work - all files are persisted on the backend

## Sample Files

Check out the `examples/` folder for sample strategies to get started.
""")
        
        # Create directories
        (WORKSPACE_ROOT / "strategies").mkdir(exist_ok=True)
        (WORKSPACE_ROOT / "data").mkdir(exist_ok=True)
        (WORKSPACE_ROOT / "notebooks").mkdir(exist_ok=True)
        (WORKSPACE_ROOT / "examples").mkdir(exist_ok=True)
        
        # Create a sample strategy
        sample_strategy = WORKSPACE_ROOT / "examples" / "simple_ma_cross.py"
        sample_strategy.write_text("""\"\"\"
Simple Moving Average Crossover Strategy
A basic example strategy for AlphaPulse
\"\"\"

import numpy as np
import pandas as pd
from typing import Optional

class SimpleMAStrategy:
    def __init__(self, fast_period: int = 10, slow_period: int = 20):
        self.fast_period = fast_period
        self.slow_period = slow_period
        self.position = 0
        
    def calculate_signals(self, data: pd.DataFrame) -> pd.DataFrame:
        \"\"\"Calculate trading signals based on MA crossover\"\"\"
        # Calculate moving averages
        data['ma_fast'] = data['close'].rolling(self.fast_period).mean()
        data['ma_slow'] = data['close'].rolling(self.slow_period).mean()
        
        # Generate signals
        data['signal'] = 0
        data.loc[data['ma_fast'] > data['ma_slow'], 'signal'] = 1
        data.loc[data['ma_fast'] < data['ma_slow'], 'signal'] = -1
        
        return data
    
    def backtest(self, data: pd.DataFrame) -> dict:
        \"\"\"Run a simple backtest\"\"\"
        data = self.calculate_signals(data)
        
        # Calculate returns
        data['returns'] = data['close'].pct_change()
        data['strategy_returns'] = data['signal'].shift(1) * data['returns']
        
        # Calculate metrics
        total_return = (1 + data['strategy_returns']).prod() - 1
        sharpe_ratio = data['strategy_returns'].mean() / data['strategy_returns'].std() * np.sqrt(252)
        
        return {
            'total_return': total_return,
            'sharpe_ratio': sharpe_ratio,
            'num_trades': data['signal'].diff().abs().sum() / 2
        }

if __name__ == "__main__":
    print("Simple MA Crossover Strategy loaded successfully!")
    print("To run a backtest, load some market data and call strategy.backtest(data)")
""")
        
        print(f"✅ Workspace initialized at {WORKSPACE_ROOT}")


# Initialize workspace on module load
init_workspace()
"""
NautilusTrader integration for AlphaPulse
Handles file serving and strategy execution
"""

import os
import sys
from pathlib import Path
from typing import Dict, Any, Optional
import json
import importlib.util

from flask import Blueprint, jsonify, request

# Path to nautilus-trader source for accessing examples
NAUTILUS_PATH = Path(__file__).parent.parent / "nautilus-trader"

from nautilus_trader.config import BacktestRunConfig, BacktestVenueConfig, BacktestDataConfig
from nautilus_trader.backtest.node import BacktestNode
from nautilus_trader.model.identifiers import Venue, InstrumentId
from nautilus_trader.persistence.catalog import ParquetDataCatalog

# Create Blueprint
nt_api = Blueprint('nautilus', __name__, url_prefix='/api/nautilus')

# Paths to NT code
NT_EXAMPLES_PATH = NAUTILUS_PATH / "nautilus_trader" / "examples"
NT_INDICATORS_PATH = NAUTILUS_PATH / "nautilus_trader" / "indicators"


@nt_api.route('/files/<path:filepath>', methods=['GET'])
def get_file(filepath):
    """Serve NT files from examples or indicators"""
    try:
        # Determine which base path to use
        if filepath.startswith('indicators/'):
            # For indicator files, use the NT indicators path
            relative_path = filepath.replace('indicators/', '')
            safe_path = NT_INDICATORS_PATH / relative_path
            base_path = NT_INDICATORS_PATH
        else:
            # For everything else, use examples path
            safe_path = NT_EXAMPLES_PATH / filepath
            base_path = NT_EXAMPLES_PATH
            
        # Security check
        if not str(safe_path).startswith(str(base_path)):
            return jsonify({'error': 'Invalid path'}), 403
            
        if not safe_path.exists():
            return jsonify({'error': 'File not found'}), 404
            
        # Check if it's a Cython file (.pyx or .pxd)
        if safe_path.suffix in ['.pyx', '.pxd']:
            with open(safe_path, 'r', encoding='utf-8') as f:
                content = f.read()
        else:
            with open(safe_path, 'r') as f:
                content = f.read()
            
        return jsonify({
            'content': content,
            'path': filepath
        })
    except Exception as e:
        return jsonify({'error': str(e)}), 500


@nt_api.route('/list-files', methods=['GET'])
def list_files():
    """List available NT files"""
    try:
        files = {
            'examples': {
                'strategies': [],
                'algorithms': [],
                'indicators': []
            },
            'indicators': []
        }
        
        # List example strategies
        strategies_path = NT_EXAMPLES_PATH / 'strategies'
        if strategies_path.exists():
            for f in strategies_path.glob('*.py'):
                if f.name != '__init__.py':
                    files['examples']['strategies'].append(f.name)
                    
        # List example algorithms  
        algorithms_path = NT_EXAMPLES_PATH / 'algorithms'
        if algorithms_path.exists():
            for f in algorithms_path.glob('*.py'):
                if f.name != '__init__.py':
                    files['examples']['algorithms'].append(f.name)
                    
        # List example indicators
        example_indicators_path = NT_EXAMPLES_PATH / 'indicators'
        if example_indicators_path.exists():
            for f in example_indicators_path.glob('*.py'):
                if f.name != '__init__.py':
                    files['examples']['indicators'].append(f.name)
        
        # List main indicators (Cython files)
        if NT_INDICATORS_PATH.exists():
            for f in NT_INDICATORS_PATH.glob('*.pyx'):
                # Skip __init__ and base files
                if f.name not in ['__init__.pyx', 'base.pyx']:
                    files['indicators'].append(f.name)
                    
        # Sort all lists
        for key in files['examples']:
            files['examples'][key].sort()
        files['indicators'].sort()
                    
        return jsonify(files)
    except Exception as e:
        return jsonify({'error': str(e)}), 500


@nt_api.route('/run-strategy', methods=['POST'])
def run_strategy():
    """Run a strategy backtest"""
    try:
        data = request.json
        strategy_code = data.get('code')
        strategy_name = data.get('name', 'UserStrategy')
        
        # TODO: This is a simplified version
        # In production, we would:
        # 1. Save the code to a temporary file
        # 2. Import and validate the strategy
        # 3. Configure and run backtest
        # 4. Return results
        
        return jsonify({
            'status': 'success',
            'message': 'Strategy execution not yet implemented',
            'strategy': strategy_name
        })
        
    except Exception as e:
        return jsonify({'error': str(e)}), 500


@nt_api.route('/tutorials', methods=['GET'])
def list_tutorials():
    """List available NT tutorial notebooks"""
    try:
        tutorials_path = NAUTILUS_PATH / "docs" / "tutorials"
        tutorials = []
        
        if tutorials_path.exists():
            for f in tutorials_path.glob('*.ipynb'):
                tutorials.append({
                    'name': f.stem,
                    'filename': f.name,
                    'path': str(f.relative_to(NAUTILUS_PATH))
                })
                
        return jsonify({'tutorials': tutorials})
    except Exception as e:
        return jsonify({'error': str(e)}), 500


@nt_api.route('/notebook/<path:notebook_path>', methods=['GET'])
def get_notebook(notebook_path):
    """Get a specific notebook's content"""
    try:
        # Construct the full path
        full_path = NAUTILUS_PATH / notebook_path
        
        # Security check
        if not str(full_path).startswith(str(NAUTILUS_PATH)):
            return jsonify({'error': 'Invalid path'}), 403
            
        if not full_path.exists() or not full_path.suffix == '.ipynb':
            return jsonify({'error': 'Notebook not found'}), 404
            
        # Read and parse the notebook
        with open(full_path, 'r') as f:
            notebook_data = json.load(f)
            
        # Return the notebook cells
        return jsonify({
            'cells': notebook_data.get('cells', []),
            'metadata': notebook_data.get('metadata', {})
        })
    except Exception as e:
        return jsonify({'error': str(e)}), 500
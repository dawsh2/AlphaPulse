"""
Template Service - Load and manage notebook templates
"""
import json
import os
from pathlib import Path


def load_arbitrage_template():
    """Load the basic arbitrage analysis template"""
    template_path = Path(__file__).parent.parent / 'notebook_templates' / 'arbitrage_basic.py'
    
    # Execute the template file to get the template dict
    with open(template_path, 'r') as f:
        template_code = f.read()
        
    # Create a namespace and execute the template
    namespace = {}
    exec(template_code, namespace)
    
    return namespace.get('template', {})


def get_available_templates():
    """Get list of available templates"""
    return [
        {
            "id": "arbitrage_basic",
            "title": "Cross-Exchange Arbitrage",
            "description": "Analyze BTC price differences between Coinbase and Kraken"
        }
    ]
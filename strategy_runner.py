#!/usr/bin/env python3
"""
NautilusTrader Strategy Runner for AlphaPulse
Validates and executes user strategies from the web IDE
"""

import sys
import importlib.util
import traceback
from pathlib import Path
from typing import Dict, Any, Optional

from nautilus_trader.config import StrategyConfig
from nautilus_trader.trading.strategy import Strategy


class StrategyValidator:
    """Validates user strategies before execution"""
    
    @staticmethod
    def validate_strategy_file(filepath: str) -> Dict[str, Any]:
        """
        Validate a strategy Python file
        
        Returns:
            Dict with validation results
        """
        result = {
            'valid': False,
            'errors': [],
            'warnings': [],
            'strategy_class': None,
            'config_class': None
        }
        
        try:
            # Load the module
            spec = importlib.util.spec_from_file_location("user_strategy", filepath)
            if spec is None or spec.loader is None:
                result['errors'].append("Could not load strategy file")
                return result
                
            module = importlib.util.module_from_spec(spec)
            sys.modules["user_strategy"] = module
            spec.loader.exec_module(module)
            
            # Find Strategy and Config classes
            strategy_classes = []
            config_classes = []
            
            for name, obj in module.__dict__.items():
                if isinstance(obj, type):
                    if issubclass(obj, Strategy) and obj != Strategy:
                        strategy_classes.append(obj)
                    elif issubclass(obj, StrategyConfig) and obj != StrategyConfig:
                        config_classes.append(obj)
            
            # Validate findings
            if not strategy_classes:
                result['errors'].append("No Strategy class found. Must inherit from nautilus_trader.trading.strategy.Strategy")
            elif len(strategy_classes) > 1:
                result['warnings'].append(f"Multiple Strategy classes found: {[c.__name__ for c in strategy_classes]}")
            else:
                result['strategy_class'] = strategy_classes[0].__name__
                
            if not config_classes:
                result['errors'].append("No StrategyConfig class found. Must inherit from nautilus_trader.config.StrategyConfig")
            elif len(config_classes) > 1:
                result['warnings'].append(f"Multiple Config classes found: {[c.__name__ for c in config_classes]}")
            else:
                result['config_class'] = config_classes[0].__name__
                
            # Check required methods
            if strategy_classes:
                strategy = strategy_classes[0]
                required_methods = ['on_start', 'on_stop']
                for method in required_methods:
                    if not hasattr(strategy, method):
                        result['warnings'].append(f"Strategy missing {method} method (will use default)")
                        
            # If we have both classes, it's valid
            if result['strategy_class'] and result['config_class']:
                result['valid'] = True
                
        except SyntaxError as e:
            result['errors'].append(f"Syntax error at line {e.lineno}: {e.msg}")
        except Exception as e:
            result['errors'].append(f"Error loading strategy: {str(e)}")
            result['errors'].append(traceback.format_exc())
            
        finally:
            # Clean up
            if "user_strategy" in sys.modules:
                del sys.modules["user_strategy"]
                
        return result


def main():
    """CLI entry point"""
    if len(sys.argv) < 2:
        print("Usage: python strategy_runner.py <strategy_file> [validate|run]")
        sys.exit(1)
        
    filepath = sys.argv[1]
    action = sys.argv[2] if len(sys.argv) > 2 else "validate"
    
    if action == "validate":
        result = StrategyValidator.validate_strategy_file(filepath)
        
        if result['valid']:
            print("✅ Strategy validation passed!")
            print(f"   Strategy: {result['strategy_class']}")
            print(f"   Config: {result['config_class']}")
        else:
            print("❌ Strategy validation failed!")
            
        if result['errors']:
            print("\nErrors:")
            for error in result['errors']:
                print(f"  - {error}")
                
        if result['warnings']:
            print("\nWarnings:")
            for warning in result['warnings']:
                print(f"  - {warning}")
                
        sys.exit(0 if result['valid'] else 1)
        
    elif action == "run":
        print("Strategy execution not yet implemented")
        print("Would run strategy from:", filepath)
        sys.exit(0)
        
    else:
        print(f"Unknown action: {action}")
        sys.exit(1)


if __name__ == "__main__":
    main()
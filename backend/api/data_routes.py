"""
Data API Routes - Clean HTTP endpoint layer
Handles all data-related API endpoints with proper separation of concerns
"""
from flask import Blueprint, request, jsonify, make_response
from typing import Dict, Any
import json

from services.data_service import DataService
from services.analysis_service import AnalysisService
from data_manager import DataManager
from utils.validation import validate_json, validate_query_params, validate_response, add_cors_headers
from schemas.market_data import (
    SaveDataRequest, QueryRequest, QueryResult, CorrelationResponse, 
    ApiResponse, AnalysisRequest, RollingStatsRequest, RiskMetricsRequest,
    BacktestConfig, BacktestResult, RegimeAnalysis, StatisticsResult
)

# Create blueprint
data_api = Blueprint('data_api', __name__)

# Initialize services lazily to avoid database lock conflicts
data_manager = None
data_service = None
analysis_service = None

def get_services():
    """Lazy initialization of services"""
    global data_manager, data_service, analysis_service
    if data_manager is None:
        data_manager = DataManager()
        data_service = DataService(data_manager)
        analysis_service = AnalysisService(data_manager)
    return data_service, analysis_service


# Remove duplicate CORS function since it's now in utils.validation


@data_api.route('/api/market-data/save', methods=['POST', 'OPTIONS'])
@validate_json(SaveDataRequest)
@validate_response(ApiResponse)
def save_market_data(body: SaveDataRequest):
    """Save market data to database (optional endpoint for caching)."""
    # Handle preflight OPTIONS request
    if request.method == 'OPTIONS':
        response = make_response(jsonify({'status': 'ok'}))
        return add_cors_headers(response), 200
    
    try:
        data_service, _ = get_services()
        # Convert Pydantic model to dict for service layer
        data_dict = {
            'symbol': body.symbol,
            'exchange': body.exchange,
            'candles': body.candles,
            'interval': body.interval
        }
        result = data_service.save_market_data(data_dict)
        
        response = make_response(jsonify(result))
        return add_cors_headers(response)
        
    except Exception as e:
        print(f"❌ Error saving market data: {str(e)}")
        response = make_response(jsonify({'status': 'error', 'message': str(e)}), 500)
        return add_cors_headers(response)


@data_api.route('/api/proxy/coinbase/<path:endpoint>', methods=['GET', 'OPTIONS'])
def proxy_coinbase(endpoint):
    """Proxy requests to Coinbase API to avoid CORS issues."""
    # Handle preflight OPTIONS request
    if request.method == 'OPTIONS':
        response = make_response()
        return add_cors_headers(response)
    
    try:
        # Forward query parameters
        params = request.args.to_dict()
        
        data_service, _ = get_services()
        result = data_service.proxy_coinbase_data(endpoint, params)
        
        if result['status'] == 'error':
            response = make_response(jsonify({'error': result['message']}), result['status_code'])
        else:
            response = make_response(jsonify(result['data']), result['status_code'])
        
        return add_cors_headers(response)
        
    except Exception as e:
        response = make_response(jsonify({'error': f'Unexpected error: {str(e)}'}), 500)
        return add_cors_headers(response)


@data_api.route('/api/data/summary', methods=['GET', 'OPTIONS'])
def get_data_summary():
    """Get summary of all data stored in Parquet files."""
    # Handle preflight OPTIONS request
    if request.method == 'OPTIONS':
        response = make_response()
        return add_cors_headers(response)
    
    try:
        data_service, _ = get_services()
        summary = data_service.get_data_summary()
        response = make_response(jsonify(summary))
        return add_cors_headers(response)
    except Exception as e:
        response = make_response(jsonify({'error': str(e)}), 500)
        return add_cors_headers(response)


@data_api.route('/api/data/query', methods=['POST', 'OPTIONS'])
@validate_json(QueryRequest)
@validate_response(QueryResult)
def query_data(body: QueryRequest):
    """Execute SQL query on DuckDB."""
    # Handle preflight OPTIONS request
    if request.method == 'OPTIONS':
        response = make_response()
        return add_cors_headers(response)
    
    try:
        data_service, _ = get_services()
        result = data_service.query_data(body.query)
        response = make_response(jsonify(result))
        return add_cors_headers(response)
    except Exception as e:
        response = make_response(jsonify({'error': str(e)}), 500)
        return add_cors_headers(response)


@data_api.route('/api/data/correlation/<symbol1>/<symbol2>', methods=['GET', 'OPTIONS'])
def get_correlation(symbol1, symbol2):
    """Get correlation between two symbols."""
    # Handle preflight OPTIONS request
    if request.method == 'OPTIONS':
        response = make_response()
        return add_cors_headers(response)
    
    try:
        data_service, _ = get_services()
        result = data_service.get_correlation(symbol1, symbol2)
        response = make_response(jsonify(result))
        return add_cors_headers(response)
    except Exception as e:
        response = make_response(jsonify({'error': str(e)}), 500)
        return add_cors_headers(response)


@data_api.route('/api/catalog/list', methods=['GET', 'OPTIONS'])
def list_catalog_data():
    """List all available data in the backend catalog."""
    # Handle preflight OPTIONS request
    if request.method == 'OPTIONS':
        response = make_response()
        return add_cors_headers(response)
    
    try:
        data_service, _ = get_services()
        result = data_service.list_catalog_data()
        response = make_response(jsonify(result))
        return add_cors_headers(response)
        
    except Exception as e:
        response = make_response(jsonify({'error': str(e)}), 500)
        return add_cors_headers(response)


# ================================================================================
# Analysis API Routes
# ================================================================================

@data_api.route('/api/analysis/statistics/<symbol>', methods=['GET', 'OPTIONS'])
def get_symbol_statistics(symbol):
    """Get basic statistics for a symbol."""
    # Handle preflight OPTIONS request
    if request.method == 'OPTIONS':
        response = make_response()
        return add_cors_headers(response)
    
    try:
        exchange = request.args.get('exchange', 'coinbase')
        # Convert URL format to our format
        symbol = symbol.replace('-', '/')
        
        _, analysis_service = get_services()
        stats = analysis_service.calculate_basic_statistics(symbol, exchange)
        response = make_response(jsonify({
            'symbol': symbol,
            'exchange': exchange,
            'statistics': stats
        }))
        return add_cors_headers(response)
    except Exception as e:
        response = make_response(jsonify({'error': str(e)}), 500)
        return add_cors_headers(response)


@data_api.route('/api/analysis/correlation-matrix', methods=['POST', 'OPTIONS'])
@validate_json(AnalysisRequest)
@validate_response(ApiResponse)
def get_correlation_matrix(body: AnalysisRequest):
    """Get correlation matrix for multiple symbols."""
    # Handle preflight OPTIONS request
    if request.method == 'OPTIONS':
        response = make_response()
        return add_cors_headers(response)
    
    try:
        # Note: AnalysisRequest validator already handles symbol format normalization
        _, analysis_service = get_services()
        result = analysis_service.calculate_correlation_matrix(body.symbols, body.exchange.value)
        response = make_response(jsonify(result))
        return add_cors_headers(response)
    except Exception as e:
        response = make_response(jsonify({'error': str(e)}), 500)
        return add_cors_headers(response)


@data_api.route('/api/analysis/rolling-stats/<symbol>', methods=['GET', 'OPTIONS'])
def get_rolling_statistics(symbol):
    """Get rolling statistics for a symbol."""
    # Handle preflight OPTIONS request
    if request.method == 'OPTIONS':
        response = make_response()
        return add_cors_headers(response)
    
    try:
        exchange = request.args.get('exchange', 'coinbase')
        window = request.args.get('window', 20, type=int)
        # Convert URL format to our format
        symbol = symbol.replace('-', '/')
        
        _, analysis_service = get_services()
        stats = analysis_service.calculate_rolling_statistics(symbol, window, exchange)
        response = make_response(jsonify(stats))
        return add_cors_headers(response)
    except Exception as e:
        response = make_response(jsonify({'error': str(e)}), 500)
        return add_cors_headers(response)


@data_api.route('/api/analysis/risk-metrics/<symbol>', methods=['GET', 'OPTIONS'])
def get_risk_metrics(symbol):
    """Get comprehensive risk metrics for a symbol."""
    # Handle preflight OPTIONS request
    if request.method == 'OPTIONS':
        response = make_response()
        return add_cors_headers(response)
    
    try:
        exchange = request.args.get('exchange', 'coinbase')
        risk_free_rate = request.args.get('risk_free_rate', 0.02, type=float)
        # Convert URL format to our format
        symbol = symbol.replace('-', '/')
        
        _, analysis_service = get_services()
        metrics = analysis_service.calculate_risk_metrics(symbol, exchange, risk_free_rate)
        response = make_response(jsonify(metrics))
        return add_cors_headers(response)
    except Exception as e:
        response = make_response(jsonify({'error': str(e)}), 500)
        return add_cors_headers(response)


@data_api.route('/api/analysis/backtest', methods=['POST', 'OPTIONS'])
def run_backtest():
    """Run backtesting analysis on a strategy."""
    # Handle preflight OPTIONS request
    if request.method == 'OPTIONS':
        response = make_response()
        return add_cors_headers(response)
    
    try:
        strategy_config = request.get_json()
        
        _, analysis_service = get_services()
        result = analysis_service.perform_backtesting_analysis(strategy_config)
        response = make_response(jsonify(result))
        return add_cors_headers(response)
    except Exception as e:
        response = make_response(jsonify({'error': str(e)}), 500)
        return add_cors_headers(response)


@data_api.route('/api/analysis/market-regime', methods=['POST', 'OPTIONS'])
def get_market_regime():
    """Get market regime analysis for multiple symbols."""
    # Handle preflight OPTIONS request
    if request.method == 'OPTIONS':
        response = make_response()
        return add_cors_headers(response)
    
    try:
        data = request.get_json()
        symbols = data.get('symbols', [])
        exchange = data.get('exchange', 'coinbase')
        
        # Convert URL format to our format
        symbols = [s.replace('-', '/') for s in symbols]
        
        _, analysis_service = get_services()
        result = analysis_service.get_market_regime_analysis(symbols, exchange)
        response = make_response(jsonify(result))
        return add_cors_headers(response)
    except Exception as e:
        response = make_response(jsonify({'error': str(e)}), 500)
        return add_cors_headers(response)


# Cleanup function
def cleanup_services():
    """Cleanup services on shutdown"""
    global data_service, analysis_service, data_manager
    if data_service:
        data_service.close()
    if analysis_service:
        analysis_service.close()
    if data_manager:
        data_manager.close()
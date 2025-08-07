"""
AlphaPulse API Server - Real Alpaca Integration
Event-driven trading system using OS environment variables
"""
from flask import Flask, request, jsonify, session
from flask_cors import CORS
from flask_sqlalchemy import SQLAlchemy
import jwt
from datetime import datetime, timedelta
import uuid
import os
from pathlib import Path

# Local imports
from config import Config
from models import db, init_db, User, Strategy, EventLog
from alpaca_client import create_alpaca_client
from nautilus_integration import nt_api

# Validate configuration on startup
if not Config.validate():
    print("❌ Configuration validation failed. Please set your Alpaca API keys.")
    exit(1)

# Initialize Flask app
app = Flask(__name__)
app.config['SECRET_KEY'] = Config.SECRET_KEY
app.config['SQLALCHEMY_DATABASE_URI'] = Config.DATABASE_URL
app.config['SQLALCHEMY_TRACK_MODIFICATIONS'] = False

# Initialize extensions
CORS(app, origins=Config.CORS_ORIGINS, supports_credentials=True)
init_db(app)

# Register blueprints
app.register_blueprint(nt_api)

# Global Alpaca client
alpaca_client = create_alpaca_client()

def generate_jwt_token(user_id):
    """Generate JWT token for user authentication."""
    payload = {
        'user_id': user_id,
        'exp': datetime.utcnow() + timedelta(seconds=Config.JWT_ACCESS_TOKEN_EXPIRES),
        'iat': datetime.utcnow()
    }
    return jwt.encode(payload, Config.JWT_SECRET_KEY, algorithm='HS256')

def verify_jwt_token(token):
    """Verify and decode JWT token."""
    try:
        payload = jwt.decode(token, Config.JWT_SECRET_KEY, algorithms=['HS256'])
        return payload['user_id']
    except jwt.ExpiredSignatureError:
        return None
    except jwt.InvalidTokenError:
        return None

def require_auth(f):
    """Decorator to require authentication."""
    def decorated(*args, **kwargs):
        token = request.headers.get('Authorization')
        if not token:
            return jsonify({'error': 'No authorization token provided'}), 401
        
        if token.startswith('Bearer '):
            token = token[7:]
        
        user_id = verify_jwt_token(token)
        if not user_id:
            return jsonify({'error': 'Invalid or expired token'}), 401
        
        user = User.query.get(user_id)
        if not user or not user.is_active:
            return jsonify({'error': 'User not found or inactive'}), 401
        
        request.current_user = user
        return f(*args, **kwargs)
    
    decorated.__name__ = f.__name__
    return decorated

# ================================================================================
# Health Check and System Info
# ================================================================================

@app.route('/api/health')
def health_check():
    """System health check."""
    # Test Alpaca connection
    alpaca_status = 'disconnected'
    if alpaca_client:
        test_result = alpaca_client.test_connection()
        alpaca_status = test_result.get('status', 'error')
    
    return jsonify({
        'status': 'ok',
        'message': 'AlphaPulse API Server (Live Data)',
        'version': '1.0.0',
        'timestamp': datetime.utcnow().isoformat(),
        'alpaca_status': alpaca_status,
        'alpaca_url': Config.ALPACA_BASE_URL
    })

@app.route('/api/info')
def system_info():
    """Get system information."""
    return jsonify({
        'name': 'AlphaPulse',
        'description': 'Event-driven quantitative trading system',
        'version': '1.0.0',
        'mode': 'live_data',
        'broker_integration': bool(alpaca_client),
        'frontend_url': Config.FRONTEND_URL,
        'alpaca_base_url': Config.ALPACA_BASE_URL
    })

# ================================================================================
# Authentication & User Management
# ================================================================================

@app.route('/api/auth/demo-login', methods=['POST'])
def demo_login():
    """Create or login demo user."""
    try:
        demo_email = 'demo@alphapulse.com'
        user = User.query.filter_by(email=demo_email).first()
        
        if not user:
            user = User(
                email=demo_email,
                username='demo_user',
                subscription_tier='premium'
            )
            db.session.add(user)
            db.session.commit()
        
        user.last_login = datetime.utcnow()
        db.session.commit()
        
        token = generate_jwt_token(user.id)
        
        return jsonify({
            'status': 'success',
            'message': 'Demo login successful',
            'data': {
                'user': user.to_dict(),
                'token': token
            }
        })
        
    except Exception as e:
        return jsonify({'status': 'error', 'message': str(e)}), 500

@app.route('/api/auth/user')
@require_auth
def get_current_user():
    """Get current authenticated user."""
    return jsonify({
        'status': 'success',
        'data': request.current_user.to_dict()
    })

# ================================================================================
# Alpaca Integration Endpoints
# ================================================================================

@app.route('/api/alpaca/test-connection')
@require_auth
def test_alpaca_connection():
    """Test Alpaca API connection."""
    if not alpaca_client:
        return jsonify({
            'status': 'error',
            'message': 'Alpaca client not configured. Check your API keys.'
        }), 500
    
    result = alpaca_client.test_connection()
    return jsonify(result)

@app.route('/api/account')
@require_auth
def get_account_info():
    """Get account information from Alpaca."""
    try:
        if not alpaca_client:
            return jsonify({
                'status': 'error',
                'message': 'Alpaca client not configured'
            }), 500
        
        account_info = alpaca_client.get_account()
        if not account_info:
            return jsonify({
                'status': 'error',
                'message': 'Failed to get account info from Alpaca'
            }), 500
        
        return jsonify({
            'status': 'success',
            'data': {
                'buying_power': float(account_info.get('buying_power', 0)),
                'cash': float(account_info.get('cash', 0)),
                'portfolio_value': float(account_info.get('portfolio_value', 0)),
                'account_type': 'paper' if 'paper' in Config.ALPACA_BASE_URL else 'live',
                'broker_name': 'alpaca',
                'market_open': alpaca_client.is_market_open(),
                'account_status': account_info.get('status', 'unknown'),
                'day_trading_buying_power': float(account_info.get('day_trading_buying_power', 0)),
                'equity': float(account_info.get('equity', 0))
            }
        })
        
    except Exception as e:
        return jsonify({'status': 'error', 'message': str(e)}), 500

@app.route('/api/positions')
@require_auth
def get_positions():
    """Get current positions from Alpaca."""
    try:
        if not alpaca_client:
            return jsonify({'status': 'error', 'message': 'Alpaca client not configured'}), 500
        
        positions = alpaca_client.get_positions()
        
        return jsonify({
            'status': 'success',
            'data': [{
                'symbol': pos.symbol,
                'qty': float(pos.qty),
                'market_value': float(pos.market_value) if pos.market_value else 0,
                'unrealized_pl': float(pos.unrealized_pl) if pos.unrealized_pl else 0,
                'unrealized_plpc': float(pos.unrealized_plpc) if pos.unrealized_plpc else 0,
                'current_price': float(pos.current_price) if pos.current_price else 0,
                'avg_entry_price': float(pos.avg_entry_price) if pos.avg_entry_price else 0
            } for pos in positions]
        })
        
    except Exception as e:
        return jsonify({'status': 'error', 'message': str(e)}), 500

@app.route('/api/orders')
@require_auth
def get_orders():
    """Get recent orders from Alpaca."""
    try:
        if not alpaca_client:
            return jsonify({'status': 'error', 'message': 'Alpaca client not configured'}), 500
        
        limit = request.args.get('limit', 50, type=int)
        status = request.args.get('status', 'all')
        
        orders = alpaca_client.get_orders(status=status, limit=limit)
        
        return jsonify({
            'status': 'success',
            'data': [{
                'id': order.id,
                'symbol': order.symbol,
                'qty': float(order.qty) if order.qty else 0,
                'side': order.side,
                'order_type': order.order_type,
                'status': order.status,
                'filled_qty': float(order.filled_qty) if order.filled_qty else 0,
                'filled_avg_price': float(order.filled_avg_price) if order.filled_avg_price else None,
                'created_at': order.created_at
            } for order in orders]
        })
        
    except Exception as e:
        return jsonify({'status': 'error', 'message': str(e)}), 500

@app.route('/api/market-data/<symbol>')
@require_auth
def get_market_data(symbol):
    """Get real market data from Alpaca."""
    try:
        if not alpaca_client:
            return jsonify({'status': 'error', 'message': 'Alpaca client not configured'}), 500
        
        # Get timeframe from query params (default to 5Min for live trading view)
        timeframe = request.args.get('timeframe', '5Min')
        limit = request.args.get('limit', 100, type=int)
        
        # Get bars from Alpaca
        bars = alpaca_client.get_bars(
            symbol=symbol, 
            timeframe=timeframe, 
            limit=limit
        )
        
        # Convert to format expected by frontend
        chart_data = []
        for bar in bars:
            if bar.timestamp:
                # Convert timestamp to seconds if it's in milliseconds
                timestamp = bar.timestamp
                if isinstance(timestamp, str):
                    # Parse ISO format timestamp
                    from datetime import datetime as dt_parser
                    # Remove 'Z' suffix and parse
                    timestamp_clean = timestamp.rstrip('Z')
                    dt = dt_parser.fromisoformat(timestamp_clean)
                    timestamp = int(dt.timestamp())
                elif timestamp > 10**10:  # If timestamp is in milliseconds
                    timestamp = timestamp // 1000
                
                chart_data.append({
                    'time': timestamp,
                    'open': float(bar.open) if bar.open else 0,
                    'high': float(bar.high) if bar.high else 0,
                    'low': float(bar.low) if bar.low else 0,
                    'close': float(bar.close) if bar.close else 0,
                    'volume': int(bar.volume) if bar.volume else 0
                })
        
        return jsonify({
            'status': 'success',
            'data': {
                'symbol': symbol,
                'timeframe': timeframe,
                'bars': chart_data
            }
        })
        
    except Exception as e:
        return jsonify({'status': 'error', 'message': str(e)}), 500

# ================================================================================
# Trading Operations (Paper Trading Safe)
# ================================================================================

@app.route('/api/orders', methods=['POST'])
@require_auth
def submit_order():
    """Submit a trading order (paper trading only for safety)."""
    try:
        if not alpaca_client:
            return jsonify({'status': 'error', 'message': 'Alpaca client not configured'}), 500
        
        # Safety check - only allow paper trading
        if 'paper' not in Config.ALPACA_BASE_URL.lower():
            return jsonify({
                'status': 'error',
                'message': 'Live trading not enabled. Use paper trading for development.'
            }), 403
        
        data = request.get_json()
        
        # Validate required fields
        required_fields = ['symbol', 'qty', 'side']
        for field in required_fields:
            if field not in data:
                return jsonify({
                    'status': 'error',
                    'message': f'Missing required field: {field}'
                }), 400
        
        # Submit order to Alpaca
        order = alpaca_client.submit_order(
            symbol=data['symbol'],
            qty=data['qty'],
            side=data['side'],
            type=data.get('type', 'market'),
            time_in_force=data.get('time_in_force', 'day')
        )
        
        if order:
            # Log the trade event
            event = EventLog(
                user_id=request.current_user.id,
                event_type='trade',
                message=f"Order submitted: {data['side']} {data['qty']} {data['symbol']}",
                symbol=data['symbol'],
                severity='info'
            )
            event.set_event_data({
                'order_id': order.id,
                'symbol': data['symbol'],
                'qty': data['qty'],
                'side': data['side'],
                'type': data.get('type', 'market')
            })
            db.session.add(event)
            db.session.commit()
            
            return jsonify({
                'status': 'success',
                'data': {
                    'order_id': order.id,
                    'symbol': order.symbol,
                    'qty': float(order.qty) if order.qty else 0,
                    'side': order.side,
                    'status': order.status
                }
            })
        else:
            return jsonify({
                'status': 'error',
                'message': 'Failed to submit order to Alpaca'
            }), 500
        
    except Exception as e:
        return jsonify({'status': 'error', 'message': str(e)}), 500

# ================================================================================
# Strategy Management
# ================================================================================

@app.route('/api/strategies')
@require_auth
def get_strategies():
    """Get user's strategies."""
    try:
        strategies = Strategy.query.filter_by(user_id=request.current_user.id).all()
        return jsonify({
            'status': 'success',
            'data': [strategy.to_dict() for strategy in strategies]
        })
    except Exception as e:
        return jsonify({'status': 'error', 'message': str(e)}), 500

@app.route('/api/strategies', methods=['POST'])
@require_auth
def create_strategy():
    """Create a new strategy."""
    try:
        data = request.get_json()
        
        strategy = Strategy(
            user_id=request.current_user.id,
            name=data['name'],
            description=data.get('description', ''),
            config=data.get('config', '{}')
        )
        
        if isinstance(strategy.config, dict):
            strategy.set_config(strategy.config)
        
        db.session.add(strategy)
        db.session.commit()
        
        return jsonify({
            'status': 'success',
            'data': strategy.to_dict()
        })
        
    except Exception as e:
        return jsonify({'status': 'error', 'message': str(e)}), 500

# ================================================================================
# Event Logging
# ================================================================================

@app.route('/api/events')
@require_auth
def get_events():
    """Get recent events for user."""
    try:
        limit = request.args.get('limit', 50, type=int)
        event_type = request.args.get('type')
        
        query = EventLog.query.filter_by(user_id=request.current_user.id)
        
        if event_type:
            query = query.filter_by(event_type=event_type)
        
        events = query.order_by(EventLog.created_at.desc()).limit(limit).all()
        
        return jsonify({
            'status': 'success',
            'data': [event.to_dict() for event in events]
        })
        
    except Exception as e:
        return jsonify({'status': 'error', 'message': str(e)}), 500

@app.route('/api/events', methods=['POST'])
@require_auth
def create_event():
    """Create a new event log entry."""
    try:
        data = request.get_json()
        
        event = EventLog(
            user_id=request.current_user.id,
            strategy_id=data.get('strategy_id'),
            event_type=data['event_type'],
            message=data.get('message'),
            symbol=data.get('symbol'),
            severity=data.get('severity', 'info')
        )
        
        if 'event_data' in data:
            event.set_event_data(data['event_data'])
        
        db.session.add(event)
        db.session.commit()
        
        return jsonify({
            'status': 'success',
            'data': event.to_dict()
        })
        
    except Exception as e:
        return jsonify({'status': 'error', 'message': str(e)}), 500

# ================================================================================
# Error Handlers
# ================================================================================

@app.errorhandler(404)
def not_found(error):
    return jsonify({'status': 'error', 'message': 'Endpoint not found'}), 404

@app.errorhandler(500)
def internal_error(error):
    return jsonify({'status': 'error', 'message': 'Internal server error'}), 500

# ================================================================================
# NT Reference File Management
# ================================================================================

@app.route('/api/nt-reference/list-files', methods=['GET'])
def list_nt_reference_files():
    """List all files in the nt_reference directory."""
    try:
        nt_ref_path = Path(__file__).parent / 'nt_reference'
        
        result = {
            'examples': {
                'strategies': [],
                'algorithms': [],
                'indicators': []
            },
            'tutorials': []
        }
        
        # List example files
        examples_path = nt_ref_path / 'examples'
        if examples_path.exists():
            # Strategies
            strategies_path = examples_path / 'strategies'
            if strategies_path.exists():
                result['examples']['strategies'] = [
                    f.name for f in strategies_path.glob('*.py')
                    if f.name != '__init__.py'
                ]
            
            # Algorithms
            algorithms_path = examples_path / 'algorithms'
            if algorithms_path.exists():
                result['examples']['algorithms'] = [
                    f.name for f in algorithms_path.glob('*.py')
                    if f.name != '__init__.py'
                ]
            
            # Indicators
            indicators_path = examples_path / 'indicators'
            if indicators_path.exists():
                result['examples']['indicators'] = [
                    f.name for f in indicators_path.glob('*.py')
                    if f.name != '__init__.py'
                ]
        
        # List tutorial notebooks
        tutorials_path = nt_ref_path / 'tutorials'
        if tutorials_path.exists():
            result['tutorials'] = [
                f.name for f in tutorials_path.glob('*.ipynb')
            ]
        
        return jsonify(result)
    
    except Exception as e:
        return jsonify({'error': str(e)}), 500

@app.route('/api/nt-reference/files/<path:filepath>', methods=['GET'])
def get_nt_reference_file(filepath):
    """Get content of a specific file from nt_reference."""
    try:
        # Security: ensure the path doesn't escape nt_reference
        nt_ref_path = Path(__file__).parent / 'nt_reference'
        
        # Clean the filepath and ensure it's within nt_reference
        if filepath.startswith('strategies/') or filepath.startswith('algorithms/') or filepath.startswith('indicators/'):
            file_path = nt_ref_path / 'examples' / filepath
        elif filepath.startswith('tutorials/'):
            file_path = nt_ref_path / filepath
        else:
            # Default to examples
            file_path = nt_ref_path / 'examples' / filepath
        
        # Security check - ensure resolved path is within nt_reference
        file_path = file_path.resolve()
        if not str(file_path).startswith(str(nt_ref_path.resolve())):
            return jsonify({'error': 'Invalid file path'}), 403
        
        if not file_path.exists():
            return jsonify({'error': 'File not found'}), 404
        
        content = file_path.read_text()
        
        return jsonify({
            'content': content,
            'filename': file_path.name,
            'path': filepath
        })
    
    except Exception as e:
        return jsonify({'error': str(e)}), 500

# ================================================================================
# Development Seeding
# ================================================================================

def seed_development_data():
    """Seed database with development data."""
    try:
        # Create demo user if it doesn't exist
        demo_user = User.query.filter_by(email='demo@alphapulse.com').first()
        if not demo_user:
            demo_user = User(
                email='demo@alphapulse.com',
                username='demo_user',
                subscription_tier='premium'
            )
            db.session.add(demo_user)
            db.session.commit()
            
            # Create sample strategy
            sample_strategy = Strategy(
                user_id=demo_user.id,
                name='adaptive_ensemble',
                description='Multi-factor adaptive strategy with live data',
                status='draft'
            )
            sample_strategy.set_config({
                'symbols': ['SPY'],
                'timeframe': '5m',
                'components': [
                    {'type': 'trend', 'weight': 0.35},
                    {'type': 'mean_reversion', 'weight': 0.35},
                    {'type': 'breakout', 'weight': 0.30}
                ]
            })
            db.session.add(sample_strategy)
            
            # Create sample events
            event = EventLog(
                user_id=demo_user.id,
                event_type='system',
                message='AlphaPulse live data environment initialized',
                severity='info'
            )
            db.session.add(event)
            
            db.session.commit()
            print("✅ Development data seeded successfully")
    
    except Exception as e:
        print(f"Error seeding development data: {e}")

# ================================================================================
# Main Application Entry Point
# ================================================================================

if __name__ == '__main__':
    print("=" * 70)
    print(f"🚀 Starting AlphaPulse API Server (Live Data)")
    print(f"📊 Environment: {Config.FLASK_ENV}")
    print(f"🌐 Port: {Config.FLASK_PORT}")
    print(f"🔗 Frontend URL: {Config.FRONTEND_URL}")
    print(f"🏦 Alpaca URL: {Config.ALPACA_BASE_URL}")
    print(f"📈 Live Data: {'✅' if alpaca_client else '❌'}")
    if 'paper' in Config.ALPACA_BASE_URL.lower():
        print("🧪 Paper Trading Mode (Safe for Development)")
    else:
        print("⚠️  LIVE TRADING MODE - Real Money!")
    print("=" * 70)
    
    # Test Alpaca connection on startup
    if alpaca_client:
        print("Testing Alpaca connection...")
        test_result = alpaca_client.test_connection()
        if test_result.get('status') == 'success':
            print(f"✅ Alpaca connected successfully")
            print(f"   Account type: {test_result.get('account_type')}")
            print(f"   Buying power: ${float(test_result.get('buying_power', 0)):,.2f}")
        else:
            print(f"❌ Alpaca connection failed: {test_result.get('message')}")
    
    # Seed development data
    with app.app_context():
        seed_development_data()
    
    app.run(
        debug=(Config.FLASK_ENV == 'development'),
        port=Config.FLASK_PORT,
        host='0.0.0.0'
    )

"""
Database models for AlphaPulse (simplified for development)
"""
from flask_sqlalchemy import SQLAlchemy
from datetime import datetime
import uuid
import json

db = SQLAlchemy()

class User(db.Model):
    """User accounts and authentication."""
    __tablename__ = 'users'
    
    id = db.Column(db.String(36), primary_key=True, default=lambda: str(uuid.uuid4()))
    email = db.Column(db.String(255), unique=True, nullable=False)
    username = db.Column(db.String(100), unique=True, nullable=False)
    password_hash = db.Column(db.String(255), nullable=True)
    
    # Account settings
    subscription_tier = db.Column(db.String(50), default='free')
    is_active = db.Column(db.Boolean, default=True)
    email_verified = db.Column(db.Boolean, default=False)
    
    # Timestamps
    created_at = db.Column(db.DateTime, default=datetime.utcnow)
    last_login = db.Column(db.DateTime)
    
    # Relationships
    strategies = db.relationship('Strategy', backref='user', lazy=True, cascade='all, delete-orphan')
    
    def to_dict(self):
        return {
            'id': self.id,
            'email': self.email,
            'username': self.username,
            'subscription_tier': self.subscription_tier,
            'is_active': self.is_active,
            'created_at': self.created_at.isoformat() if self.created_at else None
        }

class Strategy(db.Model):
    """User's trading strategies."""
    __tablename__ = 'strategies'
    
    id = db.Column(db.String(36), primary_key=True, default=lambda: str(uuid.uuid4()))
    user_id = db.Column(db.String(36), db.ForeignKey('users.id'), nullable=False)
    
    # Strategy info
    name = db.Column(db.String(255), nullable=False)
    description = db.Column(db.Text, nullable=True)
    type = db.Column(db.String(50), default='single')  # 'single' or 'ensemble'
    
    # Configuration (stored as JSON)
    config = db.Column(db.Text, nullable=False, default='{}')
    parameters = db.Column(db.Text, default='{}')  # JSON
    entry_conditions = db.Column(db.Text, default='[]')  # JSON array
    exit_conditions = db.Column(db.Text, default='[]')  # JSON array
    risk_management = db.Column(db.Text, default='{}')  # JSON
    components = db.Column(db.Text, default='[]')  # JSON array for ensemble strategies
    
    # Status
    status = db.Column(db.String(50), default='draft')  # 'draft', 'testing', 'live', 'paused', 'stopped'
    is_active = db.Column(db.Boolean, default=False)
    is_public = db.Column(db.Boolean, default=False)
    is_template = db.Column(db.Boolean, default=False)
    
    # Performance tracking
    total_return = db.Column(db.Float, default=0.0)
    sharpe_ratio = db.Column(db.Float, nullable=True)
    max_drawdown = db.Column(db.Float, nullable=True)
    backtest_results = db.Column(db.Text, default='{}')  # JSON
    
    # Metadata
    tags = db.Column(db.Text, default='[]')  # JSON array
    
    # Timestamps
    created_at = db.Column(db.DateTime, default=datetime.utcnow)
    updated_at = db.Column(db.DateTime, default=datetime.utcnow, onupdate=datetime.utcnow)
    deployed_at = db.Column(db.DateTime, nullable=True)
    
    def get_config(self):
        """Parse JSON config."""
        try:
            return json.loads(self.config) if self.config else {}
        except json.JSONDecodeError:
            return {}
    
    def set_config(self, config_dict):
        """Set config from dictionary."""
        self.config = json.dumps(config_dict)
    
    def _parse_json(self, field):
        """Helper to parse JSON fields."""
        try:
            value = getattr(self, field, '{}')
            if isinstance(value, str):
                return json.loads(value) if value else {} if field.endswith('management') or field == 'parameters' or field == 'backtest_results' else []
            return value
        except json.JSONDecodeError:
            return {} if field.endswith('management') or field == 'parameters' or field == 'backtest_results' else []
    
    def to_dict(self):
        return {
            'id': self.id,
            'name': self.name,
            'description': self.description,
            'type': self.type,
            'config': self.get_config(),
            'parameters': self._parse_json('parameters'),
            'entry_conditions': self._parse_json('entry_conditions'),
            'exit_conditions': self._parse_json('exit_conditions'),
            'risk_management': self._parse_json('risk_management'),
            'components': self._parse_json('components'),
            'backtest_results': self._parse_json('backtest_results'),
            'tags': self._parse_json('tags'),
            'status': self.status,
            'is_active': self.is_active,
            'is_public': self.is_public,
            'is_template': self.is_template,
            'total_return': self.total_return,
            'sharpe_ratio': self.sharpe_ratio,
            'max_drawdown': self.max_drawdown,
            'created_at': self.created_at.isoformat() if self.created_at else None,
            'updated_at': self.updated_at.isoformat() if self.updated_at else None,
            'deployed_at': self.deployed_at.isoformat() if self.deployed_at else None
        }
    
    def to_python_code(self):
        """Generate Python code for the strategy."""
        python_template = f'''
# AlphaPulse Strategy: {self.name}
# Generated: {datetime.utcnow().isoformat()}

from alphapulse import Strategy, Condition, RiskManager

class {self.name.replace(" ", "")}(Strategy):
    """
    {self.description or 'Auto-generated strategy'}
    """
    
    def __init__(self):
        super().__init__(name="{self.name}")
        
        # Parameters
        self.parameters = {self._parse_json('parameters')}
        
        # Entry Conditions
        self.entry_conditions = {self._parse_json('entry_conditions')}
        
        # Exit Conditions  
        self.exit_conditions = {self._parse_json('exit_conditions')}
        
        # Risk Management
        self.risk_management = {self._parse_json('risk_management')}
'''
        return python_template

class EventLog(db.Model):
    """Event logging for trading activities."""
    __tablename__ = 'event_logs'
    
    id = db.Column(db.String(36), primary_key=True, default=lambda: str(uuid.uuid4()))
    user_id = db.Column(db.String(36), db.ForeignKey('users.id'), nullable=False)
    strategy_id = db.Column(db.String(36), db.ForeignKey('strategies.id'), nullable=True)
    
    # Event details
    event_type = db.Column(db.String(50), nullable=False)  # 'signal', 'trade', 'risk', 'system', 'error'
    event_data = db.Column(db.Text, nullable=True, default='{}')  # JSON string
    message = db.Column(db.Text, nullable=True)
    
    # Metadata
    symbol = db.Column(db.String(20), nullable=True)
    severity = db.Column(db.String(20), default='info')  # 'info', 'warning', 'error', 'critical'
    
    # Timestamp
    created_at = db.Column(db.DateTime, default=datetime.utcnow)
    
    def get_event_data(self):
        """Parse JSON event data."""
        try:
            return json.loads(self.event_data) if self.event_data else {}
        except json.JSONDecodeError:
            return {}
    
    def set_event_data(self, data_dict):
        """Set event data from dictionary."""
        self.event_data = json.dumps(data_dict)
    
    def to_dict(self):
        return {
            'id': self.id,
            'event_type': self.event_type,
            'event_data': self.get_event_data(),
            'message': self.message,
            'symbol': self.symbol,
            'severity': self.severity,
            'created_at': self.created_at.isoformat() if self.created_at else None
        }

def init_db(app):
    """Initialize database with Flask app."""
    db.init_app(app)
    
    with app.app_context():
        # Create all tables
        db.create_all()
        print("Database initialized successfully")

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
    
    # Configuration (stored as JSON)
    config = db.Column(db.Text, nullable=False, default='{}')
    
    # Status
    status = db.Column(db.String(50), default='draft')  # 'draft', 'testing', 'live', 'paused', 'stopped'
    is_active = db.Column(db.Boolean, default=False)
    
    # Performance tracking (mock data for development)
    total_return = db.Column(db.Float, default=0.0)
    sharpe_ratio = db.Column(db.Float, nullable=True)
    max_drawdown = db.Column(db.Float, nullable=True)
    
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
    
    def to_dict(self):
        return {
            'id': self.id,
            'name': self.name,
            'description': self.description,
            'config': self.get_config(),
            'status': self.status,
            'is_active': self.is_active,
            'total_return': self.total_return,
            'sharpe_ratio': self.sharpe_ratio,
            'max_drawdown': self.max_drawdown,
            'created_at': self.created_at.isoformat() if self.created_at else None,
            'updated_at': self.updated_at.isoformat() if self.updated_at else None,
            'deployed_at': self.deployed_at.isoformat() if self.deployed_at else None
        }

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

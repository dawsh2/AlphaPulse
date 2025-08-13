"""
Database models for AlphaPulse using pure SQLAlchemy (no Flask dependencies)
"""
from sqlalchemy import Column, String, Boolean, DateTime, Float, Text, ForeignKey, Integer
from sqlalchemy.orm import relationship
from datetime import datetime
import uuid
import json
import bcrypt
from database import Base

class User(Base):
    """User accounts and authentication"""
    __tablename__ = 'users'
    
    id = Column(Integer, primary_key=True, index=True)
    email = Column(String(255), unique=True, nullable=False, index=True)
    username = Column(String(100), unique=True, nullable=False)
    password_hash = Column(String(255), nullable=True)
    
    # Account settings
    subscription_tier = Column(String(50), default='free')
    is_active = Column(Boolean, default=True)
    email_verified = Column(Boolean, default=False)
    
    # Timestamps
    created_at = Column(DateTime, default=datetime.utcnow)
    last_login = Column(DateTime)
    
    # Relationships
    strategies = relationship('Strategy', back_populates='user', cascade='all, delete-orphan')
    event_logs = relationship('EventLog', back_populates='user', cascade='all, delete-orphan')
    
    def set_password(self, password: str):
        """Hash and set password"""
        self.password_hash = bcrypt.hashpw(password.encode('utf-8'), bcrypt.gensalt()).decode('utf-8')
    
    def verify_password(self, password: str) -> bool:
        """Verify password against hash"""
        if not self.password_hash:
            return False
        return bcrypt.checkpw(password.encode('utf-8'), self.password_hash.encode('utf-8'))
    
    def to_dict(self):
        return {
            'id': self.id,
            'email': self.email,
            'username': self.username,
            'subscription_tier': self.subscription_tier,
            'is_active': self.is_active,
            'created_at': self.created_at.isoformat() if self.created_at else None
        }

class Strategy(Base):
    """User's trading strategies"""
    __tablename__ = 'strategies'
    
    id = Column(Integer, primary_key=True, index=True)
    user_id = Column(Integer, ForeignKey('users.id'), nullable=False)
    
    # Strategy info
    name = Column(String(255), nullable=False)
    description = Column(Text, nullable=True)
    type = Column(String(50), default='single')  # 'single' or 'ensemble'
    
    # Configuration (stored as JSON)
    config = Column(Text, nullable=False, default='{}')
    parameters = Column(Text, default='{}')  # JSON
    entry_conditions = Column(Text, default='[]')  # JSON array
    exit_conditions = Column(Text, default='[]')  # JSON array
    risk_management = Column(Text, default='{}')  # JSON
    components = Column(Text, default='[]')  # JSON array for ensemble strategies
    
    # Status
    status = Column(String(50), default='draft')  # 'draft', 'testing', 'live', 'paused', 'stopped'
    is_active = Column(Boolean, default=False)
    is_public = Column(Boolean, default=False)
    is_template = Column(Boolean, default=False)
    
    # Performance tracking
    total_return = Column(Float, default=0.0)
    sharpe_ratio = Column(Float, nullable=True)
    max_drawdown = Column(Float, nullable=True)
    backtest_results = Column(Text, default='{}')  # JSON
    
    # Metadata
    tags = Column(Text, default='[]')  # JSON array
    
    # Timestamps
    created_at = Column(DateTime, default=datetime.utcnow)
    updated_at = Column(DateTime, default=datetime.utcnow, onupdate=datetime.utcnow)
    deployed_at = Column(DateTime, nullable=True)
    
    # Relationships
    user = relationship('User', back_populates='strategies')
    event_logs = relationship('EventLog', back_populates='strategy', cascade='all, delete-orphan')
    
    def get_config(self):
        """Parse JSON config"""
        try:
            return json.loads(self.config) if self.config else {}
        except json.JSONDecodeError:
            return {}
    
    def set_config(self, config_dict):
        """Set config from dictionary"""
        self.config = json.dumps(config_dict)
    
    def _parse_json(self, field):
        """Helper to parse JSON fields"""
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

class EventLog(Base):
    """Event logging for trading activities"""
    __tablename__ = 'event_logs'
    
    id = Column(Integer, primary_key=True, index=True)
    user_id = Column(Integer, ForeignKey('users.id'), nullable=False)
    strategy_id = Column(Integer, ForeignKey('strategies.id'), nullable=True)
    
    # Event details
    event_type = Column(String(50), nullable=False)  # 'signal', 'trade', 'risk', 'system', 'error'
    event_data = Column(Text, nullable=True, default='{}')  # JSON string
    message = Column(Text, nullable=True)
    details = Column(Text, nullable=True, default='{}')  # Additional JSON data
    
    # Metadata
    symbol = Column(String(20), nullable=True)
    severity = Column(String(20), default='info')  # 'info', 'warning', 'error', 'critical'
    
    # Timestamp
    created_at = Column(DateTime, default=datetime.utcnow)
    
    # Relationships
    user = relationship('User', back_populates='event_logs')
    strategy = relationship('Strategy', back_populates='event_logs')
    
    def get_event_data(self):
        """Parse JSON event data"""
        try:
            return json.loads(self.event_data) if self.event_data else {}
        except json.JSONDecodeError:
            return {}
    
    def set_event_data(self, data_dict):
        """Set event data from dictionary"""
        self.event_data = json.dumps(data_dict)
    
    def get_details(self):
        """Parse JSON details"""
        try:
            return json.loads(self.details) if self.details else {}
        except json.JSONDecodeError:
            return {}
    
    def to_dict(self):
        return {
            'id': self.id,
            'event_type': self.event_type,
            'event_data': self.get_event_data(),
            'message': self.message,
            'details': self.get_details(),
            'symbol': self.symbol,
            'severity': self.severity,
            'created_at': self.created_at.isoformat() if self.created_at else None
        }
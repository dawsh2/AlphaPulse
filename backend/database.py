"""
Database configuration for FastAPI
Uses SQLAlchemy directly without Flask-SQLAlchemy wrapper
"""
from sqlalchemy import create_engine
from sqlalchemy.ext.declarative import declarative_base
from sqlalchemy.orm import sessionmaker
from config import Config

# Create database engine
engine = create_engine(
    Config.DATABASE_URL,
    connect_args={"check_same_thread": False} if Config.DATABASE_URL.startswith("sqlite") else {},
    echo=False  # Set to True for SQL query logging
)

# Create SessionLocal class
SessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)

# Create Base class for models
Base = declarative_base()
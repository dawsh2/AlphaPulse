"""
AlphaPulse FastAPI Server
Handles business logic, authentication, and Python-specific functionality
High-performance data routes will be handled by Rust/Tokio services
"""
from fastapi import FastAPI, Depends, HTTPException, status
from fastapi.middleware.cors import CORSMiddleware
from fastapi.security import HTTPBearer, HTTPAuthorizationCredentials
from contextlib import asynccontextmanager
from datetime import datetime, timedelta
from typing import Optional
import jwt
import os
import uvicorn

from sqlalchemy.orm import Session
from database import SessionLocal, engine, Base
from models import User, Strategy, EventLog
from config import Config

# Validate configuration on startup
if not Config.validate():
    print("❌ Configuration validation failed. Please set your Alpaca API keys.")
    exit(1)

# Create database tables
Base.metadata.create_all(bind=engine)

@asynccontextmanager
async def lifespan(app: FastAPI):
    """Manage application lifecycle"""
    # Startup
    print("🚀 Starting AlphaPulse FastAPI Server")
    
    # Ensure demo user exists
    db = SessionLocal()
    try:
        demo_user = db.query(User).filter_by(email="demo@alphapulse.io").first()
        if not demo_user:
            demo_user = User(
                email="demo@alphapulse.io",
                username="demo",
                is_active=True
            )
            demo_user.set_password("demo123")
            db.add(demo_user)
            db.commit()
            print("✅ Demo user created")
    finally:
        db.close()
    
    yield
    
    # Shutdown
    print("👋 Shutting down AlphaPulse FastAPI Server")

# Initialize FastAPI app
app = FastAPI(
    title="AlphaPulse API",
    description="Quantitative Trading Platform API",
    version="2.0.0",
    lifespan=lifespan
)

# Configure CORS
app.add_middleware(
    CORSMiddleware,
    allow_origins=Config.CORS_ORIGINS + ["http://localhost:5173", "http://localhost:5175"],  # Add Vite dev servers
    allow_credentials=True,
    allow_methods=["GET", "POST", "PUT", "DELETE", "OPTIONS"],
    allow_headers=["Content-Type", "Authorization"],
)

# Security
security = HTTPBearer()

# Dependency to get database session
def get_db():
    db = SessionLocal()
    try:
        yield db
    finally:
        db.close()

# JWT Token Management
def create_access_token(data: dict, expires_delta: Optional[timedelta] = None):
    """Generate JWT access token"""
    to_encode = data.copy()
    if expires_delta:
        expire = datetime.utcnow() + expires_delta
    else:
        expire = datetime.utcnow() + timedelta(seconds=Config.JWT_ACCESS_TOKEN_EXPIRES)
    to_encode.update({"exp": expire, "iat": datetime.utcnow()})
    encoded_jwt = jwt.encode(to_encode, Config.JWT_SECRET_KEY, algorithm="HS256")
    return encoded_jwt

def verify_token(credentials: HTTPAuthorizationCredentials = Depends(security)) -> dict:
    """Verify JWT token and return payload"""
    token = credentials.credentials
    try:
        payload = jwt.decode(token, Config.JWT_SECRET_KEY, algorithms=["HS256"])
        return payload
    except jwt.ExpiredSignatureError:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Token has expired",
            headers={"WWW-Authenticate": "Bearer"},
        )
    except jwt.InvalidTokenError:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Invalid token",
            headers={"WWW-Authenticate": "Bearer"},
        )

async def get_current_user(
    token_payload: dict = Depends(verify_token),
    db: Session = Depends(get_db)
) -> User:
    """Get current authenticated user"""
    user_id = token_payload.get("user_id")
    if not user_id:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Invalid token payload"
        )
    
    user = db.query(User).filter(User.id == user_id).first()
    if not user or not user.is_active:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="User not found or inactive"
        )
    
    return user

# ================================================================================
# Health Check and System Info
# ================================================================================

@app.get("/api/health")
async def health_check():
    """System health check"""
    return {
        "status": "ok",
        "message": "AlphaPulse FastAPI Server",
        "version": "2.0.0",
        "timestamp": datetime.utcnow().isoformat(),
        "framework": "FastAPI",
        "python_components": [
            "authentication",
            "jupyter_integration", 
            "analysis_engine",
            "strategy_development"
        ],
        "rust_components": [
            "market_data_collectors",
            "websocket_servers",
            "orderbook_processors"
        ]
    }

# ================================================================================
# Authentication Endpoints
# ================================================================================

from pydantic import BaseModel, EmailStr

class LoginRequest(BaseModel):
    email: EmailStr
    password: str

class LoginResponse(BaseModel):
    access_token: str
    token_type: str = "bearer"
    user_id: int
    username: str
    email: str

@app.post("/api/auth/login", response_model=LoginResponse)
async def login(request: LoginRequest, db: Session = Depends(get_db)):
    """User login endpoint"""
    user = db.query(User).filter(User.email == request.email).first()
    
    if not user or not user.verify_password(request.password):
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Invalid email or password"
        )
    
    if not user.is_active:
        raise HTTPException(
            status_code=status.HTTP_403_FORBIDDEN,
            detail="User account is inactive"
        )
    
    # Create access token
    access_token = create_access_token(
        data={"user_id": user.id, "email": user.email}
    )
    
    # Log the login event
    event = EventLog(
        user_id=user.id,
        event_type="LOGIN",
        details={"ip": "127.0.0.1"}  # In production, get real IP
    )
    db.add(event)
    db.commit()
    
    return LoginResponse(
        access_token=access_token,
        user_id=user.id,
        username=user.username,
        email=user.email
    )

@app.post("/api/auth/demo-login", response_model=LoginResponse)
async def demo_login(db: Session = Depends(get_db)):
    """Quick demo login without credentials"""
    user = db.query(User).filter(User.email == "demo@alphapulse.io").first()
    
    if not user:
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail="Demo user not found"
        )
    
    access_token = create_access_token(
        data={"user_id": user.id, "email": user.email}
    )
    
    return LoginResponse(
        access_token=access_token,
        user_id=user.id,
        username=user.username,
        email=user.email
    )

@app.get("/api/auth/me")
async def get_me(current_user: User = Depends(get_current_user)):
    """Get current user info"""
    return {
        "id": current_user.id,
        "email": current_user.email,
        "username": current_user.username,
        "is_active": current_user.is_active,
        "created_at": current_user.created_at.isoformat()
    }

# ================================================================================
# Import API Routers (to be migrated)
# ================================================================================

# Import migrated routers
from api.notebook_routes_fastapi import router as notebook_router
from api.workspace_routes_fastapi import router as workspace_router
from api.terminal_routes_fastapi import router as terminal_router
from api.data_routes_fastapi import router as data_router
from api.metrics_routes import router as metrics_router
from api.dev_routes import router as dev_router

# Include routers
app.include_router(notebook_router)
app.include_router(workspace_router)
app.include_router(terminal_router)
app.include_router(data_router)
app.include_router(metrics_router, prefix="/api")
app.include_router(dev_router)  # WebSocket routes at /ws/dev/*

# ================================================================================
# Deprecated Routes (Will move to Rust/Tokio)
# ================================================================================

@app.get("/api/market-data/{symbol}", deprecated=True)
async def get_market_data(symbol: str):
    """
    DEPRECATED: This endpoint will be moved to Rust/Tokio services
    for better performance with real-time market data.
    """
    return {
        "warning": "This endpoint is deprecated and will be moved to Rust services",
        "symbol": symbol,
        "migration_timeline": "Phase 2 of Rust migration"
    }

@app.get("/api/crypto-data/{symbol}", deprecated=True)
async def get_crypto_data(symbol: str):
    """
    DEPRECATED: High-frequency crypto data will be served by Rust/Tokio
    """
    return {
        "warning": "This endpoint is deprecated and will be moved to Rust services",
        "symbol": symbol,
        "migration_timeline": "Phase 2 of Rust migration"
    }

if __name__ == "__main__":
    # Run with: python app_fastapi.py
    # Or better: uvicorn app_fastapi:app --reload --port 8001
    uvicorn.run(
        "app:app",
        host="0.0.0.0",
        port=8080,  # Using 8080 to avoid conflicts
        reload=True,
        log_level="info"
    )
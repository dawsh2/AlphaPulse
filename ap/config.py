import os
from dotenv import load_dotenv

# Load environment variables from .env file
load_dotenv()

class Config:
    """Application configuration."""
    
    # Flask settings
    SECRET_KEY = os.getenv('SECRET_KEY', 'dev-secret-key-change-in-production')
    FLASK_PORT = int(os.getenv('FLASK_PORT', 5000))
    FLASK_ENV = os.getenv('FLASK_ENV', 'development')
    
    # Database
    DATABASE_URL = os.getenv('DATABASE_URL', 'sqlite:///alphapulse.db')
    
    # Frontend settings
    FRONTEND_URL = os.getenv('FRONTEND_URL', 'http://localhost:5173')
    CORS_ORIGINS = os.getenv('CORS_ORIGINS', 'http://localhost:5173,http://localhost:8000').split(',')
    
    # JWT settings
    JWT_SECRET_KEY = os.getenv('JWT_SECRET_KEY', 'jwt-secret-change-in-production')
    JWT_ACCESS_TOKEN_EXPIRES = int(os.getenv('JWT_ACCESS_TOKEN_EXPIRES', 3600))
    
    # Production flag
    PRODUCTION_MODE = os.getenv('PRODUCTION_MODE', 'false').lower() == 'true'
    
    # Alpaca API settings - Match your .zshrc exactly
    ALPACA_API_KEY = os.getenv('ALPACA_API_KEY')
    ALPACA_SECRET_KEY = os.getenv('ALPACA_API_SECRET')  # This matches your .zshrc variable name
    ALPACA_BASE_URL = os.getenv('ALPACA_BASE_URL', 'https://paper-api.alpaca.markets')
    
    @classmethod
    def validate(cls):
        """Validate that required environment variables are set."""
        # Debug: Print what we're actually reading
        print(f"🔍 Debug - Reading environment variables:")
        print(f"   ALPACA_API_KEY: {cls.ALPACA_API_KEY}")
        print(f"   ALPACA_API_SECRET: {os.getenv('ALPACA_API_SECRET')}")
        print(f"   ALPACA_BASE_URL: {cls.ALPACA_BASE_URL}")
        
        # Check for Alpaca API keys
        missing_keys = []
        if not cls.ALPACA_API_KEY:
            missing_keys.append('ALPACA_API_KEY')
        if not cls.ALPACA_SECRET_KEY:
            missing_keys.append('ALPACA_API_SECRET')  # Updated to match your .zshrc
            
        if missing_keys:
            print("=" * 60)
            print("🔑 MISSING ALPACA API KEYS")
            print("=" * 60)
            print(f"Missing environment variables: {', '.join(missing_keys)}")
            print("\nYour .zshrc should have:")
            print("export ALPACA_API_KEY='your_key_here'")
            print("export ALPACA_API_SECRET='your_secret_here'")  # Updated
            print("export ALPACA_BASE_URL='https://paper-api.alpaca.markets'")
            print("\nAfter updating .zshrc, run: source ~/.zshrc")
            print("Or restart your terminal")
            print("Get your keys from: https://app.alpaca.markets/paper/dashboard/overview")
            print("=" * 60)
            return False
            
        print(f"✅ Alpaca API configured: {cls.ALPACA_BASE_URL}")
        return True
    
    @classmethod
    def get_alpaca_headers(cls):
        """Get headers for Alpaca API requests."""
        return {
            'APCA-API-KEY-ID': cls.ALPACA_API_KEY,
            'APCA-API-SECRET-KEY': cls.ALPACA_SECRET_KEY,
            'Content-Type': 'application/json'
        }

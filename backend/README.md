# AlphaPulse Backend

Flask API server for AlphaPulse trading platform.

## Quick Start

```bash
# Install dependencies
pip install -r requirements.txt

# Set environment variables (add to ~/.zshrc or ~/.bashrc)
export ALPACA_API_KEY="your_key_here"
export ALPACA_API_SECRET="your_secret_here"
export ALPACA_BASE_URL="https://paper-api.alpaca.markets"

# Run the server
python app.py
```

Server runs on http://localhost:5000

## Files

- `app.py` - Main Flask application
- `models.py` - Database models (User, Strategy, EventLog)
- `config.py` - Configuration management
- `alpaca_client.py` - Alpaca API integration
- `nautilus_integration.py` - NautilusTrader integration
- `instance/alphapulse.db` - SQLite database

## API Endpoints

- `GET /api/health` - Health check
- `POST /api/auth/demo-login` - Demo authentication
- `GET /api/account` - Alpaca account info
- `GET /api/positions` - Current positions
- `GET /api/orders` - Order history
- `POST /api/orders` - Submit order
- `GET /api/market-data/<symbol>` - Market data
- `GET /api/events` - Event logs
- `GET /api/strategies` - User strategies
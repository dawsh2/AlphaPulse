# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

AlphaPulse is an event-driven quantitative trading system with a Flask backend and vanilla JavaScript frontend, designed for paper trading with Alpaca Markets. The system emphasizes a clean separation between backend (pulse-engine) and frontend (ui), with real-time market data and trading capabilities.

## Common Development Commands

### Setup
```bash
# Install dependencies (Python 3.8+ required, 3.13 compatible)
python setup.py
# OR
pip install -r pulse-engine/requirements.txt
```

### Run Backend
```bash
cd pulse-engine
python app.py
# Backend runs on port 5000 by default (configurable via FLASK_PORT)
```

### Run Frontend
```bash
cd ui
python -m http.server 8000
# Frontend served at http://localhost:8000
```

### Environment Configuration
The system uses OS environment variables for Alpaca API keys (not stored in .env):
```bash
# Add to ~/.zshrc or ~/.bashrc
export ALPACA_API_KEY="your_key_here"
export ALPACA_API_SECRET="your_secret_here"  # Note: API_SECRET not SECRET_KEY
export ALPACA_BASE_URL="https://paper-api.alpaca.markets"
```

## Architecture

### Backend (pulse-engine/)
- **app.py**: Main Flask application with REST API endpoints
- **models.py**: SQLAlchemy models (User, Strategy, EventLog)
- **alpaca_client.py**: Custom Alpaca API client using requests library
- **config.py**: Configuration management, reads from .env and OS environment

### Frontend (ui/)
- **live-trading.html**: Main trading interface
- **index.html**: Landing page
- Uses TradingView Lightweight Charts for professional charting
- Pure JavaScript, no build process required

### Key Design Patterns
1. **Event-Driven Architecture**: All trading activities logged as events in EventLog table
2. **JWT Authentication**: Token-based auth for API security
3. **Paper Trading Safety**: Defaults to paper trading, requires explicit config for live
4. **Real-time Data**: Direct integration with Alpaca market data APIs

### API Endpoints
- `GET /api/health` - System health check
- `POST /api/auth/demo-login` - Demo authentication
- `GET /api/account` - Alpaca account info
- `GET /api/positions` - Current positions
- `GET /api/orders` - Order history
- `POST /api/orders` - Submit new order
- `GET /api/market-data/<symbol>` - Real-time market data
- `GET /api/events` - Event logs
- `GET /api/strategies` - User strategies

### Database
- SQLite for development (instance/alphapulse.db)
- Models: User, Strategy, EventLog
- Auto-creates demo user on startup
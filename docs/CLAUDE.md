# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

AlphaPulse is a high-performance quantitative trading system with:
- **Backend**: Rust microservices for ultra-low latency market data processing (~10-50μs)
- **Legacy Backend**: Flask/Python for strategy management and Alpaca integration
- **Frontend**: React/TypeScript dashboard for real-time visualization

The system uses Unix domain sockets for inter-process communication, achieving institutional-grade performance suitable for high-frequency trading.

## Common Development Commands

### Setup
```bash
# Rust services (Rust 1.75+ required)
cd backend/rust-services
cargo build --release

# Python backend dependencies (Python 3.8+ required, 3.13 compatible)
cd backend
pip install -r requirements.txt

# Frontend dependencies (Node.js required)
cd frontend
npm install
```

### Run Rust Services (High-Performance Data Pipeline)
```bash
# In separate terminals:
cd backend/rust-services

# Start exchange data collector
./target/release/exchange_collector

# Start relay server
./target/release/relay_server

# Start WebSocket bridge for frontend
./target/release/ws_bridge
```

### Run Python Backend (Legacy)
```bash
cd backend
python app.py
# Backend runs on port 5000 by default (configurable via FLASK_PORT)
```

### Run Frontend
```bash
cd frontend
npm run dev
# Frontend served at http://localhost:5173 (Vite dev server)
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

### High-Performance Data Pipeline (backend/rust-services/)
- **protocol/**: Binary protocol definitions for ultra-low latency
- **exchange_collector/**: Connects to exchange WebSockets, converts to binary
- **relay_server/**: Fan-out distribution with lock-free broadcasting
- **ws_bridge/**: Converts binary to JSON for frontend WebSocket clients
- Uses Unix domain sockets for ~10-50μs inter-process latency

### Python Backend (backend/)
- **app.py**: Main Flask application with REST API endpoints
- **models.py**: SQLAlchemy models (User, Strategy, EventLog)
- **alpaca_client.py**: Custom Alpaca API client using requests library
- **config.py**: Configuration management, reads from .env and OS environment
- **nautilus_integration.py**: Integration with Nautilus Trader engine

### Frontend (frontend/)
- **React/TypeScript SPA**: Modern component-based architecture
- **Vite**: Fast build tool and dev server
- **Pages**: DevelopPage, ResearchPage, ExplorePage, MonitorPage, NewsPage
- **Components**: Modular UI components in /src/components/
- **Features**: Domain-specific components in /src/components/features/
- **Uses TradingView Lightweight Charts**: Professional charting library
- **Monaco Editor**: VSCode-like code editor for strategy development

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
- SQLite for development (backend/instance/alphapulse.db)
- Models: User, Strategy, EventLog
- Auto-creates demo user on startup

### Development Structure
- **archive_ap_legacy/**: Legacy codebase archive
- **backend/**: Flask API server
- **frontend/**: React/TypeScript SPA
- **services/**: Microservices (auth, market-data, news, etc.)
- **docs/**: Documentation and guides
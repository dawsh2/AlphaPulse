# AlphaPulse - Event-Driven Trading System

## Quick Start Guide

### 1. Setup Environment Variables

Copy the example environment file and configure it:

```bash
cp .env.example .env
```

Edit `.env` with your settings:

```bash
# Required for OAuth (get from https://app.alpaca.markets/developers/oauth)
ALPACA_CLIENT_ID=your_alpaca_client_id
ALPACA_CLIENT_SECRET=your_alpaca_client_secret

# Optional - defaults are fine for development
FLASK_PORT=5000
FRONTEND_URL=http://localhost:8000
```

### 2. Install Backend Dependencies

```bash
cd pulse-engine
pip install -r requirements.txt
```

### 3. Start the Backend Server

```bash
cd pulse-engine
python app.py
```

The API server will start on `http://localhost:5000`

### 4. Serve the Frontend

From the `ui` directory:

```bash
# Using Python's built-in server
python -m http.server 8000

# Or using Node.js
npx serve -p 8000

# Or any other static file server
```

The frontend will be available at `http://localhost:8000`

### 5. Connect Alpaca Account

1. Open `http://localhost:8000/live-trading.html`
2. Click "Connect Alpaca" 
3. Authorize AlphaPulse in the Alpaca OAuth flow
4. You'll be redirected back with your account connected

## Architecture Overview

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  ui │    │   pulse-engine  │    │     Alpaca      │
│   (Frontend)    │◄──►│   (Backend)     │◄──►│   (Broker)      │
│                 │    │                 │    │                 │
│ • Live Trading  │    │ • OAuth Flow    │    │ • Market Data   │
│ • Strategy Lab  │    │ • API Proxy     │    │ • Order Exec    │
│ • News Feed     │    │ • Event System  │    │ • Account Info  │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

## Features Implemented

### ✅ OAuth Integration
- Secure Alpaca OAuth flow
- No API keys in frontend code
- Token management and refresh

### ✅ Live Trading Interface
- Real-time account data
- Position monitoring
- Strategy controls
- Event logging

### ✅ Professional Charts
- TradingView Lightweight Charts
- Real-time data capability
- Professional appearance

### ✅ Event-Driven Architecture
- Structured event logging
- Database persistence
- Real-time updates

## Next Steps

1. **Strategy Engine**: Implement the actual trading logic
2. **WebSocket Streaming**: Add real-time market data
3. **Risk Management**: Position sizing, stop losses
4. **Backtesting**: Historical strategy testing
5. **Analytics**: Performance metrics and reporting

## Development

### Database Schema

The system uses SQLite by default with these tables:
- `users` - User accounts
- `broker_accounts` - Connected broker accounts (OAuth tokens)
- `strategies` - Trading strategy configurations
- `event_logs` - System and trading events

### API Endpoints

- `GET /api/health` - System health check
- `POST /api/auth/demo-login` - Demo authentication
- `GET /api/auth/alpaca/connect` - Initiate OAuth
- `GET /auth/alpaca/callback` - OAuth callback
- `GET /api/account` - Account information
- `GET /api/positions` - Current positions
- `GET /api/events` - Event logs

### Security

- JWT tokens for authentication
- OAuth 2.0 for broker connections
- Encrypted token storage
- CORS protection
- Environment variable configuration

## Production Deployment

For production deployment:

1. Set `PRODUCTION_MODE=true` in `.env`
2. Use PostgreSQL instead of SQLite
3. Set up proper SSL certificates
4. Configure domain-specific OAuth redirect URLs
5. Set strong JWT secret keys
6. Enable proper logging and monitoring

## Getting Alpaca OAuth Credentials

1. Go to https://app.alpaca.markets/developers/oauth
2. Create a new OAuth application
3. Set redirect URI to: `http://localhost:5000/auth/alpaca/callback`
4. Copy the Client ID and Client Secret to your `.env` file

## Support

The system is designed to be:
- **Secure**: OAuth instead of API keys
- **Scalable**: Event-driven architecture
- **Professional**: Production-ready UI/UX
- **Extensible**: Modular design for easy expansion
# alphapulse

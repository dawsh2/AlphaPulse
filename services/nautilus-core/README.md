# NautilusTrader Core Service

Core trading engine powered by NautilusTrader.

## Features
- Strategy execution (backtest & live)
- Market data management
- Position & order management
- Risk management
- Performance analytics

## Architecture
```
nautilus-core/
├── src/
│   ├── strategies/     # User strategy implementations
│   ├── backtest/       # Backtesting engine
│   ├── live/           # Live trading connections
│   ├── catalog/        # Data catalog management
│   └── api/            # REST/WebSocket API
├── user-workspaces/    # User files and notebooks
└── requirements.txt
```

## API Endpoints
- `POST /strategies/backtest` - Run backtest
- `POST /strategies/deploy` - Deploy strategy live
- `GET /strategies/{id}/status` - Strategy status
- `GET /catalog/symbols` - Available symbols
- `GET /catalog/data/{symbol}` - Historical data
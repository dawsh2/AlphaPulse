# AlphaPulse API Documentation

## Base URL
- Development: `http://localhost:5001/api`
- Production: `https://api.alphapulse.io/api`

## Authentication
All endpoints except `/auth/*` require JWT token in header:
```
Authorization: Bearer <token>
```

## Core Data Structures

### AnalysisManifest
```typescript
interface AnalysisManifest {
  symbol: string | string[];
  timeframe: '1m' | '5m' | '15m' | '1h' | '1d';
  dateRange: {
    start: string; // ISO 8601
    end: string;   // ISO 8601
  };
  strategy: {
    type: 'trend_following' | 'mean_reversion' | 'momentum' | 'ml_ensemble' | 'custom';
    version: string;
    parameters: {
      [key: string]: number | string | boolean;
    };
  };
  indicators: string[]; // ['RSI', 'MACD', 'BB', etc.]
  features: string[];   // ['returns_1d', 'volume_ratio', etc.]
  hash: string;        // SHA256 of above for caching
}
```

### BacktestResult
```typescript
interface BacktestResult {
  manifest_hash: string;
  metrics: {
    total_return: number;
    annualized_return: number;
    sharpe_ratio: number;
    sortino_ratio: number;
    max_drawdown: number;
    win_rate: number;
    profit_factor: number;
    total_trades: number;
  };
  equity_curve: Array<{
    timestamp: number;
    value: number;
  }>;
  trades: Trade[];
  signals: Signal[];
  cached: boolean;
  computation_time_ms: number;
}
```

### Signal
```typescript
interface Signal {
  timestamp: number;
  symbol: string;
  indicator: string;
  value: number;
  metadata?: Record<string, any>;
}
```

### Trade
```typescript
interface Trade {
  id: string;
  entry_time: number;
  exit_time?: number;
  symbol: string;
  side: 'long' | 'short';
  entry_price: number;
  exit_price?: number;
  quantity: number;
  pnl?: number;
  status: 'open' | 'closed' | 'cancelled';
}
```

## Endpoints

### Authentication

#### POST `/auth/login`
```json
Request:
{
  "email": "user@example.com",
  "password": "password123"
}

Response:
{
  "token": "jwt_token_here",
  "user": {
    "id": "user_123",
    "email": "user@example.com",
    "subscription_tier": "premium"
  }
}
```

#### POST `/auth/demo-login`
```json
Response:
{
  "token": "jwt_token_here",
  "user": {
    "id": "demo_user",
    "email": "demo@alphapulse.com",
    "subscription_tier": "premium"
  }
}
```

### Analysis & Backtesting

#### POST `/analysis/run`
Run analysis with caching support
```json
Request:
{
  "manifest": AnalysisManifest
}

Response:
{
  "result": BacktestResult,
  "cache_hit": boolean,
  "execution_time_ms": number
}
```

#### GET `/analysis/cache/{hash}`
Check if analysis is cached
```json
Response:
{
  "exists": boolean,
  "timestamp": string,
  "ttl_seconds": number
}
```

#### POST `/analysis/signals`
Compute signals without full backtest
```json
Request:
{
  "symbol": "BTC/USD",
  "indicators": ["RSI", "MACD"],
  "timeframe": "1h",
  "start": "2024-01-01T00:00:00Z",
  "end": "2024-01-31T23:59:59Z"
}

Response:
{
  "signals": Signal[],
  "cached": boolean
}
```

### Strategies

#### GET `/strategies`
List user's strategies
```json
Response:
{
  "strategies": [
    {
      "id": "strat_123",
      "name": "My Strategy",
      "type": "trend_following",
      "created_at": "2024-01-01T00:00:00Z",
      "performance": {
        "sharpe": 1.5,
        "total_return": 0.25
      }
    }
  ]
}
```

#### POST `/strategies`
Create new strategy
```json
Request:
{
  "name": "New Strategy",
  "type": "custom",
  "code": "def strategy(data): ...",
  "parameters": {},
  "is_public": false
}

Response:
{
  "id": "strat_456",
  "created_at": "2024-01-15T00:00:00Z"
}
```

#### POST `/strategies/{id}/backtest`
Run backtest for specific strategy
```json
Request:
{
  "symbol": "SPY",
  "timeframe": "1d",
  "start": "2023-01-01T00:00:00Z",
  "end": "2023-12-31T23:59:59Z",
  "parameters": {
    "rsi_period": 14,
    "rsi_oversold": 30
  }
}

Response: BacktestResult
```

#### POST `/strategies/{id}/compile`
Validate strategy code
```json
Request:
{
  "code": "def strategy(data): ..."
}

Response:
{
  "valid": boolean,
  "errors": string[],
  "warnings": string[]
}
```

### Templates & Button-UI

#### GET `/templates/button-ui`
Get available button-UI templates
```json
Response:
{
  "templates": [
    {
      "id": "tmpl_123",
      "name": "RSI Analysis",
      "description": "Analyze RSI conditions",
      "buttons": [
        {
          "label": "Check Oversold",
          "action": "compute_rsi_oversold",
          "parameters": {"threshold": 30}
        }
      ]
    }
  ]
}
```

#### POST `/templates/button-ui`
Save custom button-UI template
```json
Request:
{
  "name": "My Analysis",
  "buttons": [
    {
      "label": "Custom Check",
      "action": "custom_function",
      "code": "def custom_function(data): ..."
    }
  ]
}
```

### Market Data

#### GET `/market-data/{symbol}`
Get historical market data
```json
Parameters:
- timeframe: 1m, 5m, 15m, 1h, 1d
- limit: number of bars (max 10000)

Response:
{
  "symbol": "BTC/USD",
  "timeframe": "1h",
  "bars": [
    {
      "time": 1704067200,
      "open": 42000,
      "high": 42500,
      "low": 41800,
      "close": 42300,
      "volume": 1000
    }
  ]
}
```

#### WebSocket `/ws/market-data`
Real-time market data stream
```json
Subscribe:
{
  "action": "subscribe",
  "symbols": ["BTC/USD", "ETH/USD"]
}

Message:
{
  "type": "trade",
  "symbol": "BTC/USD",
  "price": 42350,
  "volume": 0.5,
  "timestamp": 1704067200
}
```

### Monitoring & Live Trading

#### GET `/positions`
Get current positions
```json
Response:
{
  "positions": [
    {
      "symbol": "BTC/USD",
      "quantity": 0.5,
      "entry_price": 40000,
      "current_price": 42000,
      "pnl": 1000,
      "pnl_percentage": 5
    }
  ]
}
```

#### GET `/orders`
Get orders
```json
Parameters:
- status: open, closed, all
- limit: number

Response:
{
  "orders": [
    {
      "id": "ord_123",
      "symbol": "BTC/USD",
      "side": "buy",
      "quantity": 0.5,
      "type": "limit",
      "price": 41000,
      "status": "open",
      "created_at": "2024-01-15T10:00:00Z"
    }
  ]
}
```

#### POST `/orders`
Submit new order
```json
Request:
{
  "symbol": "BTC/USD",
  "side": "buy",
  "quantity": 0.1,
  "type": "market"
}

Response:
{
  "order_id": "ord_456",
  "status": "submitted"
}
```

#### WebSocket `/ws/events`
Live event stream
```json
Subscribe:
{
  "action": "subscribe",
  "types": ["trade", "signal", "alert"]
}

Message:
{
  "type": "signal",
  "timestamp": 1704067200,
  "strategy": "trend_following",
  "action": "buy",
  "symbol": "BTC/USD",
  "confidence": 0.85
}
```

### Data Management

#### POST `/data/upload`
Upload custom dataset
```json
Request (multipart/form-data):
- file: CSV file
- name: "My Dataset"
- description: "Custom price data"

Response:
{
  "dataset_id": "ds_123",
  "rows": 10000,
  "columns": ["time", "open", "high", "low", "close", "volume"]
}
```

#### GET `/data/datasets`
List available datasets
```json
Response:
{
  "datasets": [
    {
      "id": "ds_123",
      "name": "My Dataset",
      "created_at": "2024-01-01T00:00:00Z",
      "rows": 10000,
      "size_mb": 2.5
    }
  ]
}
```

## Rate Limits

| Tier | Requests/min | Backtest/hour | Concurrent |
|------|-------------|---------------|------------|
| Free | 60 | 10 | 1 |
| Pro | 300 | 100 | 5 |
| Premium | 1000 | Unlimited | 20 |

## Error Responses

```json
{
  "error": {
    "code": "RATE_LIMIT_EXCEEDED",
    "message": "Too many requests",
    "details": {
      "limit": 60,
      "reset_at": "2024-01-15T10:01:00Z"
    }
  }
}
```

Common error codes:
- `UNAUTHORIZED` - Invalid or missing token
- `INVALID_PARAMETERS` - Request validation failed
- `STRATEGY_ERROR` - Strategy compilation/execution error
- `DATA_NOT_FOUND` - Requested data unavailable
- `CACHE_MISS` - No cached result found
- `RATE_LIMIT_EXCEEDED` - Too many requests
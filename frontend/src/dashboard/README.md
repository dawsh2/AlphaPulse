# AlphaPulse Developer Dashboard

A separate real-time monitoring dashboard for the AlphaPulse Rust migration, running on port 5174.

## Features

- **Real-time WebSocket Firehose**: Direct connection to unfiltered data streams
- **Full Orderbook Visualization**: Shows ALL orderbook levels (not just top 20)
- **Trade Stream Monitor**: Live trade flow with volume and direction
- **Data Flow Visualization**: Visual representation of Exchange → Rust → Redis flow
- **System Status**: CPU, Memory, Disk, Network metrics
- **Prometheus Integration**: Direct metrics from `/api/metrics` endpoint
- **Raw Data Viewer**: See the actual WebSocket messages in real-time

## Running the Dashboard

### Option 1: Run both main app and dashboard
```bash
cd frontend
npm run dev
```
- Main app: http://localhost:5173
- Dev dashboard: http://localhost:5174

### Option 2: Run only the dashboard
```bash
cd frontend
npm run dev:dashboard
```
- Dashboard only: http://localhost:5174

### Option 3: Run only the main app
```bash
cd frontend
npm run dev:app
```
- Main app only: http://localhost:5173

## Architecture

The dashboard is completely separate from the main application:
- Separate Vite config (`vite.dashboard.config.ts`)
- Separate entry point (`src/dashboard/index.html`)
- Own styles and components
- Direct WebSocket connection to `/ws/dev/firehose`

## Components

### DataFlowMonitor
Shows real-time data flow from exchanges through Rust collectors to Redis Streams.

### OrderbookVisualizer
Displays complete orderbook depth with:
- All bid/ask levels (configurable, default 50)
- Spread indicator
- Volume imbalance
- Visual depth bars

### TradeStream
Real-time trade ticker showing:
- Timestamp with millisecond precision
- Price and volume
- Buy/sell direction indicators
- Auto-scroll with toggle

### PrometheusMetrics
Fetches and displays metrics from `/api/metrics`:
- HTTP requests
- Trades processed
- Redis operations
- System resources

### WebSocketFirehose
Raw WebSocket message viewer for debugging:
- Filter by message type
- Pause/resume
- JSON formatted output

### SystemStatus
System resource monitoring:
- CPU, Memory, Disk usage
- Network I/O
- Uptime

## WebSocket Protocol

The dashboard expects WebSocket messages at `/ws/dev/firehose` with format:

```json
{
  "type": "trade|orderbook|metrics|system|firehose",
  "data": { ... },
  "timestamp": 1234567890
}
```

For Redis Streams data:
```json
{
  "type": "firehose",
  "streams": {
    "trades:coinbase:BTC-USD": [
      {
        "id": "1234-0",
        "fields": {
          "timestamp": "1234567890",
          "price": "50000.00",
          "volume": "0.01",
          "side": "buy"
        }
      }
    ]
  }
}
```

## Development

The dashboard uses:
- React 18 with TypeScript
- Custom CSS (no heavy frameworks)
- Minimal dependencies
- Monospace font for data display
- Dark theme optimized for monitoring

## Purpose

This dashboard is specifically for monitoring the Rust migration:
1. Verify data flows correctly through Redis Streams
2. Monitor performance improvements
3. Debug issues with full visibility
4. Compare Python vs Rust implementations

Not intended for production use - development only!
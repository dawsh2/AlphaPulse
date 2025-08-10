# AlphaPulse UI

A progressive quantitative trading platform that bridges button-driven analysis and code-based strategy development.

## Overview

AlphaPulse is a modern React-based trading platform that enables users to progress from no-code strategy analysis to full algorithmic trading development, all within the same environment.

## Quick Start

### Prerequisites
- Node.js 18+ 
- Python 3.8+ (for backend)
- Git

### Installation

```bash
# 1. Clone the repository
git clone https://github.com/yourusername/alphapulse.git
cd alphapulse/ap/alphapulse-ui

# 2. Install frontend dependencies
npm install

# 3. Set up environment variables (add to ~/.zshrc or ~/.bashrc)
export ALPACA_API_KEY="your_key_here"
export ALPACA_API_SECRET="your_secret_here"
export ALPACA_BASE_URL="https://paper-api.alpaca.markets"

# 4. Start the backend (in separate terminal)
cd ../  # Go to ap/ directory
python app.py
# Backend runs on http://localhost:5001

# 5. Start the frontend
npm run dev
# Frontend runs on http://localhost:5173
```

## Tech Stack

### Frontend
- **React 18** with TypeScript
- **Vite** - Fast build tool
- **Monaco Editor** - Code editing
- **TradingView Lightweight Charts** - Market visualization
- **IndexedDB** - Local data caching
- **Zustand** - State management
- **CSS Modules** - Scoped styling

### Backend
- **Flask** - REST API (port 5001)
- **NautilusTrader** - Event-driven backtesting engine
- **Alpaca Markets** - Market data and paper trading
- **PostgreSQL** - Strategy and user data
- **Redis** - Signal caching
- **TimescaleDB** - Time-series event data

## Project Structure

```
alphapulse-ui/
├── src/
│   ├── components/      # Reusable UI components
│   │   ├── Layout/      # Main layout wrapper
│   │   ├── Navigation/  # Header navigation  
│   │   ├── MonitorPage/ # Live trading monitor
│   │   ├── StrategyBuilder/ # Strategy workbench
│   │   └── common/      # Shared components
│   ├── pages/           # Main application pages
│   │   ├── HomePage.tsx    # News feed & market summary
│   │   ├── ResearchPage.tsx # Strategy discovery & notebooks
│   │   ├── DevelopPage.tsx  # IDE environment
│   │   └── MonitorPage.tsx  # Live monitoring (wrapper)
│   ├── services/        # API and data services
│   │   ├── exchanges/   # Exchange integrations
│   │   └── data/        # Data storage and caching
│   └── styles/          # Global styles and themes
├── docs/
│   └── ui.md           # Architecture documentation
└── public/             # Static assets
```

## Key Features

### Research Page (Core Innovation)
- **Strategy Cards**: Browse pre-built strategies with live metrics
- **Button-UI Mode**: No-code strategy analysis - click buttons to run analysis
- **Jupyter Notebooks**: Full Python environment for power users
- **Progressive Learning**: Export any button action to code for inspection
- **AI Assistant**: Get guidance on next analysis steps

### Develop Page (Power User IDE)
- **Monaco Editor**: VSCode-like editing experience
- **File Explorer**: Manage strategies and templates
- **Integrated Terminal**: Run backtests and scripts
- **Git Integration**: Version control for strategies
- **Test Runner**: Unit test your strategies

### Monitor Page (Live Trading)
- **Real-time Charts**: Live market data with TradingView charts
- **Bar-by-Bar Replay**: Debug strategies step by step
- **Event Stream**: Track all trading events
- **Performance Metrics**: Live P&L, Sharpe, drawdown

### Home Page
- **Smart News Feed**: Filtered by your positions and strategies
- **Market Summary**: Key indices and metrics
- **Community Comments**: Discuss market events

## Development

### Available Scripts
```bash
npm run dev          # Start development server
npm run build        # Build for production
npm run preview      # Preview production build
npm run lint         # Run ESLint
npm run type-check   # Run TypeScript checks
```

### Testing
```bash
npm run test         # Run test suite
npm run test:watch   # Run tests in watch mode
```

## API Documentation

See [docs/ui.md](docs/ui.md) for detailed API endpoints and data flow.

### Key Endpoints
- `POST /api/analysis/run` - Run analysis with manifest
- `GET /api/strategies` - List user strategies
- `POST /api/strategies/backtest` - Run backtest
- `GET /api/market-data/<symbol>` - Get market data
- `WebSocket /ws/live` - Real-time market updates

## Caching Strategy

AlphaPulse uses multi-layer caching to minimize redundant computations:

```typescript
interface AnalysisManifest {
  symbol: string | string[];
  timeframe: '1m' | '5m' | '15m' | '1h' | '1d';
  dateRange: { start: Date; end: Date };
  strategy: {
    type: string;
    parameters: Record<string, any>;
  };
  indicators: string[];
  hash: string; // SHA256 for cache key
}
```

- **Signal Cache** (Redis): Pre-computed indicators
- **Feature Store** (PostgreSQL): Derived features
- **Backtest Cache** (S3): Complete backtest results
- **Local Cache** (IndexedDB): Recent user analyses

## Deployment

### GitHub Pages
```bash
./deploy_to_site.sh
# Deploys to https://yourusername.github.io/alphapulse-ui
```

### Docker
```bash
docker build -t alphapulse-ui .
docker run -p 3000:3000 alphapulse-ui
```

### Production
```bash
npm run build
# Serve dist/ folder with any static host
```

## Contributing

1. Fork the repository
2. Create feature branch (`git checkout -b feature/amazing-feature`)
3. Commit changes (`git commit -m 'Add amazing feature'`)
4. Push to branch (`git push origin feature/amazing-feature`)
5. Open Pull Request

## License

MIT

## Support

- Documentation: [docs/ui.md](docs/ui.md)
- Issues: [GitHub Issues](https://github.com/yourusername/alphapulse/issues)
- Discord: [Join our community](https://discord.gg/alphapulse)
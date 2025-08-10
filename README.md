# AlphaPulse

Event-driven quantitative trading platform with microservices architecture.

## Quick Start

```bash
# Initial setup
make setup

# Start development environment
make dev

# View logs
make dev-logs

# Stop environment
make dev-stop
```

## Architecture

AlphaPulse uses a microservices architecture for scalability and maintainability:

```
alphapulse/
├── frontend/           # React/TypeScript UI
├── services/          # Microservices
│   ├── gateway/       # API Gateway (Nginx)
│   ├── auth/          # Authentication service
│   ├── market-data/   # Market data service
│   ├── news/          # News & sentiment service
│   ├── social/        # Comments & social features
│   └── nautilus-core/ # NautilusTrader engine
├── infrastructure/    # Docker, K8s configs
├── shared/           # Shared libraries
└── data/            # Persistent data (git-ignored)
```

## Services

### Frontend
- **URL**: http://localhost:3000
- **Tech**: React, TypeScript, Vite
- **Features**: Research workbench, strategy builder, backtesting UI

### API Gateway
- **URL**: http://localhost:80
- **Purpose**: Routes requests to appropriate microservices
- **Tech**: Nginx

### Auth Service
- **Purpose**: User authentication and management
- **Tech**: Python FastAPI, PostgreSQL, JWT

### Market Data Service
- **Purpose**: Real-time and historical market data
- **Tech**: Python, Alpaca API, Redis cache
- **Features**: WebSocket feeds, data caching

### NautilusTrader Core
- **Purpose**: Trading engine for backtesting and live trading
- **Tech**: NautilusTrader, Python
- **Features**: Strategy execution, risk management, performance analytics

## Development

### Prerequisites
- Docker & Docker Compose
- Node.js 18+ (for frontend development)
- Python 3.11+ (for backend development)

### Environment Variables
Copy `.env.example` to `.env` and configure:
```bash
ALPACA_API_KEY=your_key_here
ALPACA_API_SECRET=your_secret_here
ALPACA_BASE_URL=https://paper-api.alpaca.markets
JWT_SECRET=your-secret-key
```

### Common Commands

```bash
# Build all services
make build

# Run tests
make test

# Format code
make format

# Shell into service
make shell-nautilus
make shell-auth

# Database access
make db-shell    # PostgreSQL
make mongo-shell # MongoDB
make redis-cli   # Redis

# Backup data
make backup
```

## Migration from Old Structure

The project has been migrated from `ap/` directory structure to a proper microservices architecture. Old code remains in `ap/` for reference during migration.

### Migration Status
- ✅ Frontend moved to `/frontend`
- ✅ Service directories created
- ✅ Docker Compose configuration
- ✅ Makefile for common commands
- ⏳ Backend service extraction
- ⏳ API Gateway configuration
- ⏳ Service implementations

## Documentation

- [Architecture Overview](./ARCHITECTURE.md)
- [API Documentation](./docs/api/)
- [Development Setup](./docs/development/)
- [Deployment Guide](./docs/deployment/)

## Contributing

1. Create feature branch
2. Make changes
3. Run tests: `make test`
4. Format code: `make format`
5. Submit PR

## License

Proprietary - All rights reserved

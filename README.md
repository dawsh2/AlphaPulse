# AlphaPulse 📈

> High-performance quantitative trading platform with event-driven architecture, real-time market data processing, and integrated research environment.

## 🎯 Overview

AlphaPulse is a hybrid Python/Rust trading system designed for cryptocurrency and equity markets. It combines the performance of Rust for data collection with Python's rich ecosystem for analytics and machine learning.

```
┌─────────────────────────────────────────────────────────────────┐
│                      AlphaPulse Platform                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐     │
│  │   Home   │  │  Develop │  │ Research │  │ Monitor  │     │
│  │  📰 News │  │ 💻 IDE   │  │ 📊 Jupyter│  │ 📈 Charts│     │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘     │
│                                                                 │
│  ┌───────────────────────────────────────────────────────┐    │
│  │              React + TypeScript Frontend              │    │
│  └───────────────────────────────────────────────────────┘    │
│                            │                                   │
│                            ↓ REST + WebSocket                  │
│  ┌───────────────────────────────────────────────────────┐    │
│  │              FastAPI Backend (Python)                 │    │
│  │  ┌─────────────┐  ┌──────────────┐  ┌─────────────┐ │    │
│  │  │  Services   │  │ Repositories │  │   Analytics │ │    │
│  │  │  Business   │←→│   Data       │  │   Jupyter   │ │    │
│  │  │   Logic     │  │   Access     │  │  Backtests  │ │    │
│  │  └─────────────┘  └──────────────┘  └─────────────┘ │    │
│  └───────────────────────────────────────────────────────┘    │
│                            │                                   │
│  ┌───────────────────────────────────────────────────────┐    │
│  │          Data Collection Layer (Migrating to Rust)    │    │
│  │                                                       │    │
│  │  Exchange → Rust Collectors → TimescaleDB → Parquet  │    │
│  │                                      ↓                │    │
│  │                                   DuckDB              │    │
│  │                                 (Analytics)           │    │
│  └───────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

## 🏗️ Architecture

### Data Flow Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     RESEARCH WORKFLOW                        │
│                                                              │
│  Exchanges          Rust Services        Storage            │
│  ┌────────┐        ┌────────────┐      ┌──────────┐       │
│  │Coinbase│───────→│            │      │TimescaleDB│       │
│  └────────┘   WS   │  Collectors│─────→│  (Buffer) │       │
│  ┌────────┐        │            │      └─────┬────┘       │
│  │ Kraken │───────→│  - Trades  │            │ Batch       │
│  └────────┘        │  - Orders  │            ↓ Export      │
│  ┌────────┐        │  - L2 Book │      ┌──────────┐       │
│  │ Alpaca │───────→│            │      │  Parquet │       │
│  └────────┘        └────────────┘      │   Files  │       │
│                                         └─────┬────┘       │
│                                               ↓             │
│                                         ┌──────────┐       │
│                                         │  DuckDB  │       │
│                                         │(Analytics)│       │
│                                         └──────────┘       │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                     TRADING WORKFLOW                         │
│                                                              │
│  Exchanges        NautilusTrader        Execution           │
│  ┌────────┐      ┌──────────────┐     ┌──────────┐        │
│  │Exchange│─────→│   WebSocket   │────→│ Strategy │        │
│  └────────┘  WS  │   Adapters    │     │  Engine  │        │
│                  └──────────────┘     └─────┬────┘        │
│                                              ↓              │
│                                        ┌──────────┐        │
│                                        │  Orders  │        │
│                                        │   Out    │        │
│                                        └──────────┘        │
└─────────────────────────────────────────────────────────────┘
```

### Service Layer Architecture (Repository Pattern)

```python
# Clean separation enables Python → Rust migration without changing business logic

┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   FastAPI    │────→│   Service    │────→│ Repository   │
│   Routes     │     │   Layer      │     │  Interface   │
└──────────────┘     └──────────────┘     └──────┬───────┘
                                                  │
                           ┌──────────────────────┼──────────────────────┐
                           ↓                      ↓                      ↓
                    ┌──────────────┐      ┌──────────────┐      ┌──────────────┐
                    │   Python     │      │    Rust      │      │   Mock       │
                    │   Impl       │      │    Impl      │      │   Impl       │
                    │  (Current)   │      │  (Future)    │      │  (Tests)     │
                    └──────────────┘      └──────────────┘      └──────────────┘

# Example: Swapping implementations with feature flags
if settings.USE_RUST_MARKET_DATA:
    repo = RustMarketDataRepo()  # Fast Rust implementation
else:
    repo = PythonMarketDataRepo()  # Current Python implementation
```

## 🚀 Key Engineering Decisions

### 1. **Hybrid Python/Rust Architecture**

**Decision**: Keep Python for business logic, migrate performance-critical paths to Rust

```
Performance Requirements by Component:

Component               Language    Latency Target    Throughput
─────────────────────────────────────────────────────────────
WebSocket Collectors    Rust        <1ms             10,000 msg/s
Orderbook Processing    Rust        <100μs           100,000 updates/s
Business Logic          Python      <100ms           100 req/s
Analytics/ML            Python      <1s              Batch processing
Jupyter Integration     Python      N/A              Interactive
```

### 2. **Database Strategy: Streaming → Batch → Analytics**

**Decision**: TimescaleDB for streaming buffer, Parquet for storage, DuckDB for analytics

```
┌────────────────────────────────────────────────────────┐
│              Storage Cost & Performance                 │
├────────────────────────────────────────────────────────┤
│                                                        │
│  TimescaleDB (7-day window)                           │
│  ├─ Role: Streaming buffer                            │
│  ├─ Size: ~50GB                                       │
│  └─ Query: <10ms for recent data                      │
│                                                        │
│  Parquet Files (permanent)                            │
│  ├─ Role: Long-term storage                           │
│  ├─ Size: ~5GB (10x compression)                      │
│  └─ Cost: $0.023/GB/month (S3)                        │
│                                                        │
│  DuckDB (in-process)                                  │
│  ├─ Role: Fast analytics                              │
│  ├─ Performance: 100x faster than Postgres            │
│  └─ Location: Runs in Jupyter/Python process          │
└────────────────────────────────────────────────────────┘
```

### 3. **Repository Pattern for Clean Migration**

**Decision**: Abstract data access to enable gradual Python → Rust migration

```python
# Repository interface (unchanged during migration)
class MarketDataRepository(Protocol):
    async def get_trades(self, symbol: str) -> List[Trade]: ...
    async def get_orderbook(self, symbol: str) -> OrderBook: ...

# Service layer (unchanged during migration)
class TradingService:
    def __init__(self, repo: MarketDataRepository):
        self.repo = repo  # Can be Python OR Rust implementation
    
    async def analyze_market(self, symbol: str):
        trades = await self.repo.get_trades(symbol)
        # Business logic remains identical
        return compute_signals(trades)
```

### 4. **Event-Driven Architecture**

**Decision**: All actions logged as events for audit and replay

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Action    │────→│    Event    │────→│  Event Log  │
└─────────────┘     └─────────────┘     └─────────────┘
                           │                    │
                           ↓                    ↓
                    ┌─────────────┐     ┌─────────────┐
                    │  Handlers   │     │   Replay    │
                    └─────────────┘     └─────────────┘
```

## 📊 Performance Targets

| Metric | Current (Python) | Target (Rust) | Improvement |
|--------|-----------------|---------------|-------------|
| WebSocket Throughput | 1,000 msg/s | 10,000 msg/s | 10x |
| Orderbook Updates | 10,000/s | 100,000/s | 10x |
| Message Latency | 10ms | <1ms | 10x |
| Memory Usage | 2GB | 200MB | 10x |
| Database Writes | 1,000/s | 10,000/s | 10x |

## 🛠️ Tech Stack

### Frontend
- **Framework**: React 18 + TypeScript
- **Build**: Vite
- **Charts**: TradingView Lightweight Charts
- **Editor**: Monaco Editor (VSCode)
- **State**: React Context + Hooks

### Backend (Current)
- **API**: FastAPI (migrating from Flask)
- **Async**: asyncio + aiohttp
- **Database**: SQLAlchemy + Alembic
- **WebSocket**: websockets library
- **Auth**: JWT tokens

### Backend (Future)
- **Performance**: Rust + Tokio
- **WebSocket**: tokio-tungstenite
- **Serialization**: Serde + JSON
- **Database**: tokio-postgres + duckdb-rs

### Data Storage
- **Time-Series**: TimescaleDB (7-day buffer)
- **Analytics**: DuckDB + Parquet files
- **Cache**: Redis
- **Application**: SQLite / PostgreSQL

### Infrastructure
- **Monitoring**: Prometheus + Grafana
- **Tracing**: OpenTelemetry
- **Container**: Docker + docker-compose
- **CI/CD**: GitHub Actions

## 🚦 Getting Started

### Prerequisites
```bash
# Python 3.8+ (3.13 compatible)
python --version

# Node.js 18+
node --version

# Rust 1.70+ (for future services)
rustc --version
```

### Environment Setup
```bash
# Required environment variables
export ALPACA_API_KEY="your_key"
export ALPACA_API_SECRET="your_secret"
export ALPACA_BASE_URL="https://paper-api.alpaca.markets"
```

### Quick Start
```bash
# Backend
cd backend
pip install -r requirements.txt
python app.py

# Frontend (new terminal)
cd frontend
npm install
npm run dev

# Access at http://localhost:5173
```

## 📁 Project Structure

```
alphapulse/
├── backend/
│   ├── api/                 # FastAPI routes
│   ├── services/            # Business logic
│   ├── repositories/        # Data access layer
│   ├── analytics/           # Jupyter, ML, backtesting
│   ├── core/               # Models, schemas, config
│   └── tests/              # Test suite
│
├── frontend/
│   ├── src/
│   │   ├── pages/          # Page components
│   │   ├── components/     # Reusable UI
│   │   └── services/       # API clients
│   └── public/             # Static assets
│
├── rust-services/          # Future Rust services
│   ├── collectors/         # Market data collectors
│   └── streaming/          # WebSocket servers
│
└── docs/                   # Documentation
```

## 🔄 Migration Status

Currently migrating from Python to Rust for performance-critical components:

| Phase | Status | Timeline | Description |
|-------|--------|----------|-------------|
| Phase 0 | 🟡 In Progress | 4 weeks | Service layer, monitoring setup |
| Phase 1 | ⏳ Pending | 2 weeks | Rust PoC with single collector |
| Phase 2 | ⏳ Pending | 4 weeks | All collectors in Rust |
| Phase 3 | ⏳ Pending | 3 weeks | WebSocket infrastructure |
| Phase 4 | ⏳ Pending | 2 weeks | Production deployment |

## 📈 Roadmap

- [x] FastAPI migration from Flask
- [x] Repository pattern implementation
- [ ] Prometheus + Grafana monitoring
- [ ] Rust trade collector PoC
- [ ] Complete Rust migration
- [ ] NautilusTrader integration
- [ ] Production deployment
- [ ] ML strategy development

## 🤝 Contributing

See [CONTRIBUTING.md](docs/CONTRIBUTING.md) for development guidelines.

## 📄 License

Proprietary - All rights reserved

## 🔗 Links

- [Architecture Documentation](docs/architecture.md)
- [API Documentation](docs/api.md)
- [Rust Migration Plan](rust-migration.md)
- [Deployment Guide](docs/deployment.md)

---

*Built for speed. Designed for scale. Optimized for profit.*
# AlphaPulse Rust Services

High-performance market data collectors and API server built with Rust and Tokio.

## 🎯 Overview

This is **Phase 1** of the AlphaPulse Rust migration - a proof of concept implementing:

- **WebSocket Collectors**: Real-time data collection from Coinbase and Kraken
- **Redis Streams**: High-throughput message streaming 
- **HTTP API Server**: REST API compatible with Python repository pattern
- **Prometheus Metrics**: Comprehensive monitoring and observability

## 🏗️ Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Coinbase WS   │    │   Kraken WS     │    │   Other APIs    │
└─────────┬───────┘    └─────────┬───────┘    └─────────┬───────┘
          │                      │                      │
          └──────────────────────┼──────────────────────┘
                                 │
                         ┌───────▼───────┐
                         │ Rust Collectors│
                         │  (Tokio/Axum)  │
                         └───────┬───────┘
                                 │
                         ┌───────▼───────┐
                         │ Redis Streams │
                         └───────┬───────┘
                                 │
          ┌─────────────────────┼─────────────────────┐
          │                     │                     │
  ┌───────▼───────┐    ┌────────▼────────┐   ┌───────▼───────┐
  │ Python Backend │    │  Rust API Server│   │   Grafana     │
  │   (FastAPI)    │    │    (Axum)       │   │ (Monitoring)  │
  └────────────────┘    └─────────────────┘   └───────────────┘
```

## 🚀 Quick Start

### Prerequisites

- Rust 1.75+ (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- Redis server running
- Docker (optional)

### Build & Run

```bash
# Clone and build
cd rust-services
./build.sh

# Start Redis (if not running)
redis-server

# Run collectors (terminal 1)
RUST_LOG=info ./target/release/alphapulse-collectors

# Run API server (terminal 2)  
RUST_LOG=info ./target/release/alphapulse-api-server

# Test the API
curl http://localhost:3001/health
curl http://localhost:3001/trades/BTC-USD
```

### With Docker

```bash
# From project root
docker-compose -f docker-compose.yml -f docker-compose.rust.yml up
```

## 📊 Performance Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Throughput** | 10,000+ trades/sec | vs Python ~1,000/sec |
| **Latency** | <1ms p99 | WebSocket → Redis |
| **Memory** | <500MB total | vs Python ~1GB |
| **CPU** | <20% under load | Multi-core efficiency |
| **Reliability** | 99.99% uptime | Zero message loss |

## 🛠️ Configuration

### Environment Variables

```bash
# Redis Configuration
REDIS_URL=redis://localhost:6379

# API Server
API_PORT=3001

# Collectors
BUFFER_SIZE=1000
BATCH_TIMEOUT_MS=100

# Logging
RUST_LOG=alphapulse_collectors=info,alphapulse_api_server=info
```

### Python Integration

Enable Rust services in your Python backend:

```bash
# Backend environment
export USE_RUST_SERVICES=true
export RUST_API_URL=http://localhost:3001
```

The Python backend will automatically switch to using the Rust repository implementation.

## 📡 API Endpoints

The Rust API server implements the same interface as the Python `MarketDataRepository`:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check |
| `/trades/{symbol}` | GET | Get trades for symbol |
| `/trades/{symbol}/recent` | GET | Get recent trades |
| `/ohlcv/{symbol}` | GET | Get OHLCV bars |
| `/symbols/{exchange}` | GET | List available symbols |
| `/summary` | GET | Data summary statistics |
| `/metrics` | GET | Prometheus metrics |

### Example Requests

```bash
# Get recent BTC trades from Coinbase
curl "http://localhost:3001/trades/BTC-USD?exchange=coinbase&limit=10"

# Get ETH OHLCV data
curl "http://localhost:3001/ohlcv/ETH-USD?exchange=coinbase&interval=1m"

# Get Prometheus metrics
curl http://localhost:3001/metrics
```

## 📈 Monitoring

### Prometheus Metrics

The services expose comprehensive metrics:

- **Trades**: `trades_processed_total`, `processing_latency_ms`
- **WebSocket**: `websocket_messages_total`, `websocket_connected`
- **Redis**: `redis_operations_total`, `redis_operation_latency_ms`
- **HTTP**: `http_requests_total`, `http_request_duration_ms`
- **System**: `memory_usage_bytes`, `cpu_usage_percent`

### Grafana Dashboard

Import the dashboard from `/grafana/rust-services-dashboard.json` for real-time monitoring.

## 🧪 Testing

### Unit Tests

```bash
cargo test
```

### Integration Tests

```bash
# Requires Redis running
cargo test --test integration
```

### Load Testing

```bash
# Install k6 for load testing
./scripts/load-test.sh
```

## 🔧 Development

### Project Structure

```
rust-services/
├── common/          # Shared types and utilities
├── collectors/      # WebSocket collectors  
├── api-server/      # HTTP API server
├── Dockerfile       # Multi-stage build
└── build.sh         # Build script
```

### Adding New Exchanges

1. Implement `MarketDataCollector` trait
2. Add exchange-specific message types to `common/types.rs`
3. Register collector in `collectors/main.rs`
4. Update API server to handle new exchange

### Code Quality

```bash
cargo fmt           # Format code
cargo clippy        # Lint code
cargo audit         # Security audit
cargo doc --open    # Generate docs
```

## 🚀 Deployment

### Production Environment

```bash
# Build optimized release
cargo build --release --target x86_64-unknown-linux-musl

# Docker production image
docker build -t alphapulse-rust:latest .

# Deploy with monitoring
docker-compose -f docker-compose.yml -f docker-compose.rust.yml -f docker-compose.monitoring.yml up -d
```

### Health Checks

The services include comprehensive health checks:

- **Collectors**: WebSocket connection status
- **API Server**: Redis connectivity  
- **Metrics**: Service uptime and performance

## 🎯 Phase 1 Success Criteria

- [x] **Rust Collectors**: Coinbase and Kraken WebSocket implementations
- [x] **Redis Streams**: High-throughput message streaming
- [x] **HTTP API**: Python-compatible REST interface
- [x] **Monitoring**: Prometheus metrics and health checks
- [x] **Integration**: Seamless Python backend integration
- [ ] **Performance**: 10x improvement validation
- [ ] **Load Testing**: Zero message loss under stress
- [ ] **Documentation**: Complete setup and operations guide

## 🔮 Phase 2 Preview

Next phase will add:

- **OrderBook Processing**: L2 market depth
- **Multi-Exchange Aggregation**: Cross-exchange analytics
- **gRPC Interface**: High-performance internal communication
- **Database Writers**: Direct TimescaleDB/DuckDB integration
- **Advanced Metrics**: Trading signal detection

## 📚 Resources

- [Rust Migration Plan](../rust-migration.md)
- [AlphaPulse Architecture](../ARCHITECTURE.md)
- [Repository Pattern](../backend/repositories/README.md)
- [Tokio Documentation](https://tokio.rs/)
- [Axum Web Framework](https://docs.rs/axum/)

## 🤝 Contributing

See [CONTRIBUTING.md](../CONTRIBUTING.md) for development guidelines.

## 📝 License

MIT License - see [LICENSE](../LICENSE) for details.

---

**🚀 Built with Rust for maximum performance and reliability**
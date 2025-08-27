# DevOps Procedures for AlphaPulse

## Overview

This document outlines the complete DevOps infrastructure and procedures implemented for AlphaPulse's high-frequency trading system. All procedures are designed to maintain zero-downtime operations during live trading hours with Protocol V2 TLV message integrity.

## Infrastructure Components

### 1. GitHub Actions Deployment Pipeline

**File**: `.github/workflows/deploy.yml`

**Deployment Strategy**: Blue-green deployment for zero-downtime

**Pipeline Stages**:
1. **Pre-deployment Validation**
   ```bash
   # Protocol V2 tests (CRITICAL)
   cargo test --package protocol_v2 --test tlv_parsing
   cargo test --package protocol_v2 --test precision_validation
   
   # Performance benchmarks
   cargo run --bin test_protocol --release
   # Must maintain: >1M msg/s construction, >1.6M msg/s parsing
   ```

2. **Staging Deployment**
   ```bash
   # Deploy to staging environment
   ./scripts/deploy_staging.sh
   
   # Synthetic load testing
   ./scripts/load_test_staging.sh
   ```

3. **Production Switch**
   ```bash
   # Atomic blue-green switch
   ./scripts/switch_production.sh
   
   # Validate new environment
   curl https://prod.alphapulse.com/health
   ```

4. **Monitoring & Rollback**
   ```bash
   # Monitor performance for 5 minutes
   ./scripts/monitor_production.sh
   
   # Automatic rollback if performance degrades
   if performance < baseline; then
       ./scripts/rollback_production.sh
   fi
   ```

**Critical Requirements**:
- Maximum 30-second rollback time
- Zero message loss during switch
- Performance regression detection
- Automatic failover on health check failure

### 2. Health Check System

**Implementation**: `libs/health_check/src/lib.rs`

**HTTP Endpoints**:
- `GET /health` - Simple liveness check
- `GET /ready` - Readiness probe for load balancers  
- `GET /metrics` - Performance metrics (JSON)
- `GET /status` - Detailed diagnostics

**Service Integration**:
```rust
use alphapulse_health_check::{HealthCheckServer, ServiceHealth};

// In service main()
let mut health = ServiceHealth::new("service_name");
health.set_socket_path("/path/to/service.sock");
health.set_health_port(8001);

let health_server = HealthCheckServer::new(health, 8001);
tokio::spawn(async move { health_server.start().await });
```

**Health Check Ports**:
- Market Data Relay: 8001
- Signal Relay: 8002
- Execution Relay: 8003
- Dashboard WebSocket: 8004
- Flash Arbitrage: 8005

### 3. Service Discovery System

**Implementation**: `libs/service_discovery/src/lib.rs`

**Problem Solved**: Eliminated 47+ hardcoded socket paths that created single points of failure.

**Architecture**:
```rust
use alphapulse_service_discovery::{ServiceDiscovery, ServiceConnector};

let discovery = ServiceDiscovery::new().await?;
let stream = discovery.connect_to_service("market_data_relay").await?;
```

**Environment Detection**:
- `ALPHAPULSE_ENV` environment variable
- Container detection (`/.dockerenv`)
- System paths (`/var/run/alphapulse`)
- CI environment (`CI=true`)

**Load Balancing Strategies**:
- `FirstHealthy`: Use first available endpoint
- `RoundRobin`: Distribute across healthy endpoints
- `Priority`: Use lowest priority number

### 4. Environment Management

**Configuration Files**:
```
config/environments/
├── development.toml    # /tmp/alphapulse
├── staging.toml        # /tmp/alphapulse-staging
├── production.toml     # /var/run/alphapulse
├── testing.toml        # /tmp/alphapulse-test
└── docker.toml         # /app/sockets
```

**Environment-Specific Settings**:
```toml
# production.toml
socket_dir = "/var/run/alphapulse"
log_dir = "/var/log/alphapulse"
pid_file = "/var/run/alphapulse/alphapulse.pid"

[services.market_data_relay]
socket_path = "/var/run/alphapulse/market_data.sock"
health_port = 8001
priority = 10
enabled = true
```

## Operational Procedures

### Daily Operations

#### Morning Startup Checklist
```bash
# 1. Check system health
./scripts/health_check_all_services.sh

# 2. Verify market data feeds
curl http://localhost:8001/metrics | jq '.messages_per_second'

# 3. Test service discovery
ALPHAPULSE_ENV=production cargo test --package alphapulse_service_discovery

# 4. Check socket connections
netstat -ln | grep alphapulse
```

#### End-of-Day Procedures
```bash
# 1. Collect performance metrics
./scripts/collect_daily_metrics.sh

# 2. Archive logs
./scripts/archive_daily_logs.sh

# 3. Database maintenance
./scripts/cleanup_old_data.sh

# 4. Health check summary
./scripts/daily_health_report.sh
```

### Deployment Procedures

#### Standard Deployment
```bash
# 1. Pre-deployment validation
git checkout main
cargo test --workspace --release

# 2. Deploy via GitHub Actions
git push origin main
# Monitors deployment at: https://github.com/alphapulse/backend_v2/actions

# 3. Post-deployment verification
./scripts/validate_deployment.sh

# 4. Performance monitoring
./scripts/monitor_performance.sh --duration 10min
```

#### Emergency Deployment
```bash
# 1. Hotfix branch
git checkout -b hotfix/critical-fix
# Make critical changes
git commit -m "hotfix: critical trading issue"

# 2. Fast-track deployment
git push origin hotfix/critical-fix
# Triggers emergency deployment workflow

# 3. Monitor closely
watch -n 1 'curl -s http://localhost:8001/metrics | jq ".messages_per_second"'

# 4. Validate fix
./scripts/validate_hotfix.sh
```

#### Rollback Procedures
```bash
# 1. Automatic rollback (GitHub Actions)
# Triggered automatically on health check failure

# 2. Manual rollback
./scripts/emergency_rollback.sh

# 3. Verify rollback success
./scripts/validate_rollback.sh

# 4. Root cause analysis
./scripts/collect_failure_logs.sh
```

### Monitoring Procedures

#### Real-Time Monitoring
```bash
# 1. Service health dashboard
curl -s http://localhost:8001/status | jq '.'

# 2. Performance metrics
watch -n 5 'curl -s http://localhost:8001/metrics | jq ".messages_per_second,.active_connections"'

# 3. Socket status
watch -n 10 'ls -la /var/run/alphapulse/'

# 4. Log monitoring
tail -f /var/log/alphapulse/market_data_relay.log | grep ERROR
```

#### Performance Analysis
```bash
# 1. TLV message throughput
cargo run --bin test_protocol --release

# 2. Service discovery latency
time curl http://localhost:8001/health

# 3. Memory usage analysis
ps aux | grep alphapulse

# 4. Socket connection analysis
ss -x | grep alphapulse
```

### Testing Procedures

#### End-to-End Pipeline Testing
```bash
# Comprehensive E2E test
./scripts/e2e_pipeline_test.sh

# Expected output:
# ✅ Real data ingestion working
# ✅ TLV message processing working  
# ✅ Market Data Relay operational
# ✅ Health monitoring functional
# 🚀 Pipeline ready for production deployment!
```

#### Health Check Testing
```bash
# Start health check demo
cargo run --example health_check_demo

# Test all endpoints
curl http://127.0.0.1:8001/health    # Should return: {"status": "healthy"}
curl http://127.0.0.1:8001/ready     # Should return: {"ready": true}
curl http://127.0.0.1:8001/metrics   # Should return performance data
curl http://127.0.0.1:8001/status    # Should return detailed status
```

#### Service Discovery Testing
```bash
# Test environment detection
ALPHAPULSE_ENV=development cargo test test_environment_detection

# Test service resolution
ALPHAPULSE_ENV=production cargo test test_service_resolution

# Test connection pooling
cargo test test_connection_pooling
```

#### Performance Regression Testing
```bash
# Run performance benchmarks
cargo bench --baseline master

# Check TLV protocol performance
cargo test --package protocol_v2 --release -- --test-threads=1

# Validate no performance regression
python scripts/check_performance_regression.py
```

## Troubleshooting Guide

### Common Issues

#### Service Discovery Problems
```bash
# Symptom: Service not found errors
Error: Unknown service type: market_data_relay

# Solution: Check service registration
ALPHAPULSE_ENV=development cargo test test_service_types

# Verify configuration
cat config/environments/development.toml
```

#### Health Check Failures
```bash
# Symptom: Health endpoint returning 500
curl http://localhost:8001/health
# Returns: {"status": "unhealthy", "error": "Socket not found"}

# Solution: Check socket existence
ls -la /tmp/alphapulse/
ls -la /var/run/alphapulse/

# Restart service if needed
systemctl restart alphapulse-market-data-relay
```

#### Performance Degradation
```bash
# Symptom: Message throughput below 1M/s
cargo run --bin test_protocol --release
# Shows: 750K msg/s (below threshold)

# Solution: Check system resources
top -p $(pgrep alphapulse)
iostat 1 5
netstat -s

# Analyze bottlenecks
perf record -g cargo run --bin test_protocol --release
perf report
```

#### Socket Connection Issues
```bash
# Symptom: Cannot connect to Unix socket
Error: No such file or directory (os error 2)

# Solution: Check socket permissions and existence
ls -la /tmp/alphapulse/
sudo lsof /tmp/alphapulse/market_data.sock

# Restart socket-creating service
cargo run --release --bin market_data_relay &
```

### Emergency Procedures

#### Complete System Failure
```bash
# 1. Stop all services
./scripts/stop_all_services.sh

# 2. Check system resources
df -h
free -h
dmesg | tail -20

# 3. Clean restart
./scripts/clean_restart_system.sh

# 4. Validate recovery
./scripts/validate_system_recovery.sh
```

#### Data Corruption Detection
```bash
# 1. Check TLV message integrity
cargo test --package protocol_v2 --test integrity

# 2. Validate socket files
./scripts/validate_socket_integrity.sh

# 3. Compare with exchange data
python scripts/compare_with_exchange_data.py --duration 1h

# 4. Rebuild corrupted state if needed
./scripts/rebuild_market_state.sh
```

#### Network Connectivity Issues
```bash
# 1. Check exchange connections
./scripts/test_exchange_connections.sh

# 2. Validate DNS resolution
nslookup stream.polygon.io
nslookup api.kraken.com

# 3. Test WebSocket connections
./scripts/test_websocket_connectivity.sh

# 4. Check firewall rules
iptables -L | grep alphapulse
```

## Performance Benchmarks

### Target Performance

- **TLV Message Construction**: >1M msg/s
- **TLV Message Parsing**: >1.6M msg/s
- **Health Check Response**: <10ms
- **Service Discovery Resolution**: <100μs
- **Deployment Time**: <5 minutes
- **Rollback Time**: <30 seconds

### Monitoring Commands

```bash
# Real-time performance
watch -n 1 'curl -s http://localhost:8001/metrics | jq ".messages_per_second"'

# Historical performance
./scripts/performance_history.sh --last-24h

# Benchmark comparison
cargo bench --save-baseline current
cargo bench --baseline current
```

## Integration with Protocol V2

### TLV Message Compatibility

DevOps infrastructure maintains Protocol V2 invariants:
- 32-byte MessageHeader + variable TLV payload
- Nanosecond timestamp precision 
- Zero precision loss for DEX tokens
- Domain separation (MarketData 1-19, Signals 20-39, Execution 40-79)

### Zero-Copy Operations

Health checks and service discovery don't interfere with hot path:
- <1% overhead on message processing
- Connection pooling prevents allocation churn
- Async operations don't block TLV parsing

## Security Procedures

### Socket Security
```bash
# Set proper permissions
chmod 660 /var/run/alphapulse/*.sock
chown alphapulse:alphapulse /var/run/alphapulse/*.sock

# Monitor socket access
auditctl -w /var/run/alphapulse/ -p rwxa -k alphapulse_sockets
```

### Health Check Security
```bash
# Bind health checks to localhost only
# Configure firewall for external monitoring
iptables -A INPUT -p tcp --dport 8001 -s monitoring_server_ip -j ACCEPT
```

## Maintenance Schedule

### Weekly Maintenance
- [ ] Review deployment pipeline metrics
- [ ] Update service discovery configurations
- [ ] Check health check coverage
- [ ] Analyze performance trends
- [ ] Archive old log files

### Monthly Maintenance
- [ ] Update base Docker images
- [ ] Review security configurations
- [ ] Performance regression analysis
- [ ] Documentation updates
- [ ] Backup configuration files

### Quarterly Maintenance
- [ ] System architecture review
- [ ] Disaster recovery testing
- [ ] Performance optimization review
- [ ] Security audit
- [ ] Infrastructure capacity planning

## References

### Key Files
- Health Check Library: `libs/health_check/src/lib.rs`
- Service Discovery: `libs/service_discovery/src/lib.rs`
- Deployment Pipeline: `.github/workflows/deploy.yml`
- E2E Testing: `scripts/e2e_pipeline_test.sh`
- Environment Configs: `config/environments/*.toml`

### Example Implementations
- Health Demo: `examples/health_check_demo.rs`
- Enhanced Relay: `relays/src/bin/enhanced_signal_relay.rs`
- Launch Scripts: `scripts/launch_enhanced_pipeline.sh`

### Testing Commands
```bash
# Health checks
cargo run --example health_check_demo

# Service discovery
cargo test --package alphapulse_service_discovery

# E2E pipeline
./scripts/e2e_pipeline_test.sh

# Performance validation
cargo run --bin test_protocol --release
```
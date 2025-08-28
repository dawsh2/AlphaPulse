# Torq System Management Scripts

This directory contains the unified control interface for managing the Torq trading system. The main entry point is `manage.sh`, which provides a consistent interface for system lifecycle operations.

## Quick Start

```bash
# Start all services
./scripts/manage.sh up

# Check system status  
./scripts/manage.sh status

# View system logs
./scripts/manage.sh logs

# Stop all services
./scripts/manage.sh down

# Get help
./scripts/manage.sh help
```

## Architecture

```
scripts/
├── manage.sh           # Main dispatcher script (entry point)
├── lib/               # Implementation modules
│   ├── startup.sh     # Service startup logic
│   ├── shutdown.sh    # Graceful shutdown logic
│   ├── status.sh      # System status monitoring
│   └── logs.sh        # Log aggregation and streaming
└── README.md          # This documentation
```

### Design Principles

- **Single Entry Point**: All operations go through `manage.sh` for consistency
- **Modular Architecture**: Logic separated into focused library modules
- **Path Independence**: Scripts work from any directory location
- **Robust Error Handling**: Clear error messages and proper exit codes
- **Production Ready**: Used in development and deployment environments

## Commands

### `manage.sh up` (alias: `start`)
Starts all Torq services in the correct dependency order:
1. Market Data Relay
2. Signal Relay  
3. Execution Relay
4. Exchange Adapters (Polygon, Kraken, Coinbase)
5. Trading Strategies (Flash Arbitrage)
6. Dashboard Services (WebSocket)

**Features:**
- PID tracking for all services
- Automatic log file creation
- Dependency-aware startup order
- Health checks after startup

### `manage.sh down` (alias: `stop`)
Gracefully shuts down all services in reverse order:
- Sends SIGTERM for graceful shutdown
- Waits up to 10 seconds per service
- Force kills unresponsive services (SIGKILL)
- Cleans up PID files and orphaned processes

**Features:**
- Reverse shutdown order (strategies first, relays last)
- Graceful termination with fallback to force kill
- Orphan process cleanup
- Emergency shutdown mode

### `manage.sh restart`
Convenience command that performs:
1. `manage.sh down` - Stop all services
2. 2-second grace period
3. `manage.sh up` - Start all services

### `manage.sh status`
Shows comprehensive system status:
- Service states (RUNNING/STOPPED/DEAD)
- Process IDs for running services
- Resource usage (CPU/Memory) with `--verbose`
- Log file information
- System health warnings
- Recent error summary

**Status Indicators:**
- `● RUNNING` (green) - Service active with valid PID
- `○ STOPPED` (yellow) - Service not running, no PID file
- `✗ DEAD` (red) - PID file exists but process dead

### `manage.sh logs`
Log viewing and streaming:
- Default: Shows last 50 lines from each service
- `--follow` (`-f`): Stream logs in real-time
- Color-coded log levels (ERROR=red, WARN=yellow, INFO=blue, DEBUG=gray)
- Log statistics and health metrics

**Features:**
- Multi-service log aggregation
- Real-time streaming with `tail -f`
- Log rotation and cleanup functions
- Search and filtering capabilities

## Options

All commands support these options:

### Global Options
- `--verbose` (`-v`): Enable detailed output and diagnostics
- `--quiet` (`-q`): Suppress non-error messages
- `--help` (`-h`): Show command help

### Command-Specific Options
- `logs --follow` (`-f`): Follow log output in real-time

## Directory Structure

The scripts automatically create and manage these directories:

```
backend_v2/
├── logs/              # Service log files (auto-created)
│   ├── market_data_relay.log
│   ├── signal_relay.log
│   └── ...
├── .pids/             # Process ID files (auto-created)
│   ├── market_data_relay.pid
│   ├── signal_relay.pid
│   └── ...
└── scripts/
    └── (this directory)
```

## Service Configuration

Services are defined in `scripts/lib/startup.sh` with this structure:

```bash
declare -A SERVICES=(
    ["service_name"]="command_to_run"
    # e.g., ["market_data_relay"]="cargo run --release --bin market_data_relay"
)

SERVICE_ORDER=(
    "market_data_relay"
    "signal_relay"
    # ... dependency order
)
```

## Error Handling

The management system provides robust error handling:

### Exit Codes
- `0` - Success
- `1` - General error (invalid command, missing files)
- `2` - Service operation failed

### Error Recovery
- **Missing library scripts**: Clear error message with path
- **Failed services**: Continue with other services, report failures
- **Permission issues**: Helpful guidance for resolution
- **Stale PID files**: Automatic cleanup and reporting

### Health Monitoring
- Disk space warnings for log directory
- Zombie process detection
- Resource usage monitoring
- Connection health tracking

## Testing

The management scripts include comprehensive test suites:

```bash
# Run unit tests (command validation, basic functionality)
./tests/test_manage_unit.sh

# Run integration tests (full workflow testing)  
./tests/test_manage_integration.sh
```

### Test Coverage
- **Unit Tests**: Command parsing, directory creation, error handling
- **Integration Tests**: Command delegation, environment setup, path resolution
- **Mock Framework**: Safe testing without affecting real services

## Development

### Adding New Services
1. Update `SERVICES` array in `scripts/lib/startup.sh`
2. Add service to `SERVICE_ORDER` array
3. Update `SHUTDOWN_ORDER` in `scripts/lib/shutdown.sh` (reverse order)
4. Test with `manage.sh up/down/status`

### Modifying Library Scripts
- Each library script is self-contained
- Include color codes and helper functions
- Follow existing patterns for consistency
- Test changes with both unit and integration tests

### Debugging
Enable verbose output for troubleshooting:
```bash
./scripts/manage.sh status --verbose
./scripts/manage.sh up --verbose
```

Check log files for service-specific issues:
```bash
./scripts/manage.sh logs --follow
tail -f logs/service_name.log
```

## Integration

### systemd Integration (Future)
The scripts are designed to integrate with systemd:
- Service definitions can call `manage.sh up/down`
- Status checks compatible with systemd health monitoring
- Proper exit codes for service management

### Docker Integration (Future)  
Container orchestration support:
- `manage.sh up` as container entrypoint
- Health checks via `manage.sh status`
- Graceful shutdown with `manage.sh down`

### CI/CD Integration
Use in automated testing and deployment:
```bash
# Start system for testing
./scripts/manage.sh up
sleep 5  # Allow startup time

# Run tests
./tests/run_integration_tests.sh

# Clean shutdown
./scripts/manage.sh down
```

## Troubleshooting

### Common Issues

**Services won't start:**
- Check log files: `./scripts/manage.sh logs`
- Verify binary builds: `cargo build --release`
- Check port conflicts: `lsof -i :PORT`

**Stale PID files:**
- Status shows "DEAD" services
- Scripts auto-clean on restart
- Manual cleanup: `rm -f .pids/*.pid`

**Permission errors:**
- Ensure scripts are executable: `chmod +x scripts/manage.sh`
- Check log directory permissions
- Verify cargo/rust installation

**Services fail to stop:**
- Emergency shutdown: `manage.sh down --force` (future feature)
- Manual cleanup: `pkill -f torq`
- Check for zombie processes: `ps aux | grep -i torq`

### Getting Help

1. Check command help: `./scripts/manage.sh help`
2. Enable verbose output: `./scripts/manage.sh <command> --verbose`
3. Check service logs: `./scripts/manage.sh logs`
4. Run health checks: `./scripts/manage.sh status`

## License

Part of the Torq trading system. See project root for license information.
# Sprint 011: Control Script Pattern - System Management Orchestration
*Sprint Duration: 1 week*
*Objective: Implement unified control script pattern (manage.sh) for streamlined system management*

## Mission Statement
Create a single, user-friendly entry point for managing the entire Torq system lifecycle. Replace scattered scripts with a cohesive `manage.sh` orchestrator that handles startup, shutdown, status monitoring, and log streaming in the correct order with proper PID tracking.

## Core Problems Being Solved
1. **Complex Startup Sequence**: Multiple services need to start in specific order
2. **Manual Process Tracking**: No automatic PID management for running services  
3. **Scattered Scripts**: Management commands spread across multiple files
4. **Log Fragmentation**: Service logs scattered across different locations
5. **Inconsistent Shutdown**: No unified way to gracefully stop all services
6. **Development Friction**: Frontend work blocked by complex service management

## Architecture Overview
```
scripts/
├── manage.sh           # Main orchestrator (user-facing)
└── lib/                # Internal implementation scripts
    ├── startup.sh      # Service startup with PID tracking
    ├── shutdown.sh     # Graceful shutdown using PIDs
    ├── status.sh       # Service status monitoring
    └── logs.sh         # Unified log streaming

Root directories:
├── logs/               # Centralized log storage (gitignored)
└── .pids/              # Process ID tracking (gitignored)
```

## User Experience Design
```bash
# Simple, intuitive commands
./scripts/manage.sh up       # Start entire system
./scripts/manage.sh down     # Stop all services gracefully  
./scripts/manage.sh status   # Check what's running
./scripts/manage.sh logs     # Stream all service logs
./scripts/manage.sh restart  # Full system restart
```

## Task Breakdown

### 🔴 Core Infrastructure

#### CTRL-001: Main Orchestrator Script
**Priority**: CRITICAL
**Estimate**: 3 hours
**Status**: TODO
**Files**: `scripts/manage.sh`

Create the main dispatch script with subcommand routing:
- Command validation and help text
- Directory structure initialization (logs/, .pids/)
- Delegated execution to lib/ scripts
- Error handling and user feedback

**Implementation**:
- [ ] Create manage.sh with case statement dispatch
- [ ] Auto-create support directories (logs/, .pids/)
- [ ] Add usage help and command validation
- [ ] Include safety checks and error reporting
- [ ] Make script executable and add shebang

#### CTRL-002: Service Startup Engine
**Priority**: CRITICAL  
**Estimate**: 4 hours
**Status**: TODO
**Files**: `scripts/lib/startup.sh`

Implement intelligent service startup with dependency ordering:
- Service dependency resolution (market data → signal processing → execution)
- Background process launching with output redirection
- PID capture and storage for each service
- Health checks and startup validation

**Key Services Order**:
1. Market Data Relay (`market_data_relay`)
2. Signal Relay (`signal_relay`)
3. Execution Relay (`execution_relay`)
4. Exchange Collectors (`exchange_collector`)
5. Strategy Services (`flash_arbitrage`)
6. Dashboard Services (`websocket_server`)

**Implementation**:
- [ ] Map service dependency graph
- [ ] Create startup sequence with proper delays
- [ ] Implement PID tracking for all services
- [ ] Add log file redirection per service
- [ ] Include startup health validation
- [ ] Handle startup failures gracefully

#### CTRL-003: Graceful Shutdown System
**Priority**: HIGH
**Estimate**: 2 hours
**Status**: TODO
**Files**: `scripts/lib/shutdown.sh`

Build reliable shutdown using stored PIDs:
- Read PID files from .pids/ directory
- Send SIGTERM for graceful shutdown (escalate to SIGKILL if needed)
- Clean up PID files after successful termination
- Report shutdown status per service

**Implementation**:
- [ ] Iterate through all PID files
- [ ] Send SIGTERM with timeout handling
- [ ] Escalate to SIGKILL for unresponsive processes
- [ ] Clean up PID files after termination
- [ ] Report per-service shutdown status

### 🟡 Observability & Management

#### CTRL-004: Service Status Monitor
**Priority**: HIGH
**Estimate**: 3 hours  
**Status**: TODO
**Files**: `scripts/lib/status.sh`

Create comprehensive service status reporting:
- PID-based process existence checking
- Service health verification (port listening, log activity)
- Resource usage reporting (CPU, memory)
- Formatted status output with service details

**Status Information**:
- Process ID and running state
- Resource consumption (RAM, CPU)
- Port binding status
- Last log activity timestamp
- Service-specific health indicators

**Implementation**:
- [ ] Check process existence from PID files
- [ ] Verify port binding for network services
- [ ] Read recent log entries for activity
- [ ] Format status in readable table
- [ ] Color-coded status indicators

#### CTRL-005: Unified Log Streaming
**Priority**: MEDIUM
**Estimate**: 2 hours
**Status**: TODO  
**Files**: `scripts/lib/logs.sh`

Implement centralized log aggregation:
- Multi-file tail following for all service logs
- Service identification in log output
- Log filtering and searching capabilities
- Timestamped, unified log stream

**Features**:
- Follow all .log files in logs/ directory
- Prefix each line with service name
- Support log filtering by service or pattern
- Handle log rotation and new file creation

**Implementation**:
- [ ] Multi-file tail using tail -f
- [ ] Service name prefixing for log lines
- [ ] Optional filtering by service name
- [ ] Handle dynamic log file creation

### 🟢 Integration & Migration

#### CTRL-006: Legacy Script Migration  
**Priority**: MEDIUM
**Estimate**: 3 hours
**Status**: TODO
**Files**: Existing scripts in `scripts/`

Consolidate existing management scripts:
- Audit current scripts for functionality
- Migrate useful logic to new lib/ structure
- Update documentation and remove duplicates
- Preserve specialized scripts that aren't replaced

**Migration Tasks**:
- [ ] Inventory existing scripts and their purposes
- [ ] Extract reusable logic for lib/ scripts
- [ ] Update any hardcoded paths or assumptions
- [ ] Create migration guide for developers
- [ ] Archive or remove superseded scripts

#### CTRL-007: Directory Structure Setup
**Priority**: LOW
**Estimate**: 1 hour
**Status**: TODO
**Files**: `.gitignore`, directory creation

Establish proper directory structure:
- Create logs/ and .pids/ directories
- Update .gitignore to exclude runtime files
- Set proper permissions on script files
- Document directory purposes

**Implementation**:
- [ ] Create logs/ and .pids/ directories
- [ ] Add logs/ and .pids/ to .gitignore  
- [ ] Set executable permissions on scripts
- [ ] Create README for directory structure

## Implementation Details

### Service Startup Sequence
```bash
# Example from lib/startup.sh
echo "Starting Market Data Relay..."
cargo run --release --bin market_data_relay > "$LOGS_DIR/market_data.log" 2>&1 &
echo $! > "$PIDS_DIR/market_data.pid"

echo "Starting Signal Relay..."  
cargo run --release --bin signal_relay > "$LOGS_DIR/signal.log" 2>&1 &
echo $! > "$PIDS_DIR/signal.pid"

# Continue for all services...
```

### PID-Based Shutdown
```bash
# Example from lib/shutdown.sh
for pidfile in "$PIDS_DIR"/*.pid; do
  if [ -f "$pidfile" ]; then
    pid=$(cat "$pidfile")
    echo "Stopping $(basename "$pidfile" .pid)..."
    kill -TERM "$pid" || echo "Process $pid was not running"
    rm "$pidfile"
  fi
done
```

### Status Reporting Format
```
📊 Torq System Status
===========================
🟢 market_data_relay    PID: 12345  CPU: 2%   RAM: 45MB   Port: 8081 ✓
🟢 signal_relay         PID: 12346  CPU: 1%   RAM: 32MB   Port: 8082 ✓
🔴 execution_relay      Not running
🟡 exchange_collector   PID: 12347  CPU: 5%   RAM: 67MB   No port
...
```

## Validation Requirements
- All services start in correct dependency order
- PID tracking works for background processes
- Graceful shutdown terminates all services cleanly
- Status command shows accurate process state
- Log streaming aggregates all service outputs
- No data loss during restart operations
- Script works from any directory location

## Performance Targets
- **System startup**: <30 seconds for full stack
- **Shutdown time**: <10 seconds graceful termination
- **Status response**: <2 seconds for full system scan
- **Memory overhead**: <10MB for management processes
- **Log aggregation**: Real-time streaming with minimal delay

## Integration Testing
```bash
# Test full lifecycle
./scripts/manage.sh up
sleep 30  # Allow startup
./scripts/manage.sh status  # Verify all running
./scripts/manage.sh down    # Clean shutdown
./scripts/manage.sh status  # Verify all stopped
```

## Documentation Requirements
- User guide for manage.sh commands
- Developer guide for adding new services
- Troubleshooting guide for common issues  
- Architecture documentation for lib/ scripts

## Success Metrics
- **Developer Experience**: 90% reduction in manual service management
- **Startup Reliability**: 100% successful startup rate
- **Shutdown Cleanliness**: Zero orphaned processes after shutdown
- **Status Accuracy**: Real-time process state reflection
- **Log Accessibility**: Single command access to all service logs
- **Maintenance Overhead**: Minimal ongoing maintenance required

## Risk Mitigation
- Preserve existing scripts during transition period
- Test extensively with running services
- Document fallback procedures for script failures
- Implement comprehensive error handling
- Create rollback plan if issues arise

## Dependencies

### Sprint Dependencies  
**Depends On**:
- [x] Sprint 002: Code cleanup - Clean workspace for script organization
- [x] Sprint 003: Data integrity - Stable service foundation

**Provides For**:
- Sprint 012: Production deployment - Unified management for production
- All future sprints: Simplified development workflow

**Conflicts With**:
- No current sprint conflicts identified

## Definition of Done
- manage.sh provides all five core commands (up/down/status/logs/restart)
- All services start reliably in correct order with PID tracking
- Graceful shutdown terminates all processes cleanly
- Status command shows accurate real-time service state
- Unified log streaming works for all services
- Legacy scripts consolidated or properly archived
- Documentation complete for users and developers
- Integration tests pass for full system lifecycle
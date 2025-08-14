# New Collector Implementation Checklist

## 📋 Quick Reference for Adding New Exchanges

Use this checklist when implementing a new exchange collector to ensure consistency with AlphaPulse's ultra-low latency architecture.

## 🎯 Pre-Implementation

### Research Phase
- [ ] **Exchange API Documentation**: Study WebSocket API documentation
- [ ] **Message Formats**: Identify trade and orderbook message structures
- [ ] **Authentication**: Determine if API keys are needed for market data
- [ ] **Rate Limits**: Understand connection and subscription limits
- [ ] **Symbol Format**: Document exchange-specific symbol naming conventions
- [ ] **Orderbook Depth**: Identify available orderbook depth options
- [ ] **Update Frequency**: Determine orderbook update intervals

### Planning Phase
- [ ] **Symbol Mapping**: Plan conversion between standard and exchange formats
- [ ] **Subscription Strategy**: Design trade and orderbook subscription logic  
- [ ] **Message Parsing**: Identify JSON fields for trades and orderbook data
- [ ] **Error Handling**: Plan reconnection and error recovery strategies

## 🏗️ Implementation Phase

### File Structure
- [ ] **Create Collector File**: `collectors/src/{exchange}.rs`
- [ ] **Update Module**: Add to `collectors/src/lib.rs`
- [ ] **Add to Cargo.toml**: Include any exchange-specific dependencies

### Basic Structure
- [ ] **Struct Definition**: Create `{Exchange}Collector` with required fields
- [ ] **Constructor**: Implement `new()` with symbol conversion
- [ ] **Builder Methods**: Add `with_orderbook_sender()`, `with_delta_sender()`, `with_shared_memory_writer()`
- [ ] **Symbol Conversion**: Implement bidirectional symbol conversion methods

### Required Imports
```rust
- [ ] alphapulse_common types (Result, Trade, OrderBookUpdate, etc.)
- [ ] OrderBookTracker and delta types
- [ ] Shared memory types (OrderBookDeltaWriter, SharedOrderBookDelta)
- [ ] Standard async/WebSocket dependencies
- [ ] Logging (tracing)
```

### Data Structures
- [ ] **Trade Message Struct**: Define with serde for JSON parsing
- [ ] **From<TradeMessage> for Trade**: Implement conversion to standard format
- [ ] **Orderbook Parsing**: Handle exchange-specific orderbook format

### WebSocket Implementation
- [ ] **Connection Logic**: Implement `run_collector()` with WebSocket connection
- [ ] **Trade Subscription**: Subscribe to trade streams for all symbols
- [ ] **Orderbook Subscription**: Subscribe to orderbook streams (conditional)
- [ ] **Message Handler**: Implement `handle_message()` for all message types
- [ ] **Reconnection Logic**: Automatic reconnection with exponential backoff

### OrderBook Processing
- [ ] **OrderBook Handler**: Implement `handle_{exchange}_orderbook()`
- [ ] **Snapshot Creation**: Convert to `OrderBookSnapshot` format
- [ ] **Delta Computation**: Use `OrderBookTracker` to compute deltas
- [ ] **Shared Memory**: Write deltas to exchange-specific shared memory path
- [ ] **Broadcasting**: Send deltas via channels for WebSocket clients

### Delta Integration
- [ ] **Delta Conversion**: Implement `convert_to_shared_delta()`
- [ ] **Shared Memory Path**: Use `/tmp/alphapulse_shm/{exchange}_orderbook_deltas`
- [ ] **Buffer Management**: Handle delta buffer overflow gracefully
- [ ] **Compression Logging**: Log compression ratios and performance metrics

### Trait Implementation
- [ ] **MarketDataCollector**: Implement all required trait methods
- [ ] **Error Handling**: Proper Result types and error propagation
- [ ] **Health Monitoring**: Update health status atomically
- [ ] **Metrics**: Record WebSocket and processing metrics

## 🧪 Testing Phase

### Unit Tests
- [ ] **Symbol Conversion**: Test bidirectional symbol mapping
- [ ] **Message Parsing**: Test trade and orderbook message parsing
- [ ] **Data Validation**: Verify parsed data accuracy
- [ ] **Error Cases**: Test invalid message handling

### Integration Tests
- [ ] **Live Connection**: Test WebSocket connection to exchange
- [ ] **Trade Processing**: Verify trade data flows correctly
- [ ] **OrderBook Processing**: Confirm orderbook reconstruction
- [ ] **Delta Compression**: Validate compression ratios
- [ ] **Shared Memory**: Test shared memory write operations

### Performance Tests
- [ ] **Latency**: Measure end-to-end message processing time
- [ ] **Throughput**: Test high-frequency message handling
- [ ] **Memory Usage**: Monitor memory consumption under load
- [ ] **Reconnection**: Test WebSocket reconnection scenarios

## 🔧 WebSocket Server Integration

### Multi-Exchange Support
- [ ] **Delta Reader**: Add `{exchange}_delta_reader()` function to `websocket-server/src/main.rs`
- [ ] **Reader ID**: Assign unique reader ID (increment from existing)
- [ ] **Shared Memory Path**: Use consistent path pattern
- [ ] **Task Spawning**: Add reader task to main function
- [ ] **Error Handling**: Exchange-specific error logging

### Reader Function Template
```rust
async fn {exchange}_delta_reader(
    delta_tx: broadcast::Sender<OrderBookDelta>,
    metrics: Arc<MetricsCollector>,
) {
    // Use reader ID {NEXT_ID}
    // Use path "/tmp/alphapulse_shm/{exchange}_orderbook_deltas"
    // Follow existing pattern from other exchange readers
}
```

## 📊 Validation Phase

### Functional Validation
- [ ] **Trade Accuracy**: Compare trade data with exchange's official feeds
- [ ] **OrderBook Accuracy**: Verify orderbook reconstruction matches exchange
- [ ] **Symbol Mapping**: Confirm all symbols convert correctly
- [ ] **Timestamp Accuracy**: Validate timestamp conversion and precision

### Performance Validation
- [ ] **Target Metrics**:
  - [ ] <10μs shared memory operations
  - [ ] >99% bandwidth reduction through deltas
  - [ ] <1ms end-to-end latency
  - [ ] Automatic reconnection within 5 seconds
- [ ] **Compression Ratios**: Log and verify delta compression effectiveness
- [ ] **Memory Safety**: Run with bounds checking enabled

### Production Readiness
- [ ] **Error Recovery**: Test network disconnection scenarios
- [ ] **Resource Management**: Verify no memory leaks under extended operation
- [ ] **Monitoring**: Confirm metrics are properly recorded
- [ ] **Documentation**: Update collector list and performance benchmarks

## 🚀 Deployment Phase

### Configuration
- [ ] **Environment Variables**: Document any required configuration
- [ ] **Symbol Lists**: Define default symbol sets for the exchange
- [ ] **Resource Limits**: Set appropriate memory and CPU limits

### Monitoring
- [ ] **Health Checks**: Verify health status reporting
- [ ] **Metrics Dashboard**: Add exchange to monitoring dashboards
- [ ] **Alerting**: Set up alerts for connection failures and performance degradation
- [ ] **Logging**: Confirm appropriate log levels and formatting

### Documentation Updates
- [ ] **README**: Add exchange to supported list
- [ ] **Architecture Docs**: Update multi-exchange diagrams
- [ ] **Performance Docs**: Add benchmark results
- [ ] **API Docs**: Document any exchange-specific features

## ⚡ Performance Targets

Each collector must meet these production requirements:

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Shared Memory Latency** | <10μs | Time from WebSocket message to shared memory write |
| **Delta Compression** | >99% | (Delta size / Full orderbook size) * 100 |
| **Message Throughput** | >10k/sec | Messages processed per second |
| **Memory Usage** | <100MB | Peak RSS memory usage |
| **Reconnection Time** | <5s | Time to re-establish connection after failure |
| **Data Accuracy** | 100% | No missing or corrupted trades/orderbook updates |

## 🔍 Code Review Checklist

Before merging:
- [ ] **No unwrap()**: All error cases handled with Result types
- [ ] **Consistent Logging**: Appropriate log levels (info/warn/error/debug)
- [ ] **Memory Safety**: Bounds checking and validation
- [ ] **Performance**: No allocations in hot paths
- [ ] **Documentation**: Code comments for complex exchange-specific logic
- [ ] **Tests**: Comprehensive unit and integration test coverage

## 📖 Reference Implementations

Use these as templates:
- **Full Featured**: `collectors/src/coinbase.rs`
- **Recent Implementation**: `collectors/src/kraken.rs`
- **Binance Format**: `collectors/src/binance_us.rs`

---

Following this checklist ensures new collectors maintain AlphaPulse's ultra-low latency performance standards and integrate seamlessly with the existing multi-exchange infrastructure.
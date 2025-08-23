# Exchange Integration Template

## Template: Adding a New Exchange

This template provides a standardized approach for integrating any new centralized exchange into the AlphaPulse system.

## Step 1: Discovery Phase

### 1.1 API Documentation Analysis
```yaml
# exchanges/NEW_EXCHANGE/docs/api_analysis.yaml
exchange_name: "NewExchange"
api_version: "v3"
documentation_url: "https://docs.newexchange.com/api"

endpoints:
  websocket:
    url: "wss://stream.newexchange.com/v3"
    protocols: ["websocket", "socket.io"]
    ping_interval: 30  # seconds
    reconnect_strategy: "exponential_backoff"
    
  rest:
    base_url: "https://api.newexchange.com/v3"
    rate_limits:
      public: 100  # requests per second
      authenticated: 300
    
authentication:
  type: "api_key"  # or "oauth", "hmac"
  headers:
    - "X-API-KEY"
    - "X-API-SIGNATURE"
    
data_formats:
  timestamp: "unix_ms"  # or "unix_s", "iso8601"
  decimals: "string"  # or "number", "scientific"
  arrays: "nested"  # or "flat"
```

### 1.2 Data Format Mapping
```yaml
# exchanges/NEW_EXCHANGE/docs/field_mapping.yaml
trade_message:
  original_fields:
    price: "p"  # or "price", "last", "rate"
    volume: "v"  # or "volume", "amount", "size", "qty"
    timestamp: "t"  # or "time", "ts", "timestamp"
    side: "s"  # or "side", "type", "direction"
    symbol: "sym"  # or "symbol", "pair", "market"
    
  transformations:
    price:
      source_type: "string"
      source_format: "decimal"
      conversion: "parse_decimal"
      
    volume:
      source_type: "string"
      source_format: "decimal"
      conversion: "parse_decimal"
      
    timestamp:
      source_type: "number"
      source_format: "unix_ms"
      conversion: "ms_to_ns"
      
    side:
      source_type: "string"
      mapping:
        "b": "buy"
        "s": "sell"
        "B": "buy"
        "S": "sell"
        
orderbook_update:
  # Similar mapping for orderbook data
  
ticker_update:
  # Similar mapping for ticker data
```

## Step 2: Implementation

### 2.1 Collector Implementation
```rust
// exchanges/NEW_EXCHANGE/src/collector.rs
use crate::protocol::{TradeMessage, Side};
use crate::exchanges::template::{ExchangeCollector, CollectorConfig};

pub struct NewExchangeCollector {
    config: CollectorConfig,
    ws_client: WebSocketClient,
    normalizer: NewExchangeNormalizer,
}

impl ExchangeCollector for NewExchangeCollector {
    const EXCHANGE_NAME: &'static str = "newexchange";
    const WS_URL: &'static str = "wss://stream.newexchange.com/v3";
    
    async fn connect(&mut self) -> Result<(), CollectorError> {
        // Standard connection logic
        self.ws_client = WebSocketClient::connect(Self::WS_URL).await?;
        self.authenticate().await?;
        self.subscribe_to_streams().await?;
        Ok(())
    }
    
    async fn process_message(&mut self, msg: Message) -> Result<Option<TradeMessage>, Error> {
        // Parse exchange-specific format
        let parsed = self.parse_message(msg)?;
        
        // Normalize to internal format
        let normalized = self.normalizer.normalize_trade(parsed)?;
        
        // Convert to binary protocol
        let binary_msg = self.to_binary_protocol(normalized)?;
        
        Ok(Some(binary_msg))
    }
    
    fn get_heartbeat_interval(&self) -> Duration {
        Duration::from_secs(30)
    }
}
```

### 2.2 Normalizer Implementation
```rust
// exchanges/NEW_EXCHANGE/src/normalizer.rs
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};

pub struct NewExchangeNormalizer;

impl NewExchangeNormalizer {
    pub fn normalize_trade(&self, raw: RawTrade) -> Result<NormalizedTrade, Error> {
        Ok(NormalizedTrade {
            price: self.parse_price(&raw.price)?,
            volume: self.parse_volume(&raw.volume)?,
            timestamp_ns: self.parse_timestamp(&raw.timestamp)?,
            side: self.parse_side(&raw.side)?,
            symbol: self.normalize_symbol(&raw.symbol)?,
        })
    }
    
    fn parse_price(&self, price_str: &str) -> Result<Decimal, Error> {
        // Exchange-specific price parsing
        Decimal::from_str(price_str)
            .map_err(|e| Error::PriceParsing(e))
    }
    
    fn parse_volume(&self, volume_str: &str) -> Result<Decimal, Error> {
        // Handle exchange-specific volume format
        // Some exchanges use base currency, others quote
        Decimal::from_str(volume_str)
            .map_err(|e| Error::VolumeParsing(e))
    }
    
    fn parse_timestamp(&self, ts: &str) -> Result<u64, Error> {
        // Convert exchange timestamp to nanoseconds
        // Handle milliseconds, seconds, or ISO8601
        match self.config.timestamp_format {
            TimestampFormat::UnixMs => {
                let ms: u64 = ts.parse()?;
                Ok(ms * 1_000_000)  // Convert to nanoseconds
            },
            TimestampFormat::UnixS => {
                let s: u64 = ts.parse()?;
                Ok(s * 1_000_000_000)
            },
            TimestampFormat::ISO8601 => {
                let dt = DateTime::parse_from_rfc3339(ts)?;
                Ok(dt.timestamp_nanos() as u64)
            }
        }
    }
    
    fn parse_side(&self, side_str: &str) -> Result<Side, Error> {
        match side_str {
            "b" | "B" | "buy" | "BUY" => Ok(Side::Buy),
            "s" | "S" | "sell" | "SELL" => Ok(Side::Sell),
            _ => Err(Error::UnknownSide(side_str.to_string()))
        }
    }
}
```

## Step 3: Testing

### 3.1 Unit Tests
```rust
// exchanges/NEW_EXCHANGE/src/tests/unit_tests.rs
#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    
    #[test]
    fn test_price_parsing() {
        let normalizer = NewExchangeNormalizer::new();
        
        // Test various price formats
        assert_eq!(normalizer.parse_price("12345.67890"), Ok(dec!(12345.67890)));
        assert_eq!(normalizer.parse_price("0.00000001"), Ok(dec!(0.00000001)));
        assert_eq!(normalizer.parse_price("999999.99999999"), Ok(dec!(999999.99999999)));
        
        // Test edge cases
        assert!(normalizer.parse_price("").is_err());
        assert!(normalizer.parse_price("abc").is_err());
        assert!(normalizer.parse_price("1.2.3").is_err());
    }
    
    #[test]
    fn test_timestamp_conversion() {
        let normalizer = NewExchangeNormalizer::new();
        
        // Test millisecond timestamps
        assert_eq!(
            normalizer.parse_timestamp("1698765432123"),
            Ok(1698765432123000000)
        );
        
        // Test second timestamps
        assert_eq!(
            normalizer.parse_timestamp("1698765432"),
            Ok(1698765432000000000)
        );
        
        // Test ISO8601
        assert_eq!(
            normalizer.parse_timestamp("2024-01-15T10:30:45.123Z"),
            Ok(1705316445123000000)
        );
    }
    
    #[test]
    fn test_side_mapping() {
        let normalizer = NewExchangeNormalizer::new();
        
        // Test all variations
        assert_eq!(normalizer.parse_side("b"), Ok(Side::Buy));
        assert_eq!(normalizer.parse_side("B"), Ok(Side::Buy));
        assert_eq!(normalizer.parse_side("buy"), Ok(Side::Buy));
        assert_eq!(normalizer.parse_side("BUY"), Ok(Side::Buy));
        
        assert_eq!(normalizer.parse_side("s"), Ok(Side::Sell));
        assert_eq!(normalizer.parse_side("S"), Ok(Side::Sell));
        assert_eq!(normalizer.parse_side("sell"), Ok(Side::Sell));
        assert_eq!(normalizer.parse_side("SELL"), Ok(Side::Sell));
        
        // Test invalid
        assert!(normalizer.parse_side("unknown").is_err());
    }
}
```

### 3.2 Integration Tests
```rust
// exchanges/NEW_EXCHANGE/src/tests/integration_tests.rs
#[tokio::test]
async fn test_full_pipeline() {
    // Load fixture data
    let fixture = load_fixture("trades.json");
    
    // Create collector
    let mut collector = NewExchangeCollector::new(test_config());
    
    // Process each message
    for raw_msg in fixture.messages {
        let result = collector.process_message(raw_msg).await;
        
        assert!(result.is_ok());
        
        if let Ok(Some(trade)) = result {
            // Verify binary protocol
            assert_eq!(trade.to_bytes().len(), 48);
            
            // Verify precision preserved
            let restored = TradeMessage::from_bytes(&trade.to_bytes()).unwrap();
            assert_eq!(trade, restored);
        }
    }
}

#[tokio::test]
async fn test_reconnection_logic() {
    let mut collector = NewExchangeCollector::new(test_config());
    
    // Simulate connection
    assert!(collector.connect().await.is_ok());
    
    // Simulate disconnect
    collector.disconnect();
    
    // Should auto-reconnect
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert!(collector.is_connected());
}
```

### 3.3 Data Validation Tests
```python
# exchanges/NEW_EXCHANGE/tests/test_data_validation.py
import pytest
from decimal import Decimal
import json

class TestNewExchangeDataValidation:
    @pytest.fixture
    def sample_messages(self):
        """Load real sample messages from the exchange"""
        with open("fixtures/newexchange_samples.json") as f:
            return json.load(f)
    
    def test_precision_preservation(self, sample_messages):
        """Verify no precision is lost in conversion"""
        for msg in sample_messages:
            original_price = Decimal(msg["price"])
            original_volume = Decimal(msg["volume"])
            
            # Process through pipeline
            binary = convert_to_binary(msg)
            restored = convert_from_binary(binary)
            
            restored_price = Decimal(str(restored["price"]))
            restored_volume = Decimal(str(restored["volume"]))
            
            # Assert precision maintained
            assert abs(original_price - restored_price) < Decimal("0.00000001")
            assert abs(original_volume - restored_volume) < Decimal("0.00000001")
    
    def test_null_field_handling(self):
        """Test handling of missing or null fields"""
        test_cases = [
            {"price": None, "volume": "1.5"},
            {"price": "100.50", "volume": None},
            {"price": "100.50"},  # Missing volume
            {},  # Empty message
        ]
        
        for case in test_cases:
            result = process_message(case)
            
            # Should either handle gracefully or raise specific error
            if result is not None:
                assert is_valid_binary_message(result)
```

## Step 4: Performance Validation

### 4.1 Latency Testing
```rust
// exchanges/NEW_EXCHANGE/benches/latency.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_message_processing(c: &mut Criterion) {
    let collector = NewExchangeCollector::new(config());
    let sample_msg = load_sample_message();
    
    c.bench_function("parse_and_normalize", |b| {
        b.iter(|| {
            collector.process_message(black_box(sample_msg.clone()))
        });
    });
}

fn benchmark_binary_conversion(c: &mut Criterion) {
    let normalized = create_normalized_trade();
    
    c.bench_function("to_binary_protocol", |b| {
        b.iter(|| {
            to_binary_protocol(black_box(&normalized))
        });
    });
}

criterion_group!(benches, benchmark_message_processing, benchmark_binary_conversion);
criterion_main!(benches);
```

### 4.2 Throughput Testing
```python
# exchanges/NEW_EXCHANGE/tests/test_performance.py
import asyncio
import time

async def test_throughput():
    """Test maximum message throughput"""
    collector = NewExchangeCollector()
    await collector.connect()
    
    message_count = 0
    start_time = time.time()
    
    # Run for 60 seconds
    while time.time() - start_time < 60:
        msg = await collector.receive_message()
        if msg:
            message_count += 1
    
    throughput = message_count / 60
    print(f"Throughput: {throughput} messages/second")
    
    # Assert minimum throughput
    assert throughput > 1000, f"Throughput {throughput} below minimum 1000 msg/s"
```

## Step 5: Documentation

### 5.1 Auto-Generated Documentation
```python
# exchanges/NEW_EXCHANGE/docs/generate_docs.py
import yaml
import json
from pathlib import Path

def generate_documentation():
    """Generate documentation from config and code"""
    
    # Load configuration
    with open("config.yaml") as f:
        config = yaml.safe_load(f)
    
    # Generate field mapping documentation
    mapping_doc = f"""
# {config['exchange_name']} Field Mappings

## WebSocket Endpoint
- URL: `{config['websocket']['url']}`
- Ping Interval: {config['websocket']['ping_interval']}s

## Message Fields

### Trade Message
| Our Field | Exchange Field | Type | Example |
|-----------|---------------|------|---------|
"""
    
    for our_field, their_field in config['field_mappings']['trade'].items():
        field_info = config['field_details'][their_field]
        mapping_doc += f"| {our_field} | {their_field} | {field_info['type']} | {field_info['example']} |\n"
    
    # Save documentation
    Path("README.md").write_text(mapping_doc)
    
    # Generate test fixtures from real data
    generate_test_fixtures()
    
    # Generate integration checklist
    generate_checklist()
```

### 5.2 Integration Checklist
```markdown
# NEW_EXCHANGE Integration Checklist

## Discovery Phase
- [ ] API documentation reviewed
- [ ] Authentication mechanism understood
- [ ] Rate limits documented
- [ ] Data formats mapped
- [ ] Sample data collected

## Implementation Phase
- [ ] Collector implemented using template
- [ ] Normalizer handles all field types
- [ ] Binary protocol conversion working
- [ ] Reconnection logic implemented
- [ ] Error handling comprehensive

## Testing Phase
- [ ] Unit tests passing (100% coverage)
- [ ] Integration tests passing
- [ ] Data validation tests passing
- [ ] Performance benchmarks met
  - [ ] Latency <35μs
  - [ ] Throughput >1000 msg/s
- [ ] Memory usage stable
- [ ] No precision loss confirmed

## Documentation Phase
- [ ] Field mappings documented
- [ ] API quirks documented
- [ ] Configuration examples provided
- [ ] Troubleshooting guide created
- [ ] Team wiki updated

## Deployment Phase
- [ ] Staging environment tested
- [ ] Monitoring configured
- [ ] Alerts set up
- [ ] Production deployment successful
- [ ] Post-deployment validation complete

## Sign-off
- [ ] Engineering review
- [ ] QA validation
- [ ] Operations approval
- [ ] Documentation complete
```

## Step 6: Configuration

### 6.1 Standard Configuration Template
```yaml
# exchanges/NEW_EXCHANGE/config/default.yaml
exchange:
  name: "newexchange"
  display_name: "New Exchange"
  
connection:
  websocket:
    url: "wss://stream.newexchange.com/v3"
    reconnect:
      enabled: true
      initial_delay: 1000  # ms
      max_delay: 30000  # ms
      max_attempts: -1  # infinite
    
    ping:
      enabled: true
      interval: 30000  # ms
      timeout: 5000  # ms
    
  rest:
    base_url: "https://api.newexchange.com/v3"
    timeout: 10000  # ms
    
authentication:
  required: true
  type: "api_key"
  credentials:
    api_key: "${NEW_EXCHANGE_API_KEY}"
    api_secret: "${NEW_EXCHANGE_API_SECRET}"
    
subscriptions:
  trades:
    enabled: true
    symbols: ["BTC-USD", "ETH-USD"]
    
  orderbook:
    enabled: false
    depth: 20
    
  ticker:
    enabled: false
    interval: 1000  # ms
    
data_processing:
  normalize_symbols: true
  decimal_places: 8
  timestamp_unit: "ms"  # ms, s, or ns
  
monitoring:
  metrics:
    enabled: true
    prefix: "exchange.newexchange"
    
  logging:
    level: "info"
    format: "json"
    
  health_check:
    enabled: true
    interval: 60000  # ms
```

This template ensures every new exchange integration follows the same high standards and can be completed efficiently with comprehensive testing.
# Protocol V2 Adapter Implementation Guide

This guide helps developers add new exchange adapters to the Protocol V2 system.

## Quick Start

New adapters go in: `services_v2/adapters/your_exchange/`

## Adapter Checklist

- [ ] Create adapter module in `services_v2/adapters/`
- [ ] Implement WebSocket connection with auto-reconnect
- [ ] Map exchange data to Protocol V2 TLV types
- [ ] Use `TLVMessageBuilder` for message construction
- [ ] Connect to appropriate relay based on domain
- [ ] Add integration tests with real connections
- [ ] Document exchange-specific quirks

## Minimal Adapter Template

```rust
use alphapulse_protocol_v2::{
    TLVMessageBuilder, TLVType, TradeTLV, InstrumentId, VenueId,
    RelayDomain, SourceType, MARKET_DATA_RELAY_PATH,
};
use tokio::net::UnixStream;
use tokio_tungstenite::{connect_async, WebSocketStream};

pub struct YourExchangeAdapter {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
    relay: UnixStream,
}

impl YourExchangeAdapter {
    pub async fn new(ws_url: &str) -> Result<Self> {
        // Connect to exchange WebSocket
        let (ws, _) = connect_async(ws_url).await?;
        
        // Connect to relay
        let relay = UnixStream::connect(MARKET_DATA_RELAY_PATH).await?;
        
        Ok(Self { ws, relay })
    }
    
    pub async fn run(&mut self) -> Result<()> {
        while let Some(msg) = self.ws.next().await {
            let msg = msg?;
            self.process_message(msg).await?;
        }
        Ok(())
    }
    
    async fn process_message(&mut self, msg: Message) -> Result<()> {
        // Parse exchange-specific format
        let exchange_data = parse_exchange_format(&msg)?;
        
        // Map to Protocol V2 types
        let instrument_id = self.map_instrument(&exchange_data.symbol)?;
        
        // Create TLV message
        let trade = TradeTLV::new(
            instrument_id,
            exchange_data.price,  // Convert to fixed8
            exchange_data.volume, // Keep native precision
            timestamp_ns(),
            exchange_data.is_buy,
        );
        
        // Build and send
        let message = TLVMessageBuilder::new(
            RelayDomain::MarketData,
            SourceType::YourExchange,
        )
        .add_tlv(TLVType::Trade, &trade)
        .build();
        
        self.relay.write_all(message.as_bytes()).await?;
        Ok(())
    }
    
    fn map_instrument(&self, symbol: &str) -> Result<InstrumentId> {
        // Map exchange symbols to Protocol V2 InstrumentIds
        match symbol {
            "BTC-USD" => Ok(InstrumentId::coin(VenueId::YourExchange, "BTC")),
            "ETH-USD" => Ok(InstrumentId::coin(VenueId::YourExchange, "ETH")),
            _ => Err(Error::UnknownSymbol(symbol.to_string())),
        }
    }
}
```

## Key Protocol V2 APIs

### InstrumentId Creation
```rust
// Available constructors (complete list):
InstrumentId::coin(venue, symbol)           // Crypto coins
InstrumentId::stock(exchange, symbol)       // Stocks  
InstrumentId::bond(exchange, symbol)        // Bonds
InstrumentId::ethereum_token(address)       // ERC-20
InstrumentId::polygon_token(address)        // Polygon tokens
InstrumentId::bsc_token(address)            // BSC tokens
InstrumentId::arbitrum_token(address)       // Arbitrum tokens
InstrumentId::pool(dex, token0, token1)     // DEX pools
InstrumentId::from_u64(id)                  // Raw numeric
```

### TLV Message Types
```rust
// Market Data Domain (1-19) → MarketDataRelay
TLVType::Trade        // Executed trades
TLVType::Quote        // Bid/ask quotes
TLVType::OrderBook    // Full order book
TLVType::PoolSwap     // DEX swaps
TLVType::PoolState    // Pool reserves

// Signal Domain (20-39) → SignalRelay  
TLVType::SignalIdentity  // Trading signals
TLVType::Economics       // Economic events

// Execution Domain (40-59) → ExecutionRelay
TLVType::OrderRequest    // Order submissions
TLVType::Fill           // Execution fills
```

### Message Building Pattern
```rust
let message = TLVMessageBuilder::new(domain, source)
    .add_tlv(tlv_type, &tlv_data)
    .build();

let bytes = message.as_bytes();  // Zero-copy serialization
```

## Common Patterns

### Reconnection Logic
```rust
loop {
    match self.connect().await {
        Ok(mut adapter) => {
            if let Err(e) = adapter.run().await {
                error!("Adapter error: {}", e);
            }
        }
        Err(e) => {
            error!("Connection failed: {}", e);
        }
    }
    tokio::time::sleep(Duration::from_secs(5)).await;
}
```

### Precision Handling
```rust
// Traditional exchanges: Convert USD to 8-decimal fixed-point
let price_fixed8 = (price_float * 100_000_000.0) as i64;

// DEX: Preserve native token precision
let weth_amount = amount_str.parse::<i64>()?;  // 18 decimals
let usdc_amount = amount_str.parse::<i64>()?;  // 6 decimals
```

### Error Handling
```rust
#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tungstenite::Error),
    
    #[error("Unknown symbol: {0}")]
    UnknownSymbol(String),
    
    #[error("Parse error: {0}")]
    Parse(String),
}
```

## Testing Your Adapter

```rust
#[tokio::test]
async fn test_real_connection() {
    // Always test with real connections
    let adapter = YourExchangeAdapter::new("wss://real.exchange.com").await.unwrap();
    
    // Process at least one real message
    let msg = adapter.ws.next().await.unwrap().unwrap();
    adapter.process_message(msg).await.unwrap();
}
```

## Performance Targets

- Message construction: >1M msg/s
- Parsing: >1.6M msg/s  
- Processing latency: <35μs
- Memory usage: <50MB per adapter

## Documentation

Run `cargo doc --open` to see the complete Protocol V2 API documentation with all available types and methods.
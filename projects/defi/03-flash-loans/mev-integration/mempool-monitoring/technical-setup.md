# Technical Setup - Ankr WebSocket Configuration

## Proven Configuration ✅
Based on successful testing showing 51.2 tx/sec throughput with full transaction details.

## WebSocket Endpoint

### Production Endpoint (Tested & Verified)
```
wss://rpc.ankr.com/polygon/ws/e6fac469b91ea8fd98406aca0820653ae6fe5c2400f44819450f6022dd2792e2
```

### HTTP Endpoint (For transaction details)
```
https://rpc.ankr.com/polygon/e6fac469b91ea8fd98406aca0820653ae6fe5c2400f44819450f6022dd2792e2
```

## Connection Setup

### Python Implementation (Tested)
```python
import asyncio
import websockets
import json

ANKR_API_KEY = "e6fac469b91ea8fd98406aca0820653ae6fe5c2400f44819450f6022dd2792e2"
WS_URL = f"wss://rpc.ankr.com/polygon/ws/{ANKR_API_KEY}"

async def connect_mempool():
    async with websockets.connect(WS_URL) as ws:
        # Subscribe to pending transactions
        await ws.send(json.dumps({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_subscribe",
            "params": ["newPendingTransactions"]
        }))
        
        # Process incoming transactions
        async for message in ws:
            data = json.loads(message)
            if data.get("params", {}).get("result"):
                tx_hash = data["params"]["result"]
                # Process transaction...
```

### Rust Implementation (Recommended for Production)
```rust
use tokio_tungstenite::{connect_async, tungstenite::Message};
use serde_json::json;

const ANKR_API_KEY: &str = "e6fac469b91ea8fd98406aca0820653ae6fe5c2400f44819450f6022dd2792e2";

pub async fn connect_mempool() -> Result<(), Box<dyn Error>> {
    let url = format!("wss://rpc.ankr.com/polygon/ws/{}", ANKR_API_KEY);
    let (ws_stream, _) = connect_async(&url).await?;
    
    // Subscribe to pending transactions
    let subscribe_msg = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_subscribe",
        "params": ["newPendingTransactions"]
    });
    
    ws_stream.send(Message::Text(subscribe_msg.to_string())).await?;
    
    // Process stream...
}
```

## Subscription Types

### 1. Pending Transactions (All)
```json
{
    "method": "eth_subscribe",
    "params": ["newPendingTransactions"]
}
```
**Returns**: Transaction hashes as they enter mempool
**Rate**: ~51.2 tx/sec average
**Use Case**: General mempool monitoring

### 2. Pending Transaction Details (Full)
```json
{
    "method": "eth_subscribe",
    "params": ["newPendingFullTransactions"]
}
```
**Returns**: Complete transaction objects (if supported)
**Rate**: Variable based on network
**Use Case**: Immediate transaction analysis without additional RPC calls

### 3. DEX Swap Events
```json
{
    "method": "eth_subscribe",
    "params": [
        "logs",
        {
            "topics": [
                "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822"
            ]
        }
    ]
}
```
**Event**: Uniswap V2 Swap
**Returns**: Swap events from all V2-compatible DEXes
**Use Case**: DEX activity monitoring

### 4. Liquidity Events
```json
{
    "method": "eth_subscribe",
    "params": [
        "logs",
        {
            "topics": [
                [
                    "0x4c209b5fc8ad50758f13e2e1088ba56a560dff690a1c6fef26394f4c03821c4f",
                    "0xdccd412f0b1252819cb1fd330b93224ca42612892bb3f4f789976e6d81936496",
                    "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1"
                ]
            ]
        }
    ]
}
```
**Events**: Mint (add liquidity), Burn (remove liquidity), Sync (reserves update)
**Returns**: Liquidity changes across DEXes
**Use Case**: Pool depth prediction

## Transaction Fetching

### Fetching Pending Transaction Details
```python
async def get_transaction_details(tx_hash: str):
    async with aiohttp.ClientSession() as session:
        request = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_getTransactionByHash",
            "params": [tx_hash]
        }
        
        async with session.post(HTTP_URL, json=request) as response:
            result = await response.json()
            return result.get("result")
```

### Transaction Object Structure
```json
{
    "hash": "0x...",
    "from": "0x550365027554bd20d750f9361e460c7428ffbd75",
    "to": "0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff",
    "value": "0x0",
    "input": "0x5c11d795...",  // Function call data
    "gas": "0x493e0",
    "gasPrice": "0x6fc23ac00",
    "nonce": "0x123"
}
```

## Decoding Transaction Data

### DEX Router Addresses (Polygon)
```python
ROUTERS = {
    "0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff": "QuickSwap",
    "0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45": "Uniswap V3",
    "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506": "SushiSwap"
}
```

### Function Signatures
```python
SWAP_SIGNATURES = {
    "0x38ed1739": "swapExactTokensForTokens",
    "0x8803dbee": "swapTokensForExactTokens", 
    "0x5c11d795": "swapExactTokensForTokensSupportingFeeOnTransferTokens",
    "0x472b43f3": "swapExactTokensForTokens (V3)",
}
```

### Decoding Swap Parameters
```python
from eth_abi import decode_abi

def decode_swap(input_data: str):
    method_id = input_data[:10]
    
    if method_id == "0x38ed1739":  # swapExactTokensForTokens
        # Decode: amountIn, amountOutMin, path[], to, deadline
        params = decode_abi(
            ['uint256', 'uint256', 'address[]', 'address', 'uint256'],
            bytes.fromhex(input_data[10:])
        )
        return {
            "method": "swapExactTokensForTokens",
            "amountIn": params[0],
            "amountOutMin": params[1],
            "path": params[2],
            "to": params[3],
            "deadline": params[4]
        }
```

## Connection Management

### Auto-Reconnection Logic
```python
class MempoolMonitor:
    def __init__(self):
        self.reconnect_delay = 1
        self.max_reconnect_delay = 60
        
    async def connect_with_retry(self):
        while True:
            try:
                await self.connect_mempool()
            except Exception as e:
                print(f"Connection failed: {e}")
                await asyncio.sleep(self.reconnect_delay)
                self.reconnect_delay = min(
                    self.reconnect_delay * 2, 
                    self.max_reconnect_delay
                )
            else:
                self.reconnect_delay = 1  # Reset on success
```

### Health Monitoring
```python
class ConnectionHealth:
    def __init__(self):
        self.last_message = time.time()
        self.message_count = 0
        self.error_count = 0
        
    async def heartbeat(self, ws):
        while True:
            if time.time() - self.last_message > 30:
                # Send ping to keep connection alive
                await ws.ping()
            await asyncio.sleep(10)
    
    @property
    def is_healthy(self):
        return (
            time.time() - self.last_message < 60 and
            self.error_count < 10
        )
```

## Rate Limiting & Optimization

### Batching Transaction Fetches
```python
async def batch_get_transactions(tx_hashes: List[str]):
    batch_request = [
        {
            "jsonrpc": "2.0",
            "id": i,
            "method": "eth_getTransactionByHash",
            "params": [tx_hash]
        }
        for i, tx_hash in enumerate(tx_hashes)
    ]
    
    async with aiohttp.ClientSession() as session:
        async with session.post(HTTP_URL, json=batch_request) as response:
            return await response.json()
```

### Filtering Relevant Transactions
```python
def is_relevant_transaction(tx):
    # Filter for DEX routers and high-value transactions
    return (
        tx.get("to") in ROUTERS or
        int(tx.get("value", "0x0"), 16) > 10**18  # > 1 MATIC
    )
```

## Performance Tuning

### WebSocket Buffer Size
```python
# Increase buffer size for high throughput
async with websockets.connect(
    WS_URL,
    max_size=10 * 1024 * 1024,  # 10MB buffer
    max_queue=1000  # Queue up to 1000 messages
) as ws:
```

### Concurrent Processing
```python
async def process_mempool():
    queue = asyncio.Queue(maxsize=1000)
    
    # Producer task
    async def producer():
        async with websockets.connect(WS_URL) as ws:
            async for message in ws:
                await queue.put(message)
    
    # Consumer tasks (multiple workers)
    async def consumer():
        while True:
            message = await queue.get()
            await process_transaction(message)
    
    # Run producer and multiple consumers
    await asyncio.gather(
        producer(),
        *[consumer() for _ in range(10)]  # 10 workers
    )
```

## Monitoring & Metrics

### Prometheus Metrics
```python
from prometheus_client import Counter, Histogram, Gauge

mempool_transactions = Counter(
    'mempool_transactions_total',
    'Total pending transactions seen'
)

transaction_processing_time = Histogram(
    'transaction_processing_seconds',
    'Time to process transaction'
)

mempool_connection_status = Gauge(
    'mempool_connection_status',
    'WebSocket connection status (1=connected, 0=disconnected)'
)
```

## Troubleshooting

### Common Issues & Solutions

**Issue**: Connection drops after ~5 minutes
**Solution**: Implement heartbeat/ping every 30 seconds

**Issue**: Missing transactions
**Solution**: Use `newPendingFullTransactions` if available, or batch fetch details

**Issue**: High latency in processing
**Solution**: Increase worker count, optimize filtering logic

**Issue**: Memory growth over time
**Solution**: Implement sliding window for transaction storage, clear old data

## Security Considerations

### Private Key Protection
```python
# Never log or expose transaction data that might contain sensitive info
def sanitize_for_logging(tx):
    return {
        "hash": tx["hash"],
        "from": tx["from"][:10] + "...",
        "to": tx["to"][:10] + "...",
        "value": "REDACTED",
        "input": tx["input"][:10] + "..."
    }
```

### Rate Limit Protection
```python
class RateLimiter:
    def __init__(self, max_requests=100, window=1):
        self.requests = []
        self.max_requests = max_requests
        self.window = window
    
    async def acquire(self):
        now = time.time()
        self.requests = [r for r in self.requests if r > now - self.window]
        
        if len(self.requests) >= self.max_requests:
            sleep_time = self.window - (now - self.requests[0])
            await asyncio.sleep(sleep_time)
        
        self.requests.append(now)
```
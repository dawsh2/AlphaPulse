# AlphaPulse API Service Layer

This centralized API service provides a backend-agnostic interface for all frontend-backend communication.

## Architecture Benefits

1. **Single Source of Truth**: All API calls go through `src/services/api/index.ts`
2. **Backend Agnostic**: Switch backends by only modifying this service layer
3. **Type Safety**: Full TypeScript types for all API operations
4. **Consistent Error Handling**: Centralized retry logic and error management
5. **Token Management**: Automatic auth token injection

## Usage Examples

### In React Components

```typescript
import { AlphaPulseAPI } from '@/services/api';

// In a component
function StrategyList() {
  const [strategies, setStrategies] = useState([]);
  
  useEffect(() => {
    AlphaPulseAPI.strategies.list()
      .then(setStrategies)
      .catch(error => console.error('Failed to load strategies:', error));
  }, []);
  
  const runBacktest = async (strategyId: string) => {
    const result = await AlphaPulseAPI.strategies.backtest(strategyId, {
      symbol: 'BTC/USD',
      timeframe: '1h',
      start: '2024-01-01T00:00:00Z',
      end: '2024-01-31T23:59:59Z',
    });
    return result;
  };
}
```

### WebSocket Connections

```typescript
// Connect to live market data
const ws = AlphaPulseAPI.marketData.connectLive(
  ['BTC/USD', 'ETH/USD'],
  (data) => {
    console.log('New price:', data);
    updateChart(data);
  }
);

// Connect to event stream
const eventWs = AlphaPulseAPI.events.connect(
  ['signal', 'trade'],
  (event) => {
    console.log('New event:', event);
    addToEventLog(event);
  }
);

// Cleanup on unmount
useEffect(() => {
  return () => {
    ws.close();
    eventWs.close();
  };
}, []);
```

### Cached Analysis

```typescript
// The manifest-based caching system
async function runAnalysis(params: AnalysisParams) {
  // Generate manifest
  const manifest: AnalysisManifest = {
    symbol: params.symbol,
    timeframe: params.timeframe,
    dateRange: params.dateRange,
    strategy: {
      type: 'trend_following',
      version: '1.0.0',
      parameters: params.strategyParams,
    },
    indicators: ['RSI', 'MACD'],
    features: ['returns_1d', 'volume_ratio'],
    hash: generateHash(params), // SHA256 of above
  };
  
  // Check cache first
  const cacheStatus = await AlphaPulseAPI.analysis.checkCache(manifest.hash);
  
  if (cacheStatus.exists) {
    console.log('Using cached result');
  }
  
  // Run analysis (will use cache if available)
  const result = await AlphaPulseAPI.analysis.runAnalysis(manifest);
  return result;
}
```

### File Upload

```typescript
async function uploadDataset(file: File) {
  const dataset = await AlphaPulseAPI.dataManagement.uploadDataset(file, {
    name: file.name,
    description: 'Historical price data',
    tags: ['bitcoin', 'hourly'],
  });
  
  console.log(`Uploaded ${dataset.rows} rows`);
}
```

## Switching Backends

To switch from NautilusTrader to another backend:

1. Update the API configuration:
```typescript
// In .env
VITE_API_URL=https://new-backend.example.com/api
VITE_WS_URL=wss://new-backend.example.com/ws
```

2. If the new backend has different endpoints, update only the service layer:
```typescript
// In src/services/api/index.ts
export const strategies = {
  async list(): Promise<Strategy[]> {
    // If new backend uses different endpoint
    return http.get('/v2/strategies/list');
  },
  // ... other methods
};
```

3. If response formats differ, add adapters:
```typescript
// In src/services/api/adapters.ts
export function adaptStrategyResponse(backendResponse: any): Strategy {
  return {
    id: backendResponse.strategy_id,
    name: backendResponse.title,
    // ... map fields
  };
}
```

## Testing

```typescript
// Mock the API for testing
import { AlphaPulseAPI } from '@/services/api';

jest.mock('@/services/api', () => ({
  AlphaPulseAPI: {
    strategies: {
      list: jest.fn().mockResolvedValue([
        { id: '1', name: 'Test Strategy' }
      ]),
    },
  },
}));
```

## Performance Considerations

1. **Request Deduplication**: The service layer can detect duplicate in-flight requests
2. **Caching**: Responses can be cached at the service layer
3. **Batch Requests**: Multiple calls can be batched into single requests
4. **Connection Pooling**: WebSocket connections are reused

## Error Handling

All API methods throw `ApiError` with status codes:

```typescript
try {
  const strategies = await AlphaPulseAPI.strategies.list();
} catch (error) {
  if (error instanceof ApiError) {
    switch (error.status) {
      case 401:
        // Redirect to login
        break;
      case 429:
        // Rate limited, show message
        break;
      case 500:
        // Server error, retry
        break;
    }
  }
}
```
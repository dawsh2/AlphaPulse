# API Integration TODO

## Overview
The AlphaPulse UI currently operates with ~90% mocked data. This document outlines the required API integrations and external systems that need to be implemented for production readiness.

## Critical Priority Integrations

### 1. NautilusTrader Backend
**Status**: Partial integration (only file listing endpoint)  
**Current Endpoint**: `http://localhost:5000/api/nt-reference/list-files`

**Required Endpoints**:
- `/api/strategies/execute` - Execute trading strategies
- `/api/strategies/save` - Save strategy configurations
- `/api/backtest/run` - Run backtesting simulations
- `/api/backtest/results` - Retrieve backtest results
- `/api/portfolio/positions` - Get current positions
- `/api/portfolio/orders` - Order management
- `/api/portfolio/performance` - Performance analytics
- `/api/performance/metrics` - Detailed performance metrics

**Files to Update**:
- `src/pages/DevelopPage.tsx`
- `src/components/SignalAnalysisPanel.tsx`

### 2. Market Data Providers
**Status**: All mock data  
**Current**: Random data generation in components

**Required Integrations**:
- **Alpaca Markets API**
  - WebSocket for real-time quotes
  - Historical data endpoints
  - Options chain data
- **Alternative Providers** (for redundancy):
  - Interactive Brokers API
  - TD Ameritrade API
  - Yahoo Finance (fallback)
- **Crypto Data**:
  - Coinbase Pro API
  - Binance API
  - CoinGecko API

**Files to Update**:
- `src/pages/MonitorPage.tsx`
- `src/pages/NewsPage.tsx` (watchlist component)

## High Priority Integrations

### 3. News Aggregation APIs
**Status**: Hardcoded articles  
**Location**: `src/pages/NewsPage.tsx`

**Required APIs**:
- **Financial News**:
  - Alpha Vantage News API
  - Polygon.io News API
  - Yahoo Finance News API
- **Academic Papers**:
  - arXiv API (quantitative finance papers)
  - SSRN API (financial research)
  - Google Scholar API
- **Social Sentiment**:
  - Reddit API (r/wallstreetbets, r/stocks)
  - Twitter API (financial sentiment)
  - StockTwits API
- **Economic Data**:
  - FRED API (Federal Reserve Economic Data)
  - Bureau of Labor Statistics API

### 4. Signal Registry System
**Status**: Mock ADMF references  
**Location**: `src/pages/ResearchPage.tsx`, `src/components/SignalAnalysisPanel.tsx`

**Required Endpoints**:
- `/api/signals/query` - Query historical signals
- `/api/signals/generate` - Generate new signals
- `/api/signals/store` - Store signal configurations
- `/api/signals/performance` - Track signal performance
- `/api/signals/share` - Share signals between users
- `/api/signals/version` - Version control for signals

### 5. Real-time Data Feeds
**Status**: No WebSocket connections  
**Required**: WebSocket implementation for live data

**Implementation Needs**:
- WebSocket client service (`src/services/websocket.ts`)
- Real-time market data streaming
- Strategy alert notifications
- Live portfolio updates
- News feed streaming

## Medium Priority Integrations

### 6. Authentication & Authorization
**Status**: Basic localStorage token  
**Current**: Simple token in `StrategyExporter.tsx`

**Required Features**:
- OAuth2 providers (Google, GitHub, Discord)
- JWT token management with refresh tokens
- Session management
- Role-based access control (RBAC)
- API key management for external services

**Files to Update**:
- Create `src/services/auth.ts`
- Update all API calls to include proper authentication

### 7. Cloud Storage & Collaboration
**Status**: Basic cloud save functionality  
**Location**: `src/components/StrategyExporter.tsx`

**Required Integrations**:
- AWS S3 or Google Cloud Storage
- Strategy version control
- Collaborative editing
- Backup and sync services
- Export to GitHub (partially implemented)

## Architecture Improvements Needed

### 1. Centralized API Client
Create `src/services/api.ts` with:
- Centralized error handling
- Retry logic with exponential backoff
- Request/response interceptors
- Rate limiting
- Caching strategies

### 2. TypeScript Interfaces
Create `src/types/api.ts` with:
- Response type definitions
- Request payload interfaces
- Error response types
- WebSocket message types

### 3. Error Handling
- Implement global error boundary
- Add error reporting service (Sentry)
- Create user-friendly error messages
- Add offline detection and handling

### 4. Testing Infrastructure
- API mocking for development
- Integration tests
- End-to-end tests with mock servers

## Implementation Roadmap

### Phase 1: Foundation (Week 1-2)
1. Create centralized API client
2. Define TypeScript interfaces
3. Implement error handling framework
4. Set up WebSocket infrastructure

### Phase 2: Core Trading (Week 3-4)
1. Complete NautilusTrader integration
2. Implement Alpaca Markets real-time data
3. Add historical data endpoints
4. Test strategy execution pipeline

### Phase 3: Data & Analytics (Week 5-6)
1. Integrate news aggregation APIs
2. Implement signal registry system
3. Add performance analytics
4. Create data caching layer

### Phase 4: Collaboration (Week 7-8)
1. Implement OAuth2 authentication
2. Add cloud storage integration
3. Enable strategy sharing
4. Add version control

### Phase 5: Polish (Week 9-10)
1. Optimize performance
2. Add comprehensive error handling
3. Implement rate limiting
4. Complete testing suite

## Current API Calls Reference

### Real Endpoints
1. `GET /api/nt-reference/list-files` - DevelopPage.tsx:82
2. `GET /api/strategies/templates` - StrategyTemplates.tsx:264
3. `POST /api/strategies` - StrategyExporter.tsx:247
4. `POST https://api.github.com/gists` - StrategyExporter.tsx:298
5. `POST /api/signals/query` - SignalAnalysisPanel.tsx:111
6. `POST /api/signals/generate` - SignalAnalysisPanel.tsx:139

### Mock Data Locations
- NewsPage.tsx - All news articles, market data
- MonitorPage.tsx - Trading charts, metrics
- ResearchPage.tsx - Strategy performance
- SignalAnalysisPanel.tsx - Signal generation

## Environment Variables Needed
```env
# API Keys
VITE_ALPACA_API_KEY=
VITE_ALPACA_SECRET_KEY=
VITE_ALPHA_VANTAGE_KEY=
VITE_POLYGON_API_KEY=
VITE_FRED_API_KEY=
VITE_ARXIV_API_KEY=
VITE_REDDIT_CLIENT_ID=
VITE_TWITTER_BEARER_TOKEN=

# Backend URLs
VITE_NAUTILUS_BACKEND_URL=http://localhost:5000
VITE_SIGNAL_REGISTRY_URL=
VITE_WEBSOCKET_URL=

# OAuth
VITE_GOOGLE_CLIENT_ID=
VITE_GITHUB_CLIENT_ID=
VITE_DISCORD_CLIENT_ID=

# Storage
VITE_AWS_S3_BUCKET=
VITE_AWS_ACCESS_KEY=
VITE_AWS_SECRET_KEY=
```

## Notes
- All mock data should be preserved as fallback for development/demo mode
- Implement feature flags to toggle between mock and real data
- Consider implementing a data abstraction layer to easily switch providers
- Add comprehensive logging for all API interactions
- Implement circuit breakers for external service failures
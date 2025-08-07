# AlphaPulse Web Migration Plan

## Executive Summary

This document outlines the migration strategy for AlphaPulse Web from a collection of standalone HTML files to a modern React-based single-page application (SPA). The migration addresses critical issues including code duplication, inconsistent design patterns, and poor maintainability while establishing a scalable architecture for future development.

## Current State Analysis

### Issues Identified

1. **Code Duplication**
   - Header and navigation HTML/CSS duplicated across 5 files
   - Mobile navigation logic repeated 24+ times
   - 400-1000+ lines of embedded styles per HTML file
   - No code reuse or shared components

2. **Inconsistent Design**
   - Different color values for same UI elements
   - Varying typography and spacing systems
   - Multiple mobile navigation patterns (hamburger, drawer, floating buttons)
   - No unified component library

3. **Poor Architecture**
   - All code embedded in HTML files
   - Global scope JavaScript
   - No separation of concerns
   - Difficult to maintain and scale

4. **Mobile Experience**
   - Inconsistent breakpoints (768px vs 480px)
   - Different UX patterns per page
   - No standardized touch interactions
   - Varying responsive behaviors

## Migration Strategy

### Core Principles

- **Component-Based Architecture**: Break down UI into small, reusable components
- **Single Source of Truth**: Centralize application state and avoid duplication
- **Separation of Concerns**: Keep UI, business logic, and API interactions distinct
- **Developer Experience**: Use modern tools for efficient development
- **Clean Educational Design**: Inspired by Full Stack Open's minimalist, readable aesthetic

### Phase 1: Immediate Consolidation (1-2 days)

Address critical duplication issues without requiring full rewrite.

#### 1.1 Create shared.css
Consolidate common styles from all HTML files:
- CSS variables and design tokens
- Header and navigation styles
- Common component styles (buttons, cards, forms)
- Responsive breakpoints and utilities
- Mobile-specific patterns

#### 1.2 Create layout.js
Dynamic injection of consistent header/navigation:
- Centralized navigation logic
- Active page highlighting
- Mobile menu handling
- User authentication state

#### 1.3 Refactor HTML Files
Update each page to use shared resources:
- Remove duplicated CSS
- Remove duplicated header HTML
- Link to shared.css and layout.js
- Maintain page-specific functionality

### Phase 2: React Foundation (3-5 days)

Establish modern development environment and project structure.

#### 2.1 Project Setup
```bash
# Create new React project with TypeScript
npm create vite@latest alphapulse-react -- --template react-ts

# Install core dependencies
npm install react-router-dom
npm install @mui/material @emotion/react @emotion/styled
npm install axios
npm install @types/react-router-dom
```

#### 2.2 Folder Structure
```
alphapulse-react/
├── src/
│   ├── api/
│   │   ├── apiService.ts      # Centralized API calls
│   │   ├── alpacaClient.ts    # Alpaca-specific endpoints
│   │   └── types.ts           # API type definitions
│   ├── assets/
│   │   ├── images/
│   │   └── fonts/
│   ├── components/
│   │   ├── common/            # Shared components
│   │   │   ├── Header.tsx
│   │   │   ├── Layout.tsx
│   │   │   ├── Button.tsx
│   │   │   └── Card.tsx
│   │   ├── charts/            # Chart components
│   │   │   └── TradingChart.tsx
│   │   └── editors/           # Editor components
│   │       └── CodeEditor.tsx
│   ├── hooks/
│   │   ├── useAuth.ts         # Authentication hook
│   │   ├── useMarketData.ts   # Real-time market data
│   │   └── useApi.ts          # Generic API hook
│   ├── pages/
│   │   ├── HomePage.tsx       # News/feed page
│   │   ├── DevelopPage.tsx    # Strategy development
│   │   ├── ReplayPage.tsx     # Market replay
│   │   ├── ResearchPage.tsx   # Jupyter-like interface
│   │   └── DeployPage.tsx     # Live trading
│   ├── styles/
│   │   ├── theme.ts           # MUI theme configuration
│   │   ├── global.css         # Global styles
│   │   └── variables.css      # CSS custom properties
│   ├── utils/
│   │   ├── formatters.ts      # Data formatting utilities
│   │   └── validators.ts      # Form validation
│   ├── App.tsx                # Main app component
│   ├── main.tsx               # Entry point
│   └── vite-env.d.ts          # TypeScript declarations
├── public/
├── index.html
├── package.json
├── tsconfig.json
├── vite.config.ts
└── .env                       # Environment variables
```

#### 2.3 Design System Configuration

Implement Full Stack Open-inspired design:
- **Typography**: IBM Plex Sans & IBM Plex Mono fonts
- **Color Scheme**: Light/dark theme with high contrast
- **Layout**: Clean, content-focused with generous whitespace
- **Components**: Minimal, functional design patterns
- **Animations**: Subtle transitions for better UX

#### 2.4 Environment Configuration
```env
# .env file
VITE_API_BASE_URL=http://localhost:5000/api
VITE_ALPACA_API_URL=https://paper-api.alpaca.markets
VITE_WEBSOCKET_URL=ws://localhost:5000
```

### Phase 3: Component Development (1-2 weeks)

Convert existing features to React components.

#### 3.1 Core Components

**Layout Components**
```typescript
// Layout.tsx - Wraps all pages with consistent structure
interface LayoutProps {
  children: React.ReactNode;
}

// Header.tsx - Responsive navigation with mobile support
interface HeaderProps {
  user?: User;
  onMenuClick: () => void;
}
```

**Page Components**
- HomePage: News feed with article cards
- DevelopPage: Monaco Editor with file browser
- ReplayPage: Chart with playback controls
- ResearchPage: Notebook interface with cells
- DeployPage: Trading dashboard with positions

#### 3.2 Feature Components

**CodeEditor Component**
- Wrapper around Monaco Editor
- Python syntax highlighting
- Auto-save functionality
- File management integration

**TradingChart Component**
- Lightweight Charts integration
- Real-time data updates
- Technical indicators
- Responsive sizing

**DataGrid Components**
- MUI DataGrid for positions
- Order history table
- Event log viewer
- Sortable/filterable columns

#### 3.3 Mobile Components
- SwipeableDrawer for mobile navigation
- BottomSheet for mobile actions
- Touch-optimized controls
- Pull-to-refresh functionality

### Phase 4: Backend Integration (3-5 days)

Create clean API layer for backend communication.

#### 4.1 API Service Layer
```typescript
// api/apiService.ts
export const api = {
  // Authentication
  login: (credentials: LoginCredentials) => 
    axios.post(`${API_BASE_URL}/auth/login`, credentials),
  
  // Market Data
  getMarketData: (symbol: string) => 
    axios.get(`${API_BASE_URL}/market-data/${symbol}`),
  
  // Trading
  getPositions: () => 
    axios.get(`${API_BASE_URL}/positions`),
  
  submitOrder: (order: OrderRequest) => 
    axios.post(`${API_BASE_URL}/orders`, order),
  
  // Strategies
  getStrategies: () => 
    axios.get(`${API_BASE_URL}/strategies`),
  
  deployStrategy: (strategyId: string) => 
    axios.post(`${API_BASE_URL}/strategies/${strategyId}/deploy`)
};
```

#### 4.2 Custom Hooks
```typescript
// hooks/useMarketData.ts
export function useMarketData(symbol: string) {
  const [data, setData] = useState<MarketData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    // Fetch initial data
    // Set up WebSocket subscription
    // Return cleanup function
  }, [symbol]);

  return { data, loading, error };
}
```

#### 4.3 State Management
- React Context for global state (auth, theme)
- Component state for local UI state
- Custom hooks for data fetching
- Consider Redux Toolkit if complexity grows

### Phase 5: Testing & Deployment (1 week)

#### 5.1 Testing Strategy
- Unit tests with Vitest
- Component tests with React Testing Library
- E2E tests with Playwright
- API mocking with MSW

#### 5.2 Build & Deployment
- Vite production build optimization
- Code splitting and lazy loading
- Asset optimization
- GitHub Pages deployment script

## Migration Timeline

### Week 1
- Day 1-2: Phase 1 consolidation
- Day 3-5: Phase 2 React setup

### Week 2
- Day 1-3: Core component development
- Day 4-5: Page component migration

### Week 3
- Day 1-2: Feature components
- Day 3-5: Backend integration

### Week 4
- Day 1-3: Testing and bug fixes
- Day 4-5: Deployment and documentation

## Success Metrics

1. **Code Quality**
   - 80% reduction in code duplication
   - 100% TypeScript coverage
   - Component test coverage >70%

2. **Performance**
   - Initial load time <3s
   - Time to interactive <5s
   - Lighthouse score >90

3. **Developer Experience**
   - Hot reload in <500ms
   - Build time <30s
   - Clear component hierarchy

4. **User Experience**
   - Consistent UI across all pages
   - Mobile-first responsive design
   - Smooth page transitions

## Risks & Mitigation

### Technical Risks
- **Learning Curve**: Team unfamiliar with React
  - Mitigation: Provide training resources, pair programming
  
- **Integration Issues**: Backend API compatibility
  - Mitigation: Create API adapter layer, versioning

- **Performance**: Large bundle size
  - Mitigation: Code splitting, lazy loading, tree shaking

### Business Risks
- **Downtime**: Migration affecting live users
  - Mitigation: Parallel deployment, gradual rollout

- **Feature Parity**: Missing functionality
  - Mitigation: Feature audit, user acceptance testing

## Next Steps

1. **Immediate Actions**
   - Create shared.css and layout.js
   - Set up React project structure
   - Begin component development

2. **Communication**
   - Team training on React/TypeScript
   - Stakeholder updates on progress
   - User communication about changes

3. **Documentation**
   - Component library documentation
   - API integration guide
   - Deployment procedures

## Conclusion

This migration plan transforms AlphaPulse Web from a collection of HTML files into a modern, scalable React application. By following this phased approach, we can address immediate pain points while building a foundation for future growth. The investment in proper architecture will pay dividends in development speed, code quality, and user experience.

### Phase 6: Decommission and Cleanup

*(This phase occurs after the new React application has been tested, approved, and deployed.)*

1.  **Feature Parity Confirmation:** Final sign-off confirming the new React application fully replaces the functionality of the old site.
2.  **Archive Old Codebase:** Create a compressed archive (e.g., `alphapulse-web.zip`) of the original `alphapulse-web` directory and store it in a safe location for historical reference.
3.  **Update Build/Deployment Scripts:** Ensure all deployment pipelines, CI/CD scripts, and local development instructions now exclusively point to the new `alphapulse-react` project's build output (e.g., the `dist` folder).
4.  **Remove Deprecated Directory:** Delete the `alphapulse-web` directory from the project's main branch to finalize the migration and eliminate technical debt.

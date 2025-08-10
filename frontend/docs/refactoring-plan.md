# Directory Structure Refactoring Plan

## Immediate Actions

### 1. Clean up backup files
```bash
# Remove backup files
rm src/pages/ExplorePage.backup.tsx.backup
rm src/pages/ResearchPage.old.tsx.backup
rm src/components/StrategyBuilder/StrategyWorkbench_OLD.tsx.backup
```

### 2. Create missing directories
```bash
mkdir -p src/types
mkdir -p src/utils
mkdir -p src/hooks
mkdir -p src/constants
mkdir -p src/config
```

## Recommended File Structure

```
src/
├── components/           # Reusable UI components
│   ├── common/          # Buttons, inputs, modals
│   ├── charts/          # Chart components
│   ├── Layout/          # Layout wrapper
│   ├── Navigation/      # Nav components
│   └── features/        # Feature-specific components
│       ├── Monitor/     
│       ├── Strategy/    
│       └── Research/    
│
├── pages/               # Route components (thin, delegate to features)
│   ├── HomePage.tsx
│   ├── ResearchPage.tsx
│   ├── DevelopPage.tsx
│   └── MonitorPage.tsx
│
├── services/            # Business logic & external communication
│   ├── api/            # Backend API abstraction
│   ├── data/           # Local data management
│   ├── exchanges/      # Exchange implementations
│   └── analysis/       # Analysis utilities
│
├── hooks/               # Custom React hooks
│   ├── useWebSocket.ts
│   ├── useBacktest.ts
│   ├── useMarketData.ts
│   └── useAnalysis.ts
│
├── types/               # Shared TypeScript types
│   ├── market.ts       # Market data types
│   ├── strategy.ts     # Strategy types
│   ├── analysis.ts     # Analysis types
│   └── index.ts        # Re-exports
│
├── utils/               # Utility functions
│   ├── format.ts       # Formatters (numbers, dates)
│   ├── hash.ts         # Manifest hashing
│   ├── validation.ts   # Input validation
│   └── performance.ts  # Performance utilities
│
├── constants/           # App constants
│   ├── markets.ts      # Market constants
│   ├── indicators.ts   # Available indicators
│   └── config.ts       # Configuration constants
│
├── config/              # Configuration
│   ├── env.ts          # Environment config
│   ├── charts.ts       # Chart configuration
│   └── theme.ts        # Theme configuration
│
└── store/               # State management
    ├── slices/         # Store slices
    │   ├── marketSlice.ts
    │   ├── strategySlice.ts
    │   └── userSlice.ts
    └── useAppStore.ts

```

## Component Splitting Strategy

### ResearchPage.tsx (2000+ lines) → Split into:
```
pages/ResearchPage.tsx (100 lines - routing & layout)
└── components/features/Research/
    ├── StrategyGrid.tsx       # Strategy cards grid
    ├── NotebookEditor.tsx     # Jupyter notebook component
    ├── ButtonUI.tsx           # Button-driven interface
    ├── DataViewer.tsx         # Data exploration view
    └── ResearchSidebar.tsx    # Sidebar with tabs
```

### StrategyBuilder.tsx (1000+ lines) → Split into:
```
components/features/Strategy/
├── StrategyBuilder/
│   ├── ParameterPanel.tsx    # Parameter configuration
│   ├── IndicatorSelector.tsx # Indicator selection
│   ├── BacktestRunner.tsx    # Backtest execution
│   └── ResultsViewer.tsx     # Results display
└── index.ts
```

## Custom Hooks to Create

```typescript
// hooks/useMarketData.ts
export function useMarketData(symbol: string) {
  const [data, setData] = useState([]);
  const [loading, setLoading] = useState(true);
  // WebSocket connection logic
  return { data, loading };
}

// hooks/useBacktest.ts
export function useBacktest() {
  const [results, setResults] = useState(null);
  const [running, setRunning] = useState(false);
  
  const runBacktest = async (manifest: AnalysisManifest) => {
    // Backtest logic
  };
  
  return { results, running, runBacktest };
}

// hooks/useAnalysis.ts
export function useAnalysis(manifest: AnalysisManifest) {
  const [cached, setCached] = useState(false);
  const [results, setResults] = useState(null);
  
  // Check cache and run analysis
  return { cached, results };
}
```

## Type Organization

```typescript
// types/market.ts
export interface MarketBar {
  time: number;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
}

// types/strategy.ts
export interface Strategy {
  id: string;
  name: string;
  type: StrategyType;
  parameters: StrategyParameters;
}

// types/analysis.ts  
export interface AnalysisManifest {
  symbol: string | string[];
  timeframe: Timeframe;
  strategy: Strategy;
  hash: string;
}
```

## Utils to Create

```typescript
// utils/format.ts
export const formatCurrency = (value: number): string => {
  return new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency: 'USD',
  }).format(value);
};

export const formatPercent = (value: number): string => {
  return `${(value * 100).toFixed(2)}%`;
};

// utils/hash.ts
export const generateManifestHash = (manifest: AnalysisManifest): string => {
  const str = JSON.stringify(manifest);
  return sha256(str);
};

// utils/validation.ts
export const validateSymbol = (symbol: string): boolean => {
  return /^[A-Z]+\/[A-Z]+$/.test(symbol);
};
```

## Migration Steps

1. **Phase 1: Structure** (Week 1)
   - Create directories
   - Move existing files
   - Update imports

2. **Phase 2: Split Components** (Week 2)
   - Break down large components
   - Extract custom hooks
   - Create shared types

3. **Phase 3: Optimize** (Week 3)
   - Add lazy loading
   - Implement code splitting
   - Add error boundaries

4. **Phase 4: Testing** (Week 4)
   - Add unit tests
   - Add integration tests
   - Performance testing

## Benefits After Refactoring

1. **Maintainability**: Smaller, focused files
2. **Reusability**: Shared hooks and utilities
3. **Performance**: Lazy loading and code splitting
4. **Type Safety**: Centralized type definitions
5. **Testing**: Easier to test smaller components
6. **Onboarding**: Clear structure for new developers
# AlphaPulse Frontend Refactoring Guide

## Overview
This document tracks the incremental refactoring of AlphaPulse frontend pages from monolithic components (2000+ lines) into modular, maintainable architecture.

**Core Principle**: Extract incrementally, test after each change, preserve exact UI/UX.

## Directory Structure

```
frontend/src/
├── components/           # Reusable UI components
│   ├── common/          # Shared across multiple features
│   ├── features/        # Feature-specific components
│   │   ├── Research/
│   │   ├── Develop/
│   │   └── Monitor/
│   ├── Layout/          # App layout components
│   ├── Navigation/      # Navigation components
│   ├── MonitorPage/     # Monitor feature components
│   └── StrategyBuilder/ # Strategy builder components
│
├── pages/               # Route-level page components
├── services/            # API clients and data services
├── hooks/               # Custom React hooks
├── store/               # State management (Zustand)
├── types/               # TypeScript type definitions
├── utils/               # Utility functions
├── config/              # App configuration
├── constants/           # App constants
├── data/                # Static data and mock data
└── styles/              # Global styles
```

## Refactoring Progress

### ✅ Research Page (COMPLETED)
**Original**: 2261 lines → **Current**: 915 lines (60% reduction)

#### Extracted Components
- [x] `MobileOverlay` → `/common/MobileOverlay.tsx`
- [x] `SwipeIndicator` → `/common/SwipeIndicator.tsx`
- [x] `SidebarTabs` → `/common/SidebarTabs.tsx`
- [x] `NotebookView` → `/common/NotebookView.tsx` (416 lines!)
- [x] `StrategyCard` → `/common/StrategyCard.tsx`
- [x] `StrategyGrid` → `/common/StrategyGrid.tsx`
- [x] `StrategyDirectory` → `/common/StrategyDirectory.tsx`
- [x] `DataExplorerSidebar` → `/common/DataExplorerSidebar.tsx`
- [x] `DataViewer` → `/common/DataViewer.tsx`
- [x] `BuilderSidebar` → `/common/BuilderSidebar.tsx`
- [x] `BuilderMainContent` → `/common/BuilderMainContent.tsx`
- [x] `TearsheetModal` → `/common/TearsheetModal.tsx`
- [x] `ExploreSearchBar` → `/common/ExploreSearchBar.tsx`
- [x] `NotebookAddCell` → `/common/NotebookAddCell.tsx`

#### Extracted Data & Logic
- [x] Strategy data → `/data/strategies.ts`
- [x] Icons → `/common/Icons.tsx`
- [x] Filtering logic → `/hooks/useStrategyFiltering.ts`

#### Still Needed
- [ ] Move interfaces to `/types/`
- [ ] Create `useResearchState` hook for state management
- [ ] Extract notebook logic to `useNotebook` hook

---

### 🔄 Develop Page (IN PROGRESS)
**Current**: 2278 lines → **Target**: < 500 lines

#### Priority Extractions
1. [ ] **File Explorer Component** (~300 lines)
   - File tree rendering
   - File operations (create, rename, delete)
   - Drag and drop logic

2. [ ] **Code Editor Wrapper** (~400 lines)
   - Monaco editor configuration
   - Theme management
   - Language server integration

3. [ ] **Terminal Component** (~200 lines)
   - Terminal emulator
   - Command history
   - Output rendering

4. [ ] **NautilusTrader Reference Panel** (~250 lines)
   - Documentation browser
   - Code snippets
   - Example templates

5. [ ] **Data Management**
   - [ ] Move file tree data to `/data/fileTemplates.ts`
   - [ ] Move code snippets to `/data/codeSnippets.ts`
   - [ ] Move terminal commands to `/data/commands.ts`

6. [ ] **Custom Hooks**
   - [ ] `useFileSystem` - File operations
   - [ ] `useCodeEditor` - Editor state management
   - [ ] `useTerminal` - Terminal operations
   - [ ] `useNautilusReference` - Documentation fetching

#### Reusable from Research
- [x] `MobileOverlay`
- [x] `SidebarTabs`
- [x] `SwipeIndicator`

---

### 🔄 Monitor Page (PLANNED)
**Current**: ~1500 lines → **Target**: < 400 lines

#### Priority Extractions
1. [ ] **Chart Component** (~400 lines)
   - TradingView Lightweight Charts wrapper
   - Real-time data updates
   - Technical indicators

2. [ ] **Position Table** (~200 lines)
   - Position display
   - P&L calculations
   - Quick actions

3. [ ] **Order Panel** (~150 lines)
   - Order entry form
   - Order validation
   - Order preview

4. [ ] **Event Log** (~150 lines)
   - Real-time event stream
   - Event filtering
   - Event details modal

5. [ ] **Metrics Dashboard** (~200 lines)
   - Performance metrics
   - Risk metrics
   - Live calculations

6. [ ] **Custom Hooks**
   - [ ] `useMarketData` - WebSocket connections
   - [ ] `usePositions` - Position management
   - [ ] `useOrders` - Order management
   - [ ] `useMetrics` - Performance calculations

#### Reusable from Research
- [x] `StrategyCard` (for strategy selection)
- [x] `TearsheetModal` (for performance details)

---

### 🔄 Home Page (PLANNED)
**Current**: ~800 lines → **Target**: < 200 lines

#### Priority Extractions
1. [ ] **Dashboard Cards** (~200 lines)
   - Account summary card
   - Recent activity card
   - Quick actions card

2. [ ] **Market Overview** (~150 lines)
   - Market indices
   - Watchlist
   - Market movers

3. [ ] **Getting Started** (~100 lines)
   - Onboarding steps
   - Tutorial links
   - Documentation links

4. [ ] **Custom Hooks**
   - [ ] `useAccountSummary`
   - [ ] `useMarketOverview`
   - [ ] `useOnboarding`

---

## Component Organization Strategy

### Tier 1: Common UI Components (`/common/`)
Truly generic, reusable across entire app:
```
common/
├── ui/                   # Basic UI elements
│   ├── MobileOverlay
│   ├── SwipeIndicator
│   └── SidebarTabs
├── cards/                # Card components
│   ├── StrategyCard
│   ├── MetricCard
│   └── NotebookCard
├── modals/               # Modal components
│   ├── TearsheetModal
│   └── ConfirmModal
└── forms/                # Form components
    ├── OrderForm
    └── StrategyForm
```

### Tier 2: Feature Components (`/features/`)
Feature-specific, may be reused within feature:
```
features/
├── Research/
│   ├── NotebookView
│   ├── StrategyDirectory
│   └── DataExplorer
├── Develop/
│   ├── FileExplorer
│   ├── CodeEditor
│   └── Terminal
└── Monitor/
    ├── ChartView
    ├── PositionTable
    └── EventLog
```

### Tier 3: Page Components (`/pages/`)
Top-level orchestrators, minimal logic:
- Import components
- Use hooks for logic
- Manage routing
- < 500 lines each

---

## Type Organization (`/types/`)

```typescript
// types/strategy.ts
export interface Strategy { ... }
export interface TearsheetData { ... }

// types/notebook.ts  
export interface NotebookCell { ... }
export interface SavedNotebook { ... }

// types/market.ts
export interface MarketData { ... }
export interface Position { ... }
export interface Order { ... }

// types/nautilus.ts
export interface NautilusStrategy { ... }
export interface BacktestResult { ... }
```

---

## Hook Organization (`/hooks/`)

### Data Hooks
- `useStrategyFiltering` ✅
- `useMarketData`
- `usePositions`
- `useOrders`

### UI Hooks
- `useResearchState`
- `useDevelopState`
- `useMonitorState`

### Utility Hooks
- `useWebSocket`
- `useLocalStorage`
- `useDebounce`

---

## API Integration Plan

### Current State
- Mock data in components
- Some real Alpaca integration
- Coinbase proxy endpoint

### Target State
1. **Centralized API Service** (`/services/api/`)
   ```typescript
   // services/api/index.ts
   export const api = {
     strategies: StrategyAPI,
     market: MarketAPI,
     nautilus: NautilusAPI,
     account: AccountAPI
   }
   ```

2. **Type-Safe Endpoints**
   ```typescript
   // services/api/strategies.ts
   export const StrategyAPI = {
     list: () => api.get<Strategy[]>('/strategies'),
     get: (id: string) => api.get<Strategy>(`/strategies/${id}`),
     backtest: (params: BacktestParams) => api.post<BacktestResult>('/strategies/backtest', params)
   }
   ```

3. **Error Handling**
   - Centralized error interceptor
   - User-friendly error messages
   - Retry logic for transient failures

---

## Next Steps Priority

### Week 1: Complete Develop Page
1. Extract FileExplorer component
2. Extract CodeEditor wrapper
3. Extract Terminal component
4. Create useFileSystem hook
5. Move to < 500 lines

### Week 2: Complete Monitor Page  
1. Extract Chart component
2. Extract PositionTable
3. Extract OrderPanel
4. Create useMarketData hook
5. Move to < 400 lines

### Week 3: API Integration
1. Create centralized API service
2. Replace mock data with real endpoints
3. Add error handling
4. Add loading states
5. Add optimistic updates

### Week 4: Polish & Testing
1. Add TypeScript strict mode
2. Add unit tests for hooks
3. Add integration tests for API
4. Performance optimization
5. Documentation

---

## Success Metrics

- [ ] All pages < 500 lines
- [ ] No duplicate code
- [ ] All components documented
- [ ] TypeScript strict mode enabled
- [ ] 80% code coverage
- [ ] Build time < 30s
- [ ] Bundle size < 500KB

---

## Notes

- Always extract incrementally
- Test after each extraction
- Preserve exact UI/UX
- Commit after each successful extraction
- Document prop interfaces
- Keep components pure when possible
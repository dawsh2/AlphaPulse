# Frontend Deployment Fix Plan

## Overview
The frontend build is failing with multiple TypeScript errors that need to be resolved before deployment.

## Error Categories & Fixes

### 1. Missing Module Imports
**Files affected:**
- `src/components/features/Develop/index.ts`
- `src/components/features/Develop/DevelopContainer.tsx`
- `src/components/features/Research/index.ts`

**Issues:**
- FileExplorer component doesn't exist but is imported
- Multiple Research components are missing (ResearchContainer, StrategyGrid, etc.)

**Fix:**
```typescript
// Remove non-existent imports or create stub components
// For Develop/index.ts - remove FileExplorer export
// For Research/index.ts - check which components actually exist and remove others
```

### 2. Type Import Issues (verbatimModuleSyntax)
**Files affected:**
- `src/components/features/Monitor/LiveChart.tsx`
- `src/services/analysis/dataAnalysis.ts`
- `src/services/api/index.ts`
- `src/services/chartService.ts`

**Issue:** Type imports need `type` keyword when verbatimModuleSyntax is enabled

**Fix:**
```typescript
// Change from:
import { IChartApi, ISeriesApi } from 'lightweight-charts';
// To:
import type { IChartApi, ISeriesApi } from 'lightweight-charts';
```

### 3. Terminal Component State Issues
**File:** `src/components/features/Develop/Terminal.tsx`

**Issue:** setState callbacks have incorrect type signatures

**Fix:**
```typescript
// Ensure state setters match the expected type
setTabs((prev: TerminalTab[]) => [...prev, newTab]);
```

### 4. JSX Namespace Issue
**File:** `src/components/features/Develop/DevelopLayoutManager.tsx`

**Issue:** JSX namespace not found

**Fix:**
```typescript
// Change JSX.Element to React.ReactElement
const renderNode = (node: LayoutNode): React.ReactElement => {
```

### 5. Monitor Component Missing Properties
**File:** `src/components/features/Monitor/MonitorContainer.tsx`

**Issue:** UseMarketDataResult doesn't have expected properties

**Fix:**
```typescript
// Check the actual return type of useMarketData hook
// Update destructuring to match actual properties
```

### 6. TradingView Chart API Issues
**File:** `src/components/features/Monitor/LiveChart.tsx`

**Issues:**
- MouseEventParams doesn't have seriesPrices property
- Time type mismatch (number vs Time)

**Fix:**
```typescript
// Cast time values properly:
time: timestamp as Time
// Check TradingView docs for correct MouseEventParams usage
```

### 7. Research Page Type Issues
**File:** `src/pages/ResearchPage.tsx`

**Issues:**
- Type comparisons failing (MainView vs string literals)
- Missing type definitions (NotebookCellData)
- Prop type mismatches

**Fix:**
```typescript
// Define missing types
interface NotebookCellData extends NotebookCell {
  // Add required properties
}

// Fix type assertions for comparisons
if ((mainView as string) === 'data') {
```

### 8. API Service Missing Types
**File:** `src/services/api/index.ts`

**Missing type definitions:**
- AnalysisManifest
- BacktestResult
- SignalRequest
- Strategy
- MarketBar
- Position
- Order
- etc.

**Fix:**
```typescript
// Create types file with all missing definitions
// src/types/api.types.ts
export interface AnalysisManifest {
  symbol: string | string[];
  timeframe: string;
  // ... other properties
}

export interface BacktestResult {
  // ... properties
}
```

## Execution Order

1. **Fix type imports** (Quick win)
   - Add `type` keyword to all type-only imports
   - Files: LiveChart, dataAnalysis, api/index, chartService

2. **Remove non-existent imports**
   - Clean up Develop/index.ts
   - Clean up Research/index.ts

3. **Create missing type definitions**
   - Create `src/types/api.types.ts`
   - Add all missing interfaces

4. **Fix component type issues**
   - Terminal setState callbacks
   - DevelopLayoutManager JSX namespace
   - MonitorContainer property access

5. **Fix ResearchPage type assertions**
   - Add type guards or assertions for MainView comparisons
   - Define NotebookCellData interface

6. **Fix TradingView integration**
   - Update MouseEventParams usage
   - Fix Time type casting

## Testing Plan

1. Run `npm run build` after each category of fixes
2. Verify no TypeScript errors remain
3. Test deployment script: `./frontend/deploy_to_site.sh`
4. Verify deployment to ap/alphapulse-ui/

## Quick Fix Script

```bash
# Run TypeScript compiler to get fresh error list
cd frontend
npx tsc --noEmit > errors.txt 2>&1

# Count errors by category
grep "TS2307" errors.txt | wc -l  # Module not found
grep "TS1484" errors.txt | wc -l  # Type import issues
grep "TS2304" errors.txt | wc -l  # Cannot find name
```

## Priority Fixes (Blocking Deployment)

1. **High Priority:**
   - Type import issues (TS1484) - Simple fix
   - Missing modules (TS2307) - Remove imports
   - Missing type definitions (TS2304) - Create types file

2. **Medium Priority:**
   - Component prop mismatches
   - State setter type issues

3. **Low Priority:**
   - Type assertion warnings
   - Unused imports

## Estimated Time
- Type imports: 5 minutes
- Missing modules: 10 minutes
- Type definitions: 20 minutes
- Component fixes: 15 minutes
- Testing: 10 minutes

**Total: ~1 hour**
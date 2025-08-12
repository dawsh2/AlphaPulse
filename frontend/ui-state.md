# UI State Management Plan

## Overview
This document outlines the implementation strategy for persisting UI state across different tabs (Develop, Research, Monitor) in the AlphaPulse application. The goal is to maintain user context when switching between tabs, improving the overall user experience.

## Current State Analysis

### Problem Statement
- Users lose their work context when switching between tabs
- No persistence of notebook cells, terminal history, or chart configurations
- Each tab resets to default state on navigation
- Lost productivity due to re-creating setups

### Affected Components
1. **Develop Tab** (`/src/pages/DevelopPage.tsx`)
2. **Research Tab** (`/src/pages/ResearchPage.tsx`)
3. **Monitor Tab** (`/src/pages/MonitorPage.tsx`)

## Implementation Options

### Option 1: LocalStorage with React Context (Recommended)
**Time Estimate: 2-3 hours**
**Complexity: Low**
**Maintenance: Easy**

#### Pros
- Simple implementation
- No backend changes required
- Immediate persistence
- Works offline
- Browser-native solution

#### Cons
- Limited to 5-10MB storage
- Lost on browser clear
- Single device only
- No cross-session sync

#### Implementation
```typescript
// src/context/AppStateContext.tsx
import React, { createContext, useContext, useState, useEffect } from 'react';

interface AppState {
  develop: DevelopState;
  research: ResearchState;
  monitor: MonitorState;
}

const AppStateContext = createContext<{
  state: AppState;
  updateState: (page: string, data: any) => void;
}>({} as any);

export const AppStateProvider: React.FC = ({ children }) => {
  const [state, setState] = useState<AppState>(() => {
    const saved = localStorage.getItem('alphapulse-ui-state');
    return saved ? JSON.parse(saved) : getDefaultState();
  });

  useEffect(() => {
    const saveTimer = setTimeout(() => {
      localStorage.setItem('alphapulse-ui-state', JSON.stringify(state));
    }, 1000); // Debounce saves
    
    return () => clearTimeout(saveTimer);
  }, [state]);

  const updateState = (page: string, data: any) => {
    setState(prev => ({
      ...prev,
      [page]: data
    }));
  };

  return (
    <AppStateContext.Provider value={{ state, updateState }}>
      {children}
    </AppStateContext.Provider>
  );
};

export const useAppState = () => useContext(AppStateContext);
```

### Option 2: Redux with Redux-Persist
**Time Estimate: 4-6 hours**
**Complexity: Medium**
**Maintenance: Moderate**

#### Pros
- Scalable state management
- Time-travel debugging
- Middleware support
- Selective persistence
- Better for complex state

#### Cons
- More boilerplate
- Learning curve
- Larger bundle size
- Overkill for simple state

#### Implementation
```typescript
// src/store/index.ts
import { configureStore } from '@reduxjs/toolkit';
import { persistStore, persistReducer, FLUSH, REHYDRATE, PAUSE, PERSIST, PURGE, REGISTER } from 'redux-persist';
import storage from 'redux-persist/lib/storage';

const persistConfig = {
  key: 'alphapulse',
  version: 1,
  storage,
  whitelist: ['develop', 'research', 'monitor']
};

const rootReducer = combineReducers({
  develop: developReducer,
  research: researchReducer,
  monitor: monitorReducer
});

const persistedReducer = persistReducer(persistConfig, rootReducer);

export const store = configureStore({
  reducer: persistedReducer,
  middleware: (getDefaultMiddleware) =>
    getDefaultMiddleware({
      serializableCheck: {
        ignoredActions: [FLUSH, REHYDRATE, PAUSE, PERSIST, PURGE, REGISTER]
      }
    })
});

export const persistor = persistStore(store);
```

### Option 3: Backend Session Storage
**Time Estimate: 6-8 hours**
**Complexity: High**
**Maintenance: Complex**

#### Pros
- Cross-device sync
- Unlimited storage
- Server-side validation
- User-specific persistence
- Shareable states

#### Cons
- Network dependency
- Backend complexity
- Latency issues
- Requires authentication
- Database storage costs

#### Implementation
```python
# backend/models.py
class UserUIState(db.Model):
    id = db.Column(db.Integer, primary_key=True)
    user_id = db.Column(db.Integer, db.ForeignKey('user.id'))
    page = db.Column(db.String(50))
    state = db.Column(db.JSON)
    updated_at = db.Column(db.DateTime, default=datetime.utcnow)
    
# backend/api/ui_state_routes.py
@app.route('/api/ui-state/<page>', methods=['GET', 'POST'])
@jwt_required()
def manage_ui_state(page):
    user_id = get_jwt_identity()
    
    if request.method == 'POST':
        state = UserUIState.query.filter_by(
            user_id=user_id, 
            page=page
        ).first()
        
        if state:
            state.state = request.json
            state.updated_at = datetime.utcnow()
        else:
            state = UserUIState(
                user_id=user_id,
                page=page,
                state=request.json
            )
            db.session.add(state)
        
        db.session.commit()
        return jsonify({'success': True})
    
    else:  # GET
        state = UserUIState.query.filter_by(
            user_id=user_id,
            page=page
        ).first()
        
        return jsonify(state.state if state else {})
```

## State Structure

### Develop Page State
```typescript
interface DevelopState {
  // Tab management
  tabs: Array<{
    id: string;
    name: string;
    type: 'editor' | 'terminal';
    content?: string;
    language?: string;
    filePath?: string;
    isDirty?: boolean;
  }>;
  activeTab: string;
  
  // Terminal state
  terminalHistory: string[];
  terminalCwd: string;
  
  // Layout
  splitOrientation: 'horizontal' | 'vertical';
  splitSize: number;
  sidebarWidth: number;
  sidebarTab: 'files' | 'search' | 'git';
  
  // Editor
  editorTheme: string;
  fontSize: number;
  
  // Timestamp
  lastModified: number;
}
```

### Research Page State
```typescript
interface ResearchState {
  // Notebook
  notebookCells: Array<{
    id: string;
    type: 'code' | 'markdown' | 'ai-chat';
    content: string;
    output?: string;
    isExecuting?: boolean;
  }>;
  activeCell: string | null;
  notebookName: string;
  
  // View state
  mainView: 'explore' | 'notebook' | 'builder' | 'data';
  activeTab: 'builder' | 'notebooks';
  
  // Sidebar
  sidebarOpen: boolean;
  selectedTemplate: string | null;
  
  // Search/Filter
  searchQuery: string;
  exploreSearchQuery: string;
  sortBy: string;
  
  // Builder
  builderCode: string;
  
  // Timestamp
  lastModified: number;
}
```

### Monitor Page State
```typescript
interface MonitorState {
  // Chart
  selectedSymbol: string;
  timeframe: string;
  chartType: 'candlestick' | 'line' | 'area';
  indicators: string[];
  
  // Positions
  positionsFilter: 'all' | 'open' | 'closed';
  sortColumn: string;
  sortDirection: 'asc' | 'desc';
  
  // Layout
  activeView: 'chart' | 'positions' | 'orders' | 'events';
  sidebarCollapsed: boolean;
  
  // WebSocket
  autoReconnect: boolean;
  connectionStatus: 'connected' | 'disconnected' | 'connecting';
  
  // Timestamp
  lastModified: number;
}
```

## Implementation Steps

### Phase 1: Core Infrastructure (Week 1)
1. **Create State Manager Utility**
   ```typescript
   // src/utils/stateManager.ts
   export class StateManager {
     private static STORAGE_KEY = 'alphapulse-ui-state';
     private static DEBOUNCE_MS = 1000;
     private static saveTimer: NodeJS.Timeout | null = null;
     
     static saveState(page: string, state: any): void {
       if (this.saveTimer) clearTimeout(this.saveTimer);
       
       this.saveTimer = setTimeout(() => {
         const allState = this.getAllState();
         allState[page] = {
           ...state,
           lastModified: Date.now()
         };
         localStorage.setItem(this.STORAGE_KEY, JSON.stringify(allState));
       }, this.DEBOUNCE_MS);
     }
     
     static loadState(page: string): any {
       const allState = this.getAllState();
       return allState[page] || null;
     }
     
     static clearState(page?: string): void {
       if (page) {
         const allState = this.getAllState();
         delete allState[page];
         localStorage.setItem(this.STORAGE_KEY, JSON.stringify(allState));
       } else {
         localStorage.removeItem(this.STORAGE_KEY);
       }
     }
     
     private static getAllState(): Record<string, any> {
       try {
         return JSON.parse(localStorage.getItem(this.STORAGE_KEY) || '{}');
       } catch {
         return {};
       }
     }
   }
   ```

2. **Add State Hooks**
   ```typescript
   // src/hooks/usePageState.ts
   export function usePageState<T>(pageName: string, defaultState: T) {
     const [state, setState] = useState<T>(() => {
       const saved = StateManager.loadState(pageName);
       return saved || defaultState;
     });
     
     useEffect(() => {
       StateManager.saveState(pageName, state);
     }, [pageName, state]);
     
     return [state, setState] as const;
   }
   ```

### Phase 2: Research Page Integration (Week 1)
1. **Add state persistence to ResearchPage.tsx**
   ```typescript
   const [persistedState, setPersistedState] = usePageState('research', {
     notebookCells: [],
     activeCell: null,
     mainView: 'explore',
     activeTab: 'builder',
     // ... other state
   });
   ```

2. **Restore state on mount**
   ```typescript
   useEffect(() => {
     if (persistedState.notebookCells.length > 0) {
       setNotebookCells(persistedState.notebookCells);
       setActiveCell(persistedState.activeCell);
       // ... restore other state
     }
   }, []);
   ```

### Phase 3: Develop Page Integration (Week 2)
1. **Persist terminal history and tabs**
2. **Save editor content for unsaved files**
3. **Restore split pane configuration**

### Phase 4: Monitor Page Integration (Week 2)
1. **Save chart configurations**
2. **Persist selected symbols and timeframes**
3. **Restore WebSocket connections**

### Phase 5: Testing & Optimization (Week 3)
1. **Add storage quota monitoring**
2. **Implement state compression for large notebooks**
3. **Add state versioning and migration**
4. **Create state export/import functionality**

## Storage Optimization

### Compression Strategy
```typescript
// For large notebook cells
import LZString from 'lz-string';

const compressState = (state: any): string => {
  return LZString.compressToUTF16(JSON.stringify(state));
};

const decompressState = (compressed: string): any => {
  return JSON.parse(LZString.decompressFromUTF16(compressed));
};
```

### Storage Limits
- **localStorage**: 5-10MB per domain
- **Notebook cells**: Limit to 100 cells in memory
- **Terminal history**: Keep last 1000 lines
- **Auto-cleanup**: Remove states older than 30 days

## Migration Strategy

### Version Management
```typescript
interface VersionedState {
  version: number;
  data: any;
}

const migrations: Record<number, (state: any) => any> = {
  1: (state) => ({ ...state, version: 1 }),
  2: (state) => ({ ...state, newField: 'default', version: 2 })
};

const migrateState = (state: VersionedState): any => {
  let current = state;
  const targetVersion = Math.max(...Object.keys(migrations).map(Number));
  
  while (current.version < targetVersion) {
    current = migrations[current.version + 1](current);
  }
  
  return current;
};
```

## User Settings

### Preferences Storage
```typescript
interface UserPreferences {
  autoSave: boolean;
  saveInterval: number; // ms
  maxStorageSize: number; // MB
  enableCompression: boolean;
  syncAcrossDevices: boolean;
}

// Store separately from state
localStorage.setItem('alphapulse-preferences', JSON.stringify(preferences));
```

## Performance Considerations

1. **Debounce saves** - Wait 1 second after last change
2. **Selective updates** - Only save changed properties
3. **Lazy loading** - Load state only when tab is accessed
4. **Background sync** - Use Web Workers for large state operations
5. **Compression** - Compress large text content

## Security Considerations

1. **No sensitive data** - Don't store API keys or passwords
2. **Sanitize input** - Clean HTML/scripts from saved content
3. **Validate state** - Check state structure on load
4. **User isolation** - Separate state by user ID when authenticated

## Rollout Plan

### Week 1
- Implement StateManager utility
- Add to Research page (highest value)
- Test with notebook persistence

### Week 2
- Add to Develop page
- Implement terminal history
- Test editor state persistence

### Week 3
- Add to Monitor page
- Implement compression
- Add state export/import

### Week 4
- Performance optimization
- User testing
- Documentation

## Success Metrics

1. **User retention**: 20% increase in session duration
2. **Productivity**: 30% reduction in setup time
3. **Storage usage**: < 2MB average per user
4. **Performance**: < 50ms state load time
5. **Reliability**: 99.9% successful state restoration

## Future Enhancements

1. **Cloud sync** - Sync state across devices via backend
2. **State sharing** - Share notebook/workspace URLs
3. **Undo/redo** - Time-travel through state changes
4. **Auto-backup** - Periodic backups to backend
5. **Collaboration** - Real-time shared state

## Conclusion

The recommended approach is **Option 1: LocalStorage with React Context** for immediate implementation, with a future migration path to **Option 3: Backend Session Storage** for enterprise features.

This provides:
- Quick wins with minimal complexity
- Good user experience improvements
- Foundation for future enhancements
- Low maintenance overhead

Expected development time: **2-3 weeks** for full implementation across all pages.
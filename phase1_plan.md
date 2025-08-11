# Phase 1: Jupyter Backend Setup - Detailed Plan with Checkpoints

## Overview
**Goal**: Get basic Python execution working from frontend Monaco editor through Jupyter backend

## Step-by-Step Implementation with Checkpoints

### Step 1.1: Create Minimal Jupyter Service
**File**: `backend/services/jupyter_service.py`
**Purpose**: Start a Jupyter kernel we can send code to

```python
# Minimal version - just get kernel running
```

**✅ Checkpoint 1.1**:
- Run: `python backend/services/jupyter_service.py`
- Should see: "Jupyter kernel started"
- No errors
- **USER CHECK**: Does it start cleanly?

---

### Step 1.2: Test Kernel Directly
**File**: `backend/test_jupyter.py`
**Purpose**: Verify we can execute code in the kernel

```python
# Simple test script to execute "1+1"
```

**✅ Checkpoint 1.2**:
- Run: `python backend/test_jupyter.py`
- Should output: `Result: 2`
- **USER CHECK**: Basic execution working?

---

### Step 1.3: Create Flask Endpoint
**File**: `backend/api/notebook_routes.py`
**Purpose**: HTTP endpoint for notebook execution

```python
# Single endpoint: POST /api/notebook/execute
# Accepts: {"code": "print('hello')"}
# Returns: {"output": "hello"}
```

**✅ Checkpoint 1.3**:
- Test with curl or Postman
- `curl -X POST http://localhost:5002/api/notebook/execute -d '{"code":"1+1"}'`
- Should return: `{"output": "2"}`
- **USER CHECK**: API endpoint working?

---

### Step 1.4: Add to Flask App
**File**: `backend/app.py`
**Purpose**: Register the notebook routes

```python
# Add: from api.notebook_routes import notebook_bp
# Add: app.register_blueprint(notebook_bp)
```

**✅ Checkpoint 1.4**:
- Restart Flask
- Test endpoint again
- Check no conflicts with existing routes
- **USER CHECK**: Flask integration clean?

---

### Step 1.5: Frontend Service
**File**: `frontend/src/services/notebookService.ts`
**Purpose**: TypeScript service to call the API

```typescript
// Simple service with executeCode() method
```

**✅ Checkpoint 1.5**:
- No errors in browser console
- Service imports correctly
- **USER CHECK**: Frontend compiles?

---

### Step 1.6: Update Research Page
**File**: `frontend/src/pages/ResearchPage.tsx`
**Purpose**: Add execute button to notebook cells

```typescript
// Add onClick handler to run button
// Display results below cell
```

**✅ Checkpoint 1.6**:
- Click run on a cell
- See result appear
- **USER CHECK**: Full integration working?

---

## Testing Progression

### Test 1: Simple Math
```python
1 + 1
```
Expected: `2`

### Test 2: Print Statement
```python
print("Hello from Jupyter")
```
Expected: `Hello from Jupyter`

### Test 3: Import Library
```python
import pandas as pd
print(pd.__version__)
```
Expected: Version number

### Test 4: Multi-line Code
```python
x = 10
y = 20
print(f"Sum: {x + y}")
```
Expected: `Sum: 30`

### Test 5: Error Handling
```python
1 / 0
```
Expected: Error message (not crash)

---

## Rollback Points

After each checkpoint, we can:
1. **Continue**: If working perfectly
2. **Debug**: If minor issues
3. **Rollback**: If major problems

```bash
# Before each step
git add .
git commit -m "Before Step 1.X"

# If rollback needed
git reset --hard HEAD~1
```

---

## Risk Mitigation

**Minimal changes per step:**
- Step 1.1: Just start kernel (no Flask)
- Step 1.2: Just test kernel (no API)
- Step 1.3: Just create endpoint (not integrated)
- Step 1.4: Just integrate endpoint
- Step 1.5: Just create service
- Step 1.6: Just wire up UI

**Each step is isolated and testable**

---

## Ready to Start?

1. Confirm you've pushed current state to git
2. We'll start with Step 1.1 - just creating the Jupyter service
3. You test and approve before moving to 1.2
4. Continue step by step with your approval

**Type "ready" when you've pushed and want to begin Step 1.1**
# Frontend Update Guide - FastAPI Migration

## ✅ Frontend Updated to Use FastAPI on Port 8080

### Changes Made:

1. **Configuration Files Updated:**
   - `frontend/src/config/env.ts` - Updated default API_URL and WS_URL to port 8080
   - `frontend/src/services/api/index.ts` - Updated baseUrl and wsUrl to port 8080
   - `frontend/src/services/notebookService.ts` - Updated baseUrl to port 8080
   - `frontend/src/services/exchanges/binance.ts` - Updated hardcoded URL to port 8080

2. **Environment File Created:**
   - `frontend/.env.local` - Created with all configuration variables pointing to port 8080

### Files Modified:
```
frontend/
├── .env.local                    # NEW - Environment configuration
├── src/
│   ├── config/
│   │   └── env.ts               # Updated: Port 5001 → 8080
│   ├── services/
│   │   ├── api/
│   │   │   └── index.ts         # Updated: Port 5001 → 8080
│   │   ├── notebookService.ts   # Updated: Port 5001 → 8080
│   │   └── exchanges/
│   │       └── binance.ts       # Updated: Port 5000 → 8080
```

## How to Run the Updated System

### 1. Start the FastAPI Backend (Port 8080)
```bash
cd backend
python app_fastapi.py
# Or with uvicorn:
uvicorn app_fastapi:app --reload --port 8080
```

### 2. Start the Frontend (Port 5173)
```bash
cd frontend
npm run dev
```

### 3. Access the Application
- Frontend: http://localhost:5173
- Backend API Docs: http://localhost:8080/docs
- Backend Health: http://localhost:8080/api/health

## API Endpoint Changes

All API endpoints remain the same, only the port has changed:

| Old URL (Flask) | New URL (FastAPI) |
|-----------------|-------------------|
| http://localhost:5001/api/* | http://localhost:8080/api/* |
| ws://localhost:5001/ws/* | ws://localhost:8080/ws/* |

## Testing the Connection

After starting both services, test the connection:

```bash
# Test backend health
curl http://localhost:8080/api/health

# Test from frontend console
fetch('http://localhost:8080/api/health')
  .then(res => res.json())
  .then(console.log)
```

## Environment Variables

The frontend now uses these environment variables (defined in `.env.local`):

- `VITE_API_URL` - API base URL (http://localhost:8080/api)
- `VITE_API_BASE_URL` - Base URL without /api (http://localhost:8080)
- `VITE_WS_URL` - WebSocket URL (ws://localhost:8080/ws)

## Troubleshooting

### CORS Issues
If you encounter CORS errors:
1. Ensure FastAPI is running with proper CORS configuration
2. Check that the frontend is accessing port 8080, not 5001 or 5000

### Connection Refused
If the frontend can't connect:
1. Verify FastAPI is running: `curl http://localhost:8080/api/health`
2. Check no firewall is blocking port 8080
3. Ensure no other service is using port 8080

### WebSocket Issues
For WebSocket connections:
1. Verify WebSocket endpoint: `ws://localhost:8080/ws`
2. Check browser console for WebSocket errors
3. Ensure FastAPI WebSocket routes are configured

## Rollback Instructions

If you need to rollback to Flask:

1. Start Flask backend on port 5001:
```bash
cd backend
python app.py  # Old Flask app
```

2. Update frontend `.env.local`:
```env
VITE_API_URL=http://localhost:5001/api
VITE_API_BASE_URL=http://localhost:5001
VITE_WS_URL=ws://localhost:5001/ws
```

3. Restart frontend:
```bash
cd frontend
npm run dev
```

## Migration Complete! 🎉

The frontend is now fully configured to work with the FastAPI backend on port 8080.

### Next Steps:
1. Test all features in the frontend
2. Remove Flask dependencies from backend
3. Deploy to production with FastAPI
# API Gateway Service

API Gateway for AlphaPulse microservices architecture.

## Purpose
- Route requests to appropriate microservices
- Handle authentication/authorization
- Rate limiting and throttling
- Request/response transformation
- API versioning

## Technology
- Nginx or Kong
- Reverse proxy configuration
- Load balancing

## Endpoints
- `/api/auth/*` → auth service
- `/api/market-data/*` → market-data service
- `/api/news/*` → news service
- `/api/social/*` → social service
- `/api/nautilus/*` → nautilus-core service
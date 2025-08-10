# AlphaPulse Architecture

## Proposed Directory Structure

```
~/alphapulse/
│
├── frontend/                    # React/TypeScript UI (current alphapulse-ui)
│   ├── src/
│   ├── public/
│   ├── package.json
│   └── vite.config.ts
│
├── services/                    # Microservices backend architecture
│   │
│   ├── gateway/                # API Gateway (Kong/Nginx)
│   │   ├── config/
│   │   └── routes/
│   │
│   ├── auth/                   # Authentication & User Management
│   │   ├── src/
│   │   ├── database/           # User DB (PostgreSQL)
│   │   ├── Dockerfile
│   │   └── requirements.txt
│   │
│   ├── market-data/            # Market Data Service
│   │   ├── src/
│   │   │   ├── providers/     # Alpaca, IB, etc.
│   │   │   ├── cache/         # Redis cache layer
│   │   │   └── websocket/     # Real-time feeds
│   │   ├── Dockerfile
│   │   └── requirements.txt
│   │
│   ├── news/                   # News & Sentiment Service
│   │   ├── src/
│   │   │   ├── scrapers/      # News sources
│   │   │   ├── nlp/           # Sentiment analysis
│   │   │   └── api/
│   │   ├── Dockerfile
│   │   └── requirements.txt
│   │
│   ├── social/                 # Comments & Social Features
│   │   ├── src/
│   │   ├── database/           # MongoDB for comments
│   │   ├── Dockerfile
│   │   └── requirements.txt
│   │
│   └── nautilus-core/          # NautilusTrader Core Service
│       ├── src/
│       │   ├── strategies/     # User strategies
│       │   ├── backtest/       # Backtesting engine
│       │   ├── live/           # Live trading
│       │   ├── catalog/        # Data catalog
│       │   └── api/            # REST/WebSocket API
│       ├── user-workspaces/    # User files/notebooks
│       ├── Dockerfile
│       └── requirements.txt
│
├── infrastructure/              # Infrastructure as Code
│   ├── docker-compose.yml      # Local development
│   ├── kubernetes/             # K8s manifests
│   │   ├── deployments/
│   │   ├── services/
│   │   └── ingress/
│   └── terraform/              # Cloud infrastructure
│
├── shared/                      # Shared libraries
│   ├── python-common/          # Shared Python utilities
│   ├── types/                  # TypeScript type definitions
│   └── protos/                 # Protocol buffers (if using gRPC)
│
├── data/                        # Persistent data (git-ignored)
│   ├── postgres/               # User database
│   ├── mongodb/                # Comments/social
│   ├── redis/                  # Cache
│   ├── parquet/                # Market data catalog
│   └── user-files/             # User strategies/notebooks
│
├── scripts/                     # Development & deployment scripts
│   ├── setup.sh                # Initial setup
│   ├── migrate-from-ap.sh     # Migration from current structure
│   └── deploy.sh               # Deployment scripts
│
├── docs/                        # Documentation
│   ├── api/                    # API documentation
│   ├── architecture/           # Architecture decisions
│   └── development/            # Dev setup guides
│
├── .env.example                # Environment variables template
├── Makefile                    # Common commands
└── README.md
```

## Service Communication

```mermaid
graph TB
    Frontend[React Frontend] --> Gateway[API Gateway]
    Gateway --> Auth[Auth Service]
    Gateway --> Market[Market Data Service]
    Gateway --> News[News Service]
    Gateway --> Social[Social Service]
    Gateway --> Nautilus[Nautilus Core]
    
    Market --> Redis[(Redis Cache)]
    Market --> Alpaca[Alpaca API]
    
    News --> MongoDB[(MongoDB)]
    Social --> MongoDB
    
    Auth --> Postgres[(PostgreSQL)]
    
    Nautilus --> Catalog[(Parquet Catalog)]
    Nautilus --> UserFiles[(User Workspaces)]
```

## Migration Steps

1. **Move frontend**:
   ```bash
   mv ~/alphapulse/ap/alphapulse-ui ~/alphapulse/frontend
   ```

2. **Extract services from current app.py**:
   - Auth endpoints → auth service
   - Market data endpoints → market-data service
   - Strategy endpoints → nautilus-core service

3. **Set up service architecture**:
   ```bash
   cd ~/alphapulse
   mkdir -p services/{gateway,auth,market-data,news,social,nautilus-core}
   mkdir -p infrastructure shared data scripts docs
   ```

4. **Docker Compose for local development**:
   ```yaml
   version: '3.8'
   services:
     frontend:
       build: ./frontend
       ports:
         - "3000:3000"
     
     gateway:
       image: nginx
       ports:
         - "80:80"
       volumes:
         - ./services/gateway/config:/etc/nginx
     
     auth:
       build: ./services/auth
       environment:
         - DATABASE_URL=postgresql://user:pass@postgres:5432/auth
     
     nautilus-core:
       build: ./services/nautilus-core
       volumes:
         - ./data/user-files:/workspace
         - ./data/parquet:/catalog
   ```

## Benefits

1. **Scalability**: Each service can scale independently
2. **Maintainability**: Clear separation of concerns
3. **Technology Flexibility**: Each service can use optimal tech stack
4. **Development Speed**: Teams can work independently
5. **Fault Isolation**: One service failure doesn't crash everything
6. **Easy Testing**: Services can be tested in isolation

## Next Steps

1. Start with moving frontend to `~/alphapulse/frontend`
2. Create nautilus-core service with current app.py functionality
3. Gradually extract other services
4. Set up Docker Compose for local development
5. Add CI/CD pipelines

Would you like me to start implementing this structure?
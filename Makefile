# AlphaPulse Makefile
.PHONY: help setup dev prod build test clean migrate

# Colors for output
GREEN := \033[0;32m
YELLOW := \033[0;33m
RED := \033[0;31m
NC := \033[0m # No Color

help: ## Show this help message
	@echo '${GREEN}AlphaPulse Development Commands${NC}'
	@echo ''
	@echo 'Usage:'
	@echo '  ${YELLOW}make${NC} ${GREEN}<command>${NC}'
	@echo ''
	@echo 'Available commands:'
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  ${YELLOW}%-15s${NC} %s\n", $$1, $$2}'

setup: ## Initial project setup
	@echo "${GREEN}Setting up AlphaPulse...${NC}"
	@echo "Creating data directories..."
	@mkdir -p data/{postgres,mongodb,redis,parquet,user-files}
	@echo "Installing frontend dependencies..."
	@cd frontend && npm install
	@echo "Creating .env file from template..."
	@cp .env.example .env 2>/dev/null || true
	@echo "${GREEN}Setup complete! Edit .env with your API keys${NC}"

dev: ## Start development environment with Docker Compose
	@echo "${GREEN}Starting development environment...${NC}"
	docker-compose up -d
	@echo "${GREEN}Services starting...${NC}"
	@echo "Frontend: http://localhost:3000"
	@echo "API Gateway: http://localhost:80"
	@echo "PostgreSQL: localhost:5432"
	@echo "MongoDB: localhost:27017"
	@echo "Redis: localhost:6379"

dev-logs: ## Show logs from all services
	docker-compose logs -f

dev-stop: ## Stop development environment
	@echo "${YELLOW}Stopping development environment...${NC}"
	docker-compose down

dev-clean: ## Stop and remove all containers, volumes
	@echo "${RED}Removing all containers and volumes...${NC}"
	docker-compose down -v

build: ## Build all Docker images
	@echo "${GREEN}Building Docker images...${NC}"
	docker-compose build

build-frontend: ## Build frontend for production
	@echo "${GREEN}Building frontend...${NC}"
	@cd frontend && npm run build

test: ## Run all tests
	@echo "${GREEN}Running tests...${NC}"
	@cd frontend && npm test
	@echo "Backend tests would run here..."

test-frontend: ## Run frontend tests
	@echo "${GREEN}Running frontend tests...${NC}"
	@cd frontend && npm test

lint: ## Run linting
	@echo "${GREEN}Running linters...${NC}"
	@cd frontend && npm run lint

format: ## Format code
	@echo "${GREEN}Formatting code...${NC}"
	@cd frontend && npm run format

migrate: ## Run database migrations
	@echo "${GREEN}Running database migrations...${NC}"
	@echo "Auth service migrations..."
	# docker-compose exec auth python manage.py migrate
	@echo "Nautilus service migrations..."
	# docker-compose exec nautilus-core python manage.py migrate

migrate-from-ap: ## Migrate from old ap/ structure to new structure
	@echo "${GREEN}Migrating from ap/ to new structure...${NC}"
	@echo "This would:"
	@echo "1. Move backend services from ap/pulse-engine to services/"
	@echo "2. Extract service components"
	@echo "3. Update import paths"
	@echo "4. Create service-specific requirements.txt"

frontend-dev: ## Start frontend development server (without Docker)
	@echo "${GREEN}Starting frontend dev server...${NC}"
	@cd frontend && npm run dev

backend-dev: ## Start backend services locally (without Docker)
	@echo "${GREEN}Starting backend services locally...${NC}"
	@echo "Would start each service individually..."

ps: ## Show running containers
	docker-compose ps

shell-auth: ## Shell into auth service
	docker-compose exec auth /bin/bash

shell-nautilus: ## Shell into nautilus-core service
	docker-compose exec nautilus-core /bin/bash

shell-frontend: ## Shell into frontend container
	docker-compose exec frontend /bin/sh

db-shell: ## PostgreSQL shell
	docker-compose exec postgres psql -U alphapulse

mongo-shell: ## MongoDB shell
	docker-compose exec mongodb mongosh

redis-cli: ## Redis CLI
	docker-compose exec redis redis-cli

backup: ## Backup data directories
	@echo "${GREEN}Creating backup...${NC}"
	@mkdir -p backups
	@tar -czf backups/alphapulse-data-$$(date +%Y%m%d-%H%M%S).tar.gz data/
	@echo "${GREEN}Backup created in backups/${NC}"

restore: ## Restore from latest backup
	@echo "${YELLOW}Restoring from latest backup...${NC}"
	@tar -xzf $$(ls -t backups/*.tar.gz | head -1) -C .
	@echo "${GREEN}Restore complete${NC}"

.DEFAULT_GOAL := help
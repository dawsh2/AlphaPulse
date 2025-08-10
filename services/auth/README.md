# Authentication Service

User authentication and management for AlphaPulse.

## Features
- JWT token generation/validation
- User registration/login
- Password reset
- Profile management
- API key management

## Tech Stack
- Python FastAPI
- PostgreSQL database
- Redis for session management
- bcrypt for password hashing

## Endpoints
- `POST /auth/register`
- `POST /auth/login`
- `POST /auth/refresh`
- `GET /auth/profile`
- `PUT /auth/profile`
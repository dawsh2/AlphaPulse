# Shared Libraries

This directory contains code shared across multiple services. Keep this as **libraries**, not services.

## Structure

- `python-common/` - Shared Python utilities, models, and helpers
- `types/` - TypeScript type definitions shared between frontend and backend
- `rust-common/` - Shared Rust code (symlink to rust-services/common)

## Rules

✅ **DO:**
- Put common data models here
- Share utility functions
- Define API contracts/types
- Include validation schemas

❌ **DON'T:**
- Turn these into microservices
- Add HTTP endpoints here
- Include business logic
- Make these deployable services

## Usage

```python
# In services, import shared code
from shared.python_common.models import User, Strategy
from shared.python_common.utils import validate_email
```

```typescript
// In frontend, import shared types
import { ApiResponse, MarketData } from '@shared/types';
```

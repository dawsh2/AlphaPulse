# Org-Mode Documentation Architecture

## Overview
This document outlines the new documentation architecture using Org-mode as the source of truth, with automatic Markdown generation for Rust documentation via `#[doc = include_str!()]`.

## Architecture

```
.claude/docs/
├── source/                   # Org-mode source files (single source of truth)
│   ├── protocol/
│   │   ├── tlv_types.org     # TLV type documentation
│   │   ├── messages.org      # Message format specs
│   │   └── precision.org     # Precision handling
│   ├── architecture/
│   │   ├── overview.org      # System architecture
│   │   ├── domains.org       # Relay domains
│   │   └── performance.org   # Performance targets
│   └── api/
│       ├── codec.org         # Codec API docs
│       ├── network.org       # Network layer docs
│       └── strategies.org    # Strategy interfaces
├── generated/                # Auto-generated Markdown (DO NOT EDIT)
│   └── [mirrors source structure]
└── build/
    └── org-to-md.sh         # Conversion script

docs/                        # Legacy docs (to be migrated)
```

## Workflow

1. **Edit Documentation**: All edits happen in `.claude/docs/source/*.org`
2. **Auto-Convert**: Git hooks or build scripts convert Org → Markdown
3. **Include in Rust**: Rust code uses `#[doc = include_str!("path/to/generated.md")]`
4. **View Results**: `cargo doc` renders the Markdown in API documentation

## Benefits

- **Single Source of Truth**: Edit once in Org, appears everywhere
- **No Duplication**: Same docs in code, README files, and API docs
- **Org Features**: TODO states, tables, code blocks, exports
- **Version Control**: Track changes in human-readable Org format
- **IDE Support**: Emacs org-mode for powerful editing

## Migration Strategy

Phase 1: Core Protocol Documentation
- TLV types and messages
- Precision and performance specs
- Critical invariants

Phase 2: API Documentation  
- Public interfaces
- Service boundaries
- Integration guides

Phase 3: Full Migration
- All README files
- Architecture docs
- Developer guides
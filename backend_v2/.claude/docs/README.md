# AI Assistant Documentation

This directory contains modular documentation for AI assistants working with the Torq codebase.

## Structure

### Core Files
- `CLAUDE.md` - Main context file (streamlined, <20K chars)
- `README.md` - This file (directory overview)

### Core Development (`core/`)
- `practices.md` - **Torq-specific requirements (zero-copy, precision, TLV)**
- `principles.md` - Core engineering principles and practical patterns  
- `development.md` - Development workflows and practices
- `testing.md` - Testing philosophy, debugging procedures, and TDD guidance
- `style.md` - Code style guide and conventions

### Tools & Automation (`tools/`)
- `tools.md` - Development tools and commands
- `cicd.md` - CI/CD pipelines, GitHub Actions, and deployment
- `rq_tool.md` - rq tool documentation and usage

### Operations & Troubleshooting (`operations/`)
- `devops_procedures.md` - DevOps procedures and infrastructure
- `common_pitfalls.md` - Common mistakes and solutions
- `live_streaming_pipeline.md` - Pipeline operations manual
- `websocket_disconnect_issue.md` - Specific troubleshooting guide
- `handover_live_streaming_pipeline.md` - Handover documentation

### Architecture (`architecture/`)
- `dependency_patterns.md` - Dependency patterns and guidelines
- `decisions/` - Architecture Decision Records (ADRs)
  - `adr_001_codec_types_separation.md` - Codec types separation decision

## Usage

AI assistants should load `CLAUDE.md` as primary context, then reference other files as needed for specific tasks.

## Guidelines

- Keep main CLAUDE.md under 20K characters for optimal performance
- Split detailed documentation into focused topic files
- Update relevant files when system architecture changes
- Maintain consistency across all documentation files
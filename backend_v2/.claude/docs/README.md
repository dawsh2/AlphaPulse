# AI Assistant Documentation

This directory contains modular documentation for AI assistants working with the AlphaPulse codebase.

## Structure

- `CLAUDE.md` - Main context file (streamlined, <20K chars)
- `practices.md` - **AlphaPulse-specific requirements (zero-copy, precision, TLV)**
- `principles.md` - Core engineering principles and practical patterns
- `development.md` - Development workflows and practices
- `testing.md` - Testing philosophy, debugging procedures, and TDD guidance
- `style.md` - Code style guide and conventions
- `tools.md` - Development tools and commands
- `cicd.md` - CI/CD pipelines, GitHub Actions, and deployment
- `rq_tool.md` - rq tool documentation and usage
- `common_pitfalls.md` - Common mistakes and solutions

## Usage

AI assistants should load `CLAUDE.md` as primary context, then reference other files as needed for specific tasks.

## Guidelines

- Keep main CLAUDE.md under 20K characters for optimal performance
- Split detailed documentation into focused topic files
- Update relevant files when system architecture changes
- Maintain consistency across all documentation files
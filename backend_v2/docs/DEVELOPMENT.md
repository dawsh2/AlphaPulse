# AlphaPulse Development Guide

This document outlines the development tools, practices, and workflows for the AlphaPulse trading system.

## Standard Rust Tooling

AlphaPulse uses industry-standard Rust tools to maintain code quality, security, and consistency:

### 🔒 Security & License Auditing (cargo-deny)

**Purpose**: Checks dependencies for security vulnerabilities and license compliance.

**Installation**:
```bash
cargo install --locked cargo-deny
```

**Usage**:
```bash
# Check all policies (advisories, licenses, bans, sources)
cargo deny check

# Check only security advisories
cargo deny check advisories

# Check only licenses
cargo deny check licenses

# Check only banned dependencies
cargo deny check bans
```

**Configuration**: See `deny.toml` for policy details.

**In CI**: Runs weekly to catch new security advisories.

### 🧹 Unused Dependency Detection (cargo-udeps)

**Purpose**: Identifies unused dependencies to keep Cargo.toml files clean and reduce build times.

**Installation**:
```bash
# Requires nightly toolchain
rustup toolchain install nightly
cargo +nightly install --locked cargo-udeps
```

**Usage**:
```bash
# Check for unused dependencies
cargo +nightly udeps

# Check all targets (includes dev, build dependencies)
cargo +nightly udeps --all-targets
```

**In CI**: Runs on pull requests to prevent dependency bloat.

### 📝 Cargo.toml Formatting (cargo-sort)

**Purpose**: Maintains consistent formatting and ordering in Cargo.toml files.

**Installation**:
```bash
cargo install --locked cargo-sort
```

**Usage**:
```bash
# Format all Cargo.toml files in workspace
cargo sort --workspace

# Check if files are properly formatted (CI mode)
cargo sort --workspace --check

# Format specific crate
cargo sort libs/types
```

**In CI**: Enforces consistent formatting across all Cargo.toml files.

## Quick Setup

Run the automated setup script to install all development tools:

```bash
./scripts/install-dev-tools.sh
```

This script will:
1. Install cargo-deny for security auditing
2. Install cargo-sort for Cargo.toml formatting
3. Install Rust nightly toolchain
4. Install cargo-udeps for unused dependency detection

## CI/CD Integration

All tools are automatically run in GitHub Actions:

- **Security checks**: Run weekly and on PRs
- **Unused dependencies**: Run on PRs
- **Formatting**: Run on all pushes and PRs

See `.github/workflows/rust-tooling.yml` for complete CI configuration.

## Pre-commit Integration

While cargo-sort can be used as a pre-commit hook, the current setup focuses on CI enforcement. To run checks locally before committing:

```bash
# Quick check before commit
cargo deny check advisories  # Fast security check
cargo sort --workspace --check  # Formatting check

# Full check (slower)
cargo deny check
cargo +nightly udeps --all-targets
```

## Tool Configuration Details

### cargo-deny Configuration (`deny.toml`)

The configuration enforces:

**Allowed Licenses**:
- MIT, Apache-2.0, BSD variants
- ISC, Unicode-DFS-2016, CC0-1.0

**Denied Licenses**:
- GPL variants (copyleft restrictions)
- AGPL, EUPL (network copyleft)

**Security Policy**:
- Deny known vulnerabilities
- Warn on unmaintained crates
- Block yanked crate versions

**Private Crates**:
- Workspace crates are ignored for licensing (not published)

### Performance Impact

**Build Time Impact**: <30 seconds additional time
- cargo-deny: ~5-10 seconds
- cargo-udeps: ~10-20 seconds (nightly compilation)
- cargo-sort: <1 second

**Caching**: All tools use cargo registry caching in CI to minimize repeated work.

## Troubleshooting

### cargo-udeps Issues

If cargo-udeps reports false positives:

1. **Conditional dependencies**: Some deps only used on specific platforms
2. **Proc macros**: May be needed at compile time only
3. **Feature gates**: Dependencies used behind feature flags

### cargo-deny Failures

For security advisories:

1. **Update dependencies**: `cargo update -p <crate-name>`
2. **Check alternatives**: Use `cargo deny list` to see alternatives
3. **Temporary ignore**: Add to `ignore` list in `deny.toml` with reason

For license issues:

1. **Review license**: Check if it's compatible with project needs
2. **Find alternatives**: Look for crates with approved licenses
3. **Add exception**: If necessary, add to `exceptions` in `deny.toml`

### cargo-sort Issues

If formatting fails:

1. **Manual fix**: Run `cargo sort --workspace` to fix formatting
2. **Conflicting edits**: Resolve any merge conflicts in Cargo.toml first
3. **Custom ordering**: Check if specific ordering is needed for features

## Development Workflow

1. **Before making changes**:
   ```bash
   # Ensure clean starting point
   cargo sort --workspace --check
   ```

2. **After adding dependencies**:
   ```bash
   # Check for security issues
   cargo deny check advisories
   
   # Format new dependencies
   cargo sort --workspace
   ```

3. **Before committing**:
   ```bash
   # Full validation
   ./scripts/install-dev-tools.sh  # If tools not installed
   cargo deny check
   cargo +nightly udeps --all-targets
   cargo sort --workspace --check
   ```

4. **CI will automatically**:
   - Run all checks on pull requests
   - Weekly security scans
   - Block merges if tools fail

This ensures consistent, secure, and maintainable code across the AlphaPulse codebase.
# CI/CD and GitHub Actions Guide

## Core Philosophy: Automated Quality Gates

**Every merge to main must pass through automated quality gates.** Manual checks are error-prone; automation enforces standards universally and consistently.

## GitHub Actions Workflows

### Required CI Checks (Must Pass Before Merge)

Create `.github/workflows/ci.yml`:

```yaml
name: CI

on:
  pull_request:
    branches: [main]
  push:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  format:
    name: Format Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt
      - name: Check formatting
        run: cargo fmt --all -- --check

  lint:
    name: Clippy Lint
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
      - uses: Swatinem/rust-cache@v2
      - name: Run clippy
        run: cargo clippy --workspace -- -D warnings

  test:
    name: Tests
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Run tests
        run: cargo test --workspace
      - name: Run doctests
        run: cargo test --doc --workspace
      - name: Protocol V2 validation
        run: |
          cargo test --package protocol_v2 --test tlv_parsing
          cargo test --package protocol_v2 --test precision_validation

  security:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: actions-rs/audit-check@v1
        with:
          token: ${{ secrets.GITHUB_TOKEN }}

  semver:
    name: Breaking Change Detection
    runs-on: ubuntu-latest
    if: github.event_name == 'pull_request'
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - uses: dtolnay/rust-toolchain@stable
      - name: Install cargo-semver-checks
        uses: taiki-e/install-action@v2
        with:
          tool: cargo-semver-checks
      - name: Check for breaking changes
        run: cargo semver-checks check-release --baseline-rev origin/main

  performance:
    name: Performance Benchmarks
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Run benchmarks
        run: cargo bench --workspace --no-fail-fast
      - name: Validate Protocol V2 performance
        run: |
          cargo run --bin test_protocol --release
          # Must maintain: >1M msg/s construction, >1.6M msg/s parsing
```

### Deployment Workflow

Create `.github/workflows/deploy.yml`:

```yaml
name: Deploy

on:
  push:
    tags:
      - 'v*'

jobs:
  build-and-release:
    name: Build and Release
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      
      - name: Build release binaries
        run: |
          cargo build --release --workspace
          
      - name: Create release artifacts
        run: |
          mkdir -p artifacts
          cp target/release/exchange_collector artifacts/
          cp target/release/relay_server artifacts/
          cp target/release/ws_bridge artifacts/
          tar -czf alphapulse-${{ github.ref_name }}.tar.gz artifacts/
          
      - name: Create GitHub Release
        uses: softprops/action-gh-release@v1
        with:
          files: alphapulse-${{ github.ref_name }}.tar.gz
          draft: false
          prerelease: false
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

## Git Workflow and Commit Standards

### Branch Strategy

```bash
main          # Production-ready code
├── feature/* # New features
├── fix/*     # Bug fixes
├── refactor/* # Code refactoring (breaking changes welcome!)
└── perf/*    # Performance improvements
```

### Commit Message Format

Follow conventional commits for clarity and automation:

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `refactor`: Code change that neither fixes a bug nor adds a feature
- `perf`: Performance improvement
- `test`: Adding or updating tests
- `docs`: Documentation only changes
- `chore`: Maintenance tasks
- `breaking`: Breaking change (or use `!` after type)

**Examples:**
```bash
feat(protocol): add PoolStateTLV message type

fix(collector): preserve WETH 18-decimal precision

refactor!: replace DataHandler with MarketDataProcessor
- Rename across entire codebase (47 files)
- Update all test files
- Remove deprecated DataHandler completely

perf(relay): optimize TLV parsing to achieve >2M msg/s
```

### Pull Request Process

1. **Create feature branch**
   ```bash
   git checkout -b feature/pool-state-tracking
   ```

2. **Make atomic commits**
   ```bash
   # Complete migrations in single commits
   git add -A
   git commit -m "refactor!: migrate all Symbol to InstrumentId"
   ```

3. **Push and create PR**
   ```bash
   git push -u origin feature/pool-state-tracking
   gh pr create --title "feat: add pool state tracking" \
                --body "$(cat PR_TEMPLATE.md)"
   ```

4. **PR must include:**
   - Clear description of changes
   - Breaking changes clearly marked
   - All CI checks passing
   - Performance impact analysis (if applicable)
   - Updated documentation

## Local Pre-Commit Hooks

Create `.githooks/pre-commit`:

```bash
#!/bin/bash
set -e

echo "Running pre-commit checks..."

# Format check
cargo fmt --all -- --check || {
    echo "❌ Format check failed. Run: cargo fmt --all"
    exit 1
}

# Clippy
cargo clippy --workspace -- -D warnings || {
    echo "❌ Clippy check failed"
    exit 1
}

# Protocol V2 tests
cargo test --package protocol_v2 --test tlv_parsing || {
    echo "❌ Protocol V2 validation failed"
    exit 1
}

echo "✅ All pre-commit checks passed"
```

Install hooks:
```bash
git config core.hooksPath .githooks
chmod +x .githooks/pre-commit
```

## Continuous Deployment (CD)

### Environment-Specific Deployments

```yaml
# .github/workflows/deploy-staging.yml
name: Deploy to Staging

on:
  push:
    branches: [staging]

jobs:
  deploy:
    runs-on: ubuntu-latest
    environment: staging
    steps:
      - uses: actions/checkout@v4
      - name: Deploy to staging
        run: |
          # Build optimized binaries
          cargo build --release --workspace
          
          # Deploy using your preferred method
          # Example: rsync to staging server
          rsync -avz target/release/ ${{ secrets.STAGING_HOST }}:/opt/alphapulse/
```

### Production Deployment Checklist

Before deploying to production:

1. **Performance validation**
   ```bash
   cargo run --bin test_protocol --release
   # Verify: >1M msg/s construction, >1.6M msg/s parsing
   ```

2. **Security audit**
   ```bash
   cargo audit
   cargo outdated --depth 1
   ```

3. **Breaking change review**
   ```bash
   cargo semver-checks check-release --baseline-rev v1.0.0
   ```

4. **Integration tests**
   ```bash
   cargo test --workspace --release
   ```

5. **Documentation**
   - Update CHANGELOG.md
   - Update API documentation
   - Tag release with semantic version

## Crate and Workspace Management

### ⚠️ Critical: Avoid Virtual Workspace Trap

**A crate can only belong to ONE workspace.** Never create nested workspaces in subdirectories. The root `backend_v2` workspace is the only true workspace.

```toml
# ❌ WRONG - Don't do this in services_v2/adapters/Cargo.toml
[workspace]
members = ["polygon", "coinbase"]

# ✅ CORRECT - Only in root backend_v2/Cargo.toml
[workspace]
members = [
    "services_v2/adapters/polygon",
    "services_v2/adapters/coinbase",
]
```

Sub-directory `Cargo.toml` files should only define shared dependencies, not create new workspaces.

### Adding New Services

When adding a new service (see `.agents/personas/architect.md` for philosophy):

1. **Create service crate**
   ```bash
   cargo new services_v2/adapters/new-exchange --bin
   ```

2. **Update root workspace**
   ```toml
   # backend_v2/Cargo.toml
   [workspace]
   members = [
       # ...
       "services_v2/adapters/new-exchange",  # Add new service
   ]
   ```

3. **Configure service Cargo.toml**
   ```toml
   [package]
   name = "new-exchange-collector"
   version = "0.1.0"
   edition = "2021"

   [dependencies]
   protocol_v2 = { path = "../../../protocol_v2" }
   tokio = { workspace = true }
   ```

4. **Add to CI matrix**
   ```yaml
   strategy:
     matrix:
       service: [polygon, coinbase, new-exchange]
   ```

## Documentation Quality Gates

### Unified Documentation with Rustdoc

Bridge markdown and rustdoc using `#[doc(include)]` to create a single source of truth:

```rust
//! # AlphaPulse Protocol V2
//!
//! Core protocol implementation.
//!
#![doc(include = "../docs/protocol_overview.md")]

// This embeds external markdown directly into rustdoc output
```

This technique:
- Maintains high-level docs in markdown (easier for prose)
- Presents unified documentation via `cargo doc`
- Versions documentation with code
- Enables navigation through rustdoc

### Doctest Verification

All code examples in documentation must be runnable:

```rust
/// # Examples
/// 
/// ```
/// use alphapulse_protocol_v2::TradeTLV;
/// 
/// let trade = TradeTLV::new(instrument_id, price, quantity);
/// assert_eq!(trade.price, price); // This will be tested!
/// ```
pub struct TradeTLV { /* ... */ }
```

### Documentation Coverage

Check documentation coverage:
```bash
cargo doc --workspace --no-deps
cargo rustdoc -- -Z unstable-options --show-coverage
```

## Monitoring and Alerts

### GitHub Actions Status Badge

Add to README.md:
```markdown
[![CI](https://github.com/alphapulse/backend_v2/actions/workflows/ci.yml/badge.svg)](https://github.com/alphapulse/backend_v2/actions/workflows/ci.yml)
```

### Failure Notifications

Configure in workflow:
```yaml
- name: Notify on failure
  if: failure()
  uses: 8398a7/action-slack@v3
  with:
    status: ${{ job.status }}
    text: 'CI failed on ${{ github.ref }}'
    webhook_url: ${{ secrets.SLACK_WEBHOOK }}
```

## Quick Reference

### Essential CI Commands
```bash
# Run all CI checks locally
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
cargo audit
cargo semver-checks check-release --baseline-rev main

# Protocol V2 specific
cargo test --package protocol_v2
cargo run --bin test_protocol --release
```

### Fix Common CI Failures
```bash
# Format issues
cargo fmt --all

# Clippy warnings
cargo clippy --workspace --fix

# Outdated dependencies
cargo update

# Security vulnerabilities
cargo audit fix
```

## Best Practices

1. **Never skip CI checks** - They protect production
2. **Fix warnings immediately** - Don't let them accumulate
3. **Keep CI fast** - Use caching, parallel jobs
4. **Test locally first** - Run pre-commit hooks
5. **Document breaking changes** - Be explicit in commit messages
6. **Monitor performance** - Regression tests are critical
7. **Automate everything** - If you do it twice, automate it
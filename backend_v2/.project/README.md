# .project/ - Organized Project Configuration

This directory contains all project configuration files in a single, organized location instead of scattered across the root directory as hidden dotfiles.

## 🎯 Why .project/?

**Problem**: Traditional projects scatter configuration across dozens of dotfiles:
- `.cargo/`, `.pre-commit-config.yaml`, `.rq.toml`, `clippy.toml`, etc.
- Hard to find, backup, or understand project structure
- Each tool creates its own convention

**Solution**: Single `.project/` directory with flat, organized structure.

## 📁 Structure

```
.project/
├── cargo.toml              # Rust toolchain configuration
├── clippy.toml            # Clippy linting rules  
├── deny.toml              # Cargo-deny security scanning
├── pre-commit.yaml        # Pre-commit hooks configuration
├── rq.toml               # RQ tool configuration
├── secretsignore         # Secret scanning exclusions
├── workspace_deps_test.toml # Workspace dependency testing
└── git-hooks/            # Git hooks (directory)
    └── pre-commit        # Pre-commit hook script
```

## ⚙️ Tool Configuration

### Cargo (Rust)
```bash
export CARGO_HOME=.project
# Now cargo reads .project/cargo.toml instead of .cargo/config.toml
```

### Pre-commit
```bash
pre-commit --config .project/pre-commit.yaml install
```

### Git Hooks
```bash
git config core.hooksPath .project/git-hooks
```

### RQ Tool
The `rq` tool needs to be updated to check `.project/rq.toml` first, then fallback to `.rq.toml`.

## 🎉 Benefits

1. **Clean Root Directory**: Only essential, unmovable files remain (`.git/`, `.github/`, etc.)
2. **Easy Discovery**: One place to find all project configuration
3. **Simple Backup**: `cp -r .project/ backup/` backs up all config
4. **Clear Separation**: Code vs. configuration is obvious
5. **Tool Agnostic**: Any tool can adopt this pattern

## 🚀 Future Vision

This could become a standard:
- Language ecosystems adopt `.project/` instead of scattered dotfiles
- IDEs look for `.project/metadata.json` to understand project type
- CI/CD systems check `.project/` for build configuration
- Containerization becomes simpler with unified config location

## 📚 References

This implementation is inspired by the XDG Base Directory Specification, but applied to project-level configuration instead of user-level.

---

*This is likely one of the first real-world .project/ implementations - a proof of concept for better project organization.*
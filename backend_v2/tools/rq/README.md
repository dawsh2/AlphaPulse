# rq - Rust Query

A fast, extensible codebase navigation and discovery tool for Rust projects. Prevents code duplication by making existing implementations instantly discoverable.

## Features

- 🔍 **Smart Search**: Find types, functions, and patterns with fuzzy matching
- 🚀 **Fast**: SQLite cache + Bloom filters for instant queries
- 🔌 **Extensible**: Plugin system for domain-specific functionality
- 📊 **Analytics**: Codebase statistics and dependency analysis
- 🎨 **Multiple UIs**: CLI, Interactive TUI, JSON output
- 🔧 **CI/CD Ready**: Duplicate detection and pattern checking

## Installation

```bash
# Install from source
cargo install --path .

# Or download pre-built binary (coming soon)
# curl -L https://github.com/yourusername/rq/releases/latest/download/rq-linux-amd64 -o rq
# chmod +x rq
```

## Quick Start

```bash
# Initialize rq for your project
rq init --with-config

# Update the cache
rq update

# Find all structs containing "Config"
rq find Config --type struct

# Check if something exists before implementing
rq check parse_json

# Find similar implementations
rq similar validate

# Interactive mode with fuzzy search
rq

# Search documentation
rq docs "error handling"

# Show codebase statistics
rq stats --by-type
```

## Core Commands

### `rq find <pattern>`
Search for types, functions, or patterns in the codebase.

```bash
rq find Handler                     # Find anything with "Handler"
rq find --type function parse       # Find functions only
rq find --public API                # Find public APIs only
rq find --module net                # Search within a module
rq find --crate server              # Search specific crate
```

### `rq similar <pattern>`
Find similar implementations to avoid duplication.

```bash
rq similar validate                 # Find validation functions
rq similar --threshold 0.9 parse    # Very similar matches only
```

### `rq check <name>`
Quick existence check before implementing.

```bash
rq check parse_config               # Check if exists
rq check --suggest validte          # With typo correction
```

### `rq docs <pattern>`
Search documentation strings.

```bash
rq docs "thread safety"             # Search in docs
rq docs --full async                # Show full documentation
```

### `rq signature`
Find by function signature patterns.

```bash
rq signature --returns Result       # Functions returning Result
rq signature --params "&str"        # Functions taking &str
rq signature --bounds Clone         # With Clone trait bound
```

### `rq deps <item>`
Analyze dependencies and relationships.

```bash
rq deps MyStruct                    # What MyStruct depends on
rq deps --reverse MyTrait          # What depends on MyTrait
rq deps --depth 3 function_name    # Deeper analysis
```

## Configuration

Create `.rq.toml` in your project root:

```toml
[cache]
max_size_mb = 500
expire_after_days = 7
auto_update = false

[ui]
theme = "default"
max_suggestions = 5
fuzzy_threshold = 0.7

# Custom aliases
[aliases]
ps = "find --type struct"
pf = "find --type function"
tests = "find test_ --type function"
api = "find --public --type function"

# CI/CD patterns
[[patterns]]
name = "todo_comments"
pattern = "TODO|FIXME"
description = "Unfinished code"
severity = "warning"

[[patterns]]
name = "debug_code"
pattern = "dbg!|println!"
description = "Debug code in production"
severity = "error"
```

## Interactive Mode

Launch without arguments for TUI mode:

```bash
rq
```

- **Type to search** in real-time
- **Arrow keys** to navigate results
- **Enter** to view details
- **Tab** for filters
- **Esc** to clear
- **q** to quit

## CI/CD Integration

### GitHub Actions

```yaml
name: Code Quality
on: [push, pull_request]

jobs:
  check-duplicates:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      
      - name: Install rq
        run: cargo install rq
      
      - name: Check for duplicates
        run: rq ci --max-similarity 0.8 --strict
      
      - name: Check patterns
        run: rq ci --patterns .rq-patterns.toml
```

### Pre-commit Hook

```bash
#!/bin/sh
# .git/hooks/pre-commit

# Check for code duplication
rq ci --max-similarity 0.85 || {
    echo "Error: Code duplication detected!"
    echo "Run 'rq similar <name>' to find existing implementations"
    exit 1
}
```

## Plugin System

Extend rq with custom functionality:

```rust
// my_plugin/src/lib.rs
use rq_plugin::{Plugin, PluginResult};

#[derive(Default)]
pub struct MyPlugin;

impl Plugin for MyPlugin {
    fn name(&self) -> &str {
        "my_plugin"
    }
    
    fn execute(&self, args: &[String]) -> PluginResult {
        // Custom functionality
        Ok(())
    }
}

// Export the plugin
rq_plugin::export_plugin!(MyPlugin);
```

Load in `.rq.toml`:

```toml
[plugins.my_plugin]
enabled = true
path = "~/.rq/plugins/my_plugin.so"
```

## Performance

- **Bloom Filters**: O(1) existence checks
- **SQLite Cache**: Indexed queries on 100k+ items
- **Incremental Updates**: Only re-parse changed files
- **Parallel Processing**: Multi-threaded rustdoc generation

Typical performance on mid-size projects (50k items):
- Initial cache build: ~5 seconds
- Incremental update: <100ms
- Query response: <10ms
- Fuzzy search: <50ms

## Architecture

```
┌─────────────────────────────────────┐
│          User Interface             │
│  (CLI / TUI / LSP / JSON)          │
└─────────────────────────────────────┘
                 │
┌─────────────────────────────────────┐
│          Query Engine               │
│  (Pattern Matching / Fuzzy Search)  │
└─────────────────────────────────────┘
                 │
┌─────────────────────────────────────┐
│          Cache Layer                │
│  (SQLite + Bloom Filter)           │
└─────────────────────────────────────┘
                 │
┌─────────────────────────────────────┐
│       Rustdoc JSON Parser           │
│  (Incremental Updates)              │
└─────────────────────────────────────┘
```

## Comparison with Similar Tools

| Feature | rq | rust-analyzer | cargo-doc | grep/ripgrep |
|---------|-----|--------------|-----------|--------------|
| Speed | ⚡ <10ms | ⚡ Fast | 🐢 Slow | ⚡ Fast |
| Fuzzy Search | ✅ | ✅ | ❌ | ❌ |
| Similarity Detection | ✅ | ❌ | ❌ | ❌ |
| Cached Results | ✅ | ✅ | ❌ | ❌ |
| Plugin System | ✅ | ✅ | ❌ | ❌ |
| Works Offline | ✅ | ✅ | ✅ | ✅ |
| CI/CD Integration | ✅ | ⚠️ | ❌ | ⚠️ |

## Examples

### Before implementing a new function:
```bash
$ rq check validate_email
❌ validate_email not found

$ rq similar validate
Found 3 similar items:
  fn: validate_input (src/validation.rs)
  fn: validate_config (src/config.rs)
  fn: validate_user (src/auth.rs)
⚠️  Review these before implementing 'validate'
```

### Finding API surface:
```bash
$ rq find --public --type function --crate mylib
fn: from_str (src/parser.rs)
fn: to_string (src/formatter.rs)
fn: validate (src/validation.rs)
```

### Analyzing dependencies:
```bash
$ rq deps Config
Dependencies for 'Config':
  serde::Deserialize
  std::path::PathBuf
    std::fs::File
```

## Troubleshooting

### Cache not updating
```bash
# Force rebuild
rq update --force

# Check cache location
rq stats
```

### Slow queries
```bash
# Check cache size
rq stats --detailed

# Reduce cache size in .rq.toml
[cache]
max_size_mb = 200
```

### Missing items
```bash
# Ensure rustdoc JSON generation works
cargo +nightly rustdoc --lib -- --output-format json -Z unstable-options

# Update with verbose output
rq update -v
```

## Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Development Setup
```bash
# Clone the repository
git clone https://github.com/yourusername/rq
cd rq

# Run tests
cargo test

# Run with debug output
RUST_LOG=rq=debug cargo run -- find test
```

## License

MIT - See [LICENSE](LICENSE) for details.

## Acknowledgments

- Built with [rustdoc JSON](https://github.com/rust-lang/rust/issues/76578)
- Fuzzy matching by [skim](https://github.com/lotabout/skim)
- TUI framework: [ratatui](https://github.com/ratatui-org/ratatui)
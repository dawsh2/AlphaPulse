# Org-Mode Documentation Workflow

## Quick Start

### 1. Install Git Hook (One-time Setup)
```bash
# Enable the pre-commit hook
git config core.hooksPath .githooks
```

### 2. Edit Documentation (In Org-mode)
```bash
# Edit the source documentation
emacs .claude/docs/source/protocol/tlv_types.org
```

### 3. Convert to Markdown (Manual or Automatic)
```bash
# Manual conversion
.claude/tools/org-to-md.sh

# Automatic conversion happens on git commit
git add .claude/docs/source/protocol/tlv_types.org
git commit -m "Update TLV documentation"
# Hook automatically generates .claude/docs/generated/protocol/tlv_types.md
```

### 4. Use in Rust Code
```rust
// At the top of your module
#![doc = include_str!("../.claude/docs/generated/protocol/tlv_types.md")]

// Or for specific items
#[doc = include_str!("../.claude/docs/generated/protocol/tlv_types.md")]
pub struct MyType;
```

## Documentation Structure

```
.claude/
├── docs/
│   ├── source/              # Org-mode files (EDIT HERE)
│   │   ├── protocol/        # Protocol documentation
│   │   │   ├── tlv_types.org
│   │   │   ├── messages.org
│   │   │   └── precision.org
│   │   ├── architecture/    # System architecture
│   │   │   ├── overview.org
│   │   │   └── domains.org
│   │   └── api/            # API documentation
│   │       ├── codec.org
│   │       └── network.org
│   └── generated/          # Auto-generated Markdown (DO NOT EDIT)
│       └── [mirrors source structure]
└── tools/
    └── org-to-md.sh        # Conversion script

```

## Org-mode Features You Can Use

### Tables with Auto-formatting
```org
| Type | Range | Domain | Performance |
|------+-------+--------+-------------|
| Trade | 1 | MarketData | <35μs |
| Signal | 20-39 | Signal | <100μs |
```

### Code Blocks with Syntax Highlighting
```org
#+BEGIN_SRC rust
let message = TLVMessageBuilder::new(domain, source)
    .add_tlv(TLVType::Trade, &trade)
    .build();
#+END_SRC
```

### Properties and Metadata
```org
:PROPERTIES:
:RELAY: MarketDataRelay
:PRIORITY: HOT_PATH
:END:
```

### TODO States (for tracking documentation tasks)
```org
* TODO Document new TLV type
* DONE Update precision documentation
```

## Benefits

1. **Single Source of Truth**: Edit once in Org, appears everywhere
2. **No Duplication**: Same docs in Rust API, README files, and documentation
3. **Org-mode Power**: Tables, TODO states, exports, literate programming
4. **Version Control**: Track changes in readable Org format
5. **Automatic Updates**: Git hooks ensure Markdown is always in sync

## Examples in Production

### Codec Library
```rust
// libs/codec/src/lib.rs
#![doc = include_str!("../.claude/docs/generated/protocol/tlv_types.md")]
```

### Types Module
```rust
// libs/types/src/protocol/tlv/mod.rs
#![doc = include_str!("../../../.claude/docs/generated/protocol/tlv_types.md")]
```

## Common Tasks

### Add New Documentation
1. Create `.org` file in `.claude/docs/source/`
2. Run `.claude/tools/org-to-md.sh`
3. Add `#[doc = include_str!(...)]` to Rust code
4. Verify with `cargo doc --open`

### Update Existing Documentation
1. Edit the `.org` file
2. Changes auto-convert on commit
3. Rust documentation updates automatically

### Debug Conversion Issues
```bash
# Check Emacs batch mode
emacs --version

# Test conversion manually
emacs file.org --batch \
    --eval "(require 'ox-md)" \
    --funcall org-md-export-to-markdown

# View generated Markdown
cat .claude/docs/generated/protocol/tlv_types.md
```

## Troubleshooting

### Emacs Not Found
Install Emacs:
```bash
# macOS
brew install emacs

# Linux
apt-get install emacs-nox  # No GUI version
```

### Org Export Not Working
Ensure Org-mode and markdown exporter are available:
```elisp
;; In your Emacs config
(require 'ox-md)
```

### Hook Not Running
Check git hooks configuration:
```bash
git config core.hooksPath
# Should output: .githooks
```

## Best Practices

1. **Keep Org Files Focused**: One topic per file for better organization
2. **Use Descriptive Headers**: Help navigation in both Org and generated docs
3. **Include Examples**: Code examples make documentation more useful
4. **Update on Changes**: Keep documentation in sync with code changes
5. **Review Generated Output**: Check Markdown output after major edits
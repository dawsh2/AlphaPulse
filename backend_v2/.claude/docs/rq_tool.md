# rq (Rust Query) - Documentation-First System Navigator

## Purpose
Transform from tactical code finder to strategic system navigator through comprehensive documentation discovery.

## Architecture
Simple semantic discovery tool with enhanced documentation indexing
- **Location**: `backend_v2/tools/rq/`
- **Philosophy**: Documentation-first development enabling rich system understanding
- **Design**: Direct rustdoc JSON parsing, no over-engineering, focused on discoverability

## Enhanced Capabilities
- **Strategic Understanding**: `rq docs` searches find comprehensive system context
- **Architecture Discovery**: Understand component relationships and data flow
- **Performance Awareness**: Access measured metrics and optimization guidance
- **Integration Patterns**: Discover how services connect and communicate
- **Error Strategies**: Find comprehensive error handling and recovery patterns

## Key Features
- Pattern search with regex support  
- Type filtering (struct, enum, function, etc.)
- **Documentation search** with structured content discovery
- Usage examples from test files  
- Relationship discovery (what calls what)
- Fuzzy matching for typos and alternatives

## Why This Design
- ❌ **Avoided Over-Engineering**: No SQLite, bloom filters, plugins, TUI, LSP server
- ✅ **Documentation-First**: Rich `//!` content enables strategic navigation
- ✅ **Simple & Fast**: Direct JSON parsing, file-based caching, maintainable codebase
- ✅ **Focused**: Solves system understanding gap identified in codebase exploration
- ✅ **Maintainable**: Easy to understand, modify, and extend

## Installation & Usage

### Installation
```bash
# Install (in rq directory)
cd backend_v2/tools/rq
cargo install --path .
```

### Essential Discovery Commands
```bash
# CRITICAL: Always check before implementing new functionality
rq check TradeTLV               # Verify implementation exists
rq check validate_pool          # Check if functionality missing
rq similar TradeTLV             # Find similar implementations with fuzzy matching

# Strategic System Understanding (NEW - Enhanced Documentation)
rq docs "zero-copy serialization"    # Find implementation patterns and performance details
rq docs "performance profile"        # Get measured metrics across system
rq docs "architecture role"          # Understand component relationships
rq docs "integration points"         # Discover how components connect
rq docs "message flow"               # Understand data flow patterns
rq docs "error handling"             # Find comprehensive error strategies
rq docs "packed struct safety"      # Critical ARM/M1 crash prevention
rq docs "sequence management"        # Message ordering and recovery patterns
rq docs "batch processing"           # Performance optimization techniques
rq docs "monitoring observability"   # System health and metrics patterns
```

### Tactical Code Discovery (Existing)
```bash
# Find existing implementations (basic patterns)
rq find Pool                    # All Pool-related code
rq find TLV --type struct       # All TLV structures
rq find parse --type function   # All parsing functions
rq find Pool --public           # Only public Pool APIs

# Advanced pattern matching with regex
rq find "^Pool.*TLV$" --regex   # Pool TLV structs with exact pattern
rq find "Trade|Quote" --regex   # Multiple patterns

# Find usage examples and relationships
rq examples TradeTLV            # Show real usage from test files
rq callers execute_arbitrage    # What functions call this
rq calls TradeTLV               # What this calls (simplified)

# Update and maintenance
rq stats                        # Cache statistics and health
rq update                       # Update rustdoc cache
rq update --force               # Force complete rebuild
```

### System Understanding Queries (NEW)
```bash
# Protocol V2 Architecture Discovery
rq docs "TLV message format"         # Complete protocol specification
rq docs "domain-based routing"       # Relay routing architecture
rq docs "size constraints"           # Fixed/bounded/variable performance trade-offs
rq docs "extended TLV format"        # Large payload handling

# Performance-Critical Paths
rq docs "hot path"                   # <35μs processing requirements
rq docs "1M messages/second"         # Throughput specifications
rq docs "memory safety"              # ARM crash prevention patterns
rq docs "cache-line alignment"       # Performance optimization details

# Development Workflow Integration
rq docs "service integration"        # Producer/consumer patterns
rq docs "type discovery"             # Runtime API exploration
rq docs "validation strategies"      # Comprehensive error handling
rq docs "recovery protocol"          # Message gap detection and repair

# Architecture Diagrams (NEW - Mermaid Support)
rq docs "mermaid"                    # Find all Mermaid architecture diagrams
rq docs "diagram"                    # Find diagram references and documentation
rq docs "architecture role"          # Traditional ASCII diagram search
```

## Key Enhancement
The `rq docs` searches now return rich, structured information including:
- System architecture and component relationships
- Performance characteristics with measured metrics  
- Integration patterns and service boundaries
- Error handling strategies and recovery protocols
- Safety guidelines and platform-specific considerations
- Complete examples with context and best practices

## Quick Reference Commands
```bash
# System understanding
rq docs "architecture"           # Component relationships
rq docs "performance"            # Measured metrics
rq docs "integration"            # Service connectivity
rq docs "mermaid"               # Find architecture diagrams
rq docs "diagram"               # Find all diagram references

# Before implementing
rq check function_name           # Prevent duplication
rq similar function_name         # Find existing patterns
rq docs "relevant domain"        # Understand context

# Cache maintenance  
rq update                        # Refresh documentation index

# Diagram aliases (after sourcing .bashrc_query_builder)
rqd                             # Short alias for mermaid search
rq-show-diagrams                # List all available diagrams
```

## Important Notes
- **ALWAYS** run `rq check <name>` before implementing to prevent code duplication
- Maintains our "One Canonical Source" principle
- Direct rustdoc JSON parsing with comprehensive documentation indexing
- No database overhead, just fast semantic discovery with strategic system understanding

## Mermaid Diagram Integration (NEW)

### Overview
Enhanced documentation now includes interactive Mermaid diagrams using the aquamarine crate:
- **Rich SVG diagrams** in rustdoc output
- **Searchable via rq** - find diagrams easily
- **GitHub integration** - renders in PRs and README files
- **Easy maintenance** - Mermaid syntax is much better than ASCII art

### Diagram Search Commands
```bash
# BEST: Find diagram functions directly
rq find architecture_diagram   # Clean list of diagram functions

# Clean diagram discovery (organized view)
source .bashrc_query_builder   # Load once per session  
diagrams                       # or rqd - organized list with package info

# View diagram source directly
grep -A 50 '```mermaid' libs/amm/src/lib.rs  # Show raw Mermaid source

# AVOID: rq docs "diagram" - shows messy HTML fragments
# AVOID: rq docs "mermaid" - shows HTML fragments
```

### Current Diagrams
- **AMM Library**: Input → Math → Sizing → Strategy flow
- **Adapter Service**: Exchanges → Adapters → Relays → Services flow

### Creating New Diagrams
```rust
/// Architecture diagram showing component relationships
#[cfg_attr(doc, aquamarine::aquamarine)]
/// ```mermaid
/// graph LR
///     A[Component A] --> B[Component B]
///     B --> C[Component C]
/// ```
pub fn architecture_diagram() {
    // Function exists for documentation only
}
```

## Impact
Transforms `rq` from basic code search to comprehensive system understanding tool, enabling developers to:
- Understand system architecture without reading implementation files
- **Visualize component relationships** through interactive diagrams
- Discover performance requirements and optimization patterns
- Find integration points and service boundaries  
- Access error handling strategies and recovery protocols
- Prevent code duplication through rich discoverability
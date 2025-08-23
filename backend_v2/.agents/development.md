# Development Guide

## Development Workflow

### Before Making Changes
1. **Ask Clarifying Questions**: Present questions to ensure complete understanding of requirements and technical trade-offs
2. **Use rq for Discovery**: Search for existing functionality and understand system architecture
3. Read relevant CLAUDE.md files in subdirectories
4. Run existing tests to understand current behavior
5. Check for related issues or ongoing migrations
6. Update existing files instead of creating duplicates with adjective prefixes
7. Respect project structure - place files in their correct service directory
8. **Write Comprehensive Documentation**: Include structured `//!` comments for system discoverability

### Enhanced rq-Based Development Workflow
```bash
# 1. DISCOVER - Check existing functionality and architecture
rq check new_feature_name           # Verify it doesn't exist
rq similar new_feature_name         # Find similar implementations
rq docs "relevant domain"           # Understand system context

# 2. UNDERSTAND - Learn system architecture and patterns
rq docs "integration points"        # How to connect with existing system
rq docs "performance profile"       # Performance requirements and constraints
rq docs "error handling"            # Existing error patterns to follow

# 3. IMPLEMENT - Build with comprehensive documentation
# Write structured //! documentation covering:
# - Purpose and system role
# - Integration points and dependencies  
# - Architecture role with data flow
# - Performance characteristics
# - Complete examples with context

# 4. VALIDATE - Ensure discoverability
rq docs "new feature keywords"      # Verify documentation is findable
rq update                           # Refresh cache for new documentation
```

### Breaking Changes Philosophy
**This is a greenfield codebase - breaking changes are encouraged for system improvement:**
- **No backward compatibility concerns** - break APIs freely to improve design
- **Remove deprecated code immediately** - don't leave legacy cruft
- **Clean up after yourself** - remove old patterns when introducing new ones
- **Refactor aggressively** - improve naming, structure, and patterns without hesitation
- **Delete unused code** - don't keep "just in case" code
- **Update all references** - when changing interfaces, update ALL callers

### Breaking Change Examples (Encouraged)
```rust
// OLD: Confusing naming
pub struct ExchangeDataHandler {
    pub async fn handle_data(&self, data: String) { ... }
}

// NEW: Clear naming + breaking change
pub struct MarketDataProcessor {
    pub async fn process_market_event(&self, event: MarketEvent) { ... }
}
// DELETE the old struct entirely, update ALL references
```

## Documentation Standards

### Structured Documentation for rq Discovery
Every module should include structured `//!` documentation:

```rust
//! # ComponentName - System Component
//!
//! ## Purpose  
//! Clear explanation of component role and why it exists
//!
//! ## Integration Points
//! - **Input**: Data sources and message types received
//! - **Output**: Destinations and message types produced
//! - **Dependencies**: Required services and libraries
//!
//! ## Architecture Role
//! ```text
//! [ASCII diagram showing component in system context]
//! ```
//!
//! ## Performance Profile  
//! - **Throughput**: Measured performance characteristics
//! - **Latency**: Timing requirements and constraints
//! - **Memory**: Usage patterns and optimization notes
//!
//! ## Examples
//! ```rust
//! // Complete, realistic usage examples
//! ```
```

**Why This Matters**: Structured documentation enables `rq docs` to provide strategic system understanding, not just tactical code discovery.

### Documentation Quality Levels

#### ❌ Minimal Documentation (Avoid)
```rust
//! TLV parsing
```

#### ⚠️ Basic Documentation (Legacy - Update These)  
```rust
//! TLV (Type-Length-Value) Parsing and Processing
//! 
//! This module provides the core TLV functionality
```

#### ✅ Comprehensive Documentation (Standard)
```rust
//! # TLV Protocol System - Core Module
//!
//! ## Purpose
//! Complete explanation of system role and value
//!
//! ## Integration Points  
//! Detailed connectivity and dependencies
//!
//! ## Architecture Role
//! ASCII diagrams and system context
//!
//! ## Performance Profile
//! Measured metrics and constraints
//!
//! ## Examples
//! Complete, realistic usage patterns
```

## Development Process & Clarifying Questions

### Core Philosophy: Always Ask Clarifying Questions
**Optimize for clarity and user involvement in the development process.** Before beginning any task, present clarifying questions to ensure complete understanding. When ambiguity arises during implementation, pause and ask for guidance.

### When to Ask Clarifying Questions
1. **Before Starting Tasks**: Always present a list of questions before beginning work
2. **During Implementation**: When requirements become unclear or technical trade-offs emerge
3. **At Decision Points**: When multiple implementation approaches are possible
4. **For Complex Changes**: Especially involving Protocol V2 TLV messages, precision handling, or performance-critical paths

### Question Categories for AlphaPulse

#### Technical Architecture Questions
- **TLV Message Changes**: "Should this new field use native token precision or 8-decimal fixed-point?"
- **Performance Trade-offs**: "This optimization could improve throughput by 15% but adds complexity. Should we prioritize raw speed or maintainability?"
- **Protocol Compatibility**: "This change breaks backward compatibility with legacy services. Should we proceed or find an alternative approach?"

#### Business Logic Questions  
- **Trading Parameters**: "What should the default minimum profit threshold be for arbitrage opportunities?"
- **Risk Management**: "Should we implement circuit breakers for this new strategy, and at what thresholds?"
- **Precision Requirements**: "For this new exchange integration, should we preserve their native precision or normalize to our standard?"

#### Implementation Approach Questions
- **Service Boundaries**: "Should this functionality go in a shared library or be service-specific?"
- **Testing Strategy**: "Should we test against mainnet pools or create isolated test scenarios?"
- **Migration Path**: "How should we handle the transition from Symbol-based to InstrumentId-based code?"

### Presenting Technical Options

When asking clarifying questions:
1. **Present Clear Options**: "We can implement this as either A (fast, more complex) or B (slower, simpler). Which approach aligns better with system goals?"
2. **Include Trade-offs**: Explain performance, complexity, and maintenance implications
3. **Provide Context**: Reference relevant system invariants, performance targets, or architectural principles
4. **Be Specific**: "This change affects the hot path and could add 2-3μs latency" vs. "This might be slower"

### AlphaPulse-Specific Clarification Examples

#### DEX Integration Questions
- "Which DEX pools should we prioritize for testing? High-volume pairs or edge cases?"
- "Should we implement V2 and V3 math separately or create a unified interface?"
- "What's the acceptable slippage tolerance for execution validation?"

#### Protocol V2 Questions  
- "This new TLV type needs a unique number. Should we use the next available in the Market Data range (1-19)?"
- "Should this message include full 20-byte addresses or is a hash sufficient for this use case?"
- "What's the expected message frequency to determine optimal buffer sizes?"

#### Performance Optimization Questions
- "We can achieve sub-microsecond latency with unsafe code or maintain safety with ~5μs overhead. What's the priority?"
- "Should we optimize for memory usage or CPU cycles in this hot path component?"
- "This caching strategy could reduce RPC calls but uses 50MB additional memory. Is that acceptable?"

### User Involvement Guidelines

1. **Pause for Clarity**: Stop work immediately when requirements are unclear
2. **Technical Translation**: Explain complex technical concepts in accessible terms when needed
3. **Decision Documentation**: Record the reasoning behind technical decisions for future reference
4. **Iterative Refinement**: Re-engage when new questions arise during implementation

## Before Submitting PR
1. ✅ All tests passing (especially precision tests)
2. ✅ No performance regression
3. ✅ **Comprehensive documentation** with structured `//!` comments
4. ✅ **rq discoverability** - verify `rq docs` finds relevant content  
5. ✅ Documentation updated (including CLAUDE.md if needed)
6. ✅ Linting and formatting clean
7. ✅ Commit message follows convention
8. ✅ No duplicate files with "enhanced", "fixed", "new", "v2" prefixes
9. ✅ Files placed in correct service directories per project structure
10. ✅ **Deprecated code removed** - no legacy patterns left behind
11. ✅ **All references updated** - breaking changes propagated throughout codebase
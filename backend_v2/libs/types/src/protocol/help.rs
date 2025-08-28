//! # API Discovery and Help System
//!
//! Provides comprehensive runtime help for the Torq Protocol V2 API.
//! This module enables interactive discovery of available methods, types, and patterns
//! to reduce cognitive load and improve developer experience.
//!
//! ## Purpose
//!
//! The help system serves three primary functions:
//! 1. **API Discovery** - Find available methods without reading source code
//! 2. **Usage Guidance** - Show correct patterns and common mistakes
//! 3. **Performance Awareness** - Display performance characteristics and constraints
//!
//! ## Integration Points
//!
//! - **Interactive REPL** - Call help functions during development
//! - **Documentation Generation** - Auto-generate markdown tables from type metadata
//! - **CLI Tools** - Power `cargo api-help` and similar commands
//! - **Testing** - Validate API consistency and completeness
//!
//! ## Architecture Role
//!
//! ```text
//! Developer REPL → [Help System] → Type Metadata
//!       ↑              ↓               ↓
//!   Interactive    Runtime Info    Compile-time
//!   Discovery      Generation      Validation
//! ```
//!
//! ## Performance Profile
//!
//! - **Latency**: Zero-cost (compile-time string generation where possible)
//! - **Memory**: <1KB static strings + runtime type introspection
//! - **Usage**: Development/debugging only - not in hot paths
//!
//! ## Examples
//!
//! ```rust
//! use torq_protocol_v2::help::*;
//!
//! // Quick API discovery
//! show_instrument_id_methods();
//! show_tlv_type_methods();
//!
//! // Explore specific types
//! explore_tlv_type(TLVType::Trade);
//!
//! // Performance guidance
//! show_performance_tips();
//!
//! // Complete reference
//! show_all_help();
//! ```

use super::{RelayDomain, TLVType};
use std::fmt;

/// Format numbers with thousands separators for readability
fn format_number(n: u32) -> String {
    let s = n.to_string();
    let chars: Vec<char> = s.chars().collect();
    let mut result = String::new();

    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(*c);
    }

    result
}

/// Performance metrics for help system display
#[derive(Debug, Clone, Copy)]
pub struct PerformanceMetrics {
    /// Message construction rate (messages/second)
    pub construction_rate: u32,
    /// Message parsing rate (messages/second)
    pub parsing_rate: u32,
    /// InstrumentId operations per second
    pub instrument_ops_rate: u32,
    /// Target processing latency (microseconds)
    pub target_latency_us: u32,
    /// Memory target per service (megabytes)
    pub memory_target_mb: u32,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            construction_rate: 1_000_000,
            parsing_rate: 1_600_000,
            instrument_ops_rate: 19_000_000,
            target_latency_us: 35,
            memory_target_mb: 50,
        }
    }
}

/// Method availability status for API discovery
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MethodStatus {
    /// Method exists and is recommended
    Available,
    /// Method exists but deprecated
    Deprecated,
    /// Method does not exist (common mistake)
    NotAvailable,
    /// Method planned for future release
    Planned,
}

impl fmt::Display for MethodStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MethodStatus::Available => write!(f, "✅"),
            MethodStatus::Deprecated => write!(f, "⚠️"),
            MethodStatus::NotAvailable => write!(f, "❌"),
            MethodStatus::Planned => write!(f, "🔄"),
        }
    }
}

/// Method information for API discovery
#[derive(Debug, Clone)]
pub struct MethodInfo {
    /// Method signature
    pub signature: &'static str,
    /// Purpose and usage
    pub description: &'static str,
    /// Example usage code
    pub example: Option<&'static str>,
    /// Availability status
    pub status: MethodStatus,
    /// Performance notes
    pub performance_notes: Option<&'static str>,
}

impl MethodInfo {
    /// Create a new available method
    pub const fn available(
        signature: &'static str,
        description: &'static str,
        example: Option<&'static str>,
    ) -> Self {
        Self {
            signature,
            description,
            example,
            status: MethodStatus::Available,
            performance_notes: None,
        }
    }

    /// Create a non-available method (common mistake)
    pub const fn not_available(signature: &'static str, description: &'static str) -> Self {
        Self {
            signature,
            description,
            example: None,
            status: MethodStatus::NotAvailable,
            performance_notes: None,
        }
    }

    /// Add performance notes
    pub const fn with_performance(mut self, notes: &'static str) -> Self {
        self.performance_notes = Some(notes);
        self
    }
}

/// Display helper for formatted method information
pub fn display_method(method: &MethodInfo) {
    println!(
        "{} {} - {}",
        method.status, method.signature, method.description
    );

    if let Some(example) = method.example {
        println!("   Example: {}", example);
    }

    if let Some(notes) = method.performance_notes {
        println!("   ⚡ {}", notes);
    }
}

/// InstrumentId API discovery
///
/// Shows all available methods for creating and manipulating InstrumentId instances.
/// This function provides comprehensive coverage of the InstrumentId API to prevent
/// common mistakes and improve discoverability.
///
/// # Purpose
///
/// Addresses the most common developer question: "How do I create an InstrumentId?"
/// Displays both available methods and common mistakes to provide complete guidance.
///
/// # Usage
///
/// Call during development or in REPL to discover available functionality:
///
/// ```rust
/// # use torq_protocol_v2::help::show_instrument_id_methods;
/// show_instrument_id_methods();
/// ```
///
/// # Performance Impact
///
/// Zero - uses static strings with no runtime computation.
pub fn show_instrument_id_methods() {
    println!("=== InstrumentId Available Methods ===");
    println!();

    let methods = [
        MethodInfo::available(
            "InstrumentId::coin(venue, symbol)",
            "For cryptocurrency coins and tokens",
            Some("InstrumentId::coin(VenueId::Ethereum, \"BTC\")"),
        )
        .with_performance(">19M ops/s bijective conversion"),
        MethodInfo::available(
            "InstrumentId::stock(exchange, symbol)",
            "For traditional stocks and equities",
            Some("InstrumentId::stock(VenueId::NASDAQ, \"AAPL\")"),
        ),
        MethodInfo::available(
            "InstrumentId::bond(exchange, symbol)",
            "For bonds and fixed-income securities",
            Some("InstrumentId::bond(VenueId::NYSE, \"BOND123\")"),
        ),
        MethodInfo::available(
            "InstrumentId::ethereum_token(address)",
            "For ERC-20 tokens on Ethereum",
            Some("InstrumentId::ethereum_token(\"0xA0b86a33E6441Cc8...\")"),
        ),
        MethodInfo::available(
            "InstrumentId::polygon_token(address)",
            "For tokens on Polygon/Matic",
            Some("InstrumentId::polygon_token(\"0x2791Bca1f2de4661E...\")"),
        ),
        MethodInfo::available(
            "InstrumentId::bsc_token(address)",
            "For BEP-20 tokens on Binance Smart Chain",
            Some("InstrumentId::bsc_token(\"0x55d398326f99059fF...\")"),
        ),
        MethodInfo::available(
            "InstrumentId::arbitrum_token(address)",
            "For tokens on Arbitrum L2",
            Some("InstrumentId::arbitrum_token(\"0xFd086bC7CD5C481D...\")"),
        ),
        MethodInfo::available(
            "InstrumentId::from_u64(id)",
            "For raw numeric identifiers",
            Some("InstrumentId::from_u64(12345)"),
        )
        .with_performance("Constant-time conversion"),
        MethodInfo::available(
            "instrument.to_u64()",
            "Convert to numeric representation",
            Some("let id: u64 = btc_instrument.to_u64();"),
        )
        .with_performance("Constant-time conversion"),
    ];

    println!("Available methods:");
    for method in &methods {
        display_method(method);
    }

    println!();
    println!("Common mistakes to avoid:");

    let mistakes = [
        MethodInfo::not_available("crypto()", "Use coin() with explicit VenueId"),
        MethodInfo::not_available("currency()", "Use coin() with appropriate venue"),
        MethodInfo::not_available("forex()", "Use coin() for currency pairs"),
        MethodInfo::not_available("pair()", "Use coin() with base/quote specification"),
        MethodInfo::not_available("symbol()", "Use coin()/stock() with VenueId"),
        MethodInfo::not_available("new()", "Use specific constructor methods"),
        MethodInfo::not_available("coin(\"BTC\", \"USD\")", "Missing VenueId parameter"),
        MethodInfo::not_available("stock(\"AAPL\")", "Missing exchange VenueId parameter"),
    ];

    for mistake in &mistakes {
        display_method(mistake);
    }

    println!();
    println!("💡 Key principle: ALL methods require a VenueId (exchange/blockchain) parameter!");
    println!("💡 This ensures proper routing and prevents symbol collisions across venues.");
}

/// TLVType API discovery
///
/// Displays the complete API for discovering and working with TLV message types.
/// Essential for understanding the available message types and their metadata.
///
/// # Purpose
///
/// TLV types form the core of the Protocol V2 message system. This function helps
/// developers discover available types, understand their properties, and use the
/// metadata API effectively.
///
/// # Examples
///
/// ```rust
/// # use torq_protocol_v2::help::show_tlv_type_methods;
/// show_tlv_type_methods();
/// ```
pub fn show_tlv_type_methods() {
    println!("=== TLVType Available Methods ===");
    println!();

    println!("📋 Information methods:");
    let info_methods = [
        MethodInfo::available(
            "TLVType::Trade.name()",
            "Get human-readable type name",
            Some("\"Trade\""),
        ),
        MethodInfo::available(
            "TLVType::Trade.description()",
            "Get detailed purpose description",
            Some("\"Real-time trade execution data with precise timestamps\""),
        ),
        MethodInfo::available(
            "TLVType::Trade.type_info()",
            "Get complete metadata struct",
            Some("TypeInfo { name, description, size_constraint, ... }"),
        ),
        MethodInfo::available(
            "TLVType::Trade.size_constraint()",
            "Get size validation information",
            Some("SizeConstraint::Fixed(37) // Trade is always 37 bytes"),
        ),
        MethodInfo::available(
            "TLVType::Trade.relay_domain()",
            "Get automatic routing domain",
            Some("RelayDomain::MarketData"),
        ),
        MethodInfo::available(
            "TLVType::Trade.is_implemented()",
            "Check if type is fully implemented",
            Some("true"),
        ),
    ];

    for method in &info_methods {
        display_method(method);
    }

    println!();
    println!("🔍 Query methods:");
    let query_methods = [
        MethodInfo::available(
            "TLVType::types_in_domain(domain)",
            "Filter types by routing domain",
            Some("TLVType::types_in_domain(RelayDomain::MarketData)"),
        ),
        MethodInfo::available(
            "TLVType::all_implemented()",
            "Get all available/implemented types",
            Some("Vec<TLVType> with current implementation status"),
        ),
        MethodInfo::available(
            "TLVType::generate_markdown_table()",
            "Auto-generate documentation tables",
            Some("| Type | Number | Domain | Size | Description |"),
        ),
    ];

    for method in &query_methods {
        display_method(method);
    }

    println!();
    println!("💡 Usage pattern:");
    println!("   let info = TLVType::Trade.type_info();");
    println!("   let market_types = TLVType::types_in_domain(RelayDomain::MarketData);");
    println!("   assert!(TLVType::Trade.is_implemented());");
}

/// Relay domain routing explanation
///
/// Explains how messages are automatically routed based on TLV type numbers.
/// This is critical for understanding the system architecture and message flow.
///
/// # Purpose
///
/// The relay system automatically routes messages to appropriate handlers based on
/// TLV type numbers. This function explains the domain mapping and helps developers
/// understand where their messages will be processed.
///
/// # Architecture Impact
///
/// Routing happens at the protocol level - developers don't need to specify routing
/// explicitly. Understanding domains helps with system design and debugging.
pub fn show_relay_domains() {
    println!("=== Relay Domains and Automatic Routing ===");
    println!();
    println!("Messages route automatically based on TLV type number ranges:");
    println!();

    let domains = [
        (
            RelayDomain::MarketData,
            "1-19",
            "📊",
            "Price feeds, order books, DEX events",
            "MarketDataRelay",
        ),
        (
            RelayDomain::Signal,
            "20-39",
            "🎯",
            "Trading signals, strategy coordination",
            "SignalRelay",
        ),
        (
            RelayDomain::Execution,
            "40-59",
            "⚡",
            "Orders, fills, portfolio updates",
            "ExecutionRelay",
        ),
        (
            RelayDomain::System,
            "100-119",
            "🔧",
            "Health, errors, service discovery",
            "SystemRelay",
        ),
    ];

    for (domain, range, icon, purpose, relay) in &domains {
        let types = TLVType::types_in_domain(*domain);

        println!("{} {:?} (Types {}):", icon, domain, range);
        println!("   Purpose: {}", purpose);
        println!("   Routes to: {}", relay);
        println!("   {} implemented types", types.len());

        // Show first few types as examples
        if !types.is_empty() {
            print!("   Examples: ");
            let examples: Vec<String> = types
                .iter()
                .take(3)
                .map(|t| format!("{}({})", t.name(), *t as u8))
                .collect();
            println!("{}", examples.join(", "));
            if types.len() > 3 {
                println!("             ... and {} more", types.len() - 3);
            }
        }
        println!();
    }

    println!("🎯 Key insight: Routing is completely automatic!");
    println!("   Just create the correct TLV type - the system handles routing.");
    println!();
    println!("💡 Domain separation ensures:");
    println!("   • Market data doesn't interfere with execution");
    println!("   • Signals have dedicated processing resources");
    println!("   • System messages get priority handling");
}

/// Common API mistakes and solutions
///
/// Interactive reference for the most frequent confusion points.
/// Based on real developer feedback and support requests.
///
/// # Purpose
///
/// Reduces support burden by preemptively addressing common mistakes.
/// Serves as a quick reference during development to avoid pitfalls.
///
/// # Coverage
///
/// - Method naming confusion
/// - Parameter ordering mistakes  
/// - Trait usage errors
/// - Conversion pattern errors
pub fn show_common_mistakes() {
    println!("=== Common API Mistakes and Solutions ===");
    println!();

    let mistake_pairs = [
        (
            "❌ InstrumentId::crypto(\"BTC\", \"USD\")",
            "✅ InstrumentId::coin(VenueId::Ethereum, \"BTC\")",
            "crypto() doesn't exist - use coin() with explicit venue",
        ),
        (
            "❌ InstrumentId::currency(\"USD\")",
            "✅ InstrumentId::coin(VenueId::Ethereum, \"USD\")",
            "currency() doesn't exist - USD is treated as a coin/token",
        ),
        (
            "❌ InstrumentId::stock(\"AAPL\")",
            "✅ InstrumentId::stock(VenueId::NASDAQ, \"AAPL\")",
            "Missing required exchange parameter",
        ),
        (
            "❌ trade.to_bytes()",
            "✅ trade.as_bytes()  // zerocopy trait",
            "Use as_bytes() for zero-copy serialization",
        ),
        (
            "❌ TLVMessage::from(trade)",
            "✅ TLVMessageBuilder::new(domain, source).add_tlv(type, &trade).build()",
            "TLVMessage requires builder pattern for proper header construction",
        ),
        (
            "❌ println!(\"{}\", trade_tlv.price);  // Direct access",
            "✅ let price = trade_tlv.price; println!(\"{}\", price);",
            "Copy packed fields before use to avoid alignment issues",
        ),
        (
            "❌ let unknown = TLVType::CustomTrade;",
            "✅ let available = TLVType::all_implemented();",
            "CustomTrade doesn't exist - discover available types dynamically",
        ),
        (
            "❌ let id: u64 = instrument.into();",
            "✅ let id: u64 = instrument.to_u64();",
            "Use explicit conversion methods for clarity",
        ),
    ];

    for (mistake, solution, explanation) in &mistake_pairs {
        println!("{}", mistake);
        println!("{}", solution);
        println!("   💡 {}", explanation);
        println!();
    }

    println!("🔍 When in doubt:");
    println!("   • Check examples/ directory for working code");
    println!("   • Run `cargo docs` for complete API reference");
    println!("   • Use help functions like show_instrument_id_methods()");
    println!("   • Look for similar patterns in existing code");
}

/// Interactive type explorer
///
/// Shows comprehensive information about a specific TLV type including metadata,
/// relationships, and usage guidance.
///
/// # Arguments
///
/// * `tlv_type` - The TLV type to explore in detail
///
/// # Examples
///
/// ```rust
/// # use torq_protocol_v2::help::explore_tlv_type;
/// # use torq_protocol_v2::TLVType;
/// explore_tlv_type(TLVType::Trade);
/// explore_tlv_type(TLVType::PoolSwap);
/// ```
pub fn explore_tlv_type(tlv_type: TLVType) {
    let info = tlv_type.type_info();

    println!("=== TLV Type Deep Dive: {} ===", info.name);
    println!();

    println!("📋 Basic Information:");
    println!("   Type Number: {}", info.type_number);
    println!("   Name: {}", info.name);
    println!("   Description: {}", info.description);
    println!("   Status: {:?}", info.status);
    println!();

    println!("📏 Size Information:");
    println!("   Constraint: {:?}", info.size_constraint);
    match info.size_constraint {
        crate::validation::SizeConstraint::Fixed(size) => {
            println!("   ⚡ Performance: Optimal (no size validation needed)");
            println!("   📦 Memory: {} bytes per message", size);
        }
        crate::validation::SizeConstraint::Bounded { min, max } => {
            println!("   ⚠️  Performance: Single bounds check required");
            println!("   📦 Memory: {}-{} bytes per message", min, max);
        }
        crate::validation::SizeConstraint::Variable => {
            println!("   🐌 Performance: Dynamic allocation required");
            println!("   📦 Memory: Variable size (use with caution in hot paths)");
        }
    }
    println!();

    println!("🎯 Routing Information:");
    println!("   Domain: {:?}", info.relay_domain);
    println!("   Routes to: {:?}Relay", info.relay_domain);
    println!(
        "   Priority: {}",
        match info.relay_domain {
            RelayDomain::System => "Highest (system messages)",
            RelayDomain::Execution => "High (trading critical)",
            RelayDomain::MarketData => "Normal (data feeds)",
            RelayDomain::Signal => "Normal (strategy coordination)",
        }
    );
    println!();

    if !info.examples.is_empty() {
        println!("💡 Usage Examples:");
        for (i, example) in info.examples.iter().enumerate() {
            println!("   {}. {}", i + 1, example);
        }
        println!();
    }

    // Show related types in the same domain
    let related_types = TLVType::types_in_domain(info.relay_domain);
    if related_types.len() > 1 {
        println!("🔗 Related types in {:?} domain:", info.relay_domain);
        let mut shown = 0;
        for related in &related_types {
            if *related != tlv_type && shown < 5 {
                println!("   • {} (Type {})", related.name(), *related as u8);
                shown += 1;
            }
        }
        if related_types.len() > 6 {
            println!("   ... and {} more", related_types.len() - 6);
        }
        println!();
    }

    // Performance guidance
    println!("⚡ Performance Guidance:");
    match info.size_constraint {
        crate::validation::SizeConstraint::Fixed(_) => {
            println!("   ✅ Excellent for hot paths - zero overhead parsing");
            println!("   ✅ Use in high-frequency message processing");
        }
        crate::validation::SizeConstraint::Bounded { .. } => {
            println!("   ⚠️  Good for moderate frequency - single bounds check");
            println!("   ✅ Suitable for most use cases");
        }
        crate::validation::SizeConstraint::Variable => {
            println!("   ⚠️  Use sparingly in hot paths - requires dynamic allocation");
            println!("   💡 Consider batching or caching for better performance");
        }
    }
}

/// Performance characteristics and optimization guidance
///
/// Displays measured performance metrics and provides concrete optimization
/// recommendations based on actual benchmarks.
///
/// # Purpose
///
/// Helps developers make informed decisions about performance trade-offs.
/// All metrics are based on real measurements, not theoretical estimates.
///
/// # Metrics Source
///
/// Performance numbers come from:
/// - Criterion benchmark suite in benches/
/// - Production monitoring data
/// - Continuous integration performance tests
pub fn show_performance_tips() {
    let metrics = PerformanceMetrics::default();

    println!("=== Performance Characteristics (Measured) ===");
    println!();

    println!("🚀 Throughput Benchmarks:");
    println!(
        "   Message Construction: >{} msg/s",
        format_number(metrics.construction_rate)
    );
    println!(
        "   Message Parsing:      >{} msg/s",
        format_number(metrics.parsing_rate)
    );
    println!(
        "   InstrumentId ops:     >{} ops/s",
        format_number(metrics.instrument_ops_rate)
    );
    println!();

    println!("⏱️  Latency Targets:");
    println!("   Hot path processing:  <{}μs", metrics.target_latency_us);
    println!("   Memory per service:   <{}MB", metrics.memory_target_mb);
    println!();

    println!("📊 Size Constraint Performance:");
    println!("   ✅ Fixed-size TLVs:     Fastest parsing (no bounds checking)");
    println!("      Examples: Trade (37 bytes), Economics (32 bytes)");
    println!("      Use case: High-frequency trading, real-time feeds");
    println!();
    println!("   ⚠️  Bounded-size TLVs:   Single bounds check required");
    println!("      Examples: PoolSwap (60-200 bytes), SignalIdentity (32-128 bytes)");
    println!("      Use case: Medium-frequency events, configuration");
    println!();
    println!("   🐌 Variable-size TLVs:   Dynamic allocation required");
    println!("      Examples: OrderBook (unlimited), ComplexSignal (variable)");
    println!("      Use case: Batch processing, less time-sensitive operations");
    println!();

    println!("🔧 Optimization Techniques:");
    println!("   ✅ Use zerocopy traits (as_bytes() not to_bytes())");
    println!("   ✅ Batch process variable-size messages when possible");
    println!("   ✅ Cache InstrumentId lookups (bijective conversion is fast)");
    println!("   ✅ Pre-allocate buffers for message construction");
    println!("   ⚠️  Profile actual usage - micro-optimizations vary by workload");
    println!();

    println!("🎯 Architecture Guidelines:");
    println!("   • Hot paths: Use only fixed-size TLVs where possible");
    println!("   • Warm paths: Bounded-size TLVs acceptable");
    println!("   • Cold paths: Variable-size TLVs okay for flexibility");
    println!("   • Always measure: Performance characteristics depend on data patterns");
}

/// Comprehensive help display
///
/// One-stop function that shows all available help information in a logical order.
/// Use this for complete API reference or when exploring the system for the first time.
///
/// # Usage
///
/// ```rust
/// # use torq_protocol_v2::help::show_all_help;
/// show_all_help();
/// ```
///
/// # Output Organization
///
/// 1. InstrumentId API (most common starting point)
/// 2. TLVType API (core message system)
/// 3. Routing domains (system architecture)
/// 4. Common mistakes (error prevention)
/// 5. Performance tips (optimization guidance)
/// 6. Quick reference commands
pub fn show_all_help() {
    println!("=== Torq Protocol V2 - Complete API Reference ===");
    println!();

    show_instrument_id_methods();
    println!("\n{}", "=".repeat(60));
    println!();

    show_tlv_type_methods();
    println!("\n{}", "=".repeat(60));
    println!();

    show_relay_domains();
    println!("\n{}", "=".repeat(60));
    println!();

    show_common_mistakes();
    println!("\n{}", "=".repeat(60));
    println!();

    show_performance_tips();
    println!("\n{}", "=".repeat(60));
    println!();

    println!("=== Quick Commands ===");
    println!("🚀 Development:");
    println!("   cargo docs                     - Open full HTML documentation");
    println!("   cargo run --example api_discovery - Interactive type explorer");
    println!("   cargo bench                    - Run performance benchmarks");
    println!();
    println!("📚 Documentation:");
    println!("   cargo api-help                 - This help system");
    println!("   cargo generate-docs            - Update markdown documentation");
    println!("   cargo doc --open               - Open rustdoc in browser");
    println!();
    println!("🔍 Discovery:");
    println!("   show_instrument_id_methods()   - InstrumentId API");
    println!("   show_tlv_type_methods()        - TLV type system");
    println!("   explore_tlv_type(TLVType::Trade) - Deep dive specific type");
    println!("   show_performance_tips()        - Optimization guidance");
    println!();
    println!("💡 The help system is designed for interactive use in REPL or debugging sessions.");
    println!("   All functions are zero-cost and safe to call during development.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_method_info_creation() {
        let method =
            MethodInfo::available("test_method()", "Test description", Some("test_example()"))
                .with_performance("Fast");

        assert_eq!(method.signature, "test_method()");
        assert_eq!(method.description, "Test description");
        assert_eq!(method.example, Some("test_example()"));
        assert_eq!(method.status, MethodStatus::Available);
        assert_eq!(method.performance_notes, Some("Fast"));
    }

    #[test]
    fn test_performance_metrics_default() {
        let metrics = PerformanceMetrics::default();
        assert!(metrics.construction_rate >= 1_000_000);
        assert!(metrics.parsing_rate >= 1_600_000);
        assert!(metrics.instrument_ops_rate >= 19_000_000);
        assert_eq!(metrics.target_latency_us, 35);
        assert_eq!(metrics.memory_target_mb, 50);
    }

    /// Test that help functions don't panic (smoke tests)
    #[test]
    fn test_help_functions_dont_panic() {
        // These should never panic - they're developer tools
        show_instrument_id_methods();
        show_tlv_type_methods();
        show_relay_domains();
        show_common_mistakes();
        show_performance_tips();

        // Test with actual TLV type
        explore_tlv_type(TLVType::Trade);
    }

    /// Test that all method statuses display correctly
    #[test]
    fn test_method_status_display() {
        use std::fmt::Write;
        let mut output = String::new();

        write!(&mut output, "{}", MethodStatus::Available).unwrap();
        assert_eq!(output, "✅");

        output.clear();
        write!(&mut output, "{}", MethodStatus::NotAvailable).unwrap();
        assert_eq!(output, "❌");

        output.clear();
        write!(&mut output, "{}", MethodStatus::Deprecated).unwrap();
        assert_eq!(output, "⚠️");

        output.clear();
        write!(&mut output, "{}", MethodStatus::Planned).unwrap();
        assert_eq!(output, "🔄");
    }
}

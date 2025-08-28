<!-- GENERATED FROM protocol/tlv_type_system.org - DO NOT EDIT DIRECTLY -->



# Overview

The TLV Type System is the central nervous system of Torq's Protocol V2 message architecture. This comprehensive type registry and introspection system provides domain-based organization with automatic routing, size validation, and rich developer APIs for discovery and documentation generation.

**Key Capabilities**:

-   Domain-based numeric organization (MarketData 1-19, Signal 20-39, Execution 40-79, System 100-119)
-   Automatic message routing based on type number
-   Zero-cost validation for fixed-size types
-   Rich introspection API for development tools
-   Documentation auto-generation from type metadata
-   IDE integration with rust-analyzer tooltips


# Architecture Role

The TLV Type System serves as the bridge between developer tools and protocol implementation:

    Developer Tools → [TLV Type Registry] → Protocol Implementation
          ↑                ↓                        ↓
      IDE Help        Type Metadata           Message Routing
      Code Gen        Size Validation         Service Discovery
      Docs Gen        Domain Mapping          Format Selection


## Integration Points

<table border="2" cellspacing="0" cellpadding="6" rules="groups" frame="hsides">


<colgroup>
<col  class="org-left" />

<col  class="org-left" />

<col  class="org-left" />
</colgroup>
<thead>
<tr>
<th scope="col" class="org-left">Component</th>
<th scope="col" class="org-left">Integration</th>
<th scope="col" class="org-left">Purpose</th>
</tr>
</thead>
<tbody>
<tr>
<td class="org-left">TLVMessageBuilder</td>
<td class="org-left">Type metadata lookup</td>
<td class="org-left">Format selection and validation</td>
</tr>

<tr>
<td class="org-left">Protocol Parser</td>
<td class="org-left">Size constraint checking</td>
<td class="org-left">Payload validation during parsing</td>
</tr>

<tr>
<td class="org-left">Relay Router</td>
<td class="org-left">Domain mapping</td>
<td class="org-left">Automatic routing to appropriate services</td>
</tr>

<tr>
<td class="org-left">Documentation</td>
<td class="org-left">Type enumeration</td>
<td class="org-left">Auto-generation of API references</td>
</tr>

<tr>
<td class="org-left">IDE Tools</td>
<td class="org-left">Rich introspection</td>
<td class="org-left">IntelliSense and code completion</td>
</tr>

<tr>
<td class="org-left">Service Discovery</td>
<td class="org-left">Runtime enumeration</td>
<td class="org-left">Dynamic service capability detection</td>
</tr>
</tbody>
</table>


# Type Organization Strategy


## Domain-Based Numeric Ranges

<table border="2" cellspacing="0" cellpadding="6" rules="groups" frame="hsides">


<colgroup>
<col  class="org-right" />

<col  class="org-left" />

<col  class="org-left" />

<col  class="org-left" />
</colgroup>
<thead>
<tr>
<th scope="col" class="org-right">Range</th>
<th scope="col" class="org-left">Domain</th>
<th scope="col" class="org-left">Relay Target</th>
<th scope="col" class="org-left">Characteristics</th>
</tr>
</thead>
<tbody>
<tr>
<td class="org-right">1-19</td>
<td class="org-left">MarketData</td>
<td class="org-left">MarketDataRelay</td>
<td class="org-left">High-frequency price/volume data</td>
</tr>

<tr>
<td class="org-right">20-39</td>
<td class="org-left">Strategy Signals</td>
<td class="org-left">SignalRelay</td>
<td class="org-left">Trading logic coordination</td>
</tr>

<tr>
<td class="org-right">40-59</td>
<td class="org-left">Execution</td>
<td class="org-left">ExecutionRelay</td>
<td class="org-left">Order lifecycle management</td>
</tr>

<tr>
<td class="org-right">60-79</td>
<td class="org-left">Portfolio/Risk</td>
<td class="org-left">SignalRelay</td>
<td class="org-left">Risk monitoring analytics</td>
</tr>

<tr>
<td class="org-right">80-99</td>
<td class="org-left">Compliance/Audit</td>
<td class="org-left">SystemRelay</td>
<td class="org-left">Regulatory tracking</td>
</tr>

<tr>
<td class="org-right">100-119</td>
<td class="org-left">System</td>
<td class="org-left">SystemRelay</td>
<td class="org-left">Infrastructure messaging</td>
</tr>

<tr>
<td class="org-right">200-254</td>
<td class="org-left">Vendor</td>
<td class="org-left">ConfigurableRelay</td>
<td class="org-left">Custom/experimental types</td>
</tr>

<tr>
<td class="org-right">255</td>
<td class="org-left">Extended</td>
<td class="org-left">Any domain</td>
<td class="org-left">Large payload marker</td>
</tr>
</tbody>
</table>


## Size Constraint Strategy


### Fixed Size Types

Zero validation overhead - size known at compile time:

-   `Trade` (40 bytes): Critical hot path trading data
-   `Economics` (32 bytes): Pool economics snapshot
-   `SignalIdentity` (32 bytes): Signal routing information


### Bounded Size Types

Single bounds check required:

-   `SwapEvent` (60-200 bytes): Variable addresses in pool events
-   `OrderStatus` (64-256 bytes): Variable status text
-   `PoolLiquidity` (60-180 bytes): Variable token pair data


### Variable Size Types

Dynamic allocation required - use sparingly in hot paths:

-   `OrderBook` (100-64KB): Full market depth
-   `PositionSnapshot` (200-10KB): Portfolio state
-   `ComplianceReport` (500-50KB): Regulatory data


# Performance Profile


## Type Lookup Performance

<table border="2" cellspacing="0" cellpadding="6" rules="groups" frame="hsides">


<colgroup>
<col  class="org-left" />

<col  class="org-left" />

<col  class="org-left" />
</colgroup>
<thead>
<tr>
<th scope="col" class="org-left">Operation</th>
<th scope="col" class="org-left">Time</th>
<th scope="col" class="org-left">Method</th>
</tr>
</thead>
<tbody>
<tr>
<td class="org-left">Type Lookup</td>
<td class="org-left">O(1)</td>
<td class="org-left">Enum-to-integer compiler optimization</td>
</tr>

<tr>
<td class="org-left">Domain Mapping</td>
<td class="org-left">Compile-time</td>
<td class="org-left">Constant folding for range checks</td>
</tr>

<tr>
<td class="org-left">Fixed Size Validation</td>
<td class="org-left">0ns</td>
<td class="org-left">Zero-cost - compile-time known</td>
</tr>

<tr>
<td class="org-left">Bounded Size Validation</td>
<td class="org-left">&lt;5ns</td>
<td class="org-left">Single branch + bounds check</td>
</tr>

<tr>
<td class="org-left">Introspection Query</td>
<td class="org-left">&lt;1μs</td>
<td class="org-left">Development/debugging only</td>
</tr>

<tr>
<td class="org-left">Memory Overhead</td>
<td class="org-left">2KB</td>
<td class="org-left">Static string tables + enum metadata</td>
</tr>
</tbody>
</table>


## Runtime Impact

-   **Hot Path**: Zero overhead - all metadata resolved at compile time
-   **Development**: Rich introspection available for tooling
-   **Memory**: Minimal static data structure footprint


# Core Data Structures


## TLVTypeInfo Structure

Complete metadata for type introspection and development tools:

    #[derive(Debug, Clone)]
    pub struct TLVTypeInfo {
        /// TLV type number (1-255) for wire protocol identification
        pub type_number: u8,
        /// Human-readable name for development tools and logging
        pub name: &'static str,
        /// Detailed description of message purpose and content structure
        pub description: &'static str,
        /// Relay domain for automatic message routing
        pub relay_domain: RelayDomain,
        /// Size validation constraint for parsing safety
        pub size_constraint: TLVSizeConstraint,
        /// Current implementation and availability status
        pub status: TLVImplementationStatus,
        /// Real-world usage examples and integration patterns
        pub examples: Vec<&'static str>,
    }


## Implementation Status Lifecycle


### Production States

1.  **Implemented**: Production-ready with zero-copy serialization
    -   Full zerocopy traits implementation
    -   Comprehensive test coverage
    -   Performance benchmarks
    -   Stable API surface

2.  **Reserved**: Type number allocated, implementation pending
    -   Number reserved to prevent conflicts
    -   Used for planning future extensions
    -   Safe to reference but compile error if used

3.  **Vendor**: Available for custom/experimental functionality
    -   Type numbers 200-254 available
    -   No standard protocol definition
    -   Vendor responsibility for implementation

4.  **Extended**: Special marker type for large payloads
    -   Type 255 reserved for variable-size extension
    -   Enables future protocol evolution


# Developer API Examples


## Basic Type Discovery

    use torq_protocol_v2::tlv::TLVType;
    use torq_protocol_v2::RelayDomain;
    
    // Get comprehensive type information
    let info = TLVType::Trade.type_info();
    println!("Type {}: {} - {}", info.type_number, info.name, info.description);
    println!("Routes to: {:?}, Size: {:?}", info.relay_domain, info.size_constraint);
    
    // Query types by relay domain for service logic
    let market_types = TLVType::types_in_domain(RelayDomain::MarketData);
    println!("Market data relay handles {} message types", market_types.len());
    
    // Development workflow
    println!("Trade type implemented: {}", TLVType::Trade.is_implemented());


## Documentation Generation

    // Auto-generate complete API documentation
    let markdown = TLVType::generate_markdown_table();
    std::fs::write("docs/message-types.md", markdown)?;
    println!("Generated documentation for {} types", TLVType::all_implemented().len());
    
    // Generate relay-specific documentation
    for domain in [RelayDomain::MarketData, RelayDomain::Signal, RelayDomain::Execution] {
        let types = TLVType::types_in_domain(domain);
        let doc = TLVType::generate_domain_docs(domain, &types);
        std::fs::write(format!("docs/{:?}-types.md", domain), doc)?;
    }


## Runtime Message Handling

    use torq_protocol_v2::tlv::{TLVType, TLVSizeConstraint};
    
    // Size validation during parsing
    let tlv_type = TLVType::try_from(message_type)?;
    match tlv_type.size_constraint() {
        TLVSizeConstraint::Fixed(expected) => {
            // Hot path: no validation needed for fixed types like Trade
            assert_eq!(payload.len(), expected);
        },
        TLVSizeConstraint::Bounded { min, max } => {
            // Bounded types: single validation for pool events
            if payload.len() < min || payload.len() > max {
                return Err(ParseError::InvalidSize);
            }
        },
        TLVSizeConstraint::Variable => {
            // Variable types: accept any size for order books
        }
    }


## Service Integration

    // Relay service automatically routes based on type number
    let relay_domain = TLVType::PoolSwap.relay_domain();
    match relay_domain {
        RelayDomain::MarketData => send_to_market_relay(message),
        RelayDomain::Signal => send_to_signal_relay(message),
        RelayDomain::Execution => send_to_execution_relay(message),
        RelayDomain::System => send_to_system_relay(message),
    }
    
    // Service capability discovery
    let available_types: Vec<TLVType> = relay_service.supported_types();
    println!("Service supports {} message types", available_types.len());


# Implementation Guidelines


## Adding New TLV Types

1.  **Reserve Type Number**: Update enum with `Reserved` status
2.  **Choose Domain**: Select appropriate numeric range
3.  **Define Size Constraint**: Fixed > Bounded > Variable preference
4.  **Implement Zero-Copy Traits**: `AsBytes` and `FromBytes`
5.  **Add Comprehensive Tests**: Unit, integration, and property tests
6.  **Update Status**: Change from `Reserved` to `Implemented`
7.  **Document Examples**: Add usage patterns to type metadata


## Performance Considerations


### Hot Path Optimization

-   Prefer fixed-size types for frequent messages
-   Use bounded types only when necessary
-   Avoid variable types in high-frequency scenarios
-   Cache type metadata lookups where possible


### Memory Efficiency

-   String tables are static - no runtime allocation
-   Enum discriminants fit in single byte
-   Metadata accessed only during development/introspection


## Breaking Change Policy


### Never Allowed

-   Reusing type numbers (even for deleted types)
-   Changing size constraints for existing types
-   Modifying domain assignments for implemented types


### Coordinated Changes

-   Renaming types (update all references)
-   Adding new fields to `TLVTypeInfo`
-   Changing status from `Implemented` to `Reserved`


# Integration Testing


## Type Registry Validation

    #[test]
    fn test_type_number_uniqueness() {
        let mut seen = std::collections::HashSet::new();
        for tlv_type in TLVType::all_types() {
            let number = tlv_type.type_number();
            assert!(!seen.contains(&number), "Duplicate type number: {}", number);
            seen.insert(number);
        }
    }
    
    #[test]
    fn test_domain_range_compliance() {
        for tlv_type in TLVType::all_types() {
            let info = tlv_type.type_info();
            match info.relay_domain {
                RelayDomain::MarketData => assert!((1..=19).contains(&info.type_number)),
                RelayDomain::Signal => assert!((20..=39).contains(&info.type_number)),
                RelayDomain::Execution => assert!((40..=59).contains(&info.type_number)),
                RelayDomain::System => assert!((100..=119).contains(&info.type_number)),
            }
        }
    }


## Performance Benchmarks

    #[bench]
    fn bench_type_lookup_performance(b: &mut Bencher) {
        b.iter(|| {
            for type_num in 1u8..=50u8 {
                black_box(TLVType::try_from(type_num));
            }
        });
    }
    
    #[bench]
    fn bench_domain_classification(b: &mut Bencher) {
        let types = TLVType::all_implemented();
        b.iter(|| {
            for tlv_type in &types {
                black_box(tlv_type.relay_domain());
            }
        });
    }


# Maintenance Procedures


## Regular Reviews

1.  **Type Number Conflicts**: Verify no duplicates in enum
2.  **Implementation Status**: Update `Reserved` → `Implemented` as development completes
3.  **Documentation Sync**: Ensure examples match current API
4.  **Performance Metrics**: Monitor benchmark results for regressions


## Migration Support


### Legacy Protocol Support

-   Maintain compatibility layer for Protocol V1 types
-   Provide translation functions for gradual migration
-   Document migration path for each legacy type


### API Evolution

-   Version type metadata schema for future extensions
-   Maintain backward compatibility in introspection API
-   Coordinate breaking changes across all consumers

This comprehensive type system forms the foundation of Torq's high-performance message processing architecture, enabling both runtime efficiency and rich development experience.


#!/usr/bin/env cargo
//! Test the new TLVType developer API

use torq_types::tlv::TLVType;
use torq_types::RelayDomain;

fn main() {
    println!("Testing TLVType Developer API\n");

    // Test type_info() method
    println!("=== Type Information ===");
    let trade_info = TLVType::Trade.type_info();
    println!("Type {}: {}", trade_info.type_number, trade_info.name);
    println!("  Description: {}", trade_info.description);
    println!("  Size: {:?}", trade_info.size_constraint);
    println!("  Routes to: {:?}", trade_info.relay_domain);
    println!("  Status: {:?}", trade_info.status);
    println!("  Examples: {:?}", trade_info.examples);

    println!("\n=== Pool Swap Information ===");
    let pool_swap_info = TLVType::PoolSwap.type_info();
    println!(
        "Type {}: {}",
        pool_swap_info.type_number, pool_swap_info.name
    );
    println!("  Description: {}", pool_swap_info.description);
    println!("  Size: {:?}", pool_swap_info.size_constraint);
    println!("  Routes to: {:?}", pool_swap_info.relay_domain);
    println!("  Examples: {:?}", pool_swap_info.examples);

    // Test domain queries
    println!("\n=== Types by Domain ===");
    let market_types = TLVType::types_in_domain(RelayDomain::MarketData);
    println!("Market Data domain has {} types:", market_types.len());
    for tlv_type in market_types.iter().take(5) {
        // Show first 5
        println!("  Type {}: {}", *tlv_type as u8, tlv_type.name());
    }

    let signal_types = TLVType::types_in_domain(RelayDomain::Signal);
    println!("\nSignal domain has {} types:", signal_types.len());
    for tlv_type in signal_types.iter().take(5) {
        // Show first 5
        println!("  Type {}: {}", *tlv_type as u8, tlv_type.name());
    }

    let execution_types = TLVType::types_in_domain(RelayDomain::Execution);
    println!("\nExecution domain has {} types:", execution_types.len());
    for tlv_type in execution_types.iter().take(5) {
        // Show first 5
        println!("  Type {}: {}", *tlv_type as u8, tlv_type.name());
    }

    // Test implementation status
    println!("\n=== Implementation Status ===");
    let all_types = TLVType::all_implemented();
    println!("Total implemented types: {}", all_types.len());

    let implemented_count = all_types.iter().filter(|t| t.is_implemented()).count();
    println!("Fully implemented: {}", implemented_count);

    // Test name and description methods
    println!("\n=== Key Types Summary ===");
    let key_types = vec![
        TLVType::Trade,
        TLVType::PoolSwap,
        TLVType::SignalIdentity,
        TLVType::Economics,
        TLVType::OrderRequest,
        TLVType::Fill,
    ];

    for tlv_type in key_types {
        println!(
            "Type {}: {} - {}",
            tlv_type as u8,
            tlv_type.name(),
            tlv_type.description()
        );
    }

    println!("\nDeveloper API test complete!");
}

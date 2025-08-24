#!/usr/bin/env rust-script
//! Test that our configuration now includes both V2 and V3 swap signatures

use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Checking polygon.toml configuration...\n");

    // Read the TOML file
    let config_content = fs::read_to_string("polygon.toml")?;
    println!("Config file contents:");
    println!("{}", config_content);

    // Check for both swap signatures
    let v2_swap = "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822";
    let v3_swap = "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67";

    println!("\n🔍 Signature Analysis:");
    println!("V2 Swap signature: {}", v2_swap);
    println!("V3 Swap signature: {}", v3_swap);

    if config_content.contains(v2_swap) {
        println!("✅ V2 swap signature found in config");
    } else {
        println!("❌ V2 swap signature MISSING from config");
    }

    if config_content.contains(v3_swap) {
        println!("✅ V3 swap signature found in config");
    } else {
        println!("❌ V3 swap signature MISSING from config");
    }

    Ok(())
}

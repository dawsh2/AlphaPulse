#!/usr/bin/env cargo
//! Generate documentation from TLV types

use protocol_v2::tlv::TLVType;
use std::fs;

fn main() {
    println!("Generating TLV type documentation...");

    // Generate markdown content
    let markdown = TLVType::generate_markdown_table();

    // Write to backend_v2 docs directory
    let docs_path = "../docs/message-types-auto.md";
    match fs::write(docs_path, &markdown) {
        Ok(_) => {
            println!("Generated {} at {}", docs_path, docs_path);
            println!(
                "Documentation includes {} types",
                TLVType::all_implemented().len()
            );
        }
        Err(e) => {
            eprintln!("Failed to write {}: {}", docs_path, e);
            println!("Here's the generated content:\n");
            println!("{}", markdown);
        }
    }
}

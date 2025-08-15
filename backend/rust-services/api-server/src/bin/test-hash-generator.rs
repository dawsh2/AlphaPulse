use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

fn generate_hash(canonical: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    canonical.hash(&mut hasher);
    hasher.finish()
}

fn main() {
    // Generate hashes for all known symbols
    let symbols = vec![
        // Coinbase crypto pairs
        ("coinbase:BTC-USD", "BTC-USD"),
        ("coinbase:ETH-USD", "ETH-USD"),
        ("coinbase:SOL-USD", "SOL-USD"),
        ("coinbase:LINK-USD", "LINK-USD"),
        ("coinbase:AVAX-USD", "AVAX-USD"),
        ("coinbase:MATIC-USD", "MATIC-USD"),
        ("coinbase:ADA-USD", "ADA-USD"),
        ("coinbase:DOT-USD", "DOT-USD"),
        
        // Additional pairs that might be used
        ("coinbase:BTC-USDT", "BTC-USDT"),
        ("coinbase:ETH-USDT", "ETH-USDT"),
        
        // Potential stock symbols from Alpaca
        ("alpaca:AAPL", "AAPL"),
        ("alpaca:GOOGL", "GOOGL"),
        ("alpaca:MSFT", "MSFT"),
        ("alpaca:TSLA", "TSLA"),
        ("alpaca:NVDA", "NVDA"),
        ("alpaca:META", "META"),
        ("alpaca:AMD", "AMD"),
        ("alpaca:SPY", "SPY"),
        ("alpaca:QQQ", "QQQ"),
        ("alpaca:AMZN", "AMZN"),
    ];
    
    println!("// Generated symbol hash mappings");
    println!("// Copy these to frontend/src/dashboard/utils/symbolHash.ts");
    println!();
    println!("const HASH_TO_SYMBOL: Record<string, string> = {{");
    
    for (canonical, display_name) in &symbols {
        let hash = generate_hash(canonical);
        println!("  '{}': '{}', // {}", hash, display_name, canonical);
    }
    
    println!("}};");
    println!();
    println!("// For reference - canonical to hash mapping:");
    println!("const CANONICAL_TO_HASH: Record<string, string> = {{");
    
    for (canonical, _) in &symbols {
        let hash = generate_hash(canonical);
        println!("  '{}': '{}',", canonical, hash);
    }
    
    println!("}};");
}
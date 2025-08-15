use alphapulse_protocol::SymbolDescriptor;

fn main() {
    // Generate hashes for all known instruments using the EXACT same code
    let mut instruments = vec![
        // Coinbase crypto pairs
        (SymbolDescriptor::spot("coinbase", "BTC", "USD"), "coinbase:BTC-USD"),
        (SymbolDescriptor::spot("coinbase", "ETH", "USD"), "coinbase:ETH-USD"),
        (SymbolDescriptor::spot("coinbase", "SOL", "USD"), "coinbase:SOL-USD"),
        (SymbolDescriptor::spot("coinbase", "LINK", "USD"), "coinbase:LINK-USD"),
        (SymbolDescriptor::spot("coinbase", "AVAX", "USD"), "coinbase:AVAX-USD"),
        (SymbolDescriptor::spot("coinbase", "MATIC", "USD"), "coinbase:MATIC-USD"),
        (SymbolDescriptor::spot("coinbase", "ADA", "USD"), "coinbase:ADA-USD"),
        (SymbolDescriptor::spot("coinbase", "DOT", "USD"), "coinbase:DOT-USD"),
        
        // Kraken pairs
        (SymbolDescriptor::spot("kraken", "BTC", "USD"), "kraken:BTC-USD"),
        (SymbolDescriptor::spot("kraken", "ETH", "USD"), "kraken:ETH-USD"),
        
        // Alpaca stocks
        (SymbolDescriptor::stock("alpaca", "AAPL"), "alpaca:AAPL"),
        (SymbolDescriptor::stock("alpaca", "GOOGL"), "alpaca:GOOGL"),
        (SymbolDescriptor::stock("alpaca", "MSFT"), "alpaca:MSFT"),
        (SymbolDescriptor::stock("alpaca", "TSLA"), "alpaca:TSLA"),
        (SymbolDescriptor::stock("alpaca", "NVDA"), "alpaca:NVDA"),
        (SymbolDescriptor::stock("alpaca", "META"), "alpaca:META"),
        (SymbolDescriptor::stock("alpaca", "AMD"), "alpaca:AMD"),
        (SymbolDescriptor::stock("alpaca", "SPY"), "alpaca:SPY"),
        (SymbolDescriptor::stock("alpaca", "QQQ"), "alpaca:QQQ"),
        (SymbolDescriptor::stock("alpaca", "AMZN"), "alpaca:AMZN"),
    ];
    
    // Add Polygon DEX pairs
    let dex_tokens = vec![
        ("WMATIC", "USDC"), ("WETH", "USDC"), ("WBTC", "USDC"),
        ("DAI", "USDC"), ("LINK", "USDC"), ("AAVE", "USDC"),
        ("USDC", "USDT"), ("DAI", "USDT"), ("WETH", "USDT"),
        ("WMATIC", "WETH"), ("WETH", "WBTC"), ("WMATIC", "DAI"),
    ];
    
    for dex in &["quickswap", "sushiswap", "uniswap_v3"] {
        for &(base, quote) in &dex_tokens {
            let descriptor = SymbolDescriptor::spot(*dex, base, quote);
            let display = format!("{}:{}-{}", dex, base, quote);
            instruments.push((descriptor, display.leak())); // leak for static lifetime
        }
    }
    
    println!("// Generated instrument hash mappings using EXACT Rust protocol code");
    println!("// Copy these to frontend/src/dashboard/utils/instrumentHash.ts");
    println!("// Total instruments: {}", instruments.len());
    println!();
    println!("export const HASH_TO_INSTRUMENT: Record<string, string> = {{");
    
    for (descriptor, display_name) in &instruments {
        let hash = descriptor.hash();
        let canonical = descriptor.to_string();
        println!("  '{}': '{}', // {}", hash, display_name, canonical);
    }
    
    println!("}};");
    println!();
    
    // Also generate exchange mapping
    println!("// Map hash to exchange for Data Flow Monitor");
    println!("export const HASH_TO_EXCHANGE: Record<string, string> = {{");
    
    for (descriptor, _) in &instruments {
        let hash = descriptor.hash();
        println!("  '{}': '{}',", hash, descriptor.exchange);
    }
    
    println!("}};");
    
    println!();
    println!("// Reverse mapping for sending commands from frontend");
    println!("export const INSTRUMENT_TO_HASH: Record<string, string> = {{");
    
    for (descriptor, display_name) in &instruments {
        let hash = descriptor.hash();
        println!("  '{}': '{}',", display_name, hash);
    }
    
    println!("}};");
}
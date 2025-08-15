use alphapulse_protocol::{SymbolDescriptor, SymbolMappingMessage};
use anyhow::Result;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info};
use serde::{Deserialize, Serialize};

/// Token configuration with chain-specific decimal properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenConfig {
    pub symbol: String,
    pub decimals: u8,          // Token decimal places (e.g., 6 for USDC, 18 for WETH)
    pub chain: String,
    pub contract_address: Option<String>,
    pub is_stablecoin: bool,
    pub display_decimals: u8,  // How many decimals to show in UI
}

impl TokenConfig {
    pub fn new(symbol: &str, decimals: u8, chain: &str, contract_address: Option<&str>, is_stablecoin: bool, display_decimals: u8) -> Self {
        Self {
            symbol: symbol.to_string(),
            decimals,
            chain: chain.to_string(),
            contract_address: contract_address.map(|s| s.to_string()),
            is_stablecoin,
            display_decimals,
        }
    }
    
    /// Convert human amount to raw token units (e.g., "1.5" USDC → 1500000)
    pub fn to_raw_amount(&self, human_amount: &str) -> Result<u64, anyhow::Error> {
        use rust_decimal::Decimal;
        use rust_decimal::prelude::ToPrimitive;
        use std::str::FromStr;
        
        let decimal = Decimal::from_str(human_amount)?;
        let multiplier = Decimal::from(10u64.pow(self.decimals as u32));
        let raw_decimal = decimal * multiplier;
        
        raw_decimal.to_u64()
            .ok_or_else(|| anyhow::anyhow!("Amount too large: {}", human_amount))
    }
    
    /// Convert raw token units to human amount (e.g., 1500000 → "1.5")
    pub fn to_human_amount(&self, raw_amount: u64) -> String {
        let divisor = 10u64.pow(self.decimals as u32);
        let whole = raw_amount / divisor;
        let fractional = raw_amount % divisor;
        
        if fractional == 0 {
            format!("{}.{:0width$}", whole, 0, width = self.decimals as usize)
        } else {
            format!("{}.{:0width$}", whole, fractional, width = self.decimals as usize)
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string()
        }
    }
}

/// Centralized instrument management for consistent symbol hashing and mapping
/// This ensures all exchanges use the same symbol hashing logic and provides
/// human-readable mappings for display in the frontend
pub struct InstrumentRegistry {
    // Maps symbol hash to human-readable string
    hash_to_symbol: Arc<RwLock<HashMap<u64, String>>>,
    // Maps exchange:symbol to hash for quick lookups
    symbol_to_hash: Arc<RwLock<HashMap<String, u64>>>,
    // Maps hash to SymbolDescriptor for reconstruction
    hash_to_descriptor: Arc<RwLock<HashMap<u64, SymbolDescriptor>>>,
    // Maps chain:symbol to token configuration
    token_configs: Arc<RwLock<HashMap<String, TokenConfig>>>,
}

impl InstrumentRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            hash_to_symbol: Arc::new(RwLock::new(HashMap::new())),
            symbol_to_hash: Arc::new(RwLock::new(HashMap::new())),
            hash_to_descriptor: Arc::new(RwLock::new(HashMap::new())),
            token_configs: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Pre-register known instruments and tokens
        registry.register_known_instruments();
        registry.register_known_tokens();
        registry
    }
    
    /// Register an instrument and return its hash
    pub fn register(&self, descriptor: SymbolDescriptor) -> u64 {
        let hash = descriptor.hash();
        let canonical = descriptor.to_string();
        let display_name = self.format_display_name(&descriptor);
        
        // Update all mappings
        {
            let mut h2s = self.hash_to_symbol.write();
            h2s.insert(hash, display_name.clone());
        }
        
        {
            let mut s2h = self.symbol_to_hash.write();
            s2h.insert(canonical.clone(), hash);
            // Also store with display name for convenience
            s2h.insert(display_name.clone(), hash);
        }
        
        {
            let mut h2d = self.hash_to_descriptor.write();
            h2d.insert(hash, descriptor);
        }
        
        debug!("Registered instrument: {} -> {} (hash: {})", canonical, display_name, hash);
        hash
    }
    
    /// Get or create hash for a symbol
    pub fn get_or_create_hash(&self, exchange: &str, symbol: &str) -> u64 {
        // Check if already registered
        let lookup_key = format!("{}:{}", exchange, symbol);
        {
            let s2h = self.symbol_to_hash.read();
            if let Some(&hash) = s2h.get(&lookup_key) {
                return hash;
            }
        }
        
        // Parse and create new descriptor
        let descriptor = self.parse_symbol(exchange, symbol);
        self.register(descriptor)
    }
    
    /// Get human-readable display name from hash
    pub fn get_display_name(&self, hash: u64) -> Option<String> {
        self.hash_to_symbol.read().get(&hash).cloned()
    }
    
    /// Get SymbolDescriptor from hash
    pub fn get_descriptor(&self, hash: u64) -> Option<SymbolDescriptor> {
        self.hash_to_descriptor.read().get(&hash).cloned()
    }
    
    /// Create SymbolMappingMessage for sending to relay
    pub fn create_mapping_message(&self, hash: u64) -> Option<SymbolMappingMessage> {
        self.get_descriptor(hash).map(|desc| SymbolMappingMessage::new(&desc))
    }
    
    /// Parse exchange-specific symbol format into SymbolDescriptor
    fn parse_symbol(&self, exchange: &str, symbol: &str) -> SymbolDescriptor {
        match exchange {
            // Crypto exchanges use spot pairs
            "coinbase" | "kraken" | "binance" => {
                if symbol.contains('-') || symbol.contains('/') {
                    let parts: Vec<&str> = symbol.split(|c| c == '-' || c == '/').collect();
                    if parts.len() == 2 {
                        return SymbolDescriptor::spot(exchange, parts[0], parts[1]);
                    }
                }
                // Fallback
                SymbolDescriptor::spot(exchange, symbol, "USD")
            }
            
            // DEX exchanges with token pairs
            "quickswap" | "sushiswap" | "uniswap_v3" => {
                if symbol.contains('-') {
                    let parts: Vec<&str> = symbol.split('-').collect();
                    if parts.len() == 2 {
                        return SymbolDescriptor::spot(exchange, parts[0], parts[1]);
                    }
                }
                // Fallback to treating as single token vs USDC
                SymbolDescriptor::spot(exchange, symbol, "USDC")
            }
            
            // Traditional markets use stock symbols
            "alpaca" | "ibkr" => {
                // Check if it's an option (contains expiry info)
                if symbol.len() > 15 && symbol.chars().any(|c| c.is_numeric()) {
                    // Parse option format: AAPL20250117C600
                    // This is simplified - real parsing would be more complex
                    SymbolDescriptor::stock(exchange, symbol)
                } else {
                    SymbolDescriptor::stock(exchange, symbol)
                }
            }
            
            _ => {
                // Unknown exchange - treat as spot pair
                if symbol.contains('-') {
                    let parts: Vec<&str> = symbol.split('-').collect();
                    if parts.len() == 2 {
                        return SymbolDescriptor::spot(exchange, parts[0], parts[1]);
                    }
                }
                SymbolDescriptor::stock(exchange, symbol)
            }
        }
    }
    
    /// Format human-readable display name
    fn format_display_name(&self, descriptor: &SymbolDescriptor) -> String {
        match &descriptor.quote {
            Some(quote) => {
                // Spot/Forex style: BTC-USD
                format!("{}-{}", descriptor.base, quote)
            }
            None => {
                // Stock/Future style
                if let Some(expiry) = descriptor.expiry {
                    if let Some(strike) = descriptor.strike {
                        if let Some(opt_type) = descriptor.option_type {
                            // Option: AAPL 01/17/25 600C
                            let year = expiry / 10000;
                            let month = (expiry / 100) % 100;
                            let day = expiry % 100;
                            format!("{} {:02}/{:02}/{:02} {:.0}{}", 
                                    descriptor.base, month, day, year % 100, strike, opt_type)
                        } else {
                            // Future
                            format!("{}_{}", descriptor.base, expiry)
                        }
                    } else {
                        // Future without strike
                        format!("{}_{}", descriptor.base, expiry)
                    }
                } else {
                    // Plain stock
                    descriptor.base.clone()
                }
            }
        }
    }
    
    /// Register all known instruments at startup
    fn register_known_instruments(&mut self) {
        info!("Registering known instruments");
        
        // Coinbase crypto pairs
        let coinbase_pairs = vec![
            ("BTC", "USD"), ("ETH", "USD"), ("SOL", "USD"), 
            ("LINK", "USD"), ("AVAX", "USD"), ("MATIC", "USD"),
            ("ADA", "USD"), ("DOT", "USD"), ("ATOM", "USD"),
            ("UNI", "USD"), ("AAVE", "USD"), ("LTC", "USD"),
        ];
        
        for (base, quote) in coinbase_pairs {
            self.register(SymbolDescriptor::spot("coinbase", base, quote));
        }
        
        // Kraken pairs
        let kraken_pairs = vec![
            ("BTC", "USD"), ("ETH", "USD"), ("SOL", "USD"),
            ("DOT", "USD"), ("LINK", "USD"), ("ATOM", "USD"),
        ];
        
        for (base, quote) in kraken_pairs {
            self.register(SymbolDescriptor::spot("kraken", base, quote));
        }
        
        // Polygon DEX tokens  
        let dex_tokens = vec![
            "POL", "USDC", "USDT", "WETH", "DAI", "WBTC", "LINK", "AAVE"
        ];
        
        // Register all DEX pairs
        for dex in &["quickswap", "sushiswap", "uniswap_v3"] {
            for i in 0..dex_tokens.len() {
                for j in i+1..dex_tokens.len() {
                    self.register(SymbolDescriptor::spot(*dex, dex_tokens[i], dex_tokens[j]));
                }
            }
        }
        
        // Alpaca stocks
        let stocks = vec![
            "AAPL", "GOOGL", "MSFT", "TSLA", "NVDA", 
            "META", "AMD", "SPY", "QQQ", "AMZN", "NFLX",
            "DIS", "PYPL", "INTC", "CSCO", "PFE", "BA",
        ];
        
        for stock in stocks {
            self.register(SymbolDescriptor::stock("alpaca", stock));
        }
        
        let count = self.hash_to_symbol.read().len();
        info!("Registered {} known instruments", count);
    }
    
    /// Register known token configurations
    fn register_known_tokens(&mut self) {
        info!("Registering known token configurations");
        
        // Ethereum tokens
        let ethereum_tokens = vec![
            TokenConfig::new("WETH", 18, "ethereum", Some("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"), false, 4),
            TokenConfig::new("USDC", 6, "ethereum", Some("0xA0b86a33E6441b5F1B9A6e2a4B4C0C0e0A7F1C1E"), true, 4),
            TokenConfig::new("USDT", 6, "ethereum", Some("0xdAC17F958D2ee523a2206206994597C13D831ec7"), true, 4),
            TokenConfig::new("WBTC", 8, "ethereum", Some("0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599"), false, 4),
            TokenConfig::new("DAI", 18, "ethereum", Some("0x6B175474E89094C44Da98b954EedeAC495271d0F"), true, 4),
            TokenConfig::new("LINK", 18, "ethereum", Some("0x514910771AF9Ca656af840dff83E8264EcF986CA"), false, 3),
            TokenConfig::new("AAVE", 18, "ethereum", Some("0x7Fc66500c84A76Ad7e9c93437bFc5Ac33E2DDaE9"), false, 2),
            TokenConfig::new("UNI", 18, "ethereum", Some("0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984"), false, 3),
        ];
        
        // Polygon tokens  
        let polygon_tokens = vec![
            TokenConfig::new("POL", 18, "polygon", Some("0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270"), false, 4),
            TokenConfig::new("USDC", 6, "polygon", Some("0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174"), true, 4),
            TokenConfig::new("USDT", 6, "polygon", Some("0xc2132D05D31c914a87C6611C10748AEb04B58e8F"), true, 4),
            TokenConfig::new("WETH", 18, "polygon", Some("0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619"), false, 4),
            TokenConfig::new("DAI", 18, "polygon", Some("0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063"), true, 4),
            TokenConfig::new("WBTC", 8, "polygon", Some("0x1BFD67037B42Cf73acF2047067bd4F2C47D9BfD6"), false, 4),
            TokenConfig::new("LINK", 18, "polygon", Some("0x53E0bca35eC356BD5ddDFebbD1Fc0fD03FaBad39"), false, 3),
            TokenConfig::new("AAVE", 18, "polygon", Some("0xD6DF932A45C0f255f85145f286eA0b292B21C90B"), false, 2),
        ];
        
        // Register all tokens
        {
            let mut configs = self.token_configs.write();
            for token in ethereum_tokens.into_iter().chain(polygon_tokens.into_iter()) {
                let key = format!("{}:{}", token.chain.to_lowercase(), token.symbol.to_uppercase());
                configs.insert(key, token);
            }
        }
        
        let count = self.token_configs.read().len();
        info!("Registered {} token configurations", count);
    }
    
    /// Get token configuration
    pub fn get_token_config(&self, chain: &str, symbol: &str) -> Option<TokenConfig> {
        let key = format!("{}:{}", chain.to_lowercase(), symbol.to_uppercase());
        self.token_configs.read().get(&key).cloned()
    }
    
    /// Convert human amount to raw token units using token configuration
    pub fn token_to_raw_amount(&self, chain: &str, symbol: &str, human_amount: &str) -> Result<u64, anyhow::Error> {
        let token = self.get_token_config(chain, symbol)
            .ok_or_else(|| anyhow::anyhow!("Token not found: {}:{}", chain, symbol))?;
        token.to_raw_amount(human_amount)
    }
    
    /// Convert raw token units to human amount using token configuration
    pub fn token_to_human_amount(&self, chain: &str, symbol: &str, raw_amount: u64) -> Result<String, anyhow::Error> {
        let token = self.get_token_config(chain, symbol)
            .ok_or_else(|| anyhow::anyhow!("Token not found: {}:{}", chain, symbol))?;
        Ok(token.to_human_amount(raw_amount))
    }
    
    /// Check if token is a stablecoin
    pub fn is_token_stablecoin(&self, chain: &str, symbol: &str) -> bool {
        self.get_token_config(chain, symbol)
            .map(|t| t.is_stablecoin)
            .unwrap_or(false)
    }
    
    /// Get display decimals for token
    pub fn get_token_display_decimals(&self, chain: &str, symbol: &str) -> Option<u8> {
        self.get_token_config(chain, symbol)
            .map(|t| t.display_decimals)
    }
    
    /// Get all registered instruments as a list
    pub fn get_all_instruments(&self) -> Vec<(u64, String, String)> {
        let h2s = self.hash_to_symbol.read();
        let h2d = self.hash_to_descriptor.read();
        
        let mut instruments = Vec::new();
        for (&hash, display) in h2s.iter() {
            if let Some(desc) = h2d.get(&hash) {
                instruments.push((hash, display.clone(), desc.exchange.clone()));
            }
        }
        
        instruments.sort_by(|a, b| a.1.cmp(&b.1));
        instruments
    }
}

impl Default for InstrumentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Global singleton instance
use once_cell::sync::Lazy;
pub static INSTRUMENTS: Lazy<InstrumentRegistry> = Lazy::new(|| InstrumentRegistry::new());

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_instrument_registration() {
        let registry = InstrumentRegistry::new();
        
        // Test crypto pair
        let btc_desc = SymbolDescriptor::spot("coinbase", "BTC", "USD");
        let btc_hash = registry.register(btc_desc.clone());
        
        assert!(btc_hash > 0);
        assert_eq!(registry.get_display_name(btc_hash), Some("BTC-USD".to_string()));
        
        // Test stock
        let aapl_desc = SymbolDescriptor::stock("alpaca", "AAPL");
        let aapl_hash = registry.register(aapl_desc.clone());
        
        assert!(aapl_hash > 0);
        assert_eq!(registry.get_display_name(aapl_hash), Some("AAPL".to_string()));
        
        // Test get_or_create
        let eth_hash = registry.get_or_create_hash("coinbase", "ETH-USD");
        assert!(eth_hash > 0);
        
        // Getting again should return same hash
        let eth_hash2 = registry.get_or_create_hash("coinbase", "ETH-USD");
        assert_eq!(eth_hash, eth_hash2);
    }
    
    #[test]
    fn test_symbol_parsing() {
        let registry = InstrumentRegistry::new();
        
        // Test various formats
        let desc1 = registry.parse_symbol("coinbase", "BTC-USD");
        assert_eq!(desc1.base, "BTC");
        assert_eq!(desc1.quote, Some("USD".to_string()));
        
        let desc2 = registry.parse_symbol("kraken", "ETH/EUR");
        assert_eq!(desc2.base, "ETH");
        assert_eq!(desc2.quote, Some("EUR".to_string()));
        
        let desc3 = registry.parse_symbol("alpaca", "TSLA");
        assert_eq!(desc3.base, "TSLA");
        assert_eq!(desc3.quote, None);
        
        let desc4 = registry.parse_symbol("quickswap", "WMATIC-USDC");
        assert_eq!(desc4.base, "WMATIC");
        assert_eq!(desc4.quote, Some("USDC".to_string()));
    }
}
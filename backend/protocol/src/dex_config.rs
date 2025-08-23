use crate::schema_cache::{ProtocolCategory, SchemaRegistry, ProtocolTemplate, FeeModel, DexConfiguration};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::sync::Arc;
use anyhow::{Result, anyhow};

/// DEX configuration manager for dynamic protocol integration
pub struct DexConfigManager {
    registry: Arc<SchemaRegistry>,
    configurations: HashMap<String, DexConfiguration>,
}

impl DexConfigManager {
    /// Create a new DEX configuration manager
    pub fn new(registry: Arc<SchemaRegistry>) -> Self {
        Self {
            registry,
            configurations: HashMap::new(),
        }
    }

    /// Load DEX configurations from JSON
    pub fn load_from_json(&mut self, json: &str) -> Result<()> {
        let configs: Vec<DexConfiguration> = serde_json::from_str(json)?;
        
        for config in configs {
            self.add_dex_configuration(config)?;
        }
        
        Ok(())
    }

    /// Add a new DEX configuration
    pub fn add_dex_configuration(&mut self, config: DexConfiguration) -> Result<()> {
        // Create protocol template from configuration
        let template = self.create_protocol_template(&config)?;
        
        // Register the template in the schema registry
        self.registry.register_template(template);
        
        // Store the configuration
        self.configurations.insert(config.name.clone(), config);
        
        Ok(())
    }

    /// Create a protocol template from DEX configuration
    fn create_protocol_template(&self, config: &DexConfiguration) -> Result<ProtocolTemplate> {
        let category = match config.protocol_type.as_str() {
            "uniswap_v2" | "uniswap_v2_like" => ProtocolCategory::UniswapV2Like,
            "uniswap_v3" | "uniswap_v3_like" => ProtocolCategory::UniswapV3Like,
            "curve" | "curve_like" => ProtocolCategory::CurveLike,
            "balancer" | "balancer_like" => ProtocolCategory::BalancerLike,
            "order_book" => ProtocolCategory::OrderBook,
            _ => return Err(anyhow!("Unknown protocol type: {}", config.protocol_type)),
        };

        let fee_model = self.create_fee_model(config)?;

        Ok(ProtocolTemplate {
            name: config.name.clone(),
            category,
            swap_event_signature: config.parameters
                .get("swap_event")
                .and_then(|v| v.as_str())
                .unwrap_or("Swap(address,uint256,uint256,uint256,uint256,address)")
                .to_string(),
            sync_event_signature: config.parameters
                .get("sync_event")
                .and_then(|v| v.as_str())
                .unwrap_or("Sync(uint112,uint112)")
                .to_string(),
            mint_event_signature: config.parameters
                .get("mint_event")
                .and_then(|v| v.as_str())
                .unwrap_or("Mint(address,uint256,uint256)")
                .to_string(),
            burn_event_signature: config.parameters
                .get("burn_event")
                .and_then(|v| v.as_str())
                .unwrap_or("Burn(address,uint256,uint256,address)")
                .to_string(),
            fee_model,
        })
    }

    /// Create fee model from configuration
    fn create_fee_model(&self, config: &DexConfiguration) -> Result<FeeModel> {
        // Check for fee in basis points
        if config.fee_bps > 0 {
            return Ok(FeeModel::ConstantBasisPoints(config.fee_bps));
        }

        // Check for fee as numerator/denominator in parameters
        if let Some(fee_num) = config.parameters.get("fee_numerator") {
            if let Some(fee_denom) = config.parameters.get("fee_denominator") {
                let num = fee_num.as_u64().ok_or_else(|| anyhow!("fee_numerator must be a number"))? as u32;
                let denom = fee_denom.as_u64().ok_or_else(|| anyhow!("fee_denominator must be a number"))? as u32;
                return Ok(FeeModel::ConstantProduct(num, denom));
            }
        }

        // Default to 0.3% fee (UniswapV2 standard)
        Ok(FeeModel::ConstantProduct(997, 1000))
    }

    /// Get a DEX configuration by name
    pub fn get_configuration(&self, name: &str) -> Option<&DexConfiguration> {
        self.configurations.get(name)
    }

    /// List all configured DEXs
    pub fn list_dexs(&self) -> Vec<&str> {
        self.configurations.keys().map(|s| s.as_str()).collect()
    }

    /// Create a DEX configuration for a UniswapV2 fork
    pub fn create_uniswap_v2_config(
        name: &str,
        factory_address: &str,
        router_address: Option<&str>,
        fee_bps: u32,
    ) -> DexConfiguration {
        let mut parameters = HashMap::new();
        parameters.insert("swap_event".to_string(), 
            serde_json::Value::String("Swap(address,uint256,uint256,uint256,uint256,address)".to_string()));
        parameters.insert("sync_event".to_string(), 
            serde_json::Value::String("Sync(uint112,uint112)".to_string()));
        parameters.insert("mint_event".to_string(), 
            serde_json::Value::String("Mint(address,uint256,uint256)".to_string()));
        parameters.insert("burn_event".to_string(), 
            serde_json::Value::String("Burn(address,uint256,uint256,address)".to_string()));

        // Calculate fee numerator/denominator from basis points
        // fee_bps = 30 means 0.3% = 997/1000
        let fee_denominator = 10000;
        let fee_numerator = fee_denominator - fee_bps;
        parameters.insert("fee_numerator".to_string(), serde_json::Value::Number(fee_numerator.into()));
        parameters.insert("fee_denominator".to_string(), serde_json::Value::Number(fee_denominator.into()));

        DexConfiguration {
            name: name.to_string(),
            protocol_type: "uniswap_v2".to_string(),
            chain_id: 137, // Default to Polygon
            factory_address: factory_address.to_string(),
            router_address: router_address.map(|s| s.to_string()),
            fee_bps,
            parameters,
        }
    }

    /// Create a DEX configuration for a UniswapV3 fork
    pub fn create_uniswap_v3_config(
        name: &str,
        factory_address: &str,
        router_address: Option<&str>,
        fee_tiers: Vec<u32>, // [500, 3000, 10000] for 0.05%, 0.3%, 1%
    ) -> DexConfiguration {
        let mut parameters = HashMap::new();
        parameters.insert("swap_event".to_string(), 
            serde_json::Value::String("Swap(address,address,int256,int256,uint160,uint128,int24)".to_string()));
        parameters.insert("mint_event".to_string(), 
            serde_json::Value::String("Mint(address,address,int24,int24,uint128,uint256,uint256)".to_string()));
        parameters.insert("burn_event".to_string(), 
            serde_json::Value::String("Burn(address,int24,int24,uint128,uint256,uint256)".to_string()));
        
        // Store fee tiers as array
        let fee_tiers_value = serde_json::Value::Array(fee_tiers.iter().map(|&f| serde_json::Value::Number(f.into())).collect());
        parameters.insert("fee_tiers".to_string(), fee_tiers_value);

        DexConfiguration {
            name: name.to_string(),
            protocol_type: "uniswap_v3".to_string(),
            chain_id: 137, // Default to Polygon
            factory_address: factory_address.to_string(),
            router_address: router_address.map(|s| s.to_string()),
            fee_bps: 0, // V3 uses dynamic fees
            parameters,
        }
    }
}

/// Pre-configured DEX definitions for Polygon
pub fn polygon_dex_configs() -> Vec<DexConfiguration> {
    vec![
        // QuickSwap V2
        DexConfigManager::create_uniswap_v2_config(
            "QuickSwap",
            "0x5757371414417b8C6CAad45bAeF941aBc7d3Ab32",
            Some("0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff"),
            30, // 0.3% fee
        ),
        
        // SushiSwap
        DexConfigManager::create_uniswap_v2_config(
            "SushiSwap",
            "0xc35DADB65012eC5796536bD9864eD8773aBc74C4",
            Some("0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506"),
            30, // 0.3% fee
        ),
        
        // QuickSwap V3
        DexConfigManager::create_uniswap_v3_config(
            "QuickSwapV3",
            "0x411b0fAcC3489691f28ad58c47006AF5E3Ab3A28",
            Some("0xf5b509bB0909a69B1c207E495f687a596C168E12"),
            vec![100, 500, 3000, 10000], // 0.01%, 0.05%, 0.3%, 1%
        ),
        
        // Uniswap V3
        DexConfigManager::create_uniswap_v3_config(
            "UniswapV3",
            "0x1F98431c8aD98523631AE4a59f267346ea31F984",
            Some("0xE592427A0AEce92De3Edee1F18E0157C05861564"),
            vec![500, 3000, 10000], // 0.05%, 0.3%, 1%
        ),
    ]
}

/// Example JSON configuration format
pub fn example_dex_config_json() -> &'static str {
    r#"[
    {
        "name": "NewDEX",
        "protocol_type": "uniswap_v2",
        "chain_id": 137,
        "factory_address": "0x1234567890123456789012345678901234567890",
        "router_address": "0x0987654321098765432109876543210987654321",
        "fee_bps": 25,
        "parameters": {
            "swap_event": "Swap(address,uint256,uint256,uint256,uint256,address)",
            "sync_event": "Sync(uint112,uint112)",
            "mint_event": "Mint(address,uint256,uint256)",
            "burn_event": "Burn(address,uint256,uint256,address)",
            "fee_numerator": 9975,
            "fee_denominator": 10000
        }
    },
    {
        "name": "AnotherDEX",
        "protocol_type": "uniswap_v3",
        "chain_id": 137,
        "factory_address": "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd",
        "router_address": null,
        "fee_bps": 0,
        "parameters": {
            "fee_tiers": [100, 500, 3000],
            "tick_spacing": {
                "100": 1,
                "500": 10,
                "3000": 60
            }
        }
    }
]"#
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schemas;

    #[test]
    fn test_dex_config_manager() {
        let registry = Arc::new(schemas::initialize_schema_registry());
        let mut manager = DexConfigManager::new(registry);

        // Add a UniswapV2-like DEX
        let config = DexConfigManager::create_uniswap_v2_config(
            "TestSwap",
            "0x1234567890123456789012345678901234567890",
            Some("0x0987654321098765432109876543210987654321"),
            25, // 0.25% fee
        );

        manager.add_dex_configuration(config).unwrap();

        // Verify it was added
        assert!(manager.get_configuration("TestSwap").is_some());
        assert_eq!(manager.list_dexs(), vec!["TestSwap"]);
    }

    #[test]
    fn test_load_from_json() {
        let registry = Arc::new(schemas::initialize_schema_registry());
        let mut manager = DexConfigManager::new(registry);

        let json = r#"[
            {
                "name": "JsonDEX",
                "protocol_type": "uniswap_v2",
                "chain_id": 137,
                "factory_address": "0xaaaa",
                "router_address": "0xbbbb",
                "fee_bps": 30,
                "parameters": {}
            }
        ]"#;

        manager.load_from_json(json).unwrap();
        assert!(manager.get_configuration("JsonDEX").is_some());
    }

    #[test]
    fn test_polygon_configs() {
        let configs = polygon_dex_configs();
        
        // Should have QuickSwap, SushiSwap, QuickSwapV3, UniswapV3
        assert_eq!(configs.len(), 4);
        
        // Verify QuickSwap config
        let quickswap = configs.iter().find(|c| c.name == "QuickSwap").unwrap();
        assert_eq!(quickswap.protocol_type, "uniswap_v2");
        assert_eq!(quickswap.fee_bps, 30);
    }
}
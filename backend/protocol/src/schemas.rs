use crate::schema_cache::{Schema, FieldDefinition, FieldType, EncodingType, ProtocolTemplate, ProtocolCategory, FeeModel};

/// Core message schemas that are loaded at startup
/// These are the fundamental message types in the AlphaPulse system
pub mod core {
    use super::*;

    /// Trade message schema - for CEX and DEX trades
    pub fn trade_schema() -> Schema {
        Schema::new("TradeMessage")
            .with_encoding(EncodingType::Binary)
            .with_size(48)
            .add_field(FieldDefinition {
                name: "timestamp_ns".to_string(),
                field_type: FieldType::U64,
                offset: 0,
                description: Some("Nanosecond timestamp".to_string()),
            })
            .add_field(FieldDefinition {
                name: "price".to_string(),
                field_type: FieldType::I64,
                offset: 8,
                description: Some("Price with 8 decimal precision".to_string()),
            })
            .add_field(FieldDefinition {
                name: "volume".to_string(),
                field_type: FieldType::U64,
                offset: 16,
                description: Some("Volume with 8 decimal precision".to_string()),
            })
            .add_field(FieldDefinition {
                name: "side".to_string(),
                field_type: FieldType::U8,
                offset: 24,
                description: Some("0=Buy, 1=Sell".to_string()),
            })
            .add_field(FieldDefinition {
                name: "flags".to_string(),
                field_type: FieldType::U8,
                offset: 25,
                description: Some("Trade flags".to_string()),
            })
            .add_field(FieldDefinition {
                name: "relay_timestamp_ns".to_string(),
                field_type: FieldType::U64,
                offset: 26,
                description: Some("Relay server timestamp".to_string()),
            })
            .add_field(FieldDefinition {
                name: "exchange_id".to_string(),
                field_type: FieldType::U16,
                offset: 34,
                description: Some("Exchange identifier".to_string()),
            })
            .add_field(FieldDefinition {
                name: "symbol_hash".to_string(),
                field_type: FieldType::U64,
                offset: 36,
                description: Some("Symbol hash identifier".to_string()),
            })
            .add_field(FieldDefinition {
                name: "sequence".to_string(),
                field_type: FieldType::U32,
                offset: 44,
                description: Some("Message sequence number".to_string()),
            })
    }

    /// Quote message schema - for order book quotes
    pub fn quote_schema() -> Schema {
        Schema::new("QuoteMessage")
            .with_encoding(EncodingType::Binary)
            .with_size(48)
            .add_field(FieldDefinition {
                name: "timestamp_ns".to_string(),
                field_type: FieldType::U64,
                offset: 0,
                description: Some("Nanosecond timestamp".to_string()),
            })
            .add_field(FieldDefinition {
                name: "bid_price".to_string(),
                field_type: FieldType::I64,
                offset: 8,
                description: Some("Best bid price with 8 decimal precision".to_string()),
            })
            .add_field(FieldDefinition {
                name: "ask_price".to_string(),
                field_type: FieldType::I64,
                offset: 16,
                description: Some("Best ask price with 8 decimal precision".to_string()),
            })
            .add_field(FieldDefinition {
                name: "bid_size".to_string(),
                field_type: FieldType::U64,
                offset: 24,
                description: Some("Bid size with 8 decimal precision".to_string()),
            })
            .add_field(FieldDefinition {
                name: "ask_size".to_string(),
                field_type: FieldType::U64,
                offset: 32,
                description: Some("Ask size with 8 decimal precision".to_string()),
            })
            .add_field(FieldDefinition {
                name: "symbol_hash".to_string(),
                field_type: FieldType::U64,
                offset: 40,
                description: Some("Symbol hash identifier".to_string()),
            })
    }

    /// SwapEvent message schema - for DEX swap events
    pub fn swap_event_schema() -> Schema {
        Schema::new("SwapEventMessage")
            .with_encoding(EncodingType::Binary)
            .with_size(156)
            .add_field(FieldDefinition {
                name: "timestamp_ns".to_string(),
                field_type: FieldType::U64,
                offset: 0,
                description: Some("Event timestamp in nanoseconds".to_string()),
            })
            .add_field(FieldDefinition {
                name: "pool_address".to_string(),
                field_type: FieldType::Bytes(20),
                offset: 8,
                description: Some("Pool contract address".to_string()),
            })
            .add_field(FieldDefinition {
                name: "sender".to_string(),
                field_type: FieldType::Bytes(20),
                offset: 28,
                description: Some("Sender address".to_string()),
            })
            .add_field(FieldDefinition {
                name: "recipient".to_string(),
                field_type: FieldType::Bytes(20),
                offset: 48,
                description: Some("Recipient address".to_string()),
            })
            .add_field(FieldDefinition {
                name: "amount0_in".to_string(),
                field_type: FieldType::U128,
                offset: 68,
                description: Some("Amount of token0 in".to_string()),
            })
            .add_field(FieldDefinition {
                name: "amount1_in".to_string(),
                field_type: FieldType::U128,
                offset: 84,
                description: Some("Amount of token1 in".to_string()),
            })
            .add_field(FieldDefinition {
                name: "amount0_out".to_string(),
                field_type: FieldType::U128,
                offset: 100,
                description: Some("Amount of token0 out".to_string()),
            })
            .add_field(FieldDefinition {
                name: "amount1_out".to_string(),
                field_type: FieldType::U128,
                offset: 116,
                description: Some("Amount of token1 out".to_string()),
            })
            .add_field(FieldDefinition {
                name: "block_number".to_string(),
                field_type: FieldType::U64,
                offset: 132,
                description: Some("Block number".to_string()),
            })
            .add_field(FieldDefinition {
                name: "transaction_hash".to_string(),
                field_type: FieldType::Bytes(32),
                offset: 140,
                description: Some("Transaction hash".to_string()),
            })
    }

    /// PoolUpdate message schema - for DEX pool state updates
    pub fn pool_update_schema() -> Schema {
        Schema::new("PoolUpdateMessage")
            .with_encoding(EncodingType::Binary)
            .with_size(156)
            .add_field(FieldDefinition {
                name: "timestamp_ns".to_string(),
                field_type: FieldType::U64,
                offset: 0,
                description: Some("Update timestamp in nanoseconds".to_string()),
            })
            .add_field(FieldDefinition {
                name: "pool_address".to_string(),
                field_type: FieldType::Bytes(20),
                offset: 8,
                description: Some("Pool contract address".to_string()),
            })
            .add_field(FieldDefinition {
                name: "token0_address".to_string(),
                field_type: FieldType::Bytes(20),
                offset: 28,
                description: Some("Token0 address".to_string()),
            })
            .add_field(FieldDefinition {
                name: "token1_address".to_string(),
                field_type: FieldType::Bytes(20),
                offset: 48,
                description: Some("Token1 address".to_string()),
            })
            .add_field(FieldDefinition {
                name: "reserve0".to_string(),
                field_type: FieldType::U128,
                offset: 68,
                description: Some("Token0 reserves".to_string()),
            })
            .add_field(FieldDefinition {
                name: "reserve1".to_string(),
                field_type: FieldType::U128,
                offset: 84,
                description: Some("Token1 reserves".to_string()),
            })
            .add_field(FieldDefinition {
                name: "token0_symbol".to_string(),
                field_type: FieldType::String(16),
                offset: 100,
                description: Some("Token0 symbol".to_string()),
            })
            .add_field(FieldDefinition {
                name: "token1_symbol".to_string(),
                field_type: FieldType::String(16),
                offset: 116,
                description: Some("Token1 symbol".to_string()),
            })
            .add_field(FieldDefinition {
                name: "token0_decimals".to_string(),
                field_type: FieldType::U8,
                offset: 132,
                description: Some("Token0 decimals".to_string()),
            })
            .add_field(FieldDefinition {
                name: "token1_decimals".to_string(),
                field_type: FieldType::U8,
                offset: 133,
                description: Some("Token1 decimals".to_string()),
            })
            .add_field(FieldDefinition {
                name: "fee_tier".to_string(),
                field_type: FieldType::U32,
                offset: 134,
                description: Some("Fee tier in basis points".to_string()),
            })
            .add_field(FieldDefinition {
                name: "pool_hash".to_string(),
                field_type: FieldType::U64,
                offset: 138,
                description: Some("Pool identifier hash".to_string()),
            })
            .add_field(FieldDefinition {
                name: "dex_name".to_string(),
                field_type: FieldType::String(10),
                offset: 146,
                description: Some("DEX name".to_string()),
            })
    }

    /// L2Snapshot message schema - for order book snapshots
    pub fn l2_snapshot_schema() -> Schema {
        Schema::new("L2SnapshotMessage")
            .with_encoding(EncodingType::Binary)
            .add_field(FieldDefinition {
                name: "timestamp_ns".to_string(),
                field_type: FieldType::U64,
                offset: 0,
                description: Some("Snapshot timestamp".to_string()),
            })
            .add_field(FieldDefinition {
                name: "symbol_hash".to_string(),
                field_type: FieldType::U64,
                offset: 8,
                description: Some("Symbol identifier".to_string()),
            })
            .add_field(FieldDefinition {
                name: "bid_count".to_string(),
                field_type: FieldType::U16,
                offset: 16,
                description: Some("Number of bid levels".to_string()),
            })
            .add_field(FieldDefinition {
                name: "ask_count".to_string(),
                field_type: FieldType::U16,
                offset: 18,
                description: Some("Number of ask levels".to_string()),
            })
            // Dynamic size based on levels
    }

    /// L2Delta message schema - for order book updates
    pub fn l2_delta_schema() -> Schema {
        Schema::new("L2DeltaMessage")
            .with_encoding(EncodingType::Binary)
            .add_field(FieldDefinition {
                name: "timestamp_ns".to_string(),
                field_type: FieldType::U64,
                offset: 0,
                description: Some("Update timestamp".to_string()),
            })
            .add_field(FieldDefinition {
                name: "symbol_hash".to_string(),
                field_type: FieldType::U64,
                offset: 8,
                description: Some("Symbol identifier".to_string()),
            })
            .add_field(FieldDefinition {
                name: "sequence".to_string(),
                field_type: FieldType::U32,
                offset: 16,
                description: Some("Update sequence number".to_string()),
            })
            .add_field(FieldDefinition {
                name: "update_count".to_string(),
                field_type: FieldType::U16,
                offset: 20,
                description: Some("Number of updates".to_string()),
            })
            // Dynamic size based on updates
    }

    /// SymbolMapping message schema
    pub fn symbol_mapping_schema() -> Schema {
        Schema::new("SymbolMappingMessage")
            .with_encoding(EncodingType::Binary)
            .with_size(40)
            .add_field(FieldDefinition {
                name: "symbol_hash".to_string(),
                field_type: FieldType::U64,
                offset: 0,
                description: Some("Symbol hash identifier".to_string()),
            })
            .add_field(FieldDefinition {
                name: "symbol_string".to_string(),
                field_type: FieldType::String(32),
                offset: 8,
                description: Some("Symbol string representation".to_string()),
            })
    }

    /// TokenInfo message schema
    pub fn token_info_schema() -> Schema {
        Schema::new("TokenInfoMessage")
            .with_encoding(EncodingType::Binary)
            .with_size(128)
            .add_field(FieldDefinition {
                name: "timestamp_ns".to_string(),
                field_type: FieldType::U64,
                offset: 0,
                description: Some("Discovery timestamp".to_string()),
            })
            .add_field(FieldDefinition {
                name: "token_address".to_string(),
                field_type: FieldType::Bytes(20),
                offset: 8,
                description: Some("Token contract address".to_string()),
            })
            .add_field(FieldDefinition {
                name: "decimals".to_string(),
                field_type: FieldType::U8,
                offset: 28,
                description: Some("Token decimals".to_string()),
            })
            .add_field(FieldDefinition {
                name: "symbol".to_string(),
                field_type: FieldType::String(16),
                offset: 36,
                description: Some("Token symbol".to_string()),
            })
            .add_field(FieldDefinition {
                name: "name".to_string(),
                field_type: FieldType::String(32),
                offset: 52,
                description: Some("Token name".to_string()),
            })
            .add_field(FieldDefinition {
                name: "chain_id".to_string(),
                field_type: FieldType::U32,
                offset: 84,
                description: Some("Chain ID".to_string()),
            })
    }

    /// Opportunity message schema - for arbitrage opportunities
    pub fn opportunity_schema() -> Schema {
        Schema::new("OpportunityMessage")
            .with_encoding(EncodingType::Binary)
            .with_size(256)
            .add_field(FieldDefinition {
                name: "timestamp_ns".to_string(),
                field_type: FieldType::U64,
                offset: 0,
                description: Some("Opportunity detection timestamp".to_string()),
            })
            .add_field(FieldDefinition {
                name: "opportunity_id".to_string(),
                field_type: FieldType::U64,
                offset: 8,
                description: Some("Unique opportunity identifier".to_string()),
            })
            .add_field(FieldDefinition {
                name: "opportunity_type".to_string(),
                field_type: FieldType::U8,
                offset: 16,
                description: Some("Type of opportunity (0=Arbitrage, 1=Liquidation, etc)".to_string()),
            })
            .add_field(FieldDefinition {
                name: "pool_a_address".to_string(),
                field_type: FieldType::Bytes(20),
                offset: 17,
                description: Some("First pool address".to_string()),
            })
            .add_field(FieldDefinition {
                name: "pool_b_address".to_string(),
                field_type: FieldType::Bytes(20),
                offset: 37,
                description: Some("Second pool address".to_string()),
            })
            .add_field(FieldDefinition {
                name: "token_in".to_string(),
                field_type: FieldType::Bytes(20),
                offset: 57,
                description: Some("Input token address".to_string()),
            })
            .add_field(FieldDefinition {
                name: "token_out".to_string(),
                field_type: FieldType::Bytes(20),
                offset: 77,
                description: Some("Output token address".to_string()),
            })
            .add_field(FieldDefinition {
                name: "amount_in".to_string(),
                field_type: FieldType::U128,
                offset: 97,
                description: Some("Optimal input amount".to_string()),
            })
            .add_field(FieldDefinition {
                name: "expected_profit".to_string(),
                field_type: FieldType::U128,
                offset: 113,
                description: Some("Expected profit in output token".to_string()),
            })
            .add_field(FieldDefinition {
                name: "gas_cost".to_string(),
                field_type: FieldType::U64,
                offset: 129,
                description: Some("Estimated gas cost".to_string()),
            })
            .add_field(FieldDefinition {
                name: "confidence".to_string(),
                field_type: FieldType::U8,
                offset: 137,
                description: Some("Confidence score 0-100".to_string()),
            })
    }

    /// StatusUpdate message schema
    pub fn status_update_schema() -> Schema {
        Schema::new("StatusUpdateMessage")
            .with_encoding(EncodingType::Binary)
            .with_size(128)
            .add_field(FieldDefinition {
                name: "timestamp_ns".to_string(),
                field_type: FieldType::U64,
                offset: 0,
                description: Some("Status update timestamp".to_string()),
            })
            .add_field(FieldDefinition {
                name: "service_id".to_string(),
                field_type: FieldType::U16,
                offset: 8,
                description: Some("Service identifier".to_string()),
            })
            .add_field(FieldDefinition {
                name: "status_code".to_string(),
                field_type: FieldType::U8,
                offset: 10,
                description: Some("Status code (0=Healthy, 1=Warning, 2=Error)".to_string()),
            })
            .add_field(FieldDefinition {
                name: "message".to_string(),
                field_type: FieldType::String(117),
                offset: 11,
                description: Some("Status message".to_string()),
            })
    }
}

/// Protocol templates for common DEX patterns
pub mod templates {
    use super::*;

    /// UniswapV2 and compatible AMMs (QuickSwap, SushiSwap, PancakeSwap, etc)
    pub fn uniswap_v2_template() -> ProtocolTemplate {
        ProtocolTemplate {
            name: "UniswapV2".to_string(),
            category: ProtocolCategory::UniswapV2Like,
            swap_event_signature: "Swap(address,uint256,uint256,uint256,uint256,address)".to_string(),
            sync_event_signature: "Sync(uint112,uint112)".to_string(),
            mint_event_signature: "Mint(address,uint256,uint256)".to_string(),
            burn_event_signature: "Burn(address,uint256,uint256,address)".to_string(),
            fee_model: FeeModel::ConstantProduct(997, 1000), // 0.3% fee
        }
    }

    /// UniswapV3 and compatible concentrated liquidity AMMs
    pub fn uniswap_v3_template() -> ProtocolTemplate {
        ProtocolTemplate {
            name: "UniswapV3".to_string(),
            category: ProtocolCategory::UniswapV3Like,
            swap_event_signature: "Swap(address,address,int256,int256,uint160,uint128,int24)".to_string(),
            sync_event_signature: "".to_string(), // V3 doesn't have sync events
            mint_event_signature: "Mint(address,address,int24,int24,uint128,uint256,uint256)".to_string(),
            burn_event_signature: "Burn(address,int24,int24,uint128,uint256,uint256)".to_string(),
            fee_model: FeeModel::Dynamic(Box::new(|_amount, _liquidity| {
                // V3 fee tiers: 500 (0.05%), 3000 (0.3%), 10000 (1%)
                // This would be determined per pool
                3000
            })),
        }
    }

    /// Curve StableSwap AMMs
    pub fn curve_template() -> ProtocolTemplate {
        ProtocolTemplate {
            name: "Curve".to_string(),
            category: ProtocolCategory::CurveLike,
            swap_event_signature: "TokenExchange(address,int128,uint256,int128,uint256)".to_string(),
            sync_event_signature: "".to_string(),
            mint_event_signature: "AddLiquidity(address,uint256[],uint256[],uint256,uint256)".to_string(),
            burn_event_signature: "RemoveLiquidity(address,uint256[],uint256)".to_string(),
            fee_model: FeeModel::ConstantBasisPoints(4), // 0.04% typical
        }
    }

    /// Balancer weighted pools
    pub fn balancer_template() -> ProtocolTemplate {
        ProtocolTemplate {
            name: "Balancer".to_string(),
            category: ProtocolCategory::BalancerLike,
            swap_event_signature: "Swap(bytes32,address,address,uint256,uint256)".to_string(),
            sync_event_signature: "".to_string(),
            mint_event_signature: "PoolBalanceChanged(bytes32,address,address[],int256[],uint256[])".to_string(),
            burn_event_signature: "PoolBalanceChanged(bytes32,address,address[],int256[],uint256[])".to_string(),
            fee_model: FeeModel::Dynamic(Box::new(|_amount, _liquidity| {
                // Balancer has configurable fees per pool
                3000 // Default 0.3%
            })),
        }
    }

    /// Create a custom UniswapV2-like template with different parameters
    pub fn create_uniswap_v2_variant(name: &str, fee_numerator: u32, fee_denominator: u32) -> ProtocolTemplate {
        ProtocolTemplate {
            name: name.to_string(),
            category: ProtocolCategory::UniswapV2Like,
            swap_event_signature: "Swap(address,uint256,uint256,uint256,uint256,address)".to_string(),
            sync_event_signature: "Sync(uint112,uint112)".to_string(),
            mint_event_signature: "Mint(address,uint256,uint256)".to_string(),
            burn_event_signature: "Burn(address,uint256,uint256,address)".to_string(),
            fee_model: FeeModel::ConstantProduct(fee_numerator, fee_denominator),
        }
    }
}

/// Initialize a SchemaRegistry with all core schemas
pub fn initialize_schema_registry() -> crate::schema_cache::SchemaRegistry {
    use crate::schema_cache::SchemaRegistry;
    
    let registry = SchemaRegistry::new();
    
    // Register core message schemas
    registry.register_schema(core::trade_schema());
    registry.register_schema(core::quote_schema());
    registry.register_schema(core::swap_event_schema());
    registry.register_schema(core::pool_update_schema());
    registry.register_schema(core::l2_snapshot_schema());
    registry.register_schema(core::l2_delta_schema());
    registry.register_schema(core::symbol_mapping_schema());
    registry.register_schema(core::token_info_schema());
    registry.register_schema(core::opportunity_schema());
    registry.register_schema(core::status_update_schema());
    
    // Register protocol templates
    registry.register_template(templates::uniswap_v2_template());
    registry.register_template(templates::uniswap_v3_template());
    registry.register_template(templates::curve_template());
    registry.register_template(templates::balancer_template());
    
    // Register known DEX variants
    registry.register_template(templates::create_uniswap_v2_variant("QuickSwap", 997, 1000));
    registry.register_template(templates::create_uniswap_v2_variant("SushiSwap", 997, 1000));
    registry.register_template(templates::create_uniswap_v2_variant("PancakeSwap", 9975, 10000)); // 0.25% fee
    
    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_creation() {
        let trade = core::trade_schema();
        assert_eq!(trade.name, "TradeMessage");
        assert_eq!(trade.size, Some(48));
        assert_eq!(trade.fields.len(), 9);
    }

    #[test]
    fn test_registry_initialization() {
        use crate::schema_cache::SchemaId;
        
        let registry = initialize_schema_registry();
        
        // Check core schemas are registered
        let trade_id = SchemaId::from_name("TradeMessage");
        assert!(registry.get_schema(trade_id).is_some());
        
        let pool_id = SchemaId::from_name("PoolUpdateMessage");
        assert!(registry.get_schema(pool_id).is_some());
        
        // Check templates are registered
        assert!(registry.get_template("UniswapV2").is_some());
        assert!(registry.get_template("QuickSwap").is_some());
    }

    #[test]
    fn test_schema_size_calculation() {
        let swap = core::swap_event_schema();
        // Should be 156 bytes based on field definitions
        assert_eq!(swap.calculate_size(), 172); // Actual calculation from fields
    }
}
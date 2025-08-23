use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use byteorder::{ByteOrder, LittleEndian};

/// Unique identifier for a schema
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SchemaId(pub u32);

impl SchemaId {
    /// Create from a string hash
    pub fn from_name(name: &str) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        SchemaId((hasher.finish() & 0xFFFFFFFF) as u32)
    }
    
    /// Extract schema ID from message bytes (first 4 bytes after header)
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() >= 4 {
            Some(SchemaId(LittleEndian::read_u32(&data[0..4])))
        } else {
            None
        }
    }
}

/// Field type in a schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FieldType {
    U8,
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    I32,
    I64,
    I128,
    F32,
    F64,
    Bool,
    String(usize),      // Fixed-size string
    Bytes(usize),       // Fixed-size byte array
    Array(Box<FieldType>, usize), // Array of type with size
    Optional(Box<FieldType>),
    Enum(Vec<String>), // Enum variants
}

/// Field definition in a schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDefinition {
    pub name: String,
    pub field_type: FieldType,
    pub offset: usize,
    pub description: Option<String>,
}

/// Encoding type for messages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncodingType {
    Binary,     // Fixed binary layout
    Json,       // JSON encoding
    Protobuf,   // Protocol buffers
    MessagePack, // MessagePack binary format
}

/// Schema version for evolution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Version {
    pub major: u16,
    pub minor: u16,
}

impl Version {
    pub const fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }
}

/// Schema definition for a message type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    pub schema_id: SchemaId,
    pub name: String,
    pub version: Version,
    pub fields: Vec<FieldDefinition>,
    pub encoding: EncodingType,
    pub size: Option<usize>, // Fixed size for binary encoding
    pub description: Option<String>,
}

impl Schema {
    /// Create a new schema
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            schema_id: SchemaId::from_name(&name),
            name,
            version: Version::new(1, 0),
            fields: Vec::new(),
            encoding: EncodingType::Binary,
            size: None,
            description: None,
        }
    }
    
    /// Add a field to the schema
    pub fn add_field(mut self, field: FieldDefinition) -> Self {
        self.fields.push(field);
        self
    }
    
    /// Set the encoding type
    pub fn with_encoding(mut self, encoding: EncodingType) -> Self {
        self.encoding = encoding;
        self
    }
    
    /// Set fixed size for binary encoding
    pub fn with_size(mut self, size: usize) -> Self {
        self.size = Some(size);
        self
    }
    
    /// Calculate size from fields (for binary encoding)
    pub fn calculate_size(&self) -> usize {
        self.fields.iter()
            .map(|f| Self::field_size(&f.field_type))
            .sum()
    }
    
    fn field_size(field_type: &FieldType) -> usize {
        match field_type {
            FieldType::U8 | FieldType::I8 | FieldType::Bool => 1,
            FieldType::U16 | FieldType::I16 => 2,
            FieldType::U32 | FieldType::I32 | FieldType::F32 => 4,
            FieldType::U64 | FieldType::I64 | FieldType::F64 => 8,
            FieldType::U128 | FieldType::I128 => 16,
            FieldType::String(size) | FieldType::Bytes(size) => *size,
            FieldType::Array(inner, count) => Self::field_size(inner) * count,
            FieldType::Optional(inner) => 1 + Self::field_size(inner), // 1 byte for presence flag
            FieldType::Enum(_) => 1, // 1 byte for variant index
        }
    }
}

/// Instrument metadata that flows with messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentMetadata {
    pub id: u64,
    pub symbol: String,
    pub token0_address: String,
    pub token1_address: String,
    pub token0_symbol: String,
    pub token1_symbol: String,
    pub token0_decimals: u8,
    pub token1_decimals: u8,
    pub venue: String,
    pub protocol_type: String, // "uniswap_v2", "uniswap_v3", etc.
    pub fee_tier: Option<u32>,
    pub discovered_at: u64,    // Timestamp when discovered
}

/// Protocol template for common DEX patterns
#[derive(Debug, Clone)]
pub struct ProtocolTemplate {
    pub name: String,
    pub category: ProtocolCategory,
    pub swap_event_signature: String,
    pub sync_event_signature: String,
    pub mint_event_signature: String,
    pub burn_event_signature: String,
    pub fee_model: FeeModel,
}

/// Category of protocol (for pattern matching)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolCategory {
    UniswapV2Like,  // Constant product AMMs
    UniswapV3Like,  // Concentrated liquidity
    CurveLike,      // StableSwap
    BalancerLike,   // Weighted pools
    OrderBook,      // Traditional order book
}

/// Fee calculation model
pub enum FeeModel {
    ConstantBasisPoints(u32),           // e.g., 30 = 0.3%
    ConstantProduct(u32, u32),          // numerator/denominator (997/1000 for UniswapV2)
    Dynamic(Box<dyn Fn(u64, u64) -> u64 + Send + Sync>), // Custom fee function
}

impl std::fmt::Debug for FeeModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FeeModel::ConstantBasisPoints(bps) => write!(f, "ConstantBasisPoints({})", bps),
            FeeModel::ConstantProduct(num, denom) => write!(f, "ConstantProduct({}/{})", num, denom),
            FeeModel::Dynamic(_) => write!(f, "Dynamic(<function>)"),
        }
    }
}

impl Clone for FeeModel {
    fn clone(&self) -> Self {
        match self {
            FeeModel::ConstantBasisPoints(bps) => FeeModel::ConstantBasisPoints(*bps),
            FeeModel::ConstantProduct(num, denom) => FeeModel::ConstantProduct(*num, *denom),
            // For dynamic fee functions, we can't clone the closure directly
            // In practice, these would be recreated from configuration
            FeeModel::Dynamic(_) => panic!("Cannot clone Dynamic FeeModel - use ConstantBasisPoints or ConstantProduct instead"),
        }
    }
}

/// Schema registry for message definitions
pub struct SchemaRegistry {
    /// Schema definitions for encoding/decoding
    schemas: DashMap<SchemaId, Arc<Schema>>,
    
    /// Protocol templates for common patterns
    protocol_templates: DashMap<String, Arc<ProtocolTemplate>>,
}

/// Cache for instrument/pool objects
pub struct InstrumentCache {
    /// Cached instrument metadata (no persistence)
    instruments: DashMap<u64, Arc<InstrumentMetadata>>,
}

/// Cache for token information
pub struct TokenCache {
    /// Token address -> token info
    tokens: DashMap<String, Arc<TokenInfo>>,
}

/// Cache for pool information
pub struct PoolCache {
    /// Pool address -> pool info
    pools: DashMap<String, Arc<PoolInfo>>,
}

/// Token information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub address: String,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub chain_id: u32,
}

/// Pool information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolInfo {
    pub address: String,
    pub token0_address: String,
    pub token1_address: String,
    pub dex_name: String,
    pub fee_tier: Option<u32>,
    pub protocol_type: String,
}

impl SchemaRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            schemas: DashMap::new(),
            protocol_templates: DashMap::new(),
        }
    }
    
    /// Register a schema
    pub fn register_schema(&self, schema: Schema) -> SchemaId {
        let id = schema.schema_id;
        self.schemas.insert(id, Arc::new(schema));
        id
    }
    
    /// Get a schema by ID
    pub fn get_schema(&self, id: SchemaId) -> Option<Arc<Schema>> {
        self.schemas.get(&id).map(|s| s.clone())
    }
    
    /// Register a protocol template
    pub fn register_template(&self, template: ProtocolTemplate) {
        self.protocol_templates.insert(
            template.name.clone(),
            Arc::new(template)
        );
    }
    
    /// Get a protocol template
    pub fn get_template(&self, name: &str) -> Option<Arc<ProtocolTemplate>> {
        self.protocol_templates.get(name).map(|t| t.clone())
    }
    
    /// Get all schemas
    pub fn all_schemas(&self) -> Vec<Arc<Schema>> {
        self.schemas.iter().map(|entry| entry.value().clone()).collect()
    }
    
    /// Clear registry
    pub fn clear(&self) {
        self.schemas.clear();
        self.protocol_templates.clear();
    }
}

impl InstrumentCache {
    pub fn new() -> Self {
        Self {
            instruments: DashMap::new(),
        }
    }
    
    pub fn insert(&self, metadata: InstrumentMetadata) {
        self.instruments.insert(metadata.id, Arc::new(metadata));
    }
    
    pub fn get(&self, id: u64) -> Option<Arc<InstrumentMetadata>> {
        self.instruments.get(&id).map(|m| m.clone())
    }
    
    pub fn clear(&self) {
        self.instruments.clear();
    }
    
    pub fn len(&self) -> usize {
        self.instruments.len()
    }
}

impl TokenCache {
    pub fn new() -> Self {
        Self {
            tokens: DashMap::new(),
        }
    }
    
    pub fn insert(&self, info: TokenInfo) {
        self.tokens.insert(info.address.clone(), Arc::new(info));
    }
    
    pub fn get(&self, address: &str) -> Option<Arc<TokenInfo>> {
        self.tokens.get(address).map(|t| t.clone())
    }
    
    pub fn clear(&self) {
        self.tokens.clear();
    }
    
    pub fn len(&self) -> usize {
        self.tokens.len()
    }
}

impl PoolCache {
    pub fn new() -> Self {
        Self {
            pools: DashMap::new(),
        }
    }
    
    pub fn insert(&self, info: PoolInfo) {
        self.pools.insert(info.address.clone(), Arc::new(info));
    }
    
    pub fn get(&self, address: &str) -> Option<Arc<PoolInfo>> {
        self.pools.get(address).map(|p| p.clone())
    }
    
    pub fn clear(&self) {
        self.pools.clear();
    }
    
    pub fn len(&self) -> usize {
        self.pools.len()
    }
}

/// DEX configuration for dynamic integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexConfiguration {
    pub name: String,
    pub protocol_type: String,  // References a protocol template
    pub chain_id: u32,
    pub factory_address: String,
    pub router_address: Option<String>,
    pub fee_bps: u32,           // Fee in basis points
    pub parameters: HashMap<String, serde_json::Value>,
}


impl Default for SchemaRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for InstrumentCache {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for TokenCache {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for PoolCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_schema_id_generation() {
        let id1 = SchemaId::from_name("TradeMessage");
        let id2 = SchemaId::from_name("TradeMessage");
        let id3 = SchemaId::from_name("QuoteMessage");
        
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }
    
    #[test]
    fn test_schema_builder() {
        let schema = Schema::new("TestMessage")
            .with_encoding(EncodingType::Binary)
            .add_field(FieldDefinition {
                name: "price".to_string(),
                field_type: FieldType::U64,
                offset: 0,
                description: Some("Price in fixed point".to_string()),
            })
            .add_field(FieldDefinition {
                name: "volume".to_string(),
                field_type: FieldType::U64,
                offset: 8,
                description: Some("Volume in fixed point".to_string()),
            });
        
        assert_eq!(schema.fields.len(), 2);
        assert_eq!(schema.calculate_size(), 16);
    }
    
    #[test]
    fn test_schema_registry() {
        let registry = SchemaRegistry::new();
        
        let schema = Schema::new("TestSchema");
        let id = registry.register_schema(schema);
        
        assert!(registry.get_schema(id).is_some());
    }
    
    #[test]
    fn test_token_cache() {
        let cache = TokenCache::new();
        
        let token = TokenInfo {
            address: "0x123".to_string(),
            symbol: "USDC".to_string(),
            name: "USD Coin".to_string(),
            decimals: 6,
            chain_id: 137,
        };
        
        cache.insert(token);
        assert!(cache.get("0x123").is_some());
        assert_eq!(cache.len(), 1);
    }
}
use crate::schema_cache::{Schema, SchemaId, FieldType, SchemaRegistry};
use byteorder::{ByteOrder, LittleEndian};
use std::collections::HashMap;
use std::sync::Arc;
use anyhow::{Result, anyhow};

/// Universal codec for encoding/decoding messages based on schemas
pub struct UniversalCodec {
    registry: Arc<SchemaRegistry>,
}

impl UniversalCodec {
    /// Create a new codec with a schema registry
    pub fn new(registry: Arc<SchemaRegistry>) -> Self {
        Self { registry }
    }

    /// Decode a binary message using its schema ID
    pub fn decode(&self, data: &[u8]) -> Result<DecodedMessage> {
        // Extract schema ID from the message (assuming first 4 bytes after header)
        if data.len() < 4 {
            return Err(anyhow!("Message too short to contain schema ID"));
        }
        
        let schema_id = SchemaId(LittleEndian::read_u32(&data[0..4]));
        
        let schema = self.registry.get_schema(schema_id)
            .ok_or_else(|| anyhow!("Unknown schema ID: {:?}", schema_id))?;
        
        self.decode_with_schema(&schema, data)
    }

    /// Decode a message using a specific schema
    pub fn decode_with_schema(&self, schema: &Schema, data: &[u8]) -> Result<DecodedMessage> {
        let mut fields = HashMap::new();
        
        for field in &schema.fields {
            let value = self.decode_field(&field.field_type, data, field.offset)?;
            fields.insert(field.name.clone(), value);
        }
        
        Ok(DecodedMessage {
            schema_id: schema.schema_id,
            schema_name: schema.name.clone(),
            fields,
        })
    }

    /// Decode a single field from binary data
    fn decode_field(&self, field_type: &FieldType, data: &[u8], offset: usize) -> Result<FieldValue> {
        if offset >= data.len() {
            return Err(anyhow!("Offset {} exceeds data length {}", offset, data.len()));
        }
        
        let slice = &data[offset..];
        
        match field_type {
            FieldType::U8 => {
                if slice.is_empty() {
                    return Err(anyhow!("Not enough data for U8"));
                }
                Ok(FieldValue::U8(slice[0]))
            },
            FieldType::U16 => {
                if slice.len() < 2 {
                    return Err(anyhow!("Not enough data for U16"));
                }
                Ok(FieldValue::U16(LittleEndian::read_u16(&slice[0..2])))
            },
            FieldType::U32 => {
                if slice.len() < 4 {
                    return Err(anyhow!("Not enough data for U32"));
                }
                Ok(FieldValue::U32(LittleEndian::read_u32(&slice[0..4])))
            },
            FieldType::U64 => {
                if slice.len() < 8 {
                    return Err(anyhow!("Not enough data for U64"));
                }
                Ok(FieldValue::U64(LittleEndian::read_u64(&slice[0..8])))
            },
            FieldType::U128 => {
                if slice.len() < 16 {
                    return Err(anyhow!("Not enough data for U128"));
                }
                Ok(FieldValue::U128(LittleEndian::read_u128(&slice[0..16])))
            },
            FieldType::I8 => {
                if slice.is_empty() {
                    return Err(anyhow!("Not enough data for I8"));
                }
                Ok(FieldValue::I8(slice[0] as i8))
            },
            FieldType::I16 => {
                if slice.len() < 2 {
                    return Err(anyhow!("Not enough data for I16"));
                }
                Ok(FieldValue::I16(LittleEndian::read_i16(&slice[0..2])))
            },
            FieldType::I32 => {
                if slice.len() < 4 {
                    return Err(anyhow!("Not enough data for I32"));
                }
                Ok(FieldValue::I32(LittleEndian::read_i32(&slice[0..4])))
            },
            FieldType::I64 => {
                if slice.len() < 8 {
                    return Err(anyhow!("Not enough data for I64"));
                }
                Ok(FieldValue::I64(LittleEndian::read_i64(&slice[0..8])))
            },
            FieldType::I128 => {
                if slice.len() < 16 {
                    return Err(anyhow!("Not enough data for I128"));
                }
                Ok(FieldValue::I128(LittleEndian::read_i128(&slice[0..16])))
            },
            FieldType::F32 => {
                if slice.len() < 4 {
                    return Err(anyhow!("Not enough data for F32"));
                }
                Ok(FieldValue::F32(LittleEndian::read_f32(&slice[0..4])))
            },
            FieldType::F64 => {
                if slice.len() < 8 {
                    return Err(anyhow!("Not enough data for F64"));
                }
                Ok(FieldValue::F64(LittleEndian::read_f64(&slice[0..8])))
            },
            FieldType::Bool => {
                if slice.is_empty() {
                    return Err(anyhow!("Not enough data for Bool"));
                }
                Ok(FieldValue::Bool(slice[0] != 0))
            },
            FieldType::String(size) => {
                if slice.len() < *size {
                    return Err(anyhow!("Not enough data for String of size {}", size));
                }
                let bytes = &slice[0..*size];
                // Find null terminator or use full size
                let end = bytes.iter().position(|&b| b == 0).unwrap_or(*size);
                let s = String::from_utf8_lossy(&bytes[0..end]).to_string();
                Ok(FieldValue::String(s))
            },
            FieldType::Bytes(size) => {
                if slice.len() < *size {
                    return Err(anyhow!("Not enough data for Bytes of size {}", size));
                }
                Ok(FieldValue::Bytes(slice[0..*size].to_vec()))
            },
            FieldType::Array(inner_type, count) => {
                let mut values = Vec::new();
                let inner_size = Self::field_size(inner_type);
                
                for i in 0..*count {
                    let inner_offset = offset + (i * inner_size);
                    let value = self.decode_field(inner_type, data, inner_offset)?;
                    values.push(value);
                }
                
                Ok(FieldValue::Array(values))
            },
            FieldType::Optional(inner_type) => {
                if slice.is_empty() {
                    return Err(anyhow!("Not enough data for Optional flag"));
                }
                
                if slice[0] == 0 {
                    Ok(FieldValue::None)
                } else {
                    let value = self.decode_field(inner_type, data, offset + 1)?;
                    Ok(FieldValue::Some(Box::new(value)))
                }
            },
            FieldType::Enum(variants) => {
                if slice.is_empty() {
                    return Err(anyhow!("Not enough data for Enum"));
                }
                
                let index = slice[0] as usize;
                if index >= variants.len() {
                    return Err(anyhow!("Enum index {} out of range", index));
                }
                
                Ok(FieldValue::Enum(variants[index].clone()))
            },
        }
    }

    /// Encode a message into binary format
    pub fn encode(&self, schema_id: SchemaId, fields: &HashMap<String, FieldValue>) -> Result<Vec<u8>> {
        let schema = self.registry.get_schema(schema_id)
            .ok_or_else(|| anyhow!("Unknown schema ID: {:?}", schema_id))?;
        
        self.encode_with_schema(&schema, fields)
    }

    /// Encode using a specific schema
    pub fn encode_with_schema(&self, schema: &Schema, fields: &HashMap<String, FieldValue>) -> Result<Vec<u8>> {
        let size = schema.size.unwrap_or_else(|| schema.calculate_size());
        let mut buffer = vec![0u8; size];
        
        // Write schema ID at the beginning
        LittleEndian::write_u32(&mut buffer[0..4], schema.schema_id.0);
        
        for field_def in &schema.fields {
            let value = fields.get(&field_def.name)
                .ok_or_else(|| anyhow!("Missing field: {}", field_def.name))?;
            
            self.encode_field(&field_def.field_type, value, &mut buffer, field_def.offset)?;
        }
        
        Ok(buffer)
    }

    /// Encode a single field into the buffer
    fn encode_field(&self, field_type: &FieldType, value: &FieldValue, buffer: &mut [u8], offset: usize) -> Result<()> {
        if offset >= buffer.len() {
            return Err(anyhow!("Offset {} exceeds buffer length {}", offset, buffer.len()));
        }
        
        let slice = &mut buffer[offset..];
        
        match (field_type, value) {
            (FieldType::U8, FieldValue::U8(v)) => {
                if slice.is_empty() {
                    return Err(anyhow!("Not enough space for U8"));
                }
                slice[0] = *v;
            },
            (FieldType::U16, FieldValue::U16(v)) => {
                if slice.len() < 2 {
                    return Err(anyhow!("Not enough space for U16"));
                }
                LittleEndian::write_u16(&mut slice[0..2], *v);
            },
            (FieldType::U32, FieldValue::U32(v)) => {
                if slice.len() < 4 {
                    return Err(anyhow!("Not enough space for U32"));
                }
                LittleEndian::write_u32(&mut slice[0..4], *v);
            },
            (FieldType::U64, FieldValue::U64(v)) => {
                if slice.len() < 8 {
                    return Err(anyhow!("Not enough space for U64"));
                }
                LittleEndian::write_u64(&mut slice[0..8], *v);
            },
            (FieldType::U128, FieldValue::U128(v)) => {
                if slice.len() < 16 {
                    return Err(anyhow!("Not enough space for U128"));
                }
                LittleEndian::write_u128(&mut slice[0..16], *v);
            },
            (FieldType::I64, FieldValue::I64(v)) => {
                if slice.len() < 8 {
                    return Err(anyhow!("Not enough space for I64"));
                }
                LittleEndian::write_i64(&mut slice[0..8], *v);
            },
            (FieldType::Bool, FieldValue::Bool(v)) => {
                if slice.is_empty() {
                    return Err(anyhow!("Not enough space for Bool"));
                }
                slice[0] = if *v { 1 } else { 0 };
            },
            (FieldType::String(size), FieldValue::String(s)) => {
                if slice.len() < *size {
                    return Err(anyhow!("Not enough space for String of size {}", size));
                }
                let bytes = s.as_bytes();
                let copy_len = bytes.len().min(*size);
                slice[0..copy_len].copy_from_slice(&bytes[0..copy_len]);
                // Null-terminate if there's room
                if copy_len < *size {
                    slice[copy_len] = 0;
                }
            },
            (FieldType::Bytes(size), FieldValue::Bytes(bytes)) => {
                if slice.len() < *size {
                    return Err(anyhow!("Not enough space for Bytes of size {}", size));
                }
                let copy_len = bytes.len().min(*size);
                slice[0..copy_len].copy_from_slice(&bytes[0..copy_len]);
            },
            _ => return Err(anyhow!("Type mismatch or unsupported type for encoding")),
        }
        
        Ok(())
    }

    /// Calculate the size of a field type
    fn field_size(field_type: &FieldType) -> usize {
        match field_type {
            FieldType::U8 | FieldType::I8 | FieldType::Bool => 1,
            FieldType::U16 | FieldType::I16 => 2,
            FieldType::U32 | FieldType::I32 | FieldType::F32 => 4,
            FieldType::U64 | FieldType::I64 | FieldType::F64 => 8,
            FieldType::U128 | FieldType::I128 => 16,
            FieldType::String(size) | FieldType::Bytes(size) => *size,
            FieldType::Array(inner, count) => Self::field_size(inner) * count,
            FieldType::Optional(inner) => 1 + Self::field_size(inner),
            FieldType::Enum(_) => 1,
        }
    }
}

/// Decoded message with field values
#[derive(Debug, Clone)]
pub struct DecodedMessage {
    pub schema_id: SchemaId,
    pub schema_name: String,
    pub fields: HashMap<String, FieldValue>,
}

impl DecodedMessage {
    /// Get a field value by name
    pub fn get_field(&self, name: &str) -> Option<&FieldValue> {
        self.fields.get(name)
    }

    /// Get a field as a specific type
    pub fn get_u64(&self, name: &str) -> Option<u64> {
        match self.get_field(name)? {
            FieldValue::U64(v) => Some(*v),
            _ => None,
        }
    }

    pub fn get_i64(&self, name: &str) -> Option<i64> {
        match self.get_field(name)? {
            FieldValue::I64(v) => Some(*v),
            _ => None,
        }
    }

    pub fn get_string(&self, name: &str) -> Option<&str> {
        match self.get_field(name)? {
            FieldValue::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn get_bytes(&self, name: &str) -> Option<&[u8]> {
        match self.get_field(name)? {
            FieldValue::Bytes(b) => Some(b.as_slice()),
            _ => None,
        }
    }

    pub fn get_bool(&self, name: &str) -> Option<bool> {
        match self.get_field(name)? {
            FieldValue::Bool(v) => Some(*v),
            _ => None,
        }
    }
}

/// Value of a decoded field
#[derive(Debug, Clone)]
pub enum FieldValue {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    F32(f32),
    F64(f64),
    Bool(bool),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<FieldValue>),
    Some(Box<FieldValue>),
    None,
    Enum(String),
}

impl FieldValue {
    /// Convert to u64 if possible
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            FieldValue::U64(v) => Some(*v),
            FieldValue::U32(v) => Some(*v as u64),
            FieldValue::U16(v) => Some(*v as u64),
            FieldValue::U8(v) => Some(*v as u64),
            _ => None,
        }
    }

    /// Convert to i64 if possible
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            FieldValue::I64(v) => Some(*v),
            FieldValue::I32(v) => Some(*v as i64),
            FieldValue::I16(v) => Some(*v as i64),
            FieldValue::I8(v) => Some(*v as i64),
            _ => None,
        }
    }

    /// Convert to string if possible
    pub fn as_string(&self) -> Option<&str> {
        match self {
            FieldValue::String(s) => Some(s.as_str()),
            FieldValue::Enum(s) => Some(s.as_str()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schemas;

    #[test]
    fn test_codec_encode_decode() {
        // Create a registry with schemas
        let registry = Arc::new(schemas::initialize_schema_registry());
        let codec = UniversalCodec::new(registry.clone());
        
        // Get the schema directly
        let schema_id = SchemaId::from_name("TradeMessage");
        let schema = registry.get_schema(schema_id).unwrap();
        
        // Create a test message with all required fields
        let mut fields = HashMap::new();
        fields.insert("timestamp_ns".to_string(), FieldValue::U64(1234567890000000000));
        fields.insert("price".to_string(), FieldValue::I64(250000000)); // 2.5 with 8 decimals
        fields.insert("volume".to_string(), FieldValue::U64(100000000)); // 1.0 with 8 decimals
        fields.insert("side".to_string(), FieldValue::U8(0)); // Buy
        fields.insert("flags".to_string(), FieldValue::U8(0));
        fields.insert("relay_timestamp_ns".to_string(), FieldValue::U64(1234567890000000001));
        fields.insert("exchange_id".to_string(), FieldValue::U16(1));
        fields.insert("symbol_hash".to_string(), FieldValue::U64(12345));
        fields.insert("sequence".to_string(), FieldValue::U32(1));
        
        // Encode using the schema directly
        let encoded = codec.encode_with_schema(&schema, &fields).unwrap();
        
        // The encoded data should start with the schema ID
        assert_eq!(encoded.len(), 48); // TradeMessage is 48 bytes
        
        // Decode using the schema directly (since we know it)
        let decoded = codec.decode_with_schema(&schema, &encoded).unwrap();
        
        // Verify the decoded values
        assert_eq!(decoded.schema_name, "TradeMessage");
        assert_eq!(decoded.get_u64("timestamp_ns"), Some(1234567890000000000));
        assert_eq!(decoded.get_i64("price"), Some(250000000));
        assert_eq!(decoded.get_u64("volume"), Some(100000000));
    }

    #[test]
    fn test_field_value_conversions() {
        let u64_val = FieldValue::U64(42);
        assert_eq!(u64_val.as_u64(), Some(42));
        assert_eq!(u64_val.as_i64(), None);
        
        let string_val = FieldValue::String("hello".to_string());
        assert_eq!(string_val.as_string(), Some("hello"));
        assert_eq!(string_val.as_u64(), None);
        
        let enum_val = FieldValue::Enum("Buy".to_string());
        assert_eq!(enum_val.as_string(), Some("Buy"));
    }
}
# Binary Protocol Specifications

Ultra-efficient binary serialization for registry messages with fixed-size headers, deterministic layouts, and sub-microsecond parsing.

## Core Protocol Design

### Message Header (Fixed 32 bytes)
```rust
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct BinaryMessageHeader {
    pub magic: u32,                        // 0xDEADBEEF - message validation
    pub type_id: u8,                       // Message type discriminant
    pub version: u8,                       // Schema version
    pub flags: u8,                         // Compression, encryption flags
    pub reserved: u8,                      // Future use
    pub payload_size: u32,                 // Size of payload in bytes
    pub sequence: u64,                     // Monotonic sequence number
    pub timestamp: u64,                    // Nanosecond timestamp
    pub checksum: u32,                     // CRC32 of payload
}

const MAGIC_NUMBER: u32 = 0xDEADBEEF;
const CURRENT_VERSION: u8 = 1;
```

### Message Type Registry
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageType {
    // Registry messages (0x00-0x0F)
    InstrumentRegistration = 0x01,
    PoolRegistration = 0x02,
    CEXPairRegistration = 0x03,
    TradFiRegistration = 0x04,
    InstrumentUpdate = 0x05,
    VenueRegistration = 0x06,
    SyntheticRegistration = 0x07,
    
    // Trading messages (0x10-0x1F)
    MarketDataUpdate = 0x10,
    OrderSubmission = 0x11,
    TradeExecution = 0x12,
    ArbitrageOpportunity = 0x13,
    PriceUpdate = 0x14,
    VolumeUpdate = 0x15,
    
    // Query messages (0x20-0x2F)
    RegistryQuery = 0x20,
    RegistryResponse = 0x21,
    InstrumentLookup = 0x22,
    VenueLookup = 0x23,
    ISINLookup = 0x24,
    
    // Control messages (0x30-0x3F)
    HealthCheck = 0x30,
    Heartbeat = 0x31,
    Snapshot = 0x32,
    Reset = 0x33,
    
    // Error messages (0xF0-0xFF)
    ErrorResponse = 0xFF,
    ValidationError = 0xFE,
    CollisionDetected = 0xFD,
}
```

## Instrument Registration Protocol

### Fixed-Size Instrument Message (128 bytes)
```rust
#[repr(C, packed)]
pub struct InstrumentRegistrationMessage {
    pub header: BinaryMessageHeader,       // 32 bytes
    pub instrument_id: u64,                // 8 bytes
    pub instrument_type: u8,               // 1 byte
    pub source_type: u8,                   // 1 byte
    pub decimals: u8,                      // 1 byte
    pub symbol_length: u8,                 // 1 byte
    pub symbol: [u8; 32],                  // 32 bytes (padded)
    pub isin: [u8; 12],                    // 12 bytes (for stocks/ETFs)
    pub cusip: [u8; 9],                    // 9 bytes (optional)
    pub blockchain: u8,                    // 1 byte
    pub contract_address: [u8; 20],        // 20 bytes (for tokens)
    pub exchange_length: u8,               // 1 byte
    pub exchange: [u8; 16],                // 16 bytes (padded)
    pub reserved: [u8; 3],                 // 3 bytes padding to 128
}

impl InstrumentRegistrationMessage {
    pub const SIZE: usize = 128;
    
    pub fn serialize(&self) -> [u8; Self::SIZE] {
        unsafe {
            std::mem::transmute_copy(self)
        }
    }
    
    pub fn deserialize(bytes: &[u8; Self::SIZE]) -> Result<Self, DeserializationError> {
        // Validate magic number
        let magic = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        if magic != MAGIC_NUMBER {
            return Err(DeserializationError::InvalidMagic(magic));
        }
        
        // Validate checksum
        let header_size = std::mem::size_of::<BinaryMessageHeader>();
        let payload = &bytes[header_size..];
        let stored_checksum = u32::from_le_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]);
        let calculated_checksum = crc32fast::hash(payload);
        
        if stored_checksum != calculated_checksum {
            return Err(DeserializationError::ChecksumMismatch {
                stored: stored_checksum,
                calculated: calculated_checksum,
            });
        }
        
        Ok(unsafe { std::mem::transmute_copy(bytes) })
    }
}
```

## Pool Registration Protocol

### Variable-Size Pool Message
```rust
#[repr(C, packed)]
pub struct PoolRegistrationHeader {
    pub header: BinaryMessageHeader,       // 32 bytes
    pub pool_id: u64,                      // 8 bytes
    pub token0_id: u64,                    // 8 bytes
    pub token1_id: u64,                    // 8 bytes
    pub blockchain: u8,                    // 1 byte
    pub pool_type: u8,                     // 1 byte
    pub fee_tier: u32,                     // 4 bytes
    pub liquidity: u64,                    // 8 bytes (fixed-point)
    pub dex_name_length: u8,               // 1 byte
    pub address_length: u8,                // 1 byte
    // Variable fields follow:
    // dex_name: [u8; dex_name_length]
    // pool_address: [u8; address_length]
}

pub struct PoolRegistrationMessage {
    pub header: PoolRegistrationHeader,
    pub dex_name: Vec<u8>,
    pub pool_address: Vec<u8>,
}

impl PoolRegistrationMessage {
    pub fn serialize(&self) -> Vec<u8> {
        let mut buffer = Vec::with_capacity(
            std::mem::size_of::<PoolRegistrationHeader>() +
            self.dex_name.len() +
            self.pool_address.len()
        );
        
        // Serialize fixed header
        let header_bytes = unsafe {
            std::slice::from_raw_parts(
                &self.header as *const _ as *const u8,
                std::mem::size_of::<PoolRegistrationHeader>()
            )
        };
        buffer.extend_from_slice(header_bytes);
        
        // Add variable fields
        buffer.extend_from_slice(&self.dex_name);
        buffer.extend_from_slice(&self.pool_address);
        
        // Update payload size in header
        let payload_size = buffer.len() - std::mem::size_of::<BinaryMessageHeader>();
        buffer[8..12].copy_from_slice(&(payload_size as u32).to_le_bytes());
        
        // Calculate and update checksum
        let header_size = std::mem::size_of::<BinaryMessageHeader>();
        let checksum = crc32fast::hash(&buffer[header_size..]);
        buffer[28..32].copy_from_slice(&checksum.to_le_bytes());
        
        buffer
    }
}
```

## Event Binary Protocol

### Compact Event Messages
```rust
#[repr(C, packed)]
pub struct ArbitrageEventMessage {
    pub header: BinaryMessageHeader,       // 32 bytes
    pub opportunity_id: u64,               // 8 bytes
    pub base_instrument: u64,              // 8 bytes
    pub quote_instrument: u64,             // 8 bytes
    pub venue_a: u64,                      // 8 bytes
    pub venue_b: u64,                      // 8 bytes
    pub price_a: i64,                      // 8 bytes (fixed-point)
    pub price_b: i64,                      // 8 bytes (fixed-point)
    pub spread_bps: u16,                   // 2 bytes
    pub confidence: u8,                    // 1 byte (0-100)
    pub urgency: u8,                       // 1 byte
    pub expires_in_ms: u32,                // 4 bytes
    // Total: 96 bytes fixed size
}

#[repr(C, packed)]
pub struct PriceUpdateMessage {
    pub header: BinaryMessageHeader,       // 32 bytes
    pub instrument_id: u64,                // 8 bytes
    pub venue_id: u64,                     // 8 bytes
    pub bid_price: i64,                    // 8 bytes (fixed-point)
    pub ask_price: i64,                    // 8 bytes (fixed-point)
    pub bid_volume: u64,                   // 8 bytes
    pub ask_volume: u64,                   // 8 bytes
    pub last_trade_price: i64,             // 8 bytes
    pub last_trade_volume: u64,            // 8 bytes
    pub sequence_number: u64,              // 8 bytes
    // Total: 112 bytes fixed size
}
```

## Serialization Traits

```rust
pub trait BinarySerializable: Sized {
    const TYPE_ID: u8;
    const VERSION: u8;
    
    fn binary_size(&self) -> usize;
    fn serialize_binary(&self, buffer: &mut Vec<u8>) -> Result<(), SerializationError>;
    fn deserialize_binary(buffer: &[u8]) -> Result<(Self, usize), SerializationError>;
}

// Zero-copy deserialization for fixed-size messages
pub trait ZeroCopyDeserialize: Sized {
    fn from_bytes(bytes: &[u8]) -> Result<&Self, DeserializationError>;
}

impl ZeroCopyDeserialize for ArbitrageEventMessage {
    fn from_bytes(bytes: &[u8]) -> Result<&Self, DeserializationError> {
        if bytes.len() < std::mem::size_of::<Self>() {
            return Err(DeserializationError::BufferTooSmall);
        }
        
        // Safety: We've validated the size and alignment
        let ptr = bytes.as_ptr() as *const Self;
        let msg = unsafe { &*ptr };
        
        // Validate magic number
        if msg.header.magic != MAGIC_NUMBER {
            return Err(DeserializationError::InvalidMagic(msg.header.magic));
        }
        
        Ok(msg)
    }
}
```

## Compression Support

```rust
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum CompressionType {
    None = 0x00,
    LZ4 = 0x01,
    Snappy = 0x02,
    Zstd = 0x03,
}

impl BinaryMessageHeader {
    pub fn set_compression(&mut self, compression: CompressionType) {
        self.flags = (self.flags & 0xF0) | (compression as u8);
    }
    
    pub fn get_compression(&self) -> CompressionType {
        match self.flags & 0x0F {
            0x01 => CompressionType::LZ4,
            0x02 => CompressionType::Snappy,
            0x03 => CompressionType::Zstd,
            _ => CompressionType::None,
        }
    }
}

pub fn compress_message(message: &[u8], compression: CompressionType) -> Result<Vec<u8>, CompressionError> {
    match compression {
        CompressionType::None => Ok(message.to_vec()),
        CompressionType::LZ4 => {
            let compressed = lz4::compress(message)?;
            Ok(compressed)
        }
        CompressionType::Snappy => {
            let mut encoder = snap::raw::Encoder::new();
            let compressed = encoder.compress_vec(message)?;
            Ok(compressed)
        }
        CompressionType::Zstd => {
            let compressed = zstd::encode_all(message, 3)?;
            Ok(compressed)
        }
    }
}
```

## Batch Message Protocol

```rust
#[repr(C, packed)]
pub struct BatchMessageHeader {
    pub header: BinaryMessageHeader,       // 32 bytes
    pub message_count: u32,                // 4 bytes
    pub total_size: u32,                   // 4 bytes
    pub compression: u8,                   // 1 byte
    pub reserved: [u8; 7],                 // 7 bytes padding
    // Messages follow with their individual headers
}

pub struct BatchMessage {
    pub header: BatchMessageHeader,
    pub messages: Vec<Vec<u8>>,
}

impl BatchMessage {
    pub fn serialize(&self) -> Result<Vec<u8>, SerializationError> {
        let mut buffer = Vec::new();
        
        // Serialize batch header
        let header_bytes = unsafe {
            std::slice::from_raw_parts(
                &self.header as *const _ as *const u8,
                std::mem::size_of::<BatchMessageHeader>()
            )
        };
        buffer.extend_from_slice(header_bytes);
        
        // Add individual messages
        for message in &self.messages {
            buffer.extend_from_slice(message);
        }
        
        // Compress if specified
        if self.header.compression != 0 {
            let compression_type = match self.header.compression {
                1 => CompressionType::LZ4,
                2 => CompressionType::Snappy,
                3 => CompressionType::Zstd,
                _ => CompressionType::None,
            };
            
            let payload_start = std::mem::size_of::<BatchMessageHeader>();
            let compressed = compress_message(&buffer[payload_start..], compression_type)?;
            buffer.truncate(payload_start);
            buffer.extend_from_slice(&compressed);
        }
        
        Ok(buffer)
    }
}
```

## Performance Optimizations

### Memory-Mapped Buffers
```rust
use memmap2::MmapMut;

pub struct MappedMessageBuffer {
    mmap: MmapMut,
    write_offset: AtomicUsize,
    capacity: usize,
}

impl MappedMessageBuffer {
    pub fn new(capacity: usize) -> Result<Self, std::io::Error> {
        let mmap = MmapMut::map_anon(capacity)?;
        Ok(Self {
            mmap,
            write_offset: AtomicUsize::new(0),
            capacity,
        })
    }
    
    pub fn write_message<T: BinarySerializable>(&self, message: &T) -> Result<usize, BufferError> {
        let size = message.binary_size();
        let offset = self.write_offset.fetch_add(size, Ordering::SeqCst);
        
        if offset + size > self.capacity {
            return Err(BufferError::OutOfSpace);
        }
        
        let mut buffer = Vec::with_capacity(size);
        message.serialize_binary(&mut buffer)?;
        
        unsafe {
            let dst = self.mmap.as_ptr().add(offset) as *mut u8;
            std::ptr::copy_nonoverlapping(buffer.as_ptr(), dst, size);
        }
        
        Ok(offset)
    }
    
    pub fn read_message<T: BinarySerializable>(&self, offset: usize) -> Result<T, BufferError> {
        if offset >= self.capacity {
            return Err(BufferError::InvalidOffset);
        }
        
        let slice = &self.mmap[offset..];
        let (message, _) = T::deserialize_binary(slice)?;
        Ok(message)
    }
}
```

### SIMD-Accelerated Checksum
```rust
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

pub fn simd_checksum(data: &[u8]) -> u32 {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        if is_x86_feature_detected!("avx2") {
            return checksum_avx2(data);
        }
    }
    
    // Fallback to standard CRC32
    crc32fast::hash(data)
}

#[cfg(target_arch = "x86_64")]
unsafe fn checksum_avx2(data: &[u8]) -> u32 {
    let mut crc = 0u32;
    let mut offset = 0;
    
    // Process 32 bytes at a time with AVX2
    while offset + 32 <= data.len() {
        let chunk = _mm256_loadu_si256(data.as_ptr().add(offset) as *const __m256i);
        // AVX2 CRC32 implementation
        // ... (implementation details)
        offset += 32;
    }
    
    // Process remaining bytes
    for byte in &data[offset..] {
        crc = _mm_crc32_u8(crc, *byte);
    }
    
    crc
}
```

## Schema Evolution

```rust
pub struct SchemaRegistry {
    schemas: DashMap<(MessageType, u8), MessageSchema>,
}

pub struct MessageSchema {
    pub version: u8,
    pub fields: Vec<FieldDefinition>,
    pub size: Option<usize>,               // Fixed size if Some
}

pub struct FieldDefinition {
    pub name: String,
    pub field_type: FieldType,
    pub offset: Option<usize>,             // Fixed offset if Some
    pub size: usize,
    pub required: bool,
    pub default_value: Option<Vec<u8>>,
}

impl SchemaRegistry {
    pub fn migrate_message(&self, 
        message: &[u8], 
        from_version: u8, 
        to_version: u8
    ) -> Result<Vec<u8>, MigrationError> {
        // Get source and target schemas
        let message_type = MessageType::from_byte(message[4])?;
        let from_schema = self.schemas.get(&(message_type, from_version))
            .ok_or(MigrationError::UnknownSchema)?;
        let to_schema = self.schemas.get(&(message_type, to_version))
            .ok_or(MigrationError::UnknownSchema)?;
        
        // Perform field-by-field migration
        let mut migrated = Vec::new();
        
        for field in &to_schema.fields {
            if let Some(source_field) = from_schema.fields.iter()
                .find(|f| f.name == field.name) {
                // Copy existing field
                let value = extract_field(message, source_field)?;
                migrated.extend_from_slice(&value);
            } else if let Some(ref default) = field.default_value {
                // Use default for new field
                migrated.extend_from_slice(default);
            } else if field.required {
                return Err(MigrationError::MissingRequiredField(field.name.clone()));
            }
        }
        
        Ok(migrated)
    }
}
```

## Network Protocol

### TCP/Unix Socket Transport
```rust
pub struct BinaryProtocolTransport {
    socket: UnixStream,
    read_buffer: Vec<u8>,
    write_buffer: Vec<u8>,
}

impl BinaryProtocolTransport {
    pub async fn send_message<T: BinarySerializable>(&mut self, message: &T) -> Result<(), TransportError> {
        self.write_buffer.clear();
        message.serialize_binary(&mut self.write_buffer)?;
        
        // Write message length prefix
        let len = self.write_buffer.len() as u32;
        self.socket.write_all(&len.to_le_bytes()).await?;
        
        // Write message
        self.socket.write_all(&self.write_buffer).await?;
        self.socket.flush().await?;
        
        Ok(())
    }
    
    pub async fn receive_message(&mut self) -> Result<Vec<u8>, TransportError> {
        // Read message length
        let mut len_bytes = [0u8; 4];
        self.socket.read_exact(&mut len_bytes).await?;
        let len = u32::from_le_bytes(len_bytes) as usize;
        
        // Validate message size
        if len > MAX_MESSAGE_SIZE {
            return Err(TransportError::MessageTooLarge(len));
        }
        
        // Read message
        self.read_buffer.resize(len, 0);
        self.socket.read_exact(&mut self.read_buffer).await?;
        
        Ok(self.read_buffer.clone())
    }
}
```
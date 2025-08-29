//! # Message Structure and Header Definitions
//!
//! ## Purpose
//!
//! Defines the core message envelope and header structures for Torq Protocol V2.
//! This module provides the foundational message format that wraps all TLV payloads,
//! ensuring consistent routing, validation, and sequencing across the entire system.
//!
//! ## Integration Points
//!
//! - **Message Envelope**: 32-byte header structure wrapping all TLV payloads
//! - **Protocol Validation**: Magic number and version verification
//! - **Service Attribution**: Source identification for debugging and metrics
//! - **Sequence Management**: Message ordering and recovery coordination
//! - **Relay Routing**: Domain-based routing to appropriate service handlers
//! - **Binary Transport**: Network and Unix socket serialization format
//!
//! ## Architecture Role
//!
//! ```text
//! Application Layer → [Message Header] → Transport Layer
//!                         ↓
//!     ┌─────────────────────────────────────┐
//!     │     32-Byte Message Header          │
//!     ├─────────────────────────────────────┤
//!     │ Magic (4) │ Version (1) │ Domain (1)│
//!     │ Source (1)│ Reserved (1)│ Seq (8)   │
//!     │ Timestamp (8) │ Checksum (4) │ ... │
//!     └─────────────────────────────────────┘
//!                         ↓
//!              [Variable TLV Payload]
//! ```
//!
//! ## Message Flow
//!
//! 1. **Construction**: Services create messages with appropriate headers
//! 2. **Validation**: Recipients verify magic number and checksum  
//! 3. **Attribution**: Source field identifies message producer
//! 4. **Routing**: Domain field directs to correct relay service
//! 5. **Sequencing**: Sequence numbers enable ordering and gap detection
//! 6. **Recovery**: Missing sequences trigger recovery protocols
//!
//! ## Performance Profile
//!
//! - **Header Size**: Fixed 32 bytes for cache-line alignment
//! - **Parsing Speed**: >2M headers/second (simple field extraction)
//! - **Validation Cost**: <5μs per message (magic + checksum verification)
//! - **Memory Layout**: Optimized for zero-copy operations
//! - **Network Efficiency**: Minimal overhead for high-frequency messages
//!
//! ## Header Field Details
//!
//! ### Magic Number (4 bytes)
//! - **Value**: `0xDEADBEEF`
//! - **Purpose**: Protocol validation and frame synchronization
//! - **Performance**: Fast rejection of malformed messages
//!
//! ### Version (1 byte)
//! - **Current**: Protocol Version 2
//! - **Purpose**: Enable protocol evolution and compatibility
//! - **Future**: Support for gradual protocol upgrades
//!
//! ### Domain (1 byte)
//! - **Values**: MarketData(1), Signal(2), Execution(3), System(4)
//! - **Purpose**: Route messages to appropriate relay services
//! - **Performance**: O(1) routing without payload inspection
//!
//! ### Source (1 byte)
//! - **Range**: BinanceCollector(1), ArbitrageStrategy(20), etc.
//! - **Purpose**: Message attribution for debugging and metrics
//! - **Monitoring**: Track message frequencies by source
//!
//! ### Sequence Number (8 bytes)
//! - **Scope**: Per-source monotonic sequence
//! - **Purpose**: Message ordering and gap detection
//! - **Recovery**: Trigger snapshot requests for missing messages
//!
//! ### Timestamp (8 bytes)
//! - **Format**: Nanoseconds since Unix epoch
//! - **Purpose**: End-to-end latency measurement and ordering
//! - **Precision**: Sub-microsecond timing for performance analysis
//!
//! ### Checksum (4 bytes)
//! - **Algorithm**: CRC32 over entire message (header + payload)
//! - **Purpose**: Detect transmission errors and corruption
//! - **Performance**: Hardware-accelerated on modern CPUs
//!
//! ## Message Lifecycle
//!
//! ### Construction
//! ```rust
//! use protocol_v2::message::{MessageHeader, MessageBuilder};
//! use protocol_v2::{RelayDomain, SourceType};
//!
//! let header = MessageBuilder::new()
//!     .domain(RelayDomain::MarketData)
//!     .source(SourceType::BinanceCollector)
//!     .sequence(sequence_counter.next())
//!     .build();
//!
//! let complete_message = header.wrap_payload(&tlv_payload);
//! ```
//!
//! ### Validation
//! ```rust
//! use protocol_v2::message::parse_header;
//!
//! let header = parse_header(&received_bytes)?;
//!
//! // Protocol validation
//! if header.magic != MESSAGE_MAGIC {
//!     return Err(MessageError::InvalidMagic(header.magic));
//! }
//!
//! // Version compatibility
//! if header.version > SUPPORTED_VERSION {
//!     return Err(MessageError::UnsupportedVersion(header.version));
//! }
//!
//! // Integrity verification
//! if !header.verify_checksum(&received_bytes) {
//!     return Err(MessageError::ChecksumMismatch);
//! }
//! ```
//!
//! ### Routing
//! ```rust
//! match header.domain {
//!     RelayDomain::MarketData => route_to_market_data_relay(message),
//!     RelayDomain::Signal => route_to_signal_relay(message),
//!     RelayDomain::Execution => route_to_execution_relay(message),
//!     RelayDomain::System => route_to_system_relay(message),
//! }
//! ```
//!
//! ## Sequence Management
//!
//! ### Gap Detection
//! ```rust
//! impl SequenceTracker {
//!     fn check_sequence(&mut self, header: &MessageHeader) -> SequenceStatus {
//!         let expected = self.next_sequence.get(&header.source).unwrap_or(0);
//!         
//!         match header.sequence.cmp(&expected) {
//!             Ordering::Equal => {
//!                 self.next_sequence.insert(header.source, expected + 1);
//!                 SequenceStatus::InOrder
//!             }
//!             Ordering::Greater => {
//!                 // Gap detected - request recovery
//!                 self.request_recovery(header.source, expected, header.sequence);
//!                 SequenceStatus::GapDetected { missing: expected..header.sequence }
//!             }
//!             Ordering::Less => SequenceStatus::Duplicate
//!         }
//!     }
//! }
//! ```
//!
//! ### Recovery Protocol
//! ```rust
//! // Request missing messages
//! let recovery_request = RecoveryRequestTLV {
//!     source: gap_source,
//!     start_sequence: gap_start,
//!     end_sequence: gap_end,
//!     timestamp: SystemTime::now(),
//! };
//!
//! system_relay.send_recovery_request(recovery_request).await?;
//! ```
//!
//! ## Performance Optimization
//!
//! ### Zero-Copy Parsing
//! ```rust
//! // Efficient header parsing without allocation
//! fn parse_header_fast(data: &[u8]) -> Result<&MessageHeader, ParseError> {
//!     if data.len() < MessageHeader::SIZE {
//!         return Err(ParseError::MessageTooShort);
//!     }
//!     
//!     unsafe {
//!         // Safe: we verified size and MessageHeader is repr(C, packed)
//!         Ok(&*(data.as_ptr() as *const MessageHeader))
//!     }
//! }
//! ```
//!
//! ### Batch Processing
//! ```rust
//! // Process multiple messages efficiently  
//! fn process_message_batch(batch: &[u8]) -> Result<Vec<ProcessedMessage>, Error> {
//!     let mut results = Vec::new();
//!     let mut offset = 0;
//!     
//!     while offset + MessageHeader::SIZE <= batch.len() {
//!         let header = parse_header(&batch[offset..])?;
//!         let payload_len = header.payload_length as usize;
//!         let total_len = MessageHeader::SIZE + payload_len;
//!         
//!         if offset + total_len > batch.len() {
//!             break; // Incomplete message
//!         }
//!         
//!         let payload = &batch[offset + MessageHeader::SIZE..offset + total_len];
//!         results.push(ProcessedMessage { header, payload });
//!         offset += total_len;
//!     }
//!     
//!     Ok(results)
//! }
//! ```
//!
//! ## Error Handling
//!
//! ### Comprehensive Error Types
//! ```rust
//! #[derive(Debug, Error)]
//! pub enum MessageError {
//!     #[error("Invalid magic number: {0:#x}")]
//!     InvalidMagic(u32),
//!     
//!     #[error("Unsupported protocol version: {0}")]
//!     UnsupportedVersion(u8),
//!     
//!     #[error("Checksum verification failed")]
//!     ChecksumMismatch,
//!     
//!     #[error("Message too short: need {need}, got {got}")]
//!     MessageTooShort { need: usize, got: usize },
//!     
//!     #[error("Sequence gap detected: expected {expected}, got {actual}")]
//!     SequenceGap { expected: u64, actual: u64 },
//! }
//! ```
//!
//! ### Recovery Strategies
//! ```rust
//! match message_error {
//!     MessageError::InvalidMagic(_) => {
//!         // Potential frame sync issue - scan for next valid header
//!         resync_stream(stream).await?;
//!     }
//!     MessageError::ChecksumMismatch => {
//!         // Request retransmission if possible
//!         request_retransmission(source, sequence).await?;
//!     }
//!     MessageError::SequenceGap { expected, actual } => {
//!         // Trigger recovery protocol for missing messages
//!         request_recovery(source, expected, actual).await?;
//!     }
//! }
//! ```
//!
//! ## Monitoring and Observability
//!
//! ### Message Metrics
//! ```rust
//! #[derive(Debug)]
//! pub struct MessageMetrics {
//!     pub messages_received: u64,
//!     pub messages_sent: u64,
//!     pub checksum_failures: u64,
//!     pub sequence_gaps: u64,
//!     pub average_latency_ns: u64,
//!     pub per_source_stats: HashMap<SourceType, SourceStats>,
//! }
//!
//! impl MessageMetrics {
//!     pub fn record_message(&mut self, header: &MessageHeader) {
//!         self.messages_received += 1;
//!         
//!         let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64;
//!         let latency = now.saturating_sub(header.timestamp_ns);
//!         
//!         // Update rolling average
//!         self.average_latency_ns = (self.average_latency_ns * 15 + latency) / 16;
//!         
//!         // Per-source tracking
//!         let source_stats = self.per_source_stats.entry(header.source).or_default();
//!         source_stats.message_count += 1;
//!         source_stats.last_sequence = header.sequence;
//!     }
//! }
//! ```
//!
//! ### Health Monitoring
//! ```rust
//! // Detect unhealthy message patterns
//! if metrics.checksum_failures > 0.01 * metrics.messages_received {
//!     alert!("High checksum failure rate: {}%",
//!            100.0 * metrics.checksum_failures as f64 / metrics.messages_received as f64);
//! }
//!
//! if metrics.average_latency_ns > 1_000_000 { // 1ms
//!     warn!("High message latency: {}μs", metrics.average_latency_ns / 1000);
//! }
//! ```
//!
//! ## Best Practices
//!
//! ### Message Construction
//! 1. **Use MessageBuilder** for consistent header creation
//! 2. **Include accurate timestamps** for latency measurement
//! 3. **Maintain sequence numbers** per source for ordering
//! 4. **Choose appropriate domains** for correct routing
//!
//! ### Performance
//! 1. **Batch messages** when possible to amortize overhead
//! 2. **Pre-allocate buffers** for message construction
//! 3. **Use zero-copy parsing** in hot paths
//! 4. **Monitor message frequencies** to detect anomalies
//!
//! ### Reliability
//! 1. **Always verify checksums** in production
//! 2. **Implement recovery protocols** for sequence gaps
//! 3. **Handle version mismatches** gracefully
//! 4. **Log message statistics** for operational visibility
//!
//! ## Module Structure
//!
//! - [`header`] - Core message header definitions and parsing functions

pub mod header;

pub use header::*;

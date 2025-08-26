# Task PERF-001: Implement Checksum Validation Sampling
*Agent Type: Performance Optimization Specialist*
*Branch: `fix/checksum-sampling`*
*Priority: 🔴 CRITICAL - Performance Blocker*

## 📋 Your Mission
Implement checksum validation sampling to maintain <35μs hot path performance while still catching corruption.

## 🎯 Context
Currently validating checksums on EVERY message is killing performance. We need to sample (e.g., every 100th message) to maintain speed while still detecting issues.

## 🔧 Git Setup Instructions

```bash
# Step 1: Start fresh from main
git checkout main
git pull origin main

# Step 2: Create your feature branch
git checkout -b fix/checksum-sampling

# Step 3: Confirm branch
git branch --show-current  # Should show: fix/checksum-sampling
```

## 📝 Task Specification

### Files to Modify
1. `relays/src/relay_consumer.rs` - Add sampling logic
2. `relays/src/config.rs` - Add configurable sample rate

### Required Implementation

#### Step 1: Add Sampling Configuration
```rust
// In relays/src/config.rs

#[derive(Debug, Clone, Deserialize)]
pub struct RelayConfig {
    // ... existing fields ...

    /// How often to validate checksums (1 = every message, 100 = every 100th)
    #[serde(default = "default_checksum_sample_rate")]
    pub checksum_sample_rate: u32,

    /// Whether to log checksum failures (for debugging)
    #[serde(default = "default_log_checksum_failures")]
    pub log_checksum_failures: bool,
}

fn default_checksum_sample_rate() -> u32 {
    100 // Validate every 100th message by default
}

fn default_log_checksum_failures() -> bool {
    true // Log failures in production for monitoring
}
```

#### Step 2: Implement Sampling in RelayConsumer
```rust
// In relays/src/relay_consumer.rs

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

pub struct RelayConsumer {
    // ... existing fields ...

    // ADD THESE:
    message_counter: AtomicU64,
    checksum_sample_rate: u32,
    checksums_validated: AtomicU64,
    checksum_failures: AtomicU64,
}

impl RelayConsumer {
    pub fn new(config: RelayConfig) -> Self {
        Self {
            // ... existing fields ...
            message_counter: AtomicU64::new(0),
            checksum_sample_rate: config.checksum_sample_rate,
            checksums_validated: AtomicU64::new(0),
            checksum_failures: AtomicU64::new(0),
        }
    }

    /// Parse header with sampling-based checksum validation
    fn parse_header_with_sampling(&self, data: &[u8]) -> Result<&MessageHeader> {
        // Increment message counter
        let msg_count = self.message_counter.fetch_add(1, Ordering::Relaxed);

        // Quick validation: always check magic and size
        if data.len() < 32 {
            return Err(ParseError::InsufficientData);
        }

        let header = MessageHeader::ref_from_prefix(data)
            .ok_or(ParseError::InvalidHeader)?;

        // Always validate magic byte (fast check)
        if header.magic != 0xDEADBEEF {
            return Err(ParseError::InvalidMagic);
        }

        // Sample-based checksum validation
        if msg_count % self.checksum_sample_rate as u64 == 0 {
            self.validate_checksum_sampled(header, data)?;
        }

        Ok(header)
    }

    /// Validate checksum for sampled messages
    fn validate_checksum_sampled(
        &self,
        header: &MessageHeader,
        data: &[u8]
    ) -> Result<()> {
        self.checksums_validated.fetch_add(1, Ordering::Relaxed);

        // Calculate expected checksum
        let payload_end = 32 + header.payload_size as usize;
        if data.len() < payload_end {
            return Err(ParseError::TruncatedPayload);
        }

        let calculated = calculate_checksum(&data[32..payload_end]);

        if calculated != header.checksum {
            self.checksum_failures.fetch_add(1, Ordering::Relaxed);

            if self.config.log_checksum_failures {
                error!(
                    "Checksum validation failed! Expected: {}, Got: {}, Message: {}",
                    header.checksum,
                    calculated,
                    self.message_counter.load(Ordering::Relaxed)
                );
            }

            // In production, we might want to continue despite failure
            // and just increment metrics
            if self.config.strict_checksum_validation {
                return Err(ParseError::ChecksumMismatch);
            }
        }

        Ok(())
    }

    /// Get sampling metrics
    pub fn get_checksum_metrics(&self) -> ChecksumMetrics {
        ChecksumMetrics {
            total_messages: self.message_counter.load(Ordering::Relaxed),
            checksums_validated: self.checksums_validated.load(Ordering::Relaxed),
            checksum_failures: self.checksum_failures.load(Ordering::Relaxed),
            sample_rate: self.checksum_sample_rate,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChecksumMetrics {
    pub total_messages: u64,
    pub checksums_validated: u64,
    pub checksum_failures: u64,
    pub sample_rate: u32,
}

impl ChecksumMetrics {
    pub fn validation_percentage(&self) -> f64 {
        if self.total_messages == 0 {
            0.0
        } else {
            (self.checksums_validated as f64 / self.total_messages as f64) * 100.0
        }
    }

    pub fn failure_rate(&self) -> f64 {
        if self.checksums_validated == 0 {
            0.0
        } else {
            (self.checksum_failures as f64 / self.checksums_validated as f64) * 100.0
        }
    }
}
```

#### Step 3: Update Message Processing
```rust
// Update the main message processing loop

impl RelayConsumer {
    pub async fn process_messages(&mut self) -> Result<()> {
        while let Some(data) = self.receive_message().await? {
            // Use sampling-based parsing
            let header = self.parse_header_with_sampling(&data)?;

            // Rest of processing remains the same
            self.process_tlv_message(header, &data[32..])?;

            // Periodically log metrics (every 10,000 messages)
            if self.message_counter.load(Ordering::Relaxed) % 10_000 == 0 {
                let metrics = self.get_checksum_metrics();
                info!(
                    "Checksum metrics: {} total, {:.2}% validated, {:.2}% failures",
                    metrics.total_messages,
                    metrics.validation_percentage(),
                    metrics.failure_rate()
                );
            }
        }

        Ok(())
    }
}
```

## ✅ Acceptance Criteria

1. **Performance Requirements**
   - [ ] Hot path maintains <35μs with sampling
   - [ ] Full validation only on sampled messages
   - [ ] No performance regression for non-sampled messages

2. **Validation Coverage**
   - [ ] Configurable sample rate (default 100)
   - [ ] Magic byte ALWAYS validated (fast check)
   - [ ] Metrics track validation rate and failures
   - [ ] Checksum failures logged but don't crash

3. **Configuration**
   - [ ] Sample rate configurable via config file
   - [ ] Can disable sampling for debugging (rate = 1)
   - [ ] Metrics exposed for monitoring

## 🧪 Testing Instructions

```bash
# Performance benchmark
cargo bench --package relays checksum_sampling

# Unit tests
cargo test --package relays checksum_validation

# Load test with high message rate
cargo run --release --bin relay_load_test -- --messages 1000000

# Verify metrics
cargo run --bin relay_consumer -- --show-metrics
```

## 📤 Commit & Push Instructions

```bash
# Stage changes
git add relays/src/relay_consumer.rs
git add relays/src/config.rs

# Commit
git commit -m "perf(relay): implement checksum validation sampling

- Add configurable sampling rate (default 1/100)
- Maintain <35μs hot path performance
- Track validation metrics for monitoring
- Critical performance fix for production throughput"

# Push
git push -u origin fix/checksum-sampling
```

## 🔄 Pull Request Template

```markdown
## Task PERF-001: Checksum Validation Sampling

### Summary
Implemented sampling-based checksum validation to maintain <35μs hot path while still detecting corruption.

### Performance Impact
- Before: 150μs per message (with full validation)
- After: 32μs per message (with 1/100 sampling)
- ✅ Meets <35μs target

### Implementation
- Configurable sample rate (default 100)
- Magic byte always validated (fast check)
- Metrics tracking for monitoring
- Graceful handling of failures

### Testing
- [x] Performance benchmarks pass
- [x] 1M message load test successful
- [x] Metrics correctly tracked
- [x] No message loss
```

## ⚠️ Important Notes

1. **Magic byte** is ALWAYS checked (4-byte comparison is fast)
2. **Sample rate** of 100 means 1% validation coverage
3. **Metrics** are critical for monitoring health
4. **Checksum failures** should be rare - investigate if >0.01%
5. **Hot path** must stay under 35μs for >1M msg/s target

## 🤝 Coordination
- Independent task - no dependencies
- Critical for maintaining throughput
- Monitor metrics after deployment

---
*Remember: Every microsecond counts in the hot path!*

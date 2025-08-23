//! Consumer Registry for Per-Consumer Sequence Tracking
//! 
//! Implements the consumer registration and sequence tracking requirements
//! from PROTOCOL.md for gap detection and recovery.

use super::{ConsumerId, RecoveryRequest, RecoveryRequestType};
use crate::RelayDomain;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use tracing::{info, warn, debug};

/// Per-consumer sequence tracking and recovery management
#[derive(Debug)]
pub struct ConsumerRegistry {
    consumers: HashMap<ConsumerId, ConsumerState>,
    domain: RelayDomain,
    global_sequence: u64,
    recovery_threshold: u64, // Gap size that triggers snapshot vs retransmit
}

/// State tracking for individual consumers
#[derive(Debug, Clone)]
pub struct ConsumerState {
    pub last_sequence: u64,
    pub expected_next: u64,
    pub gap_count: u64,
    pub last_seen: SystemTime,
    pub recovery_state: RecoveryState,
    pub total_messages: u64,
    pub recovery_requests: u64,
}

/// Recovery state for a consumer
#[derive(Debug, Clone)]
pub enum RecoveryState {
    Normal,
    RecoveryRequested { 
        from_sequence: u64, 
        to_sequence: u64,
        request_type: RecoveryRequestType,
        requested_at: SystemTime,
    },
    SnapshotPending {
        snapshot_sequence: u64,
        requested_at: SystemTime,
    },
}

/// Consumer status for monitoring
#[derive(Debug, Clone)]
pub struct ConsumerStatus {
    pub consumer_id: ConsumerId,
    pub last_sequence: u64,
    pub expected_next: u64,
    pub gap_count: u64,
    pub total_messages: u64,
    pub is_recovering: bool,
    pub last_seen_seconds_ago: u64,
    pub recovery_requests: u64,
}

impl ConsumerRegistry {
    /// Create new consumer registry for a domain
    pub fn new(domain: RelayDomain) -> Self {
        let recovery_threshold = match domain {
            RelayDomain::MarketData => 50,   // Market data - small threshold for speed
            RelayDomain::Signal => 100,      // Signals - moderate threshold for balance
            RelayDomain::Execution => 10,    // Execution - small threshold for safety
        };
        
        Self {
            consumers: HashMap::new(),
            domain,
            global_sequence: 1,
            recovery_threshold,
        }
    }
    
    /// Register a new consumer
    pub fn register_consumer(&mut self, consumer_id: ConsumerId) -> Result<u64, String> {
        if self.consumers.contains_key(&consumer_id) {
            warn!("Consumer {:?} already registered, updating registration", consumer_id);
        }
        
        let state = ConsumerState {
            last_sequence: 0,
            expected_next: self.global_sequence,
            gap_count: 0,
            last_seen: SystemTime::now(),
            recovery_state: RecoveryState::Normal,
            total_messages: 0,
            recovery_requests: 0,
        };
        
        self.consumers.insert(consumer_id.clone(), state);
        info!("Registered consumer {:?} for domain {:?} starting at sequence {}", 
              consumer_id, self.domain, self.global_sequence);
        
        Ok(self.global_sequence)
    }
    
    /// Update consumer sequence and detect gaps
    pub fn update_consumer_sequence(
        &mut self, 
        consumer_id: &ConsumerId, 
        sequence: u64
    ) -> Option<RecoveryRequest> {
        let now = SystemTime::now();
        
        if let Some(state) = self.consumers.get_mut(consumer_id) {
            state.last_seen = now;
            state.total_messages += 1;
            
            // Check for sequence gap
            if sequence != state.expected_next {
                // Gap detected!
                state.gap_count += 1;
                state.recovery_requests += 1;
                
                let gap_size = sequence - state.expected_next;
                warn!("Gap detected for {:?}: expected {}, got {} (gap size: {})", 
                      consumer_id, state.expected_next, sequence, gap_size);
                
                // Determine recovery type based on gap size and domain policy
                let request_type = if gap_size > self.recovery_threshold {
                    RecoveryRequestType::Snapshot
                } else {
                    RecoveryRequestType::Retransmit
                };
                
                // Update recovery state
                state.recovery_state = RecoveryState::RecoveryRequested {
                    from_sequence: state.expected_next,
                    to_sequence: sequence - 1,
                    request_type,
                    requested_at: now,
                };
                
                return Some(RecoveryRequest {
                    consumer_id: consumer_id.clone(),
                    start_sequence: state.expected_next,
                    end_sequence: sequence - 1,
                    request_type,
                });
            }
            
            // Normal sequence progression
            state.last_sequence = sequence;
            state.expected_next = sequence + 1;
            
            // Clear recovery state if we're back to normal
            if matches!(state.recovery_state, RecoveryState::RecoveryRequested { .. }) {
                info!("Consumer {:?} back to normal sequence progression", consumer_id);
                state.recovery_state = RecoveryState::Normal;
            }
        } else {
            warn!("Unregistered consumer {:?} sent sequence {}", consumer_id, sequence);
            // Auto-register and start from current sequence
            let _ = self.register_consumer(consumer_id.clone());
        }
        
        None
    }
    
    /// Mark recovery as completed for a consumer
    pub fn mark_recovery_completed(&mut self, consumer_id: &ConsumerId, up_to_sequence: u64) {
        if let Some(state) = self.consumers.get_mut(consumer_id) {
            state.last_sequence = up_to_sequence;
            state.expected_next = up_to_sequence + 1;
            state.recovery_state = RecoveryState::Normal;
            
            info!("Recovery completed for consumer {:?} up to sequence {}", 
                  consumer_id, up_to_sequence);
        }
    }
    
    /// Get status for a specific consumer
    pub fn get_consumer_status(&self, consumer_id: &ConsumerId) -> Option<ConsumerStatus> {
        self.consumers.get(consumer_id).map(|state| {
            let last_seen_seconds_ago = state.last_seen
                .elapsed()
                .unwrap_or(Duration::from_secs(0))
                .as_secs();
            
            ConsumerStatus {
                consumer_id: consumer_id.clone(),
                last_sequence: state.last_sequence,
                expected_next: state.expected_next,
                gap_count: state.gap_count,
                total_messages: state.total_messages,
                is_recovering: !matches!(state.recovery_state, RecoveryState::Normal),
                last_seen_seconds_ago,
                recovery_requests: state.recovery_requests,
            }
        })
    }
    
    /// Get status for all consumers
    pub fn get_all_consumer_status(&self) -> Vec<ConsumerStatus> {
        self.consumers
            .iter()
            .map(|(id, state)| {
                let last_seen_seconds_ago = state.last_seen
                    .elapsed()
                    .unwrap_or(Duration::from_secs(0))
                    .as_secs();
                
                ConsumerStatus {
                    consumer_id: id.clone(),
                    last_sequence: state.last_sequence,
                    expected_next: state.expected_next,
                    gap_count: state.gap_count,
                    total_messages: state.total_messages,
                    is_recovering: !matches!(state.recovery_state, RecoveryState::Normal),
                    last_seen_seconds_ago,
                    recovery_requests: state.recovery_requests,
                }
            })
            .collect()
    }
    
    /// Remove inactive consumers (cleanup)
    pub fn cleanup_inactive_consumers(&mut self, inactive_threshold_secs: u64) -> usize {
        let threshold = Duration::from_secs(inactive_threshold_secs);
        let now = SystemTime::now();
        
        let initial_count = self.consumers.len();
        
        self.consumers.retain(|consumer_id, state| {
            if now.duration_since(state.last_seen).unwrap_or(Duration::from_secs(0)) > threshold {
                info!("Removing inactive consumer {:?} (last seen {} seconds ago)", 
                      consumer_id, 
                      now.duration_since(state.last_seen).unwrap_or(Duration::from_secs(0)).as_secs());
                false
            } else {
                true
            }
        });
        
        let removed_count = initial_count - self.consumers.len();
        if removed_count > 0 {
            info!("Cleaned up {} inactive consumers", removed_count);
        }
        
        removed_count
    }
    
    /// Get consumers that need recovery attention
    pub fn get_consumers_needing_recovery(&self) -> Vec<(ConsumerId, RecoveryState)> {
        self.consumers
            .iter()
            .filter_map(|(id, state)| {
                match &state.recovery_state {
                    RecoveryState::Normal => None,
                    recovery_state => Some((id.clone(), recovery_state.clone())),
                }
            })
            .collect()
    }
    
    /// Update global sequence (called when relay processes messages)
    pub fn update_global_sequence(&mut self, sequence: u64) {
        if sequence > self.global_sequence {
            self.global_sequence = sequence;
        }
    }
    
    /// Get current global sequence
    pub fn current_global_sequence(&self) -> u64 {
        self.global_sequence
    }
    
    /// Get registry statistics
    pub fn get_registry_stats(&self) -> ConsumerRegistryStats {
        let total_consumers = self.consumers.len();
        let active_consumers = self.consumers.iter()
            .filter(|(_, state)| {
                state.last_seen.elapsed().unwrap_or(Duration::from_secs(u64::MAX)).as_secs() < 300
            })
            .count();
        
        let consumers_in_recovery = self.consumers.iter()
            .filter(|(_, state)| !matches!(state.recovery_state, RecoveryState::Normal))
            .count();
        
        let total_gaps = self.consumers.values().map(|s| s.gap_count).sum();
        let total_recovery_requests = self.consumers.values().map(|s| s.recovery_requests).sum();
        
        ConsumerRegistryStats {
            domain: self.domain,
            total_consumers,
            active_consumers,
            consumers_in_recovery,
            global_sequence: self.global_sequence,
            recovery_threshold: self.recovery_threshold,
            total_gaps,
            total_recovery_requests,
        }
    }
}

/// Statistics for the consumer registry
#[derive(Debug, Clone)]
pub struct ConsumerRegistryStats {
    pub domain: RelayDomain,
    pub total_consumers: usize,
    pub active_consumers: usize,
    pub consumers_in_recovery: usize,
    pub global_sequence: u64,
    pub recovery_threshold: u64,
    pub total_gaps: u64,
    pub total_recovery_requests: u64,
}

impl ConsumerRegistryStats {
    pub fn health_report(&self) -> String {
        let health_score = if self.total_consumers == 0 {
            100.0
        } else {
            ((self.total_consumers - self.consumers_in_recovery) as f64 / self.total_consumers as f64) * 100.0
        };
        
        format!(
            "Consumer Registry Health Report ({:?}):\n\
             📊 Total Consumers: {}\n\
             🟢 Active Consumers: {}\n\
             🔄 In Recovery: {}\n\
             📈 Global Sequence: {}\n\
             ⚖️  Recovery Threshold: {}\n\
             ❌ Total Gaps: {}\n\
             🔧 Total Recovery Requests: {}\n\
             💚 Health Score: {:.1}%",
            self.domain,
            self.total_consumers,
            self.active_consumers,
            self.consumers_in_recovery,
            self.global_sequence,
            self.recovery_threshold,
            self.total_gaps,
            self.total_recovery_requests,
            health_score
        )
    }
    
    pub fn is_healthy(&self) -> bool {
        // Healthy if <10% of consumers are in recovery
        if self.total_consumers == 0 {
            return true;
        }
        (self.consumers_in_recovery as f64 / self.total_consumers as f64) < 0.1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    
    #[test]
    fn test_consumer_registration() {
        let mut registry = ConsumerRegistry::new(RelayDomain::MarketData);
        let consumer = ConsumerId::new("test_consumer", 1);
        
        let start_sequence = registry.register_consumer(consumer.clone()).unwrap();
        assert_eq!(start_sequence, 1);
        
        // Check status
        let status = registry.get_consumer_status(&consumer).unwrap();
        assert_eq!(status.expected_next, 1);
        assert_eq!(status.gap_count, 0);
    }
    
    #[test]
    fn test_sequence_gap_detection() {
        let mut registry = ConsumerRegistry::new(RelayDomain::Signal);
        let consumer = ConsumerId::new("test_consumer", 1);
        
        registry.register_consumer(consumer.clone()).unwrap();
        
        // Normal sequence
        let result = registry.update_consumer_sequence(&consumer, 1);
        assert!(result.is_none());
        
        // Gap detected
        let result = registry.update_consumer_sequence(&consumer, 5);
        assert!(result.is_some());
        
        let recovery_req = result.unwrap();
        assert_eq!(recovery_req.start_sequence, 2);
        assert_eq!(recovery_req.end_sequence, 4);
        assert_eq!(recovery_req.request_type, RecoveryRequestType::Retransmit);
        
        // Check status
        let status = registry.get_consumer_status(&consumer).unwrap();
        assert_eq!(status.gap_count, 1);
        assert!(status.is_recovering);
    }
    
    #[test]
    fn test_recovery_threshold() {
        let mut registry = ConsumerRegistry::new(RelayDomain::Execution);
        let consumer = ConsumerId::new("test_consumer", 1);
        
        registry.register_consumer(consumer.clone()).unwrap();
        
        // Large gap should trigger snapshot
        let result = registry.update_consumer_sequence(&consumer, 50);
        let recovery_req = result.unwrap();
        assert_eq!(recovery_req.request_type, RecoveryRequestType::Snapshot);
    }
    
    #[test]
    fn test_recovery_completion() {
        let mut registry = ConsumerRegistry::new(RelayDomain::MarketData);
        let consumer = ConsumerId::new("test_consumer", 1);
        
        registry.register_consumer(consumer.clone()).unwrap();
        
        // Create gap
        let _result = registry.update_consumer_sequence(&consumer, 10);
        
        // Mark recovery completed
        registry.mark_recovery_completed(&consumer, 9);
        
        let status = registry.get_consumer_status(&consumer).unwrap();
        assert_eq!(status.expected_next, 10);
        assert!(!status.is_recovering);
    }
    
    #[test]
    fn test_cleanup_inactive_consumers() {
        let mut registry = ConsumerRegistry::new(RelayDomain::MarketData);
        let consumer = ConsumerId::new("test_consumer", 1);
        
        registry.register_consumer(consumer.clone()).unwrap();
        
        // Consumer should be active initially
        assert_eq!(registry.consumers.len(), 1);
        
        // Should not clean up active consumer
        let removed = registry.cleanup_inactive_consumers(1);
        assert_eq!(removed, 0);
        assert_eq!(registry.consumers.len(), 1);
        
        // Manually set last_seen to past (simulating inactive consumer)
        if let Some(state) = registry.consumers.get_mut(&consumer) {
            state.last_seen = SystemTime::now() - Duration::from_secs(3600); // 1 hour ago
        }
        
        // Now should clean up
        let removed = registry.cleanup_inactive_consumers(1800); // 30 minutes
        assert_eq!(removed, 1);
        assert_eq!(registry.consumers.len(), 0);
    }
    
    #[test]
    fn test_registry_stats() {
        let mut registry = ConsumerRegistry::new(RelayDomain::Signal);
        let consumer1 = ConsumerId::new("consumer1", 1);
        let consumer2 = ConsumerId::new("consumer2", 1);
        
        registry.register_consumer(consumer1.clone()).unwrap();
        registry.register_consumer(consumer2.clone()).unwrap();
        
        // Create a gap for one consumer
        let _result = registry.update_consumer_sequence(&consumer1, 10);
        
        let stats = registry.get_registry_stats();
        assert_eq!(stats.total_consumers, 2);
        assert_eq!(stats.consumers_in_recovery, 1);
        assert_eq!(stats.domain, RelayDomain::Signal);
        
        let health_report = stats.health_report();
        assert!(health_report.contains("Total Consumers: 2"));
        assert!(health_report.contains("In Recovery: 1"));
    }
}
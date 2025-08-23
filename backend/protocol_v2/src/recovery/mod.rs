//! Recovery Protocol Implementation
//! 
//! Handles sequence gaps and provides snapshot-based recovery for consumers

pub mod request;
pub mod snapshot;

pub use request::*;
pub use snapshot::*;

use std::collections::VecDeque;

/// Recovery-related errors
#[derive(Debug, thiserror::Error)]
pub enum RecoveryError {
    #[error("Sequence gap detected: expected {expected}, got {actual}")]
    SequenceGap { expected: u64, actual: u64 },
    
    #[error("Recovery request failed: {reason}")]
    RequestFailed { reason: String },
    
    #[error("Snapshot too large: {size} bytes")]
    SnapshotTooLarge { size: usize },
    
    #[error("Snapshot decompression failed: {error}")]
    DecompressionFailed { error: String },
    
    #[error("Invalid recovery state")]
    InvalidState,
}

/// Recovery strategy based on gap size
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RecoveryStrategy {
    Retransmit,  // For small gaps (<100 messages)
    Snapshot,    // For large gaps (>=100 messages)
}

impl RecoveryStrategy {
    /// Choose recovery strategy based on gap size
    pub fn choose(gap_size: u64) -> Self {
        if gap_size < 100 {
            Self::Retransmit
        } else {
            Self::Snapshot
        }
    }
}

/// Sequence gap detector and recovery manager
pub struct SequenceTracker {
    expected_sequence: u64,
    recovery_window: VecDeque<u64>, // Recent sequence numbers
    window_size: usize,
    max_gap_size: u64,
}

impl SequenceTracker {
    /// Create a new sequence tracker
    pub fn new(initial_sequence: u64, window_size: usize, max_gap_size: u64) -> Self {
        Self {
            expected_sequence: initial_sequence,
            recovery_window: VecDeque::with_capacity(window_size),
            window_size,
            max_gap_size,
        }
    }
    
    /// Process a new message sequence number
    /// Returns Ok(()) for normal processing, Err for gap detection
    pub fn process_sequence(&mut self, sequence: u64) -> Result<(), RecoveryError> {
        if sequence == self.expected_sequence {
            // Normal case: sequence is as expected
            self.advance_sequence(sequence);
            Ok(())
        } else if sequence > self.expected_sequence {
            // Gap detected
            let gap_size = sequence - self.expected_sequence;
            if gap_size > self.max_gap_size {
                return Err(RecoveryError::RequestFailed {
                    reason: format!("Gap too large: {}", gap_size)
                });
            }
            
            Err(RecoveryError::SequenceGap {
                expected: self.expected_sequence,
                actual: sequence,
            })
        } else {
            // Duplicate or out-of-order message
            if self.recovery_window.contains(&sequence) {
                // This is a duplicate from recent history, ignore
                Ok(())
            } else {
                // Very old message, likely an error
                Err(RecoveryError::InvalidState)
            }
        }
    }
    
    /// Advance to the next expected sequence
    fn advance_sequence(&mut self, current: u64) {
        self.expected_sequence = current + 1;
        
        // Update recovery window
        self.recovery_window.push_back(current);
        if self.recovery_window.len() > self.window_size {
            self.recovery_window.pop_front();
        }
    }
    
    /// Get the current expected sequence number
    pub fn expected_sequence(&self) -> u64 {
        self.expected_sequence
    }
    
    /// Get the last processed sequence number
    pub fn last_sequence(&self) -> u64 {
        self.expected_sequence.saturating_sub(1)
    }
    
    /// Reset to a specific sequence (used after snapshot recovery)
    pub fn reset_to_sequence(&mut self, sequence: u64) {
        self.expected_sequence = sequence;
        self.recovery_window.clear();
    }
    
    /// Check if a sequence number is in the recent recovery window
    pub fn is_recent(&self, sequence: u64) -> bool {
        self.recovery_window.contains(&sequence)
    }
}

/// Recovery state machine for consumers
#[derive(Debug, Clone, PartialEq)]
pub enum RecoveryState {
    Normal,           // Processing messages normally
    GapDetected {     // Gap detected, recovery needed
        gap_start: u64,
        gap_end: u64,
        strategy: RecoveryStrategy,
    },
    RecoveryRequested, // Recovery request sent, waiting for response
    SnapshotLoading,   // Loading snapshot data
    Resynchronizing,   // Applying snapshot and resuming
}

/// Recovery manager for message consumers
pub struct RecoveryManager {
    state: RecoveryState,
    sequence_tracker: SequenceTracker,
    retry_count: u32,
    max_retries: u32,
}

impl RecoveryManager {
    /// Create a new recovery manager
    pub fn new(initial_sequence: u64, max_retries: u32) -> Self {
        Self {
            state: RecoveryState::Normal,
            sequence_tracker: SequenceTracker::new(initial_sequence, 1000, 10000),
            retry_count: 0,
            max_retries,
        }
    }
    
    /// Process a message and handle recovery if needed
    pub fn process_message(&mut self, sequence: u64) -> Result<RecoveryAction, RecoveryError> {
        match self.state {
            RecoveryState::Normal => {
                match self.sequence_tracker.process_sequence(sequence) {
                    Ok(()) => Ok(RecoveryAction::ProcessNormally),
                    Err(RecoveryError::SequenceGap { expected, actual }) => {
                        let strategy = RecoveryStrategy::choose(actual - expected);
                        self.state = RecoveryState::GapDetected {
                            gap_start: expected,
                            gap_end: actual,
                            strategy,
                        };
                        Ok(RecoveryAction::InitiateRecovery { strategy })
                    }
                    Err(e) => Err(e),
                }
            }
            RecoveryState::RecoveryRequested => {
                // Still waiting for recovery response, buffer this message
                Ok(RecoveryAction::BufferMessage)
            }
            RecoveryState::SnapshotLoading => {
                // Loading snapshot, continue buffering
                Ok(RecoveryAction::BufferMessage)
            }
            RecoveryState::Resynchronizing => {
                // Check if we've caught up
                if sequence >= self.sequence_tracker.expected_sequence() {
                    self.state = RecoveryState::Normal;
                    self.sequence_tracker.process_sequence(sequence)?;
                    Ok(RecoveryAction::ProcessNormally)
                } else {
                    Ok(RecoveryAction::BufferMessage)
                }
            }
            RecoveryState::GapDetected { .. } => {
                // Recovery should have been initiated, continue buffering
                Ok(RecoveryAction::BufferMessage)
            }
        }
    }
    
    /// Handle successful recovery request
    pub fn recovery_initiated(&mut self) {
        if matches!(self.state, RecoveryState::GapDetected { .. }) {
            self.state = RecoveryState::RecoveryRequested;
        }
    }
    
    /// Handle snapshot loading
    pub fn snapshot_received(&mut self) {
        if matches!(self.state, RecoveryState::RecoveryRequested) {
            self.state = RecoveryState::SnapshotLoading;
        }
    }
    
    /// Handle snapshot application and resync to new sequence
    pub fn snapshot_applied(&mut self, new_sequence: u64) {
        self.sequence_tracker.reset_to_sequence(new_sequence);
        self.state = RecoveryState::Resynchronizing;
        self.retry_count = 0;
    }
    
    /// Handle recovery failure
    pub fn recovery_failed(&mut self) -> Result<RecoveryAction, RecoveryError> {
        self.retry_count += 1;
        if self.retry_count >= self.max_retries {
            return Err(RecoveryError::RequestFailed {
                reason: "Max retries exceeded".to_string()
            });
        }
        
        // Reset to gap detected state for retry
        self.state = RecoveryState::GapDetected {
            gap_start: self.sequence_tracker.expected_sequence(),
            gap_end: self.sequence_tracker.expected_sequence(),
            strategy: RecoveryStrategy::Snapshot, // Escalate to snapshot on retry
        };
        
        Ok(RecoveryAction::InitiateRecovery { 
            strategy: RecoveryStrategy::Snapshot 
        })
    }
    
    /// Get current recovery state
    pub fn state(&self) -> &RecoveryState {
        &self.state
    }
    
    /// Check if recovery is in progress
    pub fn is_recovering(&self) -> bool {
        !matches!(self.state, RecoveryState::Normal)
    }
}

/// Actions to take based on recovery state
#[derive(Debug, Clone, PartialEq)]
pub enum RecoveryAction {
    ProcessNormally,                           // Process message normally
    BufferMessage,                            // Buffer message until recovery completes
    InitiateRecovery { strategy: RecoveryStrategy }, // Start recovery process
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sequence_tracker_normal_operation() {
        let mut tracker = SequenceTracker::new(1, 100, 1000);
        
        // Normal sequence progression
        assert!(tracker.process_sequence(1).is_ok());
        assert!(tracker.process_sequence(2).is_ok());
        assert!(tracker.process_sequence(3).is_ok());
        
        assert_eq!(tracker.expected_sequence(), 4);
        assert_eq!(tracker.last_sequence(), 3);
    }
    
    #[test]
    fn test_sequence_gap_detection() {
        let mut tracker = SequenceTracker::new(1, 100, 1000);
        
        assert!(tracker.process_sequence(1).is_ok());
        
        // Gap: expecting 2, got 5
        let result = tracker.process_sequence(5);
        assert!(result.is_err());
        
        if let Err(RecoveryError::SequenceGap { expected, actual }) = result {
            assert_eq!(expected, 2);
            assert_eq!(actual, 5);
        } else {
            panic!("Expected SequenceGap error");
        }
    }
    
    #[test]
    fn test_duplicate_message_handling() {
        let mut tracker = SequenceTracker::new(1, 100, 1000);
        
        assert!(tracker.process_sequence(1).is_ok());
        assert!(tracker.process_sequence(2).is_ok());
        
        // Duplicate message should be ignored
        assert!(tracker.process_sequence(1).is_ok());
    }
    
    #[test]
    fn test_recovery_strategy_selection() {
        assert_eq!(RecoveryStrategy::choose(50), RecoveryStrategy::Retransmit);
        assert_eq!(RecoveryStrategy::choose(99), RecoveryStrategy::Retransmit);
        assert_eq!(RecoveryStrategy::choose(100), RecoveryStrategy::Snapshot);
        assert_eq!(RecoveryStrategy::choose(1000), RecoveryStrategy::Snapshot);
    }
    
    #[test]
    fn test_recovery_manager_flow() {
        let mut manager = RecoveryManager::new(1, 3);
        
        // Normal processing
        assert_eq!(manager.process_message(1).unwrap(), RecoveryAction::ProcessNormally);
        assert_eq!(manager.process_message(2).unwrap(), RecoveryAction::ProcessNormally);
        
        // Gap detection
        let action = manager.process_message(10).unwrap();
        assert!(matches!(action, RecoveryAction::InitiateRecovery { .. }));
        assert!(manager.is_recovering());
        
        // Recovery initiated
        manager.recovery_initiated();
        assert_eq!(manager.state(), &RecoveryState::RecoveryRequested);
        
        // Messages should be buffered during recovery
        assert_eq!(manager.process_message(11).unwrap(), RecoveryAction::BufferMessage);
        
        // Snapshot received and applied
        manager.snapshot_received();
        manager.snapshot_applied(10);
        assert_eq!(manager.state(), &RecoveryState::Resynchronizing);
        
        // Resume normal processing
        assert_eq!(manager.process_message(10).unwrap(), RecoveryAction::ProcessNormally);
        assert!(!manager.is_recovering());
    }
}
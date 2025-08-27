use crate::SinkError;

/// Result of a batch send operation providing partial success information
#[derive(Debug, Clone)]
pub struct BatchResult {
    /// Number of messages successfully sent
    pub succeeded: usize,
    
    /// List of failed message indices and their errors
    pub failed: Vec<(usize, SinkError)>,
    
    /// Total number of messages in the batch
    pub total: usize,
}

impl BatchResult {
    /// Create a new batch result
    pub fn new(total: usize) -> Self {
        Self {
            succeeded: 0,
            failed: Vec::new(),
            total,
        }
    }
    
    /// Record a successful send
    pub fn record_success(&mut self) {
        self.succeeded += 1;
    }
    
    /// Record a failed send
    pub fn record_failure(&mut self, index: usize, error: SinkError) {
        self.failed.push((index, error));
    }
    
    /// Check if all messages succeeded
    pub fn is_complete_success(&self) -> bool {
        self.failed.is_empty() && self.succeeded == self.total
    }
    
    /// Check if any messages succeeded
    pub fn has_partial_success(&self) -> bool {
        self.succeeded > 0 && !self.failed.is_empty()
    }
    
    /// Check if all messages failed
    pub fn is_complete_failure(&self) -> bool {
        self.succeeded == 0 && !self.failed.is_empty()
    }
    
    /// Get success rate as a percentage
    pub fn success_rate(&self) -> f64 {
        if self.total == 0 {
            return 100.0;
        }
        (self.succeeded as f64 / self.total as f64) * 100.0
    }
}
//! Strategy metrics collection

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Thread-safe metrics collector for strategies
#[derive(Debug)]
pub struct MetricsCollector {
    start_time: Instant,
    messages_processed: AtomicU64,
    signals_generated: AtomicU64,
    trades_executed: AtomicU64,
    errors: AtomicU64,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            messages_processed: AtomicU64::new(0),
            signals_generated: AtomicU64::new(0),
            trades_executed: AtomicU64::new(0),
            errors: AtomicU64::new(0),
        }
    }
    
    pub fn increment_messages(&self) {
        self.messages_processed.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn increment_signals(&self) {
        self.signals_generated.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn increment_trades(&self) {
        self.trades_executed.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn increment_errors(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn get_metrics(&self) -> super::StrategyMetrics {
        super::StrategyMetrics {
            messages_processed: self.messages_processed.load(Ordering::Relaxed),
            signals_generated: self.signals_generated.load(Ordering::Relaxed),
            trades_executed: self.trades_executed.load(Ordering::Relaxed),
            errors: self.errors.load(Ordering::Relaxed),
        }
    }
    
    pub fn uptime(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}
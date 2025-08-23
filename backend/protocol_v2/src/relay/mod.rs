//! Domain-Specific Relay Servers
//! 
//! Implements three relay servers with different validation policies:
//! - MarketDataRelay: No checksum validation (performance priority)
//! - SignalRelay: Checksum validation enforced (reliability balance)  
//! - ExecutionRelay: Full checksum validation + audit logging (security priority)

pub mod core;
pub mod market_data_relay;
pub mod signal_relay;
pub mod execution_relay;
pub mod consumer_registry;

pub use core::*;
pub use market_data_relay::*;
pub use signal_relay::*;
pub use execution_relay::*;
pub use consumer_registry::*;

use crate::{RelayDomain, SourceType};
use std::collections::HashMap;
use tokio::net::UnixListener;

/// Consumer identifier for sequence tracking
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ConsumerId {
    pub service_name: String,
    pub instance_id: u32,
}

impl ConsumerId {
    pub fn new(service_name: &str, instance_id: u32) -> Self {
        Self {
            service_name: service_name.to_string(),
            instance_id,
        }
    }
    
    pub fn from_string(id_str: &str) -> Option<Self> {
        let parts: Vec<&str> = id_str.split(':').collect();
        if parts.len() == 2 {
            if let Ok(instance_id) = parts[1].parse::<u32>() {
                return Some(Self::new(parts[0], instance_id));
            }
        }
        None
    }
    
    pub fn to_string(&self) -> String {
        format!("{}:{}", self.service_name, self.instance_id)
    }
}

/// Relay configuration for different domains
#[derive(Debug, Clone)]
pub struct RelayConfig {
    pub domain: RelayDomain,
    pub socket_path: String,
    pub max_throughput_msgs_per_sec: u64,
    pub validate_checksums: bool,
    pub buffer_size_bytes: usize,
    pub audit_log_path: Option<String>,
    pub security_log_path: Option<String>,
}

impl RelayConfig {
    /// Create market data relay configuration (performance optimized)
    pub fn market_data(socket_path: &str) -> Self {
        Self {
            domain: RelayDomain::MarketData,
            socket_path: socket_path.to_string(),
            max_throughput_msgs_per_sec: 1_000_000,
            validate_checksums: false, // SKIP for performance
            buffer_size_bytes: 65536,
            audit_log_path: None,
            security_log_path: None,
        }
    }
    
    /// Create signal relay configuration (reliability focused)
    pub fn signal(socket_path: &str) -> Self {
        Self {
            domain: RelayDomain::Signal,
            socket_path: socket_path.to_string(),
            max_throughput_msgs_per_sec: 100_000,
            validate_checksums: true, // ENFORCE for reliability
            buffer_size_bytes: 32768,
            audit_log_path: Some("/var/log/alphapulse/signals.log".to_string()),
            security_log_path: None,
        }
    }
    
    /// Create execution relay configuration (security focused)
    pub fn execution(socket_path: &str) -> Self {
        Self {
            domain: RelayDomain::Execution,
            socket_path: socket_path.to_string(),
            max_throughput_msgs_per_sec: 50_000,
            validate_checksums: true, // ALWAYS ENFORCE for security
            buffer_size_bytes: 16384,
            audit_log_path: Some("/var/log/alphapulse/execution_audit.log".to_string()),
            security_log_path: Some("/var/log/alphapulse/execution_security.log".to_string()),
        }
    }
}

/// Recovery request information  
#[derive(Debug, Clone)]
pub struct RecoveryRequest {
    pub consumer_id: ConsumerId,
    pub start_sequence: u64,
    pub end_sequence: u64,
    pub request_type: RecoveryRequestType,
}

/// Recovery request types from PROTOCOL.md
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RecoveryRequestType {
    Retransmit,  // Request individual message retransmission
    Snapshot,    // Request snapshot-based recovery
}

/// Statistics for relay monitoring
#[derive(Debug, Clone)]
pub struct RelayStats {
    pub messages_processed: u64,
    pub messages_per_second: f64,
    pub checksum_failures: u64,
    pub recovery_requests: u64,
    pub active_consumers: usize,
    pub uptime_seconds: u64,
}

impl Default for RelayStats {
    fn default() -> Self {
        Self {
            messages_processed: 0,
            messages_per_second: 0.0,
            checksum_failures: 0,
            recovery_requests: 0,
            active_consumers: 0,
            uptime_seconds: 0,
        }
    }
}
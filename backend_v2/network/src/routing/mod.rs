//! Transport Routing Module
//!
//! This module consolidates all transport selection and routing logic that was
//! previously scattered across hybrid/, mycelium/, topology/, and topology_integration/

use crate::{Result, TransportError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod hybrid;
pub mod latency;
pub mod strategy;
pub mod topology_aware;

// Re-export commonly used types
pub use hybrid::HybridRouter;
pub use latency::LatencyBasedRouter;
pub use strategy::{RoutingStrategy, RoutingDecision};
pub use topology_aware::TopologyAwareRouter;

/// Unified Router trait for all routing implementations
pub trait Router: Send + Sync {
    /// Make a routing decision for the given target and message characteristics
    fn route(&self, request: &RouteRequest) -> Result<RoutingDecision>;
    
    /// Update router configuration
    fn update_config(&mut self, config: RouterConfig) -> Result<()>;
    
    /// Check if router is healthy
    fn is_healthy(&self) -> bool;
    
    /// Get router statistics
    fn stats(&self) -> RouterStats;
}

/// Route request containing all information needed for routing decisions
#[derive(Debug, Clone)]
pub struct RouteRequest {
    pub target_node: String,
    pub target_actor: String, 
    pub message_size: usize,
    pub priority: crate::Priority,
    pub latency_requirement: LatencyRequirement,
    pub reliability_requirement: ReliabilityRequirement,
}

/// Latency requirement specification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LatencyRequirement {
    /// Ultra-low latency (<35μs)
    UltraLow,
    /// Low latency (<1ms)
    Low,
    /// Normal latency (<10ms)
    Normal,
    /// High latency (best effort)
    BestEffort,
}

/// Reliability requirement specification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReliabilityRequirement {
    /// At-most-once delivery
    AtMostOnce,
    /// At-least-once delivery (with retries)
    AtLeastOnce,
    /// Exactly-once delivery (idempotent)
    ExactlyOnce,
    /// Best effort (no guarantees)
    BestEffort,
}

/// Router configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterConfig {
    /// Default routing strategy
    pub default_strategy: RoutingStrategy,
    /// Per-actor routing overrides
    pub actor_overrides: HashMap<String, RoutingStrategy>,
    /// Per-node routing overrides  
    pub node_overrides: HashMap<String, RoutingStrategy>,
    /// Latency thresholds
    pub latency_thresholds: LatencyThresholds,
    /// Enable adaptive routing based on performance feedback
    pub adaptive_routing: bool,
}

/// Latency thresholds for routing decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyThresholds {
    pub ultra_low_threshold_ns: u64,  // <35,000ns
    pub low_threshold_ns: u64,        // <1,000,000ns  
    pub normal_threshold_ns: u64,     // <10,000,000ns
}

impl Default for LatencyThresholds {
    fn default() -> Self {
        Self {
            ultra_low_threshold_ns: 35_000,      // 35μs
            low_threshold_ns: 1_000_000,         // 1ms
            normal_threshold_ns: 10_000_000,     // 10ms
        }
    }
}

/// Router statistics for monitoring
#[derive(Debug, Clone, Default)]
pub struct RouterStats {
    pub total_routes: u64,
    pub local_routes: u64,
    pub unix_socket_routes: u64,
    pub tcp_routes: u64,
    pub udp_routes: u64,
    pub failed_routes: u64,
    pub average_decision_time_ns: f64,
}

/// Router factory for creating router instances  
pub struct RouterFactory;

impl RouterFactory {
    /// Create router from configuration
    pub fn create_router(config: RouterConfig) -> Result<Box<dyn Router>> {
        match config.default_strategy {
            RoutingStrategy::Hybrid => {
                Ok(Box::new(HybridRouter::new(config)?))
            }
            RoutingStrategy::LatencyOptimized => {
                Ok(Box::new(LatencyBasedRouter::new(config)?))
            }
            RoutingStrategy::TopologyAware => {
                Ok(Box::new(TopologyAwareRouter::new(config)?))
            }
            _ => {
                Err(TransportError::configuration(
                    "Unsupported routing strategy",
                    Some("routing_strategy")
                ))
            }
        }
    }
    
    /// Create hybrid router (most common use case)
    pub fn create_hybrid_router() -> Result<Box<dyn Router>> {
        let config = RouterConfig {
            default_strategy: RoutingStrategy::Hybrid,
            actor_overrides: HashMap::new(),
            node_overrides: HashMap::new(),
            latency_thresholds: LatencyThresholds::default(),
            adaptive_routing: true,
        };
        Self::create_router(config)
    }
}
//! Latency-Based Router Implementation
//!
//! Optimizes routing decisions purely for lowest possible latency

use super::{Router, RouteRequest, RoutingDecision, RouterConfig, RouterStats};
use crate::Result;

/// Router that optimizes for minimum latency
pub struct LatencyBasedRouter {
    config: RouterConfig,
    stats: RouterStats,
}

impl LatencyBasedRouter {
    pub fn new(config: RouterConfig) -> Result<Self> {
        Ok(Self {
            config,
            stats: RouterStats::default(),
        })
    }
}

impl Router for LatencyBasedRouter {
    fn route(&self, request: &RouteRequest) -> Result<RoutingDecision> {
        // Always prefer local for lowest latency
        Ok(RoutingDecision::Local {
            channel_name: format!("latency_{}_{}", request.target_node, request.target_actor),
            buffer_size: Some(10), // Very small buffer for minimal queuing delay
        })
    }
    
    fn update_config(&mut self, config: RouterConfig) -> Result<()> {
        self.config = config;
        Ok(())
    }
    
    fn is_healthy(&self) -> bool {
        true
    }
    
    fn stats(&self) -> RouterStats {
        self.stats.clone()
    }
}
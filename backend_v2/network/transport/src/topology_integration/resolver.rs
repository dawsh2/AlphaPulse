//! Topology Resolver
//!
//! Resolves actor locations and node addresses using the topology system.

use crate::{TransportError, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Resolver for topology-based transport
#[derive(Clone)]
pub struct TopologyResolver {
    /// Actor to node mappings
    actor_locations: Arc<RwLock<HashMap<String, String>>>,
    /// Node to address mappings
    node_addresses: Arc<RwLock<HashMap<String, NodeAddress>>>,
    /// Cache TTL in seconds
    cache_ttl_seconds: u64,
}

/// Node address information
#[derive(Debug, Clone)]
pub struct NodeAddress {
    /// Primary address (e.g., IP:port)
    pub primary: String,
    /// Alternative addresses
    pub alternatives: Vec<String>,
    /// Unix socket path if available
    pub unix_socket: Option<String>,
    /// Last update timestamp
    pub last_updated: std::time::Instant,
}

impl TopologyResolver {
    /// Create new topology resolver
    pub fn new(cache_ttl_seconds: u64) -> Self {
        Self {
            actor_locations: Arc::new(RwLock::new(HashMap::new())),
            node_addresses: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl_seconds,
        }
    }
    
    /// Resolve transport between two actors
    pub async fn resolve_transport(
        &self,
        source_actor: &str,
        target_actor: &str,
        channel: &str,
    ) -> Result<super::factory::TransportResolution> {
        let target_node = self.resolve_actor(target_actor).await?;
        let node_address = self.resolve_node(&target_node).await?;
        
        Ok(super::factory::TransportResolution {
            node_id: target_node,
            transport_type: if node_address.unix_socket.is_some() {
                "unix".to_string()
            } else {
                "tcp".to_string()
            },
            address: node_address.unix_socket.unwrap_or(node_address.primary),
        })
    }
    
    /// Update configuration
    pub async fn update_config(&self, config: crate::hybrid::TransportConfig) -> Result<()> {
        // Placeholder implementation
        Ok(())
    }
    
    /// Resolve actor location to node
    pub async fn resolve_actor(&self, actor_id: &str) -> Result<String> {
        let locations = self.actor_locations.read().await;
        
        if let Some(node_id) = locations.get(actor_id) {
            debug!("Resolved actor {} to node {}", actor_id, node_id);
            Ok(node_id.clone())
        } else {
            // In a real implementation, query topology service
            warn!("Actor {} not found in topology", actor_id);
            Err(TransportError::resolution(
                &format!("Actor {} not found", actor_id),
                Some(actor_id)
            ))
        }
    }
    
    /// Resolve node to network address
    pub async fn resolve_node(&self, node_id: &str) -> Result<NodeAddress> {
        let mut addresses = self.node_addresses.write().await;
        
        // Check cache
        if let Some(addr) = addresses.get(node_id) {
            if addr.last_updated.elapsed().as_secs() < self.cache_ttl_seconds {
                debug!("Using cached address for node {}", node_id);
                return Ok(addr.clone());
            }
        }
        
        // In a real implementation, query topology service
        // For now, create a mock address
        let node_addr = NodeAddress {
            primary: format!("127.0.0.1:{}", 5000 + node_id.len()),
            alternatives: vec![],
            unix_socket: Some(format!("/tmp/alphapulse/{}.sock", node_id)),
            last_updated: std::time::Instant::now(),
        };
        
        addresses.insert(node_id.to_string(), node_addr.clone());
        debug!("Resolved node {} to address {}", node_id, node_addr.primary);
        
        Ok(node_addr)
    }
    
    /// Update actor location
    pub async fn update_actor_location(&self, actor_id: String, node_id: String) {
        let mut locations = self.actor_locations.write().await;
        locations.insert(actor_id.clone(), node_id.clone());
        debug!("Updated actor {} location to node {}", actor_id, node_id);
    }
    
    /// Update node address
    pub async fn update_node_address(&self, node_id: String, address: NodeAddress) {
        let mut addresses = self.node_addresses.write().await;
        addresses.insert(node_id.clone(), address);
        debug!("Updated node {} address", node_id);
    }
    
    /// Remove actor from topology
    pub async fn remove_actor(&self, actor_id: &str) -> Option<String> {
        let mut locations = self.actor_locations.write().await;
        locations.remove(actor_id)
    }
    
    /// Remove node from topology
    pub async fn remove_node(&self, node_id: &str) -> Option<NodeAddress> {
        let mut addresses = self.node_addresses.write().await;
        addresses.remove(node_id)
    }
    
    /// Clear all cached data
    pub async fn clear_cache(&self) {
        let mut locations = self.actor_locations.write().await;
        let mut addresses = self.node_addresses.write().await;
        
        locations.clear();
        addresses.clear();
        
        debug!("Cleared topology resolver cache");
    }
    
    /// Get statistics
    pub async fn statistics(&self) -> ResolverStatistics {
        let locations = self.actor_locations.read().await;
        let addresses = self.node_addresses.read().await;
        
        ResolverStatistics {
            cached_actors: locations.len(),
            cached_nodes: addresses.len(),
            cache_ttl_seconds: self.cache_ttl_seconds,
        }
    }
}

/// Resolver statistics
#[derive(Debug, Clone)]
pub struct ResolverStatistics {
    /// Number of cached actor locations
    pub cached_actors: usize,
    /// Number of cached node addresses
    pub cached_nodes: usize,
    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_resolver_creation() {
        let resolver = TopologyResolver::new(60);
        let stats = resolver.statistics().await;
        
        assert_eq!(stats.cached_actors, 0);
        assert_eq!(stats.cached_nodes, 0);
        assert_eq!(stats.cache_ttl_seconds, 60);
    }
    
    #[tokio::test]
    async fn test_actor_resolution() {
        let resolver = TopologyResolver::new(60);
        
        // Add actor location
        resolver.update_actor_location("actor1".to_string(), "node1".to_string()).await;
        
        // Resolve actor
        let node = resolver.resolve_actor("actor1").await.unwrap();
        assert_eq!(node, "node1");
        
        // Try unknown actor
        let result = resolver.resolve_actor("unknown").await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_node_resolution() {
        let resolver = TopologyResolver::new(60);
        
        // Resolve node (will create mock)
        let addr = resolver.resolve_node("node1").await.unwrap();
        assert!(addr.primary.starts_with("127.0.0.1:"));
        assert!(addr.unix_socket.is_some());
        
        // Should use cache on second call
        let addr2 = resolver.resolve_node("node1").await.unwrap();
        assert_eq!(addr.primary, addr2.primary);
    }
    
    #[tokio::test]
    async fn test_cache_operations() {
        let resolver = TopologyResolver::new(60);
        
        // Add some data
        resolver.update_actor_location("actor1".to_string(), "node1".to_string()).await;
        resolver.update_actor_location("actor2".to_string(), "node2".to_string()).await;
        
        let _ = resolver.resolve_node("node1").await;
        let _ = resolver.resolve_node("node2").await;
        
        // Check statistics
        let stats = resolver.statistics().await;
        assert_eq!(stats.cached_actors, 2);
        assert_eq!(stats.cached_nodes, 2);
        
        // Remove actor
        let removed = resolver.remove_actor("actor1").await;
        assert_eq!(removed, Some("node1".to_string()));
        
        // Clear cache
        resolver.clear_cache().await;
        
        let stats = resolver.statistics().await;
        assert_eq!(stats.cached_actors, 0);
        assert_eq!(stats.cached_nodes, 0);
    }
}
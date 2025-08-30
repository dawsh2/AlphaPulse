//! Bundle Configuration (MYCEL-004)
//!
//! Actor bundling for zero-cost communication. Actors in the same bundle
//! communicate via Arc<T> passing instead of TLV serialization.

use super::registry::ActorId;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Bundle configuration for grouping actors
#[derive(Debug, Clone)]
pub struct BundleConfiguration {
    pub name: String,
    pub actors: Vec<ActorId>,
    pub deployment: DeploymentMode,
}

/// Deployment mode for actor bundles
#[derive(Debug, Clone)]
pub enum DeploymentMode {
    /// Shared memory - zero-cost Arc<T> passing
    SharedMemory {
        channels: HashMap<ActorId, mpsc::Sender<Arc<dyn std::any::Any + Send + Sync>>>,
    },
    /// Same node - Unix domain sockets
    SameNode {
        socket_paths: HashMap<ActorId, String>,
    },
    /// Distributed - network transport
    Distributed {
        node_assignments: HashMap<ActorId, String>,
    },
}

impl BundleConfiguration {
    /// Create new bundle with shared memory deployment
    pub fn new_shared_memory(name: String, actors: Vec<ActorId>) -> Self {
        let channels = HashMap::new(); // Channels created on demand
        
        Self {
            name,
            actors,
            deployment: DeploymentMode::SharedMemory { channels },
        }
    }
    
    /// Create new bundle with same-node deployment
    pub fn new_same_node(name: String, actors: Vec<ActorId>) -> Self {
        let socket_paths = actors.iter()
            .map(|id| (id.clone(), format!("/tmp/mycelium_{}.sock", id)))
            .collect();
        
        Self {
            name,
            actors,
            deployment: DeploymentMode::SameNode { socket_paths },
        }
    }
    
    /// Create new bundle with distributed deployment
    pub fn new_distributed(
        name: String, 
        actors: Vec<ActorId>,
        node_assignments: HashMap<ActorId, String>,
    ) -> Self {
        Self {
            name,
            actors,
            deployment: DeploymentMode::Distributed { node_assignments },
        }
    }
    
    /// Check if bundle contains actor
    pub fn contains_actor(&self, actor_id: &ActorId) -> bool {
        self.actors.contains(actor_id)
    }
    
    /// Get deployment mode
    pub fn deployment_mode(&self) -> &DeploymentMode {
        &self.deployment
    }
    
    /// Check if two actors are bundled together
    pub fn are_bundled(&self, actor1: &ActorId, actor2: &ActorId) -> bool {
        self.contains_actor(actor1) && self.contains_actor(actor2)
    }
    
    /// Validate bundle configuration for correctness and completeness
    pub fn validate(&self) -> crate::Result<()> {
        // Validate bundle has actors
        if self.actors.is_empty() {
            return Err(crate::TransportError::configuration(
                "Bundle cannot be empty - must contain at least one actor",
                Some("actors")
            ));
        }
        
        // Validate bundle name
        if self.name.trim().is_empty() {
            return Err(crate::TransportError::configuration(
                "Bundle name cannot be empty",
                Some("name")
            ));
        }
        
        // Validate deployment-specific constraints
        match &self.deployment {
            DeploymentMode::SharedMemory { channels } => {
                // For shared memory, channels should either be empty (created on demand)
                // or contain entries for all actors in the bundle
                if !channels.is_empty() && channels.len() != self.actors.len() {
                    return Err(crate::TransportError::configuration(
                        "Shared memory deployment: if channels are specified, must have one for each actor",
                        Some("deployment.channels")
                    ));
                }
                
                // Verify all actors in the bundle have channels if any are specified
                for actor_id in &self.actors {
                    if !channels.is_empty() && !channels.contains_key(actor_id) {
                        return Err(crate::TransportError::configuration(
                            &format!("Actor {} missing from shared memory channels", actor_id),
                            Some("deployment.channels")
                        ));
                    }
                }
            },
            DeploymentMode::SameNode { socket_paths } => {
                // For same-node deployment, must have socket paths for all actors
                if socket_paths.len() != self.actors.len() {
                    return Err(crate::TransportError::configuration(
                        "Same-node deployment: must specify socket path for each actor",
                        Some("deployment.socket_paths")
                    ));
                }
                
                // Verify all actors have unique socket paths
                for actor_id in &self.actors {
                    if !socket_paths.contains_key(actor_id) {
                        return Err(crate::TransportError::configuration(
                            &format!("Actor {} missing socket path in same-node deployment", actor_id),
                            Some("deployment.socket_paths")
                        ));
                    }
                }
                
                // Check for duplicate socket paths (would cause conflicts)
                let mut unique_paths = std::collections::HashSet::new();
                for path in socket_paths.values() {
                    if path.trim().is_empty() {
                        return Err(crate::TransportError::configuration(
                            "Socket path cannot be empty",
                            Some("deployment.socket_paths")
                        ));
                    }
                    
                    if !unique_paths.insert(path) {
                        return Err(crate::TransportError::configuration(
                            &format!("Duplicate socket path: {}", path),
                            Some("deployment.socket_paths")
                        ));
                    }
                }
            },
            DeploymentMode::Distributed { node_assignments } => {
                // For distributed deployment, must have node assignments for all actors
                if node_assignments.len() != self.actors.len() {
                    return Err(crate::TransportError::configuration(
                        "Distributed deployment: must specify node assignment for each actor",
                        Some("deployment.node_assignments")
                    ));
                }
                
                // Verify all actors have node assignments and they're valid
                for actor_id in &self.actors {
                    if let Some(node_address) = node_assignments.get(actor_id) {
                        if node_address.trim().is_empty() {
                            return Err(crate::TransportError::configuration(
                                &format!("Node assignment for actor {} cannot be empty", actor_id),
                                Some("deployment.node_assignments")
                            ));
                        }
                        
                        // Validate that node address looks like a valid socket address
                        match node_address.parse::<std::net::SocketAddr>() {
                            Ok(socket_addr) => {
                                // Additional validation for reserved IP ranges
                                if let std::net::SocketAddr::V4(v4_addr) = socket_addr {
                                    let ip = v4_addr.ip();
                                    
                                    // Check for reserved ranges that shouldn't be used in distributed deployment
                                    if ip.is_loopback() {
                                        return Err(crate::TransportError::configuration(
                                            &format!("Node address '{}' for actor {} uses loopback address (127.0.0.0/8) - use SameNode deployment instead", node_address, actor_id),
                                            Some("deployment.node_assignments")
                                        ));
                                    }
                                    
                                    if ip.is_link_local() {
                                        return Err(crate::TransportError::configuration(
                                            &format!("Node address '{}' for actor {} uses link-local address (169.254.0.0/16) - not routable", node_address, actor_id),
                                            Some("deployment.node_assignments")
                                        ));
                                    }
                                    
                                    if ip.is_broadcast() {
                                        return Err(crate::TransportError::configuration(
                                            &format!("Node address '{}' for actor {} is broadcast address", node_address, actor_id),
                                            Some("deployment.node_assignments")
                                        ));
                                    }
                                    
                                    if ip.is_documentation() {
                                        return Err(crate::TransportError::configuration(
                                            &format!("Node address '{}' for actor {} uses documentation range (TEST-NET)", node_address, actor_id),
                                            Some("deployment.node_assignments")
                                        ));
                                    }
                                    
                                    if ip.is_unspecified() {
                                        return Err(crate::TransportError::configuration(
                                            &format!("Node address '{}' for actor {} uses unspecified address (0.0.0.0)", node_address, actor_id),
                                            Some("deployment.node_assignments")
                                        ));
                                    }
                                    
                                    // Warn about private addresses (but allow them as they might be valid in some deployments)
                                    if ip.is_private() {
                                        tracing::warn!(
                                            "Node address '{}' for actor {} uses private IP range (10.0.0.0/8, 172.16.0.0/12, or 192.168.0.0/16) - ensure network connectivity",
                                            node_address, actor_id
                                        );
                                    }
                                }
                                
                                // Check port validity
                                if socket_addr.port() == 0 {
                                    return Err(crate::TransportError::configuration(
                                        &format!("Node address '{}' for actor {} has invalid port 0", node_address, actor_id),
                                        Some("deployment.node_assignments")
                                    ));
                                }
                                
                                // Reserved ports warning (but allow as user might have permissions)
                                if socket_addr.port() < 1024 {
                                    tracing::warn!(
                                        "Node address '{}' for actor {} uses privileged port {} - ensure proper permissions",
                                        node_address, actor_id, socket_addr.port()
                                    );
                                }
                            },
                            Err(e) => {
                                return Err(crate::TransportError::configuration(
                                    &format!("Invalid node address '{}' for actor {}: {}", node_address, actor_id, e),
                                    Some("deployment.node_assignments")
                                ));
                            }
                        }
                    } else {
                        return Err(crate::TransportError::configuration(
                            &format!("Actor {} missing node assignment in distributed deployment", actor_id),
                            Some("deployment.node_assignments")
                        ));
                    }
                }
            },
        }
        
        Ok(())
    }
}

/// Runtime actor bundle for zero-cost communication
#[derive(Debug)]
pub struct ActorBundle {
    /// Bundle configuration
    config: BundleConfiguration,
    /// Active channels for shared memory communication
    channels: HashMap<ActorId, mpsc::Sender<Arc<dyn std::any::Any + Send + Sync>>>,
    /// Performance metrics
    metrics: BundleMetrics,
}

/// Bundle performance metrics
#[derive(Debug, Default)]
pub struct BundleMetrics {
    /// Zero-copy messages sent
    pub zero_copy_messages: u64,
    /// Total serialization bytes eliminated
    pub serialization_bytes_eliminated: u64,
    /// Average message latency in nanoseconds
    pub avg_latency_ns: f64,
}

impl ActorBundle {
    /// Create new bundle from configuration
    pub fn new(config: BundleConfiguration) -> Self {
        Self {
            config,
            channels: HashMap::new(),
            metrics: BundleMetrics::default(),
        }
    }
    
    /// Add actor channel to bundle
    pub fn add_actor_channel(
        &mut self,
        actor_id: ActorId,
        sender: mpsc::Sender<Arc<dyn std::any::Any + Send + Sync>>,
    ) {
        self.channels.insert(actor_id, sender);
    }
    
    /// Send message within bundle (zero-copy)
    pub async fn send_local<T: Send + Sync + 'static>(
        &mut self,
        to_actor: &ActorId,
        message: T,
    ) -> Result<(), crate::TransportError> {
        let start = std::time::Instant::now();
        
        if let Some(channel) = self.channels.get(to_actor) {
            let arc_message = Arc::new(message);
            let message_size = std::mem::size_of::<T>() as u64;
            
            channel.send(arc_message as Arc<dyn std::any::Any + Send + Sync>).await
                .map_err(|_| crate::TransportError::network("Bundle channel closed"))?;
            
            // Update metrics
            self.metrics.zero_copy_messages += 1;
            self.metrics.serialization_bytes_eliminated += message_size;
            
            let latency = start.elapsed().as_nanos() as f64;
            self.metrics.avg_latency_ns = (self.metrics.avg_latency_ns + latency) / 2.0;
            
            Ok(())
        } else {
            Err(crate::TransportError::configuration(
                &format!("Actor {} not found in bundle", to_actor),
                Some("bundle_actor_id")
            ))
        }
    }
    
    /// Check if two actors are bundled for zero-copy communication
    pub fn can_use_zero_copy(&self, from_actor: &ActorId, to_actor: &ActorId) -> bool {
        self.channels.contains_key(from_actor) && self.channels.contains_key(to_actor)
    }
    
    /// Get bundle metrics
    pub fn metrics(&self) -> &BundleMetrics {
        &self.metrics
    }
    
    /// Get bundle configuration
    pub fn config(&self) -> &BundleConfiguration {
        &self.config
    }
    
    /// Get actor count in bundle
    pub fn actor_count(&self) -> usize {
        self.channels.len()
    }
}

/// Global bundle configuration manager
#[derive(Debug, Default)]
pub struct BundleManager {
    bundles: HashMap<String, BundleConfiguration>,
    active_bundles: HashMap<String, ActorBundle>,
    actor_to_bundle: HashMap<ActorId, String>,
}

impl BundleManager {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Add bundle configuration and create active bundle
    pub fn add_bundle(&mut self, config: BundleConfiguration) {
        let bundle_name = config.name.clone();
        
        // Update actor-to-bundle mapping
        for actor_id in &config.actors {
            self.actor_to_bundle.insert(actor_id.clone(), bundle_name.clone());
        }
        
        // Create active bundle
        let active_bundle = ActorBundle::new(config.clone());
        self.active_bundles.insert(bundle_name.clone(), active_bundle);
        
        self.bundles.insert(bundle_name, config);
    }
    
    /// Get active bundle for runtime use
    pub fn get_active_bundle(&mut self, name: &str) -> Option<&mut ActorBundle> {
        self.active_bundles.get_mut(name)
    }
    
    /// Add actor channel to active bundle
    pub fn add_actor_to_bundle(
        &mut self,
        bundle_name: &str,
        actor_id: ActorId,
        sender: mpsc::Sender<Arc<dyn std::any::Any + Send + Sync>>,
    ) -> Result<(), crate::TransportError> {
        if let Some(bundle) = self.active_bundles.get_mut(bundle_name) {
            bundle.add_actor_channel(actor_id, sender);
            Ok(())
        } else {
            Err(crate::TransportError::configuration(
                &format!("Bundle '{}' not found", bundle_name),
                Some("bundle_name")
            ))
        }
    }
    
    /// Find bundle for actor
    pub fn find_bundle(&self, actor_id: &ActorId) -> Option<&BundleConfiguration> {
        if let Some(bundle_name) = self.actor_to_bundle.get(actor_id) {
            self.bundles.get(bundle_name)
        } else {
            None
        }
    }
    
    /// Check if two actors are in same bundle
    pub fn are_bundled(&self, actor1: &ActorId, actor2: &ActorId) -> bool {
        if let (Some(bundle1), Some(bundle2)) = (
            self.actor_to_bundle.get(actor1),
            self.actor_to_bundle.get(actor2),
        ) {
            bundle1 == bundle2
        } else {
            false
        }
    }
    
    /// List all bundles
    pub fn list_bundles(&self) -> Vec<&str> {
        self.bundles.keys().map(|s| s.as_str()).collect()
    }
    
    /// Get bundle by name
    pub fn get_bundle(&self, name: &str) -> Option<&BundleConfiguration> {
        self.bundles.get(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundle_configuration() {
        let actor1 = ActorId::new();
        let actor2 = ActorId::new();
        let actors = vec![actor1.clone(), actor2.clone()];
        
        let bundle = BundleConfiguration::new_shared_memory(
            "test_bundle".to_string(),
            actors,
        );
        
        assert_eq!(bundle.name, "test_bundle");
        assert!(bundle.contains_actor(&actor1));
        assert!(bundle.contains_actor(&actor2));
        assert!(bundle.are_bundled(&actor1, &actor2));
        
        let actor3 = ActorId::new();
        assert!(!bundle.contains_actor(&actor3));
        assert!(!bundle.are_bundled(&actor1, &actor3));
    }
    
    #[test]
    fn test_bundle_manager() {
        let mut manager = BundleManager::new();
        
        let actor1 = ActorId::new();
        let actor2 = ActorId::new();
        let bundle = BundleConfiguration::new_shared_memory(
            "test_bundle".to_string(),
            vec![actor1.clone(), actor2.clone()],
        );
        
        manager.add_bundle(bundle);
        
        assert!(manager.are_bundled(&actor1, &actor2));
        assert_eq!(manager.list_bundles(), vec!["test_bundle"]);
        
        let found_bundle = manager.find_bundle(&actor1).unwrap();
        assert_eq!(found_bundle.name, "test_bundle");
    }
    
    #[test]
    fn test_deployment_modes() {
        let actors = vec![ActorId::new(), ActorId::new()];
        
        let shared_memory = BundleConfiguration::new_shared_memory(
            "shared".to_string(),
            actors.clone(),
        );
        
        match shared_memory.deployment {
            DeploymentMode::SharedMemory { .. } => {},
            _ => panic!("Expected SharedMemory deployment"),
        }
        
        let same_node = BundleConfiguration::new_same_node(
            "same_node".to_string(),
            actors.clone(),
        );
        
        match same_node.deployment {
            DeploymentMode::SameNode { ref socket_paths } => {
                assert_eq!(socket_paths.len(), 2);
            },
            _ => panic!("Expected SameNode deployment"),
        }
        
        let mut node_assignments = HashMap::new();
        node_assignments.insert(actors[0].clone(), "192.168.1.100:8080".to_string());
        node_assignments.insert(actors[1].clone(), "192.168.1.101:8080".to_string());
        
        let distributed = BundleConfiguration::new_distributed(
            "distributed".to_string(),
            actors.clone(),
            node_assignments,
        );
        
        match distributed.deployment {
            DeploymentMode::Distributed { ref node_assignments } => {
                assert_eq!(node_assignments.len(), 2);
            },
            _ => panic!("Expected Distributed deployment"),
        }
    }
    
    #[test]
    fn test_bundle_validation() {
        let actor1 = ActorId::new();
        let actor2 = ActorId::new();
        
        // Test valid shared memory bundle
        let valid_bundle = BundleConfiguration::new_shared_memory(
            "test_bundle".to_string(),
            vec![actor1.clone(), actor2.clone()],
        );
        assert!(valid_bundle.validate().is_ok());
        
        // Test empty bundle (should fail)
        let empty_bundle = BundleConfiguration::new_shared_memory(
            "empty_bundle".to_string(),
            vec![],
        );
        assert!(empty_bundle.validate().is_err());
        
        // Test empty name (should fail)
        let empty_name_bundle = BundleConfiguration {
            name: "".to_string(),
            actors: vec![actor1.clone()],
            deployment: DeploymentMode::SharedMemory { channels: HashMap::new() },
        };
        assert!(empty_name_bundle.validate().is_err());
        
        // Test distributed with invalid node addresses
        let mut invalid_node_assignments = HashMap::new();
        invalid_node_assignments.insert(actor1.clone(), "invalid_address".to_string());
        invalid_node_assignments.insert(actor2.clone(), "192.168.1.100:8080".to_string());
        
        let invalid_distributed = BundleConfiguration::new_distributed(
            "invalid_distributed".to_string(),
            vec![actor1.clone(), actor2.clone()],
            invalid_node_assignments,
        );
        assert!(invalid_distributed.validate().is_err());
        
        // Test valid distributed bundle
        let mut valid_node_assignments = HashMap::new();
        valid_node_assignments.insert(actor1.clone(), "192.168.1.100:8080".to_string());
        valid_node_assignments.insert(actor2.clone(), "192.168.1.101:8080".to_string());
        
        let valid_distributed = BundleConfiguration::new_distributed(
            "valid_distributed".to_string(),
            vec![actor1.clone(), actor2.clone()],
            valid_node_assignments,
        );
        assert!(valid_distributed.validate().is_ok());
        
        // Test same node with duplicate socket paths (should fail)
        let mut duplicate_socket_paths = HashMap::new();
        duplicate_socket_paths.insert(actor1.clone(), "/tmp/same_socket.sock".to_string());
        duplicate_socket_paths.insert(actor2.clone(), "/tmp/same_socket.sock".to_string()); // Duplicate!
        
        let duplicate_socket_bundle = BundleConfiguration {
            name: "duplicate_sockets".to_string(),
            actors: vec![actor1.clone(), actor2.clone()],
            deployment: DeploymentMode::SameNode { socket_paths: duplicate_socket_paths },
        };
        assert!(duplicate_socket_bundle.validate().is_err());
    }
    
    #[test]
    fn test_reserved_address_validation() {
        let actor1 = ActorId::new();
        let actor2 = ActorId::new();
        
        // Test loopback address (should fail)
        let mut loopback_assignments = HashMap::new();
        loopback_assignments.insert(actor1.clone(), "127.0.0.1:8080".to_string());
        loopback_assignments.insert(actor2.clone(), "192.168.1.100:8080".to_string());
        
        let loopback_bundle = BundleConfiguration::new_distributed(
            "loopback_test".to_string(),
            vec![actor1.clone(), actor2.clone()],
            loopback_assignments,
        );
        let result = loopback_bundle.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("loopback"));
        
        // Test link-local address (should fail)
        let mut link_local_assignments = HashMap::new();
        link_local_assignments.insert(actor1.clone(), "169.254.0.1:8080".to_string());
        link_local_assignments.insert(actor2.clone(), "192.168.1.100:8080".to_string());
        
        let link_local_bundle = BundleConfiguration::new_distributed(
            "link_local_test".to_string(),
            vec![actor1.clone(), actor2.clone()],
            link_local_assignments,
        );
        let result = link_local_bundle.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("link-local"));
        
        // Test unspecified address (should fail)
        let mut unspecified_assignments = HashMap::new();
        unspecified_assignments.insert(actor1.clone(), "0.0.0.0:8080".to_string());
        unspecified_assignments.insert(actor2.clone(), "192.168.1.100:8080".to_string());
        
        let unspecified_bundle = BundleConfiguration::new_distributed(
            "unspecified_test".to_string(),
            vec![actor1.clone(), actor2.clone()],
            unspecified_assignments,
        );
        let result = unspecified_bundle.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unspecified"));
        
        // Test port 0 (should fail)
        let mut port_zero_assignments = HashMap::new();
        port_zero_assignments.insert(actor1.clone(), "192.168.1.100:0".to_string());
        port_zero_assignments.insert(actor2.clone(), "192.168.1.101:8080".to_string());
        
        let port_zero_bundle = BundleConfiguration::new_distributed(
            "port_zero_test".to_string(),
            vec![actor1.clone(), actor2.clone()],
            port_zero_assignments,
        );
        let result = port_zero_bundle.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("invalid port 0"));
        
        // Test documentation range (should fail) 
        let mut doc_range_assignments = HashMap::new();
        doc_range_assignments.insert(actor1.clone(), "192.0.2.1:8080".to_string()); // TEST-NET-1
        doc_range_assignments.insert(actor2.clone(), "192.168.1.100:8080".to_string());
        
        let doc_range_bundle = BundleConfiguration::new_distributed(
            "doc_range_test".to_string(),
            vec![actor1.clone(), actor2.clone()],
            doc_range_assignments,
        );
        let result = doc_range_bundle.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("documentation"));
        
        // Test private addresses (should pass with warning)
        let mut private_assignments = HashMap::new();
        private_assignments.insert(actor1.clone(), "10.0.0.1:8080".to_string());
        private_assignments.insert(actor2.clone(), "172.16.0.1:8080".to_string());
        
        let private_bundle = BundleConfiguration::new_distributed(
            "private_test".to_string(),
            vec![actor1.clone(), actor2.clone()],
            private_assignments,
        );
        // Private IPs are allowed but should trigger warning
        assert!(private_bundle.validate().is_ok());
        
        // Test valid public addresses (should pass)
        let mut public_assignments = HashMap::new();
        public_assignments.insert(actor1.clone(), "8.8.8.8:8080".to_string());
        public_assignments.insert(actor2.clone(), "1.1.1.1:8080".to_string());
        
        let public_bundle = BundleConfiguration::new_distributed(
            "public_test".to_string(),
            vec![actor1.clone(), actor2.clone()],
            public_assignments,
        );
        assert!(public_bundle.validate().is_ok());
    }
}
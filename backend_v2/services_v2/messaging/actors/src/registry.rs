//! Actor Registry
//!
//! Location-transparent actor discovery and routing.

use super::transport::ActorTransport;
use crate::{Result, TransportError};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Unique actor identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ActorId {
    id: Uuid,
}

impl ActorId {
    /// Create new actor ID
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
        }
    }
    
    /// Create from UUID
    pub fn from_uuid(id: Uuid) -> Self {
        Self { id }
    }
    
    /// Get UUID
    pub fn uuid(&self) -> Uuid {
        self.id
    }
}

impl fmt::Display for ActorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "actor-{}", self.id.simple())
    }
}

impl Default for ActorId {
    fn default() -> Self {
        Self::new()
    }
}

/// Actor registry for location-transparent references
#[derive(Debug)]
pub struct ActorRegistry {
    /// Local actors (same process)
    local_actors: Arc<RwLock<HashMap<ActorId, ActorTransport>>>,
    
    /// Remote actor addresses (different process/node)
    remote_actors: Arc<RwLock<HashMap<ActorId, RemoteActorInfo>>>,
}

/// Information about remote actor location
#[derive(Debug, Clone)]
pub struct RemoteActorInfo {
    pub node_id: String,
    pub transport_address: String,
    pub last_seen: std::time::SystemTime,
}

impl ActorRegistry {
    pub fn new() -> Self {
        Self {
            local_actors: Arc::new(RwLock::new(HashMap::new())),
            remote_actors: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register local actor
    pub async fn register_actor(&self, id: ActorId, transport: ActorTransport) -> Result<()> {
        tracing::debug!("Registering local actor: {}", id);
        self.local_actors.write().await.insert(id, transport);
        Ok(())
    }
    
    /// Register remote actor
    pub async fn register_remote_actor(
        &self,
        id: ActorId,
        node_id: String,
        transport_address: String,
    ) -> Result<()> {
        tracing::debug!("Registering remote actor: {} on node {}", id, node_id);
        
        let info = RemoteActorInfo {
            node_id,
            transport_address,
            last_seen: std::time::SystemTime::now(),
        };
        
        self.remote_actors.write().await.insert(id, info);
        Ok(())
    }
    
    /// Unregister actor
    pub async fn unregister_actor(&self, id: &ActorId) -> Result<()> {
        tracing::debug!("Unregistering actor: {}", id);
        
        // Try local first
        if self.local_actors.write().await.remove(id).is_some() {
            return Ok(());
        }
        
        // Try remote
        if self.remote_actors.write().await.remove(id).is_some() {
            return Ok(());
        }
        
        tracing::warn!("Attempted to unregister unknown actor: {}", id);
        Err(TransportError::configuration(
            &format!("Actor {} not found in registry", id),
            Some("actor_id")
        ))
    }
    
    /// Find actor transport
    pub async fn find_actor(&self, id: &ActorId) -> Option<ActorLocation> {
        // Check local actors first
        if let Some(transport) = self.local_actors.read().await.get(id).cloned() {
            return Some(ActorLocation::Local(transport));
        }
        
        // Check remote actors
        if let Some(info) = self.remote_actors.read().await.get(id).cloned() {
            return Some(ActorLocation::Remote(info));
        }
        
        None
    }
    
    /// List all local actors
    pub async fn list_local_actors(&self) -> Vec<ActorId> {
        self.local_actors.read().await.keys().cloned().collect()
    }
    
    /// List all remote actors
    pub async fn list_remote_actors(&self) -> Vec<(ActorId, RemoteActorInfo)> {
        self.remote_actors.read().await
            .iter()
            .map(|(id, info)| (id.clone(), info.clone()))
            .collect()
    }
    
    /// Get total actor count
    pub async fn total_actors(&self) -> usize {
        let local_count = self.local_actors.read().await.len();
        let remote_count = self.remote_actors.read().await.len();
        local_count + remote_count
    }
    
    /// Check if actor exists
    pub async fn contains_actor(&self, id: &ActorId) -> bool {
        self.find_actor(id).await.is_some()
    }
}

/// Actor location information
#[derive(Debug, Clone)]
pub enum ActorLocation {
    /// Actor is in same process
    Local(ActorTransport),
    /// Actor is on remote node
    Remote(RemoteActorInfo),
}

impl ActorLocation {
    /// Check if actor is local
    pub fn is_local(&self) -> bool {
        matches!(self, ActorLocation::Local(_))
    }
    
    /// Check if actor is remote
    pub fn is_remote(&self) -> bool {
        matches!(self, ActorLocation::Remote(_))
    }
    
    /// Get transport if local
    pub fn local_transport(&self) -> Option<&ActorTransport> {
        match self {
            ActorLocation::Local(transport) => Some(transport),
            ActorLocation::Remote(_) => None,
        }
    }
    
    /// Get remote info if remote
    pub fn remote_info(&self) -> Option<&RemoteActorInfo> {
        match self {
            ActorLocation::Local(_) => None,
            ActorLocation::Remote(info) => Some(info),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_actor_id_creation() {
        let id1 = ActorId::new();
        let id2 = ActorId::new();
        
        assert_ne!(id1, id2);
        assert_ne!(id1.uuid(), id2.uuid());
    }
    
    #[tokio::test]
    async fn test_actor_id_display() {
        let id = ActorId::new();
        let display = format!("{}", id);
        assert!(display.starts_with("actor-"));
    }
    
    #[tokio::test]
    async fn test_registry_local_actor() {
        let registry = ActorRegistry::new();
        let actor_id = ActorId::new();
        
        // Create dummy transport
        let (sender, _receiver) = mpsc::channel(100);
        let transport = ActorTransport::new_local(sender, actor_id.to_string());
        
        // Register actor
        registry.register_actor(actor_id.clone(), transport).await.unwrap();
        
        // Verify actor exists
        assert!(registry.contains_actor(&actor_id).await);
        assert_eq!(registry.total_actors().await, 1);
        
        // Find actor
        let location = registry.find_actor(&actor_id).await.unwrap();
        assert!(location.is_local());
        
        // Unregister actor
        registry.unregister_actor(&actor_id).await.unwrap();
        assert!(!registry.contains_actor(&actor_id).await);
        assert_eq!(registry.total_actors().await, 0);
    }
    
    #[tokio::test]
    async fn test_registry_remote_actor() {
        let registry = ActorRegistry::new();
        let actor_id = ActorId::new();
        
        // Register remote actor
        registry.register_remote_actor(
            actor_id.clone(),
            "node1".to_string(),
            "tcp://192.168.1.100:8080".to_string(),
        ).await.unwrap();
        
        // Verify actor exists
        assert!(registry.contains_actor(&actor_id).await);
        assert_eq!(registry.total_actors().await, 1);
        
        // Find actor
        let location = registry.find_actor(&actor_id).await.unwrap();
        assert!(location.is_remote());
        
        if let Some(info) = location.remote_info() {
            assert_eq!(info.node_id, "node1");
            assert_eq!(info.transport_address, "tcp://192.168.1.100:8080");
        } else {
            panic!("Expected remote actor info");
        }
    }
    
    #[tokio::test]
    async fn test_registry_lists() {
        let registry = ActorRegistry::new();
        
        // Add local actor
        let local_id = ActorId::new();
        let (sender, _receiver) = mpsc::channel(100);
        let transport = ActorTransport::new_local(sender, local_id.to_string());
        registry.register_actor(local_id.clone(), transport).await.unwrap();
        
        // Add remote actor
        let remote_id = ActorId::new();
        registry.register_remote_actor(
            remote_id.clone(),
            "node1".to_string(),
            "tcp://192.168.1.100:8080".to_string(),
        ).await.unwrap();
        
        // Check lists
        let local_actors = registry.list_local_actors().await;
        assert_eq!(local_actors.len(), 1);
        assert!(local_actors.contains(&local_id));
        
        let remote_actors = registry.list_remote_actors().await;
        assert_eq!(remote_actors.len(), 1);
        assert_eq!(remote_actors[0].0, remote_id);
        
        assert_eq!(registry.total_actors().await, 2);
    }
    
    #[tokio::test]
    async fn test_actor_location() {
        let (sender, _receiver) = mpsc::channel(100);
        let transport = ActorTransport::new_local(sender, "test".to_string());
        let local_location = ActorLocation::Local(transport);
        
        assert!(local_location.is_local());
        assert!(!local_location.is_remote());
        assert!(local_location.local_transport().is_some());
        assert!(local_location.remote_info().is_none());
        
        let remote_info = RemoteActorInfo {
            node_id: "node1".to_string(),
            transport_address: "tcp://192.168.1.100:8080".to_string(),
            last_seen: std::time::SystemTime::now(),
        };
        let remote_location = ActorLocation::Remote(remote_info);
        
        assert!(!remote_location.is_local());
        assert!(remote_location.is_remote());
        assert!(remote_location.local_transport().is_none());
        assert!(remote_location.remote_info().is_some());
    }
}
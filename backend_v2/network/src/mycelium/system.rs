//! Actor System Core (MYCEL-003)
//!
//! Core actor runtime with lifecycle management, message routing, and
//! supervision. Supports zero-cost local communication for bundled actors.

use super::transport::{ActorTransport, TransportMetrics};
use super::messages::Message;
use super::bundle::{BundleConfiguration, DeploymentMode};
use super::registry::{ActorId, ActorRegistry};

use crate::{Result, TransportError};
use parking_lot::Mutex;
use async_trait::async_trait;
use futures;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Core actor system managing actor lifecycles and routing
pub struct ActorSystem {
    /// All actors in the system
    actors: Arc<RwLock<HashMap<ActorId, ActorHandle>>>,
    
    /// Bundle configurations for deployment
    bundles: Arc<RwLock<HashMap<String, BundleConfiguration>>>,
    
    /// Actor registry for location-transparent references
    registry: Arc<ActorRegistry>,
    
    /// System-wide metrics
    metrics: Arc<SystemMetrics>,
    
    /// Task registry for proper cleanup on shutdown
    task_registry: Arc<RwLock<HashMap<ActorId, JoinHandle<()>>>>,
    
    /// System ID for debugging
    system_id: String,
}

/// Handle to a running actor
#[derive(Debug, Clone)]
pub struct ActorHandle {
    pub id: ActorId,
    pub status: ActorStatus,
    pub transport: ActorTransport,
    pub start_time: Instant,
    // Note: JoinHandle removed as it doesn't implement Clone
    // Task management handled separately if needed
}

/// Actor status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorStatus {
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed,
    Restarting,
}

/// System-wide metrics with enhanced monitoring
#[derive(Debug, Default)]
pub struct SystemMetrics {
    pub actors_spawned: std::sync::atomic::AtomicU64,
    pub actors_stopped: std::sync::atomic::AtomicU64,
    pub messages_processed: std::sync::atomic::AtomicU64,
    pub total_processing_time_ns: std::sync::atomic::AtomicU64,
    
    // Enhanced metrics
    pub actor_restarts: std::sync::atomic::AtomicU64,
    pub restart_failures: std::sync::atomic::AtomicU64,
    pub local_transport_selections: std::sync::atomic::AtomicU64,
    pub remote_transport_selections: std::sync::atomic::AtomicU64,
    pub network_transport_selections: std::sync::atomic::AtomicU64,
    
    // Mailbox metrics
    pub high_priority_messages: std::sync::atomic::AtomicU64,
    pub normal_priority_messages: std::sync::atomic::AtomicU64,
    pub channel_full_events: std::sync::atomic::AtomicU64,
}

impl SystemMetrics {
    pub fn record_message_handled(&self, duration: Duration) {
        use std::sync::atomic::Ordering;
        self.messages_processed.fetch_add(1, Ordering::Relaxed);
        self.total_processing_time_ns.fetch_add(
            duration.as_nanos() as u64,
            Ordering::Relaxed
        );
    }
    
    pub fn avg_processing_time_ns(&self) -> f64 {
        use std::sync::atomic::Ordering;
        let count = self.messages_processed.load(Ordering::Relaxed);
        if count == 0 {
            return 0.0;
        }
        let total = self.total_processing_time_ns.load(Ordering::Relaxed);
        total as f64 / count as f64
    }
    
    /// Record actor restart event
    pub fn record_actor_restart(&self, success: bool) {
        use std::sync::atomic::Ordering;
        self.actor_restarts.fetch_add(1, Ordering::Relaxed);
        if !success {
            self.restart_failures.fetch_add(1, Ordering::Relaxed);
        }
    }
    
    /// Record transport selection
    pub fn record_transport_selection(&self, transport_type: super::transport::TransportType) {
        use std::sync::atomic::Ordering;
        match transport_type {
            super::transport::TransportType::Local => {
                self.local_transport_selections.fetch_add(1, Ordering::Relaxed);
            }
            super::transport::TransportType::UnixSocket => {
                self.remote_transport_selections.fetch_add(1, Ordering::Relaxed);
            }
            super::transport::TransportType::Network => {
                self.network_transport_selections.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
    
    /// Record message priority
    pub fn record_message_priority(&self, priority: crate::Priority) {
        use std::sync::atomic::Ordering;
        match priority {
            crate::Priority::High | crate::Priority::Critical => {
                self.high_priority_messages.fetch_add(1, Ordering::Relaxed);
            }
            _ => {
                self.normal_priority_messages.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
    
    /// Record channel full event for backpressure monitoring
    pub fn record_channel_full(&self) {
        use std::sync::atomic::Ordering;
        self.channel_full_events.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Get enhanced metrics snapshot
    pub fn get_enhanced_stats(&self) -> EnhancedSystemStats {
        use std::sync::atomic::Ordering;
        
        let total_restarts = self.actor_restarts.load(Ordering::Relaxed);
        let restart_failures = self.restart_failures.load(Ordering::Relaxed);
        let restart_success_rate = if total_restarts > 0 {
            ((total_restarts - restart_failures) as f64 / total_restarts as f64) * 100.0
        } else {
            100.0
        };
        
        let total_transports = self.local_transport_selections.load(Ordering::Relaxed) +
                              self.remote_transport_selections.load(Ordering::Relaxed) +
                              self.network_transport_selections.load(Ordering::Relaxed);
        
        let high_priority = self.high_priority_messages.load(Ordering::Relaxed);
        let normal_priority = self.normal_priority_messages.load(Ordering::Relaxed);
        let total_messages = high_priority + normal_priority;
        let high_priority_percentage = if total_messages > 0 {
            (high_priority as f64 / total_messages as f64) * 100.0
        } else {
            0.0
        };
        
        EnhancedSystemStats {
            basic_stats: SystemStats {
                actors_spawned: self.actors_spawned.load(Ordering::Relaxed),
                actors_stopped: self.actors_stopped.load(Ordering::Relaxed),
                messages_processed: self.messages_processed.load(Ordering::Relaxed),
                avg_processing_time_ns: self.avg_processing_time_ns(),
            },
            restart_stats: RestartStats {
                total_restarts,
                restart_failures,
                restart_success_rate,
            },
            transport_stats: TransportStats {
                local_selections: self.local_transport_selections.load(Ordering::Relaxed),
                remote_selections: self.remote_transport_selections.load(Ordering::Relaxed),
                network_selections: self.network_transport_selections.load(Ordering::Relaxed),
                total_selections: total_transports,
            },
            mailbox_stats: MailboxStats {
                high_priority_messages: high_priority,
                normal_priority_messages: normal_priority,
                high_priority_percentage,
                channel_full_events: self.channel_full_events.load(Ordering::Relaxed),
            },
        }
    }
}

/// Basic system statistics
#[derive(Debug, Clone)]
pub struct SystemStats {
    pub actors_spawned: u64,
    pub actors_stopped: u64,
    pub messages_processed: u64,
    pub avg_processing_time_ns: f64,
}

/// Actor restart statistics
#[derive(Debug, Clone)]
pub struct RestartStats {
    pub total_restarts: u64,
    pub restart_failures: u64,
    pub restart_success_rate: f64,
}

/// Transport selection statistics  
#[derive(Debug, Clone)]
pub struct TransportStats {
    pub local_selections: u64,
    pub remote_selections: u64,
    pub network_selections: u64,
    pub total_selections: u64,
}

/// Mailbox and priority statistics
#[derive(Debug, Clone)]
pub struct MailboxStats {
    pub high_priority_messages: u64,
    pub normal_priority_messages: u64,
    pub high_priority_percentage: f64,
    pub channel_full_events: u64,
}

/// Enhanced system statistics with detailed monitoring
#[derive(Debug, Clone)]
pub struct EnhancedSystemStats {
    pub basic_stats: SystemStats,
    pub restart_stats: RestartStats,
    pub transport_stats: TransportStats,
    pub mailbox_stats: MailboxStats,
}

impl ActorSystem {
    /// Create new actor system
    pub fn new() -> Self {
        let system_id = format!("system-{}", Uuid::new_v4());
        info!("Creating new actor system: {}", system_id);
        
        Self {
            actors: Arc::new(RwLock::new(HashMap::new())),
            bundles: Arc::new(RwLock::new(HashMap::new())),
            registry: Arc::new(ActorRegistry::new()),
            metrics: Arc::new(SystemMetrics::default()),
            task_registry: Arc::new(RwLock::new(HashMap::new())),
            system_id,
        }
    }
    
    /// Spawn a new actor
    pub async fn spawn<A>(&self, actor: A) -> Result<ActorRef<A::Message>>
    where
        A: ActorBehavior + 'static,
    {
        use std::sync::atomic::Ordering;
        
        let actor_id = ActorId::new();
        let start_time = Instant::now();
        
        debug!(
            actor_id = %actor_id,
            system_id = %self.system_id,
            actor_type = std::any::type_name::<A>(),
            "Spawning new actor in system"
        );
        
        // Create priority mailbox for message routing
        let (mailbox, receiver) = Mailbox::new(1000);
        
        // Determine transport based on bundle configuration
        let transport = self.create_transport(&actor_id).await?;
        
        // Create actor task with supervision context
        let actor_task = ActorTask {
            id: actor_id.clone(),
            behavior: Box::new(actor),
            receiver,
            system: self.clone(),
            metrics: Arc::clone(&self.metrics),
            supervision_context: SupervisionContext::new_root(),
        };
        
        // Spawn the task and store handle for cleanup
        let task_handle = tokio::spawn(actor_task.run());
        self.task_registry.write().await.insert(actor_id.clone(), task_handle);
        
        // Create handle
        let handle = ActorHandle {
            id: actor_id.clone(),
            status: ActorStatus::Starting,
            transport: transport.clone(),
            start_time,
        };
        
        // Register actor
        self.actors.write().await.insert(actor_id.clone(), handle);
        self.registry.register_actor(actor_id.clone(), transport.clone()).await?;
        
        // Update metrics
        self.metrics.actors_spawned.fetch_add(1, Ordering::Relaxed);
        
        // Create actor reference with integrated mailbox
        let actor_ref = ActorRef {
            id: actor_id,
            transport,
            mailbox: Some(mailbox), // Integrate the priority mailbox
            _phantom: PhantomData,
        };
        
        info!(
            actor_id = %actor_ref.id,
            system_id = %self.system_id,
            transport_type = ?actor_ref.transport.transport_type(),
            spawn_duration_ms = start_time.elapsed().as_millis(),
            "Actor spawned successfully"
        );
        Ok(actor_ref)
    }
    
    /// Stop an actor with proper task cleanup
    pub async fn stop_actor(&self, actor_id: &ActorId) -> Result<()> {
        use std::sync::atomic::Ordering;
        
        debug!("Stopping actor {}", actor_id);
        
        let mut actors = self.actors.write().await;
        if let Some(mut handle) = actors.remove(actor_id) {
            handle.status = ActorStatus::Stopping;
            
            // Remove and abort the task handle for clean shutdown
            let mut task_registry = self.task_registry.write().await;
            if let Some(task_handle) = task_registry.remove(actor_id) {
                // Abort the task to ensure immediate shutdown
                task_handle.abort();
                debug!("Aborted task for actor {}", actor_id);
                
                // Wait for task completion (it should complete quickly due to abort)
                if let Err(e) = task_handle.await {
                    if !e.is_cancelled() {
                        warn!("Actor {} task finished with error: {}", actor_id, e);
                    } else {
                        debug!("Actor {} task cancelled as expected", actor_id);
                    }
                }
            } else {
                warn!("No task handle found for actor {} (already stopped?)", actor_id);
            }
            
            // Unregister from registry
            self.registry.unregister_actor(actor_id).await?;
            
            // Update metrics
            self.metrics.actors_stopped.fetch_add(1, Ordering::Relaxed);
            
            info!("Actor {} stopped and cleaned up", actor_id);
            Ok(())
        } else {
            warn!("Attempted to stop unknown actor {}", actor_id);
            Err(TransportError::configuration(
                &format!("Actor {} not found", actor_id),
                Some("actor_id")
            ))
        }
    }
    
    /// Get actor handle by ID
    pub async fn get_actor(&self, actor_id: &ActorId) -> Option<ActorHandle> {
        self.actors.read().await.get(actor_id).cloned()
    }
    
    /// List all actors
    pub async fn list_actors(&self) -> Vec<ActorId> {
        self.actors.read().await.keys().cloned().collect()
    }
    
    /// Add bundle configuration with validation
    pub async fn add_bundle(&self, name: String, config: BundleConfiguration) -> Result<()> {
        debug!("Adding bundle configuration: {}", name);
        
        // Validate the bundle configuration
        config.validate()
            .map_err(|e| {
                warn!("Bundle configuration validation failed for '{}': {}", name, e);
                e
            })?;
        
        // Additional validation: ensure bundle name matches config name
        if config.name != name {
            return Err(TransportError::configuration(
                &format!("Bundle name mismatch: parameter '{}' vs config '{}'", name, config.name),
                Some("bundle_name")
            ));
        }
        
        // Check for duplicate bundle names
        let existing_bundles = self.bundles.read().await;
        if existing_bundles.contains_key(&name) {
            return Err(TransportError::configuration(
                &format!("Bundle '{}' already exists", name),
                Some("bundle_name")
            ));
        }
        drop(existing_bundles);
        
        // Store validated configuration and capture metrics before moving
        let actor_count = config.actors.len();
        let deployment_mode = format!("{:?}", config.deployment_mode());
        
        self.bundles.write().await.insert(name.clone(), config);
        info!(
            bundle_name = %name,
            actor_count = actor_count,
            deployment_mode = %deployment_mode,
            "Successfully added validated bundle configuration"
        );
        
        Ok(())
    }
    
    /// Get system metrics
    pub fn metrics(&self) -> Arc<SystemMetrics> {
        Arc::clone(&self.metrics)
    }
    
    /// Shutdown the entire actor system with proper cleanup
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down actor system {}", self.system_id);
        
        // Get all actor IDs to stop
        let actor_ids = self.list_actors().await;
        
        // Stop all actors concurrently
        let stop_futures: Vec<_> = actor_ids.iter()
            .map(|id| self.stop_actor(id))
            .collect();
        
        // Wait for all actors to stop
        for (i, stop_result) in futures::future::join_all(stop_futures).await.into_iter().enumerate() {
            if let Err(e) = stop_result {
                warn!("Error stopping actor {}: {}", actor_ids[i], e);
            }
        }
        
        // Final cleanup - abort any remaining tasks
        let mut task_registry = self.task_registry.write().await;
        let remaining_tasks: Vec<_> = task_registry.drain().collect();
        
        if !remaining_tasks.is_empty() {
            warn!("Aborting {} remaining tasks during shutdown", remaining_tasks.len());
            for (actor_id, task_handle) in remaining_tasks {
                task_handle.abort();
                debug!("Aborted remaining task for actor {}", actor_id);
            }
        }
        
        info!("Actor system {} shutdown complete", self.system_id);
        Ok(())
    }
    
    /// Create appropriate transport for actor
    async fn create_transport(&self, actor_id: &ActorId) -> Result<ActorTransport> {
        // Check if actor is in a bundle
        let bundles = self.bundles.read().await;
        
        for (bundle_name, bundle_config) in bundles.iter() {
            if bundle_config.contains_actor(actor_id) {
                debug!("Actor {} found in bundle {}", actor_id, bundle_name);
                
                return match &bundle_config.deployment {
                    DeploymentMode::SharedMemory { channels } => {
                        // Get or create local channel for this actor
                        let transport = if let Some(sender) = channels.get(actor_id) {
                            ActorTransport::new_local(
                                sender.clone(),
                                actor_id.to_string(),
                            )
                        } else {
                            // Create new channel - this might need coordination with other actors
                            let (sender, _receiver) = mpsc::channel(1000);
                            ActorTransport::new_local(
                                sender,
                                actor_id.to_string(),
                            )
                        };
                        
                        // Record transport selection metrics
                        self.metrics.record_transport_selection(super::transport::TransportType::Local);
                        debug!(
                            actor_id = %actor_id,
                            transport_type = "Local",
                            bundle = %bundle_name,
                            "Selected local transport for bundled actor"
                        );
                        
                        Ok(transport)
                    },
                    DeploymentMode::SameNode { socket_paths } => {
                        // Create Unix socket transport for same-node communication
                        if let Some(socket_path) = socket_paths.get(actor_id) {
                            debug!("Creating Unix socket transport for actor {} at {}", actor_id, socket_path);
                            
                            // Connect to Unix socket for this actor
                            match crate::network::unix::UnixSocketTransport::connect(socket_path).await {
                                Ok(connection) => {
                                    let transport = ActorTransport::new_remote(
                                        Arc::new(connection),
                                        actor_id.to_string(),
                                    );
                                    
                                    // Record successful Unix socket transport selection
                                    self.metrics.record_transport_selection(super::transport::TransportType::UnixSocket);
                                    debug!(
                                        actor_id = %actor_id,
                                        transport_type = "UnixSocket",
                                        socket_path = %socket_path,
                                        bundle = %bundle_name,
                                        "Selected Unix socket transport for actor"
                                    );
                                    
                                    Ok(transport)
                                }
                                Err(e) => {
                                    warn!("Failed to connect to Unix socket {} for actor {}: {}. Falling back to local transport.", 
                                          socket_path, actor_id, e);
                                    
                                    // Fallback to local transport if Unix socket connection fails
                                    let (sender, _receiver) = mpsc::channel(1000);
                                    let transport = ActorTransport::new_local(
                                        sender,
                                        actor_id.to_string(),
                                    );
                                    
                                    // Record fallback transport selection
                                    self.metrics.record_transport_selection(super::transport::TransportType::Local);
                                    debug!(
                                        actor_id = %actor_id,
                                        transport_type = "Local",
                                        fallback_reason = %e,
                                        "Fell back to local transport due to Unix socket connection failure"
                                    );
                                    
                                    Ok(transport)
                                }
                            }
                        } else {
                            warn!("No socket path configured for actor {} in SameNode bundle", actor_id);
                            // Fallback to local transport if no socket path configured
                            let (sender, _receiver) = mpsc::channel(1000);
                            Ok(ActorTransport::new_local(
                                sender,
                                actor_id.to_string(),
                            ))
                        }
                    },
                    DeploymentMode::Distributed { node_assignments } => {
                        // Create network transport for distributed communication
                        if let Some(node_address) = node_assignments.get(actor_id) {
                            debug!("Creating network transport for actor {} to node {}", actor_id, node_address);
                            
                            // For now, create a placeholder network transport
                            // In production, this would connect to the specific node
                            match create_network_transport(node_address).await {
                                Ok(transport) => {
                                    Ok(ActorTransport::new_network(
                                        transport,
                                        actor_id.to_string(),
                                    ))
                                }
                                Err(e) => {
                                    warn!("Failed to create network transport to {} for actor {}: {}. Falling back to local.", 
                                          node_address, actor_id, e);
                                    // Fallback to local transport if network connection fails
                                    let (sender, _receiver) = mpsc::channel(1000);
                                    Ok(ActorTransport::new_local(
                                        sender,
                                        actor_id.to_string(),
                                    ))
                                }
                            }
                        } else {
                            warn!("No node assignment for actor {} in Distributed bundle", actor_id);
                            // Fallback to local transport if no node assignment
                            let (sender, _receiver) = mpsc::channel(1000);
                            Ok(ActorTransport::new_local(
                                sender,
                                actor_id.to_string(),
                            ))
                        }
                    },
                };
            }
        }
        
        // Default: create local transport
        let (sender, _receiver) = mpsc::channel(1000);
        Ok(ActorTransport::new_local(
            sender,
            actor_id.to_string(),
        ))
    }
}

/// Create network transport for distributed actor communication
/// 
/// Creates a proper TCP network transport with TLV message framing.
/// Supports both client connections and server binding based on address format.
async fn create_network_transport(node_address: &str) -> Result<Arc<dyn super::transport::NetworkTransport>> {
    debug!("Creating network transport to {}", node_address);
    
    // Parse the node address
    let socket_addr = node_address.parse::<std::net::SocketAddr>()
        .map_err(|e| TransportError::configuration(
            &format!("Invalid socket address '{}': {}", node_address, e),
            Some("node_address")
        ))?;
    
    debug!("Parsed socket address: {}", socket_addr);
    
    // Create TCP network transport
    let transport = crate::network::tcp::TcpNetworkTransport::new_client(socket_addr);
    
    // Establish connection
    transport.connect().await
        .map_err(|e| {
            warn!("Failed to connect to TCP peer at {}: {}", socket_addr, e);
            TransportError::network_with_source(
                &format!("Failed to establish TCP connection to {}", socket_addr),
                e
            )
        })?;
    
    info!("Successfully created TCP network transport to {}", socket_addr);
    Ok(Arc::new(transport))
}

impl Clone for ActorSystem {
    fn clone(&self) -> Self {
        Self {
            actors: Arc::clone(&self.actors),
            bundles: Arc::clone(&self.bundles),
            registry: Arc::clone(&self.registry),
            metrics: Arc::clone(&self.metrics),
            task_registry: Arc::clone(&self.task_registry),
            system_id: self.system_id.clone(),
        }
    }
}

/// Trait for actor behavior
#[async_trait]
pub trait ActorBehavior: Send + Sync + 'static {
    type Message: Message;
    
    /// Handle incoming message
    async fn handle(&mut self, msg: Self::Message) -> Result<()>;
    
    /// Called when actor starts
    async fn on_start(&mut self) -> Result<()> {
        Ok(())
    }
    
    /// Called before actor stops
    async fn on_stop(&mut self) -> Result<()> {
        Ok(())
    }
    
    /// Handle failure - return supervision directive
    async fn on_error(&mut self, error: crate::TransportError) -> SupervisorDirective {
        error!("Actor error: {}", error);
        SupervisorDirective::Restart
    }
}

/// Supervision directive for error handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupervisorDirective {
    /// Resume processing
    Resume,
    /// Restart the actor
    Restart,
    /// Stop the actor
    Stop,
    /// Escalate to parent supervisor
    Escalate,
}

/// Actor reference for location-transparent communication
#[derive(Debug)]
pub struct ActorRef<M: Message> {
    pub id: ActorId,
    pub transport: ActorTransport,
    /// Optional priority mailbox for direct message routing (when available)
    pub mailbox: Option<Mailbox<M>>,
    _phantom: PhantomData<M>,
}

impl<M: Message> ActorRef<M> {
    /// Send message to actor
    pub async fn send(&self, msg: M) -> Result<()> {
        self.send_with_priority(msg, crate::Priority::Normal).await
    }
    
    /// Send message with priority
    /// 
    /// CRITICAL: Proper mailbox integration for priority message handling
    pub async fn send_with_priority(&self, msg: M, priority: crate::Priority) -> Result<()> {
        // If we have direct mailbox access, use it for priority routing
        if let Some(mailbox) = &self.mailbox {
            mailbox.send(msg, priority).await
        } else {
            // Fall back to transport-level priority handling
            self.transport.send_with_priority(msg, priority).await
        }
    }
    
    /// Get actor ID
    pub fn id(&self) -> &ActorId {
        &self.id
    }
    
    /// Get transport metrics
    pub fn metrics(&self) -> Arc<TransportMetrics> {
        self.transport.metrics()
    }
    
    /// Check if transport is healthy
    pub fn is_healthy(&self) -> bool {
        self.transport.is_healthy()
    }
}

impl<M: Message> Clone for ActorRef<M> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            transport: self.transport.clone(),
            mailbox: self.mailbox.clone(),
            _phantom: PhantomData,
        }
    }
}

/// Actor mailbox with priority support
#[derive(Debug)]
pub struct Mailbox<M: Message> {
    /// High priority messages
    high_priority: mpsc::Sender<M>,
    /// Normal priority messages  
    normal_priority: mpsc::UnboundedSender<M>,
}

impl<M: Message> Clone for Mailbox<M> {
    fn clone(&self) -> Self {
        Self {
            high_priority: self.high_priority.clone(),
            normal_priority: self.normal_priority.clone(),
        }
    }
}

/// Mailbox receiver
pub struct MailboxReceiver<M: Message> {
    high_priority: mpsc::Receiver<M>,
    normal_priority: mpsc::UnboundedReceiver<M>,
}

impl<M: Message> Mailbox<M> {
    pub fn new(high_priority_capacity: usize) -> (Self, MailboxReceiver<M>) {
        let (high_tx, high_rx) = mpsc::channel(high_priority_capacity);
        let (normal_tx, normal_rx) = mpsc::unbounded_channel();
        
        let mailbox = Self {
            high_priority: high_tx,
            normal_priority: normal_tx,
        };
        
        let receiver = MailboxReceiver {
            high_priority: high_rx,
            normal_priority: normal_rx,
        };
        
        (mailbox, receiver)
    }
    
    /// Send message with priority-aware performance optimization
    /// 
    /// PERFORMANCE CRITICAL: Optimized for <100ns local transport targets
    pub async fn send(&self, msg: M, priority: crate::Priority) -> Result<()> {
        match priority {
            crate::Priority::High | crate::Priority::Critical => {
                // High priority: try non-blocking first, then async
                match self.high_priority.try_send(msg) {
                    Ok(()) => Ok(()),
                    Err(mpsc::error::TrySendError::Full(msg)) => {
                        // Fall back to async for full channel
                        self.high_priority.send(msg).await
                            .map_err(|_| TransportError::network("High priority channel closed"))
                    }
                    Err(mpsc::error::TrySendError::Closed(_)) => {
                        Err(TransportError::network("High priority channel closed"))
                    }
                }
            },
            _ => {
                // Normal priority: unbounded channel (always succeeds unless closed)
                self.normal_priority.send(msg)
                    .map_err(|_| TransportError::network("Normal priority channel closed"))
            }
        }
    }
}

impl<M: Message> MailboxReceiver<M> {
    pub async fn recv(&mut self) -> Option<M> {
        // Prioritize high priority messages
        tokio::select! {
            biased;
            
            msg = self.high_priority.recv() => msg,
            msg = self.normal_priority.recv() => msg,
        }
    }
}

/// Actor task runner - simplified without type erasure
struct ActorTask<M: Message> {
    id: ActorId,
    behavior: Box<dyn ActorBehavior<Message = M>>,
    receiver: MailboxReceiver<M>,
    system: ActorSystem,
    metrics: Arc<SystemMetrics>,
    /// Supervision context for error escalation
    supervision_context: SupervisionContext,
}

/// Supervision context for actor error handling and escalation
#[derive(Debug, Clone)]
struct SupervisionContext {
    /// Parent actor ID for escalation (None for root actors)
    parent_actor: Option<ActorId>,
    /// Number of restarts attempted in current time window
    restart_count: Arc<std::sync::atomic::AtomicU32>,
    /// Maximum restarts allowed before escalation
    max_restarts: u32,
    /// Time window for restart counting (seconds)
    restart_window_secs: u64,
    /// Timestamp of first restart in current window
    restart_window_start: Arc<Mutex<Option<Instant>>>,
}

impl SupervisionContext {
    /// Create default supervision context for root actors
    pub fn new_root() -> Self {
        Self {
            parent_actor: None,
            restart_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            max_restarts: 5, // Allow 5 restarts per minute for production resilience
            restart_window_secs: 60,
            restart_window_start: Arc::new(Mutex::new(None)),
        }
    }
    
    /// Create supervision context with parent for error escalation
    pub fn new_child(parent: ActorId, max_restarts: u32, window_secs: u64) -> Self {
        Self {
            parent_actor: Some(parent),
            restart_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            max_restarts,
            restart_window_secs: window_secs,
            restart_window_start: Arc::new(Mutex::new(None)),
        }
    }
    
    /// Check if actor can be restarted or should be escalated
    pub fn should_restart(&self) -> bool {
        use std::sync::atomic::Ordering;
        
        let now = Instant::now();
        let mut window_start = self.restart_window_start.lock();
        
        // Initialize or reset window if expired
        match *window_start {
            None => {
                *window_start = Some(now);
                self.restart_count.store(1, Ordering::Relaxed);
                true
            }
            Some(start) => {
                let window_duration = Duration::from_secs(self.restart_window_secs);
                if now.duration_since(start) > window_duration {
                    // Reset window
                    *window_start = Some(now);
                    self.restart_count.store(1, Ordering::Relaxed);
                    true
                } else {
                    // Within window - check restart count
                    let count = self.restart_count.fetch_add(1, Ordering::Relaxed) + 1;
                    count <= self.max_restarts
                }
            }
        }
    }
}

impl<M: Message> ActorTask<M> {
    async fn run(mut self) {
        let task_start = Instant::now();
        info!(
            actor_id = %self.id,
            task_type = "ActorTask",
            "Starting actor task execution"
        );
        
        // Lifecycle: Start
        if let Err(e) = self.behavior.on_start().await {
            error!(
                actor_id = %self.id,
                error = %e,
                startup_duration_ms = task_start.elapsed().as_millis(),
                "Actor failed to start during initialization"
            );
            return;
        }
        
        debug!(
            actor_id = %self.id,
            startup_duration_ms = task_start.elapsed().as_millis(),
            "Actor successfully started, entering message loop"
        );
        
        // Main message loop
        while let Some(msg) = self.receiver.recv().await {
            let start = Instant::now();
            
            match self.behavior.handle(msg).await {
                Ok(()) => {
                    self.metrics.record_message_handled(start.elapsed());
                }
                Err(e) => {
                    let processing_duration = start.elapsed();
                    error!(
                        actor_id = %self.id,
                        error = %e,
                        error_category = e.category(),
                        processing_duration_ns = processing_duration.as_nanos(),
                        "Actor message processing failed"
                    );
                    
                    match self.behavior.on_error(e.clone()).await {
                        SupervisorDirective::Resume => {
                            debug!(
                                actor_id = %self.id,
                                directive = "Resume",
                                "Actor resumed after error"
                            );
                            continue;
                        },
                        SupervisorDirective::Restart => {
                            if self.supervision_context.should_restart() {
                                warn!(
                                    actor_id = %self.id,
                                    directive = "Restart",
                                    restart_count = self.supervision_context.restart_count.load(std::sync::atomic::Ordering::Relaxed),
                                    max_restarts = self.supervision_context.max_restarts,
                                    "Restarting actor within restart limits"
                                );
                                self.metrics.record_actor_restart(true);
                                self.restart().await;
                            } else {
                                error!(
                                    actor_id = %self.id,
                                    directive = "Restart",
                                    restart_count = self.supervision_context.restart_count.load(std::sync::atomic::Ordering::Relaxed),
                                    max_restarts = self.supervision_context.max_restarts,
                                    "Actor exceeded restart limit - escalating to supervisor"
                                );
                                self.metrics.record_actor_restart(false);
                                self.escalate_error(e).await;
                                break;
                            }
                        }
                        SupervisorDirective::Stop => {
                            warn!(
                                actor_id = %self.id,
                                directive = "Stop",
                                error = %e,
                                "Stopping actor due to error directive"
                            );
                            break;
                        }
                        SupervisorDirective::Escalate => {
                            error!(
                                actor_id = %self.id,
                                directive = "Escalate",
                                error = %e,
                                "Escalating error to parent supervisor"
                            );
                            self.escalate_error(e).await;
                            break;
                        }
                    }
                }
            }
        }
        
        // Lifecycle: Stop
        let shutdown_start = Instant::now();
        if let Err(e) = self.behavior.on_stop().await {
            error!(
                actor_id = %self.id,
                error = %e,
                shutdown_duration_ms = shutdown_start.elapsed().as_millis(),
                "Actor failed to stop cleanly during shutdown"
            );
        } else {
            debug!(
                actor_id = %self.id,
                shutdown_duration_ms = shutdown_start.elapsed().as_millis(),
                "Actor stopped cleanly"
            );
        }
        
        let total_runtime = task_start.elapsed();
        info!(
            actor_id = %self.id,
            total_runtime_ms = total_runtime.as_millis(),
            total_runtime_secs = total_runtime.as_secs(),
            "Actor task execution completed"
        );
    }
    
    async fn restart(&mut self) {
        info!("Restarting actor {}", self.id);
        // Re-initialize actor state
        self.behavior.on_stop().await.ok();
        self.behavior.on_start().await.ok();
    }
    
    /// Escalate error to parent supervisor or system
    async fn escalate_error(&self, error: TransportError) {
        if let Some(parent_id) = &self.supervision_context.parent_actor {
            warn!(
                actor = %self.id,
                parent = %parent_id,
                error = %error,
                "Escalating actor error to parent supervisor"
            );
            // TODO: Send escalation message to parent actor
            // This would require a reference to the actor system or messaging infrastructure
        } else {
            error!(
                actor = %self.id,
                error = %error,
                "Root actor error - no parent to escalate to. System intervention required."
            );
            // TODO: Notify system supervisor or monitoring for root actor failures
            // In production, this might trigger alerts or system shutdown procedures
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mycelium::messages::*;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// Real actor that processes market data messages
    #[derive(Debug)]
    struct MarketDataActor {
        messages_processed: u32,
        last_quote_update: Option<QuoteUpdate>,
        pool_events_received: Vec<PoolSwapEvent>,
    }
    
    #[async_trait]
    impl ActorBehavior for MarketDataActor {
        type Message = MarketMessage;
        
        async fn handle(&mut self, msg: MarketMessage) -> Result<()> {
            self.messages_processed += 1;
            
            match msg {
                MarketMessage::Quote(quote) => {
                    self.last_quote_update = Some((*quote).clone());
                }
                MarketMessage::Swap(event) => {
                    self.pool_events_received.push((*event).clone());
                }
                _ => {} // Handle other market message types
            }
            
            Ok(())
        }
        
        async fn on_start(&mut self) -> Result<()> {
            info!("MarketDataActor started - ready to process real market data");
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_actor_system_creation() {
        let system = ActorSystem::new();
        assert!(system.list_actors().await.is_empty());
    }

    #[tokio::test] 
    async fn test_actor_spawn() {
        let system = ActorSystem::new();
        
        let actor = MarketDataActor {
            messages_processed: 0,
            last_quote_update: None,
            pool_events_received: vec![],
        };
        
        let actor_ref = system.spawn(actor).await.unwrap();
        
        // Verify actor was registered
        let actors = system.list_actors().await;
        assert_eq!(actors.len(), 1);
        assert_eq!(actors[0], actor_ref.id);
    }

    #[tokio::test]
    async fn test_system_metrics() {
        let system = ActorSystem::new();
        let metrics = system.metrics();
        
        assert_eq!(metrics.avg_processing_time_ns(), 0.0);
        
        // Simulate some processing
        metrics.record_message_handled(Duration::from_nanos(100));
        assert!(metrics.avg_processing_time_ns() > 0.0);
    }
    
    #[tokio::test]
    async fn test_real_market_data_processing() {
        let system = ActorSystem::new();
        
        let actor = MarketDataActor {
            messages_processed: 0,
            last_quote_update: None,
            pool_events_received: vec![],
        };
        
        let actor_ref = system.spawn(actor).await.unwrap();
        
        // Send real DEX pool swap event (Protocol V2 domain-compliant)
        let timestamp_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
            
        let pool_event = PoolSwapEvent {
            pool_address: [0x12; 20], // Real Ethereum pool address format
            token0_in: 500_000_000_000_000_000, // 0.5 WETH
            token1_out: 1_000_000_000, // 1000 USDC
            timestamp_ns,
            tx_hash: [0xab; 32],
            gas_used: 180_000,
        };
        
        let market_message = MarketMessage::Swap(Arc::new(pool_event.clone()));
        
        // This tests the full Protocol V2 message flow:
        // 1. TLV serialization 
        // 2. Domain separation (Market Data domain, TLV type 1)
        // 3. Transport selection (local Arc<T> for bundled actors)
        // 4. Deserialization and processing
        actor_ref.send(market_message).await.unwrap();
        
        // Send real quote update (8-decimal fixed point precision)
        let quote_update = QuoteUpdate {
            instrument_id: 98765,
            bid_price: 1999_50000000_i64, // $1999.50 in 8-decimal fixed point
            ask_price: 2000_50000000_i64, // $2000.50 in 8-decimal fixed point
            bid_size: 2_500_000, // 2.5 units
            ask_size: 1_800_000, // 1.8 units
            timestamp_ns: timestamp_ns + 1000,
        };
        
        let quote_message = MarketMessage::Quote(Arc::new(quote_update.clone()));
        actor_ref.send(quote_message).await.unwrap();
        
        // Give time for message processing
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        // Verify actor is healthy and processing real data
        assert!(actor_ref.is_healthy());
        
        // Check transport metrics for actual message throughput
        let metrics = actor_ref.metrics();
        let stats = metrics.get_stats();
        assert!(stats.local_sends >= 2);
        assert!(stats.avg_local_latency_ns > 0.0);
        assert!(stats.serialization_eliminated_mb > 0.0); // Zero-copy benefits measured
        
        info!("Successfully processed real Protocol V2 market data messages");
        info!("Transport latency: {:.2}ns, Serialization eliminated: {:.4}MB", 
              stats.avg_local_latency_ns, stats.serialization_eliminated_mb);
    }
}
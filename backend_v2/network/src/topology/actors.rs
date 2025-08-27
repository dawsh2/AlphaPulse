//! Actor State Management and Definitions
//!
//! Defines logical actors with state persistence and recovery capabilities

use super::error::{Result, TopologyError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

/// Logical actor definition with state management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    pub id: String,
    pub actor_type: ActorType,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub source_id: u8, // Maps to protocol_v2 SourceType when integrated

    // State management
    pub state: ActorState,
    pub persistence: ActorPersistence,

    // Resource requirements
    pub resources: ResourceRequirements,

    // Health and monitoring
    pub health_check: HealthCheckConfig,

    // Configuration
    pub config: HashMap<String, serde_yaml::Value>,
}

/// Type of actor in the topology
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ActorType {
    /// Data sources (collectors, feeds)
    Producer,
    /// Processing services (strategies, analyzers)
    Transformer,
    /// Data sinks (executors, databases)
    Consumer,
}

/// Actor state management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorState {
    pub state_type: ActorStateType,
    pub checkpoint_interval: Duration,
    pub max_state_size: usize,
    pub compression: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActorStateType {
    /// Stateless actor (no state to persist)
    Stateless,
    /// In-memory state only (lost on restart)
    InMemory,
    /// Persistent state with automatic checkpointing
    Persistent {
        storage_backend: StorageBackend,
        consistency_level: ConsistencyLevel,
    },
    /// Replicated state across multiple nodes
    Replicated {
        replication_factor: usize,
        storage_backend: StorageBackend,
    },
}

/// Storage backend for persistent state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageBackend {
    /// Local filesystem storage
    LocalFile {
        base_path: PathBuf,
        sync_writes: bool,
    },
    /// Distributed key-value store
    DistributedKV { endpoint: String, namespace: String },
    /// Database storage
    Database {
        connection_string: String,
        table_name: String,
    },
}

/// Consistency level for state operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsistencyLevel {
    /// Best effort, may lose recent updates
    Eventual,
    /// Synchronous writes, strong consistency
    Strong,
    /// Quorum-based consistency
    Quorum,
}

/// Persistence configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorPersistence {
    pub enabled: bool,
    pub recovery_strategy: RecoveryStrategy,
    pub backup_retention: BackupRetention,
}

/// Recovery strategy for actor failures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryStrategy {
    /// Restart from last checkpoint
    FromCheckpoint { max_data_loss: Duration },
    /// Restart from beginning
    FromBeginning,
    /// Custom recovery procedure
    Custom { recovery_script: String },
    /// Fail-over to backup actor
    Failover { backup_actors: Vec<String> },
}

/// Backup retention policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupRetention {
    pub max_backups: usize,
    pub retention_period: Duration,
    pub compression: bool,
}

/// Resource requirements for actor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub min_memory_mb: usize,
    pub max_memory_mb: Option<usize>,
    pub min_cpu_cores: usize,
    pub max_cpu_cores: Option<usize>,
    pub disk_space_mb: Option<usize>,
    pub network_bandwidth_mbps: Option<usize>,
    pub gpu_required: bool,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    pub enabled: bool,
    pub check_interval: Duration,
    pub timeout: Duration,
    pub failure_threshold: usize,
    pub recovery_threshold: usize,
    pub check_type: HealthCheckType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthCheckType {
    /// Simple heartbeat ping
    Heartbeat,
    /// HTTP endpoint check
    Http { endpoint: String },
    /// Custom health check script
    Custom { script: String },
    /// Check message processing rate
    ProcessingRate { min_messages_per_second: f64 },
}

/// Runtime state of an actor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorRuntime {
    pub actor_id: String,
    pub instance_id: Uuid,
    pub node_id: String,
    pub status: ActorStatus,
    pub start_time: SystemTime,
    pub last_checkpoint: Option<SystemTime>,
    pub metrics: ActorMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActorStatus {
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed { reason: String },
    Recovering,
}

/// Actor performance metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActorMetrics {
    pub messages_processed: u64,
    pub messages_per_second: f64,
    pub error_count: u64,
    pub memory_usage_mb: usize,
    pub cpu_usage_percent: f64,
    pub last_activity: Option<SystemTime>,
}

impl Actor {
    /// Create a new actor with defaults
    pub fn new(id: String, actor_type: ActorType, source_id: u8) -> Self {
        Self {
            id,
            actor_type,
            inputs: Vec::new(),
            outputs: Vec::new(),
            source_id,
            state: ActorState::default(),
            persistence: ActorPersistence::default(),
            resources: ResourceRequirements::default(),
            health_check: HealthCheckConfig::default(),
            config: HashMap::new(),
        }
    }

    /// Add input channel
    pub fn with_input(mut self, channel: String) -> Self {
        self.inputs.push(channel);
        self
    }

    /// Add output channel
    pub fn with_output(mut self, channel: String) -> Self {
        self.outputs.push(channel);
        self
    }

    /// Set state management type
    pub fn with_state(mut self, state_type: ActorStateType) -> Self {
        self.state.state_type = state_type;
        self
    }

    /// Set resource requirements
    pub fn with_resources(mut self, resources: ResourceRequirements) -> Self {
        self.resources = resources;
        self
    }

    /// Validate actor configuration
    pub fn validate(&self) -> Result<()> {
        // Validate ID
        if self.id.is_empty() {
            return Err(TopologyError::Validation {
                message: "Actor ID cannot be empty".to_string(),
            });
        }

        // Validate inputs/outputs based on type
        match self.actor_type {
            ActorType::Producer => {
                if !self.inputs.is_empty() {
                    return Err(TopologyError::Validation {
                        message: format!("Producer actor '{}' should not have inputs", self.id),
                    });
                }
                if self.outputs.is_empty() {
                    return Err(TopologyError::Validation {
                        message: format!("Producer actor '{}' must have outputs", self.id),
                    });
                }
            }
            ActorType::Consumer => {
                if self.inputs.is_empty() {
                    return Err(TopologyError::Validation {
                        message: format!("Consumer actor '{}' must have inputs", self.id),
                    });
                }
                if !self.outputs.is_empty() {
                    return Err(TopologyError::Validation {
                        message: format!("Consumer actor '{}' should not have outputs", self.id),
                    });
                }
            }
            ActorType::Transformer => {
                if self.inputs.is_empty() {
                    return Err(TopologyError::Validation {
                        message: format!("Transformer actor '{}' must have inputs", self.id),
                    });
                }
                if self.outputs.is_empty() {
                    return Err(TopologyError::Validation {
                        message: format!("Transformer actor '{}' must have outputs", self.id),
                    });
                }
            }
        }

        // Validate resource requirements
        if self.resources.min_memory_mb == 0 {
            return Err(TopologyError::Validation {
                message: format!("Actor '{}' must specify minimum memory", self.id),
            });
        }

        if self.resources.min_cpu_cores == 0 {
            return Err(TopologyError::Validation {
                message: format!("Actor '{}' must specify minimum CPU cores", self.id),
            });
        }

        // Validate state configuration
        self.state.validate()?;

        Ok(())
    }

    /// Check if actor requires persistent state
    pub fn requires_persistence(&self) -> bool {
        matches!(
            self.state.state_type,
            ActorStateType::Persistent { .. } | ActorStateType::Replicated { .. }
        )
    }

    /// Get checkpoint interval
    pub fn checkpoint_interval(&self) -> Duration {
        self.state.checkpoint_interval
    }
}

impl Default for ActorState {
    fn default() -> Self {
        Self {
            state_type: ActorStateType::Stateless,
            checkpoint_interval: Duration::from_secs(60),
            max_state_size: 1024 * 1024 * 1024, // 1GB
            compression: true,
        }
    }
}

impl ActorState {
    fn validate(&self) -> Result<()> {
        if self.checkpoint_interval.as_secs() == 0 {
            return Err(TopologyError::Validation {
                message: "Checkpoint interval must be greater than 0".to_string(),
            });
        }

        if self.max_state_size == 0 {
            return Err(TopologyError::Validation {
                message: "Maximum state size must be greater than 0".to_string(),
            });
        }

        Ok(())
    }
}

impl Default for ActorPersistence {
    fn default() -> Self {
        Self {
            enabled: false,
            recovery_strategy: RecoveryStrategy::FromCheckpoint {
                max_data_loss: Duration::from_secs(60),
            },
            backup_retention: BackupRetention {
                max_backups: 10,
                retention_period: Duration::from_secs(7 * 24 * 3600), // 7 days
                compression: true,
            },
        }
    }
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self {
            min_memory_mb: 256,
            max_memory_mb: Some(1024),
            min_cpu_cores: 1,
            max_cpu_cores: Some(4),
            disk_space_mb: Some(1024),
            network_bandwidth_mbps: Some(100),
            gpu_required: false,
        }
    }
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            check_interval: Duration::from_secs(30),
            timeout: Duration::from_secs(5),
            failure_threshold: 3,
            recovery_threshold: 2,
            check_type: HealthCheckType::Heartbeat,
        }
    }
}

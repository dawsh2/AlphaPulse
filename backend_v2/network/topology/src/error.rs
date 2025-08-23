//! Error types for the topology system

use thiserror::Error;

pub type Result<T> = std::result::Result<T, TopologyError>;

#[derive(Error, Debug)]
pub enum TopologyError {
    #[error("Configuration error: {message}")]
    Config { message: String },

    #[error("Actor '{actor}' references missing channel '{channel}'")]
    MissingChannel { actor: String, channel: String },

    #[error("Actor '{actor}' not found in topology")]
    ActorNotFound { actor: String },

    #[error("Node '{node}' not found in topology")]
    NodeNotFound { node: String },

    #[error("Invalid CPU assignment for actor '{actor}': {reason}")]
    InvalidCpuAssignment { actor: String, reason: String },

    #[error("Invalid NUMA configuration for node '{node}': {reason}")]
    InvalidNumaConfig { node: String, reason: String },

    #[error("Channel '{channel}' has conflicting configurations")]
    ConflictingChannelConfig { channel: String },

    #[error("Network configuration error: {message}")]
    NetworkConfig { message: String },

    #[error("Actor state error: {message}")]
    ActorState { message: String },

    #[error("Deployment error: {message}")]
    Deployment { message: String },

    #[error("Validation error: {message}")]
    Validation { message: String },

    #[error("Resource constraint violation: {message}")]
    ResourceConstraint { message: String },

    #[error("Transport resolution failed: {reason}")]
    TransportResolution { reason: String },

    #[error("Unsupported actor type: {actor_type}")]
    UnsupportedActorType { actor_type: String },

    #[error("YAML parsing error: {0}")]
    YamlParse(#[from] serde_yaml::Error),

    #[error("JSON parsing error: {0}")]
    JsonParse(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("System error: {0}")]
    System(#[from] nix::Error),
}

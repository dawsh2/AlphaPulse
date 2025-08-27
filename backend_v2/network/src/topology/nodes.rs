//! Node Configuration and Physical Deployment
//!
//! Defines physical nodes, hardware topology, and actor placement

use super::error::{Result, TopologyError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Physical node configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub hostname: String,
    pub numa_topology: Vec<u8>,
    pub local_channels: HashMap<String, ChannelConfig>,
    pub actor_placements: HashMap<String, ActorPlacement>,
    pub node_resources: NodeResources,
    pub monitoring: NodeMonitoring,
}

/// Local channel configuration for intra-node communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    pub channel_type: ChannelType,
    pub buffer_size: usize,
    pub numa_node: Option<u8>,
    pub huge_pages: bool,
    pub priority: ChannelPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChannelType {
    /// Single Producer Multiple Consumer
    SPMC,
    /// Multiple Producer Single Consumer  
    MPSC,
    /// Multiple Producer Multiple Consumer
    MPMC,
    /// Single Producer Single Consumer (fastest)
    SPSC,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChannelPriority {
    Low,
    Normal,
    High,
    RealTime,
}

/// Actor placement on a node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorPlacement {
    pub numa: Option<u8>,
    pub cpu: Vec<u8>,
    pub memory_limit_mb: Option<usize>,
    pub priority: ProcessPriority,
    pub isolation: ProcessIsolation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessPriority {
    Low,
    Normal,
    High,
    RealTime { policy: RtPolicy },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RtPolicy {
    FIFO,
    RoundRobin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessIsolation {
    pub cpu_affinity: bool,
    pub memory_isolation: bool,
    pub network_namespace: Option<String>,
    pub cgroup: Option<String>,
}

/// Node hardware resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeResources {
    pub total_memory_mb: usize,
    pub total_cpu_cores: usize,
    pub numa_nodes: Vec<NumaNode>,
    pub network_interfaces: Vec<NetworkInterface>,
    pub storage: Vec<StorageDevice>,
    pub gpu: Option<GpuInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NumaNode {
    pub id: u8,
    pub memory_mb: usize,
    pub cpu_cores: Vec<u8>,
    pub local_storage: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub bandwidth_mbps: u64,
    pub mtu: u16,
    pub numa_node: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageDevice {
    pub path: String,
    pub device_type: StorageType,
    pub capacity_mb: u64,
    pub iops: Option<u32>,
    pub numa_node: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageType {
    HDD,
    SSD,
    NVMe,
    RAM,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub model: String,
    pub memory_mb: usize,
    pub compute_capability: String,
}

/// Node monitoring and health configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMonitoring {
    pub metrics_collection: MetricsConfig,
    pub health_checks: Vec<NodeHealthCheck>,
    pub alerting: AlertingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub collection_interval: std::time::Duration,
    pub retention_period: std::time::Duration,
    pub export_endpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeHealthCheck {
    pub check_type: NodeHealthCheckType,
    pub interval: std::time::Duration,
    pub timeout: std::time::Duration,
    pub threshold: HealthThreshold,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeHealthCheckType {
    CpuUsage,
    MemoryUsage,
    DiskUsage { path: String },
    NetworkLatency { target: String },
    ProcessCount,
    LoadAverage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthThreshold {
    pub warning: f64,
    pub critical: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertingConfig {
    pub enabled: bool,
    pub webhook_url: Option<String>,
    pub email_recipients: Vec<String>,
    pub severity_levels: Vec<AlertSeverity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
    Emergency,
}

/// Inter-node configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterNodeConfig {
    pub routes: Vec<InterNodeRoute>,
    pub default_transport: super::transport::NetworkProtocol,
    pub service_discovery: Option<ServiceDiscoveryConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterNodeRoute {
    pub source_node: String,
    pub target_node: String,
    pub channels: Vec<String>,
    pub transport_override: Option<super::transport::NetworkProtocol>,
    pub bandwidth_limit_mbps: Option<u64>,
    pub latency_target_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDiscoveryConfig {
    pub provider: ServiceDiscoveryProvider,
    pub endpoint: String,
    pub namespace: String,
    pub ttl: std::time::Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceDiscoveryProvider {
    Consul,
    Etcd,
    Zookeeper,
    Kubernetes,
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self {
            channel_type: ChannelType::SPMC,
            buffer_size: 64 * 1024 * 1024, // 64MB
            numa_node: None,
            huge_pages: false,
            priority: ChannelPriority::Normal,
        }
    }
}

impl Default for ActorPlacement {
    fn default() -> Self {
        Self {
            numa: None,
            cpu: vec![0], // Default to CPU 0
            memory_limit_mb: None,
            priority: ProcessPriority::Normal,
            isolation: ProcessIsolation {
                cpu_affinity: true,
                memory_isolation: false,
                network_namespace: None,
                cgroup: None,
            },
        }
    }
}

impl Default for NodeResources {
    fn default() -> Self {
        Self {
            total_memory_mb: 8 * 1024, // 8GB
            total_cpu_cores: 4,
            numa_nodes: vec![NumaNode {
                id: 0,
                memory_mb: 8 * 1024,
                cpu_cores: vec![0, 1, 2, 3],
                local_storage: vec![],
            }],
            network_interfaces: vec![NetworkInterface {
                name: "eth0".to_string(),
                bandwidth_mbps: 1000, // 1Gbps
                mtu: 1500,
                numa_node: Some(0),
            }],
            storage: vec![],
            gpu: None,
        }
    }
}

impl Default for NodeMonitoring {
    fn default() -> Self {
        Self {
            metrics_collection: MetricsConfig {
                enabled: true,
                collection_interval: std::time::Duration::from_secs(30),
                retention_period: std::time::Duration::from_secs(24 * 3600), // 24 hours
                export_endpoint: None,
            },
            health_checks: vec![
                NodeHealthCheck {
                    check_type: NodeHealthCheckType::CpuUsage,
                    interval: std::time::Duration::from_secs(10),
                    timeout: std::time::Duration::from_secs(5),
                    threshold: HealthThreshold {
                        warning: 80.0,
                        critical: 95.0,
                    },
                },
                NodeHealthCheck {
                    check_type: NodeHealthCheckType::MemoryUsage,
                    interval: std::time::Duration::from_secs(10),
                    timeout: std::time::Duration::from_secs(5),
                    threshold: HealthThreshold {
                        warning: 85.0,
                        critical: 95.0,
                    },
                },
            ],
            alerting: AlertingConfig {
                enabled: false,
                webhook_url: None,
                email_recipients: vec![],
                severity_levels: vec![
                    AlertSeverity::Warning,
                    AlertSeverity::Critical,
                    AlertSeverity::Emergency,
                ],
            },
        }
    }
}

impl Node {
    /// Create a new node with basic configuration
    pub fn new(hostname: String) -> Self {
        Self {
            hostname,
            numa_topology: vec![0], // Single NUMA node by default
            local_channels: HashMap::new(),
            actor_placements: HashMap::new(),
            node_resources: NodeResources::default(),
            monitoring: NodeMonitoring::default(),
        }
    }

    /// Add a channel to this node
    pub fn with_channel(mut self, name: String, config: ChannelConfig) -> Self {
        self.local_channels.insert(name, config);
        self
    }

    /// Add an actor placement
    pub fn with_actor(mut self, actor_id: String, placement: ActorPlacement) -> Self {
        self.actor_placements.insert(actor_id, placement);
        self
    }

    /// Set NUMA topology
    pub fn with_numa_topology(mut self, topology: Vec<u8>) -> Self {
        self.numa_topology = topology;
        self
    }

    /// Validate node configuration
    pub fn validate(&self) -> Result<()> {
        // Validate hostname
        if self.hostname.is_empty() {
            return Err(TopologyError::Config {
                message: "Node hostname cannot be empty".to_string(),
            });
        }

        // Validate NUMA topology
        for &numa_id in &self.numa_topology {
            if numa_id > 7 {
                return Err(TopologyError::InvalidNumaConfig {
                    node: self.hostname.clone(),
                    reason: format!("NUMA node ID {} too large (max 7)", numa_id),
                });
            }
        }

        // Validate actor placements
        for (actor_id, placement) in &self.actor_placements {
            self.validate_actor_placement(actor_id, placement)?;
        }

        // Validate channels
        for (channel_name, channel_config) in &self.local_channels {
            self.validate_channel_config(channel_name, channel_config)?;
        }

        Ok(())
    }

    fn validate_actor_placement(&self, actor_id: &str, placement: &ActorPlacement) -> Result<()> {
        // Validate NUMA assignment
        if let Some(numa_node) = placement.numa {
            if !self.numa_topology.contains(&numa_node) {
                return Err(TopologyError::InvalidCpuAssignment {
                    actor: actor_id.to_string(),
                    reason: format!(
                        "NUMA node {} not in topology {:?}",
                        numa_node, self.numa_topology
                    ),
                });
            }
        }

        // Validate CPU assignment
        if placement.cpu.is_empty() {
            return Err(TopologyError::InvalidCpuAssignment {
                actor: actor_id.to_string(),
                reason: "No CPU cores assigned".to_string(),
            });
        }

        for &cpu_core in &placement.cpu {
            if cpu_core as usize >= self.node_resources.total_cpu_cores {
                return Err(TopologyError::InvalidCpuAssignment {
                    actor: actor_id.to_string(),
                    reason: format!(
                        "CPU core {} exceeds available cores {}",
                        cpu_core, self.node_resources.total_cpu_cores
                    ),
                });
            }
        }

        // Validate memory limit
        if let Some(memory_limit) = placement.memory_limit_mb {
            if memory_limit > self.node_resources.total_memory_mb {
                return Err(TopologyError::ResourceConstraint {
                    message: format!(
                        "Actor '{}' memory limit {}MB exceeds node capacity {}MB",
                        actor_id, memory_limit, self.node_resources.total_memory_mb
                    ),
                });
            }
        }

        Ok(())
    }

    fn validate_channel_config(&self, channel_name: &str, config: &ChannelConfig) -> Result<()> {
        // Validate buffer size
        if config.buffer_size == 0 {
            return Err(TopologyError::ConflictingChannelConfig {
                channel: channel_name.to_string(),
            });
        }

        // Validate NUMA assignment
        if let Some(numa_node) = config.numa_node {
            if !self.numa_topology.contains(&numa_node) {
                return Err(TopologyError::ConflictingChannelConfig {
                    channel: format!("{} (invalid NUMA node {})", channel_name, numa_node),
                });
            }
        }

        // Validate huge pages with NUMA
        if config.huge_pages && config.numa_node.is_none() {
            // Warning: this is allowed but not optimal
        }

        Ok(())
    }

    /// Get total CPU cores allocated to actors
    pub fn allocated_cpu_cores(&self) -> usize {
        let mut cores = std::collections::HashSet::new();
        for placement in self.actor_placements.values() {
            for &cpu in &placement.cpu {
                cores.insert(cpu);
            }
        }
        cores.len()
    }

    /// Get total memory allocated to actors
    pub fn allocated_memory_mb(&self) -> usize {
        self.actor_placements
            .values()
            .filter_map(|placement| placement.memory_limit_mb)
            .sum()
    }

    /// Check if this node has available resources for new actor
    pub fn has_capacity_for(&self, cpu_cores: usize, memory_mb: usize) -> bool {
        let available_cores = self.node_resources.total_cpu_cores - self.allocated_cpu_cores();
        let available_memory = self.node_resources.total_memory_mb - self.allocated_memory_mb();

        available_cores >= cpu_cores && available_memory >= memory_mb
    }
}

//! Comprehensive Validation Logic
//!
//! Validates topology configurations with detailed checks for system constraints,
//! resource compatibility, and deployment feasibility.

use crate::{
    actors::ActorStateType, ActorType, Node, Result, TopologyConfig, TopologyError,
    MAX_ACTORS_PER_NODE, MAX_CPU_CORES_PER_ACTOR,
};
use std::collections::{HashMap, HashSet};

/// Comprehensive topology validator
pub struct TopologyValidator<'a> {
    config: &'a TopologyConfig,
}

impl<'a> TopologyValidator<'a> {
    pub fn new(config: &'a TopologyConfig) -> Self {
        Self { config }
    }

    /// Run all validation checks
    pub fn validate_all(&self) -> Result<ValidationReport> {
        let mut report = ValidationReport::new();

        // Basic structural validation
        self.validate_structure(&mut report)?;

        // Actor-specific validation
        self.validate_actors(&mut report)?;

        // Node-specific validation
        self.validate_nodes(&mut report)?;

        // Channel and dependency validation
        self.validate_channels(&mut report)?;

        // Resource constraint validation
        self.validate_resources(&mut report)?;

        // NUMA and hardware validation
        self.validate_hardware_constraints(&mut report)?;

        // State management validation
        self.validate_state_management(&mut report)?;

        // Network topology validation
        self.validate_network_topology(&mut report)?;

        if report.has_errors() {
            Err(TopologyError::Validation {
                message: format!("Validation failed with {} errors", report.error_count()),
            })
        } else {
            Ok(report)
        }
    }

    /// Validate basic structure and IDs
    fn validate_structure(&self, report: &mut ValidationReport) -> Result<()> {
        // Check for duplicate actor IDs
        let mut actor_ids = HashSet::new();
        for actor_id in self.config.actors.keys() {
            if !actor_ids.insert(actor_id.clone()) {
                report.add_error(format!("Duplicate actor ID: {}", actor_id));
            }
        }

        // Check for duplicate node IDs
        let mut node_ids = HashSet::new();
        for node_id in self.config.nodes.keys() {
            if !node_ids.insert(node_id.clone()) {
                report.add_error(format!("Duplicate node ID: {}", node_id));
            }
        }

        // Validate actor placement references
        for (node_id, node) in &self.config.nodes {
            for actor_id in node.actor_placements.keys() {
                if !self.config.actors.contains_key(actor_id) {
                    report.add_error(format!(
                        "Node '{}' references unknown actor '{}'",
                        node_id, actor_id
                    ));
                }
            }
        }

        // Ensure all actors are placed somewhere
        for actor_id in self.config.actors.keys() {
            let mut placed = false;
            for node in self.config.nodes.values() {
                if node.actor_placements.contains_key(actor_id) {
                    placed = true;
                    break;
                }
            }
            if !placed {
                report.add_warning(format!("Actor '{}' is not placed on any node", actor_id));
            }
        }

        Ok(())
    }

    /// Validate actor configurations
    fn validate_actors(&self, report: &mut ValidationReport) -> Result<()> {
        for (actor_id, actor) in &self.config.actors {
            // Basic actor validation
            if let Err(e) = actor.validate() {
                report.add_error(format!("Actor '{}': {}", actor_id, e));
                continue;
            }

            // Validate source_id uniqueness for producers
            if matches!(actor.actor_type, ActorType::Producer) {
                let source_conflicts: Vec<_> = self
                    .config
                    .actors
                    .iter()
                    .filter(|(other_id, other_actor)| {
                        *other_id != actor_id
                            && other_actor.source_id == actor.source_id
                            && matches!(other_actor.actor_type, ActorType::Producer)
                    })
                    .map(|(id, _)| id)
                    .collect();

                if !source_conflicts.is_empty() {
                    report.add_error(format!(
                        "Producer actor '{}' has conflicting source_id {} with: {:?}",
                        actor_id, actor.source_id, source_conflicts
                    ));
                }
            }

            // Validate channel references
            for input_channel in &actor.inputs {
                if !self.channel_exists(input_channel) {
                    report.add_error(format!(
                        "Actor '{}' references non-existent input channel '{}'",
                        actor_id, input_channel
                    ));
                }
            }

            for output_channel in &actor.outputs {
                if !self.channel_exists(output_channel) {
                    report.add_error(format!(
                        "Actor '{}' references non-existent output channel '{}'",
                        actor_id, output_channel
                    ));
                }
            }
        }

        Ok(())
    }

    /// Validate node configurations
    fn validate_nodes(&self, report: &mut ValidationReport) -> Result<()> {
        for (node_id, node) in &self.config.nodes {
            // Check actor count limit
            if node.actor_placements.len() > MAX_ACTORS_PER_NODE {
                report.add_error(format!(
                    "Node '{}' has {} actors, exceeding limit of {}",
                    node_id,
                    node.actor_placements.len(),
                    MAX_ACTORS_PER_NODE
                ));
            }

            // Validate hostname format
            if node.hostname.is_empty() {
                report.add_error(format!("Node '{}' has empty hostname", node_id));
            }

            // Validate NUMA topology
            self.validate_numa_topology(node_id, node, report)?;

            // Validate CPU assignments
            self.validate_cpu_assignments(node_id, node, report)?;

            // Validate channel configurations
            self.validate_node_channels(node_id, node, report)?;
        }

        Ok(())
    }

    /// Validate NUMA topology configuration
    fn validate_numa_topology(
        &self,
        node_id: &str,
        node: &Node,
        report: &mut ValidationReport,
    ) -> Result<()> {
        // Check NUMA node IDs are valid
        for &numa_id in &node.numa_topology {
            if numa_id > 7 {
                // Most systems have ≤8 NUMA nodes
                report.add_warning(format!(
                    "Node '{}' specifies NUMA node {} which may not exist on typical hardware",
                    node_id, numa_id
                ));
            }
        }

        // Check actor NUMA assignments
        for (actor_id, placement) in &node.actor_placements {
            if let Some(numa_node) = placement.numa {
                if !node.numa_topology.contains(&numa_node) {
                    report.add_error(format!(
                        "Actor '{}' on node '{}' assigned to NUMA node {} not in topology {:?}",
                        actor_id, node_id, numa_node, node.numa_topology
                    ));
                }
            }
        }

        Ok(())
    }

    /// Validate CPU core assignments
    fn validate_cpu_assignments(
        &self,
        node_id: &str,
        node: &Node,
        report: &mut ValidationReport,
    ) -> Result<()> {
        let mut cpu_usage = HashMap::<u8, Vec<String>>::new();

        for (actor_id, placement) in &node.actor_placements {
            // Check CPU core count limit
            if placement.cpu.len() > MAX_CPU_CORES_PER_ACTOR {
                report.add_error(format!(
                    "Actor '{}' on node '{}' assigned {} CPU cores, exceeding limit of {}",
                    actor_id,
                    node_id,
                    placement.cpu.len(),
                    MAX_CPU_CORES_PER_ACTOR
                ));
            }

            // Track CPU usage for conflict detection
            for &cpu_core in &placement.cpu {
                cpu_usage
                    .entry(cpu_core)
                    .or_default()
                    .push(actor_id.clone());

                // Warn about high CPU core numbers (likely don't exist)
                if cpu_core > 127 {
                    report.add_warning(format!(
                        "Actor '{}' on node '{}' assigned CPU core {} which may not exist",
                        actor_id, node_id, cpu_core
                    ));
                }
            }
        }

        // Check for CPU conflicts
        for (cpu_core, actors) in cpu_usage {
            if actors.len() > 1 {
                report.add_warning(format!(
                    "CPU core {} on node '{}' assigned to multiple actors: {:?}",
                    cpu_core, node_id, actors
                ));
            }
        }

        Ok(())
    }

    /// Validate node channel configurations
    fn validate_node_channels(
        &self,
        node_id: &str,
        node: &Node,
        report: &mut ValidationReport,
    ) -> Result<()> {
        for (channel_name, channel_config) in &node.local_channels {
            // Validate buffer size
            if channel_config.buffer_size == 0 {
                report.add_error(format!(
                    "Channel '{}' on node '{}' has zero buffer size",
                    channel_name, node_id
                ));
            }

            // Validate NUMA node assignment for channels
            if let Some(numa_node) = channel_config.numa_node {
                if !node.numa_topology.contains(&numa_node) {
                    report.add_error(format!(
                        "Channel '{}' on node '{}' assigned to NUMA node {} not in topology",
                        channel_name, node_id, numa_node
                    ));
                }
            }

            // Validate huge pages are only used with NUMA
            if channel_config.huge_pages && channel_config.numa_node.is_none() {
                report.add_warning(format!(
                    "Channel '{}' on node '{}' uses huge pages without NUMA assignment",
                    channel_name, node_id
                ));
            }
        }

        Ok(())
    }

    /// Validate channel connectivity and data flow
    fn validate_channels(&self, report: &mut ValidationReport) -> Result<()> {
        // Build channel producer/consumer map
        let mut channel_producers = HashMap::<String, Vec<String>>::new();
        let mut channel_consumers = HashMap::<String, Vec<String>>::new();

        for (actor_id, actor) in &self.config.actors {
            for output in &actor.outputs {
                channel_producers
                    .entry(output.clone())
                    .or_default()
                    .push(actor_id.clone());
            }
            for input in &actor.inputs {
                channel_consumers
                    .entry(input.clone())
                    .or_default()
                    .push(actor_id.clone());
            }
        }

        // Check for channels with no producers
        for (channel, consumers) in &channel_consumers {
            if !channel_producers.contains_key(channel) {
                report.add_error(format!(
                    "Channel '{}' has consumers {:?} but no producers",
                    channel, consumers
                ));
            }
        }

        // Check for channels with no consumers
        for (channel, producers) in &channel_producers {
            if !channel_consumers.contains_key(channel) {
                report.add_warning(format!(
                    "Channel '{}' has producers {:?} but no consumers",
                    channel, producers
                ));
            }
        }

        // Check for multiple producers on single-producer channels
        for (channel, producers) in &channel_producers {
            if producers.len() > 1 {
                // Check if any consumer actors expect single producer
                if let Some(consumers) = channel_consumers.get(channel) {
                    for consumer_id in consumers {
                        if let Some(consumer) = self.config.actors.get(consumer_id) {
                            // Some actor types might require single producer
                            if matches!(consumer.actor_type, ActorType::Consumer) {
                                report.add_warning(format!(
                                    "Channel '{}' has multiple producers {:?} for consumer '{}'",
                                    channel, producers, consumer_id
                                ));
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate resource constraints and feasibility
    fn validate_resources(&self, report: &mut ValidationReport) -> Result<()> {
        // Calculate resource requirements per node
        for (node_id, node) in &self.config.nodes {
            let mut total_memory = 0usize;
            let mut total_cpu_cores = HashSet::<u8>::new();
            let mut total_disk = 0usize;
            let mut _total_bandwidth = 0usize;
            let mut gpu_required = false;

            for actor_id in node.actor_placements.keys() {
                if let Some(actor) = self.config.actors.get(actor_id) {
                    total_memory += actor
                        .resources
                        .max_memory_mb
                        .unwrap_or(actor.resources.min_memory_mb);

                    // Add CPU cores from placement
                    if let Some(placement) = node.actor_placements.get(actor_id) {
                        for &cpu in &placement.cpu {
                            total_cpu_cores.insert(cpu);
                        }
                    }

                    if let Some(disk) = actor.resources.disk_space_mb {
                        total_disk += disk;
                    }

                    if let Some(bandwidth) = actor.resources.network_bandwidth_mbps {
                        _total_bandwidth += bandwidth;
                    }

                    if actor.resources.gpu_required {
                        gpu_required = true;
                    }
                }
            }

            // Check reasonable resource limits
            if total_memory > 1024 * 1024 {
                // 1TB
                report.add_warning(format!(
                    "Node '{}' requires {}MB memory which may exceed typical hardware",
                    node_id, total_memory
                ));
            }

            if total_cpu_cores.len() > 128 {
                report.add_warning(format!(
                    "Node '{}' requires {} CPU cores which may exceed typical hardware",
                    node_id,
                    total_cpu_cores.len()
                ));
            }

            if total_disk > 100 * 1024 * 1024 {
                // 100TB
                report.add_warning(format!(
                    "Node '{}' requires {}MB disk space which may be excessive",
                    node_id, total_disk
                ));
            }

            if gpu_required {
                report.add_info(format!("Node '{}' requires GPU hardware", node_id));
            }
        }

        Ok(())
    }

    /// Validate hardware-specific constraints
    fn validate_hardware_constraints(&self, report: &mut ValidationReport) -> Result<()> {
        for (node_id, node) in &self.config.nodes {
            // Check for huge pages usage
            let uses_huge_pages = node
                .local_channels
                .values()
                .any(|channel| channel.huge_pages);

            if uses_huge_pages {
                report.add_info(format!(
                    "Node '{}' requires huge pages configuration",
                    node_id
                ));
            }

            // Check for NUMA-aware placement
            let numa_aware = node
                .actor_placements
                .values()
                .any(|placement| placement.numa.is_some());

            if numa_aware && node.numa_topology.is_empty() {
                report.add_error(format!(
                    "Node '{}' has NUMA-aware actor placement but no NUMA topology",
                    node_id
                ));
            }
        }

        Ok(())
    }

    /// Validate state management configurations
    fn validate_state_management(&self, report: &mut ValidationReport) -> Result<()> {
        for (actor_id, actor) in &self.config.actors {
            match &actor.state.state_type {
                ActorStateType::Persistent {
                    storage_backend, ..
                } => {
                    self.validate_storage_backend(actor_id, storage_backend, report)?;
                }
                ActorStateType::Replicated {
                    replication_factor,
                    storage_backend,
                } => {
                    if *replication_factor < 2 {
                        report.add_error(format!(
                            "Actor '{}' has replication factor {} which must be ≥2",
                            actor_id, replication_factor
                        ));
                    }

                    if *replication_factor > self.config.nodes.len() {
                        report.add_error(format!(
                            "Actor '{}' has replication factor {} exceeding node count {}",
                            actor_id,
                            replication_factor,
                            self.config.nodes.len()
                        ));
                    }

                    self.validate_storage_backend(actor_id, storage_backend, report)?;
                }
                _ => {} // No additional validation for stateless/in-memory
            }
        }

        Ok(())
    }

    /// Validate storage backend configuration
    fn validate_storage_backend(
        &self,
        actor_id: &str,
        backend: &crate::actors::StorageBackend,
        report: &mut ValidationReport,
    ) -> Result<()> {
        use crate::actors::StorageBackend;

        match backend {
            StorageBackend::LocalFile { base_path, .. } => {
                if !base_path.is_absolute() {
                    report.add_error(format!(
                        "Actor '{}' storage path must be absolute: {:?}",
                        actor_id, base_path
                    ));
                }
            }
            StorageBackend::DistributedKV { endpoint, .. } => {
                if endpoint.is_empty() {
                    report.add_error(format!(
                        "Actor '{}' has empty distributed KV endpoint",
                        actor_id
                    ));
                }
            }
            StorageBackend::Database {
                connection_string, ..
            } => {
                if connection_string.is_empty() {
                    report.add_error(format!(
                        "Actor '{}' has empty database connection string",
                        actor_id
                    ));
                }
            }
        }

        Ok(())
    }

    /// Validate network topology and routing
    fn validate_network_topology(&self, report: &mut ValidationReport) -> Result<()> {
        // Check inter-node communication requirements
        let mut node_pairs = HashSet::<(String, String)>::new();

        // Find actors that need to communicate across nodes
        for (actor_id, actor) in &self.config.actors {
            let actor_node = self.find_actor_node(actor_id);

            for output_channel in &actor.outputs {
                // Find consumers of this channel
                for (consumer_id, consumer) in &self.config.actors {
                    if consumer.inputs.contains(output_channel) {
                        let consumer_node = self.find_actor_node(consumer_id);

                        if let (Some(source), Some(target)) =
                            (actor_node.as_ref(), consumer_node.as_ref())
                        {
                            if source != target {
                                let pair = if source < target {
                                    (source.clone(), target.clone())
                                } else {
                                    (target.clone(), source.clone())
                                };
                                node_pairs.insert(pair);
                            }
                        }
                    }
                }
            }
        }

        // Validate required inter-node connections
        for (source, target) in node_pairs {
            if let Some(inter_node) = &self.config.inter_node {
                let has_route = inter_node.routes.iter().any(|route| {
                    (route.source_node == source && route.target_node == target)
                        || (route.source_node == target && route.target_node == source)
                });

                if !has_route {
                    report.add_warning(format!(
                        "No inter-node route defined for communication between '{}' and '{}'",
                        source, target
                    ));
                }
            } else {
                report.add_warning(format!(
                    "Inter-node communication required between '{}' and '{}' but no inter_node config",
                    source, target
                ));
            }
        }

        Ok(())
    }

    /// Helper: Check if a channel exists in any node
    fn channel_exists(&self, channel_name: &str) -> bool {
        self.config
            .nodes
            .values()
            .any(|node| node.local_channels.contains_key(channel_name))
    }

    /// Helper: Find which node an actor is placed on
    fn find_actor_node(&self, actor_id: &str) -> Option<String> {
        for (node_id, node) in &self.config.nodes {
            if node.actor_placements.contains_key(actor_id) {
                return Some(node_id.clone());
            }
        }
        None
    }
}

/// Validation report with errors, warnings, and info messages
#[derive(Debug, Default)]
pub struct ValidationReport {
    errors: Vec<String>,
    warnings: Vec<String>,
    info: Vec<String>,
}

impl ValidationReport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_error(&mut self, message: String) {
        self.errors.push(message);
    }

    pub fn add_warning(&mut self, message: String) {
        self.warnings.push(message);
    }

    pub fn add_info(&mut self, message: String) {
        self.info.push(message);
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    pub fn warning_count(&self) -> usize {
        self.warnings.len()
    }

    pub fn info_count(&self) -> usize {
        self.info.len()
    }

    pub fn errors(&self) -> &[String] {
        &self.errors
    }

    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    pub fn info(&self) -> &[String] {
        &self.info
    }
}
